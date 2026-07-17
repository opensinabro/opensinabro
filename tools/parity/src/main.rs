//! 나무위키(the seed) 파리티 검사 도구.
//!
//! 두 가지를 한다.
//!
//! - `compare` — the seed 렌더 결과와 우리 렌더 결과를 정규화해 대조하고 차이를 보고한다.
//!   알려진 차이(`known-differences.txt`)는 통과시키고 새 차이가 있으면 종료 코드 1.
//! - `report` — 문서 뭉치에서 우리가 해석하지 못한 매크로·옵션을 빈도순으로 집계한다.
//!   "the seed엔 있는데 우리가 모르는 문법"을 자동으로 드러낸다.
//!
//! 코퍼스는 `tools/fetch-parity-corpus.py`가 `target/parity-corpus/`에 받아 둔다.
//! 근거와 기법은 docs/design/04-namuwiki-parity.md 참고.

mod corpus;
mod normalize;
mod report;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "compare".to_string());
    let rest: Vec<String> = arguments.collect();

    match command.as_str() {
        "compare" => compare_command(&rest),
        "report" => report_command(&rest),
        "dump" => dump_command(&rest),
        other => {
            eprintln!("알 수 없는 명령: {other}");
            eprintln!("사용법: parity <compare|report|dump> [경로...]");
            ExitCode::from(2)
        }
    }
}

fn corpus_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/parity-corpus")
}

fn known_differences_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("known-differences.txt")
}

/// 처음 갈라지는 자리를 앞뒤 조각과 함께 펼쳐 보인다. 앞에서부터 맞춰 나가는 작업에는
/// 조각 하나만 봐서는 어디인지 알 수 없다.
fn print_divergence(
    difference: &corpus::Difference,
    expected: &[normalize::Fragment],
    actual: &[normalize::Fragment],
) {
    const LEADING: usize = 8;
    const FOLLOWING: usize = 6;
    let at = difference.at;
    println!("  ── 여기까지 같음 (조각 {})", at.expected);
    for fragment in &expected[at.expected.saturating_sub(LEADING)..at.expected] {
        println!("      {}", fragment.render());
    }
    println!("  ── 여기서 갈림");
    println!("    the seed:");
    for fragment in &expected[at.expected..expected.len().min(at.expected + FOLLOWING)] {
        println!("      {}", fragment.render());
    }
    println!("    우리:");
    for fragment in &actual[at.actual..actual.len().min(at.actual + FOLLOWING)] {
        println!("      {}", fragment.render());
    }
}

