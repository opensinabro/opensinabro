//! 문법 회귀 코퍼스(fixtures/corpus/). 케이스 하나가 파일 하나로 자기완결적이다.
//!
//! 갱신: `UPDATE_GOLDEN=1 cargo test -p namumark-backend-namuwiki --test corpus_tests`
//!
//! 한 파일이 근거 등급·원문·의미론·IR·렌더링을 모두 담는다. 파일 하나만 열면 "이 마크업이
//! 무슨 근거로 이렇게 해석되고, 어떤 값으로 확정되어, 이렇게 그려진다"가 전부 보이고,
//! 문법 하나가 깨지면 파일명이 곧장 그 문법을 가리킨다.
//!
//! IR 구획은 프론트엔드가 실제로 받는 JSON이다. 렌더링 구획만 있으면 나무위키 백엔드를
//! 지나며 뭉개진 것까지만 붙들 수 있어, 다른 백엔드가 볼 값이 조용히 바뀌어도 알 수 없다.
//!
//! 구획은 첫 줄이 밝힌 **줄 수**로 가른다. 구분자를 본문에 끼워 넣는 방식은 코퍼스에
//! 맞지 않는다 — 여기 원문은 온갖 마크업이라 어떤 구분자든 원문에 나타날 수 있고, 그러면
//! 구분자를 피해 다니는 우회로가 생긴다. 길이로 가르면 충돌 자체가 성립하지 않아 원문을
//! 한 글자도 손대지 않고 그대로 담는다.
//!
//! 렌더는 `corpus::CorpusContext`(고정된 작은 위키) 위에서 한다. `EmptyContext`로는
//! 이미지·상대 링크·include·날짜 경로가 전부 대체 표기로 주저앉아 검증되지 않는다.
//!
//! 세 크레이트를 함께 보므로 파이프라인 전체를 지나는 이 크레이트에 둔다.

mod corpus;

use corpus::CorpusContext;
use namumark_ast::AstNode;
use namumark_backend_namuwiki::NamuwikiMarkup;
use namumark_ir::RenderBackend;
use std::fs;
use std::path::{Path, PathBuf};

/// 근거 등급. 뜻과 도출법은 fixtures/corpus/README.md 참고.
const EVIDENCE_GRADES: [&str; 3] = ["도움말예제", "도움말서술", "미확인"];

const HEADER_FORM: &str =
    "근거: <등급> | 원문: <N>줄 | 의미론: <N>줄 | IR: <N>줄 | 렌더링: <N>줄 | <설명>";

/// 케이스 첫 줄. 이것만으로 나머지 내용이 어떻게 나뉘는지가 정해진다.
struct Header<'a> {
    grade: &'a str,
    description: &'a str,
    source_lines: usize,
    semantics_lines: usize,
    ir_lines: usize,
    markup_lines: usize,
}

