use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wiki_account::ActorIdentifier;
use wiki_document::{DocumentTitle, Namespace};

use crate::{DiscussionError, Result};

/// 스레드가 지금 어떤 상태인가. DB의 `thread_status` 열거와 짝이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadStatus {
    Normal,
    Pause,
    Close,
}

impl ThreadStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Pause => "pause",
            Self::Close => "close",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "정상",
            Self::Pause => "중단",
            Self::Close => "닫힘",
        }
    }

    pub fn parse(name: &str) -> Self {
        match name {
            "pause" => Self::Pause,
            "close" => Self::Close,
            _ => Self::Normal,
        }
    }

    /// 새 댓글을 받을 수 있는 상태인가.
    pub fn accepts_comments(self) -> bool {
        matches!(self, Self::Normal)
    }
}

/// 스레드 안 항목의 종류. 발언과 관리 조작이 한 타임라인에 섞인다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    Comment,
    StatusChange,
    TopicChange,
    DocumentMove,
}

impl CommentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Comment => "comment",
            Self::StatusChange => "status_change",
            Self::TopicChange => "topic_change",
            Self::DocumentMove => "document_move",
        }
    }

    fn parse(name: &str) -> Self {
        match name {
            "status_change" => Self::StatusChange,
            "topic_change" => Self::TopicChange,
            "document_move" => Self::DocumentMove,
            _ => Self::Comment,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Thread {
    pub external_id: Uuid,
    pub title: DocumentTitle,
    pub topic: String,
    pub status: ThreadStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub sequence: i64,
    pub kind: CommentKind,
    pub author: String,
    pub content: String,
    /// 관리 조작이 바꾼 값. 내용 문자열을 파싱하지 않으려고 따로 둔다.
    pub metadata: Option<serde_json::Value>,
    pub admin_marked: bool,
    pub hidden: bool,
    pub created_at: DateTime<Utc>,
}

async fn enumeration_id(pool: &PgPool, table: &'static str, name: &str) -> Result<i64> {
    // 테이블 이름은 호출부의 `&'static str` 리터럴이라 사용자 입력이 섞이지 않는다.
    let query = format!("SELECT id FROM {table} WHERE name = $1");
    sqlx::query_as::<_, (i64,)>(sqlx::AssertSqlSafe(query))
        .bind(name)
        .fetch_optional(pool)
        .await?
        .map(|(id,)| id)
        .ok_or_else(|| DiscussionError::MissingEnumeration {
            table,
            name: name.to_owned(),
        })
}

/// 문서에 새 토론을 연다. 첫 발언이 곧 1번 항목이다.
pub async fn create_thread(
    pool: &PgPool,
    title: &DocumentTitle,
    topic: &str,
    actor: ActorIdentifier,
    first_comment: &str,
) -> Result<Uuid> {
    let Some(document) = wiki_document::find_document(pool, title).await? else {
        return Err(DiscussionError::MissingDocument(title.to_string()));
    };

    let status_id = enumeration_id(pool, "thread_status", ThreadStatus::Normal.as_str()).await?;
    let external_id = Uuid::new_v4();

    let (thread_id,) = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO thread (external_id, document_id, topic, status_id, created_at)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id",
    )
    .bind(external_id)
    .bind(document.identifier.as_raw())
    .bind(topic)
    .bind(status_id)
    .bind(Utc::now())
    .fetch_one(pool)
    .await?;

    insert_comment(
        pool,
        thread_id,
        CommentKind::Comment,
        actor,
        first_comment,
        None,
        false,
    )
    .await?;

    Ok(external_id)
}

async fn thread_row(pool: &PgPool, external_id: Uuid) -> Result<Option<(i64, Thread)>> {
    let row = sqlx::query_as::<_, (i64, String, String, String, String, DateTime<Utc>)>(
        "SELECT thread.id, namespace.name, document.title,
                thread.topic, thread_status.name, thread.created_at
         FROM thread
         JOIN document ON document.id = thread.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN thread_status ON thread_status.id = thread.status_id
         WHERE thread.external_id = $1",
    )
    .bind(external_id)
    .fetch_optional(pool)
    .await?;

    Ok(
        row.map(|(id, namespace, title, topic, status, created_at)| {
            (
                id,
                Thread {
                    external_id,
                    title: DocumentTitle::new(Namespace::new(namespace), title),
                    topic,
                    status: ThreadStatus::parse(&status),
                    created_at,
                },
            )
        }),
    )
}

pub async fn thread_by_id(pool: &PgPool, external_id: Uuid) -> Result<Option<Thread>> {
    Ok(thread_row(pool, external_id)
        .await?
        .map(|(_, thread)| thread))
}

/// 한 문서에 달린 토론들 (최근 활동 순).
pub async fn threads_of(pool: &PgPool, title: &DocumentTitle) -> Result<Vec<Thread>> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, DateTime<Utc>)>(
        "SELECT thread.external_id, thread.topic, thread_status.name, thread.created_at
         FROM thread
         JOIN document ON document.id = thread.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN thread_status ON thread_status.id = thread.status_id
         WHERE namespace.name = $1 AND document.title = $2
         ORDER BY thread.created_at DESC",
    )
    .bind(title.namespace.as_str())
    .bind(&title.name)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(external_id, topic, status, created_at)| Thread {
            external_id,
            title: title.clone(),
            topic,
            status: ThreadStatus::parse(&status),
            created_at,
        })
        .collect())
}

