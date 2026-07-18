# 설계: 위키 서버 데이터 모델

상태: 초안 (2026-07)

위키 서버가 영속화할 데이터 모델이다. 테이블은 docs/design/07의 소유 크레이트별로
묶는다 — 문서 구성이 코드 경계와 1:1이라 "어느 크레이트가 이 테이블을 소유하는가"에
모호함이 없다. SQLite·PostgreSQL 공통 부분집합으로 유지한다(타입 표기는 개념 수준이고
방언별 매핑은 마이그레이션에서 확정).

## 원칙

- **문서의 항등은 id, 제목은 속성이다.** 이동·맞바꾸기는 제목 컬럼 갱신일 뿐이고
  역사·토론·ACL은 id를 따라간다(the seed가 이동 후에도 역사를 유지하는 동작과 일치).
- **리비전은 append-only 전문(全文) 저장.** 원문 바이트 그대로 보존(파서의 무손실
  원칙). 삭제·이동·되돌리기도 리비전의 한 종류다 — 최근 변경·복원·감사가 전부
  리비전 조회로 환원된다.
- **내부 식별자와 외부 식별자는 완전히 분리한다.** 정수 PK와 외래키는 DB 내부
  전용이고 URL·HTML·API 어디에도 나가지 않는다. 외부 노출이 필요한 개체는 별도
  외부 식별자(UUID)나 자연 키(문서 제목, 사용자명)로만 참조한다. 내부 키를 자유롭게
  재구성할 수 있고, 순번 노출로 인한 열거·규모 추정을 막는다.
- **행위 주체는 actor로 통일.** 로그인 사용자와 IP 사용자를 한 타입으로 참조해
  리비전·토론·편집요청·aclgroup이 같은 외래키를 쓴다.
- **사용자는 항등과 부속을 분리하고, 부속은 다중을 기본으로 둔다.** `wiki_user`는
  항등(이름)만 갖고 인증 수단·이메일·개인 설정은 각자의 테이블이다. **인증 수단은
  여러 개**(보안키 여러 대, 비밀번호 + TOTP + OAuth 병행)이고 **이메일도 여러 개**
  (하나가 주 주소)일 수 있다 — 단일성이 필요한 것만 부분 유니크 인덱스로 제약한다
  (비밀번호 1개, 주 이메일 1개). 인증 방식 추가나 이메일 교체가 `wiki_user` 행과 그
  참조들을 건드리지 않는다.
- **상태를 지우지 않고 이력으로 쌓는다.** 권한 회수·차단 해제처럼 되돌릴 수 있는
  상태는 행 삭제가 아니라 revoked_at·removed_at으로 끝을 표시하고, 활성 여부는
  부분 유니크 인덱스가 보장한다 — 감사 로그가 원본 테이블에서 그대로 나온다.
- **파생 자료는 원본에서 재구성 가능해야 한다.** 역링크·검색 색인·렌더 캐시는 전부
  리비전에서 다시 만들 수 있다 — 스키마 변경·장애 복구 시 재구축이 탈출구다.
- **표준 SQL 컨벤션.** 테이블·컬럼은 snake_case 단수형. SQL 예약어는 회피한다
  (`user`→`wiki_user`, key/value 쌍→`name`/`data`, `position`→`evaluation_order`) —
  이식성을 위해 따옴표 이스케이프에 기대지 않는다. 타임스탬프는 **UTC 저장**
  (PostgreSQL `TIMESTAMPTZ`, SQLite는 ISO-8601 UTC). 외부 식별자 UUID는 **v4**
  (무작위 — 생성 순서·시각을 노출하지 않는다. 정렬·조회는 내부 정수 PK가 맡으므로
  외부 식별자에 인덱스 지역성은 필요 없다).

## 개체와 관계

```
actor ◀── revision ──▶ document ◀── document_reference (역링크)
  ▲           │            ▲ ▲
  │           ▼            │ └── thread ◀── thread_comment ──▶ actor
wiki_user file_revision    ├── edit_request ──▶ actor
  │                        ├── acl_rule (scope=document)
user_permission            └── render_cache
acl_group ◀── acl_group_member ──▶ actor
```

## wiki-document — 문서·리비전·역링크·파일·렌더 캐시

