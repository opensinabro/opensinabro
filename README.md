# opensinabro

나무위키 엔진의 오픈소스 재구현 프로젝트입니다.

## 구성

단일 역할 원칙으로 나눈 크레이트들입니다(구조·의존 관계는 [설계 문서](docs/design/03-crate-structure.md)).

- [`crates/namumark-parser`](crates/namumark-parser) — 나무마크(namumark) 파서. 완전무손실 구문 트리(`parse_syntax`)와 의미 모델(`parse` → `Document`)을 제공합니다.
- [`crates/namumark-render`](crates/namumark-render) — 렌더링 파이프라인. resolve/layout pass를 거쳐 백엔드(1차: 나무위키 동급 HTML)로 출력합니다.
- 그 외 `namumark-text`(표기 판정), `namumark-ast`(의미 모델), `namumark-syntax`(CST), `namumark-ir`(렌더 IR), `namumark-backend-namuwiki`(마크업 방출).

## 문서

- [나무마크 문법 스펙](docs/spec/namumark.md) — the seed 동작을 대조실험으로 규명한 구현자용 정밀 스펙.
- [구현 현황 — 누락과 차이](docs/spec/implementation-status.md) — 우리 렌더러가 the seed와 다르거나 아직 구현하지 않은 지점.
- [설계 문서](docs/design) — 구문 트리·렌더 파이프라인·크레이트 구조·파리티 검증·위키 서버.

## 로드맵

1. 나무마크 파서 — 완료 (red-green tree, 골든테스트)
2. 렌더링 엔진 — HTML 백엔드 1차 완료 (다중 백엔드 구조). 알파위키 문법 도움말·심화 기준
   [파리티 0](docs/design/04-namuwiki-parity.md) 달성.
3. 위키 서버 — 설계 완료, 구현 예정
   ([요구사항](docs/design/06-wiki-server-requirements.md),
   [아키텍처·로드맵](docs/design/07-wiki-server-architecture.md),
   [데이터 모델](docs/design/08-wiki-server-data-model.md))

## 라이선스

[MIT](LICENSE)
