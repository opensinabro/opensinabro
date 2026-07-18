use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use uuid::Uuid;
use wiki_authorization::AclAction;
use wiki_document::{DocumentTitle, RevisionKind};

use crate::ServerError;
use crate::handler::{escape, namespace_names, shell};
use crate::security::{issue_token, verify_token};
use crate::session::Requester;
use crate::state::AppState;

type HandlerResult = Result<Response, ServerError>;

/// 삭제·이동 사유는 짧게 적어 넘길 수 없게 한다 (the seed도 5자 이상을 요구한다).
const MINIMUM_REASON_LENGTH: usize = 5;

pub async fn move_form(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
) -> HandlerResult {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester.may(&state, &title, AclAction::Move).await? {
        return denied(&state, &requester, "이 문서를 옮길 권한이 없습니다.").await;
    }

    let (jar, csrf_token) = issue_token(jar);
    let body = format!(
        "<form method=\"post\" action=\"/move/{title}\">\
         <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
         <label>새 제목 <input type=\"text\" name=\"target\" value=\"{title}\" required></label>\
         <label>사유 <input type=\"text\" name=\"comment\" required minlength=\"5\"></label>\
         <p>옮길 자리에 역사가 있는 문서가 있으면 서로 맞바꿉니다.</p>\
         <button type=\"submit\">옮기기</button>\
         </form>",
        title = escape(&title.to_string()),
        csrf_token = escape(&csrf_token),
    );

    let page = shell(
        &state,
        &requester,
        format!("{title} (이동)"),
        body,
        &csrf_token,
    )
    .await?
    .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct MoveSubmission {
    csrf_token: String,
    target: String,
    comment: String,
}

pub async fn move_submit(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
    axum::Form(submission): axum::Form<MoveSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let namespaces = namespace_names(&state).await?;
    let from = DocumentTitle::parse(&raw_title, &namespaces);
    let to = DocumentTitle::parse(&submission.target, &namespaces);

    if !requester.may(&state, &from, AclAction::Move).await? {
        return denied(&state, &requester, "이 문서를 옮길 권한이 없습니다.").await;
    }
    if submission.comment.chars().count() < MINIMUM_REASON_LENGTH {
        return denied(&state, &requester, "사유를 다섯 자 이상 적어 주세요.").await;
    }

    let actor = requester.actor(&state).await?;
    wiki_document::move_document(&state.pool, &from, &to, actor, &submission.comment).await?;

    // 제목이 바뀌면 그 문서를 가리키던 링크의 존재 판정이 달라진다.
    wiki_document::invalidate_referrers(&state.pool, &from).await?;
    wiki_document::invalidate_referrers(&state.pool, &to).await?;
    state.search.remove(from.namespace.as_str(), &from.name)?;
    if let Some(source) = wiki_document::read_source(&state.pool, &to).await? {
        state.search.put(to.namespace.as_str(), &to.name, &source)?;
    }
    state.search.commit()?;

    Ok(Redirect::to(&format!("/w/{to}")).into_response())
}

pub async fn delete_form(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
) -> HandlerResult {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester.may(&state, &title, AclAction::Delete).await? {
        return denied(&state, &requester, "이 문서를 지울 권한이 없습니다.").await;
    }

    let (jar, csrf_token) = issue_token(jar);
    let body = format!(
        "<form method=\"post\" action=\"/delete/{title}\">\
         <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
         <label>사유 <input type=\"text\" name=\"comment\" required minlength=\"5\"></label>\
         <p>역사는 남고 문서만 없는 상태가 됩니다. 다시 쓰면 되살아납니다.</p>\
         <button type=\"submit\">삭제</button>\
         </form>",
        title = escape(&title.to_string()),
        csrf_token = escape(&csrf_token),
    );

    let page = shell(
        &state,
        &requester,
        format!("{title} (삭제)"),
        body,
        &csrf_token,
    )
    .await?
    .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct DeleteSubmission {
    csrf_token: String,
    comment: String,
}

pub async fn delete_submit(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
    axum::Form(submission): axum::Form<DeleteSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);

    if !requester.may(&state, &title, AclAction::Delete).await? {
        return denied(&state, &requester, "이 문서를 지울 권한이 없습니다.").await;
    }
    if submission.comment.chars().count() < MINIMUM_REASON_LENGTH {
        return denied(&state, &requester, "사유를 다섯 자 이상 적어 주세요.").await;
    }

    let actor = requester.actor(&state).await?;
    wiki_document::delete_document(&state.pool, &title, actor, &submission.comment).await?;

    wiki_document::invalidate_referrers(&state.pool, &title).await?;
    state.search.remove(title.namespace.as_str(), &title.name)?;
    state.search.commit()?;

    Ok(Redirect::to(&format!("/w/{title}")).into_response())
}

/// 줄마다 마지막으로 손댄 사람을 보인다.
pub async fn blame(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
) -> HandlerResult {
    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);
    let lines = wiki_document::blame(&state.pool, &title).await?;

    let mut body = String::from("<table class=\"wiki-blame\"><tbody>");
    for line in &lines {
        body.push_str(&format!(
            "<tr><td>r{sequence}</td><td>{author}</td><td><code>{text}</code></td></tr>",
            sequence = line.sequence,
            author = escape(&line.author),
            text = escape(&line.text),
        ));
    }
    body.push_str("</tbody></table>");

    let (jar, csrf_token) = issue_token(jar);
    let page = shell(
        &state,
        &requester,
        format!("{title} (blame)"),
        body,
        &csrf_token,
    )
    .await?
    .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct HideSubmission {
    csrf_token: String,
    uuid: Uuid,
    #[serde(default)]
    show: Option<String>,
}

/// 리비전 숨김·해제. 목록에는 남고 내용만 가린다.
pub async fn hide_revision(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(raw_title): Path<String>,
    axum::Form(submission): axum::Form<HideSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }
    if !requester.has_permission(&state, "hide_revision").await? {
        return denied(&state, &requester, "리비전을 숨길 권한이 없습니다.").await;
    }

    wiki_document::set_revision_hidden(&state.pool, submission.uuid, submission.show.is_none())
        .await?;

    let namespaces = namespace_names(&state).await?;
    let title = DocumentTitle::parse(&raw_title, &namespaces);
    Ok(Redirect::to(&format!("/history/{title}")).into_response())
}

