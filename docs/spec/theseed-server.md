# the seed 서버 기능 스펙 — 페이지와 동작

나무마크 문법이 아니라 **엔진이 제공하는 페이지와 기능**의 사실을 모읍니다. 문법의 정본이
[namumark.md](namumark.md)이듯, 서버 표면의 정본은 이 문서입니다.

## 근거 등급

문법 스펙과 같은 체계를 씁니다.

| 등급 | 뜻 |
|---|---|
| **실측** | 실제 요청·응답에서 직접 확인. 상태 코드·`<title>`·페이지 내 링크·설정 블롭. |
| **문서** | 알파위키/나무위키 도움말, theseed.io 공개 API 문서의 서술. |
| **추정** | 간접 근거뿐. 구현 근거로 단독 사용 금지. |

조사는 **읽기(GET)만** 수행했고 **비로그인 상태**였습니다. 따라서 인증이 필요한 화면은
"존재한다"까지만 확정되고 내부 UI·폼 파라미터는 미확인입니다. 판정 규칙: `404`=라우트 없음,
`200`=존재, `403`=존재하나 권한 부족, `302`=존재하나 로그인 필요.

조사 시점 엔진 버전은 알파위키 인스턴스 기준 **5.1.0.1094** (`/License` 노출, 실측).
나무위키·알파위키·더시드위키 셋의 라우트 구조는 완전히 동일했습니다(실측).

---

## 1. 문서 동작

접두사 + 문서명 형태. 문서명은 URL 인코딩.

| 경로 | 기능 | 근거 |
|---|---|---|
| `/w/<문서>` | 읽기 | 실측 |
| `/raw/<문서>` | 원문 | 실측 |
| `/edit/<문서>` | 편집 | 실측 |
| `/history/<문서>` | 역사 | 실측 |
| `/activity/<문서>` | **활동** — 역사와 토론을 한 타임라인으로. 5.0.0에서 추가 | 실측 |
| `/diff/<문서>` | 비교 | 실측 |
| `/blame/<문서>` | 줄별 기여자 | 실측 |
| `/revert/<문서>` | 되돌리기 | 실측 |
| `/delete/<문서>` | 삭제 | 실측(403) |
| `/move/<문서>` | 이동 | 실측(403) |
| `/acl/<문서>` | ACL 조회·설정 | 실측 |
| `/backlink/<문서>` | 역링크 | 실측 |
| `/discuss/<문서>` | 문서 토론 목록 | 실측 |
| `/new_edit_request/<문서>` | 편집 요청 생성 | 실측 |
| `/member/star/<문서>`, `/member/unstar/<문서>` | 내 문서함 넣기·빼기 | 실측(302) |

`/edit_request/<문서>`는 404입니다 — 편집 요청은 문서가 아니라 슬러그로 주소가 잡힙니다(실측).

### 리비전 지정

**리비전 선택자는 `?uuid=<UUID>` 하나뿐입니다**(실측). 리비전은 표시용 정수 번호(r245)와
UUID를 함께 갖지만 URL에서는 UUID만 유효하고, `?rev=10`은 **무시되어 최신판이 나옵니다**.
`/w/`·`/raw/`·`/revert/`·`/blame/`·`/diff/`가 모두 받습니다.

### 그 밖의 파라미터

- `/edit/<문서>?section=<N>` — **문단 단위 편집**. 응답 크기가 절 별로 달라지는 것으로 확인(실측).
- `/history/<문서>?until=<uuid>` / `?from=<uuid>` — 페이지네이션(실측).
- `/activity/<문서>?from=<uuid>` (실측).
- `/backlink/<문서>?namespace=&from=&until=` (문서 — 공개 API와 같은 파라미터).
- `/discuss/<문서>?state=close` / `?state=closed_edit_requests` (실측).

---

## 2. 토론과 편집 요청

### 슬러그 주소 체계

스레드와 편집 요청은 **정수 id가 아니라 영단어 슬러그**로 주소가 잡힙니다(실측).

```
/thread/DeliciousHolisticCageyDrawer
/thread/AnUnaccountableAndTinyPlayground
/edit_request/AnUnaccountableAndTinyPlayground
```

