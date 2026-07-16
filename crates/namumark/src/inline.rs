use crate::ast::{
    Category, ColoredText, Footnote, Image, ImageOption, Inline, Link, Macro, SizedText,
};

pub(crate) fn parse_inlines(source: &str) -> Vec<Inline> {
    InlineParser::new(source).run()
}

type StyleConstructor = fn(Vec<Inline>) -> Inline;

const STYLE_MARKERS: &[(&str, StyleConstructor)] = &[
    ("'''", Inline::Bold),
    ("''", Inline::Italic),
    ("~~", Inline::Strikethrough),
    ("--", Inline::Strikethrough),
    ("__", Inline::Underline),
    ("^^", Inline::Superscript),
    (",,", Inline::Subscript),
];

struct InlineParser<'source> {
    source: &'source str,
    position: usize,
    inlines: Vec<Inline>,
    text_buffer: String,
}

impl<'source> InlineParser<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            position: 0,
            inlines: Vec::new(),
            text_buffer: String::new(),
        }
    }

    fn run(mut self) -> Vec<Inline> {
        while self.position < self.source.len() {
            let rest = &self.source[self.position..];
            let consumed = if rest.starts_with('\\') {
                self.consume_escape()
            } else if rest.starts_with("{{{") {
                self.consume_literal()
            } else if rest.starts_with("[[") {
                self.consume_link()
            } else if rest.starts_with("[*") {
                self.consume_footnote()
            } else if rest.starts_with('[') {
                self.consume_macro()
            } else {
                self.consume_styled()
            };
            if !consumed {
                self.consume_text_character();
            }
        }
        self.flush_text();
        self.inlines
    }

    fn consume_escape(&mut self) -> bool {
        let mut characters = self.source[self.position..].chars();
        characters.next();
        let Some(escaped) = characters.next() else {
            return false;
        };
        self.text_buffer.push(escaped);
        self.position += 1 + escaped.len_utf8();
        true
    }

    fn consume_literal(&mut self) -> bool {
        let rest = &self.source[self.position..];
        let Some(end) = find_matching_braces(rest) else {
            return false;
        };
        let content = &rest[3..end];
        self.push_inline(interpret_literal(content));
        self.position += end + 3;
        true
    }

    fn consume_link(&mut self) -> bool {
        let rest = &self.source[self.position..];
        let Some(end) = find_matching_double_bracket(rest) else {
            return false;
        };
        let body = &rest[2..end];
        if body.is_empty() {
            return false;
        }
        let (target, display_source) = match body.split_once('|') {
            Some((target, display_source)) => (target, Some(display_source)),
            None => (body, None),
        };
        self.push_inline(build_link(target, display_source));
        self.position += end + 2;
        true
    }

    fn consume_footnote(&mut self) -> bool {
        let rest = &self.source[self.position..];
        let Some(end) = find_matching_bracket(rest) else {
            return false;
        };
        let body = &rest[2..end];
        let (name, content) = match body.split_once(' ') {
            Some((name, content)) => (name, content),
            None => (body, ""),
        };
        let name = (!name.is_empty()).then(|| name.to_string());
        self.push_inline(Inline::Footnote(Footnote {
            name,
            content: parse_inlines(content),
        }));
        self.position += end + 1;
        true
    }

    fn consume_macro(&mut self) -> bool {
        let rest = &self.source[self.position..];
        let Some(end) = find_matching_bracket(rest) else {
            return false;
        };
        let body = &rest[1..end];
        let (name, argument) = match body.split_once('(') {
            Some((name, argument)) => {
                let Some(argument) = argument.strip_suffix(')') else {
                    return false;
                };
                (name, Some(argument.to_string()))
            }
            None => (body, None),
        };
        if name.is_empty() || !name.chars().all(char::is_alphanumeric) {
            return false;
        }
        self.push_inline(Inline::Macro(Macro {
            name: name.to_string(),
            argument,
        }));
        self.position += end + 1;
        true
    }

    fn consume_styled(&mut self) -> bool {
        let rest = &self.source[self.position..];
        for &(marker, construct) in STYLE_MARKERS {
            if !rest.starts_with(marker) {
                continue;
            }
            let inner = &rest[marker.len()..];
            let Some(offset) = inner.find(marker) else {
                continue;
            };
            if offset == 0 {
                continue;
            }
            let content = parse_inlines(&inner[..offset]);
            self.push_inline(construct(content));
            self.position += marker.len() * 2 + offset;
            return true;
        }
        false
    }

    fn consume_text_character(&mut self) {
        let character = self.source[self.position..]
            .chars()
            .next()
            .expect("position is within bounds");
        self.text_buffer.push(character);
        self.position += character.len_utf8();
    }

    fn push_inline(&mut self, inline: Inline) {
        self.flush_text();
        self.inlines.push(inline);
    }

    fn flush_text(&mut self) {
        if !self.text_buffer.is_empty() {
            self.inlines
                .push(Inline::Text(std::mem::take(&mut self.text_buffer)));
        }
    }
}