/// 위키 전체의 최근 토론 (`/recent-discussions`).
pub async fn recent_threads(
    pool: &PgPool,
    status: Option<ThreadStatus>,
    limit: i64,
) -> Result<Vec<Thread>> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, String, String, DateTime<Utc>)>(
        "SELECT thread.external_id, namespace.name, document.title,
                thread.topic, thread_status.name, thread.created_at
         FROM thread
         JOIN document ON document.id = thread.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN thread_status ON thread_status.id = thread.status_id
         WHERE $1::TEXT IS NULL OR thread_status.name = $1
         ORDER BY thread.created_at DESC
         LIMIT $2",
    )
    .bind(status.map(|value| value.as_str()))
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(external_id, namespace, title, topic, status, created_at)| Thread {
                external_id,
                title: DocumentTitle::new(Namespace::new(namespace), title),
                topic,
                status: ThreadStatus::parse(&status),
                created_at,
            },
        )
        .collect())
}

pub async fn thread_comments(pool: &PgPool, external_id: Uuid) -> Result<Vec<Comment>> {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            Option<String>,
            Option<String>,
            String,
            Option<serde_json::Value>,
            bool,
            Option<DateTime<Utc>>,
            DateTime<Utc>,
        ),
    >(
        "SELECT thread_comment.sequence, thread_comment_kind.name,
                wiki_user.name, actor.ip_address,
                thread_comment.content, thread_comment.metadata,
                thread_comment.admin_marked, thread_comment.hidden_at,
                thread_comment.created_at
         FROM thread_comment
         JOIN thread ON thread.id = thread_comment.thread_id
         JOIN thread_comment_kind ON thread_comment_kind.id = thread_comment.kind_id
         JOIN actor ON actor.id = thread_comment.actor_id
         LEFT JOIN wiki_user ON wiki_user.id = actor.user_id
         WHERE thread.external_id = $1
         ORDER BY thread_comment.sequence",
    )
    .bind(external_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(sequence, kind, user, ip, content, metadata, admin_marked, hidden_at, created_at)| {
                Comment {
                    sequence,
                    kind: CommentKind::parse(&kind),
                    author: user.or(ip).unwrap_or_default(),
                    content,
                    metadata,
                    admin_marked,
                    hidden: hidden_at.is_some(),
                    created_at,
                }
            },
        )
        .collect())
}

async fn insert_comment(
    pool: &PgPool,
    thread_id: i64,
    kind: CommentKind,
    actor: ActorIdentifier,
    content: &str,
    metadata: Option<serde_json::Value>,
    admin_marked: bool,
) -> Result<i64> {
    let kind_id = enumeration_id(pool, "thread_comment_kind", kind.as_str()).await?;

    let (sequence,) = sqlx::query_as::<_, (i64,)>(
        "SELECT COALESCE(MAX(sequence), 0) + 1 FROM thread_comment WHERE thread_id = $1",
    )
    .bind(thread_id)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "INSERT INTO thread_comment
           (thread_id, sequence, kind_id, actor_id, content, metadata, admin_marked, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(thread_id)
    .bind(sequence)
    .bind(kind_id)
    .bind(actor.as_raw())
    .bind(content)
    .bind(metadata)
    .bind(admin_marked)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(sequence)
}

/// 스레드에 발언을 남긴다. 닫히거나 중단된 스레드는 받지 않는다.
pub async fn add_comment(
    pool: &PgPool,
    external_id: Uuid,
    actor: ActorIdentifier,
    content: &str,
    admin_marked: bool,
) -> Result<Option<i64>> {
    let Some((thread_id, thread)) = thread_row(pool, external_id).await? else {
        return Ok(None);
    };
    if !thread.status.accepts_comments() {
        return Ok(None);
    }

    let sequence = insert_comment(
        pool,
        thread_id,
        CommentKind::Comment,
        actor,
        content,
        None,
        admin_marked,
    )
    .await?;

    Ok(Some(sequence))
}

/// 스레드 상태를 바꾸고 그 사실을 타임라인에 남긴다.
pub async fn change_status(
    pool: &PgPool,
    external_id: Uuid,
    actor: ActorIdentifier,
    status: ThreadStatus,
) -> Result<bool> {
    let Some((thread_id, _)) = thread_row(pool, external_id).await? else {
        return Ok(false);
    };
    let status_id = enumeration_id(pool, "thread_status", status.as_str()).await?;

    sqlx::query("UPDATE thread SET status_id = $1 WHERE id = $2")
        .bind(status_id)
        .bind(thread_id)
        .execute(pool)
        .await?;

    insert_comment(
        pool,
        thread_id,
        CommentKind::StatusChange,
        actor,
        "",
        Some(serde_json::json!({ "to": status.as_str() })),
        false,
    )
    .await?;

    Ok(true)
}

/// 스레드 주제를 바꾸고 그 사실을 타임라인에 남긴다.
pub async fn change_topic(
    pool: &PgPool,
    external_id: Uuid,
    actor: ActorIdentifier,
    topic: &str,
) -> Result<bool> {
    let Some((thread_id, _)) = thread_row(pool, external_id).await? else {
        return Ok(false);
    };

    sqlx::query("UPDATE thread SET topic = $1 WHERE id = $2")
        .bind(topic)
        .bind(thread_id)
        .execute(pool)
        .await?;

    insert_comment(
        pool,
        thread_id,
        CommentKind::TopicChange,
        actor,
        "",
        Some(serde_json::json!({ "topic": topic })),
        false,
    )
    .await?;

    Ok(true)
}

/// 발언을 가린다. 지우지 않고 가린 사실과 가린 사람을 남긴다.
pub async fn hide_comment(
    pool: &PgPool,
    external_id: Uuid,
    sequence: i64,
    actor: ActorIdentifier,
) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE thread_comment
         SET hidden_at = $1, hidden_by = $2
         WHERE sequence = $3
           AND thread_id = (SELECT id FROM thread WHERE external_id = $4)",
    )
    .bind(Utc::now())
    .bind(actor.as_raw())
    .bind(sequence)
    .bind(external_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
