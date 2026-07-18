use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;
use wiki_document::{DocumentTitle, Namespace};

use crate::ServerError;
use crate::security::issue_token;
use crate::session::Requester;
use crate::state::AppState;
use crate::template::Shell;

type HandlerResult = Result<Response, ServerError>;

pub async fn index(State(state): State<AppState>) -> HandlerResult {
    let main = state.settings.main_document.clone();
    Ok(Redirect::to(&format!("/w/{main}")).into_response())
}

/// 문서 보기. 없는 문서는 404 + 안내(위키 관례), 리다이렉트 문서는 302다.
pub async fn view(
    State(state): State<AppState>,
    requester: Requester,
    jar: axum_extra::extract::CookieJar,
    Path(raw_title): Path<String>,
) -> HandlerResult {
    let (jar, csrf_token) = issue_token(jar);
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    let Some(source) = wiki_document::read_source(&state.pool, &title).await? else {
        return render_missing(&state, &requester, &title, &csrf_token).await;
    };

    // 렌더는 참조 상태에 따라 여러 회차를 돌 수 있어 보기 요청마다 되풀이하지 않는다.
    if let Some(html) = wiki_document::cached_render(&state.pool, &title).await? {
        let body = with_document_tools(&state, &requester, &title, html, &csrf_token).await?;
        let page = shell(&state, &requester, title.to_string(), body, &csrf_token)
            .await?
            .render()?;
        return Ok((jar, Html(page)).into_response());
    }

    let rendered = wiki_document::render_document(&state.pool, &title, &source).await?;

    if let Some(target) = rendered.redirect {
        return Ok(Redirect::to(&format!("/w/{target}?from={title}")).into_response());
    }

    wiki_document::store_render(&state.pool, &title, &rendered.html).await?;

    let body = with_document_tools(&state, &requester, &title, rendered.html, &csrf_token).await?;
    let page = shell(&state, &requester, title.to_string(), body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

/// 본문 아래에 붙는 문서 도구 — 편집·역사·토론 링크, 구독 단추, 분류 목록.
async fn with_document_tools(
    state: &AppState,
    requester: &Requester,
    title: &DocumentTitle,
    html: String,
    csrf_token: &str,
) -> Result<String, ServerError> {
    let label = escape(&title.to_string());
    let mut body = html;

    body.push_str(&format!(
        "<nav class=\"wiki-document-tools\">\
         <a href=\"/edit/{label}\">편집</a> · <a href=\"/history/{label}\">역사</a> · \
         <a href=\"/discuss/{label}\">토론</a> · <a href=\"/backlink/{label}\">역링크</a>"
    ));

    if let Some(user) = &requester.user {
        let starred =
            wiki_document::is_starred(&state.pool, user.identifier.as_raw(), title).await?;
        body.push_str(&format!(
            " · <form method=\"post\" action=\"/star/{label}\" class=\"wiki-star\">\
             <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf}\">\
             <button type=\"submit\">{text}</button></form>",
            csrf = escape(csrf_token),
            text = if starred { "구독 해제" } else { "구독" },
        ));
    }
    body.push_str("</nav>");

    let members = crate::browse::category_members(state, title).await?;
    if !members.is_empty() {
        body.push_str("<h2>이 분류에 속한 문서</h2><ul class=\"wiki-category-members\">");
        for member in &members {
            let name = escape(&member.to_string());
            body.push_str(&format!("<li><a href=\"/w/{name}\">{name}</a></li>"));
        }
        body.push_str("</ul>");
    }

    Ok(body)
}

/// 원문 보기. 위키 원문은 마크업이 아니라 평문으로 낸다.
pub async fn raw(State(state): State<AppState>, Path(raw_title): Path<String>) -> HandlerResult {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    match wiki_document::read_source(&state.pool, &title).await? {
        Some(source) => Ok((
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            source,
        )
            .into_response()),
        None => Ok((StatusCode::NOT_FOUND, "문서가 없습니다.").into_response()),
    }
}

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    q: String,
}

/// 검색. 제목이 정확히 맞으면 그 문서로 보낸다 (일반 위키 관례 — 별도 `/Go`를 두지 않음).
pub async fn search(
    State(state): State<AppState>,
    requester: Requester,
    jar: axum_extra::extract::CookieJar,
    Query(parameters): Query<SearchQuery>,
) -> HandlerResult {
    let (jar, csrf_token) = issue_token(jar);
    let query = parameters.q.trim().to_owned();
    if query.is_empty() {
        let body = "<p>검색어를 입력하세요.</p>".to_owned();
        let page = shell(&state, &requester, "검색", body, &csrf_token)
            .await?
            .render()?;
        return Ok((jar, Html(page)).into_response());
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
        return Ok(Redirect::to(&format!("/w/{exact}")).into_response());
    }

    let hits = state.search.search(&query, 50)?;
    let mut body = String::new();
    if hits.is_empty() {
        body.push_str("<p>결과가 없습니다.</p>");
    } else {
        body.push_str("<ul class=\"wiki-search-results\">");
        for hit in hits {
            let title = DocumentTitle::new(Namespace::new(hit.namespace), hit.title);
            body.push_str(&format!(
                "<li><a href=\"/w/{title}\">{title}</a></li>",
                title = escape(&title.to_string())
            ));
        }
        body.push_str("</ul>");
    }

    let page = shell(
        &state,
        &requester,
        format!("\"{query}\" 검색 결과"),
        body,
        &csrf_token,
    )
    .await?
    .with_query(&query)
    .render()?;
    Ok((jar, Html(page)).into_response())
}

pub async fn random(
    State(state): State<AppState>,
    requester: Requester,
    jar: axum_extra::extract::CookieJar,
) -> HandlerResult {
    match wiki_document::random_title(&state.pool).await? {
        Some(title) => Ok(Redirect::to(&format!("/w/{title}")).into_response()),
        None => {
            let (jar, csrf_token) = issue_token(jar);
            let body = "<p>문서가 없습니다.</p>".to_owned();
            let page = shell(&state, &requester, "임의 문서", body, &csrf_token)
                .await?
                .render()?;
            Ok((jar, Html(page)).into_response())
        }
    }
}

pub async fn needed_pages(
    State(state): State<AppState>,
    requester: Requester,
    jar: axum_extra::extract::CookieJar,
) -> HandlerResult {
    let missing = wiki_document::titles_missing(&state.pool, 200).await?;

    let mut body = String::from("<p>링크는 있지만 아직 작성되지 않은 문서입니다.</p>");
    if missing.is_empty() {
        body.push_str("<p>없습니다.</p>");
    } else {
        body.push_str("<ol class=\"wiki-needed-pages\">");
        for (title, count) in missing {
            let label = escape(&title.to_string());
            body.push_str(&format!(
                "<li><a href=\"/w/{label}\">{label}</a> <span>({count}회 링크됨)</span></li>"
            ));
        }
        body.push_str("</ol>");
    }

    let (jar, csrf_token) = issue_token(jar);
    let page = shell(&state, &requester, "작성이 필요한 문서", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

pub async fn license(
    State(state): State<AppState>,
    requester: Requester,
    jar: axum_extra::extract::CookieJar,
) -> HandlerResult {
    let body = format!(
        "<p>엔진 <strong>opensinabro</strong>는 MIT 라이선스입니다.</p>\
         <p>문서 내용은 {}를 따릅니다.</p>",
        escape(&state.settings.content_license)
    );
    let (jar, csrf_token) = issue_token(jar);
    let page = shell(&state, &requester, "라이선스", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

/// 렌더러가 동봉한 본문 스타일시트.
pub async fn stylesheet() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        namumark_backend_namuwiki::stylesheet(),
    )
}

async fn render_missing(
    state: &AppState,
    requester: &Requester,
    title: &DocumentTitle,
    csrf_token: &str,
) -> HandlerResult {
    let body = format!(
        "<p>\"{}\" 문서가 아직 없습니다. <a href=\"/edit/{}\">지금 만들기</a></p>",
        escape(&title.to_string()),
        escape(&title.to_string())
    );
    let page = shell(state, requester, title.to_string(), body, csrf_token)
        .await?
        .render()?;
    Ok((StatusCode::NOT_FOUND, Html(page)).into_response())
}

/// 로그인 상태를 실은 셸.
pub(crate) async fn shell(
    state: &AppState,
    requester: &Requester,
    heading: impl Into<String>,
    body: String,
    csrf_token: &str,
) -> Result<Shell, ServerError> {
    let unread = match &requester.user {
        Some(user) => wiki_account::unread_count(&state.pool, user.identifier).await?,
        None => 0,
    };

    Ok(Shell::new(&state.settings, heading, body)
        .with_requester(
            requester.user.as_ref().map(|user| user.name.clone()),
            csrf_token.to_owned(),
        )
        .with_unread(unread))
}

pub(crate) async fn namespace_names(state: &AppState) -> Result<Vec<String>, ServerError> {
    let rows = sqlx::query_as::<_, (String,)>("SELECT name FROM namespace ORDER BY id")
        .fetch_all(&state.pool)
        .await?;
    Ok(rows.into_iter().map(|(name,)| name).collect())
}

pub(crate) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
