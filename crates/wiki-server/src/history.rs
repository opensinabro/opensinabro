use askama::Template;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wiki_document::{DiffLineKind, DocumentTitle, RevisionKind};

use crate::ServerError;
use crate::handler::{escape, namespace_names, shell};
use crate::security::{issue_token, verify_token};
use crate::session::Requester;
use crate::state::AppState;

type HandlerResult = Result<Response, ServerError>;

const HISTORY_LIMIT: i64 = 100;

/// 문서 역사.
pub async fn history(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
) -> HandlerResult {
    let (jar, csrf_token) = issue_token(jar);
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);
    let revisions = wiki_document::revision_history(&state.pool, &title, HISTORY_LIMIT).await?;

    if revisions.is_empty() {
        let body = format!(
            "<p>\"{}\" 문서의 역사가 없습니다.</p>",
            escape(&title.to_string())
        );
        let page = shell(&state, &requester, title.to_string(), body, &csrf_token)
            .await?
            .render()?;
        return Ok((StatusCode::NOT_FOUND, Html(page)).into_response());
    }

    let mut body = format!(
        "<p><a href=\"/blame/{title}\">blame</a> · \
         <a href=\"/move/{title}\">이동</a> · <a href=\"/delete/{title}\">삭제</a></p>\
         <ul class=\"wiki-history\">",
        title = escape(&title.to_string())
    );
    for revision in &revisions {
        body.push_str(&format!(
            "<li>r{sequence} · <a href=\"/raw/{title}?uuid={uuid}\">원문</a> \
             · <a href=\"/diff/{title}?uuid={uuid}\">비교</a> \
             · <a href=\"/revert/{title}?uuid={uuid}\">되돌리기</a> \
             · {author} · {created} · {bytes}바이트{comment}</li>",
            sequence = revision.sequence,
            title = escape(&title.to_string()),
            uuid = revision.external_id,
            author = escape(&revision.author),
            created = format_time(revision.created_at),
            bytes = revision.content_bytes,
            comment = if revision.comment.is_empty() {
                String::new()
            } else {
                format!(" · {}", escape(&revision.comment))
            },
        ));
    }
    body.push_str("</ul>");

    if requester.has_permission(&state, "hide_revision").await? {
        body.push_str(&format!(
            "<h2>리비전 숨기기</h2>\
             <form method=\"post\" action=\"/hide-revision/{title}\">\
             <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
             <label>리비전 <input type=\"text\" name=\"uuid\" required></label>\
             <label><input type=\"checkbox\" name=\"show\" value=\"1\"> 도로 보이기</label>\
             <button type=\"submit\">적용</button>\
             </form>",
            title = escape(&title.to_string()),
            csrf_token = escape(&csrf_token),
        ));
    }

    let page = shell(
        &state,
        &requester,
        format!("{title} (역사)"),
        body,
        &csrf_token,
    )
    .await?
    .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct RevisionQuery {
    uuid: Option<Uuid>,
}

/// 리비전 비교 — 지정한 리비전과 그 직전을 견준다.
pub async fn diff(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
    Query(parameters): Query<RevisionQuery>,
) -> HandlerResult {
    let (jar, csrf_token) = issue_token(jar);
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);
    let revisions = wiki_document::revision_history(&state.pool, &title, 500).await?;

    let Some(position) = parameters
        .uuid
        .as_ref()
        .and_then(|uuid| revisions.iter().position(|item| &item.external_id == uuid))
    else {
        return Ok((StatusCode::NOT_FOUND, "리비전을 찾을 수 없습니다.").into_response());
    };

    let after = content_of(&state, revisions[position].external_id).await?;
    let before = match revisions.get(position + 1) {
        Some(previous) => content_of(&state, previous.external_id).await?,
        None => String::new(),
    };

    let mut body = String::from("<pre class=\"wiki-diff\">");
    for line in wiki_document::diff_lines(&before, &after) {
        let (marker, class) = match line.kind {
            DiffLineKind::Inserted => ('+', "wiki-diff-insert"),
            DiffLineKind::Deleted => ('-', "wiki-diff-delete"),
            DiffLineKind::Context => (' ', "wiki-diff-context"),
        };
        body.push_str(&format!(
            "<span class=\"{class}\">{marker}{}</span>\n",
            escape(&line.text)
        ));
    }
    body.push_str("</pre>");

    let page = shell(
        &state,
        &requester,
        format!("{title} (비교)"),
        body,
        &csrf_token,
    )
    .await?
    .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct RevertSubmission {
    csrf_token: String,
    uuid: Uuid,
}

