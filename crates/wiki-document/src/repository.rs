use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wiki_account::ActorIdentifier;

use crate::reference::ReferenceTarget;
use crate::{DocumentError, DocumentTitle, Namespace, Result};

/// 문서의 내부 식별자. 외부 식별은 [`DocumentTitle`]이 맡는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentIdentifier(i64);

impl DocumentIdentifier {
    pub fn as_raw(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct DocumentRecord {
    pub identifier: DocumentIdentifier,
    pub title: DocumentTitle,
}

/// 리비전의 종류. DB의 `revision_kind` 열거와 짝이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionKind {
    Create,
    Edit,
    Move,
    Delete,
    Restore,
    Revert,
    Import,
}

impl RevisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Edit => "edit",
            Self::Move => "move",
            Self::Delete => "delete",
            Self::Restore => "restore",
            Self::Revert => "revert",
            Self::Import => "import",
        }
    }

    /// 이 리비전이 문서를 없앤 상태로 두는가 — "존재하는 문서" 판정의 기준이다.
    pub fn removes_document(self) -> bool {
        matches!(self, Self::Delete)
    }
}

#[derive(Debug, Clone)]
pub struct RevisionRecord {
    pub external_id: Uuid,
    pub sequence: i64,
    pub kind: RevisionKind,
    pub comment: String,
    pub content_bytes: i64,
    pub created_at: DateTime<Utc>,
    /// 사용자 이름, 비로그인이면 IP 주소.
    pub author: String,
}

/// 최근 변경 한 줄 — 어떤 문서의 어떤 리비전인가.
#[derive(Debug, Clone)]
pub struct RecentChange {
    pub title: DocumentTitle,
    pub revision: RevisionRecord,
}

async fn enumeration_id(pool: &PgPool, table: &'static str, name: &str) -> Result<i64> {
    // 테이블 이름은 호출부의 `&'static str` 리터럴이라 사용자 입력이 섞이지 않는다.
    let query = format!("SELECT id FROM {table} WHERE name = $1");
    sqlx::query_as::<_, (i64,)>(sqlx::AssertSqlSafe(query))
        .bind(name)
        .fetch_optional(pool)
        .await?
        .map(|(id,)| id)
        .ok_or_else(|| DocumentError::MissingEnumeration {
            table,
            name: name.to_owned(),
        })
}

pub async fn namespace_names(pool: &PgPool) -> Result<Vec<String>> {
    let rows = sqlx::query_as::<_, (String,)>("SELECT name FROM namespace ORDER BY id")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|(name,)| name).collect())
}

pub async fn find_document(pool: &PgPool, title: &DocumentTitle) -> Result<Option<DocumentRecord>> {
    let row = sqlx::query_as::<_, (i64,)>(
        "SELECT document.id
         FROM document
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE namespace.name = $1 AND document.title = $2",
    )
    .bind(title.namespace.as_str())
    .bind(&title.name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id,)| DocumentRecord {
        identifier: DocumentIdentifier(id),
        title: title.clone(),
    }))
}

async fn ensure_document(pool: &PgPool, title: &DocumentTitle) -> Result<DocumentIdentifier> {
    if let Some(record) = find_document(pool, title).await? {
        return Ok(record.identifier);
    }

    let namespace_id = enumeration_id(pool, "namespace", title.namespace.as_str())
        .await
        .map_err(|_| DocumentError::UnknownNamespace(title.namespace.to_string()))?;

    let (id,) = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO document (namespace_id, title, created_at) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(namespace_id)
    .bind(&title.name)
    .bind(Utc::now())
    .fetch_one(pool)
    .await?;

    Ok(DocumentIdentifier(id))
}

/// 문서의 현재 원문. 삭제된 문서와 없는 문서는 모두 None이다.
pub async fn read_source(pool: &PgPool, title: &DocumentTitle) -> Result<Option<String>> {
    let row = sqlx::query_as::<_, (Option<String>, String)>(
        "SELECT revision.content, revision_kind.name
         FROM revision
         JOIN document ON document.id = revision.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN revision_kind ON revision_kind.id = revision.kind_id
         WHERE namespace.name = $1 AND document.title = $2
         ORDER BY revision.sequence DESC
         LIMIT 1",
    )
    .bind(title.namespace.as_str())
    .bind(&title.name)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some((content, kind)) if kind != RevisionKind::Delete.as_str() => content,
        _ => None,
    })
}

