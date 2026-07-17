//! `#!wiki`가 실어 온 CSS를 나무위키가 받아들이는 만큼만 통과시킨다.
//!
//! 위키 입력이 CSS로 나가는 통로라 `#!html`과 같은 부류다. 나무위키는 여기를 그냥
//! 흘려보내지 않고 걸러 낸다 — 렌더확정 근거:
//!
//! - `image-rendering`은 속성째 사라진다(문법 도움말은 동작하는 것처럼 서술하지만
//!   the seed 렌더에는 없다).
//! - 값이 무효한 선언도 버린다. `틀:다른 뜻`의 `display: @paragraph1=inl@@anchor1=ine@`가
//!   `display: 5ine`으로 채워지면 the seed는 그 선언을 통째로 버린다.
//!
//! 증거가 있는 것만 막는다. 나머지는 통과시킨다 — 목록을 넘겨짚으면 멀쩡한 CSS가
//! 조용히 사라진다.

use std::fmt::{self, Display, Formatter};

/// 걸러 낸 뒤 남는 style 선언들.
pub(crate) struct SupportedStyle<'source>(pub &'source str);

impl SupportedStyle<'_> {
    /// 남는 선언이 하나도 없으면 style 속성 자체를 두지 않는다.
    pub(crate) fn is_empty(&self) -> bool {
        declarations(self.0).next().is_none()
    }
}

impl Display for SupportedStyle<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for (index, (name, value)) in declarations(self.0).enumerate() {
            if index > 0 {
                formatter.write_str("; ")?;
            }
            write!(formatter, "{name}: {value}")?;
        }
        Ok(())
    }
}

fn declarations(source: &str) -> impl Iterator<Item = (&str, &str)> {
    source.split(';').filter_map(|declaration| {
        let (name, value) = declaration.split_once(':')?;
        let (name, value) = (name.trim(), value.trim());
        (!name.is_empty() && !value.is_empty() && is_supported(name, value))
            .then_some((name, value))
    })
}

fn is_supported(name: &str, value: &str) -> bool {
    if has_nested_call(value) {
        return false;
    }
    let name = name.to_ascii_lowercase();
    match name.as_str() {
        "image-rendering" => false,
        "display" => is_display_keyword(&value.to_ascii_lowercase()),
        _ => true,
    }
}

/// 함수 호출 안에 또 함수 호출이 있는가.
///
/// 나무위키는 이런 값을 통째로 버린다 — `repeating-linear-gradient(45deg, #1f719a 6%, …)`는
/// 받지만 `linear-gradient(0deg, rgba(255,255,255,.875), …)`는 안 받는다. 렌더에 중첩 호출이
/// 든 선언이 하나도 없다(`hsla(` 44건·`repeating-` 7건은 있어도 함수 안 함수는 0건).
fn has_nested_call(value: &str) -> bool {
    let mut depth = 0usize;
    let mut previous = ' ';
    for character in value.chars() {
        match character {
            '(' => {
                // 여는 괄호 앞이 이름의 일부면 함수 호출이다.
                if depth > 0 && (previous.is_alphanumeric() || previous == '-') {
                    return true;
                }
                depth += 1;
            }
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        previous = character;
    }
    false
}

fn is_display_keyword(value: &str) -> bool {
    matches!(
        value,
        "block"
            | "contents"
            | "flex"
            | "flow-root"
            | "grid"
            | "inline"
            | "inline-block"
            | "inline-flex"
            | "inline-grid"
            | "inline-table"
            | "list-item"
            | "none"
            | "table"
            | "table-caption"
            | "table-cell"
            | "table-column"
            | "table-column-group"
            | "table-footer-group"
            | "table-header-group"
            | "table-row"
            | "table-row-group"
    )
}
