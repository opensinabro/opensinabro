//! 나무마크 의미 모델 진단.
//!
//! 의미 모델([`namumark_ast::Document`])을 받아 저자·유지보수자에게 알릴 진단을 낸다.
//! **문맥 자유** 검사만 한다 — 문서 하나만 보고 판정 가능한 것들이다. 대상 문서 존재
//! 여부처럼 `WikiContext`(DB)가 필요한 문맥 의존 진단은 이 크레이트 밖(resolve)이 낸다.
//!
//! 렌더 경로와 완전히 분리된 opt-in 계층이라, 편집기·플레이그라운드가 필요할 때만 부른다.
//!
//! 새 진단은 [`DiagnosticCode`]에 변형을 더하고 대응 검사 함수를 [`analyze`]에 잇는다.

mod diagnostic;

pub use diagnostic::{Category, Diagnostic, DiagnosticCode, Replacement, Severity};
pub use namumark_syntax::TextRange;

use namumark_ast::{AstNode, Category as CategoryNode, Document, Footnote, Heading, Inline};
use namumark_syntax::{SyntaxKind, SyntaxNode};

/// 문서를 검사해 진단을 원문 위치 순으로 낸다.
pub fn analyze(document: &Document) -> Vec<Diagnostic> {
    let blocks: Vec<SyntaxNode> = document.syntax().children().collect();

    let mut diagnostics = Vec::new();
    check_redirect(&blocks, &mut diagnostics);
    check_heading_levels(&blocks, &mut diagnostics);
    check_descendants(document, &mut diagnostics);
    diagnostics.sort_by_key(|diagnostic| diagnostic.range.start());
    diagnostics
}

/// 렌더 화면에 무언가를 내는 블록인지. 주석과 리다이렉트 지시자는 아니다.
fn is_visible(block: &SyntaxNode) -> bool {
    !matches!(block.kind(), SyntaxKind::Comment | SyntaxKind::Redirect)
}

/// 리다이렉트 뒤 표시되지 않는 내용, 그리고 무시되는 중복 리다이렉트를 짚는다.
///
/// resolve는 첫 리다이렉트만 채택하고 나머지·후행 내용을 조용히 드롭한다. 이 검사는
/// 그 손실을 표면화할 뿐, 판정 규칙은 렌더와 같다.
fn check_redirect(blocks: &[SyntaxNode], diagnostics: &mut Vec<Diagnostic>) {
    let mut redirects = blocks
        .iter()
        .filter(|block| block.kind() == SyntaxKind::Redirect);
    let Some(first_redirect) = redirects.next() else {
        return;
    };

    for extra_redirect in redirects {
        diagnostics.push(Diagnostic {
            code: DiagnosticCode::DuplicateRedirect,
            range: extra_redirect.text_range(),
            message: "리다이렉트가 둘 이상입니다. 첫 리다이렉트만 적용되고 이 지시자는 무시됩니다."
                .into(),
            suggestion: None,
        });
    }

    let redirect_end = first_redirect.text_range().end();
    let trailing: Vec<&SyntaxNode> = blocks
        .iter()
        .filter(|block| block.text_range().start() >= redirect_end && is_visible(block))
        .collect();
    if let (Some(first), Some(last)) = (trailing.first(), trailing.last()) {
        diagnostics.push(Diagnostic {
            code: DiagnosticCode::RedirectTrailingContent,
            range: TextRange::new(first.text_range().start(), last.text_range().end()),
            message: "리다이렉트 뒤의 내용은 표시되지 않습니다.".into(),
            suggestion: None,
        });
    }
}

/// 제목 단계가 한 단계를 건너뛰면(예: 1단계 뒤 3단계) 향상 제안을 낸다.
fn check_heading_levels(blocks: &[SyntaxNode], diagnostics: &mut Vec<Diagnostic>) {
    let mut previous_level: Option<u8> = None;
    for block in blocks {
        let Some(heading) = Heading::cast(block.clone()) else {
            continue;
        };
        let level = heading.level();
        if let Some(previous) = previous_level
            && level > previous + 1
        {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::HeadingLevelSkip,
                range: block.text_range(),
                message: format!(
                    "제목 단계가 {previous}단계에서 {level}단계로 건너뜁니다. 한 단계씩 내려가면 목차가 자연스럽습니다."
                ),
                suggestion: None,
            });
        }
        previous_level = Some(level);
    }
}

/// 인라인이 렌더에 아무것도 남기지 않는지(빈 공백 텍스트뿐인지).
fn inlines_are_blank(inlines: &[Inline]) -> bool {
    inlines.iter().all(|inline| match inline {
        Inline::Text(text) => text.trim().is_empty(),
        _ => false,
    })
}

