//! JSON API가 함께 쓰는 표현.
//!
//! 프론트엔드가 화면을 그리는 데 필요한 만큼만 담고, 내부 식별자는 내보내지 않는다
//! (docs/architecture.md의 데이터 모델 원칙).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use uuid::Uuid;
use wiki_document::RevisionRecord;

use crate::ServerError;
use crate::session::Requester;
use crate::state::AppState;

/// 권한이 없을 때의 응답. 무엇이 막혔는지는 알리지 않는다.
pub fn forbidden() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({ "error": "forbidden" })),
    )
        .into_response()
}

/// 로그인해야만 쓸 수 있는 API에 비로그인으로 왔을 때. 화면이 로그인으로 이끌 수
/// 있게 권한 부족(403)과 구분해 낸다.
pub fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "unauthorized" })),
    )
        .into_response()
}

pub fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "not_found" })),
    )
        .into_response()
}

/// 제목 목록 화면이 함께 쓰는 한 줄. `note`는 화면마다 다른 곁들임(링크된 횟수·바이트
/// 수·마지막 편집일)이고, 곁들일 것이 없는 목록은 빈 문자열을 낸다.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleEntry {
    pub title: String,
    pub note: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleListPayload {
    pub entries: Vec<TitleEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionSummary {
    pub id: Uuid,
    pub sequence: i64,
    pub kind: &'static str,
    pub author: String,
    pub comment: String,
    pub content_bytes: i64,
    pub created_at: String,
    pub hidden: bool,
}

impl From<&RevisionRecord> for RevisionSummary {
    fn from(record: &RevisionRecord) -> Self {
        Self {
            id: record.external_id,
            sequence: record.sequence,
            kind: record.kind.as_str(),
            author: record.author.clone(),
            comment: record.comment.clone(),
            content_bytes: record.content_bytes,
            created_at: record.created_at.to_rfc3339(),
            hidden: record.hidden,
        }
    }
}

/// 셸이 화면마다 필요로 하는 것 — 위키 이름·로그인 상태·알림 수.
///
/// 화면마다 따로 묻지 않고 한 번에 내주는 이유는 askama 셸에서 겪은 것과 같다:
/// 일부 화면만 로그인 상태를 싣지 않으면 그 화면의 폼만 조용히 403을 낸다.
///
/// CSRF 토큰은 여기 싣지 않는다. 이 응답을 받는 것은 서버 컴포넌트이고, 그쪽은
/// 응답의 Set-Cookie를 브라우저로 돌려주지 않아 짝이 되는 쿠키가 유실된다 —
/// 검증할 수 없는 토큰이 된다. 브라우저는 `/api/csrf`로 직접 받는다.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub wiki_name: String,
    pub main_document: String,
    pub content_license: String,
    pub user_name: Option<String>,
    pub unread: i64,
}

pub async fn session_api(
    State(state): State<AppState>,
    requester: Requester,
) -> Result<Response, ServerError> {
    let unread = match &requester.user {
        Some(user) => wiki_account::unread_count(&state.pool, user.identifier).await?,
        None => 0,
    };

    Ok(Json(SessionView {
        wiki_name: state.settings.wiki_name.clone(),
        main_document: state.settings.main_document.clone(),
        content_license: state.settings.content_license.clone(),
        user_name: requester.user.as_ref().map(|user| user.name.clone()),
        unread,
    })
    .into_response())
}
