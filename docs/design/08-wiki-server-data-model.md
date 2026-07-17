# 설계: 위키 서버 데이터 모델

상태: 초안 (2026-07)

docs/design/07의 `wiki-storage`가 영속화할 데이터 모델이다. SQLite·PostgreSQL 공통
부분집합으로 유지한다(타입 표기는 개념 수준이고 방언별 매핑은 마이그레이션에서 확정).

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
- **사용자는 항등과 부속을 분리한다.** user는 항등(이름)만 갖고, 인증 수단·이메일·
  개인 설정은 각자의 테이블로 — 인증 방식 추가(passkey·OAuth)나 이메일 교체가
  user 행과 그 참조들을 건드리지 않는다.
- **파생 자료는 원본에서 재구성 가능해야 한다.** 역링크·검색 색인·렌더 캐시는 전부
  리비전에서 다시 만들 수 있다 — 스키마 변경·장애 복구 시 재구축이 탈출구다.

## 개체와 관계

```
actor ◀── revision ──▶ document ◀── document_reference (역링크)
  ▲           │            ▲ ▲
  │           ▼            │ └── thread ◀── thread_comment ──▶ actor
user      file_content     ├── edit_request ──▶ actor
  │                        ├── acl_rule (scope=document)
user_permission            └── render_cache
acl_group ◀── acl_group_member ──▶ actor
```

## 테이블

### 문서·리비전

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
  kind          TEXT                  -- create·edit·move·delete·restore·revert
  actor_id      INTEGER → actor
  content       TEXT NULL             -- 원문 전문. delete면 NULL
  comment       TEXT                  -- 편집·삭제 사유
  metadata      JSON NULL             -- move: {from, to} / revert: {to_revision}
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

### 행위 주체

```sql
actor
  id            INTEGER PK            -- 내부 전용
  user_id       INTEGER NULL → user   -- 로그인 사용자
  ip_address    TEXT NULL             -- IP 사용자 (정규화 표기)
  CHECK (둘 중 정확히 하나)
  UNIQUE (user_id), UNIQUE (ip_address)

user
  id            INTEGER PK            -- 내부 전용
  external_id   UUID UNIQUE           -- 이름 변경(30일 1회)에도 안정적인 외부 식별자
  name          TEXT UNIQUE           -- 표시·URL 식별자, '/' 불허
  created_at    TIMESTAMP

user_credential
  user_id       INTEGER → user
  kind          TEXT                  -- password·totp (추후 passkey·oauth)
  secret        TEXT                  -- password: argon2 해시 / totp: 시크릿
  updated_at    TIMESTAMP
  PRIMARY KEY (user_id, kind)

user_email
  user_id       INTEGER → user
  email         TEXT UNIQUE
  verified_at   TIMESTAMP NULL        -- 가입·기기 인증 흐름의 상태

user_preference
  user_id       INTEGER → user
  key           TEXT                  -- 스킨·표시 설정 등
  value         TEXT
  PRIMARY KEY (user_id, key)

user_permission
  user_id       INTEGER → user
  permission    TEXT                  -- admin·grant·aclgroup·nsacl·… (docs/design/06)
  granted_by    INTEGER → actor
  created_at    TIMESTAMP
  PRIMARY KEY (user_id, permission)
```

### 권한

```sql
acl_rule
  id             INTEGER PK
  document_id    INTEGER NULL → document  -- NULL이면 이름공간 규칙
  namespace      TEXT NULL
  action         TEXT       -- read·edit·move·delete·create_thread·… (docs/design/06)
  position       INTEGER    -- 평가 순서 (문서 → 이름공간 → 기본 거부)
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
  reason        TEXT
  expires_at    TIMESTAMP NULL
  added_by      INTEGER → actor
  created_at    TIMESTAMP
  removed_at    TIMESTAMP NULL        -- 해제도 기록으로 남김
  removed_by    INTEGER NULL → actor
```