fn build_link(target: &str, display_source: Option<&str>) -> Inline {
    if let Some(file_name) = strip_link_prefix(target, &["파일:", "file:"]) {
        return Inline::Image(Image {
            file_name: file_name.to_string(),
            options: parse_image_options(display_source.unwrap_or("")),
        });
    }
    if let Some(name) = strip_link_prefix(target, &["분류:", "category:"]) {
        return Inline::Category(Category {
            name: name.to_string(),
        });
    }
    // `[[:파일:x]]`는 파일 삽입이 아니라 해당 문서로 가는 일반 링크다.
    let target = match target.strip_prefix(':') {
        Some(stripped)
            if strip_link_prefix(stripped, &["파일:", "file:", "분류:", "category:"]).is_some() =>
        {
            stripped
        }
        _ => target,
    };
    let (target, anchor) = split_anchor(target);
    Inline::Link(Link {
        target: target.to_string(),
        anchor,
        display: display_source.map(parse_inlines),
    })
}

fn strip_link_prefix<'target>(target: &'target str, prefixes: &[&str]) -> Option<&'target str> {
    for prefix in prefixes {
        if target.len() >= prefix.len()
            && target.is_char_boundary(prefix.len())
            && target[..prefix.len()].eq_ignore_ascii_case(prefix)
        {
            return Some(&target[prefix.len()..]);
        }
    }
    None
}

fn split_anchor(target: &str) -> (&str, Option<String>) {
    if strip_link_prefix(target, &["http://", "https://", "ftp://"]).is_some() {
        return (target, None);
    }
    match target.rsplit_once('#') {
        Some((page, anchor)) if !anchor.is_empty() => (page, Some(anchor.to_string())),
        _ => (target, None),
    }
}

fn parse_image_options(source: &str) -> Vec<ImageOption> {
    source
        .split('&')
        .filter(|part| !part.trim().is_empty())
        .map(|part| match part.split_once('=') {
            Some((name, value)) => ImageOption {
                name: name.trim().to_string(),
                value: Some(value.trim().to_string()),
            },
            None => ImageOption {
                name: part.trim().to_string(),
                value: None,
            },
        })
        .collect()
}

fn interpret_literal(content: &str) -> Inline {
    if let Some(html) = content.strip_prefix("#!html ") {
        return Inline::Html(html.to_string());
    }
    if let Some((level, rest)) = parse_size_marker(content) {
        return Inline::Sized(SizedText {
            level,
            content: parse_inlines(rest),
        });
    }
    if let Some((color, dark_color, rest)) = parse_color_marker(content) {
        return Inline::Colored(ColoredText {
            color,
            dark_color,
            content: parse_inlines(rest),
        });
    }
    Inline::Literal(content.to_string())
}

pub(crate) fn parse_size_marker(content: &str) -> Option<(i8, &str)> {
    let (sign, rest) = if let Some(rest) = content.strip_prefix('+') {
        (1i8, rest)
    } else if let Some(rest) = content.strip_prefix('-') {
        (-1i8, rest)
    } else {
        return None;
    };
    let digit = rest.chars().next()?;
    if !('1'..='5').contains(&digit) {
        return None;
    }
    let level = sign * (digit as u8 - b'0') as i8;
    let rest = &rest[1..];
    if rest.is_empty() {
        return Some((level, ""));
    }
    rest.strip_prefix(' ').map(|rest| (level, rest))
}

pub(crate) fn parse_color_marker(content: &str) -> Option<(String, Option<String>, &str)> {
    if !content.starts_with('#') {
        return None;
    }
    let (specification, rest) = match content.split_once(' ') {
        Some((specification, rest)) => (specification, rest),
        None => (content, ""),
    };
    let (first, second) = match specification.split_once(',') {
        Some((first, second)) => (first, Some(second)),
        None => (specification, None),
    };
    let color = parse_color(first)?;
    let dark_color = match second {
        Some(second) => Some(parse_color(second)?),
        None => None,
    };
    Some((color, dark_color, rest))
}

// 색상은 `#` 접두사로 표기한다. hex는 `#` 포함 그대로, 색상 이름은 `#`를 제거해 CSS 값으로 보관한다.
fn parse_color(source: &str) -> Option<String> {
    let body = source.strip_prefix('#')?;
    if matches!(body.len(), 3 | 6) && body.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(source.to_string())
    } else if !body.is_empty() && body.chars().all(char::is_alphanumeric) {
        Some(body.to_string())
    } else {
        None
    }
}

// `[[문서|[[파일:...]]]]`처럼 표시부에 링크가 중첩될 수 있어 `[[`/`]]` 깊이를 추적한다.
fn find_matching_double_bracket(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"[[") {
            depth += 1;
            index += 2;
        } else if bytes[index..].starts_with(b"]]") {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    None
}

pub(crate) fn find_matching_braces(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"{{{") {
            depth += 1;
            index += 3;
        } else if bytes[index..].starts_with(b"}}}") {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    None
}

fn find_matching_bracket(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, byte) in text.bytes().enumerate() {
        match byte {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}
