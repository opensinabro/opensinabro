use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wiki_authorization::AclAction;
use wiki_document::{DocumentTitle, RevisionKind};

use crate::ServerError;
use crate::handler::namespace_names;
use crate::security::verify_header;
use crate::session::Requester;
use crate::state::AppState;

/// 삭제·이동 사유는 짧게 적어 넘길 수 없게 한다 (the seed도 5자 이상을 요구한다).
const MINIMUM_REASON_LENGTH: usize = 5;

#[derive(Deserialize)]
pub struct BatchRevertQuery {
    author: Option<String>,
}

/// 사유가 짧아 되돌려보낼 때. 권한과 달리 고쳐서 다시 낼 수 있는 입력 문제라
/// 403이 아니라 400으로 낸다 — 폼 경로는 403을 내는데 이는 결함이다.
fn bad_request(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}

const SHORT_REASON_MESSAGE: &str = "사유를 다섯 자 이상 적어 주세요.";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationPayload {
    title: String,
    may: bool,
}

/// 이 문서를 옮길 수 있는가.
pub async fn move_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(raw_title): Path<String>,
) -> Result<Response, ServerError> {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    Ok(axum::Json(OperationPayload {
        may: requester.may(&state, &title, AclAction::Move).await?,
        title: title.to_string(),
    })
    .into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveRequestBody {
    target: String,
    comment: String,
}

pub async fn move_submit_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(raw_title): Path<String>,
    axum::Json(submission): axum::Json<MoveRequestBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let namespaces = namespace_names(&state).await?;
    let from = DocumentTitle::parse(&raw_title, &namespaces);
    let to = DocumentTitle::parse(&submission.target, &namespaces);

    // 폼 경로는 출발 제목만 검사하는데, 그러면 권한이 미치지 않는 자리로 문서를 밀어
    // 넣을 수 있다 — 도착 제목도 함께 판정한다.
    if !requester.may(&state, &from, AclAction::Move).await?
        || !requester.may(&state, &to, AclAction::Move).await?
    {
        return Ok(crate::api::forbidden());
    }
    if submission.comment.chars().count() < MINIMUM_REASON_LENGTH {
        return Ok(bad_request(SHORT_REASON_MESSAGE));
    }

    let actor = requester.actor(&state).await?;
    wiki_document::move_document(&state.pool, &from, &to, actor, &submission.comment).await?;

    state.search.remove(from.namespace.as_str(), &from.name)?;
    if let Some(source) = wiki_document::read_source(&state.pool, &to).await? {
        state.search.put(to.namespace.as_str(), &to.name, &source)?;
    }
    state.search.commit()?;

    Ok(axum::Json(serde_json::json!({ "title": to.to_string() })).into_response())
}

/// 이 문서를 지울 수 있는가.
pub async fn delete_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(raw_title): Path<String>,
) -> Result<Response, ServerError> {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    Ok(axum::Json(OperationPayload {
        may: requester.may(&state, &title, AclAction::Delete).await?,
        title: title.to_string(),
    })
    .into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRequestBody {
    comment: String,
}

pub async fn delete_submit_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(raw_title): Path<String>,
    axum::Json(submission): axum::Json<DeleteRequestBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester.may(&state, &title, AclAction::Delete).await? {
        return Ok(crate::api::forbidden());
    }
    if submission.comment.chars().count() < MINIMUM_REASON_LENGTH {
        return Ok(bad_request(SHORT_REASON_MESSAGE));
    }

    let actor = requester.actor(&state).await?;
    wiki_document::delete_document(&state.pool, &title, actor, &submission.comment).await?;

    state.search.remove(title.namespace.as_str(), &title.name)?;
    state.search.commit()?;

    Ok(axum::Json(serde_json::json!({ "title": title.to_string() })).into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameLineEntry {
    sequence: i64,
    author: String,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlamePayload {
    title: String,
    lines: Vec<BlameLineEntry>,
}

/// 줄마다 마지막으로 손댄 사람을 보인다.
pub async fn blame_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(raw_title): Path<String>,
) -> Result<Response, ServerError> {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    // blame은 본문을 줄 단위로 그대로 드러내므로 읽기 권한을 물어야 한다 — 폼 경로는
    // 묻지 않는데, 그러면 못 읽는 문서의 내용이 blame으로 새어 나간다.
    if !requester.may(&state, &title, AclAction::Read).await? {
        return Ok(crate::api::forbidden());
    }

    let lines = wiki_document::blame(&state.pool, &title).await?;

    Ok(axum::Json(BlamePayload {
        title: title.to_string(),
        lines: lines
            .into_iter()
            .map(|line| BlameLineEntry {
                sequence: line.sequence,
                author: line.author,
                text: line.text,
            })
            .collect(),
    })
    .into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HideRevisionRequestBody {
    uuid: Uuid,
    hidden: bool,
}

/// 리비전 숨김·해제. 목록에는 남고 내용만 가린다.
pub async fn hide_revision_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(raw_title): Path<String>,
    axum::Json(submission): axum::Json<HideRevisionRequestBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }
    if !requester.is_member() {
        return Ok(crate::api::unauthorized());
    }
    if !requester.has_permission(&state, "hide_revision").await? {
        return Ok(crate::api::forbidden());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    // 폼 경로는 uuid가 이 문서의 것인지 보지 않는다 — 경로의 제목과 무관하게 아무
    // 문서의 리비전이나 숨길 수 있어 결함이다.
    if crate::history::revision_sequence_within(&state, &title, submission.uuid)
        .await?
        .is_none()
    {
        return Ok(crate::api::not_found());
    }

    wiki_document::set_revision_hidden(&state.pool, submission.uuid, submission.hidden).await?;

    Ok(axum::Json(serde_json::json!({ "ok": true })).into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRevertPayload {
    author: String,
    titles: Vec<String>,
}

/// 한 사람이 마지막으로 손댄 문서들 — 되돌리기 전에 대상을 먼저 보인다.
pub async fn batch_revert_api(
    State(state): State<AppState>,
    requester: Requester,
    Query(parameters): Query<BatchRevertQuery>,
) -> Result<Response, ServerError> {
    if !requester.is_member() {
        return Ok(crate::api::unauthorized());
    }
    if !requester.has_permission(&state, "batch_revert").await? {
        return Ok(crate::api::forbidden());
    }

    let author = parameters.author.unwrap_or_default();
    let titles = if author.is_empty() {
        Vec::new()
    } else {
        wiki_document::documents_last_edited_by(&state.pool, &author, 100)
            .await?
            .iter()
            .map(ToString::to_string)
            .collect()
    };

    Ok(axum::Json(BatchRevertPayload { author, titles }).into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRevertRequestBody {
    author: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRevertResultPayload {
    reverted: usize,
}

pub async fn batch_revert_submit_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    axum::Json(submission): axum::Json<BatchRevertRequestBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }
    if !requester.is_member() {
        return Ok(crate::api::unauthorized());
    }
    if !requester.has_permission(&state, "batch_revert").await? {
        return Ok(crate::api::forbidden());
    }

    let targets =
        wiki_document::documents_last_edited_by(&state.pool, &submission.author, 100).await?;
    let actor = requester.actor(&state).await?;
    let mut reverted = 0;

    for title in &targets {
        let Some(content) =
            wiki_document::content_before_author(&state.pool, title, &submission.author).await?
        else {
            continue;
        };

        wiki_document::record_revision(
            &state.pool,
            title,
            actor,
            RevisionKind::Revert,
            Some(&content),
            &format!("{}의 편집 일괄 되돌림", submission.author),
            Some(serde_json::json!({ "batch_revert": submission.author })),
        )
        .await?;
        crate::edit::apply_side_effects(&state, title, &content).await?;
        reverted += 1;
    }

    Ok(axum::Json(BatchRevertResultPayload { reverted }).into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPayload {
    wiki_name: String,
    main_document: String,
    content_license: String,
}

/// 위키 전역 설정.
pub async fn config_api(
    State(state): State<AppState>,
    requester: Requester,
) -> Result<Response, ServerError> {
    if !requester.is_member() {
        return Ok(crate::api::unauthorized());
    }
    if !requester.has_permission(&state, "config").await? {
        return Ok(crate::api::forbidden());
    }

    Ok(axum::Json(ConfigPayload {
        wiki_name: state.settings.wiki_name.clone(),
        main_document: state.settings.main_document.clone(),
        content_license: state.settings.content_license.clone(),
    })
    .into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigRequestBody {
    wiki_name: String,
    main_document: String,
    content_license: String,
}

pub async fn config_submit_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    axum::Json(submission): axum::Json<ConfigRequestBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }
    if !requester.is_member() {
        return Ok(crate::api::unauthorized());
    }
    if !requester.has_permission(&state, "config").await? {
        return Ok(crate::api::forbidden());
    }

    for (name, data) in [
        ("wiki_name", &submission.wiki_name),
        ("main_document", &submission.main_document),
        ("content_license", &submission.content_license),
    ] {
        sqlx::query(
            "INSERT INTO site_setting (name, data) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET data = excluded.data",
        )
        .bind(name)
        .bind(data)
        .execute(&state.pool)
        .await?;
    }

    Ok(axum::Json(ConfigPayload {
        wiki_name: submission.wiki_name,
        main_document: submission.main_document,
        content_license: submission.content_license,
    })
    .into_response())
}