/// "존재하는 문서" = 최신 리비전이 삭제가 아닌 문서 (docs/design/08).
pub async fn document_exists(pool: &PgPool, title: &DocumentTitle) -> Result<bool> {
    Ok(read_source(pool, title).await?.is_some())
}

pub async fn latest_revision(
    pool: &PgPool,
    title: &DocumentTitle,
) -> Result<Option<RevisionRecord>> {
    Ok(revision_history(pool, title, 1).await?.into_iter().next())
}

fn parse_revision_kind(name: &str) -> RevisionKind {
    match name {
        "create" => RevisionKind::Create,
        "move" => RevisionKind::Move,
        "delete" => RevisionKind::Delete,
        "restore" => RevisionKind::Restore,
        "revert" => RevisionKind::Revert,
        "import" => RevisionKind::Import,
        _ => RevisionKind::Edit,
    }
}

/// 문서의 리비전 목록 (최신 순).
pub async fn revision_history(
    pool: &PgPool,
    title: &DocumentTitle,
    limit: i64,
) -> Result<Vec<RevisionRecord>> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            i64,
            String,
            String,
            i64,
            DateTime<Utc>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT revision.external_id, revision.sequence, revision_kind.name,
                revision.comment, revision.content_bytes, revision.created_at,
                wiki_user.name, actor.ip_address
         FROM revision
         JOIN document ON document.id = revision.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN revision_kind ON revision_kind.id = revision.kind_id
         JOIN actor ON actor.id = revision.actor_id
         LEFT JOIN wiki_user ON wiki_user.id = actor.user_id
         WHERE namespace.name = $1 AND document.title = $2
         ORDER BY revision.sequence DESC
         LIMIT $3",
    )
    .bind(title.namespace.as_str())
    .bind(&title.name)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(external_id, sequence, kind, comment, content_bytes, created_at, user, ip)| {
                RevisionRecord {
                    external_id,
                    sequence,
                    kind: parse_revision_kind(&kind),
                    comment,
                    content_bytes,
                    created_at,
                    author: user.or(ip).unwrap_or_default(),
                }
            },
        )
        .collect())
}

/// 특정 리비전의 원문.
pub async fn revision_content(pool: &PgPool, external_id: Uuid) -> Result<Option<Option<String>>> {
    let row = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT content FROM revision WHERE external_id = $1",
    )
    .bind(external_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(content,)| content))
}

/// 최근 변경 — 문서를 가로질러 시간순으로 모은다.
pub async fn recent_changes(pool: &PgPool, limit: i64) -> Result<Vec<RecentChange>> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            Uuid,
            String,
            String,
            i64,
            DateTime<Utc>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT namespace.name, document.title, revision.external_id, revision_kind.name,
                revision.comment, revision.content_bytes, revision.created_at,
                wiki_user.name, actor.ip_address
         FROM revision
         JOIN document ON document.id = revision.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN revision_kind ON revision_kind.id = revision.kind_id
         JOIN actor ON actor.id = revision.actor_id
         LEFT JOIN wiki_user ON wiki_user.id = actor.user_id
         ORDER BY revision.created_at DESC, revision.id DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(namespace, name, external_id, kind, comment, content_bytes, created_at, user, ip)| {
                RecentChange {
                    title: DocumentTitle::new(Namespace::new(namespace), name),
                    revision: RevisionRecord {
                        external_id,
                        sequence: 0,
                        kind: parse_revision_kind(&kind),
                        comment,
                        content_bytes,
                        created_at,
                        author: user.or(ip).unwrap_or_default(),
                    },
                }
            },
        )
        .collect())
}