`/block-history`는 acl_group_member의 추가·해제와 user_permission 변경을 시간순으로
합친 조회다 — 별도 로그 테이블을 두지 않고 원본 행이 곧 기록이 되게 삭제 대신
removed_at을 쓴다.

### 토론·편집요청

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
  content       TEXT
  admin_marked  BOOLEAN               -- [ADMIN] 발언
  hidden_by     INTEGER NULL → actor  -- 블라인드
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

상태 변경·주제 변경·스레드 이동을 thread_comment의 kind로 스레드 안에 남기는 것은
the seed의 표시 방식(스레드 타임라인에 관리 조작이 끼어듦)과 같다.

### 역링크·파생 자료

```sql
document_reference
  source_document_id INTEGER → document
  target_namespace   TEXT
  target_title       TEXT              -- 대상은 없는 문서일 수 있어 id가 아닌 제목
  kind               TEXT              -- link·include·redirect·image·category
  PRIMARY KEY (source_document_id, target_namespace, target_title, kind)
  INDEX (target_namespace, target_title)

render_cache
  document_id   INTEGER → document
  revision_id   INTEGER → revision
  html          TEXT
  rendered_at   TIMESTAMP
```

- 문서 저장 시 resolve 결과에서 재작성한다(그 문서의 행 전체 삭제 후 삽입).
- 분류 소속은 kind=category 행이고, `/needed-pages`는 target에 문서가 없는 link 행,
  `/orphaned-pages`는 target으로 등장하지 않는 문서 — 전부 이 테이블의 조회다.
- 대상 문서가 생기거나 삭제되면 `INDEX (target_namespace, target_title)`로 역참조해
  해당 source들의 render_cache만 무효화한다. include·redirect 대상의 내용 변경도 동일.
- 검색 색인(tantivy)은 DB 밖 파일이다 — 리비전 저장 시 동기 갱신(M1), 전체 재색인
  명령을 항상 제공.

### 파일

```sql
file_content
  hash          TEXT PK               -- sha256, 저장 경로 키 (내용 중복 제거)
  media_type    TEXT
  byte_size     INTEGER
  width         INTEGER NULL
  height        INTEGER NULL

file_revision
  revision_id   INTEGER PK → revision -- 파일 이름공간 문서의 리비전에 1:1
  content_hash  TEXT → file_content
  license       TEXT                  -- 업로드 시 필수 선택
```

파일은 "파일 이름공간 문서 + 바이너리"다. 업로드가 리비전을 만들고(설명·분류가 본문),
바이너리는 해시로 별도 저장 — 문서 모델의 역사·ACL·토론이 파일에도 그대로 적용된다.

### 기타

```sql
notification: id, user_id, kind, payload JSON, read_at NULL, created_at
star:         user_id, document_id, created_at  PK(user_id, document_id)
site_setting: key TEXT PK, value TEXT           -- 위키 이름·메인 문서·라이선스·가입 정책
```

세션은 tower-sessions의 저장소(DB 테이블)를 그대로 쓴다.

## 검토한 대안

- **리비전 delta 저장**(the seed 추정 방식은 불명): 전문 저장 대비 용량은 줄지만
  임의 리비전 열람·blame·복구가 체인 재생에 묶인다. 나무위키 덤프 전체도 수 GB
  수준이라 전문 + (필요시) 압축이 단순하고 안전하다.
- **분류 전용 테이블**: document_reference의 kind로 충분하고, resolve가 링크와 분류를
  같은 pass에서 수집하므로 쓰기 경로도 하나다. 분류별 정렬 키 등 요구가 생기면 분리.
- **차단 전용 테이블**: the seed와 같이 aclgroup으로 일원화(docs/design/07). 전용
  테이블은 두 시스템의 정합성 문제를 만든다.
- **openNAMU식 key-value 스키마**(text 컬럼 위주, 외래키 없음): 마이그레이션은 쉽지만
  정합성을 코드가 떠안는다. 관계·제약을 스키마에 두는 쪽을 택한다.