/// 몇 군데나 펼쳐 볼지. 맨 앞 하나가 근원일 때가 많지만, 근거가 모자라 미뤄 둔
/// 차이가 앞에 있으면 그 뒤를 봐야 한다.
fn divergences_to_show() -> usize {
    std::env::var("PARITY_DIVERGENCES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
}

fn compare_command(arguments: &[String]) -> ExitCode {
    let directory = arguments
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(corpus_directory);
    let Ok(corpus) = corpus::Corpus::load(&directory) else {
        eprintln!("코퍼스를 읽지 못했습니다: {}", directory.display());
        eprintln!("먼저 `python3 tools/fetch-parity-corpus.py`를 실행하세요.");
        return ExitCode::from(2);
    };
    let cases = corpus.comparable_cases();
    if cases.is_empty() {
        eprintln!("대조할 케이스가 없습니다 ({}).", directory.display());
        eprintln!("먼저 `python3 tools/fetch-parity-corpus.py`를 실행하세요.");
        return ExitCode::from(2);
    }

    let known = corpus::KnownDifferences::load(&known_differences_path());
    let mut new_differences = 0;
    let mut known_hits = 0;
    let mut kinds: std::collections::BTreeMap<String, (usize, String)> = Default::default();

    for case in &cases {
        let ours = corpus.render(case, &corpus);
        let expected = normalize::normalize(&case.rendered);
        let actual = normalize::normalize(&ours);
        let differences = corpus::diff(&expected, &actual);
        if differences.is_empty() {
            println!("일치  {}", case.document);
            continue;
        }
        let (known_here, fresh): (Vec<_>, Vec<_>) = differences
            .into_iter()
            .partition(|difference| known.matches(difference));
        known_hits += known_here.len();
        if fresh.is_empty() {
            println!(
                "일치  {} (알려진 차이 {}건 무시)",
                case.document,
                known_here.len()
            );
            continue;
        }
        println!("차이  {} — 새 차이 {}건", case.document, fresh.len());
        // 맨 앞의 차이가 근원이다. 그 뒤는 정렬이 밀려 따라 나온 것일 때가 많다.
        for difference in fresh.iter().take(divergences_to_show()) {
            print_divergence(difference, &expected, &actual);
        }
        new_differences += fresh.len();
        for difference in &fresh {
            let entry = kinds.entry(difference.kind()).or_insert_with(|| {
                let context = if difference.context.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", difference.context)
                };
                (
                    0,
                    format!(
                        "the seed: {} | 우리: {}{context}",
                        difference.expected, difference.actual
                    ),
                )
            });
            entry.0 += 1;
        }
    }

    if !kinds.is_empty() {
        // 어긋난 지점 하나가 뒤따르는 조각들의 정렬을 흐트러뜨리므로 건수 자체보다
        // 유형 분포가 원인을 가리킨다. 상위 유형부터 잡는 것이 효율적이다.
        let mut ranked: Vec<(&String, &(usize, String))> = kinds.iter().collect();
        ranked.sort_by(|left, right| right.1.0.cmp(&left.1.0).then(left.0.cmp(right.0)));
        println!("\n== 차이 유형 상위 (건수 내림차순)");
        for (kind, (count, sample)) in ranked.iter().take(15) {
            println!("  {count:>5}  {kind}");
            println!("         예: {sample}");
        }
        if ranked.len() > 15 {
            println!("  … 외 유형 {}종", ranked.len() - 15);
        }
    }

    println!(
        "\n== 케이스 {}건 | 새 차이 {new_differences}건 | 알려진 차이 {known_hits}건",
        cases.len()
    );
    if new_differences > 0 {
        println!(
            "새 차이를 확인한 뒤, 의도된 것이면 {}에 등록하세요.",
            known_differences_path().display()
        );
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// 우리 렌더 결과를 the seed 렌더와 나란히 파일로 떨군다.
///
/// 차이의 원문을 역추적할 때 쓴다. 코퍼스와 같은 `WikiContext`를 거치므로
/// 이미지·링크 존재 여부가 실제 대조와 같다 — 별도 도구로 찍으면 어긋난다.
fn dump_command(arguments: &[String]) -> ExitCode {
    let Ok(corpus) = corpus::Corpus::load(&corpus_directory()) else {
        eprintln!(
            "코퍼스를 읽지 못했습니다. 먼저 `python3 tools/fetch-parity-corpus.py`를 실행하세요."
        );
        return ExitCode::from(2);
    };
    let wanted = arguments.first();
    for case in corpus.comparable_cases() {
        if let Some(wanted) = wanted
            && !case.document.contains(wanted.as_str())
        {
            continue;
        }
        let slug = case.document.replace([':', '/'], "_");
        let directory = corpus_directory();
        let ours_path = directory.join(format!("{slug}.ours.html"));
        let seed_path = directory.join(format!("{slug}.seed.html"));
        if std::fs::write(&ours_path, corpus.render(case, &corpus)).is_err()
            || std::fs::write(&seed_path, &case.rendered).is_err()
        {
            eprintln!("쓰기 실패: {}", ours_path.display());
            return ExitCode::from(2);
        }
        println!("{}\n  {}", ours_path.display(), seed_path.display());
    }
    ExitCode::SUCCESS
}

fn report_command(arguments: &[String]) -> ExitCode {
    let paths: Vec<PathBuf> = if arguments.is_empty() {
        let directory = corpus_directory();
        match std::fs::read_dir(&directory) {
            Ok(entries) => entries
                .filter_map(|entry| {
                    let path = entry.ok()?.path();
                    (path.extension()? == "namu").then_some(path)
                })
                .collect(),
            Err(_) => {
                eprintln!("코퍼스를 읽지 못했습니다: {}", directory.display());
                return ExitCode::from(2);
            }
        }
    } else {
        arguments.iter().map(PathBuf::from).collect()
    };

    if paths.is_empty() {
        eprintln!("대상 문서가 없습니다.");
        return ExitCode::from(2);
    }
    report::run(&paths);
    ExitCode::SUCCESS
}
