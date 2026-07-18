use std::path::{Path, PathBuf};
use std::sync::Arc;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use wiki_search::SearchIndex;

use crate::ServerError;

/// 위키 이름처럼 화면에 늘 나오는 설정. 값의 정본은 `site_setting` 테이블이다.
#[derive(Clone)]
pub struct SiteSettings {
    pub wiki_name: String,
    pub main_document: String,
    pub content_license: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub search: Arc<SearchIndex>,
    pub settings: SiteSettings,
    /// 업로드한 바이너리를 담는 디렉터리. DB에는 해시만 두고 내용은 여기 있다.
    pub file_root: PathBuf,
}

/// 데이터베이스만 열고 마이그레이션을 적용한다.
///
/// 검색 색인은 파일이고 쓰기 잠금이 프로세스 하나로 제한되므로, 임포터처럼 서버와
/// 따로 도는 도구는 색인을 열지 않는다 — 색인은 원본에서 다시 만들 수 있는 파생
/// 자료라(docs/design/08) 서버가 시작할 때 채우면 된다.
///
/// 컨테이너로 함께 뜰 때 데이터베이스가 아직 받을 준비가 안 됐을 수 있어 잠시 기다린다.
pub async fn open_database(database_url: &str) -> Result<PgPool, ServerError> {
    let pool = PgPoolOptions::new()
        .max_connections(16)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// 데이터베이스와 검색 색인을 함께 연다 (서버 프로세스용).
pub async fn open_state(
    database_url: &str,
    index_path: &Path,
    file_root: &Path,
) -> Result<AppState, ServerError> {
    let pool = open_database(database_url).await?;
    std::fs::create_dir_all(file_root).map_err(|_| ServerError::Upload)?;

    let settings = load_settings(&pool).await?;
    let search = SearchIndex::open_or_create(index_path)?;

    // 임포터가 DB만 채웠거나 색인 파일을 잃었으면 원본에서 다시 만든다.
    if search.is_empty()? {
        rebuild_index(&pool, &search).await?;
    }

    Ok(AppState {
        pool,
        search: Arc::new(search),
        settings,
        file_root: file_root.to_owned(),
    })
}

/// 저장된 문서 전체로 검색 색인을 다시 만든다.
pub async fn rebuild_index(pool: &PgPool, search: &SearchIndex) -> Result<usize, ServerError> {
    let titles = wiki_document::document_titles(pool).await?;
    let mut indexed = 0;

    for title in &titles {
        if let Some(source) = wiki_document::read_source(pool, title).await? {
            search.put(title.namespace.as_str(), &title.name, &source)?;
            indexed += 1;
        }
    }

    search.commit()?;
    Ok(indexed)
}

async fn load_settings(pool: &PgPool) -> Result<SiteSettings, ServerError> {
    Ok(SiteSettings {
        wiki_name: setting(pool, "wiki_name", "opensinabro").await?,
        main_document: setting(pool, "main_document", "대문").await?,
        content_license: setting(pool, "content_license", "CC BY-NC-SA 2.0 KR").await?,
    })
}

async fn setting(pool: &PgPool, name: &str, fallback: &str) -> Result<String, ServerError> {
    let row = sqlx::query_as::<_, (String,)>("SELECT data FROM site_setting WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(row
        .map(|(data,)| data)
        .unwrap_or_else(|| fallback.to_owned()))
}

impl From<sqlx::migrate::MigrateError> for ServerError {
    fn from(error: sqlx::migrate::MigrateError) -> Self {
        Self::Database(sqlx::Error::Migrate(Box::new(error)))
    }
}