/// 새 리비전을 남긴다. 문서가 없으면 만든다.
pub async fn record_revision(
    pool: &PgPool,
    title: &DocumentTitle,
    actor: ActorIdentifier,
    kind: RevisionKind,
    content: Option<&str>,
    comment: &str,
    metadata: Option<serde_json::Value>,
) -> Result<Uuid> {
    let document_id = ensure_document(pool, title).await?;
    let kind_id = enumeration_id(pool, "revision_kind", kind.as_str()).await?;

    let (next_sequence,) = sqlx::query_as::<_, (i64,)>(
        "SELECT COALESCE(MAX(sequence), 0) + 1 FROM revision WHERE document_id = $1",
    )
    .bind(document_id.as_raw())
    .fetch_one(pool)
    .await?;

    let external_id = Uuid::new_v4();
    let content_bytes = content.map(|text| text.len() as i64).unwrap_or(0);

    sqlx::query(
        "INSERT INTO revision
           (external_id, document_id, sequence, kind_id, actor_id,
            content, comment, metadata, content_bytes, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(external_id)
    .bind(document_id.as_raw())
    .bind(next_sequence)
    .bind(kind_id)
    .bind(actor.as_raw())
    .bind(content)
    .bind(comment)
    .bind(metadata)
    .bind(content_bytes)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(external_id)
}

/// 문서를 다른 제목으로 옮긴다.
///
/// 문서의 항등은 id라 제목 컬럼만 바꾸면 역사·토론·ACL이 따라온다(docs/design/08).
/// 옮길 자리에 이미 역사가 있는 문서가 있으면 **서로 맞바꾼다** — the seed의 동작이고,
/// 그래야 잘못 만든 제목과 본래 제목을 뒤집는 일이 역사를 잃지 않고 된다.
pub async fn move_document(
    pool: &PgPool,
    from: &DocumentTitle,
    to: &DocumentTitle,
    actor: ActorIdentifier,
    comment: &str,
) -> Result<bool> {
    let Some(source) = find_document(pool, from).await? else {
        return Ok(false);
    };
    let target_namespace = enumeration_id(pool, "namespace", to.namespace.as_str())
        .await
        .map_err(|_| DocumentError::UnknownNamespace(to.namespace.to_string()))?;

    let mut transaction = pool.begin().await?;
    let existing = sqlx::query_as::<_, (i64,)>(
        "SELECT document.id
         FROM document
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE namespace.name = $1 AND document.title = $2",
    )
    .bind(to.namespace.as_str())
    .bind(&to.name)
    .fetch_optional(&mut *transaction)
    .await?;

    if let Some((occupant,)) = existing {
        // 제목은 유일해야 하므로 한쪽을 잠시 비켜 둔 뒤 맞바꾼다.
        let parked = format!("\u{0}이동중-{}", Uuid::new_v4());
        sqlx::query("UPDATE document SET title = $1 WHERE id = $2")
            .bind(&parked)
            .bind(occupant)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("UPDATE document SET namespace_id = $1, title = $2 WHERE id = $3")
            .bind(target_namespace)
            .bind(&to.name)
            .bind(source.identifier.as_raw())
            .execute(&mut *transaction)
            .await?;
        let origin_namespace = enumeration_id(pool, "namespace", from.namespace.as_str()).await?;
        sqlx::query("UPDATE document SET namespace_id = $1, title = $2 WHERE id = $3")
            .bind(origin_namespace)
            .bind(&from.name)
            .bind(occupant)
            .execute(&mut *transaction)
            .await?;
    } else {
        sqlx::query("UPDATE document SET namespace_id = $1, title = $2 WHERE id = $3")
            .bind(target_namespace)
            .bind(&to.name)
            .bind(source.identifier.as_raw())
            .execute(&mut *transaction)
            .await?;
    }

    transaction.commit().await?;

    record_revision(
        pool,
        to,
        actor,
        RevisionKind::Move,
        read_source(pool, to).await?.as_deref(),
        comment,
        Some(serde_json::json!({ "from": from.to_string(), "to": to.to_string() })),
    )
    .await?;

    Ok(true)
}

/// 문서를 삭제한다 — 내용을 지우는 게 아니라 "없는 상태"를 리비전으로 남긴다.
pub async fn delete_document(
    pool: &PgPool,
    title: &DocumentTitle,
    actor: ActorIdentifier,
    comment: &str,
) -> Result<bool> {
    if read_source(pool, title).await?.is_none() {
        return Ok(false);
    }

    record_revision(
        pool,
        title,
        actor,
        RevisionKind::Delete,
        None,
        comment,
        None,
    )
    .await?;
    Ok(true)
}

/// 줄마다 그 줄을 마지막으로 바꾼 리비전을 짚는다.
pub async fn blame(pool: &PgPool, title: &DocumentTitle) -> Result<Vec<BlameLine>> {
    let mut revisions = revision_history(pool, title, 500).await?;
    revisions.reverse();

    let mut attribution: Vec<(String, i64)> = Vec::new();
    let mut previous = String::new();

    for revision in &revisions {
        let Some(content) = revision_content(pool, revision.external_id)
            .await?
            .flatten()
        else {
            continue;
        };

        let mut next = Vec::new();
        for (index, line) in content.lines().enumerate() {
            // 앞 판과 같은 자리에 같은 글이면 그때의 지은이를 그대로 물려받는다.
            let inherited = previous
                .lines()
                .nth(index)
                .filter(|old| *old == line)
                .and_then(|_| attribution.get(index).cloned());

            next.push(inherited.unwrap_or((revision.author.clone(), revision.sequence)));
        }

        attribution = next;
        previous = content;
    }

    Ok(previous
        .lines()
        .enumerate()
        .map(|(index, text)| {
            let (author, sequence) = attribution
                .get(index)
                .cloned()
                .unwrap_or_else(|| (String::new(), 0));
            BlameLine {
                text: text.to_owned(),
                author,
                sequence,
            }
        })
        .collect())
}

#[derive(Debug, Clone)]
pub struct BlameLine {
    pub text: String,
    pub author: String,
    pub sequence: i64,
}

/// 리비전을 숨긴다 — 목록에는 남고 내용만 가린다.
pub async fn set_revision_hidden(pool: &PgPool, external_id: Uuid, hidden: bool) -> Result<bool> {
    let result = sqlx::query("UPDATE revision SET hidden = $1 WHERE external_id = $2")
        .bind(hidden)
        .bind(external_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// 한 사람이 마지막으로 손댄 문서들 — 일괄 되돌리기가 대상으로 삼는다.
pub async fn documents_last_edited_by(
    pool: &PgPool,
    author: &str,
    limit: i64,
) -> Result<Vec<DocumentTitle>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT namespace.name, document.title
         FROM document
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN revision ON revision.id = (
             SELECT id FROM revision
             WHERE document_id = document.id
             ORDER BY sequence DESC
             LIMIT 1
         )
         JOIN actor ON actor.id = revision.actor_id
         LEFT JOIN wiki_user ON wiki_user.id = actor.user_id
         WHERE wiki_user.name = $1 OR actor.ip_address = $1
         LIMIT $2",
    )
    .bind(author)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name)| DocumentTitle::new(Namespace::new(namespace), name))
        .collect())
}

