# 설계: 위키 서버 아키텍처와 로드맵

상태: 초안 (2026-07)

docs/design/06(기능·요구사항)을 전제로 한다. 목표는 자체 호스팅 1순위의 단일 바이너리
위키 서버이고, 기존 렌더링 파이프라인(docs/design/03)을 소비하는 상위 계층이다.

## 기술 선택

| 영역 | 선택 | 근거 |
|---|---|---|
| HTTP | axum (tokio·tower) | Rust 생태계 표준. tower 미들웨어로 세션·캡차·요청 제한을 계층화 |
| DB | sqlx + SQLite 기본, PostgreSQL 옵션 | 단일 바이너리 요구사항. SQL을 공통 부분집합으로 유지해 두 방언 지원(openNAMU의 SQLite/MySQL 이중화와 같은 접근) |
| 검색 | tantivy + lindera(한국어 형태소) | 임베디드 전문 검색 — 외부 검색엔진 의존 없이 P1 검색을 충족. the seed의 Elasticsearch 분리는 규모가 커질 때의 선택지로만 |
| 페이지 셸 | askama | 컴파일 타임 템플릿. 본문 HTML은 backend-namuwiki가 이미 방출하므로 셸(헤더·사이드바·스킨)만 담당 |
| 프론트엔드 | 서버 사이드 렌더링 + 최소 JS | 편집기·토론 갱신 등 필요한 곳만 JS. SPA 전환은 비목표 |
| 세션·비밀번호 | tower-sessions + argon2 | 표준 선택 |

## 크레이트 구조

분할 기준은 **데이터 소유권**이다 — 각 크레이트가 자기 부분영역의 타입·규칙·저장·질의를
전부 소유한다. 초안 두 번(4개 layer 분할, service를 더한 5개)은 기술 계층
(순수 규칙/영속화/조율/HTTP)으로 잘랐는데, 그 기준은 MECE가 아니었다 — 기능 하나
(예: 토론)가 domain·storage·service·server 네 크레이트에 조각나 얹히고, "유스케이스
조율"은 무엇이든 담기는 잡동사니 계층이 된다. 나무마크 계열이 MECE인 이유는 계층이어서가
아니라 **크레이트마다 소유하는 산출물(CST·AST·IR·마크업)이 다르기 때문**이다. 위키
서버에서 그에 대응하는 소유물은 부분영역의 데이터다.

```
┌──────────────┐   ┌───────────────┐   ┌─────────────────┐
│ wiki-account │◀──│ wiki-document │◀──│ wiki-discussion │
│ 행위 주체      │   │ 문서·리비전     │   │ 토론·편집요청     │
└──────▲───────┘   └───▲───────┬───┘   └────────▲────────┘
       │               │       └─▶ namumark-render·backend-namuwiki
┌──────┴───────────────┴───┐       ┌─────────────┐   ┌─────────────┐
│ wiki-authorization       │       │ wiki-search │   │ wiki-server │──▶ (전부)
│ ACL·aclgroup·perm        │       │ tantivy 색인 │   │ axum + 스킨  │
└──────────────────────────┘       └─────────────┘   └─────────────┘
```

