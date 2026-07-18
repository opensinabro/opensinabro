use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use uuid::Uuid;
use wiki_authorization::AclAction;
use wiki_discussion::{CommentKind, EditRequestStatus, ThreadStatus};
use wiki_document::{DocumentTitle, RevisionKind};

use crate::ServerError;
use crate::handler::{escape, namespace_names, shell};
use crate::security::{issue_token, verify_token};
use crate::session::Requester;
use crate::state::AppState;

type HandlerResult = Result<Response, ServerError>;

/// 문서의 토론 목록과 새 토론 열기.
pub async fn document_threads(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
) -> HandlerResult {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);
    let threads = wiki_discussion::threads_of(&state.pool, &title).await?;

    let mut body = String::from("<ul class=\"wiki-threads\">");
    if threads.is_empty() {
        body.push_str("<li>아직 토론이 없습니다.</li>");
    }
    for thread in &threads {
        body.push_str(&format!(
            "<li><a href=\"/thread/{id}\">{topic}</a> · {status}</li>",
            id = thread.external_id,
            topic = escape(&thread.topic),
            status = thread.status.label(),
        ));
    }
    body.push_str("</ul>");

    let (jar, csrf_token) = issue_token(jar);
    if requester
        .may(&state, &title, AclAction::CreateThread)
        .await?
    {
        body.push_str(&format!(
            "<h2>새 토론</h2>\
             <form method=\"post\" action=\"/discuss/{title}\">\
             <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
             <label>주제 <input type=\"text\" name=\"topic\" required></label>\
             <label>내용 <textarea name=\"content\" rows=\"5\" required></textarea></label>\
             <button type=\"submit\">토론 열기</button>\
             </form>",
            title = escape(&title.to_string()),
            csrf_token = escape(&csrf_token),
        ));
    } else {
        body.push_str("<p>토론을 열 권한이 없습니다.</p>");
    }

    let page = shell(
        &state,
        &requester,
        format!("{title} (토론)"),
        body,
        &csrf_token,
    )
    .await?
    .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct NewThreadSubmission {
    csrf_token: String,
    topic: String,
    content: String,
}