/// 그 사람이 손대기 직전의 내용 — 일괄 되돌리기가 되돌아갈 자리.
pub async fn content_before_author(
    pool: &PgPool,
    title: &DocumentTitle,
    author: &str,
) -> Result<Option<String>> {
    let revisions = revision_history(pool, title, 500).await?;

    // 최신부터 훑어 그 사람이 아닌 첫 리비전을 찾는다.
    for revision in revisions {
        if revision.author != author {
            return Ok(revision_content(pool, revision.external_id)
                .await?
                .flatten());
        }
    }

    Ok(None)
}

/// 역링크를 그 문서의 것만 통째로 갈아 끼운다 (파생 자료라 재구성이 정답이다).
pub async fn replace_references(
    pool: &PgPool,
    title: &DocumentTitle,
    references: &[ReferenceTarget],
) -> Result<()> {
    let document_id = ensure_document(pool, title).await?;

    sqlx::query("DELETE FROM document_reference WHERE source_document_id = $1")
        .bind(document_id.as_raw())
        .execute(pool)
        .await?;

    for reference in references {
        let namespace_id =
            match enumeration_id(pool, "namespace", reference.title.namespace.as_str()).await {
                Ok(id) => id,
                // 없는 이름공간을 가리키는 링크는 역링크로 남기지 않는다.
                Err(DocumentError::MissingEnumeration { .. }) => continue,
                Err(error) => return Err(error),
            };
        let kind_id =
            enumeration_id(pool, "document_reference_kind", reference.kind.as_str()).await?;

        sqlx::query(
            "INSERT INTO document_reference
               (source_document_id, target_namespace_id, target_title, kind_id)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT DO NOTHING",
        )
        .bind(document_id.as_raw())
        .bind(namespace_id)
        .bind(&reference.title.name)
        .bind(kind_id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// 현재 리비전에 대한 렌더 결과가 캐시에 있으면 꺼낸다.
///
/// 렌더는 문서가 참조하는 다른 문서의 상태에 따라 여러 번 돌 수 있으므로(include가
/// 끌어온 원문 안의 링크), 보기 요청마다 되풀이하지 않도록 결과를 남긴다.
pub async fn cached_render(pool: &PgPool, title: &DocumentTitle) -> Result<Option<String>> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT render_cache.html
         FROM render_cache
         JOIN document ON document.id = render_cache.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE namespace.name = $1
           AND document.title = $2
           AND render_cache.revision_id = (
               SELECT id FROM revision
               WHERE document_id = document.id
               ORDER BY sequence DESC
               LIMIT 1
           )",
    )
    .bind(title.namespace.as_str())
    .bind(&title.name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(html,)| html))
}

