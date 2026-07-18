use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wiki_authorization::AclAction;
use wiki_discussion::{CommentKind, EditRequestStatus, ThreadStatus};
use wiki_document::{DocumentTitle, RevisionKind};

use crate::ServerError;
use crate::handler::namespace_names;
use crate::security::verify_header;
use crate::session::Requester;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RecentQuery {
    status: Option<String>,
}

/// 이미 처리된 요청에 같은 조작을 다시 낸 경우.
fn conflict(reason: &'static str) -> Response {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({ "error": reason })),
    )
        .into_response()
}

/// 관리 조작이 남긴 값 — HTML 경로와 달리 이스케이프하지 않는다.
fn raw_metadata_value(comment: &wiki_discussion::Comment, key: &str) -> Option<String> {
    comment
        .metadata
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummary {
    id: Uuid,
    topic: String,
    status: &'static str,
    status_label: &'static str,
    created_at: String,
}

impl From<&wiki_discussion::Thread> for ThreadSummary {
    fn from(thread: &wiki_discussion::Thread) -> Self {
        Self {
            id: thread.external_id,
            topic: thread.topic.clone(),
            status: thread.status.as_str(),
            status_label: thread.status.label(),
            created_at: thread.created_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentThreadsPayload {
    title: String,
    threads: Vec<ThreadSummary>,
    may_create: bool,
}

/// 문서의 토론 목록.
pub async fn document_threads_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(raw_title): Path<String>,
) -> Result<Response, ServerError> {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester.may(&state, &title, AclAction::Read).await? {
        return Ok(crate::api::forbidden());
    }

    let threads = wiki_discussion::threads_of(&state.pool, &title).await?;
    let may_create = requester
        .may(&state, &title, AclAction::CreateThread)
        .await?;

    Ok(Json(DocumentThreadsPayload {
        title: title.to_string(),
        threads: threads.iter().map(Into::into).collect(),
        may_create,
    })
    .into_response())
}

#[derive(Deserialize)]
pub struct NewThreadBody {
    topic: String,
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewThreadPayload {
    thread_id: Uuid,
}

pub async fn create_thread_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(raw_title): Path<String>,
    Json(submission): Json<NewThreadBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester
        .may(&state, &title, AclAction::CreateThread)
        .await?
    {
        return Ok(crate::api::forbidden());
    }

    let actor = requester.actor(&state).await?;
    let thread = wiki_discussion::create_thread(
        &state.pool,
        &title,
        &submission.topic,
        actor,
        &submission.content,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(NewThreadPayload { thread_id: thread }),
    )
        .into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentView {
    sequence: i64,
    kind: &'static str,
    author: String,
    /// 가려진 발언은 빈 문자열이다 — 원문은 내보내지 않는다.
    content: String,
    /// 관리 조작이 바꾼 값. 화면이 문장을 만들어 쓴다.
    detail: Option<String>,
    admin_marked: bool,
    hidden: bool,
    created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadPayload {
    id: Uuid,
    topic: String,
    title: String,
    status: &'static str,
    status_label: &'static str,
    comments: Vec<CommentView>,
    may_comment: bool,
    may_moderate: bool,
}

/// 스레드 하나 — 발언과 관리 조작이 한 타임라인에 섞여 있다.
pub async fn view_thread_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(id): Path<Uuid>,
) -> Result<Response, ServerError> {
    let Some(thread) = wiki_discussion::thread_by_id(&state.pool, id).await? else {
        return Ok(crate::api::not_found());
    };

    let comments = wiki_discussion::thread_comments(&state.pool, id)
        .await?
        .iter()
        .map(|comment| CommentView {
            sequence: comment.sequence,
            kind: comment.kind.as_str(),
            author: comment.author.clone(),
            content: if comment.hidden {
                String::new()
            } else {
                comment.content.clone()
            },
            detail: match comment.kind {
                CommentKind::Comment => None,
                CommentKind::StatusChange => raw_metadata_value(comment, "to"),
                CommentKind::TopicChange => raw_metadata_value(comment, "topic"),
                CommentKind::DocumentMove => raw_metadata_value(comment, "document"),
            },
            admin_marked: comment.admin_marked,
            hidden: comment.hidden,
            created_at: comment.created_at.to_rfc3339(),
        })
        .collect();

    let may_comment = thread.status.accepts_comments()
        && requester
            .may(&state, &thread.title, AclAction::WriteThreadComment)
            .await?;
    let may_moderate = requester
        .has_permission(&state, "update_thread_status")
        .await?;

    Ok(Json(ThreadPayload {
        id: thread.external_id,
        topic: thread.topic.clone(),
        title: thread.title.to_string(),
        status: thread.status.as_str(),
        status_label: thread.status.label(),
        comments,
        may_comment,
        may_moderate,
    })
    .into_response())
}

#[derive(Deserialize)]
pub struct CommentBody {
    content: String,
}

pub async fn add_comment_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Json(submission): Json<CommentBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let Some(thread) = wiki_discussion::thread_by_id(&state.pool, id).await? else {
        return Ok(crate::api::not_found());
    };
    if !requester
        .may(&state, &thread.title, AclAction::WriteThreadComment)
        .await?
    {
        return Ok(crate::api::forbidden());
    }
    if !thread.status.accepts_comments() {
        return Ok(conflict("thread_closed"));
    }

    let actor = requester.actor(&state).await?;
    let admin_marked = requester.has_permission(&state, "admin").await?;
    wiki_discussion::add_comment(&state.pool, id, actor, &submission.content, admin_marked).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Deserialize)]
pub struct StatusBody {
    status: String,
}

pub async fn change_status_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Json(submission): Json<StatusBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }
    if !requester
        .has_permission(&state, "update_thread_status")
        .await?
    {
        return Ok(crate::api::forbidden());
    }

    let status = match submission.status.as_str() {
        "pause" => ThreadStatus::Pause,
        "close" => ThreadStatus::Close,
        _ => ThreadStatus::Normal,
    };
    let actor = requester.actor(&state).await?;
    if !wiki_discussion::change_status(&state.pool, id, actor, status).await? {
        return Ok(crate::api::not_found());
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentThreadSummary {
    id: Uuid,
    topic: String,
    title: String,
    status: &'static str,
    status_label: &'static str,
    created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentDiscussionsPayload {
    threads: Vec<RecentThreadSummary>,
}

/// 최근 토론 — 상태로 걸러 볼 수 있다.
pub async fn recent_discussions_api(
    State(state): State<AppState>,
    Query(parameters): Query<RecentQuery>,
) -> Result<Response, ServerError> {
    let status = parameters.status.as_deref().and_then(|name| match name {
        "normal" => Some(ThreadStatus::Normal),
        "pause" => Some(ThreadStatus::Pause),
        "close" => Some(ThreadStatus::Close),
        _ => None,
    });

    let threads = wiki_discussion::recent_threads(&state.pool, status, 100).await?;

    Ok(Json(RecentDiscussionsPayload {
        threads: threads
            .iter()
            .map(|thread| RecentThreadSummary {
                id: thread.external_id,
                topic: thread.topic.clone(),
                title: thread.title.to_string(),
                status: thread.status.as_str(),
                status_label: thread.status.label(),
                created_at: thread.created_at.to_rfc3339(),
            })
            .collect(),
    })
    .into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditRequestSummary {
    id: Uuid,
    title: String,
    author: String,
    comment: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditRequestsPayload {
    requests: Vec<EditRequestSummary>,
}

/// 열린 편집요청 목록. 목록 질의는 원문·상태·기준 리비전을 채우지 않으므로 내보내지 않는다.
pub async fn edit_requests_api(State(state): State<AppState>) -> Result<Response, ServerError> {
    let requests = wiki_discussion::open_edit_requests(&state.pool, 100).await?;

    Ok(Json(EditRequestsPayload {
        requests: requests
            .iter()
            .map(|request| EditRequestSummary {
                id: request.external_id,
                title: request.title.to_string(),
                author: request.author.clone(),
                comment: request.comment.clone(),
            })
            .collect(),
    })
    .into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLineView {
    kind: &'static str,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditRequestPayload {
    id: Uuid,
    title: String,
    author: String,
    comment: String,
    status: &'static str,
    status_label: &'static str,
    created_at: String,
    diff: Vec<DiffLineView>,
    may_review: bool,
}

/// 편집요청 하나 — 지금 원문과의 차이를 함께 낸다.
pub async fn view_edit_request_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(id): Path<Uuid>,
) -> Result<Response, ServerError> {
    let Some(request) = wiki_discussion::edit_request_by_id(&state.pool, id).await? else {
        return Ok(crate::api::not_found());
    };

    let current = wiki_document::read_source(&state.pool, &request.title)
        .await?
        .unwrap_or_default();

    let diff = wiki_document::diff_lines(&current, &request.content)
        .into_iter()
        .map(|line| DiffLineView {
            kind: match line.kind {
                wiki_document::DiffLineKind::Inserted => "inserted",
                wiki_document::DiffLineKind::Deleted => "deleted",
                wiki_document::DiffLineKind::Context => "context",
            },
            text: line.text,
        })
        .collect();

    // 반영은 그 문서를 편집할 수 있는 사람만 한다 — 편집요청이 권한 우회로가 되지 않게.
    let may_review = request.status == EditRequestStatus::Open
        && requester
            .may(&state, &request.title, AclAction::Edit)
            .await?;

    Ok(Json(EditRequestPayload {
        id: request.external_id,
        title: request.title.to_string(),
        author: request.author.clone(),
        comment: request.comment.clone(),
        status: request.status.as_str(),
        status_label: request.status.label(),
        created_at: request.created_at.to_rfc3339(),
        diff,
        may_review,
    })
    .into_response())
}

/// 요청을 문서에 반영한다 — 리비전은 편집 경로를 그대로 탄다.
pub async fn accept_edit_request_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let Some(request) = wiki_discussion::edit_request_by_id(&state.pool, id).await? else {
        return Ok(crate::api::not_found());
    };
    if !requester
        .may(&state, &request.title, AclAction::Edit)
        .await?
    {
        return Ok(crate::api::forbidden());
    }
    // 이미 처리된 요청을 다시 반영하면 같은 내용의 리비전이 또 쌓인다.
    if request.status != EditRequestStatus::Open {
        return Ok(conflict("already_reviewed"));
    }

    let actor = requester.actor(&state).await?;
    wiki_document::record_revision(
        &state.pool,
        &request.title,
        actor,
        RevisionKind::Edit,
        Some(&request.content),
        &format!("편집요청 반영: {}", request.comment),
        Some(serde_json::json!({ "edit_request": request.external_id })),
    )
    .await?;
    crate::edit::apply_side_effects(&state, &request.title, &request.content).await?;
    wiki_discussion::accept_edit_request(&state.pool, id, actor).await?;

    Ok(Json(serde_json::json!({ "title": request.title.to_string() })).into_response())
}

pub async fn close_edit_request_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let Some(request) = wiki_discussion::edit_request_by_id(&state.pool, id).await? else {
        return Ok(crate::api::not_found());
    };
    if !requester
        .may(&state, &request.title, AclAction::Edit)
        .await?
    {
        return Ok(crate::api::forbidden());
    }

    let actor = requester.actor(&state).await?;
    wiki_discussion::close_edit_request(&state.pool, id, actor).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewEditRequestBody {
    content: String,
    comment: String,
    base_revision: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewEditRequestPayload {
    edit_request_id: Uuid,
}

/// 편집 권한이 없을 때 변경안을 내는 통로.
pub async fn submit_edit_request_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(raw_title): Path<String>,
    Json(submission): Json<NewEditRequestBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester
        .may(&state, &title, AclAction::EditRequest)
        .await?
    {
        return Ok(crate::api::forbidden());
    }

    let actor = requester.actor(&state).await?;
    let request = wiki_discussion::submit_edit_request(
        &state.pool,
        &title,
        actor,
        &submission.content,
        &submission.comment,
        submission.base_revision.parse().ok(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(NewEditRequestPayload {
            edit_request_id: request,
        }),
    )
        .into_response())
}
