//! 코퍼스가 렌더할 때 서는 작은 위키.
//!
//! `EmptyContext`는 파일도 문서도 시각도 없는 세계라, 이미지는 전부 "파일 없음" 링크로,
//! 날짜 매크로와 `[include]`는 원문 표기 그대로 주저앉는다. 그 상태로 골든을 떠 두면
//! 이미지 레이아웃·상대 링크·include 전개·날짜 계산이 **한 번도 실행되지 않은 채**
//! 통과한다. 그래서 코퍼스는 이 문맥 위에서 렌더한다.
//!
//! 값은 전부 고정이다. 문맥이 흔들리면 골든이 흔들려 회귀 감시가 무너진다.

use namumark_render::{Date, DateTime, Time, WikiContext};

/// 렌더 기준 문서. 하위 문서라서 `[[../]]`가 실제로 상위를 가리킨다.
const CURRENT_TITLE: &str = "시험 문서/하위";

/// 존재하는 문서. 나머지는 빨간 링크가 되어 두 갈래가 모두 골든에 남는다.
const EXISTING_DOCUMENTS: [&str; 4] = ["문서", "알파위키", "시험 문서", "링크"];

/// 올라와 있는 파일. 나머지는 "파일 없음" 링크로 떨어진다.
const EXISTING_FILES: [&str; 3] = ["example.png", "알파위키 로고.svg", "투명.png"];

/// `[include(...)]`가 펼칠 틀. 인자를 받는 것과 받지 않는 것을 함께 둔다.
///
/// `틀:상위 문서`는 나무위키의 실제 틀을 본떴다. `#!if`의 변수 바인딩과 `calleeTitle`은
/// **틀이 include될 때만** 스코프가 생기므로(resolve의 `expand_include`), 최상위에 적어 둔
/// `#!if`로는 그 경로를 지날 수 없다.
const TEMPLATES: [(&str, &str); 5] = [
    ("틀:다른 뜻 설명", "다른 뜻: @설명@"),
    ("틀:설명", "@설명@"),
    ("틀:안쪽", "안쪽 틀"),
    ("틀:바깥", "바깥[include(틀:안쪽)]"),
    (
        "틀:상위 문서",
        "{{{#!if top = 문서명1 != null ? 문서명1 : calleeTitle\n상위 문서: [[@top@]]}}}",
    ),
];

/// 고정 시각. `[date]`·`[age(...)]`·`[dday(...)]`가 늘 같은 값을 내야 한다.
const NOW: DateTime = DateTime {
    date: Date {
        year: 2026,
        month: 7,
        day: 17,
    },
    time: Time {
        hour: 12,
        minute: 0,
        second: 0,
    },
};

pub struct CorpusContext;

impl WikiContext for CorpusContext {
    fn document_exists(&self, title: &str) -> bool {
        EXISTING_DOCUMENTS.contains(&title)
    }

    fn current_title(&self) -> Option<String> {
        Some(CURRENT_TITLE.to_string())
    }

    fn include_source(&self, title: &str) -> Option<String> {
        TEMPLATES
            .iter()
            .find(|(name, _)| *name == title)
            .map(|(_, source)| (*source).to_string())
    }

    fn file_url(&self, file_name: &str) -> Option<String> {
        EXISTING_FILES
            .contains(&file_name)
            .then(|| format!("/file/{file_name}"))
    }

    fn now(&self) -> Option<DateTime> {
        Some(NOW)
    }
}