/// 현재 리비전의 렌더 결과를 캐시에 둔다.
pub async fn store_render(pool: &PgPool, title: &DocumentTitle, html: &str) -> Result<()> {
    let Some(record) = find_document(pool, title).await? else {
        return Ok(());
    };

    let Some((revision_id,)) = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM revision WHERE document_id = $1 ORDER BY sequence DESC LIMIT 1",
    )
    .bind(record.identifier.as_raw())
    .fetch_optional(pool)
    .await?
    else {
        return Ok(());
    };

    sqlx::query(
        "INSERT INTO render_cache (document_id, revision_id, html, rendered_at)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (document_id) DO UPDATE
           SET revision_id = excluded.revision_id,
               html = excluded.html,
               rendered_at = excluded.rendered_at",
    )
    .bind(record.identifier.as_raw())
    .bind(revision_id)
    .bind(html)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

/// 이 문서를 가리키는 문서들의 렌더 캐시를 버린다.
///
/// 문서가 생기거나 사라지면 그것을 링크한 문서의 빨간 링크 표시가 달라지고,
/// include·리다이렉트 대상이면 내용 자체가 달라지기 때문이다.
pub async fn invalidate_referrers(pool: &PgPool, title: &DocumentTitle) -> Result<()> {
    sqlx::query(
        "DELETE FROM render_cache
         WHERE document_id IN (
             SELECT document_reference.source_document_id
             FROM document_reference
             JOIN namespace ON namespace.id = document_reference.target_namespace_id
             WHERE namespace.name = $1 AND document_reference.target_title = $2
         )",
    )
    .bind(title.namespace.as_str())
    .bind(&title.name)
    .execute(pool)
    .await?;

    Ok(())
}

/// 존재하는 문서 제목들 (검색 색인 재구축·목록용).
pub async fn document_titles(pool: &PgPool) -> Result<Vec<DocumentTitle>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT namespace.name, document.title
         FROM document
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE (
             SELECT revision_kind.name
             FROM revision
             JOIN revision_kind ON revision_kind.id = revision.kind_id
             WHERE revision.document_id = document.id
             ORDER BY revision.sequence DESC
             LIMIT 1
         ) <> 'delete'
         ORDER BY namespace.name, document.title",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name)| DocumentTitle::new(Namespace::new(namespace), name))
        .collect())
}

/// 임의 문서 (`/random`).
pub async fn random_title(pool: &PgPool) -> Result<Option<DocumentTitle>> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT namespace.name, document.title
         FROM document
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE (
             SELECT revision_kind.name
             FROM revision
             JOIN revision_kind ON revision_kind.id = revision.kind_id
             WHERE revision.document_id = document.id
             ORDER BY revision.sequence DESC
             LIMIT 1
         ) <> 'delete'
         ORDER BY RANDOM()
         LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(namespace, name)| DocumentTitle::new(Namespace::new(namespace), name)))
}