#[derive(Deserialize)]
pub struct BatchRevertQuery {
    author: Option<String>,
}

/// 한 사람의 최근 편집을 문서마다 그 사람 직전 상태로 되돌린다.
pub async fn batch_revert_form(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Query(parameters): Query<BatchRevertQuery>,
) -> HandlerResult {
    if !requester.has_permission(&state, "batch_revert").await? {
        return denied(&state, &requester, "일괄 되돌리기 권한이 없습니다.").await;
    }

    let (jar, csrf_token) = issue_token(jar);
    let author = parameters.author.unwrap_or_default();
    let mut body = format!(
        "<form method=\"get\" action=\"/admin/batch-revert\">\
         <label>대상 <input type=\"text\" name=\"author\" value=\"{author}\" \
           placeholder=\"사용자 이름 또는 IP\"></label>\
         <button type=\"submit\">대상 보기</button>\
         </form>",
        author = escape(&author)
    );

    if !author.is_empty() {
        let targets = wiki_document::documents_last_edited_by(&state.pool, &author, 100).await?;
        body.push_str(&format!(
            "<p>{}님이 마지막으로 손댄 문서 {}건입니다.</p><ul>",
            escape(&author),
            targets.len()
        ));
        for title in &targets {
            body.push_str(&format!("<li>{}</li>", escape(&title.to_string())));
        }
        body.push_str("</ul>");

        if !targets.is_empty() {
            body.push_str(&format!(
                "<form method=\"post\" action=\"/admin/batch-revert\">\
                 <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
                 <input type=\"hidden\" name=\"author\" value=\"{author}\">\
                 <button type=\"submit\">모두 되돌리기</button>\
                 </form>",
                csrf_token = escape(&csrf_token),
                author = escape(&author),
            ));
        }
    }

    let page = shell(&state, &requester, "일괄 되돌리기", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct BatchRevertSubmission {
    csrf_token: String,
    author: String,
}

pub async fn batch_revert_submit(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    axum::Form(submission): axum::Form<BatchRevertSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }
    if !requester.has_permission(&state, "batch_revert").await? {
        return denied(&state, &requester, "일괄 되돌리기 권한이 없습니다.").await;
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

    let (jar, csrf_token) = issue_token(jar);
    let body = format!("<p>{reverted}건을 되돌렸습니다.</p>");
    let page = shell(&state, &requester, "일괄 되돌리기", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

/// 위키 전역 설정.
pub async fn config_form(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    if !requester.has_permission(&state, "config").await? {
        return denied(&state, &requester, "설정을 바꿀 권한이 없습니다.").await;
    }

    let (jar, csrf_token) = issue_token(jar);
    let body = format!(
        "<form method=\"post\" action=\"/admin/config\">\
         <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
         <label>위키 이름 <input type=\"text\" name=\"wiki_name\" value=\"{wiki_name}\"></label>\
         <label>대문 문서 <input type=\"text\" name=\"main_document\" value=\"{main}\"></label>\
         <label>문서 라이선스 <input type=\"text\" name=\"content_license\" \
           value=\"{license}\"></label>\
         <button type=\"submit\">저장</button>\
         </form>\
         <p>바뀐 설정은 다시 시작한 뒤 화면에 반영됩니다.</p>",
        csrf_token = escape(&csrf_token),
        wiki_name = escape(&state.settings.wiki_name),
        main = escape(&state.settings.main_document),
        license = escape(&state.settings.content_license),
    );

    let page = shell(&state, &requester, "위키 설정", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct ConfigSubmission {
    csrf_token: String,
    wiki_name: String,
    main_document: String,
    content_license: String,
}

pub async fn config_submit(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    axum::Form(submission): axum::Form<ConfigSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }
    if !requester.has_permission(&state, "config").await? {
        return denied(&state, &requester, "설정을 바꿀 권한이 없습니다.").await;
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

    Ok(Redirect::to("/admin/config").into_response())
}

async fn denied(state: &AppState, requester: &Requester, message: &str) -> HandlerResult {
    let body = format!("<p>{}</p>", escape(message));
    let page = shell(state, requester, "권한 없음", body, "")
        .await?
        .render()?;
    Ok((StatusCode::FORBIDDEN, Html(page)).into_response())
}