/// 문서 전체를 훑어 위치가 흩어진 인라인 진단을 낸다: 중복 분류와 같은 이름 각주의
/// 재정의. 인라인은 문단·인용·리스트·표 셀 어디에나 있으므로 최상위 블록이 아니라
/// 구문 트리 전체 자손을 순회한다.
fn check_descendants(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen_categories: Vec<String> = Vec::new();
    let mut defined_footnotes: Vec<String> = Vec::new();

    for node in document.syntax().descendants() {
        match node.kind() {
            SyntaxKind::Category => {
                if let Some(category) = CategoryNode::cast(node.clone()) {
                    let name = category.name();
                    if seen_categories.contains(&name) {
                        diagnostics.push(Diagnostic {
                            code: DiagnosticCode::DuplicateCategory,
                            range: node.text_range(),
                            message: format!("분류 `{name}`이(가) 이미 지정되어 중복입니다."),
                            suggestion: None,
                        });
                    } else {
                        seen_categories.push(name);
                    }
                }
            }
            SyntaxKind::Footnote => {
                if let Some(footnote) = Footnote::cast(node.clone())
                    && let Some(name) = footnote.name()
                    && !inlines_are_blank(&footnote.content())
                {
                    if defined_footnotes.contains(&name) {
                        diagnostics.push(Diagnostic {
                            code: DiagnosticCode::DuplicateFootnoteDefinition,
                            range: node.text_range(),
                            message: format!(
                                "각주 `{name}`이(가) 이미 정의되어 이 정의 내용은 무시됩니다."
                            ),
                            suggestion: None,
                        });
                    } else {
                        defined_footnotes.push(name);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_source(source: &str) -> Vec<Diagnostic> {
        analyze(&namumark_ast::parse(source))
    }

    fn codes(source: &str) -> Vec<DiagnosticCode> {
        analyze_source(source)
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    #[test]
    fn clean_document_has_no_diagnostics() {
        assert!(analyze_source("== 제목 ==\n본문입니다.").is_empty());
    }

    #[test]
    fn redirect_only_is_clean() {
        assert!(analyze_source("#redirect 대문").is_empty());
    }

    #[test]
    fn redirect_followed_by_content_warns() {
        let diagnostics = analyze_source("#redirect 대문\n보이지 않는 본문");
        assert_eq!(
            codes("#redirect 대문\n보이지 않는 본문"),
            vec![DiagnosticCode::RedirectTrailingContent]
        );
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.severity(), Severity::Warning);
        assert_eq!(diagnostic.category(), Category::Correctness);
    }

    #[test]
    fn comment_after_redirect_is_not_content() {
        assert!(analyze_source("#redirect 대문\n## 주석은 렌더되지 않는다").is_empty());
    }

    #[test]
    fn duplicate_redirect_warns_only_on_the_second() {
        assert_eq!(
            codes("#redirect 가\n#redirect 나"),
            vec![DiagnosticCode::DuplicateRedirect]
        );
    }

    #[test]
    fn duplicate_redirect_and_visible_content_both_warn() {
        assert_eq!(
            codes("#redirect 가\n#redirect 나\n본문"),
            vec![
                DiagnosticCode::DuplicateRedirect,
                DiagnosticCode::RedirectTrailingContent,
            ]
        );
    }

    #[test]
    fn heading_level_skip_suggests() {
        let diagnostics = analyze_source("= 1단계 =\n=== 3단계 ===");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::HeadingLevelSkip);
        assert_eq!(diagnostics[0].severity(), Severity::Suggestion);
    }

    #[test]
    fn sequential_heading_levels_are_clean() {
        assert!(analyze_source("= 1단계 =\n== 2단계 ==\n=== 3단계 ===").is_empty());
    }

    #[test]
    fn duplicate_category_is_suggested() {
        assert_eq!(
            codes("[[분류:음악]]\n[[분류:음악]]"),
            vec![DiagnosticCode::DuplicateCategory]
        );
    }

    #[test]
    fn distinct_categories_are_clean() {
        assert!(analyze_source("[[분류:음악]]\n[[분류:역사]]").is_empty());
    }

    #[test]
    fn duplicate_footnote_definition_warns() {
        assert_eq!(
            codes("[* 첫 정의][* 둘째 정의]\n본문[*a 내용][*a 다시]"),
            vec![DiagnosticCode::DuplicateFootnoteDefinition]
        );
    }

    #[test]
    fn footnote_reference_reuse_is_clean() {
        // 이름만 재사용(내용 없음)은 참조라 정상 — 병합이 의도된 동작이다.
        assert!(analyze_source("[*a 정의][*a]").is_empty());
    }
}