/// 이 문서를 가리키는 문서들 (`/backlink/`).
pub async fn backlinks(
    pool: &PgPool,
    title: &DocumentTitle,
) -> Result<Vec<(DocumentTitle, String)>> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT namespace.name, document.title, document_reference_kind.name
         FROM document_reference
         JOIN document ON document.id = document_reference.source_document_id
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN document_reference_kind ON document_reference_kind.id = document_reference.kind_id
         JOIN namespace target ON target.id = document_reference.target_namespace_id
         WHERE target.name = $1 AND document_reference.target_title = $2
         ORDER BY namespace.name, document.title",
    )
    .bind(title.namespace.as_str())
    .bind(&title.name)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name, kind)| (DocumentTitle::new(Namespace::new(namespace), name), kind))
        .collect())
}

/// 문서 구독을 켜고 끈다. 이미 구독 중이면 끄고, 아니면 켠다.
pub async fn toggle_star(pool: &PgPool, user_id: i64, title: &DocumentTitle) -> Result<bool> {
    let Some(document) = find_document(pool, title).await? else {
        return Ok(false);
    };

    let removed = sqlx::query("DELETE FROM star WHERE user_id = $1 AND document_id = $2")
        .bind(user_id)
        .bind(document.identifier.as_raw())
        .execute(pool)
        .await?;

    if removed.rows_affected() > 0 {
        return Ok(false);
    }

    sqlx::query("INSERT INTO star (user_id, document_id, created_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(document.identifier.as_raw())
        .bind(Utc::now())
        .execute(pool)
        .await?;

    Ok(true)
}

pub async fn is_starred(pool: &PgPool, user_id: i64, title: &DocumentTitle) -> Result<bool> {
    let found = sqlx::query_as::<_, (i64,)>(
        "SELECT star.user_id
         FROM star
         JOIN document ON document.id = star.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE star.user_id = $1 AND namespace.name = $2 AND document.title = $3",
    )
    .bind(user_id)
    .bind(title.namespace.as_str())
    .bind(&title.name)
    .fetch_optional(pool)
    .await?;

    Ok(found.is_some())
}

/// 내가 구독한 문서들.
pub async fn starred_titles(pool: &PgPool, user_id: i64) -> Result<Vec<DocumentTitle>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT namespace.name, document.title
         FROM star
         JOIN document ON document.id = star.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE star.user_id = $1
         ORDER BY star.created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name)| DocumentTitle::new(Namespace::new(namespace), name))
        .collect())
}

/// 이 문서를 구독한 사람들 — 알림을 보낼 대상.
pub async fn subscribers(pool: &PgPool, title: &DocumentTitle) -> Result<Vec<i64>> {
    let rows = sqlx::query_as::<_, (i64,)>(
        "SELECT star.user_id
         FROM star
         JOIN document ON document.id = star.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE namespace.name = $1 AND document.title = $2",
    )
    .bind(title.namespace.as_str())
    .bind(&title.name)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(user_id,)| user_id).collect())
}

/// 아무도 가리키지 않는 문서들 (`/orphaned-pages`).
pub async fn orphaned_titles(pool: &PgPool, limit: i64) -> Result<Vec<DocumentTitle>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT namespace.name, document.title
         FROM document
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE NOT EXISTS (
             SELECT 1 FROM document_reference
             WHERE document_reference.target_namespace_id = document.namespace_id
               AND document_reference.target_title = document.title
         )
         AND (
             SELECT revision_kind.name FROM revision
             JOIN revision_kind ON revision_kind.id = revision.kind_id
             WHERE revision.document_id = document.id
             ORDER BY revision.sequence DESC LIMIT 1
         ) <> 'delete'
         ORDER BY namespace.name, document.title
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name)| DocumentTitle::new(Namespace::new(namespace), name))
        .collect())
}

