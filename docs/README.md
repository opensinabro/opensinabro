# 문서

opensinabro의 문서는 성격에 따라 두 갈래로 나뉩니다.

- **[`spec/`](spec)** — *무엇이 참인가*. the seed의 실제 동작을 대조실험으로 규명한 결과.
  구현이 바뀌어도 사실은 바뀌지 않습니다.
- **[`design/`](design)** — *우리가 어떻게 만들었는가*. 구조 결정과 그 근거·대안 기각 이유.
  구현과 함께 갱신됩니다.

## 스펙 — 나무마크의 사실

| 문서 | 내용 |
|---|---|
| [나무마크 문법 스펙](spec/namumark.md) | 구현자용 정밀 문법 스펙. 규칙마다 근거 등급 표기. **개별 문법 동작은 여기가 정본.** |
| [구현 현황 — 누락과 차이](spec/implementation-status.md) | 우리 렌더러가 the seed와 다르거나 아직 구현하지 않은 지점. |

문법 사실의 실행 가능한 짝은 [`fixtures/corpus/`](../fixtures/corpus)의 회귀 케이스입니다.

## 설계 — 나무마크 엔진

| 문서 | 상태 |
|---|---|
| [01 · Red-Green 구문 트리](design/01-red-green-syntax-tree.md) | 구현 완료 |
| [02 · 렌더링 파이프라인](design/02-render-pipeline.md) | 구현 완료 |
| [03 · 단일 역할 크레이트 구조](design/03-crate-structure.md) | 구현 완료 |
| [04 · 나무위키 파리티 검증](design/04-namuwiki-parity.md) | 검증 체계 확립 |
| [05 · 틀 인자를 구문 트리로 흡수](design/05-template-parameters.md) | 구현 완료 |

## 설계 — 위키 서버

설계만 완료되고 아직 구현하지 않은 단계입니다. 06 → 07 → 08 순으로 읽습니다.

| 문서 | 상태 |
|---|---|
| [06 · 기능·요구사항](design/06-wiki-server-requirements.md) | 초안 |
| [07 · 아키텍처와 로드맵](design/07-wiki-server-architecture.md) | 초안 |
| [08 · 데이터 모델](design/08-wiki-server-data-model.md) | 초안 |

## 문서 밖

- 구현 세부(왜·어떻게)는 코드와 그 주석에 있습니다.
- 검증 데이터의 출처·라이선스·갱신법은 [`fixtures/README.md`](../fixtures/README.md).
- 프로젝트 개요와 빌드 방법은 [루트 README](../README.md).
