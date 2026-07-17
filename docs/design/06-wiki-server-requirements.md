# 설계: 위키 서버 기능·요구사항

상태: 초안 (2026-07)

나무위키(the seed)·알파위키·openNAMU를 조사해 정리한 위키 서버의 기능 목록과 요구사항이다.
파서·렌더링(나무마크)은 별도 완료 단계이므로 여기서는 서버 기능만 다룬다.

## 근거 우선순위

문법 파리티와 같은 원칙을 따른다.

1. **the seed 실동작** — 알파위키(www.alphawiki.org, 같은 엔진)와 theseed.io의
   「the seed/URL 및 기능」·「ACLGroup」 문서, 알파위키 기능·운영진 권한 도움말.
2. **openNAMU·the tree** — 참고용. 자체 확장(게시판, 뱃지 등)이 섞여 있어 단독 근거로 쓰지 않는다.

the seed는 비공개(Node.js + Vue/Nuxt, Elasticsearch, main·search·file 3서비스 분리)다.
기존 오픈소스 재구현 중 the seed에 가장 근접한 the tree는 수정·재배포 금지 라이선스라
MIT 오픈소스 재구현이라는 이 프로젝트의 목표와 겹치지 않는다.

## 기능 요구사항

우선순위: **P1**(위키로 성립하는 최소), **P2**(나무위키형 위키의 정체성), **P3**(운영·확장).

### 1. 문서 시스템

| 기능 | 우선순위 | 내용 |
|---|---|---|
| 문서 CRUD | P1 | 제목 = 이름공간 + 이름. 보기(`/w/`)·편집(`/edit/`)·원문(`/raw/`) |
| 이름공간 | P1 | 문서(기본)·틀·분류·파일·사용자·위키운영·휴지통. 이름공간별 기본 ACL의 단위 |
| 리다이렉트 | P1 | `#redirect 대상`. 이동 후 자동 생성 옵션 |
| 하위 문서 | P1 | `상위/하위` 경로 관습. 상대 링크(`../`, `/하위`)는 렌더러가 이미 구현 |
| 분류 | P1 | `[[분류:이름]]` 수집(렌더러의 resolve가 이미 수집) → 분류 문서가 소속 목록 표시 |
| 역링크 | P2 | `/backlink/` — 문서를 링크·include·리다이렉트하는 문서 목록. 링크 존재 여부 판정(`not-exist`)의 역방향 자료 |
| 파일 업로드 | P2 | `/Upload` — 파일명·라이선스 선택·분류 지정 필수. `[[파일:이름]]` 첨부 |
| 문서 이동 | P2 | `/move/` — 사유 필수, 양쪽에 역사가 있으면 맞바꾸기(swap) 지원 |
| 문서 삭제 | P2 | `/delete/` — 사유 필수(5자 이상). 역사가 보존되는 논리적 삭제, 복원 가능 |
| 문서 제목 예외 | P2 | `..`·`/` 등 경로로 표현 못 하는 제목은 `?doc=` 파라미터 fallback |

### 2. 리비전·역사

| 기능 | 우선순위 | 내용 |
|---|---|---|
| 리비전 저장 | P1 | append-only. 순번(r1, r2, …)과 UUID 이중 식별(the seed와 동일) |
| 히스토리 | P1 | `/history/` — 리비전 목록, 편집 요약, 바이트 증감 |
| diff | P1 | `/diff/` — 임의 두 리비전 비교 |
| RAW 보기 | P1 | `/raw/문서?uuid=` — 특정 리비전 원문 |
| 되돌리기 | P2 | `/revert/?uuid=` — 특정 리비전 내용으로 새 리비전 생성 |
| blame | P3 | `/blame/` — 줄 단위 마지막 수정자·리비전 |
| 편집 충돌 병합 | P2 | 편집 시작 시점 리비전 기준 3-way 자동 병합, 실패 시 충돌 화면 |
| 리비전 숨김 | P3 | `hide_revision` 권한으로 특정 리비전 비공개 |

