use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use wiki_authorization::AclAction;
use wiki_document::{DocumentTitle, MergeOutcome, RevisionKind};

use crate::ServerError;
use crate::handler::namespace_names;
use crate::security::{issue_token, verify_header};
use crate::session::Requester;
use crate::state::AppState;

/// 저장 뒤에 따라오는 갱신들 — 역링크·검색 색인.
pub(crate) async fn apply_side_effects(
    state: &AppState,
    title: &DocumentTitle,
    content: &str,
) -> Result<(), ServerError> {
    let rendered = wiki_document::render_document(&state.pool, title, content).await?;

    wiki_document::replace_references(&state.pool, title, &rendered.references).await?;

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditPayload {
    title: String,
    content: String,
    base_revision: String,
    /// 편집할 수 없지만 변경안은 낼 수 있는 문서가 있다 — the seed의 편집요청 유도.
    edit_request_only: bool,
}

/// 편집 화면이 필요한 것 — 원문과 편집을 시작한 시점의 리비전.
pub async fn edit_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(raw_title): Path<String>,
) -> Result<Response, ServerError> {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    let may_edit = requester.may(&state, &title, AclAction::Edit).await?;
    if !may_edit
        && !requester
            .may(&state, &title, AclAction::EditRequest)
            .await?
    {
        return Ok(crate::api::forbidden());
    }

    Ok(Json(EditPayload {
        title: title.to_string(),
        content: wiki_document::read_source(&state.pool, &title)
            .await?
            .unwrap_or_default(),
        base_revision: latest_identifier(&state, &title).await?,
        edit_request_only: !may_edit,
    })
    .into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditRequestBody {
    base_revision: String,
    content: String,
    comment: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictPayload {
    /// 충돌 표시(`<<<<<<<`)가 섞인 원문. 편집자가 정리해 다시 낸다.
    content: String,
    base_revision: String,
}

/// 편집 저장. 저장한 뒤 어디로 갈지는 화면이 정한다.
pub async fn edit_submit_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(raw_title): Path<String>,
    Json(submission): Json<EditRequestBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester.may(&state, &title, AclAction::Edit).await? {
        return Ok(crate::api::forbidden());
    }

    let current = wiki_document::read_source(&state.pool, &title)
        .await?
        .unwrap_or_default();
    let latest = latest_identifier(&state, &title).await?;

    // 편집하는 사이에 다른 사람이 저장했으면 세 원문을 합쳐 본다.
    let content = if latest == submission.base_revision {
        submission.content
    } else {
        let base = base_content(&state, &submission.base_revision).await?;
        match wiki_document::merge_edits(&base, &current, &submission.content) {
            MergeOutcome::Merged(merged) => merged,
            MergeOutcome::Conflicted(conflicted) => {
                return Ok((
                    StatusCode::CONFLICT,
                    Json(ConflictPayload {
                        content: conflicted,
                        base_revision: latest,
                    }),
                )
                    .into_response());
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

    Ok(Json(serde_json::json!({ "title": title.to_string() })).into_response())
}

#[derive(Deserialize)]
pub struct PreviewBody {
    title: String,
    content: String,
}

/// 저장하지 않고 렌더만 해 본다. 결과를 캐시에 남기지 않는다.
pub async fn preview_api(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Json(body): Json<PreviewBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&body.title, &namespaces);
    let rendered = wiki_document::render_document(&state.pool, &title, &body.content).await?;

    Ok(Json(serde_json::json!({ "html": rendered.html })).into_response())
}

/// 상태를 바꾸는 API를 부르기 전에 화면이 받아 가는 토큰.
pub async fn csrf_api(jar: CookieJar) -> Response {
    let (jar, token) = issue_token(jar);
    (jar, Json(serde_json::json!({ "token": token }))).into_response()
}

async fn latest_identifier(state: &AppState, title: &DocumentTitle) -> Result<String, ServerError> {
    Ok(wiki_document::latest_revision(&state.pool, title)
        .await?
        .map(|revision| revision.external_id.to_string())
        .unwrap_or_default())
}