| 크레이트 | 소유 영역 | 소유 테이블(docs/design/08) | 내용 |
|---|---|---|---|
| `wiki-account` | 행위 주체 | actor·wiki_user·user_credential·user_email·user_preference·notification·star | 가입·인증(비밀번호·TOTP)·IP 사용자·알림·북마크. `ActorIdentifier` 등 참조 타입의 원천 |
| `wiki-document` | 문서와 그 역사 | document·revision·document_reference·render_cache·file_content·file_revision | 리비전 채번·이동·삭제·diff·3-way 병합·역링크·파일·렌더 캐시. `WikiContext` 구현과 렌더 파이프라인 연결도 여기 — 링크 존재·include 원문이 곧 이 크레이트의 데이터다 |
| `wiki-authorization` | 권한 판정 | acl_rule·acl_group·acl_group_member·user_permission | ACL 평가기(순수 함수)와 그 규칙의 저장·관리, aclgroup(차단), perm 부여, `/block-history` 질의 |
| `wiki-discussion` | 대화 | thread·thread_comment·edit_request | 스레드·댓글·상태 전이·편집요청 흐름 |
| `wiki-search` | 전문 검색 색인 | (DB 테이블 없음 — tantivy 색인 파일) | 색인 구축·질의. 다른 크레이트를 모름 — 색인할 내용은 호출자가 공급 |
| `wiki-server` | HTTP 조립 | site_setting·세션 | axum 라우팅(아래 URL 설계)·세션·폼·상태 코드·스킨 셸(askama). 유스케이스는 핸들러가 하위 크레이트 호출 몇 개를 이어 붙이는 얇은 조립 |

- **MECE의 실체는 소유권 표다**: 08의 모든 테이블과 06의 모든 기능 영역이 정확히 한
  크레이트에 속한다(문서 1·2·M0 잔여→document, 3·8→authorization, 4→discussion,
  5→account, 6→search, 7 특수 페이지는 소유 크레이트의 질의 + server 라우트).
  기능 추가·수정은 소유 크레이트 하나 + server 라우트만 건드린다.
- **의존은 참조 방향**: 리비전이 actor를 참조하므로 document → account, 토론이 문서에
  달리므로 discussion → document, ACL scope가 문서·이름공간이므로 authorization →
  document·account. 순환 없음.
- **마이그레이션은 단일 순서 폴더**: 크레이트가 교차 FK(document → account 등)를
  가지므로 마이그레이션은 소유 크레이트별로 흩지 않고 서버 바이너리의 단일
  `migrations/`에 타임스탬프 순서로 둔다(sqlx 표준). 논리적 소유(크레이트)와
  물리적 적용 순서(단일 폴더)를 분리한다 — 크레이트가 자기 테이블의 질의·타입을
  소유하는 것과 별개다.
- **부분영역 간 정합은 최상위에서**: "편집 = 권한 판정 → 리비전 저장 → 색인 갱신"처럼
  크레이트를 가로지르는 흐름은 wiki-server(와 덤프 임포터 바이너리)가 조립한다.
  전용 조율 크레이트는 두지 않는다 — 그것이 잡동사니가 되는 자리다.
- **두지 않는 것**: repository trait 간접층(sqlx 직접, 테스트는 SQLite in-memory),
  스킨 계약 크레이트(M6에 wiki-server에서 추출), 공유 "domain 타입" 크레이트(참조
  타입은 소유 크레이트가 공개한다 — text↛ast처럼 원천을 하나로).

데이터 모델은 docs/design/08-wiki-server-data-model.md에 따로 정리한다.

## URL 설계

원칙: **일반적인 웹페이지 경험을 따른다.** the seed URL 호환은 목표가 아니다.

- 소문자 kebab-case 경로. the seed의 PascalCase 특수 페이지(`/RecentChanges`)는
  `/recent-changes`처럼 바꾼다.
- 표준 HTTP 의미론: 조회는 GET(부작용 없음), 변경은 POST 후 303 리다이렉트(PRG),
  권한 거부 403, 없는 문서 404(빈 문서 안내 페이지로), 리다이렉트 문서는 302 +
  `?from=` 표시.
- 페이지네이션은 쿼리 파라미터(`?page=`, 목록 성격에 따라 `?from=` 커서), 필터도
  쿼리 파라미터(`?namespace=`, `?status=`).
- 제목은 표준 퍼센트 인코딩. 렌더러의 링크 인코딩 규칙(대문자 hex, `:`·`/`·`(`·`)`
  보존)과 같은 함수를 공유해 문서 안 링크와 주소창 표기가 일치하게 한다.

문서 동작 경로는 **동사 접두사** 방식을 쓴다:

