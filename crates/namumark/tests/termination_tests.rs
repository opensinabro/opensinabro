use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn assert_parse_terminates(label: &str, source: String) {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let document = namumark::parse(&source);
            let _ = sender.send(document.blocks.len());
        })
        .expect("파싱 스레드 생성 실패");
    match receiver.recv_timeout(Duration::from_secs(10)) {
        Ok(_) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => panic!("무한루프 의심 (10초 초과): {label}"),
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("파싱 중 panic: {label}"),
    }
}

#[test]
fn unclosed_constructs_terminate() {
    let cases = [
        "{{{",
        "{{{#!wiki style=\"열림",
        "{{{#!folding",
        "{{{#!syntax rust",
        "{{{#red",
        "{{{+1",
        "[[",
        "[[문서",
        "[[문서|표시",
        "[*",
        "[* 각주",
        "[매크로(",
        "'''닫히지 않은 굵게",
        "''기울임",
        "~~취소",
        "--취소",
        "__밑줄",
        "^^위",
        ",,아래",
        "'''가 ''나''' 다''",
        "|| 셀",
        "|캡션",
        "|캡션| 셀",
        "> 인용",
        " 들여쓰기",
        " * 리스트",
        "\\",
        "= 제목",
        "==# 접기",
        "----이어짐",
    ];
    for case in cases {
        assert_parse_terminates(case, case.to_string());
    }
}

#[test]
fn marker_floods_terminate() {
    let markers = [
        "'", "-", "=", "[", "]", "{", "}", "|", "~", "_", "^", ",", ">", "*", "#", "\\", "\n", " ",
    ];
    for marker in markers {
        assert_parse_terminates(&format!("flood:{marker:?}"), marker.repeat(2000));
    }
    let pairs = [
        "[]", "[[]]", "{{{}}}", "''", "'''", "||", "~~", "> ", " *", "\n\n",
    ];
    for pair in pairs {
        assert_parse_terminates(&format!("pair-flood:{pair:?}"), pair.repeat(500));
    }
}

#[test]
fn deep_quote_nesting_terminates() {
    assert_parse_terminates("deep-quote", format!("{}인용", ">".repeat(500)));
}

#[test]
fn deep_indent_nesting_terminates() {
    assert_parse_terminates("deep-indent", format!("{}본문", " ".repeat(500)));
}

#[test]
fn deep_list_nesting_terminates() {
    let source = (1..=150)
        .map(|depth| format!("{}* 항목", " ".repeat(depth)))
        .collect::<Vec<_>>()
        .join("\n");
    assert_parse_terminates("deep-list", source);
}

#[test]
fn deep_footnote_nesting_terminates() {
    assert_parse_terminates(
        "deep-footnote",
        format!("{}끝{}", "[* ".repeat(300), "]".repeat(300)),
    );
}

#[test]
fn deep_brace_literal_terminates() {
    assert_parse_terminates(
        "deep-brace",
        format!("{}중심{}", "{{{".repeat(300), "}}}".repeat(300)),
    );
}

#[test]
fn deep_wiki_block_nesting_terminates() {
    let mut source = String::new();
    for _ in 0..200 {
        source.push_str("{{{#!wiki\n");
    }
    source.push_str("내용");
    for _ in 0..200 {
        source.push_str("\n}}}");
    }
    assert_parse_terminates("deep-wiki", source);
}

#[test]
fn deep_styled_nesting_terminates() {
    let markers = ["'''", "''", "~~", "__", "^^", ",,"];
    let mut source = String::new();
    for index in 0..240 {
        source.push_str(markers[index % markers.len()]);
    }
    source.push_str("중심");
    for index in (0..240).rev() {
        source.push_str(markers[index % markers.len()]);
    }
    assert_parse_terminates("deep-styled", source);
}

#[test]
fn deep_link_display_nesting_terminates() {
    assert_parse_terminates(
        "deep-link",
        format!("{}끝{}", "[[문서|".repeat(200), "]]".repeat(200)),
    );
}

