//! 나무마크 원문 파일을 위키에 적재하는 임포터.
//!
//! HTTP를 거치지 않고 쓰기 경로(리비전 채번 → 역링크 → 렌더 캐시)를 그대로 쓴다.
//! 기여자는 단일 시스템 사용자로 뭉갠다 (docs/design/08).
//!
//! 검색 색인은 열지 않는다 — 색인 파일의 쓰기 잠금은 프로세스 하나만 가질 수 있어,
//! 서버가 돌고 있으면 열 수 없기 때문이다. 색인은 파생 자료라 서버가 시작할 때
//! 비어 있으면 저장된 문서에서 다시 만든다.

use std::path::{Path, PathBuf};

use wiki_document::{DocumentTitle, RevisionKind};

const SYSTEM_USER: &str = "덤프 임포터";

#[tokio::main]
async fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.is_empty() {
        eprintln!("사용법: opensinabro-import <나무마크 파일 또는 디렉터리>...");
        std::process::exit(2);
    }

    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL을 설정하세요 (예: postgres://opensinabro@localhost/opensinabro).");
        std::process::exit(2);
    };

    let pool = match wiki_server::open_database(&database_url).await {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("저장소를 열지 못했습니다: {error}");
            std::process::exit(1);
        }
    };

    let user = wiki_account::ensure_system_user(&pool, SYSTEM_USER)
        .await
        .expect("시스템 사용자 확보");
    let actor = wiki_account::ensure_user_actor(&pool, user.identifier)
        .await
        .expect("시스템 actor 확보");

    let mut sources = Vec::new();
    for argument in &arguments {
        collect_sources(Path::new(argument), &mut sources);
    }

    let namespaces = sqlx::query_as::<_, (String,)>("SELECT name FROM namespace ORDER BY id")
        .fetch_all(&pool)
        .await
        .expect("이름공간 조회")
        .into_iter()
        .map(|(name,)| name)
        .collect::<Vec<_>>();

    let mut imported = 0usize;
    for path in &sources {
        let Ok(content) = std::fs::read_to_string(path) else {
            eprintln!("건너뜀 (읽기 실패): {}", path.display());
            continue;
        };

        let title = title_of(path, &namespaces);

        wiki_document::record_revision(
            &pool,
            &title,
            actor,
            RevisionKind::Import,
            Some(&content),
            "덤프 임포트",
            None,
        )
        .await
        .expect("리비전 기록");

        imported += 1;
    }

    // 역링크와 렌더는 문서가 모두 적재된 뒤에 만든다 — 링크 존재 판정이 최종 상태를 봐야 한다.
    for path in &sources {
        let title = title_of(path, &namespaces);
        let Some(source) = wiki_document::read_source(&pool, &title)
            .await
            .expect("원문 조회")
        else {
            continue;
        };

        let rendered = wiki_document::render_document(&pool, &title, &source)
            .await
            .expect("렌더");
        wiki_document::replace_references(&pool, &title, &rendered.references)
            .await
            .expect("역링크 기록");
        wiki_document::store_render(&pool, &title, &rendered.html)
            .await
            .expect("렌더 캐시 저장");
    }

    println!("{imported}개 문서를 적재했습니다. 검색 색인은 서버가 시작할 때 만들어집니다.");
}

/// 덤프 파일 이름이 곧 문서 제목이다 (공백은 밑줄로 치환돼 있다).
fn title_of(path: &Path, namespaces: &[String]) -> DocumentTitle {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .replace('_', " ");
    DocumentTitle::parse(&stem, namespaces)
}

fn collect_sources(path: &Path, found: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            collect_sources(&entry.path(), found);
        }
        return;
    }

    if path.extension().is_some_and(|value| value == "namu") {
        found.push(path.to_owned());
    }
}