```
/w/<제목>          보기          /edit/<제목>       편집
/raw/<제목>        원문          /history/<제목>    역사
/diff/<제목>       비교          /revert/<제목>     되돌리기
/blame/<제목>      blame        /discuss/<제목>    토론 목록
/backlink/<제목>   역링크        /acl/<제목>        ACL
/move/<제목>       이동          /delete/<제목>     삭제
```

접미사 방식(`/w/<제목>/history`)이 더 흔한 관례지만, 나무위키 제목은 `/`를 포함할 수
있어(하위 문서 `상위/하위`) 접미사가 제목의 일부인지 동작인지 모호해진다. 동사 접두사는
이 모호성이 없고 위키 관례(위키백과의 `action=history`, the seed)와도 어긋나지 않는다.
`/`나 `..` 등 경로로 표현 못 하는 제목은 `?doc=` fallback.

그 외 영역:

```
/search?q=&target=&namespace=     검색 (제목 완전 일치 시 즉시 리다이렉트)
/file/<파일명>                     파일 바이너리 서빙 (현재 리비전의 content_hash 조회)
/thread/<uuid>                    토론 스레드 (외부 식별자 — 내부 PK 비노출)
/edit-request/<uuid>              편집요청 (외부 식별자)
/login  /logout  /signup          로그인(GET 폼/POST)·로그아웃(POST)·가입
/settings                         내 설정 (the seed의 /member/mypage)
/users/<이름>                     사용자 (사용자 문서로 리다이렉트)
/users/<이름>/contributions       기여 목록 (제목에 /가 없는 사용자명이라 접미사 가능)
/admin/grant  /admin/config  …    운영 도구
/recent-changes  /random  …       특수 페이지 (docs/design/06 표 참고)
```

파일은 문서(파일 이름공간)라 보기·역사·ACL은 문서 동작 경로를 그대로 쓰고, 바이너리
전송만 `/file/<파일명>`으로 분리한다 — 렌더러 `WikiContext`의 파일 URL 훅이 이 경로를
돌려줘 문서 안 `[[파일:…]]`의 `<img src>`와 일치한다.

## 핵심 설계 결정

- **리비전은 append-only 전문 저장**: 원문 바이트 그대로. diff·blame은 조회 시 계산
  (필요 시 캐시). 삭제도 리비전(빈 상태 전이)이라 복원이 자연스럽다. 외부 식별은
  the seed와 같이 순번(r숫자) + UUID 이중이되, 내부 PK는 따로 둔다(docs/design/08의
  내부·외부 식별자 분리 원칙).
- **렌더 캐시와 역링크**: 문서 저장 시 resolve가 수집한 링크·include·분류를 역링크
  테이블에 기록한다. 이 테이블이 (1) `/backlink/`·`/needed-pages`의 자료, (2) 렌더 캐시
  무효화의 근거다 — 문서 A의 생성·삭제는 A를 링크한 문서들의 `not-exist` 클래스와
  include 결과를 바꾸므로 해당 캐시만 무효화한다.
- **ACL 평가는 wiki-authorization의 순수 함수**: 규칙 목록 + 요청 주체(사용자/IP/aclgroup 소속) →
  허용/거부. 평가 순서는 the seed 그대로(문서 → 이름공간 → 기본 거부). 저장·UI와 분리해
  규칙 조합을 단위 테스트로 못박는다.
- **차단은 별도 시스템이 아니라 aclgroup**: the seed와 동일 — 그룹 + 만료 + 사유가
  전부이고, ACL 조건 `aclgroup:이름`이 효력을 만든다.
- **본문과 셸의 경계**: backend-namuwiki가 본문(`wiki-paragraph` 어휘)을, wiki-server의
  스킨이 셸을 그린다. 파리티 대조에서 확인했듯 the seed도 본문 클래스는 공개 어휘,
  셸은 스킨 소관이다 — 이 경계를 그대로 코드 경계로 쓴다.

