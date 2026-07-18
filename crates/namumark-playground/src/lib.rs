//! 나무마크 플레이그라운드 WASM 바인딩.
//!
//! 브라우저에서 나무마크 원문을 받아 렌더 파이프라인
//! (parse → [`build_render_tree`] → 백엔드)을 돌려 HTML·CSS를 돌려준다.
//! 외부 세계가 없는 [`EmptyContext`]를 쓰므로 모든 링크는 빨간 링크이고
//! include는 확장되지 않는다 — 플레이그라운드는 단일 문서만 다룬다.

use namumark_analysis::DiagnosticCode;
use namumark_ir::RenderTree;
use namumark_render::{EmptyContext, build_render_tree, build_render_tree_with_diagnostics};
use namumark_syntax::{NodeOrToken, SyntaxKind};
use std::fmt::Write as _;
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

/// 무손실 구문 트리의 리프 토큰을 원문 순서대로 훑어
/// `[{"kind","parent","text","start"}, ...]` JSON으로 돌려준다.
///
/// `kind`는 토큰의 역할(Marker·Text·ListMarker …), `parent`는 토큰을 감싼 노드
/// (Bold·Heading·Link …)다. 둘을 합치면 "이 조각이 무슨 뜻인지"가 된다.
/// 토큰 `text`를 이으면 원문이 그대로 복원된다(무손실).
#[wasm_bindgen]
pub fn inspect(source: &str) -> String {
    let tree = namumark_syntax::parse(source);
    let mut json = String::from("[");
    let mut first = true;
    for token in tree
        .root()
        .descendants_with_tokens()
        .filter_map(NodeOrToken::into_token)
    {
        if !first {
            json.push(',');
        }
        first = false;
        let parent = token
            .parent()
            .map(|node| kind_name(node.kind()))
            .unwrap_or_else(|| "Document".to_string());
        json.push_str(r#"{"kind":""#);
        json.push_str(&kind_name(token.kind()));
        json.push_str(r#"","parent":""#);
        json.push_str(&parent);
        json.push_str(r#"","start":"#);
        let _ = write!(json, "{}", usize::from(token.text_range().start()));
        json.push_str(r#","text":""#);
        escape_json_into(token.text(), &mut json);
        json.push_str(r#""}"#);
    }
    json.push(']');
    json
}

/// 나무마크 원문을 검사해 진단을 `[{"code","severity","category","message","start","end"}, ...]`
/// JSON으로 돌려준다. `start`·`end`는 원문 바이트 오프셋이다.
///
/// 문맥 자유 진단([`namumark_analysis::analyze`])과 resolve의 문맥 의존 진단(미지원
/// 매크로 등)을 합쳐 원문 위치 순으로 낸다.
///
/// 플레이그라운드는 [`EmptyContext`]라 어떤 문서도 존재하지 않는다 — 그 상태에서
/// `include-target-missing`은 모든 include에 붙어 뜻이 없으므로 걸러낸다. 존재 판정은
/// 실제 위키 문맥이 있을 때만 의미가 있다.
#[wasm_bindgen]
pub fn diagnose(source: &str) -> String {
    let document = namumark_parser::parse(source);
    let mut diagnostics = namumark_analysis::analyze(&document);
    let (_, render_diagnostics) = build_render_tree_with_diagnostics(&document, &EmptyContext);
    diagnostics.extend(render_diagnostics);
    diagnostics.retain(|diagnostic| diagnostic.code != DiagnosticCode::IncludeTargetMissing);
    diagnostics.sort_by_key(|diagnostic| diagnostic.range.start());

    let mut json = String::from("[");
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str(r#"{"code":""#);
        json.push_str(diagnostic.code.as_str());
        json.push_str(r#"","severity":""#);
        json.push_str(diagnostic.severity().as_str());
        json.push_str(r#"","category":""#);
        json.push_str(diagnostic.category().as_str());
        json.push_str(r#"","start":"#);
        let _ = write!(json, "{}", usize::from(diagnostic.range.start()));
        json.push_str(r#","end":"#);
        let _ = write!(json, "{}", usize::from(diagnostic.range.end()));
        json.push_str(r#","message":""#);
        escape_json_into(&diagnostic.message, &mut json);
        json.push_str(r#""}"#);
    }
    json.push(']');
    json
}

/// SyntaxKind의 변형 이름 그대로(Debug 파생). 프런트가 한국어 설명으로 매핑한다.
fn kind_name(kind: SyntaxKind) -> String {
    format!("{kind:?}")
}

fn escape_json_into(text: &str, out: &mut String) {
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", control as u32);
            }
            other => out.push(other),
        }
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
