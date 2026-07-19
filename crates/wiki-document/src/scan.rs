//! 렌더 전에 원문이 물어볼 만한 제목을 훑는다.
//!
//! 이것은 **의미론이 아니라 추측**이다. 목적은 렌더가 실제로 묻기 전에 답을 배치로
//! 받아 두는 것 하나뿐이라, 넉넉히 잡아 헛조회를 하는 것은 손해가 아니고 놓치는 것도
//! 틀린 결과를 내지 않는다 — 놓친 제목은 [`crate::render`]의 수렴 루프가 받아낸다.
//!
//! 다만 상대 제목만은 resolve와 같은 모양으로 편다. 펴지 않으면 스캔이 담는 키와 렌더가
//! 묻는 키가 어긋나, 하위 문서를 거는 문서에서 프리페치가 통째로 헛돌기 때문이다.

use namumark_ast::{Block, Inline, Template};

/// 원문 한 편이 물어볼 법한 것들. resolve가 물을 모양으로 맞춰 둔 제목이다.
#[derive(Default)]
pub(crate) struct Candidates {
    pub links: Vec<String>,
    pub includes: Vec<String>,
    pub files: Vec<String>,
}

impl Candidates {
    /// 같은 문서를 여러 번 거는 것은 흔하다. 배치에 중복을 실어 보낼 이유가 없다.
    pub fn dedup(&mut self) {
        for titles in [&mut self.links, &mut self.includes, &mut self.files] {
            titles.sort_unstable();
            titles.dedup();
        }
    }
}

/// 원문을 파싱해 후보를 모은다.
///
/// 구문 트리(rowan)는 `Send`가 아니므로 이 함수 안에서 수명이 끝나야 한다 — 호출자가
/// 결과를 들고 DB를 기다리기 때문이다. 그래서 소유 `String`만 남기고 트리는 버린다.
pub(crate) fn scan(source: &str, current_title: &str) -> Candidates {
    let document = namumark_parser::parse(source);
    let mut scanner = Scanner {
        current_title,
        found: Candidates::default(),
    };
    scanner.blocks(&document.blocks());
    let mut found = scanner.found;
    found.dedup();
    found
}

struct Scanner<'a> {
    current_title: &'a str,
    found: Candidates,
}

impl Scanner<'_> {
    fn blocks(&mut self, blocks: &[Block]) {
        for block in blocks {
            match block {
                Block::Heading(heading) => self.inlines(&heading.content()),
                Block::Paragraph(paragraph) => self.inlines(&paragraph.inlines()),
                Block::Quote(quote) => self.blocks(&quote.blocks()),
                Block::Indent(indent) => self.blocks(&indent.blocks()),
                Block::List(list) => {
                    for item in list.items() {
                        self.blocks(&item.blocks());
                    }
                }
                Block::Table(table) => {
                    if let Some(caption) = table.caption() {
                        self.inlines(&caption);
                    }
                    for row in table.rows() {
                        for cell in row.cells {
                            self.blocks(&cell.blocks);
                        }
                    }
                }
                Block::HorizontalRule | Block::Comment(_) | Block::Redirect(_) => {}
            }
        }
    }

    fn inlines(&mut self, inlines: &[Inline]) {
        for inline in inlines {
            match inline {
                Inline::Link(link) => {
                    if let Some(written) = literal_of(&link.target()) {
                        let target = self.resolve_relative(written);
                        push(&mut self.found.links, &target);
                    }
                    if let Some(display) = link.display() {
                        self.inlines(&display);
                    }
                }
                Inline::Image(image) => {
                    if let Some(file_name) = literal_of(&image.file_name()) {
                        push(&mut self.found.files, file_name);
                    }
                }
                Inline::Macro(macro_call) => {
                    if macro_call.name().eq_ignore_ascii_case("include")
                        && let Some(argument) = macro_call.argument()
                        && let Some(literal) = literal_of(&argument)
                        // `[include(틀:X, 인자=값)]`의 첫 조각이 대상 문서다.
                        && let Some(target) = literal.split(',').next()
                    {
                        push(&mut self.found.includes, target);
                    }
                }
                Inline::Footnote(footnote) => self.inlines(&footnote.content()),
                Inline::Bold(styled) => self.inlines(&styled.content()),
                Inline::Italic(styled) => self.inlines(&styled.content()),
                Inline::Strikethrough(styled) => self.inlines(&styled.content()),
                Inline::Underline(styled) => self.inlines(&styled.content()),
                Inline::Superscript(styled) => self.inlines(&styled.content()),
                Inline::Subscript(styled) => self.inlines(&styled.content()),
                Inline::Colored(colored) => self.inlines(&colored.content()),
                Inline::Sized(sized) => self.inlines(&sized.content()),
                Inline::WikiStyle(wiki_style) => self.blocks(&wiki_style.blocks()),
                Inline::Folding(folding) => self.blocks(&folding.blocks()),
                Inline::Conditional(conditional) => self.blocks(&conditional.blocks()),
                // 분류는 분류 문서의 존재를 묻지 않고, 나머지는 문서를 가리키지 않는다.
                Inline::Text(_)
                | Inline::LineBreak
                | Inline::Literal(_)
                | Inline::Category(_)
                | Inline::Variable(_)
                | Inline::CodeBlock(_)
                | Inline::Html(_) => {}
            }
        }
    }

    /// `resolve_link_target`(namumark-render)과 같은 규칙. 둘이 어긋나면 프리페치가
    /// 빗나가므로 여기 손댈 일이 생기면 그쪽도 같이 본다.
    fn resolve_relative(&self, written: &str) -> String {
        if let Some(absolute) = written.strip_prefix("문서:") {
            return absolute.to_string();
        }
        if let Some(rest) = written.strip_prefix("../") {
            let parent = match self.current_title.rsplit_once('/') {
                Some((parent, _)) => parent,
                None => self.current_title,
            };
            return if rest.is_empty() {
                parent.to_string()
            } else {
                format!("{parent}/{rest}")
            };
        }
        match written.strip_prefix('/') {
            Some(child) => format!("{}/{child}", self.current_title),
            None => written.to_string(),
        }
    }
}

