//! 뷰 접근자가 계산해 반환하는 소유값 타입과 언어 어휘 enum.
//!
//! 이 타입들은 구문 트리를 가리키지 않는 순수 값이다 — 렌더 IR·layout·backend가
//! 공유하므로 뷰로 두지 않는다. 뷰([`crate::node`])는 토큰을 읽어 이 값들을 만든다.

/// 틀 인자(`@이름@`)가 낄 수 있는 문자열.
///
/// 인자는 나무마크 구조를 만들지 않으므로(문법 도움말: 나무마크 자체 문법엔 매개변수를
/// 쓸 수 없다) 값이 정해지기 전에도 구조는 확정된다. 값 결정은 렌더 단계의 몫이다.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Template(pub Vec<Fragment>);

impl Template {
    /// 인자가 끼지 않은 평범한 문자열이면 그 내용.
    pub fn as_literal(&self) -> Option<&str> {
        match self.0.as_slice() {
            [] => Some(""),
            [Fragment::Text(text)] => Some(text),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|fragment| match fragment {
            Fragment::Text(text) => text.is_empty(),
            Fragment::Variable(_) => false,
        })
    }
}

impl From<&str> for Template {
    fn from(text: &str) -> Template {
        if text.is_empty() {
            return Template::default();
        }
        Template(vec![Fragment::Text(text.to_string())])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fragment {
    Text(String),
    Variable(Variable),
}

/// `@이름@` 또는 `@이름=기본값@`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    /// 인자가 넘어오지 않았을 때 쓸 값. 생략하면 빈 문자열이다.
    pub default: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableAttribute {
    pub scope: TableAttributeScope,
    pub name: String,
    pub value: Option<Template>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAttributeScope {
    Cell,
    /// 열 지정. `columns`는 이 옵션을 적은 자리까지 아는 칸 수다 — 나무위키는 셀 옵션을
    /// 왼쪽부터 처리해서, `<-3><colbgcolor=…>`는 세 열에 걸리지만
    /// `<colbgcolor=…><-4>`는 적힌 시점에 한 칸뿐이라 한 열에만 걸린다(렌더확정).
    Column {
        columns: u32,
    },
    Row,
    Table,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Unordered,
    Decimal,
    LowerAlphabet,
    UpperAlphabet,
    LowerRoman,
    UpperRoman,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageOption {
    pub name: String,
    pub value: Option<Template>,
}

/// 원문 표기를 되살린다. 값이 정해지기 전이므로 인자는 `@이름=기본값@` 그대로다.
impl std::fmt::Display for Template {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for fragment in &self.0 {
            match fragment {
                Fragment::Text(text) => formatter.write_str(text)?,
                Fragment::Variable(variable) => write!(formatter, "{variable}")?,
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for Variable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.default {
            Some(default) => write!(formatter, "@{}={default}@", self.name),
            None => write!(formatter, "@{}@", self.name),
        }
    }
}