형식은 `형용사+형용사+형용사+명사` CamelCase, 또는 `The/A/An…And…` 꼴입니다. 공개 API의
`/api/discuss/<문서>` 응답 `slug` 필드가 이 URL과 일치합니다(실측 대조).

### 상태와 관리 조작

스레드 상태는 `normal` / `close` / `pause` 셋(문서). 관리 조작마다 별도 권한이 붙습니다(문서):
상태 변경 `update_thread_status`, 주제 변경 `update_thread_topic`, **다른 문서로 이동**
`update_thread_document`, 댓글 숨김 `hide_thread_comment`.

### 편집 요청 흐름 (문서)

편집 권한이 없는 문서에서 편집을 시도하면 자동으로 편집 요청이 만들어집니다. 편집 권한이
있어도 `/new_edit_request/`로 강제 생성할 수 있습니다. 승인(Accept)은 문서 편집 권한자,
닫기(Close)는 편집 권한자와 요청자, **수정(Edit)은 요청자 본인만** — 운영진도 남의 편집
요청 내용은 고치지 못합니다.

### 최근 토론 필터

`/RecentDiscuss?logtype=` 8종(실측): `normal_thread` · `closed_thread` · `pause_thread` ·
`old_thread` · `open_editrequest` · `accepted_editrequest` · `closed_editrequest` ·
`old_editrequest`.

---

## 3. 특수 기능

**`/Special` 같은 인덱스 페이지는 없습니다**(404, 실측). 각 기능이 독립된 최상위 라우트입니다.

| 경로 | 기능 |
|---|---|
| `/RecentChanges` | 최근 변경 |
| `/RecentDiscuss` | 최근 토론 |
| `/Search` | 검색 |
| `/Go` | 바로가기 — 제목이 정확히 맞으면 문서로 302 |
| `/random` | 임의 문서로 즉시 이동(302) |
| `/RandomPage` | 이름공간별 임의 문서 20건 나열(200) |
| `/NeededPages` | 작성이 필요한 문서 |
| `/OrphanedPages` | 고립된 문서 |
| `/UncategorizedPages` | 분류가 없는 문서 |
| `/OldPages` | 오래된 문서 |
| `/ShortestPages`, `/LongestPages` | 길이순 |
| `/BlockHistory` | 차단·운영 기록 |
| `/Upload` | 파일 올리기 |
| `/License` | 오픈소스 라이선스 + 엔진 버전 |
| `/aclgroup` | ACL 그룹 관리 |
| `/contribution/<uuid>` | 기여 목록 |
| `/opensearch.xml` | OpenSearch 디스크립터 |
| `/sidebar.json` | 최근 토론 15건 JSON, **인증 불필요** |

모두 실측. `/random`(즉시 이동)과 `/RandomPage`(목록)가 **다른 기능**인 점에 주의.

### 404로 확인된 비존재 라우트 (실측)

`/Special` · `/NeedPages` · `/Login` · `/Register` · `/Logout` · `/Namespace` ·
`/TitleIndex` · `/AllPages` · `/Star` · `/EditRequests` · `/ChangeSkin` · `/Notifications` ·
`/Statistics` · `/FileList` · `/NewPages` · `/UnusedFiles` · `/WantedPages` · `/rss` ·
`/feed.xml` · `/sitemap.xml`

즉 **RSS/Atom·사이트맵·통계·전체 문서 목록은 엔진이 제공하지 않습니다**. 알림도 전용
페이지가 없습니다(스킨이 그리는 것으로 추정).

### 파라미터

- `/RecentChanges?logtype=` 6종(실측): `all` · `create` · `delete` · `move` · `revert` · `recover`
- `/Search?q=&target=&namespace=&page=` — `target`은 `title` / `content` /
  `title_content`(기본, 잘못된 값도 여기로 폴백). 결과에 `<span class="search-highlight">`
  하이라이트 포함(실측).
- `/BlockHistory?from=<uuid>`, `?query=&target=` (실측/문서).
- `/RandomPage`·`/OldPages`·`/ShortestPages` 등은 `namespace=`를 받음(실측).
- `/opensearch.xml`이 가리키는 검색 템플릿은 `/Go?q={searchTerms}`입니다(실측).

### 기여 목록