```sql
document
  id            INTEGER PK            -- 내부 전용. 외부 식별은 (namespace, title) 자연 키
  namespace     TEXT      -- 문서·틀·분류·파일·사용자·위키운영·휴지통
  title         TEXT      -- 이름공간 제외 이름, '/' 포함 가능
  created_at    TIMESTAMP
  UNIQUE (namespace, title)

revision
  id            INTEGER PK            -- 내부 전용
  external_id   UUID UNIQUE           -- 외부 노출 식별자 (?uuid=)
  document_id   INTEGER → document
  sequence      INTEGER               -- 문서 안 순번 (r1, r2, …)
  kind          TEXT                  -- create·edit·move·delete·restore·revert·import
  actor_id      INTEGER → actor
  content       TEXT NULL             -- 원문 전문. delete면 NULL
  comment       TEXT                  -- 편집·삭제 사유
  metadata      JSON NULL             -- move: {from, to} / revert: {to_revision} / import: {contributor}
  content_bytes INTEGER               -- 바이트 증감 표시용
  hidden        BOOLEAN               -- hide_revision
  created_at    TIMESTAMP
  UNIQUE (document_id, sequence)
  INDEX (created_at)                  -- /recent-changes
```

- "존재하는 문서" = 최신 리비전의 kind가 delete가 아닌 문서. 링크 존재 판정
  (`not-exist`)·`/needed-pages`·검색 색인이 이 정의를 공유한다.
- sequence는 트랜잭션 안에서 `max+1` 채번. SQLite는 단일 쓰기라 충돌이 없고,
  PostgreSQL은 UNIQUE 제약 충돌 시 재시도.
- 이동·맞바꾸기는 document.title 갱신 + kind=move 리비전 기록을 한 트랜잭션으로.
- 본문 압축(zstd)은 필요해지면 도입 — 스키마가 아니라 저장소 계층의 선택으로 남긴다.
- **덤프 임포트의 과거 기여자**는 단일 시스템 actor(아래 wiki-account)로 뭉갠다. 원
  덤프의 기여자명은 `metadata.contributor`에 보존해 표시용으로만 남기고, actor 모델은
  the seed와 같이 user/ip 둘로 유지한다.

```sql
document_reference
  source_document_id INTEGER → document
  target_namespace   TEXT
  target_title       TEXT              -- 대상은 없는 문서일 수 있어 id가 아닌 제목
  kind               TEXT              -- link·include·redirect·image·category
  PRIMARY KEY (source_document_id, target_namespace, target_title, kind)
  INDEX (target_namespace, target_title)

render_cache
  document_id   INTEGER PK → document  -- 문서당 한 행 (현재 리비전의 렌더 결과)
  revision_id   INTEGER → revision
  html          TEXT
  rendered_at   TIMESTAMP

star
  user_id       INTEGER → wiki_user   -- 구독자
  document_id   INTEGER → document    -- 구독한 문서
  created_at    TIMESTAMP
  PRIMARY KEY (user_id, document_id)
```

- 문서 저장 시 resolve 결과에서 재작성한다(그 문서의 행 전체 삭제 후 삽입).
- 분류 소속은 kind=category 행이고, `/needed-pages`는 target에 문서가 없는 link 행,
  `/orphaned-pages`는 target으로 등장하지 않는 문서 — 전부 이 테이블의 조회다.
- 대상 문서가 생기거나 삭제되면 `INDEX (target_namespace, target_title)`로 역참조해
  해당 source들의 render_cache만 무효화한다. include·redirect 대상의 내용 변경도 동일.
- 검색 색인(tantivy)은 DB 밖 파일이다 — 리비전 저장 시 동기 갱신(M1), 전체 재색인
  명령을 항상 제공.
- `star`(문서 구독)는 사용자 기능이지만 wiki-document가 소유한다 — wiki-account에 두면
  account → document 참조가 생겨 document → account(리비전의 actor)와 순환한다.
  document → account 방향은 이미 성립하므로 여기서는 순환이 없다.

```sql
file_content
  hash          TEXT PK               -- sha256, 저장 경로 키 (내용 중복 제거)
  media_type    TEXT
  byte_size     INTEGER
  width         INTEGER NULL
  height        INTEGER NULL
  created_at    TIMESTAMP

file_revision
  revision_id   INTEGER PK → revision -- 파일 이름공간 문서의 리비전에 1:1
  content_hash  TEXT → file_content
  license       TEXT                  -- 업로드 시 필수 선택
```

