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

단일 역할 원칙(docs/design/03)을 이어 간다. 나무마크 계열과 접두사를 분리한다.

```
┌─────────────┐   ┌──────────────┐   ┌─────────────┐
│ wiki-domain │◀──│ wiki-storage │◀──│ wiki-server │──▶ namumark-render
│ 도메인 타입   │   │ sqlx 저장소   │   │ axum HTTP   │──▶ namumark-backend-
│ + 순수 규칙   │◀──┼──────────────┤   │ + 스킨 셸    │       namuwiki
└─────────────┘   │ wiki-search  │◀──│             │
                  │ tantivy 색인  │   └─────────────┘
                  └──────────────┘
```

| 크레이트 | 단일 역할 | 내용 |
|---|---|---|
| `wiki-domain` | 도메인 타입 + 순수 규칙 | `DocumentTitle`(이름공간+이름), `Revision`(순번+UUID), ACL 모델과 평가기, 3-way 병합, perm 정의. I/O 없음 — 전부 단위 테스트 가능 |
| `wiki-storage` | 영속화 | 문서·리비전·사용자·ACL·토론·역링크·감사 로그 저장소. 마이그레이션 포함 |
| `wiki-search` | 전문 검색 색인 | tantivy 색인 구축·질의. 저장소 변경 이벤트를 구독해 색인 갱신 |
| `wiki-server` | HTTP·조립 | 라우팅(아래 URL 설계), 세션·인증, `WikiContext` 구현(저장소를 등에 업고 링크 존재·include 원문·파일 URL 공급), 렌더 캐시, 스킨 셸 |

렌더링 파이프라인과의 접점은 `namumark_render::WikiContext` 하나다 — 지금 파리티
하네스가 코퍼스로 흉내 내는 것을 wiki-server가 실 저장소로 구현한다.

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
/thread/<uuid>                    토론 스레드 (외부 식별자 — 내부 PK 비노출)
/login  /logout  /signup          로그인(GET 폼/POST)·로그아웃(POST)·가입
/settings                         내 설정 (the seed의 /member/mypage)
/users/<이름>                     사용자 (사용자 문서로 리다이렉트)
/users/<이름>/contributions       기여 목록 (제목에 /가 없는 사용자명이라 접미사 가능)
/admin/grant  /admin/config  …    운영 도구
/recent-changes  /random  …       특수 페이지 (docs/design/06 표 참고)
```

## 핵심 설계 결정

- **리비전은 append-only 전문 저장**: 원문 바이트 그대로. diff·blame은 조회 시 계산
  (필요 시 캐시). 삭제도 리비전(빈 상태 전이)이라 복원이 자연스럽다. 외부 식별은
  the seed와 같이 순번(r숫자) + UUID 이중이되, 내부 PK는 따로 둔다(docs/design/08의
  내부·외부 식별자 분리 원칙).
- **렌더 캐시와 역링크**: 문서 저장 시 resolve가 수집한 링크·include·분류를 역링크
  테이블에 기록한다. 이 테이블이 (1) `/backlink/`·`/NeededPages`의 자료, (2) 렌더 캐시
  무효화의 근거다 — 문서 A의 생성·삭제는 A를 링크한 문서들의 `not-exist` 클래스와
  include 결과를 바꾸므로 해당 캐시만 무효화한다.
- **ACL은 wiki-domain의 순수 함수**: 규칙 목록 + 요청 주체(사용자/IP/aclgroup 소속) →
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
wiki-domain·wiki-storage·wiki-server 골격. 문서 보기(`/w/`, `/raw/`)·리다이렉트·
하위 문서·분류 목록·`/RecentChanges`·`/RandomPage`·`/Search`(tantivy)·`/Go`·`/License`.
나무위키 덤프 임포터로 실데이터를 넣어 검증 — 파리티 코퍼스가 그대로 통합 테스트가 된다.

### M2 — 편집과 리비전
편집(IP 사용자 포함)·히스토리·diff·되돌리기·편집 충돌 3-way 병합·역링크 갱신·
렌더 캐시 무효화·`/NeededPages` 등 목록 특수 페이지.

### M3 — 계정과 ACL
가입(이메일 인증)·로그인·세션·perm·문서/이름공간 ACL·aclgroup(차단)·grant·
`/BlockHistory` 감사 로그.

### M4 — 토론과 편집요청
스레드·상태(normal/pause/close)·댓글 관리·편집요청 흐름·기여 목록.

### M5 — 파일과 운영
파일 업로드(라이선스·분류 필수)·문서 이동(swap)/삭제·위키 설정(config)·blame·
리비전 숨김·일괄 되돌리기·캡차.

### M6 — 다듬기
스킨 체계·알림·북마크·TOTP·자동완성·읽기 API·나머지 특수 페이지.

## 비목표

- the seed 내부 구현(난독화 코드) 모방 — 근거는 관찰 가능한 동작뿐이다.
- SPA 프론트엔드, 다중 위키 호스팅, 외부 검색엔진 필수 의존.
- openNAMU 자체 확장(게시판, 도전과제)의 재현.