현행 형식은 `/contribution/<사용자 UUID>?type=<종류>&from=<uuid>`이고, `type` 10종을
탭으로 제공합니다(실측): `document` · `discuss` · `create` · `delete` · `edit` ·
`edit_request` · `move` · `recover` · `revert` · `thread`.

> 도움말이 안내하는 `/contribution/author/<uuid>/document`, `/contribution/ip/<ip>` 형식은
> 현재 404입니다(실측). 해당 도움말 문서에 "5.0.0 개편 반영 필요" 틀이 붙어 있어 구버전
> URL로 판단됩니다.

---

## 4. 계정

모두 `/member/` 아래에 있습니다(실측, 세 사이트 공통).

| 경로 | 기능 |
|---|---|
| `/member/login` | 로그인 |
| `/member/signup` | 가입 — 이메일 입력 |
| `/member/signup/<토큰>` | 인증 링크 → 이름·비밀번호 설정 |
| `/member/signup_verify` | 모바일(전화) 인증 |
| `/member/recover_password` | 비밀번호 찾기 |
| `/member/logout` | 로그아웃 |
| `/member/mypage` | 내 정보 |
| `/member/change_password` | 비밀번호 변경 |
| `/member/change_email` | 이메일 변경 |
| `/member/change_name` | 계정명 변경 (30일 주기, 문서) |
| `/member/withdraw` | 탈퇴 |
| `/member/starred_documents` | 내 문서함 |

**가입**(문서): 이메일 입력 → 24시간 유효한 인증 메일(요청 IP 표기) → 링크에서 이름과
비밀번호 설정. **로그인**(문서): 미확인 기기에서는 이메일로 온 인증 코드를 추가로 요구합니다.

**2FA(TOTP)와 API 토큰 발급 UI의 경로는 확인하지 못했습니다.** 후보 URL이 전부 404였으나
로그인 후에만 노출될 가능성이 있습니다(추정). 다만 삭제된 권한 목록에
`disable_two_factor_login`이 있어 2단계 인증이 존재했음은 확실합니다(실측).

---

## 5. 권한

알파위키 `the seed/권한` 문서 원문에서 추출(실측).

**현존 21종**: `grant` · `login_history` · `delete_thread` · `nsacl` · `admin` · `aclgroup` ·
`manage_thread` · `api_access` · `no_force_captcha` · `batch_revert` · `mark_troll_revision` ·
`aclgroup_hidelog` · `hideip` · `delete_edit_request` · `hide_document_history_log` ·
`hide_revision` · `developer` · `edit_protected_file` · `skip_captcha` · `config` ·
`lock_document`

**삭제된 것**: `update_thread` · `ipacl` · `suspend_account` · `disable_two_factor_login` ·
`editable_other_user_document` · `member_info` · `acl`

도움말에 나오는 스레드 세부 권한(`hide_thread_comment`, `update_thread_status`,
`update_thread_topic`, `update_thread_document`)이 `manage_thread`의 하위인지 별도인지는
확정하지 못했습니다(추정).

관리자 화면은 넷뿐이고 모두 단일 페이지 + 폼입니다(하위 경로는 404, 실측):
`/admin/config`(권한 `config`) · `/admin/grant`(`grant`) ·
`/admin/login_history`(`login_history`) · `/admin/batch_revert`(`batch_revert`).

---

## 6. ACL

### 동작 8종 (실측 — `/acl/` 응답에서 직접 추출)

`read` · `edit` · `move` · `delete` · `create_thread` · `write_thread_comment` ·
`edit_request` · `acl`

### 규칙 문법

`/RecentChanges` 로그 항목에 규칙 원문이 그대로 노출됩니다(실측).

```
insert,edit,gotons,perm:admin
delete,edit,allow,perm:admin
```

| 자리 | 값 |
|---|---|
| 조작 | `insert`(설정) / `delete`(해제) |
| 동작 | 위 8종 |
| 판정 | `allow` / `deny` / **`gotons`** |
| 조건 | 아래 대상 지정자 |

`gotons`는 "이름공간 ACL을 실행하라"는 뜻입니다 — 해당 대상은 이름공간 규칙을 통과하면
허용되므로, **이름공간 ACL을 문서 ACL과 같은 우선순위로 끌어올리는 장치**입니다(문서).

