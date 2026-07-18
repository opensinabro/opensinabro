use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{AccountError, Result, UserIdentifier};

/// 알림의 종류. DB의 `notification_kind` 열거와 짝이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    ThreadComment,
    EditRequestReviewed,
}

impl NotificationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ThreadComment => "thread_comment",
            Self::EditRequestReviewed => "edit_request_reviewed",
        }
    }

    fn parse(name: &str) -> Self {
        match name {
            "edit_request_reviewed" => Self::EditRequestReviewed,
            _ => Self::ThreadComment,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: i64,
    pub kind: NotificationKind,
    pub payload: serde_json::Value,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

/// 알림을 남긴다.
///
/// payload에는 문서 제목처럼 표시에 쓸 값만 담는다 — 외래키로 문서를 참조하면
/// account가 document를 향하게 되어 의존 그래프가 순환한다 (docs/architecture.md).
pub async fn notify(
    pool: &PgPool,
    user: UserIdentifier,
    kind: NotificationKind,
    payload: serde_json::Value,
) -> Result<()> {
    let (kind_id,) =
        sqlx::query_as::<_, (i64,)>("SELECT id FROM notification_kind WHERE name = $1")
            .bind(kind.as_str())
            .fetch_optional(pool)
            .await?
            .ok_or(AccountError::MissingNotificationKind(kind.as_str()))?;

    sqlx::query(
        "INSERT INTO notification (user_id, kind_id, payload, created_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(user.as_raw())
    .bind(kind_id)
    .bind(payload)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn notifications(
    pool: &PgPool,
    user: UserIdentifier,
    limit: i64,
) -> Result<Vec<Notification>> {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            serde_json::Value,
            Option<DateTime<Utc>>,
            DateTime<Utc>,
        ),
    >(
        "SELECT notification.id, notification_kind.name, notification.payload,
                notification.read_at, notification.created_at
         FROM notification
         JOIN notification_kind ON notification_kind.id = notification.kind_id
         WHERE notification.user_id = $1
         ORDER BY notification.created_at DESC
         LIMIT $2",
    )
    .bind(user.as_raw())
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, kind, payload, read_at, created_at)| Notification {
            id,
            kind: NotificationKind::parse(&kind),
            payload,
            read: read_at.is_some(),
            created_at,
        })
        .collect())
}

pub async fn unread_count(pool: &PgPool, user: UserIdentifier) -> Result<i64> {
    let (count,) = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM notification WHERE user_id = $1 AND read_at IS NULL",
    )
    .bind(user.as_raw())
    .fetch_one(pool)
    .await?;

    Ok(count)
}

pub async fn mark_all_read(pool: &PgPool, user: UserIdentifier) -> Result<()> {
    sqlx::query("UPDATE notification SET read_at = $1 WHERE user_id = $2 AND read_at IS NULL")
        .bind(Utc::now())
        .bind(user.as_raw())
        .execute(pool)
        .await?;

    Ok(())
}
