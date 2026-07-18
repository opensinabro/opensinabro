use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wiki_document::{DiffLineKind, DocumentTitle, RevisionKind};

use crate::ServerError;
use crate::handler::namespace_names;
use crate::security::verify_header;
use crate::session::Requester;
use crate::state::AppState;

const HISTORY_LIMIT: i64 = 100;

#[derive(Deserialize)]
pub struct RevisionQuery {
    pub(crate) uuid: Option<Uuid>,
}

async fn content_of(state: &AppState, external_id: Uuid) -> Result<String, ServerError> {
    Ok(wiki_document::revision_content(&state.pool, external_id)
        .await?
        .flatten()
        .unwrap_or_default())
}

/// 이 uuid가 정말 이 문서의 리비전인가. 맞으면 그 리비전의 순번을 낸다.
///
/// 리비전 uuid는 문서와 독립된 값이라, 경로의 제목만 믿으면 남의 문서 리비전을
/// 이 문서인 양 다루게 된다 — 되돌리기·숨기기가 모두 이 확인을 거친다.
pub(crate) async fn revision_sequence_within(
    state: &AppState,
    title: &DocumentTitle,
    external_id: Uuid,
) -> Result<Option<i64>, ServerError> {
    let row = sqlx::query_as::<_, (i64,)>(
        "SELECT revision.sequence
         FROM revision
         JOIN document ON document.id = revision.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE revision.external_id = $1 AND namespace.name = $2 AND document.title = $3",
    )
    .bind(external_id)
    .bind(title.namespace.as_str())
    .bind(&title.name)
    .fetch_optional(&state.pool)
    .await?;

    Ok(row.map(|(sequence,)| sequence))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLineEntry {
    kind: &'static str,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffPayload {
    title: String,
    sequence: i64,
    lines: Vec<DiffLineEntry>,
}

/// 리비전 비교 — 지정한 리비전과 그 직전을 견준다.
pub async fn diff_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(raw_title): Path<String>,
    Query(parameters): Query<RevisionQuery>,
) -> Result<Response, ServerError> {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    // 비교 결과에는 양쪽 리비전의 본문이 그대로 실린다 — 폼 경로는 읽기 권한을 묻지
    // 않는데, 그러면 못 읽는 문서의 내용이 비교로 새어 나간다.
    if !requester
        .may(&state, &title, wiki_authorization::AclAction::Read)
        .await?
    {
        return Ok(crate::api::forbidden());
    }

    let revisions = wiki_document::revision_history(&state.pool, &title, 500).await?;
    let Some(position) = parameters
        .uuid
        .as_ref()
        .and_then(|uuid| revisions.iter().position(|item| &item.external_id == uuid))
    else {
        return Ok(crate::api::not_found());
    };

    let after = content_of(&state, revisions[position].external_id).await?;
    let before = match revisions.get(position + 1) {
        Some(previous) => content_of(&state, previous.external_id).await?,
        None => String::new(),
    };

    let lines = wiki_document::diff_lines(&before, &after)
        .into_iter()
        .map(|line| DiffLineEntry {
            kind: match line.kind {
                DiffLineKind::Inserted => "inserted",
                DiffLineKind::Deleted => "deleted",
                DiffLineKind::Context => "context",
            },
            text: line.text,
        })
        .collect();

    Ok(Json(DiffPayload {
        title: title.to_string(),
        sequence: revisions[position].sequence,
        lines,
    })
    .into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevertPayload {
    title: String,
    sequence: i64,
    may: bool,
}

/// 되돌릴 수 있는가. 되돌림은 상태를 바꾸므로 GET으로 실행하지 않는다.
pub async fn revert_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(raw_title): Path<String>,
    Query(parameters): Query<RevisionQuery>,
) -> Result<Response, ServerError> {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    let Some(uuid) = parameters.uuid else {
        return Ok(crate::api::not_found());
    };
    let Some(sequence) = revision_sequence_within(&state, &title, uuid).await? else {
        return Ok(crate::api::not_found());
    };

    Ok(Json(RevertPayload {
        title: title.to_string(),
        sequence,
        may: requester
            .may(&state, &title, wiki_authorization::AclAction::Edit)
            .await?,
    })
    .into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevertRequestBody {
    uuid: Uuid,
}

/// 되돌리기 실행 — 옛 내용으로 새 리비전을 남긴다(역사를 지우지 않는다).
pub async fn revert_submit_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(raw_title): Path<String>,
    Json(submission): Json<RevertRequestBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    // 폼 경로는 CSRF만 보고 권한을 전혀 묻지 않는다 — 누구나 아무 문서나 되돌릴 수
    // 있는 구멍이라 여기서는 편집 권한을 판정한다.
    if !requester
        .may(&state, &title, wiki_authorization::AclAction::Edit)
        .await?
    {
        return Ok(crate::api::forbidden());
    }

    // 대상 리비전이 이 문서의 것이어야 한다. 아니면 남의 문서 내용을 이 문서에
    // 덮어쓰게 된다.
    if revision_sequence_within(&state, &title, submission.uuid)
        .await?
        .is_none()
    {
        return Ok(crate::api::not_found());
    }

    // 내용을 읽지 못하면(숨겨졌거나 비어 있으면) 되돌림이 문서를 비워 버린다.
    let Some(Some(content)) = wiki_document::revision_content(&state.pool, submission.uuid).await?
    else {
        return Ok(crate::api::not_found());
    };

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

    Ok(Json(serde_json::json!({ "title": title.to_string() })).into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPayload {
    title: String,
    revisions: Vec<crate::api::RevisionSummary>,
    /// 이 사람이 리비전을 가릴 수 있는가. 화면이 줄마다 숨김 단추를 보일지 정한다.
    may_hide_revision: bool,
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
        may_hide_revision: requester.has_permission(&state, "hide_revision").await?,
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
