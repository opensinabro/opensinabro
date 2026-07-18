use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ServerError;
use crate::security::verify_header;
use crate::session::Requester;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantOptionsPayload {
    permissions: Vec<String>,
}

/// 줄 수 있는 권한의 목록. 이 화면 자체가 `grant` 권한을 요구한다.
pub async fn grant_api(
    State(state): State<AppState>,
    requester: Requester,
) -> Result<Response, ServerError> {
    if !requester.is_member() {
        return Ok(crate::api::unauthorized());
    }
    if !requester.has_permission(&state, "grant").await? {
        return Ok(crate::api::forbidden());
    }

    let permissions = sqlx::query_as::<_, (String,)>("SELECT name FROM permission ORDER BY name")
        .fetch_all(&state.pool)
        .await?;

    Ok(axum::Json(GrantOptionsPayload {
        permissions: permissions.into_iter().map(|(name,)| name).collect(),
    })
    .into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantRequestBody {
    user_name: String,
    permission: String,
    revoke: bool,
}

pub async fn grant_submit_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    axum::Json(submission): axum::Json<GrantRequestBody>,
) -> Result<Response, ServerError> {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }
    if !requester.is_member() {
        return Ok(crate::api::unauthorized());
    }
    if !requester.has_permission(&state, "grant").await? {
        return Ok(crate::api::forbidden());
    }

    // 폼 경로는 없는 사용자에게도 403을 내는데, 이는 권한 부족과 대상 없음을 뒤섞는
    // 결함이다 — 여기서는 404로 구분해 화면이 오타를 알려 줄 수 있게 한다.
    let Some(target) = wiki_account::find_user_by_name(&state.pool, &submission.user_name).await?
    else {
        return Ok(crate::api::not_found());
    };

    let actor = requester.actor(&state).await?;
    if submission.revoke {
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

    Ok(axum::Json(serde_json::json!({ "ok": true })).into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockEntry {
    target: String,
    group: String,
    reason: String,
    created_at: String,
    removed_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionEntry {
    user_name: String,
    permission: String,
    granted_at: String,
    revoked_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockHistoryPayload {
    blocks: Vec<BlockEntry>,
    permissions: Vec<PermissionEntry>,
}

/// 차단·권한 변경을 시간순으로 보이는 공개 기록.
pub async fn block_history_api(
    State(state): State<AppState>,
) -> Result<Response, ServerError> {
    let rows = sqlx::query_as::<
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

    let blocks = rows
        .into_iter()
        .map(|(group, user, ip_address, reason, created, removed)| BlockEntry {
            target: user.or(ip_address).unwrap_or_default(),
            group,
            reason,
            created_at: created.to_rfc3339(),
            removed_at: removed.map(|at| at.to_rfc3339()),
        })
        .collect();

    let permissions = wiki_authorization::permission_log(&state.pool, 100)
        .await?
        .into_iter()
        .map(|entry| PermissionEntry {
            user_name: entry.user_name,
            permission: entry.permission,
            granted_at: entry.granted_at.to_rfc3339(),
            revoked_at: entry.revoked_at.map(|at| at.to_rfc3339()),
        })
        .collect();

    Ok(axum::Json(BlockHistoryPayload {
        blocks,
        permissions,
    })
    .into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributionEntry {
    title: String,
    sequence: i64,
    created_at: String,
    comment: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributionsPayload {
    name: String,
    entries: Vec<ContributionEntry>,
}

/// 이 사용자가 남긴 편집들.
pub async fn contributions_api(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Response, ServerError> {
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

    let entries = rows
        .into_iter()
        .map(
            |(namespace, title, sequence, created, comment)| ContributionEntry {
                title: if namespace == "문서" {
                    title
                } else {
                    format!("{namespace}:{title}")
                },
                sequence,
                created_at: created.to_rfc3339(),
                comment,
            },
        )
        .collect();

    Ok(axum::Json(ContributionsPayload { name, entries }).into_response())
}
