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
- **나무위키 동급 출력** — 알파위키 문법 도움말·심화 기준 [파리티 0](docs/design/04-namuwiki-parity.md)을 달성했습니다.

## 직접 실행하기

플레이그라운드를 로컬에서 빌드·구동합니다.

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

단일 역할 원칙으로 나눈 크레이트들입니다(구조·의존 관계는 [설계 문서](docs/design/03-crate-structure.md)).

| 크레이트 | 역할 |
| --- | --- |
| [`namumark-parser`](crates/namumark-parser) | 나무마크 파서 — 무손실 구문 트리(`parse_syntax`)와 의미 모델(`parse` → `Document`) |
| [`namumark-render`](crates/namumark-render) | 렌더링 파이프라인 — resolve/layout pass를 거쳐 백엔드로 출력 |
| [`namumark-syntax`](crates/namumark-syntax) | 완전무손실 구문 트리 (red-green) |
| [`namumark-text`](crates/namumark-text) | 표기의 문자열 수준 판정 유틸리티 |
| [`namumark-ast`](crates/namumark-ast) | 의미 모델 타입 |
| [`namumark-ir`](crates/namumark-ir) | 렌더 IR 타입과 백엔드 계약 |
| [`namumark-backend-namuwiki`](crates/namumark-backend-namuwiki) | 나무위키 동등 마크업 방출 |
| [`namumark-playground`](crates/namumark-playground) | 플레이그라운드 WASM 바인딩 |

## 문서

전체 목록과 읽는 순서는 [문서 인덱스](docs/README.md)에 있습니다.

- [나무마크 문법 스펙](docs/spec/namumark.md) — the seed 동작을 대조실험으로 규명한 구현자용 정밀 스펙.
- [구현 현황 — 누락과 차이](docs/spec/implementation-status.md) — 우리 렌더러가 the seed와 다르거나 아직 구현하지 않은 지점.
- [설계 문서](docs/design) — 구문 트리·렌더 파이프라인·크레이트 구조·파리티 검증·위키 서버.

## 로드맵

1. **나무마크 파서** — 완료 (red-green tree, 골든테스트)
2. **렌더링 엔진** — HTML 백엔드 1차 완료 (다중 백엔드 구조). [파리티 0](docs/design/04-namuwiki-parity.md) 달성.
3. **위키 서버** — M1(읽기 전용)·M2(편집·리비전·ACL)·M3(계정·권한)·M4(토론·편집요청)·
   M5(파일·운영)·M6(알림·북마크·API·특수 페이지) 완료. 남은 것은 스킨 체계·TOTP
   ([요구사항](docs/design/06-wiki-server-requirements.md),
   [아키텍처·로드맵](docs/design/07-wiki-server-architecture.md),
   [데이터 모델](docs/design/08-wiki-server-data-model.md))

## 실행

작업은 [`justfile`](justfile)에 모아 두었습니다. `just`만 있으면 됩니다.

```sh
just            # 무엇을 할 수 있는지 봅니다
```

### 기본: 서버는 로컬, 데이터베이스는 컨테이너

PostgreSQL을 따로 설치할 필요 없이 컴포즈가 띄웁니다.

```sh
just setup      # 데이터베이스 컨테이너를 띄웁니다 (처음 한 번)
just import     # fixtures/documents의 문서를 적재합니다
just run        # http://127.0.0.1:3000
```

`just start`는 위 셋을 잇고, `run`·`dev`·`import`는 데이터베이스가 꺼져 있으면 알아서
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