/// 인자(`@이름@`)가 낀 제목은 스코프 없이는 알 수 없으므로 건너뛴다 — 수렴 루프의 몫이다.
fn literal_of(template: &Template) -> Option<&str> {
    template.as_literal()
}

fn push(target: &mut Vec<String>, text: &str) {
    let trimmed = text.trim();
    // 바깥 링크는 문서가 아니고, 앵커만 있는 링크는 제 문서를 가리킨다.
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.contains("://") {
        return;
    }
    target.push(trimmed.to_owned());
}

#[cfg(test)]
mod tests {
    use super::scan;

    #[test]
    fn 링크와_틀과_파일을_모은다() {
        let found = scan(
            "[[가나다]] [[라마바|보임글]]\n[include(틀:안내, 색=red)]\n[[파일:그림.png]]",
            "현재문서",
        );
        assert_eq!(
            found.links,
            vec!["가나다".to_string(), "라마바".to_string()]
        );
        assert_eq!(found.includes, vec!["틀:안내".to_string()]);
        assert_eq!(found.files, vec!["그림.png".to_string()]);
    }

    /// 상대 제목은 resolve와 같은 모양으로 펴야 프리페치가 맞는다.
    #[test]
    fn 상대_제목을_렌더러와_같은_규칙으로_편다() {
        let found = scan("[[/하위]] [[../]] [[../형제]] [[문서:/절대]]", "부모/자식");
        assert_eq!(
            found.links,
            vec![
                "/절대".to_string(),
                "부모".to_string(),
                "부모/자식/하위".to_string(),
                "부모/형제".to_string(),
            ]
        );
    }

    /// 링크는 깊이 어디에나 있을 수 있다. 하나라도 빠지면 그 문서는 렌더가 두 번 돈다.
    #[test]
    fn 중첩된_자리의_링크도_빠뜨리지_않는다() {
        let found = scan(
            "== [[제목링크]] ==\n> [[인용링크]]\n * [[리스트링크]]\n||[[표링크]]||\n'''[[굵은링크]]'''\n각주[* [[각주링크]]]",
            "현재문서",
        );
        for expected in [
            "제목링크",
            "인용링크",
            "리스트링크",
            "표링크",
            "굵은링크",
            "각주링크",
        ] {
            assert!(
                found.links.iter().any(|link| link == expected),
                "{expected}를 놓쳤다: {:?}",
                found.links
            );
        }
    }

    #[test]
    fn 바깥_링크와_앵커만_있는_링크는_건너뛴다() {
        let found = scan("[[https://example.com]] [[#앵커]] [[  ]]", "현재문서");
        assert!(found.links.is_empty(), "{:?}", found.links);
    }

    #[test]
    fn 같은_제목은_한_번만_묻는다() {
        let found = scan("[[같은문서]] [[같은문서]] [[같은문서]]", "현재문서");
        assert_eq!(found.links, vec!["같은문서".to_string()]);
    }
}
