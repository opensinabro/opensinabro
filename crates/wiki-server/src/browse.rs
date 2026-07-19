use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Json, Response};
use axum_extra::extract::CookieJar;
use namumark_ir::RenderTree;
use serde::{Deserialize, Serialize};
use wiki_document::{DocumentTitle, Namespace};

use crate::ServerError;
use crate::api::{
    RevisionSummary, TitleEntry, TitleListPayload, forbidden, not_found, unauthorized,
};
use crate::handler::namespace_names;
use crate::security::verify_header;
use crate::session::Requester;
use crate::state::AppState;

const LIST_LIMIT: i64 = 200;

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "include" => "포함",
        "redirect" => "넘겨주기",
        "image" => "이미지",
        "category" => "분류",
        _ => "링크",
    }
}

#[derive(Deserialize)]
pub struct SuggestQuery {
    #[serde(default)]
    q: String,
}

#[derive(Serialize)]
pub struct Suggestion {
    title: String,
}

/// 검색창 자동완성 — 제목 앞부분이 맞는 문서를 돌려준다.
pub async fn suggest_titles(
    State(state): State<AppState>,
    Query(parameters): Query<SuggestQuery>,
) -> Result<Json<Vec<Suggestion>>, ServerError> {
    let prefix = parameters.q.trim();
    if prefix.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let titles = wiki_document::titles_starting_with(&state.pool, prefix, 10).await?;
    Ok(Json(
        titles
            .into_iter()
            .map(|title| Suggestion {
                title: title.to_string(),
            })
            .collect(),
    ))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPayload {
    title: String,
    namespace: String,
    source: String,
    /// 렌더 트리. 프론트엔드가 이걸로 본문을 그린다.
    tree: RenderTree,
    revision: Option<RevisionSummary>,
    backlink_count: usize,
    thread_count: usize,
    /// 요청자가 이 문서를 구독하고 있는가. 비로그인은 언제나 false.
    starred: bool,
    /// 리다이렉트 문서면 대상 제목. 이동은 화면을 그리는 쪽이 한다.
    #[serde(skip_serializing_if = "Option::is_none")]
    redirect: Option<String>,
}

/// 읽기 API. the seed에는 공개 API가 없지만 우리는 열어 둔다 (docs/architecture.md).
///
/// 프론트엔드의 문서 보기가 이 응답 하나로 화면을 그리므로, 본문과 함께 셸이 쓰는
/// 정보(리비전·역링크·토론 수)까지 싣는다 (docs/architecture.md).
pub async fn document_api(
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
        return Ok(forbidden());
    }

    let Some(source) = wiki_document::read_source(&state.pool, &title).await? else {
        return Ok(not_found());
    };

    let rendered = wiki_document::render_document(&state.pool, &title, &source).await?;

    let redirect = rendered.redirect().map(str::to_string);

    let revision = wiki_document::latest_revision(&state.pool, &title)
        .await?
        .as_ref()
        .map(RevisionSummary::from);

    Ok(Json(DocumentPayload {
        title: title.to_string(),
        namespace: title.namespace.to_string(),
        source,
        redirect,
        tree: rendered.tree,
        revision,
        backlink_count: wiki_document::backlinks(&state.pool, &title).await?.len(),
        thread_count: wiki_discussion::threads_of(&state.pool, &title)
            .await?
            .len(),
        starred: match &requester.user {
            Some(user) => {
                wiki_document::is_starred(&state.pool, user.identifier.as_raw(), &title).await?
            }
            None => false,
        },
    })
    .into_response())
}

/// 역링크 한 줄. 코드값(`kind`)과 사람이 읽는 이름(`kindLabel`)을 함께 낸다 — 라벨만
/// 내보내면 화면이 종류에 따라 갈라 다루지 못한다.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkEntry {
    title: String,
    kind: String,
    kind_label: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkPayload {
    title: String,
    entries: Vec<BacklinkEntry>,
}

/// 이 문서를 가리키는 문서 목록.
pub async fn backlink_api(
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
        return Ok(forbidden());
    }

    let entries = wiki_document::backlinks(&state.pool, &title)
        .await?
        .into_iter()
        .map(|(source, kind)| BacklinkEntry {
            title: source.to_string(),
            kind_label: kind_label(&kind).to_owned(),
            kind,
        })
        .collect();

    Ok(Json(BacklinkPayload {
        title: title.to_string(),
        entries,
    })
    .into_response())
}

fn title_entries(rows: impl Iterator<Item = (String, String)>) -> TitleListPayload {
    TitleListPayload {
        entries: rows
            .map(|(title, note)| TitleEntry { title, note })
            .collect(),
    }
}

/// 아무도 가리키지 않는 문서.
pub async fn orphaned_pages_api(State(state): State<AppState>) -> Result<Response, ServerError> {
    let titles = wiki_document::orphaned_titles(&state.pool, LIST_LIMIT).await?;
    Ok(Json(title_entries(
        titles
            .iter()
            .map(|title| (title.to_string(), String::new())),
    ))
    .into_response())
}

/// 분류가 없는 문서.
pub async fn uncategorized_pages_api(
    State(state): State<AppState>,
) -> Result<Response, ServerError> {
    let titles = wiki_document::uncategorized_titles(&state.pool, LIST_LIMIT).await?;
    Ok(Json(title_entries(
        titles
            .iter()
            .map(|title| (title.to_string(), String::new())),
    ))
    .into_response())
}