### 3. 권한 (ACL)

the seed의 모델을 그대로 따른다. 평가 규칙: 문서 ACL을 순서대로 검사 → 첫 매치의
allow/deny로 즉시 결정 → 문서 규칙이 없으면 이름공간 ACL → 그것도 없으면 거부.

| 요소 | 우선순위 | 내용 |
|---|---|---|
| ACL 항목(action) | P2 | read, edit, move, delete, create_thread, write_thread_comment, edit_request, acl |
| 조건(condition) | P2 | `perm:*`(any/member/admin/ip/document_contributor/…), `user:이름`, `ip:CIDR`, `geoip:국가`, `aclgroup:그룹` |
| 이름공간 ACL | P2 | `nsacl` 권한으로 수정 |
| aclgroup | P2 | 사용자·IP(CIDR) 집합 + 만료 시간 + 사유. 차단·경고 시스템의 실체 |
| perm(권한 등급) | P2 | admin, grant, aclgroup, nsacl, delete_thread, hide_thread_comment, update_thread_status, update_thread_document, update_thread_topic, hide_revision, batch_revert, login_history, config, api_access, skip_captcha, developer |

### 4. 토론·편집요청

| 기능 | 우선순위 | 내용 |
|---|---|---|
| 토론 스레드 | P2 | `/discuss/문서`(목록·생성), `/thread/주소`(댓글 #번호 단위) |
| 스레드 상태 | P2 | normal / pause / close. 주제 변경, 다른 문서로 이동 |
| 댓글 관리 | P3 | 블라인드·복구, 관리자 `[ADMIN]` 표시 발언 |
| 편집요청 | P2 | 편집 권한이 없을 때 변경안 제출 → 권한자가 Accept/Close, 요청자 수정 가능 |

### 5. 계정·사용자

| 기능 | 우선순위 | 내용 |
|---|---|---|
| 가입·로그인 | P2 | 이메일 인증 가입(사용자 문서 자동 생성), 로그인·로그아웃 |
| IP 사용자 | P1 | 비로그인 편집 시 IP가 식별자. IP에는 grant 불가 |
| 계정 관리 | P3 | 스킨 설정, 이름 변경(기간 제한), 비밀번호·이메일 변경, 탈퇴, TOTP 2단계 인증, 미확인 기기 이메일 인증 |
| 기여 목록 | P2 | 사용자별 문서·토론 기여 |
| 북마크 | P3 | 내 문서함(문서 구독) |
| 알림 | P3 | 토론 댓글 등 |

### 6. 검색

| 기능 | 우선순위 | 내용 |
|---|---|---|
| 전문 검색 | P1 | `/search?q=` — 제목/내용/RAW 대상(`target=`), 이름공간 필터. 한국어 형태소 분석 필요 |
| 바로가기 | P1 | 검색창 제출 시 제목 완전 일치면 문서로 즉시 리다이렉트, 아니면 검색 결과(일반 위키 관례 — the seed의 별도 `/Go` 경로는 두지 않음) |
| 제목 자동완성 | P3 | 검색창 자동완성 |

### 7. 특수 페이지

경로는 일반 웹 관례(소문자 kebab-case)를 따른다. the seed의 원래 경로는 괄호로 병기 —
기능 대응 관계의 기록이지 호환 목표가 아니다.

| 경로 | 우선순위 | 기능 |
|---|---|---|
| `/recent-changes` (`/RecentChanges`) | P1 | 최근 변경 — 로그 타입 필터(새 문서/삭제/이동/되돌림/복원) |
| `/random` (`/RandomPage`) | P1 | 임의 문서 |
| `/needed-pages` (`/NeededPages`) | P2 | 링크만 있고 없는 문서 |
| `/orphaned-pages` (`/OrphanedPages`) | P2 | 고립된 문서 |
| `/uncategorized-pages` (`/UncategorizedPages`) | P2 | 분류가 없는 문서 |
| `/old-pages` (`/OldPages`) | P2 | 편집된 지 오래된 문서 |
| `/shortest-pages` `/longest-pages` | P3 | 짧은/긴 문서 |
| `/recent-discussions` (`/RecentDiscuss`) | P2 | 최근 토론 — 상태 필터 |
| `/license` (`/License`) | P1 | 위키·엔진 라이선스 |
| `/block-history` (`/BlockHistory`) | P2 | 차단·권한 설정 공개 로그 |
| `/acl-groups` (`/aclgroup`) | P2 | ACLGroup 관리 |
| `/upload` (`/Upload`) | P2 | 파일 업로드 |

### 8. 운영

| 기능 | 우선순위 | 내용 |
|---|---|---|
| grant | P2 | perm 부여·회수 UI |
| 일괄 되돌리기 | P3 | 특정 사용자 기여 전체 되돌림(batch_revert) |
| 위키 설정 | P2 | 위키 이름·메인 문서·라이선스 등 전역 설정(config) |
| 감사 로그 | P2 | ACL·차단·권한 변경이 공개 로그에 남음 |
| 캡차 | P3 | 가입·편집 시. skip_captcha 권한으로 면제 |
| 로그인 내역 조회 | P3 | 다중 계정 검사(login_history) |

### 9. 프론트엔드·기타

| 기능 | 우선순위 | 내용 |
|---|---|---|
| 스킨 | P2 | 스킨 교체 체계. 1차는 기본 스킨 하나(backend-namuwiki의 CSS 어휘 위) |
| 다크모드 | P1 | 렌더러가 이미 `data-dark-style`로 지원 — 스킨이 토글 제공 |
| 목차·각주·접기 | P1 | 렌더러 완료분의 서빙 |
| API | P3 | the seed는 공개 API 없음(api_access perm의 봇용 내부 API만). 우리는 raw/render 정도의 읽기 API는 열어 둘 수 있음 — openNAMU가 선례 |

## 비기능 요구사항

- **자체 호스팅 1순위**: 단일 바이너리 + 파일 DB로 곧장 실행 가능해야 한다. 외부 서비스
  (검색엔진·캐시 서버) 의존은 선택 사항으로만 둔다. openNAMU가 채운 수요(소규모 개인·커뮤니티
  위키)와 같은 자리이되, the seed 동작 충실도는 이 프로젝트의 파서·렌더러 수준을 따른다.
- **성능**: 문서 보기는 렌더 결과 캐시로 상수 시간에 가깝게. 렌더러의 기존 기준
  (allocation_probe, termination_tests)을 서버 계층에서도 유지 — 악의적 입력(깊은 재귀,
  거대 표)에 무한루프·무한정 메모리 금지.
- **저장 무손실**: 원문은 바이트 그대로 보존(파서의 완전무손실 원칙과 동일). diff·blame은
  저장본에서 계산하고 원문을 변형하지 않는다.
- **라이선스 분리**: 엔진 코드 MIT. 문서 본문 라이선스(CC 계열)는 위키 설정으로 선언하고
  `/License`에 표시 — 코드와 콘텐츠의 라이선스를 섞지 않는다(픽스처에서 이미 확립한 원칙).
- **URL·웹 관례**: 경로는 일반적인 웹페이지 경험을 따른다 — 소문자 kebab-case,
  표준 HTTP 메서드 의미론(조회 GET, 변경 POST 후 303 리다이렉트), 올바른 상태 코드
  (403/404), 쿼리 파라미터 페이지네이션. the seed 특유의 표기(PascalCase 특수 페이지,
  `/Go`)는 채택하지 않는다. 문서 동작 경로의 동사 접두사(`/w/`, `/edit/`, `/history/`)는
  the seed와 겹치지만, 제목에 `/`가 허용되는 이상 접미사 방식이 모호해 관례로도 타당해
  유지한다(설계 근거는 docs/design/07의 URL 설계 절).