### 조건 접두사

- `perm:<권한>` — 실측 확인 값: `any`, `member`, `member_signup_15days_ago`, `admin`, `ip`,
  `contributor`, `document_contributor`, `match_username_and_document_title`,
  `mobile_verified_member`, `auto_verified_member`, `api_access`, `developer`, `bot`
- `aclgroup:<그룹명>` — 실측: `차단된 사용자`, `경고`, `주의`, `로그인 허용 차단`,
  `통신사 아이피`, `IDC`
- `member:<사용자명>`
- `ip:<주소>` — **단일 IP만, 대역 미지원**(문서)
- `geoip:<ISO 국가코드>` — `KR`, `US`, `JP`…

UI에는 기간(Duration) 칸이 있어 영구/기간 지정이 됩니다(문서).

### 평가 규칙 (문서)

- 규칙은 **위에서부터 순차 평가, 먼저 맞는 규칙이 적용**. 그래서 `perm:any allow`는 반드시
  맨 아래에 두어야 합니다.
- 문서 ACL이 이름공간 ACL보다 우선.
- **`read`는 개별 문서에서 조정 불가** — 이름공간 단위로만 설정합니다(나무위키 2019-12-16,
  알파위키 2020-03-01부터).
- `read`가 거부되면 ACL 확인을 뺀 모든 동작이 거부됩니다.
- `edit`가 거부되면 `move`·`delete`도 자동 거부됩니다.
- `acl` 동작의 조정은 `nsacl` 권한자만 가능합니다.

### 실제 예 (실측 — 알파위키 `the seed` 문서)

```
read:   perm:any allow
edit:   aclgroup:차단된 사용자 deny → aclgroup:경고 deny → aclgroup:주의 deny
      → perm:member allow → aclgroup:로그인 허용 차단 deny
      → aclgroup:통신사 아이피 deny → aclgroup:IDC deny → perm:any allow
move:   perm:member_signup_15days_ago allow
delete: perm:member_signup_15days_ago allow
acl:    perm:admin allow
```

차단이 별도 테이블이 아니라 **ACL 그룹 소속으로 구현**된다는 점이 드러납니다.

### ACL 그룹과 차단

`/aclgroup`에서 그룹 생성·삭제, 사용자/IP 추가·제거, 메모·기간 지정을 합니다(실측+문서).
최근변경·토론·역사에서 사용자명을 누르면 뜨는 팝업(기여내역 / 사용자 문서 / 차단)이
**빠른 차단** 경로이고, 사유가 `문서:알파위키 r1 긴급차단` 꼴로 자동 입력됩니다(문서).
그룹을 지우면 소속 사용자가 전원 빠지고 같은 이름으로 다시 만들어도 복구되지 않습니다(문서).

`/BlockHistory`에 남는 기록 종류(실측): ACL 그룹 추가·제거, 일괄 되돌리기, 사용자 권한
부여·회수, 로그인 내역 조회.

---

## 7. 이름공간

`/RandomPage` 응답의 드롭다운에서 알파위키 기준 22종이 노출됩니다(실측).

```
문서 · 틀 · 분류 · 파일 · 사용자 · 특수기능 · 알파위키 · 토론 · 휴지통 · 투표
· 나무파일 · 집단창작 · 임시조치 · 파일휴지통 · 템플릿 · 보존문서 · 시스템
· 위키운영 · 특정판 · 삭제된사용자 · 아이피사용자 · 편집필터
```

엔진 기본으로 보이는 것과 위키 커스텀의 구분은 **추정**입니다. 엔진 쪽으로 보이는 것:
`문서`(기본) · `틀` · `분류` · `파일` · `사용자` · `특수기능` · `토론` · `휴지통` ·
`파일휴지통` · `삭제된사용자` · `아이피사용자` · `특정판` · `편집필터`.

읽기 제한이 이름공간 단위로 걸린다는 것이 실측됩니다 — `/RandomPage?namespace=휴지통`과
`?namespace=임시조치`가 비로그인에서 **403**입니다.

---

## 8. 공개 API

