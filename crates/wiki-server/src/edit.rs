use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use wiki_authorization::AclAction;
use wiki_document::{DocumentTitle, MergeOutcome, RevisionKind};

use crate::ServerError;
use crate::handler::{escape, namespace_names};
use crate::security::{issue_token, verify_token};
use crate::session::Requester;
use crate::state::AppState;
use crate::template::{EditForm, Shell};

type HandlerResult = Result<Response, ServerError>;

#[derive(Deserialize)]
pub struct EditSubmission {
    csrf_token: String,
    base_revision: String,
    content: String,
    comment: String,
}

/// 편집 폼.
pub async fn edit_form(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
) -> HandlerResult {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester.may(&state, &title, AclAction::Edit).await? {
        // 편집은 막혔어도 변경안은 낼 수 있는 문서가 있다 — the seed의 편집요청 유도.
        if requester
            .may(&state, &title, AclAction::EditRequest)
            .await?
        {
            return edit_request_form(&state, &requester, jar, &title).await;
        }
        return forbidden(&state, &title);
    }

    let content = wiki_document::read_source(&state.pool, &title)
        .await?
        .unwrap_or_default();
    let base_revision = wiki_document::latest_revision(&state.pool, &title)
        .await?
        .map(|revision| revision.external_id.to_string())
        .unwrap_or_default();

    let (jar, csrf_token) = issue_token(jar);
    let form = EditForm {
        title: title.to_string(),
        content,
        base_revision,
        csrf_token,
        conflict_message: None,
    }
    .render()?;

    let page = Shell::new(&state.settings, format!("{title} (편집)"), form).render()?;
    Ok((jar, Html(page)).into_response())
}

/// 편집 저장. 저장 뒤에는 303으로 보기 화면에 보낸다(PRG).
pub async fn edit_submit(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
    axum::Form(submission): axum::Form<EditSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester.may(&state, &title, AclAction::Edit).await? {
        return forbidden(&state, &title);
    }

    let current = wiki_document::read_source(&state.pool, &title)
        .await?
        .unwrap_or_default();
    let latest = wiki_document::latest_revision(&state.pool, &title)
        .await?
        .map(|revision| revision.external_id.to_string())
        .unwrap_or_default();

    // 편집하는 사이에 다른 사람이 저장했으면 세 원문을 합쳐 본다.
    let content = if latest == submission.base_revision {
        submission.content
    } else {
        let base = base_content(&state, &submission.base_revision).await?;
        match wiki_document::merge_edits(&base, &current, &submission.content) {
            MergeOutcome::Merged(merged) => merged,
            MergeOutcome::Conflicted(conflicted) => {
                return conflict_page(&state, jar, &title, conflicted, &latest);
            }
        }
    };

    let actor = requester.actor(&state).await?;
    let kind = if current.is_empty() {
        RevisionKind::Create
    } else {
        RevisionKind::Edit
    };

    wiki_document::record_revision(
        &state.pool,
        &title,
        actor,
        kind,
        Some(&content),
        &submission.comment,
        None,
    )
    .await?;

    apply_side_effects(&state, &title, &content).await?;

    Ok(Redirect::to(&format!("/w/{title}")).into_response())
}

/// 저장 뒤에 따라오는 갱신들 — 역링크·렌더 캐시·검색 색인.
pub(crate) async fn apply_side_effects(
    state: &AppState,
    title: &DocumentTitle,
    content: &str,
) -> Result<(), ServerError> {
    let rendered = wiki_document::render_document(&state.pool, title, content).await?;

    wiki_document::replace_references(&state.pool, title, &rendered.references).await?;
    wiki_document::store_render(&state.pool, title, &rendered.html).await?;
    // 이 문서가 생기거나 내용이 바뀌면 이 문서를 가리키던 쪽의 렌더 결과도 달라진다.
    wiki_document::invalidate_referrers(&state.pool, title).await?;

    state
        .search
        .put(title.namespace.as_str(), &title.name, content)?;
    state.search.commit()?;

    notify_subscribers(state, title, wiki_account::NotificationKind::ThreadComment).await?;

    Ok(())
}

