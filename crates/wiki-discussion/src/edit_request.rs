use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wiki_account::ActorIdentifier;
use wiki_document::{DocumentTitle, Namespace};

use crate::{DiscussionError, Result};

/// 편집요청의 상태. DB의 `edit_request_status` 열거와 짝이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditRequestStatus {
    Open,
    Accepted,
    Closed,
}

impl EditRequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Accepted => "accepted",
            Self::Closed => "closed",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "열림",
            Self::Accepted => "반영됨",
            Self::Closed => "닫힘",
        }
    }

    fn parse(name: &str) -> Self {
        match name {
            "accepted" => Self::Accepted,
            "closed" => Self::Closed,
            _ => Self::Open,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditRequest {
    pub external_id: Uuid,
    pub title: DocumentTitle,
    pub author: String,
    pub content: String,
    pub comment: String,
    pub status: EditRequestStatus,
    pub base_revision: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

async fn status_id(pool: &PgPool, status: EditRequestStatus) -> Result<i64> {
    sqlx::query_as::<_, (i64,)>("SELECT id FROM edit_request_status WHERE name = $1")
        .bind(status.as_str())
        .fetch_optional(pool)
        .await?
        .map(|(id,)| id)
        .ok_or_else(|| DiscussionError::MissingEnumeration {
            table: "edit_request_status",
            name: status.as_str().to_owned(),
        })
}

/// 편집 권한이 없는 사람이 변경안을 낸다.
pub async fn submit_edit_request(
    pool: &PgPool,
    title: &DocumentTitle,
    actor: ActorIdentifier,
    content: &str,
    comment: &str,
    base_revision: Option<Uuid>,
) -> Result<Uuid> {
    let Some(document) = wiki_document::find_document(pool, title).await? else {
        return Err(DiscussionError::MissingDocument(title.to_string()));
    };

    let external_id = Uuid::new_v4();
    let open = status_id(pool, EditRequestStatus::Open).await?;

    sqlx::query(
        "INSERT INTO edit_request
           (external_id, document_id, base_revision_id, actor_id,
            content, comment, status_id, created_at)
         VALUES ($1, $2, (SELECT id FROM revision WHERE external_id = $3), $4, $5, $6, $7, $8)",
    )
    .bind(external_id)
    .bind(document.identifier.as_raw())
    .bind(base_revision)
    .bind(actor.as_raw())
    .bind(content)
    .bind(comment)
    .bind(open)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(external_id)
}

pub async fn edit_request_by_id(pool: &PgPool, external_id: Uuid) -> Result<Option<EditRequest>> {
    let row = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
            String,
            Option<Uuid>,
            DateTime<Utc>,
        ),
    >(
        "SELECT namespace.name, document.title, wiki_user.name, actor.ip_address,
                edit_request.content, edit_request.comment, edit_request_status.name,
                base.external_id, edit_request.created_at
         FROM edit_request
         JOIN document ON document.id = edit_request.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN actor ON actor.id = edit_request.actor_id
         JOIN edit_request_status ON edit_request_status.id = edit_request.status_id
         LEFT JOIN wiki_user ON wiki_user.id = actor.user_id
         LEFT JOIN revision base ON base.id = edit_request.base_revision_id
         WHERE edit_request.external_id = $1",
    )
    .bind(external_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(
        |(namespace, title, user, ip, content, comment, status, base, created_at)| EditRequest {
            external_id,
            title: DocumentTitle::new(Namespace::new(namespace), title),
            author: user.or(ip).unwrap_or_default(),
            content,
            comment,
            status: EditRequestStatus::parse(&status),
            base_revision: base,
            created_at,
        },
    ))
}

/// 아직 처리되지 않은 편집요청들.
pub async fn open_edit_requests(pool: &PgPool, limit: i64) -> Result<Vec<EditRequest>> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            DateTime<Utc>,
        ),
    >(
        "SELECT edit_request.external_id, namespace.name, document.title,
                wiki_user.name, actor.ip_address, edit_request.comment, edit_request.created_at
         FROM edit_request
         JOIN document ON document.id = edit_request.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN actor ON actor.id = edit_request.actor_id
         JOIN edit_request_status ON edit_request_status.id = edit_request.status_id
         LEFT JOIN wiki_user ON wiki_user.id = actor.user_id
         WHERE edit_request_status.name = 'open'
         ORDER BY edit_request.created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(external_id, namespace, title, user, ip, comment, created_at)| EditRequest {
                external_id,
                title: DocumentTitle::new(Namespace::new(namespace), title),
                author: user.or(ip).unwrap_or_default(),
                content: String::new(),
                comment,
                status: EditRequestStatus::Open,
                base_revision: None,
                created_at,
            },
        )
        .collect())
}

/// 요청을 받아들인 것으로 표시한다. 실제 문서 반영은 호출자(서버)가 편집 경로로 한다
/// — 리비전 기록과 그에 딸린 갱신들이 한곳에 모여 있기 때문이다.
pub async fn accept_edit_request(
    pool: &PgPool,
    external_id: Uuid,
    reviewer: ActorIdentifier,
) -> Result<bool> {
    mark(pool, external_id, EditRequestStatus::Accepted, reviewer).await
}

pub async fn close_edit_request(
    pool: &PgPool,
    external_id: Uuid,
    reviewer: ActorIdentifier,
) -> Result<bool> {
    mark(pool, external_id, EditRequestStatus::Closed, reviewer).await
}

async fn mark(
    pool: &PgPool,
    external_id: Uuid,
    status: EditRequestStatus,
    reviewer: ActorIdentifier,
) -> Result<bool> {
    let target = status_id(pool, status).await?;
    let result = sqlx::query(
        "UPDATE edit_request
         SET status_id = $1, reviewed_by = $2
         WHERE external_id = $3
           AND status_id = (SELECT id FROM edit_request_status WHERE name = 'open')",
    )
    .bind(target)
    .bind(reviewer.as_raw())
    .bind(external_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
