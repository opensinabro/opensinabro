//! 진단 데이터 모델.
//!
//! 분류는 [`DiagnosticCode`]가 정본이다 — 각 code가 자신의 심각도와 범주를 안다.
//! 새 진단 종류를 더하려면 변형과 그 매핑만 추가하면 된다.

use namumark_syntax::TextRange;

/// 진단이 저자에게 갖는 무게.
///
/// 나무마크는 무손실이라 파싱을 막는 오류가 없다. 모든 진단은 경고·제안·정보급이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// 저자 의도가 렌더에서 손실된다(예: 리다이렉트 뒤 내용은 표시되지 않음).
    Warning,
    /// 렌더는 정상이지만 더 나은 표현이 있다(추후 향상).
    Suggestion,
    /// 엔진이 이렇게 해석했음을 알린다.
    Info,
}

impl Severity {
    /// 편집기·API가 참조하는 안정적 식별자.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Suggestion => "suggestion",
            Self::Info => "info",
        }
    }
}

/// 진단이 다루는 관심사.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// 저자 의도와 실제 렌더가 어긋난다.
    Correctness,
    /// 구식 문법 — 권장 문법으로 대체 가능.
    Deprecation,
    /// 더 관용적인 표현 제안.
    Style,
    /// 아직 구현하지 않은 문법이라 드롭됨.
    Unsupported,
}

impl Category {
    /// 편집기·API가 참조하는 안정적 식별자.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Deprecation => "deprecation",
            Self::Style => "style",
            Self::Unsupported => "unsupported",
        }
    }
}

/// 진단 종류의 안정적 식별자. `Display`가 kebab-case 코드를 낸다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    /// 리다이렉트 뒤에 표시되지 않는 내용이 있다.
    RedirectTrailingContent,
    /// 리다이렉트가 둘 이상이라 두 번째부터 무시된다.
    DuplicateRedirect,
    /// 문서 제목 단계(heading level)가 한 단계를 건너뛴다.
    HeadingLevelSkip,
    /// 같은 분류가 여러 번 붙었다(뒤 것은 중복이라 효과 없음).
    DuplicateCategory,
    /// 같은 이름 각주가 여러 번 정의됐다(내용 있는 정의가 둘 이상).
    ///
    /// the seed는 같은 이름 각주를 병합해 첫 정의만 남기므로 뒤 정의 내용은 버려진다.
    DuplicateFootnoteDefinition,
    /// 엔진이 인식하지 못하는 매크로라 특화되지 않고 남는다.
    ///
    /// 아래 둘은 문맥 의존 검사다 — 매크로·문서 어휘가 resolve에 있으므로 render가 낸다.
    UnsupportedMacro,
    /// 인식하는 매크로이지만 인자가 없거나 잘못돼 특화되지 않는다.
    InvalidMacroArgument,
    /// `[include(...)]` 대상 문서가 존재하지 않는다.
    IncludeTargetMissing,
    /// 리다이렉트 대상이 자기 문서라 순환한다.
    SelfRedirect,
}

impl DiagnosticCode {
    pub fn severity(self) -> Severity {
        match self {
            Self::RedirectTrailingContent
            | Self::DuplicateRedirect
            | Self::DuplicateFootnoteDefinition
            | Self::InvalidMacroArgument
            | Self::IncludeTargetMissing
            | Self::SelfRedirect => Severity::Warning,
            Self::HeadingLevelSkip | Self::DuplicateCategory => Severity::Suggestion,
            Self::UnsupportedMacro => Severity::Info,
        }
    }

    pub fn category(self) -> Category {
        match self {
            Self::RedirectTrailingContent
            | Self::DuplicateRedirect
            | Self::DuplicateFootnoteDefinition
            | Self::InvalidMacroArgument
            | Self::IncludeTargetMissing
            | Self::SelfRedirect => Category::Correctness,
            Self::HeadingLevelSkip | Self::DuplicateCategory => Category::Style,
            Self::UnsupportedMacro => Category::Unsupported,
        }
    }

    /// 편집기·API가 참조하는 안정적 식별자.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RedirectTrailingContent => "redirect-trailing-content",
            Self::DuplicateRedirect => "duplicate-redirect",
            Self::HeadingLevelSkip => "heading-level-skip",
            Self::DuplicateCategory => "duplicate-category",
            Self::DuplicateFootnoteDefinition => "duplicate-footnote-definition",
            Self::UnsupportedMacro => "unsupported-macro",
            Self::InvalidMacroArgument => "invalid-macro-argument",
            Self::IncludeTargetMissing => "include-target-missing",
            Self::SelfRedirect => "self-redirect",
        }
    }
}

impl std::fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// 향상 제안이 제시하는 원문 치환. 편집기의 quick-fix에 그대로 매핑된다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    pub range: TextRange,
    pub new_text: String,
}

/// 문서 한 지점에 대한 진단.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    /// 원문에서 이 진단이 가리키는 범위.
    pub range: TextRange,
    pub message: String,
    /// 자동 적용 가능한 향상 제안(있으면).
    pub suggestion: Option<Replacement>,
}

impl Diagnostic {
    pub fn severity(&self) -> Severity {
        self.code.severity()
    }

    pub fn category(&self) -> Category {
        self.code.category()
    }
}
