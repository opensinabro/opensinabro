use crate::DocumentTitle;

/// 문서가 다른 문서를 가리키는 방식. DB의 `document_reference_kind` 열거와 짝이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    Link,
    Include,
    Redirect,
    Image,
    Category,
}

impl ReferenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Include => "include",
            Self::Redirect => "redirect",
            Self::Image => "image",
            Self::Category => "category",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferenceTarget {
    pub title: DocumentTitle,
    pub kind: ReferenceKind,
}