`Authorization: Bearer <토큰>`, 계정에 `api_access` 권한 필요(문서). 알파위키·나무위키에서도
같은 경로가 403(토큰 없음)으로 응답해 존재가 확인됩니다(실측).

| 메서드 | 경로 | 파라미터 | 응답 |
|---|---|---|---|
| GET | `/api/edit/<문서>` | — | `text`, `exists`, `token` |
| POST | `/api/edit/<문서>` | `text`, `log`, `token` | `status`, `rev` |
| GET | `/api/backlink/<문서>` | `namespace`, `from`, `until` | `namespaces`, `backlinks`, `from`, `until` |
| GET | `/api/discuss/<문서>` | — | `slug`, `topic`, `updated_date`, `status` |

**공개 API는 이 넷뿐입니다.** 읽기(`/api/w/`)조차 없습니다.

내부 통신은 별개입니다 — 페이지 상태가 `window.INITIAL_STATE`에 실려 오는데, 알파위키·
더시드위키는 base64+zlib 평문이지만 **나무위키는 암호화되어 있습니다**(실측). 초기에는
평문이었다가 이용자 증가 후 전환했다고 서술됩니다(문서).

인증 없이 열리는 JSON은 `/sidebar.json` 하나로, 최근 토론 15건을
`{document, status, date}` 배열로 냅니다(실측).

---

## 9. 스킨

`/skins/<스킨명>/<해시>.{js,css}` 로 서빙되는 독립 번들입니다(실측). 사이트마다 다릅니다 —
나무위키 `espejo`, 알파위키 `liberty`, 더시드위키 `vutterfly`.

설정은 위키 공통 키와 **스킨 네임스페이스 키**로 나뉩니다(실측, 상태 블롭에서 추출).

```
wiki.site_name, wiki.front_page, wiki.canonical_url, wiki.logo_url,
wiki.copyright_text, wiki.copyright_url, wiki.editagree_text,
wiki.delete_document_text, wiki.site_head

skin.<스킨명>.navbar_logo_image / navbar_logo_size / navbar_logo_width /
              navbar_logo_padding / navbar_logo_margin / navbar_text /
              brand_color_1 / brand_bright_color_1 /
              light_brand_color / dark_brand_color / footer_html / special_day
```

`skin.vutterfly.special_day`가 날짜별 배너·테마를 제어하는 것까지 확인됩니다(실측).

```json
{"date":{"month":0,"date":1},"label":"새해 첫날","to":"/w/새해 첫날",
 "effect":"snow","theme":"happy"}
```

`theme`은 `happy`/`sad`, `effect`는 `snow`/`rainy`. 현충일은 `sad`입니다.

스킨 변경은 내 정보에서 고르는 방식이고 전용 URL은 없습니다(`/ChangeSkin` 404, 실측).

---

## 10. 다크 모드

theseed.io 인라인 스크립트에서 확정(실측).

```js
switch (JSON.parse(localStorage.getItem("theseed_settings") || "{}")["wiki.theme"]) {
  case void 0: case "auto":
    e = matchMedia("(prefers-color-scheme: dark)").matches ? 1 : 0; break;
  case "dark": e = 1; break;
  default: e = 0;
}
// body에 theseed-dark-mode / theseed-light-mode 부여
```

저장소는 `localStorage["theseed_settings"]`의 `wiki.theme` 키, 값은 `auto`(기본, OS 추종) /
`dark` / `light`, body 클래스는 `theseed-dark-mode` / `theseed-light-mode`입니다.

문법 차원의 다크 대응(`<tablebgcolor=#fff,#2d2f34>`, `{{{#0275d8,#ec9f19 …}}}`)과 렌더
결과의 `data-dark-style` 병기는 [namumark.md](namumark.md)가 다룹니다.

---

## 11. 그 밖의 기능

