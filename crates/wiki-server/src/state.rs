use std::path::{Path, PathBuf};
use std::sync::Arc;

use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
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

/// 프론트엔드로 요청을 넘기는 클라이언트. 응답 본문을 그대로 흘려보낸다.
pub type HttpClient = Client<HttpConnector, axum::body::Body>;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub search: Arc<SearchIndex>,
    pub settings: SiteSettings,
    /// 업로드한 바이너리를 담는 디렉터리. DB에는 해시만 두고 내용은 여기 있다.
    pub file_root: PathBuf,
    /// Next.js가 듣는 주소. 없으면 모든 화면을 서버가 직접 그린다.
    pub frontend_origin: Option<String>,
    pub http: HttpClient,
}

/// 데이터베이스만 열고 마이그레이션을 적용한다.
///
/// 검색 색인은 파일이고 쓰기 잠금이 프로세스 하나로 제한되므로, 임포터처럼 서버와
/// 따로 도는 도구는 색인을 열지 않는다 — 색인은 원본에서 다시 만들 수 있는 파생
/// 자료라(docs/architecture.md) 서버가 시작할 때 채우면 된다.
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

    let state = AppState {
        pool,
        search: Arc::new(search),
        settings,
        file_root: file_root.to_owned(),
        frontend_origin: std::env::var("OPENSINABRO_FRONTEND").ok(),
        http: Client::builder(TokioExecutor::new()).build_http(),
    };

    ensure_main_document(&state).await?;

    Ok(state)
}

/// 갓 만든 위키의 대문.
const MAIN_DOCUMENT_SEED: &str = "\
[목차]

== 오픈시나브로 ==
'''오픈시나브로'''는 '''나무마크'''로 글을 쓰는 위키 엔진 오픈소스 프로젝트입니다.
나무마크 문법을 그대로 해석해 문서를 보여 주고, 편집·역사·토론·검색까지 위키를
운영하는 데 필요한 기능을 갖추고 있습니다.

== 나무마크 ==
나무마크는 나무위키에서 쓰는 문법입니다. '''굵게''', ''기울임'', __밑줄__,
--취소선--, 문단, 표, 각주, 틀 같은 요소를 문법 그대로 지원합니다.

== 시작하기 ==
 * 아무 문서나 열어 '''편집'''을 누르면 바로 고칠 수 있습니다.
 * '''최근 변경'''에서 방금 바뀐 문서를 볼 수 있습니다.
 * 아직 없는 문서로 가는 링크는 [[없는 문서|붉게]] 보입니다. 눌러서 새로 쓰세요.

== 이 문서 고치기 ==
이 문서도 다른 문서와 마찬가지로 누구나 고칠 수 있습니다. 위의 '''편집'''을 눌러
이 위키를 소개하는 글로 채우세요.
";

/// 대문이 한 번도 없었으면 심는다.
///
/// 루트 경로가 대문으로 보내므로, 대문이 없는 위키는 첫 화면부터 404다. 기본 ACL을
/// 마이그레이션이 심는 것과 같은 이유다 (docs/architecture.md). 다만 **리비전이 하나도
/// 없을 때만** 만든다 — 운영자가 지운 대문이 재시작마다 되살아나면 안 된다.
async fn ensure_main_document(state: &AppState) -> Result<(), ServerError> {
    let namespaces = sqlx::query_as::<_, (String,)>("SELECT name FROM namespace ORDER BY id")
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .map(|(name,)| name)
        .collect::<Vec<_>>();

    let title = wiki_document::DocumentTitle::parse(&state.settings.main_document, &namespaces);
    if !wiki_document::revision_history(&state.pool, &title, 1)
        .await?
        .is_empty()
    {
        return Ok(());
    }

    let user = wiki_account::ensure_system_user(&state.pool, "시스템").await?;
    let actor = wiki_account::ensure_user_actor(&state.pool, user.identifier).await?;

    wiki_document::record_revision(
        &state.pool,
        &title,
        actor,
        wiki_document::RevisionKind::Create,
        Some(MAIN_DOCUMENT_SEED),
        "대문 만들기",
        None,
    )
    .await?;

    crate::edit::apply_side_effects(state, &title, MAIN_DOCUMENT_SEED).await
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
        wiki_name: setting(pool, "wiki_name", "오픈시나브로").await?,
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