- 파일은 "파일 이름공간 문서 + 바이너리"다. 업로드가 리비전을 만들고(설명·분류가
  본문), 바이너리는 해시로 별도 저장 — 문서 모델의 역사·ACL·토론이 파일에도 그대로
  적용된다. 문서 보기는 `/w/파일:이름`, 바이너리 서빙은 `/file/<파일명>`(현재 리비전의
  content_hash로 조회) — 렌더러 `WikiContext`의 파일 URL 훅이 이 경로를 돌려준다.
- `file_content`는 내용 주소(해시)라 external_id가 필요 없다 — 해시 자체가 노출돼도
  열거·규모 추정 정보가 아니다.

## wiki-account — 행위 주체·인증·알림

```sql
actor
  id            INTEGER PK            -- 내부 전용
  user_id       INTEGER NULL → wiki_user  -- 로그인 사용자
  ip_address    TEXT NULL             -- IP 사용자 (정규화 표기)
  CHECK (user_id와 ip_address 중 정확히 하나)
  UNIQUE (user_id), UNIQUE (ip_address)

wiki_user
  id            INTEGER PK            -- 내부 전용
  external_id   UUID UNIQUE           -- 이름 변경(30일 1회)에도 안정적인 외부 식별자
  name          TEXT UNIQUE           -- 표시·URL 식별자, '/' 불허
  kind          TEXT                  -- normal·system (system: 덤프 임포트 등 기계 주체)
  created_at    TIMESTAMP

user_credential
  id            INTEGER PK            -- 내부 전용
  user_id       INTEGER → wiki_user
  kind          TEXT                  -- password·totp·passkey·oauth
  label         TEXT NULL             -- 사용자가 붙인 이름 ("업무용 보안키")
  identifier    TEXT NULL             -- passkey: credential id / oauth: 제공자+subject
  secret        TEXT                  -- password: argon2 해시 / totp: 시크릿 / passkey: 공개키
  created_at    TIMESTAMP
  last_used_at  TIMESTAMP NULL
  UNIQUE (kind, identifier)                 -- 같은 보안키·외부 계정이 두 사용자에 붙지 않게
  UNIQUE (user_id) WHERE kind = 'password'  -- 비밀번호만 사용자당 하나

user_email
  id            INTEGER PK            -- 내부 전용
  user_id       INTEGER → wiki_user
  email         TEXT UNIQUE
  verified_at   TIMESTAMP NULL        -- 가입·기기 인증 흐름의 상태
  is_primary    BOOLEAN               -- 알림·계정 복구 수신 주소
  created_at    TIMESTAMP
  UNIQUE (user_id) WHERE is_primary   -- 주 주소는 하나

user_preference
  user_id       INTEGER → wiki_user
  name          TEXT                  -- 설정 키 (스킨·표시 등)
  data          TEXT                  -- 설정 값
  PRIMARY KEY (user_id, name)

notification
  id            INTEGER PK
  user_id       INTEGER → wiki_user
  kind          TEXT
  payload       JSON                  -- 문서 참조는 FK가 아닌 제목으로 (역방향 의존 회피)
  read_at       TIMESTAMP NULL
  created_at    TIMESTAMP
```

- **시스템 사용자**: 덤프 임포트·자동 편집의 주체는 `kind=system` wiki_user 하나다.
  credential이 없어 로그인할 수 없고, 그 actor가 임포트 리비전의 actor_id가 된다.
- wiki-account는 **다른 크레이트를 참조하지 않는다** — 의존 그래프의 뿌리다. 문서
  구독(star)이 여기 있으면 account → document 참조가 생겨 document → account(리비전의
  actor)와 순환하므로, star는 wiki-document가 소유한다(위).

## wiki-authorization — ACL·aclgroup·perm