#[test]
fn corpus_cases() {
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    let mut failures = Vec::new();

    for path in &collect_cases() {
        let name = case_name(path);
        let text = fs::read_to_string(path).expect("케이스 읽기 실패");
        let expected = match regenerate(&text) {
            Ok(expected) => expected,
            Err(reason) => {
                failures.push(format!("{name}: {reason}"));
                continue;
            }
        };
        if update {
            fs::write(path, &expected).expect("케이스 쓰기 실패");
        } else if text != expected {
            failures.push(format!(
                "{name}: 골든 불일치 (UPDATE_GOLDEN=1 로 갱신)\n{}",
                first_difference(&text, &expected)
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// 원문으로부터 케이스 파일 전체를 다시 짓는다.
///
/// 첫 줄이 밝힌 `원문: <N>줄`만이 입력을 정하고, 나머지 구획과 줄 수는 전부 여기서 나온다.
/// 그래서 선언한 줄 수가 틀리면 다시 지은 결과와 어긋나 골든 대조가 잡아낸다.
fn regenerate(text: &str) -> Result<String, String> {
    let (header, body) = split_header(text)?;
    let (source, rest) = take_lines(body, header.source_lines).ok_or(format!(
        "원문이 선언한 {}줄보다 짧습니다",
        header.source_lines
    ))?;

    // 갱신은 이 파일을 통째로 덮어쓴다. 원문 줄 수 선언이 실제 내용과 어긋난 채로
    // 덮어쓰면 선언 범위 밖의 원문이 **소리 없이 사라진다** — 원문을 고치고 `원문:`을
    // 고치는 걸 잊는 것은 흔한 일이고, 그때 잃는 것은 되살릴 수 없다. 그래서 다시 짓기
    // 전에 파일이 제 선언과 맞는지 먼저 본다.
    let declared = header.semantics_lines + header.ir_lines + header.markup_lines;
    if line_count(rest) != declared {
        return Err(format!(
            "첫 줄 선언이 파일 내용과 맞지 않습니다 (원문 {}줄 뒤로 {}줄이 남았는데 \
             의미론 {}줄 + IR {}줄 + 렌더링 {}줄 = {declared}줄이라 선언했습니다).\n  \
             원문을 고쳤다면 첫 줄의 `원문:`도 함께 고치십시오 — 갱신은 선언한 줄 수까지만 \
             원문으로 보고 나머지를 버립니다.",
            header.source_lines,
            line_count(rest),
            header.semantics_lines,
            header.ir_lines,
            header.markup_lines,
        ));
    }

    let document = namumark_parser::parse(source);
    let tree = namumark_render::build_render_tree(&document, &CorpusContext);
    // 의미론 섹션은 무손실 구문 트리를 보인다 — 세분화된 토큰·스팬까지 그대로 드러난다.
    let semantics = terminate(&format!("{:#?}", document.syntax()));
    // IR 섹션은 프론트엔드가 받는 JSON 그대로다.
    let ir = terminate(&serde_json::to_string_pretty(&tree).expect("IR은 항상 직렬화된다"));
    let markup = terminate(&NamuwikiMarkup.render(&tree));

    Ok(format!(
        "근거: {} | 원문: {}줄 | 의미론: {}줄 | IR: {}줄 | 렌더링: {}줄 | {}\n\
         {source}{semantics}{ir}{markup}",
        header.grade,
        line_count(source),
        line_count(&semantics),
        line_count(&ir),
        line_count(&markup),
        header.description,
    ))
}

/// 첫 줄을 읽는다. 형식이 어긋나거나 등급이 낯설면 실패다 — 근거 없이 케이스가 느는 것을
/// 막는 장치다.
fn split_header(text: &str) -> Result<(Header<'_>, &str), String> {
    let (line, body) = text
        .split_once('\n')
        .ok_or("첫 줄에 메타데이터가 없습니다")?;
    let fields: Vec<&str> = line.splitn(6, " | ").collect();
    let [
        grade_field,
        source_field,
        semantics_field,
        ir_field,
        markup_field,
        description,
    ] = fields[..]
    else {
        return Err(format!("첫 줄 형식이 `{HEADER_FORM}`이 아닙니다"));
    };

    let grade = grade_field
        .strip_prefix("근거: ")
        .ok_or(format!("첫 줄 형식이 `{HEADER_FORM}`이 아닙니다"))?;
    if !EVIDENCE_GRADES.contains(&grade) {
        return Err(format!(
            "낯선 근거 등급 `{grade}` (가능한 값: {})",
            EVIDENCE_GRADES.join(", ")
        ));
    }

    Ok((
        Header {
            grade,
            description,
            source_lines: line_count_field(source_field, "원문")?,
            semantics_lines: line_count_field(semantics_field, "의미론")?,
            ir_lines: line_count_field(ir_field, "IR")?,
            markup_lines: line_count_field(markup_field, "렌더링")?,
        },
        body,
    ))
}

fn line_count_field(field: &str, name: &str) -> Result<usize, String> {
    field
        .strip_prefix(&format!("{name}: "))
        .and_then(|value| value.strip_suffix('줄'))
        .and_then(|value| value.parse().ok())
        .ok_or(format!("`{name}: <N>줄` 형식이 아닙니다: `{field}`"))
}

/// 구획이 온전한 줄로만 이뤄지게 개행으로 맺는다. 빈 구획은 빈 채로 둔다 — 개행을 붙이면
/// 없던 빈 줄이 생긴다.
fn terminate(text: &str) -> String {
    if text.is_empty() || text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    }
}

/// 앞에서부터 `count`줄을 떼어 낸다. 줄은 개행까지 포함한다.
fn take_lines(text: &str, count: usize) -> Option<(&str, &str)> {
    let mut offset = 0;
    for _ in 0..count {
        offset += text[offset..].find('\n')? + 1;
    }
    Some(text.split_at(offset))
}

fn line_count(text: &str) -> usize {
    text.matches('\n').count()
}

fn corpus_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/corpus")
}

/// `fixtures/corpus/<카테고리>/<케이스>.case`를 이름순으로 모은다.
fn collect_cases() -> Vec<PathBuf> {
    let mut categories: Vec<PathBuf> = fs::read_dir(corpus_directory())
        .expect("코퍼스 디렉토리 읽기 실패")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            path.is_dir().then_some(path)
        })
        .collect();
    categories.sort();

    let mut cases = Vec::new();
    for category in categories {
        let mut found: Vec<PathBuf> = fs::read_dir(&category)
            .expect("카테고리 읽기 실패")
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                (path.extension()? == "case").then_some(path)
            })
            .collect();
        found.sort();
        cases.append(&mut found);
    }
    assert!(!cases.is_empty(), "코퍼스 케이스가 없습니다");
    cases
}

