use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json, Redirect, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use wiki_document::{DocumentTitle, Namespace};

use crate::ServerError;
use crate::handler::{escape, namespace_names, shell};
use crate::security::{issue_token, verify_token};
use crate::session::Requester;
use crate::state::AppState;

type HandlerResult = Result<Response, ServerError>;

const LIST_LIMIT: i64 = 200;

/// 아무도 가리키지 않는 문서.
pub async fn orphaned_pages(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    let titles = wiki_document::orphaned_titles(&state.pool, LIST_LIMIT).await?;
    let body = title_list(
        "어느 문서에서도 링크되지 않은 문서입니다.",
        titles
            .iter()
            .map(|title| (title.to_string(), String::new())),
    );
    render_list(&state, &requester, jar, "고립된 문서", body).await
}

/// 분류가 없는 문서.
pub async fn uncategorized_pages(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    let titles = wiki_document::uncategorized_titles(&state.pool, LIST_LIMIT).await?;
    let body = title_list(
        "분류가 붙지 않은 문서입니다.",
        titles
            .iter()
            .map(|title| (title.to_string(), String::new())),
    );
    render_list(&state, &requester, jar, "분류가 없는 문서", body).await
}

/// 오래 손대지 않은 문서.
pub async fn old_pages(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    let rows = wiki_document::stale_titles(&state.pool, LIST_LIMIT).await?;
    let body = title_list(
        "가장 오래 손대지 않은 문서입니다.",
        rows.iter()
            .map(|(title, at)| (title.to_string(), at.format("%Y-%m-%d").to_string())),
    );
    render_list(&state, &requester, jar, "편집된 지 오래된 문서", body).await
}

#[derive(Deserialize)]
pub struct LengthQuery {
    #[serde(default)]
    longest: Option<String>,
}

/// 길이로 줄 세운 문서.
pub async fn pages_by_length(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Query(parameters): Query<LengthQuery>,
) -> HandlerResult {
    let longest_first = parameters.longest.is_some();
    let rows = wiki_document::titles_by_length(&state.pool, longest_first, LIST_LIMIT).await?;

    let heading = if longest_first {
        "내용이 긴 문서"
    } else {
        "내용이 짧은 문서"
    };
    let body = title_list(
        "현재 판의 바이트 수로 줄 세웁니다.",
        rows.iter()
            .map(|(title, bytes)| (title.to_string(), format!("{bytes}바이트"))),
    );
    render_list(&state, &requester, jar, heading, body).await
}

/// 이 문서를 가리키는 문서들.
pub async fn backlinks(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
) -> HandlerResult {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);
    let rows = wiki_document::backlinks(&state.pool, &title).await?;

    let body = title_list(
        "이 문서를 링크하거나 포함하는 문서입니다.",
        rows.iter()
            .map(|(source, kind)| (source.to_string(), kind_label(kind).to_owned())),
    );
    render_list(&state, &requester, jar, &format!("{title} (역링크)"), body).await
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "include" => "포함",
        "redirect" => "넘겨주기",
        "image" => "이미지",
        "category" => "분류",
        _ => "링크",
    }
}

/// 내가 구독한 문서.
pub async fn starred_documents(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    let Some(user) = &requester.user else {
        return Ok(Redirect::to("/login").into_response());
    };

    let titles = wiki_document::starred_titles(&state.pool, user.identifier.as_raw()).await?;
    let body = title_list(
        "별을 붙여 둔 문서입니다. 이 문서가 바뀌면 알림이 옵니다.",
        titles
            .iter()
            .map(|title| (title.to_string(), String::new())),
    );
    render_list(&state, &requester, jar, "내 문서함", body).await
}

#[derive(Deserialize)]
pub struct StarSubmission {
    csrf_token: String,
}

/// 구독을 켜고 끈다.
pub async fn toggle_star(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
    axum::Form(submission): axum::Form<StarSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }
    let Some(user) = &requester.user else {
        return Ok(Redirect::to("/login").into_response());
    };

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);
    wiki_document::toggle_star(&state.pool, user.identifier.as_raw(), &title).await?;

    Ok(Redirect::to(&format!("/w/{title}")).into_response())
}

/// 알림함.
pub async fn notifications(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    let Some(user) = &requester.user else {
        return Ok(Redirect::to("/login").into_response());
    };

    let items = wiki_account::notifications(&state.pool, user.identifier, 100).await?;
    let mut body = String::from("<ul class=\"wiki-notifications\">");
    if items.is_empty() {
        body.push_str("<li>알림이 없습니다.</li>");
    }
    for item in &items {
        let document = item
            .payload
            .get("document")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let text = match item.kind {
            wiki_account::NotificationKind::ThreadComment => "새 토론 발언",
            wiki_account::NotificationKind::EditRequestReviewed => "편집요청이 처리됨",
        };
        body.push_str(&format!(
            "<li>{unread}{text} · <a href=\"/w/{document}\">{document}</a> · {created}</li>",
            unread = if item.read { "" } else { "● " },
            document = escape(document),
            created = item.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
        ));
    }
    body.push_str("</ul>");

    // 화면을 열면 읽은 것으로 본다.
    wiki_account::mark_all_read(&state.pool, user.identifier).await?;

    let (jar, csrf_token) = issue_token(jar);
    let page = shell(&state, &requester, "알림", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
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
pub struct DocumentPayload {
    title: String,
    namespace: String,
    source: String,
    html: String,
}

/// 읽기 API. the seed에는 공개 API가 없지만 우리는 열어 둔다 (docs/design/06).
pub async fn document_api(
    State(state): State<AppState>,
    Path(raw_title): Path<String>,
) -> Result<Response, ServerError> {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    let Some(source) = wiki_document::read_source(&state.pool, &title).await? else {
        return Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "not_found" })),
        )
            .into_response());
    };

    let html = match wiki_document::cached_render(&state.pool, &title).await? {
        Some(cached) => cached,
        None => {
            wiki_document::render_document(&state.pool, &title, &source)
                .await?
                .html
        }
    };

    Ok(Json(DocumentPayload {
        title: title.name.clone(),
        namespace: title.namespace.to_string(),
        source,
        html,
    })
    .into_response())
}

fn title_list(caption: &str, rows: impl Iterator<Item = (String, String)>) -> String {
    let mut body = format!("<p>{}</p><ol class=\"wiki-title-list\">", escape(caption));
    let mut empty = true;
    for (title, note) in rows {
        empty = false;
        let label = escape(&title);
        body.push_str(&format!(
            "<li><a href=\"/w/{label}\">{label}</a>{note}</li>",
            note = if note.is_empty() {
                String::new()
            } else {
                format!(" <span>({})</span>", escape(&note))
            },
        ));
    }
    if empty {
        body.push_str("<li>없습니다.</li>");
    }
    body.push_str("</ol>");
    body
}

async fn render_list(
    state: &AppState,
    requester: &Requester,
    jar: CookieJar,
    heading: &str,
    body: String,
) -> HandlerResult {
    let (jar, csrf_token) = issue_token(jar);
    let page = shell(state, requester, heading, body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
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