```sql
acl_rule
  id             INTEGER PK
  document_id    INTEGER NULL → document  -- NULL이면 이름공간 규칙
  namespace      TEXT NULL
  action         TEXT       -- read·edit·move·delete·create_thread·… (docs/design/06)
  evaluation_order INTEGER  -- 평가 순서 (문서 → 이름공간 → 기본 거부)
  condition_kind TEXT       -- perm·user·ip·geoip·aclgroup
  condition_value TEXT      -- any·member·… / 사용자명 / CIDR / 국가코드 / 그룹명
  decision       TEXT       -- allow·deny
  CHECK (document_id와 namespace 중 정확히 하나)

acl_group
  id            INTEGER PK            -- 내부 전용. 외부 식별은 name
  name          TEXT UNIQUE
  created_at    TIMESTAMP

acl_group_member
  id            INTEGER PK
  group_id      INTEGER → acl_group
  actor_id      INTEGER NULL → actor  -- 사용자 또는 단일 IP
  ip_range      TEXT NULL             -- CIDR
  CHECK (actor_id와 ip_range 중 정확히 하나)
  reason        TEXT
  expires_at    TIMESTAMP NULL
  added_by      INTEGER → actor
  created_at    TIMESTAMP
  removed_at    TIMESTAMP NULL        -- 해제도 기록으로 남김
  removed_by    INTEGER NULL → actor

user_permission
  id            INTEGER PK            -- 내부 전용
  user_id       INTEGER → wiki_user
  permission    TEXT                  -- admin·grant·aclgroup·nsacl·… (docs/design/06)
  granted_by    INTEGER → actor
  created_at    TIMESTAMP
  revoked_at    TIMESTAMP NULL        -- 회수도 기록으로 남김
  revoked_by    INTEGER NULL → actor
  UNIQUE (user_id, permission) WHERE revoked_at IS NULL  -- 활성 권한은 하나
```

- `/block-history`는 acl_group_member의 추가·해제와 user_permission의 부여·회수를
  시간순으로 합친 조회다 — 별도 로그 테이블 없이 **원본 행이 곧 감사 기록**이 되도록
  둘 다 삭제 대신 removed_at·revoked_at을 쓴다. 활성 상태는 부분 유니크 인덱스가
  보장하므로(같은 권한을 회수 후 재부여해도 이력이 쌓인다) 감사 요구(docs/design/06
  운영 8)와 정합한다.
- authorization은 document(scope)·account(actor·wiki_user)를 참조한다 — 방향은
  authorization → document·account. 순환 없음.

## wiki-discussion — 토론·편집요청

```sql
thread
  id            INTEGER PK            -- 내부 전용
  external_id   UUID UNIQUE           -- /thread/<uuid>
  document_id   INTEGER → document
  topic         TEXT
  status        TEXT                  -- normal·pause·close
  created_at    TIMESTAMP

thread_comment
  id            INTEGER PK
  thread_id     INTEGER → thread
  sequence      INTEGER               -- #번호
  kind          TEXT                  -- comment·status_change·topic_change·document_move
  actor_id      INTEGER → actor
  content       TEXT                  -- kind=comment의 본문
  metadata      JSON NULL             -- 관리 조작의 값: {to: 'close'} / {topic: '…'} / {document: '…'}
  admin_marked  BOOLEAN               -- [ADMIN] 발언
  hidden_at     TIMESTAMP NULL        -- 블라인드
  hidden_by     INTEGER NULL → actor
  created_at    TIMESTAMP
  UNIQUE (thread_id, sequence)

edit_request
  id               INTEGER PK         -- 내부 전용
  external_id      UUID UNIQUE        -- /edit-request/<uuid>
  document_id      INTEGER → document
  base_revision_id INTEGER NULL → revision  -- 제안 기준 리비전 (새 문서면 NULL)
  actor_id         INTEGER → actor
  content          TEXT
  comment          TEXT
  status           TEXT               -- open·accepted·closed
  reviewed_by      INTEGER NULL → actor
  created_at       TIMESTAMP
```

- 상태 변경·주제 변경·스레드 이동을 thread_comment의 kind로 스레드 안에 남기는 것은
  the seed의 표시 방식(스레드 타임라인에 관리 조작이 끼어듦)과 같다. 관리 조작의
  **바뀐 값은 content 문자열이 아니라 metadata**에 둔다 — content를 파싱해 상태를
  복원하는 구조를 만들지 않는다. `thread.status`는 그 이력의 현재 값(파생)이다.

