use chrono::Utc;
use sqlx::PgPool;

use crate::evaluate::{AclAction, AclCondition, AclRule, RuleScope};

pub type Result<T> = std::result::Result<T, AuthorizationError>;

#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    #[error("권한 저장소 오류")]
    Database(#[from] sqlx::Error),
}

/// 한 문서에 적용될 수 있는 규칙을 전부 읽는다 (문서 규칙 + 그 이름공간 규칙).
///
/// 평가 순서는 [`crate::evaluate`]가 정하므로 여기서는 모으기만 한다.
pub async fn load_rules(
    pool: &PgPool,
    namespace: &str,
    title: &str,
    action: AclAction,
) -> Result<Vec<AclRule>> {
    let rows = sqlx::query_as::<_, (Option<i64>, i64, String, String, bool)>(
        "SELECT acl_rule.document_id, acl_rule.evaluation_order,
                acl_condition_kind.name, acl_rule.condition_value, acl_rule.allowed
         FROM acl_rule
         JOIN acl_action ON acl_action.id = acl_rule.action_id
         JOIN acl_condition_kind ON acl_condition_kind.id = acl_rule.condition_kind_id
         LEFT JOIN document ON document.id = acl_rule.document_id
         LEFT JOIN namespace document_namespace
                ON document_namespace.id = document.namespace_id
         LEFT JOIN namespace rule_namespace ON rule_namespace.id = acl_rule.namespace_id
         WHERE acl_action.name = $1
           AND (
             (document.title = $2 AND document_namespace.name = $3)
             OR rule_namespace.name = $3
           )",
    )
    .bind(action.as_str())
    .bind(title)
    .bind(namespace)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(document_id, order, condition_kind, condition_value, allowed)| AclRule {
                scope: if document_id.is_some() {
                    RuleScope::Document
                } else {
                    RuleScope::Namespace
                },
                action,
                evaluation_order: order,
                condition: parse_condition(&condition_kind, &condition_value),
                allowed,
            },
        )
        .collect())
}

fn parse_condition(kind: &str, value: &str) -> AclCondition {
    match kind {
        "ip" => AclCondition::IpRange(value.to_owned()),
        "aclgroup" => AclCondition::AclGroup(value.to_owned()),
        "perm" => match value {
            "ip" => AclCondition::Ip,
            "member" => AclCondition::Member,
            _ => AclCondition::Any,
        },
        _ => AclCondition::Unsupported,
    }
}

/// 이 사용자가 지금 가진 운영 권한 이름들 (회수된 것은 빼고).
pub async fn granted_permissions(pool: &PgPool, user_id: i64) -> Result<Vec<String>> {
    let rows = sqlx::query_as::<_, (String,)>(
        "SELECT permission.name
         FROM user_permission
         JOIN permission ON permission.id = user_permission.permission_id
         WHERE user_permission.user_id = $1 AND user_permission.revoked_at IS NULL
         ORDER BY permission.name",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}

/// 권한을 부여한다. 이미 가진 권한이면 부분 유니크 인덱스가 중복을 막는다.
pub async fn grant_permission(
    pool: &PgPool,
    user_id: i64,
    permission: &str,
    granted_by: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO user_permission (user_id, permission_id, granted_by, created_at)
         SELECT $1, permission.id, $2, $3
         FROM permission
         WHERE permission.name = $4
         ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(granted_by)
    .bind(Utc::now())
    .bind(permission)
    .execute(pool)
    .await?;

    Ok(())
}

/// 권한을 회수한다. 행을 지우지 않고 끝을 표시해 이력이 남는다.
pub async fn revoke_permission(
    pool: &PgPool,
    user_id: i64,
    permission: &str,
    revoked_by: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE user_permission
         SET revoked_at = $1, revoked_by = $2
         WHERE user_id = $3
           AND revoked_at IS NULL
           AND permission_id = (SELECT id FROM permission WHERE name = $4)",
    )
    .bind(Utc::now())
    .bind(revoked_by)
    .bind(user_id)
    .bind(permission)
    .execute(pool)
    .await?;

    Ok(())
}

/// 권한 부여·회수 기록 한 줄.
#[derive(Debug, Clone)]
pub struct PermissionLogEntry {
    pub user_name: String,
    pub permission: String,
    pub granted_at: chrono::DateTime<Utc>,
    pub revoked_at: Option<chrono::DateTime<Utc>>,
}

/// `/block-history`가 쓰는 권한 변경 이력.
pub async fn permission_log(pool: &PgPool, limit: i64) -> Result<Vec<PermissionLogEntry>> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            chrono::DateTime<Utc>,
            Option<chrono::DateTime<Utc>>,
        ),
    >(
        "SELECT wiki_user.name, permission.name,
                user_permission.created_at, user_permission.revoked_at
         FROM user_permission
         JOIN wiki_user ON wiki_user.id = user_permission.user_id
         JOIN permission ON permission.id = user_permission.permission_id
         ORDER BY COALESCE(user_permission.revoked_at, user_permission.created_at) DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(user_name, permission, granted_at, revoked_at)| PermissionLogEntry {
                user_name,
                permission,
                granted_at,
                revoked_at,
            },
        )
        .collect())
}

/// 이 주체가 지금 속해 있는 차단·경고 그룹 이름들.
///
/// 해제(removed_at)나 만료(expires_at)가 지난 소속은 세지 않는다 — 기록은 남되
/// 효력은 없다는 뜻이다. 로그인 사용자는 계정으로, 비로그인은 IP로 걸린다.
pub async fn active_group_names(
    pool: &PgPool,
    ip_address: &str,
    user_id: Option<i64>,
) -> Result<Vec<String>> {
    let rows = sqlx::query_as::<_, (String,)>(
        "SELECT DISTINCT acl_group.name
         FROM acl_group_member
         JOIN acl_group ON acl_group.id = acl_group_member.group_id
         LEFT JOIN actor ON actor.id = acl_group_member.actor_id
         WHERE acl_group_member.removed_at IS NULL
           AND (acl_group_member.expires_at IS NULL OR acl_group_member.expires_at > $1)
           AND (
             actor.ip_address = $2
             OR ($3::BIGINT IS NOT NULL AND actor.user_id = $3)
           )",
    )
    .bind(Utc::now())
    .bind(ip_address)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}
