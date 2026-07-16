# opensinabro

나무위키 엔진의 오픈소스 재구현 프로젝트입니다.

## 구성

- [`crates/namumark-parser`](crates/namumark-parser) — 나무마크(namumark) 파서. 완전무손실 구문 트리(`parse_syntax`)와 의미 모델(`parse` → `Document`)을 제공합니다.
- [`crates/namumark-render`](crates/namumark-render) — 렌더링 파이프라인. resolve/layout pass를 거쳐 백엔드(1차: 나무위키 동급 HTML)로 출력합니다.

## 로드맵

1. 나무마크 파서 — 완료 (red-green tree, 골든테스트)
2. 렌더링 엔진 — HTML 백엔드 1차 완료 (다중 백엔드 구조)
3. 위키 서버 (문서 저장소, 히스토리, 검색)

## 라이선스

[MIT](LICENSE)
