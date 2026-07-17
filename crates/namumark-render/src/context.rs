/// resolve pass가 외부 세계를 보는 유일한 창구.
/// layout pass와 백엔드는 순수 함수다.
pub trait WikiContext {
    /// 문서 존재 여부. 빨간 링크 판별에 쓰인다.
    fn document_exists(&self, title: &str) -> bool {
        let _ = title;
        false
    }

    /// 지금 렌더 중인 문서의 제목. 틀이 조건식에서 `calleeTitle`로 참조한다.
    fn current_title(&self) -> Option<String> {
        None
    }

    /// `[include(...)]` 대상 문서의 나무마크 원문.
    fn include_source(&self, title: &str) -> Option<String> {
        let _ = title;
        None
    }

    /// `[[파일:...]]`이 가리키는 실제 이미지 URL.
    fn file_url(&self, file_name: &str) -> Option<String> {
        let _ = file_name;
        None
    }

    /// 현재 시각. `[age(...)]`·`[dday(...)]`·`[date]`·`[datetime]` 계산에 쓰이며,
    /// None이면 해당 매크로는 원문 표기로 보존된다(렌더링 결정성).
    fn now(&self) -> Option<DateTime> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    pub date: Date,
    pub time: Time,
}

impl std::fmt::Display for Date {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:02}:{:02}:{:02}",
            self.hour, self.minute, self.second
        )
    }
}

impl std::fmt::Display for DateTime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} {}", self.date, self.time)
    }
}

impl Date {
    /// 율리우스 적일. 날짜 차이 계산용.
    pub(crate) fn julian_day_number(&self) -> i64 {
        let adjustment = (14 - self.month as i64) / 12;
        let year = self.year as i64 + 4800 - adjustment;
        let month = self.month as i64 + 12 * adjustment - 3;
        self.day as i64 + (153 * month + 2) / 5 + 365 * year + year / 4 - year / 100 + year / 400
            - 32045
    }
}

/// 아무 컨텍스트도 없는 기본 구현. 모든 링크는 빨간 링크, include는 확장되지 않는다.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyContext;

impl WikiContext for EmptyContext {}