#[test]
fn pathological_tables_terminate() {
    let cases = vec![
        ("pipes-only", "||".repeat(400)),
        ("bare-rows", "||\n".repeat(300)),
        ("unclosed-brace-rows", "|| {{{ ||\n".repeat(200)),
        ("caption-only", "|캡션|".repeat(200)),
        (
            "mixed-spans",
            format!("|캡션| {} ||", "셀 |||| ".repeat(100)),
        ),
        (
            "row-gather",
            format!("|| 시작\n{}끝 ||", "가운데 줄\n".repeat(300)),
        ),
    ];
    for (label, source) in cases {
        assert_parse_terminates(label, source);
    }
}

const KITCHEN_SINK: &str = r#"= 개요 =
'''굵게''' ''기울임'' ~~취소~~ __밑줄__ ^^위^^ ,,아래,,
[[링크]] [[링크|'''표시''']] [* 각주 [[링크]]] [age(2000-01-01)]
{{{#red 빨강}}} {{{#ff0000,#00ff00 듀얼}}} {{{+3 크게}}} {{{리터럴 '''그대로'''}}}
== 표 ==
|캡션|<-2><:> 병합 ||
||<^|2><bgcolor=#eee> 왼쪽 || 오른쪽 ||
||<:>셀
{{{#!folding 접기
|| 안쪽 || 표 ||
}}}
끝 ||
=== 블록 ===
> 인용
>> 중첩 인용
> || 인용 속 표 ||
 * 리스트
  * 중첩
   1. 순서
 들여쓰기 문단
{{{#!wiki style="margin: 4px" dark-style='color: red'
{{{#!folding 요약
 * 접힌 리스트
}}}
}}}
{{{#!syntax rust
fn main() { println!("{}", 1); }
}}}
{{{#!html
<b>굵게</b>
}}}
----
## 주석
#redirect 대상 아님
마지막 문단
"#;

#[test]
fn kitchen_sink_terminates() {
    assert_parse_terminates("kitchen-sink", KITCHEN_SINK.to_string());
}

#[test]
fn every_prefix_and_suffix_of_kitchen_sink_terminates() {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            for (index, _) in KITCHEN_SINK.char_indices() {
                namumark::parse(&KITCHEN_SINK[..index]);
                namumark::parse(&KITCHEN_SINK[index..]);
            }
            let _ = sender.send(());
        })
        .expect("파싱 스레드 생성 실패");
    match receiver.recv_timeout(Duration::from_secs(30)) {
        Ok(_) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => panic!("무한루프 의심 (30초 초과): 접두사/접미사"),
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("파싱 중 panic: 접두사/접미사"),
    }
}

#[test]
fn snippet_pair_combinations_terminate() {
    const SNIPPETS: [&str; 16] = [
        "= 제목 =",
        "'''굵게'''",
        "~~취소~~",
        "[[링크|표시]]",
        "[* 각주]",
        "[br]",
        "{{{리터럴}}}",
        "{{{#red 색}}}",
        "{{{+2 크기}}}",
        "|| 표 ||",
        "> 인용",
        " * 리스트",
        "----",
        "{{{#!folding 접기\n내용\n}}}",
        "{{{#!wiki\n내용\n}}}",
        "## 주석",
    ];
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            for first in SNIPPETS {
                for second in SNIPPETS {
                    namumark::parse(&format!("{first}{second}"));
                    namumark::parse(&format!("{first} {second}"));
                    namumark::parse(&format!("{first}\n{second}"));
                }
            }
            let _ = sender.send(());
        })
        .expect("파싱 스레드 생성 실패");
    match receiver.recv_timeout(Duration::from_secs(30)) {
        Ok(_) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => panic!("무한루프 의심 (30초 초과): 문법 조합"),
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("파싱 중 panic: 문법 조합"),
    }
}

#[test]
fn crlf_input_is_handled() {
    let document = namumark::parse("= 제목 =\r\n본문\r\n");
    assert_eq!(document.blocks.len(), 2);
}

#[test]
fn unicode_chaos_terminates() {
    assert_parse_terminates(
        "unicode",
        "'''🎉👨‍👩‍👧‍👦''' [[한글✨|{{{#red 🔥}}}]] ~~é́́조합문자~~ {{{+1 ｚｅｎｋａｋｕ}}}".to_string(),
    );
}
