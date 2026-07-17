# 나무마크 플레이그라운드

나무마크를 브라우저에서 준실시간으로 렌더하는 스탠드얼론 웹 앱. 렌더는 전부
클라이언트 사이드(WASM)에서 돌아가므로 위키 서버가 필요 없다.

- 파이프라인: `namumark-parser` → `namumark-render` → 렌더 백엔드
- WASM 바인딩: [`crates/namumark-playground`](../crates/namumark-playground)
- 프론트엔드: React + Vite + Tailwind CSS + shadcn/ui, 상태는 Zustand.
  프리뷰는 Shadow DOM으로 격리해 백엔드 CSS(`wiki-*`)를 그대로 산다.
- 백엔드: 현재 나무위키(`namumark-backend-namuwiki`)만. 추가 백엔드는 WASM 크레이트의
  `BACKENDS` 레지스트리에 항목을 더하면 UI 드롭다운에 자동 노출된다.
- 다크모드는 지원하지 않는다.

## 구동

```sh
npm install
npm run wasm   # crates/namumark-playground → playground/wasm (WASM 빌드)
npm run dev    # Vite 개발 서버
```

`npm run build`로 정적 번들(`dist/`)을 만들면 아무 정적 서버로나 배포할 수 있다.
렌더 로직(Rust)을 고치면 `npm run wasm`을 다시 돌려야 반영된다.
