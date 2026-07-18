use std::path::{Path, PathBuf};

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{DocumentError, Result};

/// 저장된 바이너리의 내용 주소. 같은 내용은 한 번만 담긴다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 내용을 해시해 저장 경로를 정한다. 앞 두 글자로 디렉터리를 나눠
    /// 한 폴더에 파일이 수십만 개 쌓이지 않게 한다.
    fn path_within(&self, root: &Path) -> PathBuf {
        root.join(&self.0[..2]).join(&self.0)
    }
}

/// 업로드를 받을 수 있는 형식. 위키가 그리는 것은 이미지뿐이라 그 밖은 받지 않는다
/// — 받아 둔 파일이 곧 서비스가 내보내는 것이라, 늘리려면 그때 판단해 늘린다.
pub const SUPPORTED_MEDIA_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/svg+xml",
];

pub fn is_supported_media_type(media_type: &str) -> bool {
    SUPPORTED_MEDIA_TYPES.contains(&media_type)
}

/// 바이너리를 저장하고 내용 주소를 낸다. 이미 있는 내용이면 다시 쓰지 않는다.
pub async fn store_content(
    pool: &PgPool,
    root: &Path,
    bytes: &[u8],
    media_type: &str,
) -> Result<ContentHash> {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(bytes);
    let hash = ContentHash(
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    );

    let path = hash.path_within(root);
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(DocumentError::FileStorage)?;
        }
        std::fs::write(&path, bytes).map_err(DocumentError::FileStorage)?;
    }

    sqlx::query(
        "INSERT INTO file_content (hash, media_type, byte_size, created_at)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (hash) DO NOTHING",
    )
    .bind(hash.as_str())
    .bind(media_type)
    .bind(bytes.len() as i64)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(hash)
}

/// 리비전에 바이너리와 라이선스를 묶는다 — 파일 문서의 그 판이 무엇이었나를 남긴다.
pub async fn attach_file(
    pool: &PgPool,
    revision: Uuid,
    hash: &ContentHash,
    license: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO file_revision (revision_id, content_hash, license_id)
         SELECT revision.id, $2, license.id
         FROM revision, license
         WHERE revision.external_id = $1 AND license.name = $3",
    )
    .bind(revision)
    .bind(hash.as_str())
    .bind(license)
    .execute(pool)
    .await?;

    Ok(())
}

/// 서빙할 바이너리 — 파일 문서의 현재 리비전에 묶인 것.
pub struct StoredFile {
    pub path: PathBuf,
    pub media_type: String,
}

pub async fn read_file(pool: &PgPool, root: &Path, name: &str) -> Result<Option<StoredFile>> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT file_revision.content_hash, file_content.media_type
         FROM file_revision
         JOIN file_content ON file_content.hash = file_revision.content_hash
         JOIN revision ON revision.id = file_revision.revision_id
         JOIN document ON document.id = revision.document_id
         JOIN namespace ON namespace.id = document.namespace_id
         WHERE namespace.name = '파일' AND document.title = $1
         ORDER BY revision.sequence DESC
         LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(hash, media_type)| StoredFile {
        path: ContentHash(hash).path_within(root),
        media_type,
    }))
}

/// 운영자가 고른 라이선스 목록 (업로드 폼이 쓴다).
pub async fn licenses(pool: &PgPool) -> Result<Vec<(String, String)>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT name, display_name FROM license ORDER BY display_name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