/// 오래 손대지 않은 문서.
pub async fn old_pages_api(State(state): State<AppState>) -> Result<Response, ServerError> {
    let rows = wiki_document::stale_titles(&state.pool, LIST_LIMIT).await?;
    Ok(Json(title_entries(rows.iter().map(|(title, at)| {
        (title.to_string(), at.format("%Y-%m-%d").to_string())
    })))
    .into_response())
}

#[derive(Deserialize)]
pub struct LengthOrderQuery {
    /// `shortest`(기본)와 `longest`만 뜻이 있다. 방향을 경로가 아니라 값으로 받는 이유는
    /// HTML 쪽 `pages_by_length`가 쿼리 키의 유무로 정하다 `/longest-pages`에서 짧은
    /// 목록을 내던 결함 때문이다.
    #[serde(default)]
    order: Option<String>,
}

/// 길이로 줄 세운 문서.
pub async fn pages_by_length_api(
    State(state): State<AppState>,
    Query(parameters): Query<LengthOrderQuery>,
) -> Result<Response, ServerError> {
    let longest_first = parameters.order.as_deref() == Some("longest");
    let rows = wiki_document::titles_by_length(&state.pool, longest_first, LIST_LIMIT).await?;

    Ok(Json(title_entries(rows.iter().map(|(title, bytes)| {
        (title.to_string(), format!("{bytes}바이트"))
    })))
    .into_response())
}

/// 내가 구독한 문서.
pub async fn starred_documents_api(
    State(state): State<AppState>,
    requester: Requester,
) -> Result<Response, ServerError> {
    let Some(user) = &requester.user else {
        return Ok(unauthorized());
    };

    let titles = wiki_document::starred_titles(&state.pool, user.identifier.as_raw()).await?;
    Ok(Json(title_entries(
        titles
            .iter()
            .map(|title| (title.to_string(), String::new())),
    ))
    .into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StarPayload {
    starred: bool,
}

/// 구독을 켜고 끈다. 토글한 뒤의 상태를 낸다.
pub async fn toggle_star_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Path(raw_title): Path<String>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(forbidden());
    }
    let Some(user) = &requester.user else {
        return Ok(unauthorized());
    };

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);
    let starred = wiki_document::toggle_star(&state.pool, user.identifier.as_raw(), &title).await?;

    Ok(Json(StarPayload { starred }).into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEntry {
    kind: &'static str,
    kind_label: &'static str,
    document: String,
    read: bool,
    created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPayload {
    items: Vec<NotificationEntry>,
}

fn notification_label(kind: wiki_account::NotificationKind) -> &'static str {
    match kind {
        wiki_account::NotificationKind::ThreadComment => "새 토론 발언",
        wiki_account::NotificationKind::EditRequestReviewed => "편집요청이 처리됨",
    }
}

/// 알림함. 읽기만 한다 — 읽음 표시는 `notifications_read_api`가 따로 맡는다.
pub async fn notifications_api(
    State(state): State<AppState>,
    requester: Requester,
) -> Result<Response, ServerError> {
    let Some(user) = &requester.user else {
        return Ok(unauthorized());
    };

    let items = wiki_account::notifications(&state.pool, user.identifier, 100)
        .await?
        .into_iter()
        .map(|item| NotificationEntry {
            kind: item.kind.as_str(),
            kind_label: notification_label(item.kind),
            document: item
                .payload
                .get("document")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned(),
            read: item.read,
            created_at: item.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(NotificationPayload { items }).into_response())
}

/// 알림을 모두 읽음으로 표시한다.
pub async fn notifications_read_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(forbidden());
    }
    let Some(user) = &requester.user else {
        return Ok(unauthorized());
    };

    wiki_account::mark_all_read(&state.pool, user.identifier).await?;
    Ok(Json(serde_json::json!({ "read": true })).into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryPayload {
    members: Vec<TitleEntry>,
}

/// 분류에 속한 문서 목록.
pub async fn category_api(
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
        return Ok(forbidden());
    }

    let members = category_members(&state, &title)
        .await?
        .into_iter()
        .map(|member| TitleEntry {
            title: member.to_string(),
            note: String::new(),
        })
        .collect();

    Ok(Json(CategoryPayload { members }).into_response())
}

/// 분류 문서를 볼 때 그 분류에 속한 문서들을 함께 보인다.
pub async fn category_members(
    state: &AppState,
    title: &DocumentTitle,
) -> Result<Vec<DocumentTitle>, ServerError> {
    if title.namespace.as_str() != Namespace::CATEGORY {
        return Ok(Vec::new());
    }

    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT namespace.name, document.title
         FROM document_reference
         JOIN document ON document.id = document_reference.source_document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN document_reference_kind ON document_reference_kind.id = document_reference.kind_id
         JOIN namespace target ON target.id = document_reference.target_namespace_id
         WHERE document_reference_kind.name = 'category'
           AND target.name = '분류'
           AND document_reference.target_title = $1
         ORDER BY namespace.name, document.title",
    )
    .bind(&title.name)
    .fetch_all(&state.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name)| DocumentTitle::new(Namespace::new(namespace), name))
        .collect())
}
