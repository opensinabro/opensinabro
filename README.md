<img src="docs/assets/logo.svg" alt="" width="64" height="64">

# opensinabro

**나무위키 엔진의 오픈소스 재구현.** 나무마크(namumark)를 무손실로 파싱하고, the seed와 대조해 규명한 정밀 스펙대로 렌더합니다.

[![라이브 데모](https://img.shields.io/badge/라이브_데모-플레이그라운드-2e8b57)](https://opensinabro.github.io/opensinabro/)
[![Pages 배포](https://github.com/opensinabro/opensinabro/actions/workflows/deploy-pages.yml/badge.svg)](https://github.com/opensinabro/opensinabro/actions/workflows/deploy-pages.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue)](LICENSE)

## ▶ 바로 써보기

**[opensinabro.github.io/opensinabro](https://opensinabro.github.io/opensinabro/)** — 브라우저에서 나무마크를 입력하면 실시간으로 렌더됩니다. 설치도, 서버도 필요 없이 WASM으로 로컬에서 동작합니다.

## 맛보기

```
= 나무마크 =
'''굵게''', ''기울임'', __밑줄__, ~~취소선~~, 그리고 {{{#ff0000 색}}}.

 * 리스트와 [[문서 링크]], 각주도[* 이렇게] 지원합니다.

|| 표 || 헤더 ||
|| 셀 || 셀 ||
```

위 문법이 각주 번호·목차·표 스타일까지 나무위키와 같은 결과로 렌더됩니다. [플레이그라운드](https://opensinabro.github.io/opensinabro/)에서 바로 확인하세요.

## 무엇이 다른가

- **정본 대조로 규명한 스펙** — 문서가 아닌 the seed의 실제 동작을 대조실험으로 밝혀 [정밀 스펙](docs/spec/namumark.md)으로 정리했습니다.
- **완전무손실 파싱** — 공백·주석까지 원문을 100% 보존하는 red-green 구문 트리 위에서 동작합니다.
- **나무위키 동급 출력** — 알파위키 문법 도움말·심화 기준 [파리티 0](tools/parity/README.md)을 달성했습니다.

## 직접 실행하기

### 위키 서버

[just](https://github.com/casey/just)와 Docker, Node가 필요합니다. 명령은 하나입니다.

```bash
just dev
```

데이터베이스 준비, 의존성 설치, 포트 정리까지 알아서 합니다.
이미 떠 있던 서버가 있으면 정리하고 새로 띄우므로, 몇 번을 쳐도 같은 결과가 됩니다.
Ctrl+C 한 번으로 전부 멈춥니다.

브라우저에서 **http://127.0.0.1:3000** 을 엽니다. 프론트엔드는 3001에서 따로 돌지만
백엔드가 대신 넘겨주므로 직접 열 일이 없습니다.

가끔 쓰는 것들입니다.

| 명령 | 하는 일 |
| --- | --- |
| `just back` / `just front` | 백엔드·프론트엔드를 각각 다른 터미널에서 띄웁니다 |
| `just import <경로>` | 나무마크 원문을 적재합니다 (기본값 `fixtures/documents`) |
| `just database-reset` | 데이터베이스와 색인을 비웁니다 |
| `just free-ports` | 3000·3001을 물고 있는 프로세스만 정리합니다 |
| `just docker-up` | 전부 컨테이너로 띄웁니다 |
| `just check` | 커밋 전 포맷·린트·테스트 |

`just` 를 인자 없이 치면 전체 목록이 나옵니다.

### 플레이그라운드

```bash
cd playground
npm install
npm run wasm    # Rust → WASM 빌드
npm run dev     # 개발 서버
```

Rust 크레이트 테스트:

```bash
cargo test
```

## 구성

나무마크 엔진은 **산출물**(CST·AST·IR·마크업)로, 위키 서버는 **데이터 소유권**으로 나눕니다.
각 크레이트의 역할과 의존 방향은 그 `lib.rs` 문서주석에 있습니다.

| 나무마크 엔진 | 역할 |
| --- | --- |
| [`namumark-parser`](crates/namumark-parser) | 나무마크 파서 — 무손실 구문 트리(`parse_syntax`)와 의미 모델(`parse` → `Document`) |
| [`namumark-render`](crates/namumark-render) | 렌더링 파이프라인 — resolve/layout pass를 거쳐 백엔드로 출력 |
| [`namumark-syntax`](crates/namumark-syntax) | 완전무손실 구문 트리 (red-green) |
| [`namumark-text`](crates/namumark-text) | 표기의 문자열 수준 판정 유틸리티 |
| [`namumark-ast`](crates/namumark-ast) | 의미 모델 타입 |
| [`namumark-ir`](crates/namumark-ir) | 렌더 IR 타입과 백엔드 계약 |
| [`namumark-backend-namuwiki`](crates/namumark-backend-namuwiki) | 나무위키 동등 마크업 방출 |
| [`namumark-analysis`](crates/namumark-analysis) | 의미 모델 진단 (문맥 자유) |
| [`namumark-playground`](crates/namumark-playground) | 플레이그라운드 WASM 바인딩 |

| 위키 서버 | 소유 영역 |
| --- | --- |
| [`wiki-account`](crates/wiki-account) | 행위 주체(actor)와 인증 — 의존 그래프의 뿌리 |
| [`wiki-document`](crates/wiki-document) | 문서와 그 역사, 렌더 파이프라인 접점 |
| [`wiki-authorization`](crates/wiki-authorization) | 권한 판정 — ACL·aclgroup·perm |
| [`wiki-discussion`](crates/wiki-discussion) | 토론 스레드와 편집요청 |
| [`wiki-search`](crates/wiki-search) | 전문 검색 색인 (tantivy) |
| [`wiki-server`](crates/wiki-server) | HTTP 조립 — axum 라우팅·세션·JSON API |

## 문서

전체 목록과 읽는 순서는 [문서 인덱스](docs/README.md)에 있습니다.

- [나무마크 문법 스펙](docs/spec/namumark.md) — the seed 동작을 대조실험으로 규명한 구현자용 정밀 스펙.
- [구현 현황 — 누락과 차이](docs/spec/implementation-status.md) — 우리 렌더러가 the seed와 다르거나 아직 구현하지 않은 지점.
- [위키 서버 아키텍처](docs/architecture.md) — 크레이트 분할 기준·URL 설계·데이터 모델 원칙과 기각한 대안.
- [파리티 검증](tools/parity/README.md) — the seed 실동작을 근거로 얻는 경로와 대조 도구 사용법.

## 로드맵

1. **나무마크 파서** — 완료 (red-green tree, 골든테스트)
2. **렌더링 엔진** — HTML 백엔드 1차 완료 (다중 백엔드 구조). [파리티 0](tools/parity/README.md) 달성.
3. **위키 서버** — 읽기·편집·리비전·ACL·계정·토론·편집요청·파일·운영·알림·API,
   그리고 Next.js 프론트엔드까지 완료. 남은 것은 TOTP 로그인입니다.
   알려진 렌더링 차이는 [구현 현황](docs/spec/implementation-status.md)에 있습니다.

## 실행

작업은 [`justfile`](justfile)에 모아 두었습니다. `just`만 있으면 됩니다.

```sh
just            # 무엇을 할 수 있는지 봅니다
```

### 기본: 서버는 로컬, 데이터베이스는 컨테이너

PostgreSQL을 따로 설치할 필요 없이 컴포즈가 띄웁니다.

```sh
just setup      # 데이터베이스 컨테이너를 띄웁니다 (처음 한 번)
just run        # http://127.0.0.1:3000
```

갓 만든 위키에는 대문 하나만 있습니다. 예시 문서가 필요하면 `just import`로
`fixtures/documents`의 나무마크 원문을 넣을 수 있습니다.

`run`·`dev`·`import`는 데이터베이스가 꺼져 있으면 알아서
띄웁니다. 스키마는 서버가 시작하며 스스로 적용합니다. 접속 정보는 `DATABASE_URL`·
`OPENSINABRO_DATABASE_PORT`·`OPENSINABRO_ADDRESS`로 덮어쓸 수 있습니다.

### 서버까지 컨테이너로

```sh
just docker-up
just docker-import
```

### 그 밖

```sh
just dev              # 디버그 빌드로 띄웁니다 (빌드가 빠릅니다)
just database-shell   # psql
just database-reset   # 데이터베이스·검색 색인·올린 파일을 비웁니다
just down             # 컨테이너를 내립니다 (데이터는 남습니다)
just check            # 커밋 전에: 서식·클리피·테스트
```

## 라이선스

[MIT](LICENSE)