pub async fn create_thread(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
    axum::Form(submission): axum::Form<NewThreadSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester
        .may(&state, &title, AclAction::CreateThread)
        .await?
    {
        return Ok((StatusCode::FORBIDDEN, "토론을 열 권한이 없습니다.").into_response());
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

    Ok(Redirect::to(&format!("/thread/{thread}")).into_response())
}

/// 스레드 하나 — 발언과 관리 조작이 한 타임라인에 섞여 보인다.
pub async fn view_thread(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> HandlerResult {
    let Some(thread) = wiki_discussion::thread_by_id(&state.pool, id).await? else {
        return Ok((StatusCode::NOT_FOUND, "그런 토론이 없습니다.").into_response());
    };
    let comments = wiki_discussion::thread_comments(&state.pool, id).await?;

    let mut body = format!(
        "<p><a href=\"/w/{title}\">{title}</a> · {status}</p><ol class=\"wiki-thread\">",
        title = escape(&thread.title.to_string()),
        status = thread.status.label(),
    );

    for comment in &comments {
        let text = match comment.kind {
            CommentKind::Comment if comment.hidden => "(가려진 발언)".to_owned(),
            CommentKind::Comment => escape(&comment.content),
            CommentKind::StatusChange => {
                format!("상태를 {}(으)로 바꿨습니다.", metadata_value(comment, "to"))
            }
            CommentKind::TopicChange => {
                format!(
                    "주제를 \"{}\"(으)로 바꿨습니다.",
                    metadata_value(comment, "topic")
                )
            }
            CommentKind::DocumentMove => format!(
                "토론을 {} 문서로 옮겼습니다.",
                metadata_value(comment, "document")
            ),
        };
        body.push_str(&format!(
            "<li id=\"{sequence}\">#{sequence} · {author}{admin} · {created}<br>{text}</li>",
            sequence = comment.sequence,
            author = escape(&comment.author),
            admin = if comment.admin_marked { " [ADMIN]" } else { "" },
            created = comment.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
        ));
    }
    body.push_str("</ol>");

    let (jar, csrf_token) = issue_token(jar);

    if thread.status.accepts_comments()
        && requester
            .may(&state, &thread.title, AclAction::WriteThreadComment)
            .await?
    {
        body.push_str(&format!(
            "<form method=\"post\" action=\"/thread/{id}/comment\">\
             <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
             <label>발언 <textarea name=\"content\" rows=\"4\" required></textarea></label>\
             <button type=\"submit\">남기기</button>\
             </form>",
            csrf_token = escape(&csrf_token),
        ));
    } else if !thread.status.accepts_comments() {
        body.push_str("<p>이 토론은 더 이상 발언을 받지 않습니다.</p>");
    }

    // 관리 조작은 그 권한을 가진 사람에게만 보인다.
    if requester
        .has_permission(&state, "update_thread_status")
        .await?
    {
        body.push_str(&format!(
            "<h2>관리</h2>\
             <form method=\"post\" action=\"/thread/{id}/status\">\
             <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
             <select name=\"status\">\
             <option value=\"normal\">정상</option>\
             <option value=\"pause\">중단</option>\
             <option value=\"close\">닫힘</option>\
             </select>\
             <button type=\"submit\">상태 바꾸기</button>\
             </form>",
            csrf_token = escape(&csrf_token),
        ));
    }

    let page = shell(&state, &requester, thread.topic.clone(), body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

fn metadata_value(comment: &wiki_discussion::Comment, key: &str) -> String {
    comment
        .metadata
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
        .map(escape)
        .unwrap_or_default()
}

#[derive(Deserialize)]
pub struct CommentSubmission {
    csrf_token: String,
    content: String,
}

pub async fn add_comment(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(id): Path<Uuid>,
    axum::Form(submission): axum::Form<CommentSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let Some(thread) = wiki_discussion::thread_by_id(&state.pool, id).await? else {
        return Ok((StatusCode::NOT_FOUND, "그런 토론이 없습니다.").into_response());
    };
    if !requester
        .may(&state, &thread.title, AclAction::WriteThreadComment)
        .await?
    {
        return Ok((StatusCode::FORBIDDEN, "발언할 권한이 없습니다.").into_response());
    }

    let actor = requester.actor(&state).await?;
    let admin_marked = requester.has_permission(&state, "admin").await?;
    wiki_discussion::add_comment(&state.pool, id, actor, &submission.content, admin_marked).await?;

    Ok(Redirect::to(&format!("/thread/{id}")).into_response())
}

#[derive(Deserialize)]
pub struct StatusSubmission {
    csrf_token: String,
    status: String,
}

pub async fn change_status(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(id): Path<Uuid>,
    axum::Form(submission): axum::Form<StatusSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }
    if !requester
        .has_permission(&state, "update_thread_status")
        .await?
    {
        return Ok((StatusCode::FORBIDDEN, "상태를 바꿀 권한이 없습니다.").into_response());
    }

    let status = match submission.status.as_str() {
        "pause" => ThreadStatus::Pause,
        "close" => ThreadStatus::Close,
        _ => ThreadStatus::Normal,
    };
    let actor = requester.actor(&state).await?;
    wiki_discussion::change_status(&state.pool, id, actor, status).await?;

    Ok(Redirect::to(&format!("/thread/{id}")).into_response())
}

#[derive(Deserialize)]
pub struct RecentQuery {
    status: Option<String>,
}

/// 최근 토론 — 상태로 걸러 볼 수 있다.
pub async fn recent_discussions(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Query(parameters): Query<RecentQuery>,
) -> HandlerResult {
    let status = parameters.status.as_deref().and_then(|name| match name {
        "normal" => Some(ThreadStatus::Normal),
        "pause" => Some(ThreadStatus::Pause),
        "close" => Some(ThreadStatus::Close),
        _ => None,
    });

    let threads = wiki_discussion::recent_threads(&state.pool, status, 100).await?;

    let mut body = String::from(
        "<p><a href=\"/recent-discussions\">전체</a> · \
         <a href=\"/recent-discussions?status=normal\">정상</a> · \
         <a href=\"/recent-discussions?status=pause\">중단</a> · \
         <a href=\"/recent-discussions?status=close\">닫힘</a></p>\
         <ul class=\"wiki-recent-discussions\">",
    );
    if threads.is_empty() {
        body.push_str("<li>토론이 없습니다.</li>");
    }
    for thread in &threads {
        body.push_str(&format!(
            "<li><a href=\"/thread/{id}\">{topic}</a> · \
             <a href=\"/w/{title}\">{title}</a> · {status} · {created}</li>",
            id = thread.external_id,
            topic = escape(&thread.topic),
            title = escape(&thread.title.to_string()),
            status = thread.status.label(),
            created = thread.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
        ));
    }
    body.push_str("</ul>");

    let (jar, csrf_token) = issue_token(jar);
    let page = shell(&state, &requester, "최근 토론", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

/// 편집요청 목록.
pub async fn edit_requests(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    let requests = wiki_discussion::open_edit_requests(&state.pool, 100).await?;

    let mut body = String::from("<ul class=\"wiki-edit-requests\">");
    if requests.is_empty() {
        body.push_str("<li>열린 편집요청이 없습니다.</li>");
    }
    for request in &requests {
        body.push_str(&format!(
            "<li><a href=\"/edit-request/{id}\">{title}</a> · {author} · {comment}</li>",
            id = request.external_id,
            title = escape(&request.title.to_string()),
            author = escape(&request.author),
            comment = escape(&request.comment),
        ));
    }
    body.push_str("</ul>");

    let (jar, csrf_token) = issue_token(jar);
    let page = shell(&state, &requester, "편집요청", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

/// 편집요청 하나 — 제안된 원문과 처리 단추.
pub async fn view_edit_request(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> HandlerResult {
    let Some(request) = wiki_discussion::edit_request_by_id(&state.pool, id).await? else {
        return Ok((StatusCode::NOT_FOUND, "그런 편집요청이 없습니다.").into_response());
    };

    let current = wiki_document::read_source(&state.pool, &request.title)
        .await?
        .unwrap_or_default();

    let mut body = format!(
        "<p><a href=\"/w/{title}\">{title}</a> · {author} · {status}</p><p>{comment}</p>",
        title = escape(&request.title.to_string()),
        author = escape(&request.author),
        status = request.status.label(),
        comment = escape(&request.comment),
    );

    body.push_str("<pre class=\"wiki-diff\">");
    for line in wiki_document::diff_lines(&current, &request.content) {
        let (marker, class) = match line.kind {
            wiki_document::DiffLineKind::Inserted => ('+', "wiki-diff-insert"),
            wiki_document::DiffLineKind::Deleted => ('-', "wiki-diff-delete"),
            wiki_document::DiffLineKind::Context => (' ', "wiki-diff-context"),
        };
        body.push_str(&format!(
            "<span class=\"{class}\">{marker}{}</span>\n",
            escape(&line.text)
        ));
    }
    body.push_str("</pre>");

    let (jar, csrf_token) = issue_token(jar);

    // 반영은 그 문서를 편집할 수 있는 사람만 한다 — 편집요청이 권한 우회로가 되지 않게.
    if request.status == EditRequestStatus::Open
        && requester
            .may(&state, &request.title, AclAction::Edit)
            .await?
    {
        body.push_str(&format!(
            "<form method=\"post\" action=\"/edit-request/{id}/accept\">\
             <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
             <button type=\"submit\">반영</button>\
             </form>\
             <form method=\"post\" action=\"/edit-request/{id}/close\">\
             <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
             <button type=\"submit\">닫기</button>\
             </form>",
            csrf_token = escape(&csrf_token),
        ));
    }

    let page = shell(
        &state,
        &requester,
        format!("{} 편집요청", request.title),
        body,
        &csrf_token,
    )
    .await?
    .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct NewEditRequestSubmission {
    csrf_token: String,
    content: String,
    comment: String,
    base_revision: String,
}

/// 편집 권한이 없을 때 변경안을 내는 통로.
pub async fn submit_edit_request(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
    axum::Form(submission): axum::Form<NewEditRequestSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester
        .may(&state, &title, AclAction::EditRequest)
        .await?
    {
        return Ok((StatusCode::FORBIDDEN, "편집요청을 낼 권한이 없습니다.").into_response());
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

    Ok(Redirect::to(&format!("/edit-request/{request}")).into_response())
}

#[derive(Deserialize)]
pub struct ReviewSubmission {
    csrf_token: String,
}

/// 요청을 문서에 반영한다 — 리비전은 편집 경로를 그대로 탄다.
pub async fn accept_edit_request(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(id): Path<Uuid>,
    axum::Form(submission): axum::Form<ReviewSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let Some(request) = wiki_discussion::edit_request_by_id(&state.pool, id).await? else {
        return Ok((StatusCode::NOT_FOUND, "그런 편집요청이 없습니다.").into_response());
    };
    if !requester
        .may(&state, &request.title, AclAction::Edit)
        .await?
    {
        return Ok((StatusCode::FORBIDDEN, "반영할 권한이 없습니다.").into_response());
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

    Ok(Redirect::to(&format!("/w/{}", request.title)).into_response())
}

pub async fn close_edit_request(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(id): Path<Uuid>,
    axum::Form(submission): axum::Form<ReviewSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let Some(request) = wiki_discussion::edit_request_by_id(&state.pool, id).await? else {
        return Ok((StatusCode::NOT_FOUND, "그런 편집요청이 없습니다.").into_response());
    };
    if !requester
        .may(&state, &request.title, AclAction::Edit)
        .await?
    {
        return Ok((StatusCode::FORBIDDEN, "닫을 권한이 없습니다.").into_response());
    }

    let actor = requester.actor(&state).await?;
    wiki_discussion::close_edit_request(&state.pool, id, actor).await?;

    Ok(Redirect::to(&format!("/edit-request/{id}")).into_response())
}