/// 되돌리기 확인 화면 — 되돌림은 상태를 바꾸므로 GET으로 실행하지 않는다.
pub async fn revert_form(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
    Query(parameters): Query<RevisionQuery>,
) -> HandlerResult {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    let Some(uuid) = parameters.uuid else {
        return Ok((StatusCode::BAD_REQUEST, "리비전을 지정하세요.").into_response());
    };

    let (jar, csrf_token) = issue_token(jar);
    let body = format!(
        "<form method=\"post\" action=\"/revert/{title}\">\
         <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
         <input type=\"hidden\" name=\"uuid\" value=\"{uuid}\">\
         <p>이 리비전의 내용으로 되돌립니다.</p>\
         <button type=\"submit\">되돌리기</button> <a href=\"/history/{title}\">취소</a>\
         </form>",
        title = escape(&title.to_string()),
        csrf_token = escape(&csrf_token),
        uuid = uuid,
    );

    let page = shell(
        &state,
        &requester,
        format!("{title} (되돌리기)"),
        body,
        &csrf_token,
    )
    .await?
    .render()?;
    Ok((jar, Html(page)).into_response())
}

/// 되돌리기 실행 — 옛 내용으로 새 리비전을 남긴다(역사를 지우지 않는다).
pub async fn revert_submit(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
    axum::Form(submission): axum::Form<RevertSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);
    let content = content_of(&state, submission.uuid).await?;

    let actor = requester.actor(&state).await?;
    wiki_document::record_revision(
        &state.pool,
        &title,
        actor,
        RevisionKind::Revert,
        Some(&content),
        "되돌림",
        Some(serde_json::json!({ "to_revision": submission.uuid })),
    )
    .await?;

    crate::edit::apply_side_effects(&state, &title, &content).await?;

    Ok(Redirect::to(&format!("/w/{title}")).into_response())
}

/// 최근 변경.
pub async fn recent_changes(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    let changes = wiki_document::recent_changes(&state.pool, HISTORY_LIMIT).await?;

    let mut body = String::from("<ul class=\"wiki-recent-changes\">");
    for change in &changes {
        let title = escape(&change.title.to_string());
        body.push_str(&format!(
            "<li><a href=\"/w/{title}\">{title}</a> \
             · <a href=\"/diff/{title}?uuid={uuid}\">비교</a> \
             · {kind} · {author} · {created}{comment}</li>",
            uuid = change.revision.external_id,
            kind = kind_label(change.revision.kind),
            author = escape(&change.revision.author),
            created = format_time(change.revision.created_at),
            comment = if change.revision.comment.is_empty() {
                String::new()
            } else {
                format!(" · {}", escape(&change.revision.comment))
            },
        ));
    }
    body.push_str("</ul>");

    let (jar, csrf_token) = issue_token(jar);
    let page = shell(&state, &requester, "최근 변경", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

fn kind_label(kind: RevisionKind) -> &'static str {
    match kind {
        RevisionKind::Create => "새 문서",
        RevisionKind::Edit => "편집",
        RevisionKind::Move => "이동",
        RevisionKind::Delete => "삭제",
        RevisionKind::Restore => "복원",
        RevisionKind::Revert => "되돌림",
        RevisionKind::Import => "가져옴",
    }
}

async fn content_of(state: &AppState, external_id: Uuid) -> Result<String, ServerError> {
    Ok(wiki_document::revision_content(&state.pool, external_id)
        .await?
        .flatten()
        .unwrap_or_default())
}

/// 화면에 보일 시각 표기. 저장은 UTC이고 표시는 초 단위까지만 보인다.
fn format_time(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

#[derive(Serialize)]
pub struct HistoryPayload {
    title: String,
    revisions: Vec<crate::api::RevisionSummary>,
}

/// 문서 역사.
pub async fn history_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(raw_title): Path<String>,
) -> Result<Response, ServerError> {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester
        .may(&state, &title, wiki_authorization::AclAction::Read)
        .await?
    {
        return Ok(crate::api::forbidden());
    }

    let revisions = wiki_document::revision_history(&state.pool, &title, HISTORY_LIMIT).await?;
    if revisions.is_empty() {
        return Ok(crate::api::not_found());
    }

    Ok(Json(HistoryPayload {
        title: title.to_string(),
        revisions: revisions.iter().map(Into::into).collect(),
    })
    .into_response())
}

#[derive(Serialize)]
pub struct RecentChangeEntry {
    title: String,
    revision: crate::api::RevisionSummary,
}

/// 최근 변경. 문서마다 권한을 묻지 않으므로 제목만 드러난다 — 목록의 성격이
/// 그러하고, 본문은 어차피 문서 보기가 다시 판정한다.
pub async fn recent_changes_api(
    State(state): State<AppState>,
) -> Result<Json<Vec<RecentChangeEntry>>, ServerError> {
    let changes = wiki_document::recent_changes(&state.pool, HISTORY_LIMIT).await?;

    Ok(Json(
        changes
            .iter()
            .map(|change| RecentChangeEntry {
                title: change.title.to_string(),
                revision: (&change.revision).into(),
            })
            .collect(),
    ))
}
