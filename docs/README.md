# 문서

opensinabro의 문서는 성격에 따라 나뉩니다.

- **[`spec/`](spec)** — *무엇이 참인가*. the seed의 실제 동작을 대조실험으로 규명한 결과.
  구현이 바뀌어도 사실은 바뀌지 않습니다.
- **[`architecture.md`](architecture.md)** — *왜 이렇게 만들었는가*. 코드에서 읽어낼 수
  없는 판단 기준과 기각한 대안만 담습니다.

구현의 세부는 문서가 아니라 코드가 정본입니다 — 크레이트의 역할과 의존은 각 `lib.rs`의
문서주석이, 테이블 정의는 `crates/wiki-server/migrations/`가, API 경로는 `wiki-server`의
`router()`가 말합니다.

## 스펙 — 나무마크의 사실

| 문서 | 내용 |
|---|---|
| [나무마크 문법 스펙](spec/namumark.md) | 구현자용 정밀 문법 스펙. 규칙마다 근거 등급 표기. **개별 문법 동작은 여기가 정본.** |
| [구현 현황 — 누락과 차이](spec/implementation-status.md) | 우리 렌더러가 the seed와 다르거나 아직 구현하지 않은 지점. |

문법 사실의 실행 가능한 짝은 [`fixtures/corpus/`](../fixtures/corpus)의 회귀 케이스입니다.

## 설계

| 문서 | 내용 |
|---|---|
| [위키 서버 아키텍처](architecture.md) | 크레이트 분할 기준, URL 설계 원칙, 데이터 모델 원칙, 보안 표준, 기각한 대안. |

## 문서 밖

- 파리티 검증 방법론과 도구 사용법은 [`tools/parity/README.md`](../tools/parity/README.md).
- 검증 데이터의 출처·라이선스·갱신법은 [`fixtures/README.md`](../fixtures/README.md),
  회귀 코퍼스의 케이스 형식은 [`fixtures/corpus/README.md`](../fixtures/corpus/README.md).
- 프로젝트 개요와 빌드 방법은 [루트 README](../README.md).
