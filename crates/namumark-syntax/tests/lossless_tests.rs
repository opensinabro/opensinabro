//! 무손실 불변식 검증: 어떤 입력이든 `parse_syntax(x).text() == x`.
//! 그리고 임의 오프셋에서 조상 체인으로 의미를 파악할 수 있어야 한다.

use namumark_syntax::SyntaxKind;
use std::fs;
use std::path::Path;

fn assert_roundtrip(label: &str, source: &str) {
    let tree = namumark_syntax::parse(source);
    assert_eq!(tree.text(), source, "라운드트립 위반: {label}");
}

const KITCHEN_SINK: &str = include_str!("../../../fixtures/corpus/termination_corpus.namu");

#[test]
fn fixtures_roundtrip() {
    let fixtures_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    for entry in fs::read_dir(&fixtures_directory).expect("fixtures 읽기 실패") {
        let path = entry.expect("entry").path();
        if path
            .extension()
            .is_some_and(|extension| extension == "namu")
        {
            let source = fs::read_to_string(&path).expect("픽스처 읽기 실패");
            assert_roundtrip(&path.display().to_string(), &source);
        }
    }
}

#[test]
fn kitchen_sink_prefixes_and_suffixes_roundtrip() {
    assert_roundtrip("kitchen-sink", KITCHEN_SINK);
    for (index, _) in KITCHEN_SINK.char_indices() {
        assert_roundtrip("prefix", &KITCHEN_SINK[..index]);
        assert_roundtrip("suffix", &KITCHEN_SINK[index..]);
    }
}

#[test]
fn pathological_inputs_roundtrip() {
    let mut cases: Vec<(String, String)> = Vec::new();
    for marker in [
        "'", "-", "=", "[", "]", "{", "}", "|", "~", "_", "^", ",", ">", "*", "#", "\\", "\n", " ",
    ] {
        cases.push((format!("flood:{marker:?}"), marker.repeat(500)));
    }
    cases.push(("deep-quote".into(), format!("{}인용", ">".repeat(200))));
    cases.push(("deep-indent".into(), format!("{}본문", " ".repeat(200))));
    cases.push((
        "deep-footnote".into(),
        format!("{}끝{}", "[* ".repeat(100), "]".repeat(100)),
    ));
    cases.push(("pipes".into(), "||".repeat(200)));
    cases.push(("caption-only".into(), "|캡션|".repeat(100)));
    cases.push(("crlf".into(), "= 제목 =\r\n|| A ||\r\n본문\r\n".into()));
    cases.push((
        "unicode".into(),
        "'''🎉👨‍👩‍👧‍👦''' [[한글✨|{{{#red 🔥}}}]] ~~é́́조합~~".into(),
    ));
    for (label, source) in cases {
        assert_roundtrip(&label, &source);
    }
}

#[test]
fn position_query_reveals_semantic_path() {
    let source = "||<:>셀\n{{{#!folding 보기\n|| 안 || 표 ||\n}}}\n끝 ||";
    let tree = namumark_syntax::parse(source);
    let offset = source.find('안').expect("대상 문자") as u32;
    let root = tree.root();
    let deepest = root
        .descendants()
        .filter(|node| node.text_range().contains(offset.into()))
        .last()
        .expect("오프셋을 덮는 노드");
    let ancestor_kinds: Vec<SyntaxKind> = std::iter::once(deepest.kind())
        .chain(deepest.ancestors().map(|ancestor| ancestor.kind()))
        .collect();
    for expected in [
        SyntaxKind::TableCell,
        SyntaxKind::Folding,
        SyntaxKind::Table,
        SyntaxKind::Document,
    ] {
        assert!(
            ancestor_kinds.contains(&expected),
            "조상 체인에 {expected:?}가 없다: {ancestor_kinds:?}"
        );
    }
    // 중첩: 바깥 표 → 셀 → folding → 안쪽 표 → 셀 순의 경로가 존재한다
    let table_count = ancestor_kinds
        .iter()
        .filter(|kind| **kind == SyntaxKind::Table)
        .count();
    assert_eq!(table_count, 2, "안쪽·바깥 표가 모두 경로에 있어야 한다");
}
