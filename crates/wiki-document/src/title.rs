use std::fmt;

/// 이름공간 — 값 집합은 DB의 `namespace` 열거 테이블이 정본이고, 여기서는 이름만 다룬다.
///
/// 코드에 이름공간 목록을 박지 않는다(하드코딩 최소화). 본문 이름공간만은 접두사 없는
/// 제목의 기본값이라 상수로 안다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Namespace(String);

impl Namespace {
    pub const MAIN: &'static str = "문서";
    pub const FILE: &'static str = "파일";
    pub const CATEGORY: &'static str = "분류";
    pub const TEMPLATE: &'static str = "틀";
    pub const USER: &'static str = "사용자";

    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn main() -> Self {
        Self(Self::MAIN.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_main(&self) -> bool {
        self.0 == Self::MAIN
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// 이름공간 + 이름. 문서의 외부 식별자다(내부 정수 id는 노출하지 않는다).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentTitle {
    pub namespace: Namespace,
    pub name: String,
}

impl DocumentTitle {
    pub fn new(namespace: Namespace, name: impl Into<String>) -> Self {
        Self {
            namespace,
            name: name.into(),
        }
    }

    /// 사용자가 적은 제목을 이름공간과 이름으로 가른다.
    ///
    /// `문서:`는 이름공간이 아니라 "본문 이름공간 못박기" 표시라 떼기만 한다(제목이
    /// `/`로 시작해 하위 문서로 읽히는 걸 막는 용도 — docs/spec/namumark.md).
    /// 알려진 이름공간 목록은 호출자가 DB에서 읽어 넘긴다.
    pub fn parse(raw: &str, known_namespaces: &[String]) -> Self {
        let trimmed = raw.trim();
        if let Some((prefix, rest)) = trimmed.split_once(':')
            && known_namespaces.iter().any(|known| known == prefix)
        {
            let namespace = Namespace::new(prefix);
            if namespace.is_main() {
                return Self::new(Namespace::main(), rest);
            }
            return Self::new(namespace, rest);
        }
        Self::new(Namespace::main(), trimmed)
    }
}

impl fmt::Display for DocumentTitle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.namespace.is_main() {
            formatter.write_str(&self.name)
        } else {
            write!(formatter, "{}:{}", self.namespace, self.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn namespaces() -> Vec<String> {
        ["문서", "틀", "분류", "파일"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn 접두사가_없으면_본문_이름공간이다() {
        let title = DocumentTitle::parse("나무위키", &namespaces());
        assert_eq!(title.namespace.as_str(), "문서");
        assert_eq!(title.name, "나무위키");
        assert_eq!(title.to_string(), "나무위키");
    }

    #[test]
    fn 알려진_접두사는_이름공간으로_갈린다() {
        let title = DocumentTitle::parse("틀:상위 문서", &namespaces());
        assert_eq!(title.namespace.as_str(), "틀");
        assert_eq!(title.name, "상위 문서");
        assert_eq!(title.to_string(), "틀:상위 문서");
    }

    #[test]
    fn 모르는_접두사는_이름의_일부다() {
        let title = DocumentTitle::parse("C:\\윈도우", &namespaces());
        assert_eq!(title.namespace.as_str(), "문서");
        assert_eq!(title.name, "C:\\윈도우");
    }

    #[test]
    fn 문서_접두사는_본문_못박기라_떼어_낸다() {
        let title = DocumentTitle::parse("문서:/// (너 먹구름 비)", &namespaces());
        assert_eq!(title.namespace.as_str(), "문서");
        assert_eq!(title.name, "/// (너 먹구름 비)");
    }
}
