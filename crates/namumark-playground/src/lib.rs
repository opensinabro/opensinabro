//! 나무마크 플레이그라운드 WASM 바인딩.
//!
//! 브라우저에서 나무마크 원문을 받아 렌더 파이프라인
//! (parse → [`build_render_tree`] → 백엔드)을 돌려 HTML·CSS를 돌려준다.
//! 외부 세계가 없는 [`EmptyContext`]를 쓰므로 모든 링크는 빨간 링크이고
//! include는 확장되지 않는다 — 플레이그라운드는 단일 문서만 다룬다.

use namumark_ir::RenderTree;
use namumark_render::{EmptyContext, build_render_tree};
use wasm_bindgen::prelude::*;

/// 렌더링 백엔드 하나. 백엔드마다 마크업 어휘와 스타일시트가 다르므로
/// 방출 HTML과 동봉 CSS를 함께 낸다.
struct Backend {
    id: &'static str,
    label: &'static str,
    emit: fn(&RenderTree) -> Emission,
}

struct Emission {
    html: String,
    css: String,
}

fn emit_namuwiki(tree: &RenderTree) -> Emission {
    Emission {
        html: namumark_backend_namuwiki::namuwiki_markup(tree).to_string(),
        css: namumark_backend_namuwiki::stylesheet().to_string(),
    }
}

/// 지원 백엔드 레지스트리. 본 프로젝트용 백엔드가 생기면 여기에 항목을 추가한다.
const BACKENDS: &[Backend] = &[Backend {
    id: "namuwiki",
    label: "나무위키",
    emit: emit_namuwiki,
}];

/// 백엔드 목록을 `[{"id","label"}, ...]` JSON으로 돌려준다.
#[wasm_bindgen]
pub fn backends() -> String {
    let mut json = String::from("[");
    for (index, backend) in BACKENDS.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            r#"{{"id":"{}","label":"{}"}}"#,
            backend.id, backend.label
        ));
    }
    json.push(']');
    json
}

/// 한 백엔드의 렌더 결과. wasm-bindgen getter로 JS에 노출한다.
#[wasm_bindgen]
pub struct RenderOutput {
    html: String,
    css: String,
}

#[wasm_bindgen]
impl RenderOutput {
    #[wasm_bindgen(getter)]
    pub fn html(&self) -> String {
        self.html.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn css(&self) -> String {
        self.css.clone()
    }
}

/// 나무마크 원문을 주어진 백엔드로 렌더한다.
#[wasm_bindgen]
pub fn render(source: &str, backend_id: &str) -> Result<RenderOutput, JsError> {
    let backend = BACKENDS
        .iter()
        .find(|backend| backend.id == backend_id)
        .ok_or_else(|| JsError::new(&format!("알 수 없는 백엔드: {backend_id}")))?;

    let document = namumark_parser::parse(source);
    let tree = build_render_tree(&document, &EmptyContext);
    let emission = (backend.emit)(&tree);

    Ok(RenderOutput {
        html: emission.html,
        css: emission.css,
    })
}
