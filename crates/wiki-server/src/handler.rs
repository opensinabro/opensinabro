use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use wiki_document::{DocumentTitle, Namespace};

use crate::ServerError;
use crate::api::{TitleEntry, TitleListPayload, forbidden, not_found};
use crate::session::Requester;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    q: String,
}

/// 링크는 있지만 아직 없는 문서.
pub async fn needed_pages_api(State(state): State<AppState>) -> Result<Response, ServerError> {
    let missing = wiki_document::titles_missing(&state.pool, 200).await?;

    Ok(Json(TitleListPayload {
        entries: missing
            .into_iter()
            .map(|(title, count)| TitleEntry {
                title: title.to_string(),
                note: format!("{count}회 링크됨"),
            })
            .collect(),
    })
    .into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPayload {
    query: String,
    /// 제목이 정확히 맞는 문서가 있으면 그 제목. 이동은 화면을 그리는 쪽이 한다.
    redirect: Option<String>,
    results: Vec<TitleEntry>,
}

/// 검색.
pub async fn search_api(
    State(state): State<AppState>,
    Query(parameters): Query<SearchQuery>,
) -> Result<Response, ServerError> {
    let query = parameters.q.trim().to_owned();
    if query.is_empty() {
        return Ok(Json(SearchPayload {
            query,
            redirect: None,
            results: Vec::new(),
        })
        .into_response());
    }

    let namespaces = namespace_names(&state).await?;
    let exact = DocumentTitle::parse(&query, &namespaces);
    if wiki_document::find_document(&state.pool, &exact)
        .await?
        .is_some()
        && wiki_document::read_source(&state.pool, &exact)
            .await?
            .is_some()
    {
        return Ok(Json(SearchPayload {
            query,
            redirect: Some(exact.to_string()),
            results: Vec::new(),
        })
        .into_response());
    }

    let results = state
        .search
        .search(&query, 50)?
        .into_iter()
        .map(|hit| TitleEntry {
            title: DocumentTitle::new(Namespace::new(hit.namespace), hit.title).to_string(),
            note: String::new(),
        })
        .collect();

    Ok(Json(SearchPayload {
        query,
        redirect: None,
        results,
    })
    .into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RandomPayload {
    title: Option<String>,
}

pub async fn random_api(State(state): State<AppState>) -> Result<Response, ServerError> {
    let title = wiki_document::random_title(&state.pool)
        .await?
        .map(|title| title.to_string());
    Ok(Json(RandomPayload { title }).into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicensePayload {
    engine_notice: String,
    content_license: String,
}

pub async fn license_api(State(state): State<AppState>) -> Response {
    Json(LicensePayload {
        engine_notice: "엔진 opensinabro는 MIT 라이선스입니다.".to_owned(),
        content_license: format!("문서 내용은 {}를 따릅니다.", state.settings.content_license),
    })
    .into_response()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawPayload {
    title: String,
    content: String,
    /// 특정 리비전을 요청했을 때 그 순번. 화면이 "지금 원문"과 구분해 알린다.
    revision: Option<i64>,
}

/// 원문 보기. HTML 쪽 `raw`와 달리 읽기 권한을 본다 — 권한이 막힌 문서의 전문이
/// 그대로 나가서는 안 된다.
pub async fn raw_api(
    State(state): State<AppState>,
    requester: Requester,
    Path(raw_title): Path<String>,
    Query(parameters): Query<crate::history::RevisionQuery>,
) -> Result<Response, ServerError> {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester
        .may(&state, &title, wiki_authorization::AclAction::Read)
        .await?
    {
        return Ok(forbidden());
    }

    // 리비전 uuid는 문서와 독립된 값이라, 이 문서의 것임을 먼저 확인해야 남의 문서
    // 원문이 이 제목으로 새어 나가지 않는다.
    if let Some(uuid) = parameters.uuid {
        let Some(sequence) = crate::history::revision_sequence_within(&state, &title, uuid).await?
        else {
            return Ok(not_found());
        };

        return match wiki_document::revision_content(&state.pool, uuid).await? {
            Some(content) => Ok(Json(RawPayload {
                title: title.to_string(),
                content: content.unwrap_or_default(),
                revision: Some(sequence),
            })
            .into_response()),
            None => Ok(not_found()),
        };
    }

    match wiki_document::read_source(&state.pool, &title).await? {
        Some(content) => Ok(Json(RawPayload {
            title: title.to_string(),
            content,
            revision: None,
        })
        .into_response()),
        None => Ok(not_found()),
    }
}

/// 렌더러가 동봉한 본문 스타일시트.
pub async fn stylesheet() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        namumark_backend_namuwiki::stylesheet(),
    )
}

pub(crate) async fn namespace_names(state: &AppState) -> Result<Vec<String>, ServerError> {
    let rows = sqlx::query_as::<_, (String,)>("SELECT name FROM namespace ORDER BY id")
        .fetch_all(&state.pool)
        .await?;
    Ok(rows.into_iter().map(|(name,)| name).collect())
}