/// 분류가 붙지 않은 문서들 (`/uncategorized-pages`).
pub async fn uncategorized_titles(pool: &PgPool, limit: i64) -> Result<Vec<DocumentTitle>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT namespace.name, document.title
         FROM document
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE NOT EXISTS (
             SELECT 1 FROM document_reference
             JOIN document_reference_kind
               ON document_reference_kind.id = document_reference.kind_id
             WHERE document_reference.source_document_id = document.id
               AND document_reference_kind.name = 'category'
         )
         AND (
             SELECT revision_kind.name FROM revision
             JOIN revision_kind ON revision_kind.id = revision.kind_id
             WHERE revision.document_id = document.id
             ORDER BY revision.sequence DESC LIMIT 1
         ) <> 'delete'
         ORDER BY namespace.name, document.title
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name)| DocumentTitle::new(Namespace::new(namespace), name))
        .collect())
}

/// 손댄 지 오래된 문서들 (`/old-pages`).
pub async fn stale_titles(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<(DocumentTitle, DateTime<Utc>)>> {
    let rows = sqlx::query_as::<_, (String, String, DateTime<Utc>)>(
        "SELECT namespace.name, document.title, latest.created_at
         FROM document
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN LATERAL (
             SELECT revision.created_at, revision_kind.name AS kind
             FROM revision
             JOIN revision_kind ON revision_kind.id = revision.kind_id
             WHERE revision.document_id = document.id
             ORDER BY revision.sequence DESC
             LIMIT 1
         ) latest ON true
         WHERE latest.kind <> 'delete'
         ORDER BY latest.created_at
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name, at)| (DocumentTitle::new(Namespace::new(namespace), name), at))
        .collect())
}

/// 길이로 줄 세운 문서들 (`/shortest-pages`·`/longest-pages`).
pub async fn titles_by_length(
    pool: &PgPool,
    longest_first: bool,
    limit: i64,
) -> Result<Vec<(DocumentTitle, i64)>> {
    let rows = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT namespace.name, document.title, latest.content_bytes
         FROM document
         JOIN namespace ON namespace.id = document.namespace_id
         JOIN LATERAL (
             SELECT revision.content_bytes, revision_kind.name AS kind
             FROM revision
             JOIN revision_kind ON revision_kind.id = revision.kind_id
             WHERE revision.document_id = document.id
             ORDER BY revision.sequence DESC
             LIMIT 1
         ) latest ON true
         WHERE latest.kind <> 'delete'
         ORDER BY CASE WHEN $1 THEN -latest.content_bytes ELSE latest.content_bytes END
         LIMIT $2",
    )
    .bind(longest_first)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name, bytes)| {
            (DocumentTitle::new(Namespace::new(namespace), name), bytes)
        })
        .collect())
}

/// 제목 앞부분이 맞는 문서들 — 검색창 자동완성이 쓴다.
pub async fn titles_starting_with(
    pool: &PgPool,
    prefix: &str,
    limit: i64,
) -> Result<Vec<DocumentTitle>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT namespace.name, document.title
         FROM document
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE document.title ILIKE $1 || '%'
           AND (
             SELECT revision_kind.name FROM revision
             JOIN revision_kind ON revision_kind.id = revision.kind_id
             WHERE revision.document_id = document.id
             ORDER BY revision.sequence DESC LIMIT 1
           ) <> 'delete'
         ORDER BY LENGTH(document.title), document.title
         LIMIT $2",
    )
    .bind(prefix)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name)| DocumentTitle::new(Namespace::new(namespace), name))
        .collect())
}

/// 링크만 있고 문서가 없는 제목들 (`/needed-pages`).
pub async fn titles_missing(pool: &PgPool, limit: i64) -> Result<Vec<(DocumentTitle, i64)>> {
    let rows = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT namespace.name, document_reference.target_title, COUNT(*) AS reference_count
         FROM document_reference
         JOIN namespace ON namespace.id = document_reference.target_namespace_id
         WHERE NOT EXISTS (
             SELECT 1 FROM document
             WHERE document.namespace_id = document_reference.target_namespace_id
               AND document.title = document_reference.target_title
         )
         GROUP BY namespace.name, document_reference.target_title
         ORDER BY reference_count DESC, document_reference.target_title
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(namespace, name, count)| {
            (DocumentTitle::new(Namespace::new(namespace), name), count)
        })
        .collect())
}