| 기능 | 내용 | 근거 |
|---|---|---|
| 파일 올리기 | `/Upload`. 이름은 `파일:`로 시작하고 확장자가 실제와 일치해야 함. 라이선스·분류·요약 필수. 첨부는 `[[파일:이름]]` | 실측+문서 |
| 문서 이동 | **5자 이상** 편집 요약 필수. **맞바꾸기(swap) 체크박스** — 대상에 역사가 있으면 필수 | 문서 |
| 문서 삭제 | 5자 이상 요약 + 확인 체크박스 | 문서 |
| 리비전 숨김 | `hide_revision`. 역사 로그 자체 숨김은 `hide_document_history_log` | 실측(권한) |
| 일괄 되돌리기 | `/admin/batch_revert`. BlockHistory에 기록됨 | 실측 |
| 반달 리비전 표시 | `mark_troll_revision` | 실측(권한) |
| IP 숨김 / 문서 잠금 / 보호 파일 편집 | `hideip` / `lock_document` / `edit_protected_file` | 실측(권한) |
| **기여 이전** | IP로 편집한 뒤 1시간 이내 같은 접속 환경에서 로그인하면, 역사에서 "이 기여를 로그인 사용자로 이전하기" | 문서 |
| CAPTCHA | 편집 시 표시. `no_force_captcha`(빈도 감소) / `skip_captcha`(건너뜀) | 실측(권한)+문서 |
| 모바일 인증 | `/member/signup_verify`. 한국 010은 **음성 통화로 PIN 4자리**. 엔진은 UI만 주고 실동작은 각 위키 몫. 나무위키만 활성 | 문서 |
| 자동 인증 | `auto_verified_member` — v4.26.0(2025-01-08) 도입, 조건 충족 시 엔진이 자동 부여 | 문서 |
| 편집기 | **시각 편집 없음.** 문법 도움말 링크 + 미리보기 + `?section=N` 문단 편집 | 문서+실측 |
| 광고 | 나무위키만 `/ads.txt` 존재 | 실측 |

### 엔진의 알려진 한계 (문서 — 알파위키 `the seed` 문서 서술)

- 시각 편집 미지원.
- 역사 완전 삭제(redaction)가 어려워 서버 관리자의 직접 DB 조작이 필요.
- **편집 취소(undo)가 없습니다** — 되돌리기만 있어 중간의 정당한 기여까지 함께 사라집니다.
- 편집 필터 미구현(단 `편집필터` 이름공간은 존재 — 도입 예정으로 추정).

---

## 12. 우리 구현과의 대조

우리 URL은 the seed 호환이 아니라 일반 웹 관례(소문자 kebab-case)를 따르므로
**경로 표기 차이는 차이가 아닙니다**(`/RecentChanges` ↔ `/recent-changes`). 아래는
기능 자체의 차이만 봅니다. 구현 목록의 정본은 `wiki-server`의 `router()`입니다.

### 이미 갖춘 것

문서 읽기·원문·편집·역사·비교·blame·되돌리기·삭제·이동·역링크·토론·편집요청·파일 올리기,
최근변경·최근토론·검색·임의문서·필요한문서·고립문서·분류없음·오래된문서·길이순·차단기록·
라이선스, 로그인·가입·이메일 검증, 권한 부여/회수·설정·일괄 되돌리기·리비전 숨김,
북마크·알림·자동완성. ACL 평가기(동작 8종·조건 5종)는 the seed와 어휘까지 일치합니다.

### 빠진 기능 — 영향이 큰 순서

1. **ACL 편집 화면이 없습니다.** 평가기와 `acl_rule` 테이블은 있으나 `/acl/<문서>`에
   해당하는 조회·설정 라우트가 없어, 규칙을 바꾸려면 DB를 직접 만져야 합니다.
   `gotons` 판정과 "`read`는 이름공간 단위로만" 규칙도 함께 확인이 필요합니다.
2. **ACL 그룹 관리 화면(`/aclgroup`)이 없습니다.** the seed에서 **차단은 곧 ACL 그룹 소속**
   이므로, 이 화면이 없으면 차단 운영이 성립하지 않습니다. `acl_group`·`acl_group_member`
   테이블은 이미 있습니다.
3. **계정 관리 일체가 없습니다** — 비밀번호 변경·찾기, 이메일 변경, 계정명 변경, 탈퇴,
   내 정보. `user_verification`의 `password_reset`·`email_change` 목적값은 이미 시드돼
   있으나 이를 쓰는 경로가 없습니다.