## 로드맵

각 마일스톤은 그 시점에 배포 가능한 상태를 만든다.

### M0 — 파서·렌더러 파리티 (완료)
알파위키 문법 도움말·심화 두 문서 모두 파리티 0(알려진 차이 제외). 규명한 문법은
[나무마크 문법 스펙](../spec/namumark.md)에 정리했다. 위키 서버(M1~)와 병행 가능하며,
코퍼스를 다른 문서로 넓히는 것이 후속 과제다.

### M1 — 읽기 전용 위키
wiki-document·wiki-search·wiki-server 골격(account는 actor만, 익명 참조용). 문서
보기(`/w/`, `/raw/`)·리다이렉트·하위 문서·분류 목록·`/random`·`/search`(tantivy,
제목 완전 일치 리다이렉트)·`/license`. 나무위키 덤프 임포터로 실데이터를 넣어 검증 —
파리티 코퍼스가 그대로 통합 테스트가 된다.

### M2 — 편집·리비전과 최소 권한
편집(IP 사용자)·히스토리·diff·되돌리기·편집 충돌 3-way 병합·역링크 갱신·렌더 캐시
무효화·`/recent-changes`·`/needed-pages` 등 목록 특수 페이지. **wiki-authorization
골격을 여기서**: ACL 평가기 + 계정에 기대지 않는 규칙(`perm:any`/`perm:ip`/`ip:CIDR`/
이름공간)과 편집 차단 aclgroup. 편집을 여는 첫 마일스톤이 곧 스팸·반달에 노출되는
지점이라 권한 판정이 함께 가야 배포 가능하다.

### M3 — 계정과 완전한 권한
가입(이메일 인증)·로그인·세션·기여 목록. wiki-authorization을 계정까지 확장:
`perm:member` 등 계정 조건·perm 부여(grant)·nsacl·aclgroup 관리 UI·`/block-history`
감사 로그. account의 user, authorization의 계정 의존부가 여기서 완성된다.

### M4 — 토론과 편집요청
스레드·상태(normal/pause/close)·댓글 관리·편집요청 흐름.

### M5 — 파일과 운영
파일 업로드(라이선스·분류 필수)·문서 이동(swap)/삭제·위키 설정(config)·blame·
리비전 숨김·일괄 되돌리기·캡차.

### M6 — 다듬기
스킨 체계·알림·북마크·TOTP·자동완성·읽기 API·나머지 특수 페이지.

로드맵 재검토(2026-07)에서 초안을 고친 근거:

- **ACL을 M3→M2로 당김(권한 판정을 편집과 함께)**: 초안은 편집(M2)을 권한 없이 열고
  ACL을 M3에 뒀는데, 그러면 M2 배포본이 스팸·반달에 무방비라 "배포 가능" 전제와
  어긋났다. 대신 authorization을 계정 유무로 쪼갰다 — IP·이름공간 규칙은 M2,
  계정 조건·perm·관리 UI는 M3. account가 actor(M1)/user(M3)로 갈리는 것과 같은 결이라
  수직 분할과 일관된다.
- **`/recent-changes`를 M1→M2로**: 덤프 임포트는 편집 이력이 아니라 스냅샷 적재라
  M1에서는 최근 변경이 임포트 시각 한 줄뿐이다. 편집이 생기는 M2에서 의미를 갖는다.
- **읽기 전용을 첫 마일스톤으로 유지**: 렌더러 파리티 검증과 직결(덤프 임포트 → 실문서
  대량 렌더)이라 위키 서버의 첫 산출물로 가장 값지다 — 순서 유지.

## 비목표

- the seed 내부 구현(난독화 코드) 모방 — 근거는 관찰 가능한 동작뿐이다.
- SPA 프론트엔드, 다중 위키 호스팅, 외부 검색엔진 필수 의존.
- openNAMU 자체 확장(게시판, 도전과제)의 재현.
