//! 실제 나무위키 문서 픽스처의 나무위키 동등 마크업 골든 스냅샷.
//! 갱신: `UPDATE_GOLDEN=1 cargo test -p namumark-render --test namuwiki_markup_golden_tests`

use namumark_backend_namuwiki::NamuwikiMarkup;
use namumark_ir::RenderBackend;
use namumark_render::EmptyContext;
use std::fs;
use std::path::Path;

#[test]
fn namuwiki_markup_golden_fixtures() {
    let fixtures_directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .canonicalize()
        .expect("픽스처 디렉토리");
    let golden_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden-namuwiki");
    fs::create_dir_all(&golden_directory).expect("golden 디렉토리 생성 실패");
    let update = std::env::var("UPDATE_GOLDEN").is_ok();

    let mut slugs: Vec<String> = fs::read_dir(&fixtures_directory)
        .expect("fixtures 읽기 실패")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".namu").map(str::to_string)
        })
        .collect();
    slugs.sort();
    assert!(!slugs.is_empty(), "픽스처가 없습니다");

    let mut failures = Vec::new();
    for slug in &slugs {
        let source = fs::read_to_string(fixtures_directory.join(format!("{slug}.namu")))
            .expect("픽스처 읽기 실패");
        let document = namumark_parser::parse(&source);
        let tree = namumark_render::build_render_tree(&document, &EmptyContext);
        let rendered = NamuwikiMarkup.render(&tree);
        let golden_path = golden_directory.join(format!("{slug}.html"));
        if update {
            fs::write(&golden_path, &rendered).expect("골든 쓰기 실패");
            continue;
        }
        match fs::read_to_string(&golden_path) {
            Ok(expected) if expected == rendered => {}
            Ok(_) => failures.push(format!(
                "{slug}: HTML 골든 불일치 (UPDATE_GOLDEN=1 로 갱신)"
            )),
            Err(_) => failures.push(format!("{slug}: 골든 파일 없음 (UPDATE_GOLDEN=1 로 생성)")),
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
