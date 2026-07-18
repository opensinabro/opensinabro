use askama::Template;

use crate::state::SiteSettings;

/// 셸은 제목·검색창·푸터만 그리고 본문은 렌더러가 방출한 HTML을 그대로 넣는다
/// (본문과 셸의 경계 — docs/design/07).
#[derive(Template)]
#[template(path = "shell.html")]
pub struct Shell {
    pub wiki_name: String,
    pub content_license: String,
    pub heading: String,
    pub query: String,
    pub body: String,
    /// 로그인한 사용자 이름. 없으면 로그인·가입 링크를 보인다.
    pub user_name: Option<String>,
    /// 로그아웃 폼이 쓰는 토큰.
    pub csrf_token: String,
    /// 읽지 않은 알림 수.
    pub unread: i64,
}

impl Shell {
    pub fn new(settings: &SiteSettings, heading: impl Into<String>, body: String) -> Self {
        Self {
            wiki_name: settings.wiki_name.clone(),
            content_license: settings.content_license.clone(),
            heading: heading.into(),
            query: String::new(),
            body,
            user_name: None,
            csrf_token: String::new(),
            unread: 0,
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    /// 로그인 상태를 셸에 싣는다.
    pub fn with_requester(mut self, user_name: Option<String>, csrf_token: String) -> Self {
        self.user_name = user_name;
        self.csrf_token = csrf_token;
        self
    }

    pub fn with_unread(mut self, unread: i64) -> Self {
        self.unread = unread;
        self
    }
}

#[derive(Template)]
#[template(path = "edit.html")]
pub struct EditForm {
    pub title: String,
    pub content: String,
    pub base_revision: String,
    pub csrf_token: String,
    /// 자동 병합이 실패했을 때 사용자에게 알릴 말.
    pub conflict_message: Option<String>,
}