4. **문단 편집(`?section=N`)이 없습니다.** 긴 문서의 실사용 편집 경험을 좌우합니다.
5. **스킨 체계가 없습니다.** the seed는 번들 + `skin.<이름>.*` 설정 네임스페이스 구조입니다.
6. **공개 API가 다릅니다.** the seed의 넷(`/api/edit` GET·POST, `/api/backlink`,
   `/api/discuss`)은 `api_access` 권한 + Bearer 토큰 체계인데, 우리에겐 토큰 발급도
   권한 검사도 없습니다. 반대로 우리는 프론트엔드용 JSON API가 훨씬 넓습니다.
7. **`/activity/<문서>`(역사+토론 통합 타임라인)가 없습니다.**
8. **`/Go`(정확 일치 바로가기)와 `/opensearch.xml`이 없습니다.** 브라우저 검색창 연동이
   여기에 달려 있습니다.
9. **로그인 기록 조회 화면(`/admin/login_history`)이 없습니다.** `login_record` 테이블과
   `login_history` 권한은 있습니다.
10. **필터·페이지네이션 파라미터가 얕습니다.** `RecentChanges`의 `logtype` 6종,
    `RecentDiscuss`의 8종, 검색의 `target`·`namespace`, 기여 목록의 `type` 10종,
    커서(`from`/`until`) 페이지네이션이 없습니다.
11. **이름공간이 7종뿐입니다**(`문서`·`틀`·`분류`·`파일`·`사용자`·`위키운영`·`휴지통`).
    the seed에는 `특수기능`·`토론`·`특정판`·`삭제된사용자`·`아이피사용자`·`파일휴지통`이
    더 있습니다. `Namespace` 타입도 사실상 `main`만 다룹니다.
12. **권한이 16종입니다.** the seed 21종 대비 빠진 것: `manage_thread`·`no_force_captcha`·
    `mark_troll_revision`·`aclgroup_hidelog`·`hideip`·`delete_edit_request`·
    `hide_document_history_log`·`edit_protected_file`·`lock_document`. 우리 쪽에만 있는
    것은 스레드 세부 권한 4종(the seed에서 `manage_thread` 하위일 가능성).
13. **리비전 UUID 주소 지정이 없습니다.** the seed는 `?uuid=`만 받고 정수 번호는 URL에
    쓰지 않습니다 — 우리 데이터 모델의 "내부 정수 PK 비노출" 원칙과 같은 방향이므로
    맞춰 둘 만합니다.
14. **CAPTCHA·모바일 인증·기여 이전·TOTP**가 없습니다. CAPTCHA는 자체 호스팅 원칙에 따라
    **의도적 미구현**이고, 모바일 인증도 엔진이 UI만 주는 것이라 우선순위가 낮습니다.
    `credential_kind`에 `totp`는 이미 시드돼 있습니다.

### 우리가 앞서는 지점

- 리비전 UUID·정수 PK 분리 등 데이터 모델 원칙이 처음부터 서 있습니다.
- 3-way 자동 병합이 있습니다(the seed의 동시 편집 처리는 미확인).
- 특수 페이지에 전용 URL이 있습니다 — the seed는 알림·내 문서함이 스킨 안에만 있습니다.

### 우리가 **따라가지 말아야** 할 것

- **정수 리비전 번호를 URL에 노출**하는 방식(the seed도 UUID로 갔습니다).
- **편집 취소(undo) 부재** — the seed가 스스로 한계로 꼽는 지점입니다. 우리는 3-way 병합이
  있으므로 특정 리비전만 되돌리는 undo를 만들 여지가 있습니다.
- **`ip:` 조건의 대역 미지원** — CIDR을 받는 편이 낫습니다.

---

## 조사의 한계

1. 비로그인 GET만 수행했습니다. `/admin/*`·`/member/mypage` 등은 존재만 확정했고 폼
   파라미터는 미확인입니다.
2. TOTP·API 토큰 발급 UI의 실제 경로를 찾지 못했습니다.
3. 이름공간의 엔진 기본/커스텀 구분은 추정입니다.
4. `manage_thread`와 스레드 세부 권한의 관계는 추정입니다.
5. openNAMU·the tree는 참고하지 않았습니다 — the seed 실동작과 섞일 위험이 있고,
   the tree는 수정·재배포 금지 라이선스입니다.