fn case_name(path: &Path) -> String {
    let category = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .expect("카테고리 이름");
    let case = path
        .file_stem()
        .and_then(|name| name.to_str())
        .expect("케이스 이름");
    format!("{category}/{case}")
}

fn first_difference(expected: &str, actual: &str) -> String {
    for (number, (expected_line, actual_line)) in expected.lines().zip(actual.lines()).enumerate() {
        if expected_line != actual_line {
            return format!(
                "  줄 {}:\n  - {expected_line}\n  + {actual_line}",
                number + 1
            );
        }
    }
    "  (길이만 다름)".to_string()
}

/// 아래는 코퍼스가 아니라 **하네스 자신**을 붙드는 테스트다. 이 파일은 케이스를 통째로
/// 덮어쓰므로, 여기가 조용히 잘못되면 원문이 사라진다.
#[cfg(test)]
mod harness {
    use super::*;

    const BOLD: &str =
        "근거: 미확인 | 원문: 1줄 | 의미론: 15줄 | IR: 12줄 | 렌더링: 1줄 | 굵게\n'''굵게'''\n";

    fn case(header: &str, body: &str) -> String {
        format!("{header}\n{body}")
    }

    #[test]
    fn regenerated_case_is_stable() {
        let once = regenerate(&case(
            "근거: 미확인 | 원문: 1줄 | 의미론: 0줄 | IR: 0줄 | 렌더링: 0줄 | 굵게",
            "'''굵게'''\n",
        ))
        .expect("다시 짓기");
        assert_eq!(
            regenerate(&once).expect("두 번째"),
            once,
            "왕복이 안정적이지 않다"
        );
    }

    // 원문을 고치고 `원문:`을 고치는 걸 잊었을 때, 갱신이 그 줄을 삼켜서는 안 된다.
    #[test]
    fn stale_source_line_count_is_refused_before_overwriting() {
        let reason = regenerate(&case(
            "근거: 미확인 | 원문: 1줄 | 의미론: 15줄 | IR: 0줄 | 렌더링: 1줄 | 굵게",
            "'''굵게'''\n''기울임''\n",
        ))
        .expect_err("선언과 어긋난 파일은 거부해야 한다");
        assert!(reason.contains("맞지 않습니다"), "{reason}");
    }

    #[test]
    fn source_shorter_than_declared_is_refused() {
        let reason = regenerate(&case(
            "근거: 미확인 | 원문: 9줄 | 의미론: 0줄 | IR: 0줄 | 렌더링: 0줄 | 짧음",
            "한 줄\n",
        ))
        .expect_err("원문이 모자라면 거부해야 한다");
        assert!(reason.contains("짧습니다"), "{reason}");
    }

    #[test]
    fn unknown_evidence_grade_is_refused() {
        let reason = regenerate(&BOLD.replace("미확인", "렌더확정")).expect_err("낯선 등급");
        assert!(reason.contains("낯선 근거 등급"), "{reason}");
    }

    #[test]
    fn missing_metadata_is_refused() {
        assert!(regenerate("'''굵게'''\n").is_err(), "메타데이터 없는 파일");
        let reason = regenerate(&case("굵게만 적음", "'''굵게'''\n")).expect_err("형식 어긋남");
        assert!(reason.contains("첫 줄 형식"), "{reason}");
    }

    // 설명에 ` | `가 들어가도 필드 경계가 밀리면 안 된다.
    #[test]
    fn description_may_contain_the_field_separator() {
        let text = case(
            "근거: 미확인 | 원문: 1줄 | 의미론: 0줄 | IR: 0줄 | 렌더링: 0줄 | 표시명 [[문서 | 출력]] 설명",
            "'''굵게'''\n",
        );
        let (header, _) = split_header(&text).expect("첫 줄 읽기");
        assert_eq!(header.description, "표시명 [[문서 | 출력]] 설명");
    }

    #[test]
    fn empty_source_round_trips() {
        let text = regenerate(&case(
            "근거: 미확인 | 원문: 0줄 | 의미론: 0줄 | IR: 0줄 | 렌더링: 0줄 | 빈 문서",
            "",
        ))
        .expect("빈 원문");
        assert!(text.starts_with("근거: 미확인 | 원문: 0줄 |"), "{text}");
        assert_eq!(regenerate(&text).expect("두 번째"), text);
    }

    #[test]
    fn take_lines_counts_whole_lines() {
        assert_eq!(take_lines("가\n나\n", 0), Some(("", "가\n나\n")));
        assert_eq!(take_lines("가\n나\n", 1), Some(("가\n", "나\n")));
        assert_eq!(take_lines("가\n나\n", 2), Some(("가\n나\n", "")));
        assert_eq!(take_lines("가\n나\n", 3), None);
    }
}