## wiki-server — 전역 설정·세션

```sql
site_setting
  name          TEXT PK               -- 설정 키 (위키 이름·메인 문서·라이선스·가입 정책)
  data          TEXT                  -- 설정 값
```

세션은 tower-sessions의 저장소(DB 테이블)를 그대로 쓴다 — 스키마는 그 크레이트가 관리.

## 모델링 판단 (정론성 검토)

값 집합·다형 데이터·KV처럼 정론이 갈리는 자리마다 어느 쪽을 왜 택했는지 남긴다.

- **고정 집합 문자열(namespace·kind·status·action·permission)**: 참조 테이블 대신
  TEXT + 코드 enum. 값이 **코드의 분기와 1:1**이라(이름공간마다 ACL 기본값·렌더 취급이
  다르다) DB에 행으로 두면 두 곳이 어긋난다. 운영자가 늘릴 수 있어야 하는 것은
  이름공간뿐인데 그조차 코드 지원이 필요하므로 설정(site_setting) + 코드로 간다.
  DB 제약은 CHECK로 건다.
- **다형 데이터(revision.metadata·thread_comment.metadata·notification.payload)**:
  kind별 별도 테이블 대신 JSON. 조인해서 질의할 일이 없고(표시 시점에 그 행과 함께
  읽는다) kind가 늘 때 테이블이 늘지 않는다. 대신 **질의·정렬에 쓰는 값은 JSON에 두지
  않는다** — `content_bytes`·`created_at`처럼 목록 화면이 쓰는 것은 컬럼이다.
- **KV 테이블(user_preference·site_setting)**: EAV는 일반적으로 안티패턴이지만, 여기
  값들은 서로 관계가 없고 조인·집계 대상이 아니며 스키마 진화가 잦은 **설정**이다.
  값이 관계를 갖기 시작하면(예: 스킨별 옵션 묶음) 그때 정규 테이블로 뺀다.
- **파생값 저장(revision.content_bytes·thread.status)**: 원본에서 계산 가능하지만
  목록 화면이 매번 전문을 읽거나 이력을 접어야 하므로 저장한다. 원칙의 "파생 자료는
  재구성 가능"을 만족하므로 불일치 시 원본이 정답이다.
- **file_revision.license TEXT**: 라이선스 목록은 위키 설정이 정하고(the seed도 업로드
  시 선택지를 위키가 제공) 저장은 선택된 식별자 문자열이다 — 참조 테이블은 설정과
  이중 관리가 된다.
- **acl_rule의 condition_kind/condition_value**: 태그드 유니온을 두 컬럼으로 편 형태.
  조건 종류마다 컬럼을 나누면(user_name·ip_range·country…) 대부분 NULL인 넓은 테이블이
  된다. 평가기가 kind로 분기해 value를 해석하는 것이 코드와도 1:1이다.

## 검토한 대안

- **리비전 delta 저장**(the seed 추정 방식은 불명): 전문 저장 대비 용량은 줄지만
  임의 리비전 열람·blame·복구가 체인 재생에 묶인다. 나무위키 덤프 전체도 수 GB
  수준이라 전문 + (필요시) 압축이 단순하고 안전하다.
- **덤프 기여자를 actor 세 번째 변형(imported)으로**: actor에 이름만 있는 종류를 더해
  기여 이력을 정확히 보존할 수 있으나 the seed actor 모델(user/ip)에서 벗어난다.
  기여자명은 표시 정보일 뿐이라 metadata 보존으로 충분해, 시스템 actor로 뭉개는 쪽을
  택했다(사용자 결정).
- **분류 전용 테이블**: document_reference의 kind로 충분하고, resolve가 링크와 분류를
  같은 pass에서 수집하므로 쓰기 경로도 하나다. 분류별 정렬 키 등 요구가 생기면 분리.
- **차단 전용 테이블**: the seed와 같이 aclgroup으로 일원화(docs/design/07). 전용
  테이블은 두 시스템의 정합성 문제를 만든다.
- **openNAMU식 key-value 스키마**(text 컬럼 위주, 외래키 없음): 마이그레이션은 쉽지만
  정합성을 코드가 떠안는다. 관계·제약을 스키마에 두는 쪽을 택한다.