/// 구독자에게 알린다. 문서는 제목으로만 실어 account가 document를 향하지 않게 한다.
pub(crate) async fn notify_subscribers(
    state: &AppState,
    title: &DocumentTitle,
    kind: wiki_account::NotificationKind,
) -> Result<(), ServerError> {
    for user_id in wiki_document::subscribers(&state.pool, title).await? {
        wiki_account::notify(
            &state.pool,
            wiki_account::UserIdentifier::from_raw(user_id),
            kind,
            serde_json::json!({ "document": title.to_string() }),
        )
        .await?;
    }
    Ok(())
}

/// 편집을 시작한 시점의 원문. 폼이 실어 온 식별자가 형식부터 어긋나면 빈 원문으로
/// 두어, 병합이 "처음부터 새로 쓴 편집"으로 다뤄지게 한다.
async fn base_content(state: &AppState, base_revision: &str) -> Result<String, ServerError> {
    let Ok(identifier) = base_revision.parse() else {
        return Ok(String::new());
    };
    Ok(wiki_document::revision_content(&state.pool, identifier)
        .await?
        .flatten()
        .unwrap_or_default())
}

fn conflict_page(
    state: &AppState,
    jar: CookieJar,
    title: &DocumentTitle,
    conflicted: String,
    latest: &str,
) -> HandlerResult {
    let (jar, csrf_token) = issue_token(jar);
    let form = EditForm {
        title: title.to_string(),
        content: conflicted,
        base_revision: latest.to_owned(),
        csrf_token,
        conflict_message: Some(
            "편집하는 사이에 다른 사람이 문서를 고쳤고, 같은 자리가 겹쳐 자동으로 합치지 \
             못했습니다. 아래에서 충돌 표시(<<<<<<<)를 정리한 뒤 저장하세요."
                .to_owned(),
        ),
    }
    .render()?;

    let page = Shell::new(&state.settings, format!("{title} (편집 충돌)"), form).render()?;
    Ok((jar, Html(page)).into_response())
}

/// 편집 대신 변경안을 내는 폼.
async fn edit_request_form(
    state: &AppState,
    requester: &Requester,
    jar: CookieJar,
    title: &DocumentTitle,
) -> HandlerResult {
    let content = wiki_document::read_source(&state.pool, title)
        .await?
        .unwrap_or_default();
    let base_revision = wiki_document::latest_revision(&state.pool, title)
        .await?
        .map(|revision| revision.external_id.to_string())
        .unwrap_or_default();

    let (jar, csrf_token) = issue_token(jar);
    let body = format!(
        "<p>이 문서를 직접 편집할 권한이 없어, 변경안을 제출합니다. \
         권한이 있는 사람이 검토해 반영합니다.</p>\
         <form method=\"post\" action=\"/new-edit-request/{title}\">\
         <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
         <input type=\"hidden\" name=\"base_revision\" value=\"{base_revision}\">\
         <textarea name=\"content\" rows=\"30\" aria-label=\"문서 원문\">{content}</textarea>\
         <label>요청 사유 <input type=\"text\" name=\"comment\"></label>\
         <button type=\"submit\">변경안 제출</button>\
         </form>",
        title = escape(&title.to_string()),
        csrf_token = escape(&csrf_token),
        base_revision = escape(&base_revision),
        content = escape(&content),
    );

    let page = crate::handler::shell(
        state,
        requester,
        format!("{title} (편집요청)"),
        body,
        &csrf_token,
    )
    .await?
    .render()?;
    Ok((jar, Html(page)).into_response())
}

fn forbidden(state: &AppState, title: &DocumentTitle) -> Result<Response, ServerError> {
    let body = format!(
        "<p>\"{}\" 문서를 편집할 권한이 없습니다.</p>",
        escape(&title.to_string())
    );
    let page = Shell::new(&state.settings, title.to_string(), body).render()?;
    Ok((StatusCode::FORBIDDEN, Html(page)).into_response())
}
