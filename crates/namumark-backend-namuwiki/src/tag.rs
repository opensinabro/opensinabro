//! 스트리밍 태그 라이터.
//!
//! 스택 전용 구조체로 Formatter에 즉시 방출한다(버퍼링·힙 할당 없음).
//! 속성값은 항상 이스케이프되고, 닫는 태그는 [`Tag::content`]가 방출하므로
//! 여닫이 짝이 구조적으로 보장된다.
//!
//! 유일한 예외는 형제 블록에 걸치는 헤딩 콘텐츠 래퍼(`wiki-heading-content`)로,
//! 문서 래퍼가 수동 관리한다.

use std::fmt::{self, Display, Formatter, Write as _};

pub(crate) struct Tag<'writer, 'buffer> {
    formatter: &'writer mut Formatter<'buffer>,
    name: &'static str,
}

pub(crate) fn tag<'writer, 'buffer>(
    formatter: &'writer mut Formatter<'buffer>,
    name: &'static str,
) -> Result<Tag<'writer, 'buffer>, fmt::Error> {
    write!(formatter, "<{name}")?;
    Ok(Tag { formatter, name })
}

impl<'buffer> Tag<'_, 'buffer> {
    pub(crate) fn attribute(self, name: &str, value: &dyn Display) -> Result<Self, fmt::Error> {
        write!(self.formatter, " {name}=\"{}\"", escape_attribute(value))?;
        Ok(self)
    }

    pub(crate) fn attribute_when(
        self,
        condition: bool,
        name: &str,
        value: &dyn Display,
    ) -> Result<Self, fmt::Error> {
        if condition {
            self.attribute(name, value)
        } else {
            Ok(self)
        }
    }

    pub(crate) fn attribute_if_some(
        self,
        name: &str,
        value: Option<&dyn Display>,
    ) -> Result<Self, fmt::Error> {
        match value {
            Some(value) => self.attribute(name, value),
            None => Ok(self),
        }
    }

    /// 값 없는 속성 (`allowfullscreen` 등)
    pub(crate) fn flag(self, name: &str) -> Result<Self, fmt::Error> {
        write!(self.formatter, " {name}")?;
        Ok(self)
    }

    /// 내용을 방출하고 닫는 태그까지 쓴다.
    pub(crate) fn content(
        self,
        body: impl FnOnce(&mut Formatter<'buffer>) -> fmt::Result,
    ) -> fmt::Result {
        self.formatter.write_char('>')?;
        body(self.formatter)?;
        write!(self.formatter, "</{}>", self.name)
    }

    /// [`Tag::content`] 후 개행 (블록 요소용)
    pub(crate) fn content_line(
        self,
        body: impl FnOnce(&mut Formatter<'buffer>) -> fmt::Result,
    ) -> fmt::Result {
        self.formatter.write_char('>')?;
        body(self.formatter)?;
        writeln!(self.formatter, "</{}>", self.name)
    }

    /// 내용 없는 태그 (`<br>`, `<img ...>`)
    pub(crate) fn void(self) -> fmt::Result {
        self.formatter.write_char('>')
    }

    pub(crate) fn void_line(self) -> fmt::Result {
        writeln!(self.formatter, ">")
    }
}

// ---- 이스케이프 어댑터 (중간 문자열 없이 Formatter로 직접 방출) ----

pub(crate) fn escape_text<T: Display>(value: T) -> impl Display {
    EscapeAdapter {
        value,
        escape_quotes: false,
    }
}

pub(crate) fn escape_attribute<T: Display>(value: T) -> impl Display {
    EscapeAdapter {
        value,
        escape_quotes: true,
    }
}

struct EscapeAdapter<T> {
    value: T,
    escape_quotes: bool,
}

impl<T: Display> Display for EscapeAdapter<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut escaper = EscapingWriter {
            inner: formatter,
            escape_quotes: self.escape_quotes,
        };
        write!(escaper, "{}", self.value)
    }
}

struct EscapingWriter<'writer, 'buffer> {
    inner: &'writer mut Formatter<'buffer>,
    escape_quotes: bool,
}

impl fmt::Write for EscapingWriter<'_, '_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for character in text.chars() {
            match character {
                '&' => self.inner.write_str("&amp;")?,
                '<' => self.inner.write_str("&lt;")?,
                '>' => self.inner.write_str("&gt;")?,
                '"' if self.escape_quotes => self.inner.write_str("&quot;")?,
                '\'' if self.escape_quotes => self.inner.write_str("&#x27;")?,
                _ => self.inner.write_char(character)?,
            }
        }
        Ok(())
    }
}

pub(crate) fn percent_encode(text: &str) -> impl Display + '_ {
    PercentEncode(text)
}

/// 각주 앵커의 퍼센트 인코딩. 문서 경로와 달리 **소문자** hex를 쓰고 `:`·`/`도 인코딩한다
/// (렌더확정: 각주 `[*예시 …]`의 참조가 `href='#fn-%ec%98%88%ec%8b%9c'`다).
pub(crate) fn percent_encode_anchor(text: &str) -> impl Display + '_ {
    PercentEncodeAnchor(text)
}

struct PercentEncodeAnchor<'text>(&'text str);

impl Display for PercentEncodeAnchor<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.0.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    formatter.write_char(byte as char)?
                }
                _ => write!(formatter, "%{byte:02x}")?,
            }
        }
        Ok(())
    }
}

struct PercentEncode<'text>(&'text str);

impl Display for PercentEncode<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.0.bytes() {
            match byte {
                // 나무위키는 문서 경로에서 이름공간 구분자 `:`, 하위 문서 구분자 `/`,
                // 동음이의 괄호 `()`를 인코딩하지 않는다
                // (렌더확정: `[[표(자료)]]` → `/w/%ED%91%9C(%EC%9E%90%EB%A3%8C)`).
                b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'_'
                | b'.'
                | b'~'
                | b':'
                | b'/'
                | b'('
                | b')' => formatter.write_char(byte as char)?,
                _ => write!(formatter, "%{byte:02X}")?,
            }
        }
        Ok(())
    }
}
