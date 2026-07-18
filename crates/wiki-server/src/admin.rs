use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::ServerError;
use crate::handler::{escape, shell};
use crate::security::{issue_token, verify_token};
use crate::session::Requester;
use crate::state::AppState;
use crate::template::Shell;

type HandlerResult = Result<Response, ServerError>;

/// 권한을 주고 거두는 화면. 이 화면 자체가 `grant` 권한을 요구한다.
pub async fn grant_form(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    if !requester.has_permission(&state, "grant").await? {
        return forbidden(&state, "권한을 부여하려면 grant 권한이 필요합니다.");
    }

    let permissions = sqlx::query_as::<_, (String,)>("SELECT name FROM permission ORDER BY name")
        .fetch_all(&state.pool)
        .await?;

    let (jar, csrf_token) = issue_token(jar);
    let options = permissions
        .into_iter()
        .map(|(name,)| format!("<option value=\"{0}\">{0}</option>", escape(&name)))
        .collect::<String>();

    let body = format!(
        "<form method=\"post\" action=\"/admin/grant\">\
         <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
         <label>사용자 <input type=\"text\" name=\"user_name\" required></label>\
         <label>권한 <select name=\"permission\">{options}</select></label>\
         <label><input type=\"checkbox\" name=\"revoke\" value=\"1\"> 회수</label>\
         <button type=\"submit\">적용</button>\
         </form>\
         <p><a href=\"/block-history\">운영 기록 보기</a></p>",
        csrf_token = escape(&csrf_token)
    );

    let page = shell(&state, &requester, "권한 관리", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct GrantSubmission {
    csrf_token: String,
    user_name: String,
    permission: String,
    revoke: Option<String>,
}

pub async fn grant_submit(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    axum::Form(submission): axum::Form<GrantSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }
    if !requester.has_permission(&state, "grant").await? {
        return forbidden(&state, "권한을 부여하려면 grant 권한이 필요합니다.");
    }

    let Some(target) = wiki_account::find_user_by_name(&state.pool, &submission.user_name).await?
    else {
        return forbidden(&state, "그런 사용자가 없습니다.");
    };

    let actor = requester.actor(&state).await?;
    if submission.revoke.is_some() {
        wiki_authorization::revoke_permission(
            &state.pool,
            target.identifier.as_raw(),
            &submission.permission,
            actor.as_raw(),
        )
        .await?;
    } else {
        wiki_authorization::grant_permission(
            &state.pool,
            target.identifier.as_raw(),
            &submission.permission,
            actor.as_raw(),
        )
        .await?;
    }

    Ok(Redirect::to("/block-history").into_response())
}

/// 차단·권한 변경을 시간순으로 보이는 공개 기록.
///
/// 별도 로그 테이블 없이 원본 행(removed_at·revoked_at)이 곧 기록이다 (docs/design/08).
pub async fn block_history(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    let blocks = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            Option<String>,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
        ),
    >(
        "SELECT acl_group.name, wiki_user.name, actor.ip_address,
                acl_group_member.reason, acl_group_member.created_at,
                acl_group_member.removed_at
         FROM acl_group_member
         JOIN acl_group ON acl_group.id = acl_group_member.group_id
         LEFT JOIN actor ON actor.id = acl_group_member.actor_id
         LEFT JOIN wiki_user ON wiki_user.id = actor.user_id
         ORDER BY acl_group_member.created_at DESC
         LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await?;

    let mut body = String::from("<h2>차단 기록</h2><ul class=\"wiki-block-history\">");
    if blocks.is_empty() {
        body.push_str("<li>없습니다.</li>");
    }
    for (group, user, ip, reason, created, removed) in blocks {
        body.push_str(&format!(
            "<li>{target} → {group} · {reason} · {created}{removed}</li>",
            target = escape(&user.or(ip).unwrap_or_default()),
            group = escape(&group),
            reason = escape(&reason),
            created = created.format("%Y-%m-%d %H:%M:%S UTC"),
            removed = match removed {
                Some(at) => format!(" · 해제 {}", at.format("%Y-%m-%d %H:%M:%S UTC")),
                None => String::new(),
            },
        ));
    }
    body.push_str("</ul>");

    body.push_str("<h2>권한 기록</h2><ul class=\"wiki-permission-history\">");
    let permissions = wiki_authorization::permission_log(&state.pool, 100).await?;
    if permissions.is_empty() {
        body.push_str("<li>없습니다.</li>");
    }
    for entry in permissions {
        body.push_str(&format!(
            "<li>{user} · {permission} · 부여 {granted}{revoked}</li>",
            user = escape(&entry.user_name),
            permission = escape(&entry.permission),
            granted = entry.granted_at.format("%Y-%m-%d %H:%M:%S UTC"),
            revoked = match entry.revoked_at {
                Some(at) => format!(" · 회수 {}", at.format("%Y-%m-%d %H:%M:%S UTC")),
                None => String::new(),
            },
        ));
    }
    body.push_str("</ul>");

    let (jar, csrf_token) = issue_token(jar);
    let page = shell(&state, &requester, "운영 기록", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

/// 사용자 문서로 보낸다 — 사용자 정보는 위키 문서로 다룬다.
pub async fn user_profile(Path(name): Path<String>) -> HandlerResult {
    Ok(Redirect::to(&format!("/w/사용자:{name}")).into_response())
}

/// 이 사용자가 남긴 편집들.
pub async fn contributions(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    Path(name): Path<String>,
) -> HandlerResult {
    let rows = sqlx::query_as::<_, (String, String, i64, DateTime<Utc>, String)>(
        "SELECT namespace.name, document.title, revision.sequence,
                revision.created_at, revision.comment
         FROM revision
         JOIN document ON document.id = revision.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN actor ON actor.id = revision.actor_id
         LEFT JOIN wiki_user ON wiki_user.id = actor.user_id
         WHERE wiki_user.name = $1 OR actor.ip_address = $1
         ORDER BY revision.created_at DESC
         LIMIT 200",
    )
    .bind(&name)
    .fetch_all(&state.pool)
    .await?;

    let mut body = String::from("<ul class=\"wiki-contributions\">");
    if rows.is_empty() {
        body.push_str("<li>기여가 없습니다.</li>");
    }
    for (namespace, title, sequence, created, comment) in rows {
        let full = if namespace == "문서" {
            title
        } else {
            format!("{namespace}:{title}")
        };
        body.push_str(&format!(
            "<li><a href=\"/w/{full}\">{full}</a> · r{sequence} · {created}{comment}</li>",
            full = escape(&full),
            created = created.format("%Y-%m-%d %H:%M:%S UTC"),
            comment = if comment.is_empty() {
                String::new()
            } else {
                format!(" · {}", escape(&comment))
            },
        ));
    }
    body.push_str("</ul>");

    let (jar, csrf_token) = issue_token(jar);
    let page = shell(
        &state,
        &requester,
        format!("{name}의 기여"),
        body,
        &csrf_token,
    )
    .await?
    .render()?;
    Ok((jar, Html(page)).into_response())
}

fn forbidden(state: &AppState, message: &str) -> HandlerResult {
    let body = format!("<p>{}</p>", escape(message));
    let page = Shell::new(&state.settings, "권한 없음", body).render()?;
    Ok((StatusCode::FORBIDDEN, Html(page)).into_response())
}
