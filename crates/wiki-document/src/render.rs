use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use namumark_ir::RenderTree;
use namumark_render::WikiContext;
use sqlx::PgPool;

use crate::reference::{ReferenceKind, ReferenceTarget};
use crate::repository::{documents_that_exist, namespace_names, read_sources};
use crate::scan::{Candidates, scan};
use crate::{DocumentTitle, Namespace, Result};

/// 렌더 한 번의 결과.
pub struct RenderedDocument {
    /// 렌더 트리 그 자체. 백엔드를 고르는 것은 이 값을 받는 쪽의 몫이다.
    pub tree: RenderTree,
    /// 이 문서가 가리킨 것들 — 역링크 테이블을 이 값으로 재작성한다.
    pub references: Vec<ReferenceTarget>,
}

impl RenderedDocument {
    pub fn redirect(&self) -> Option<&str> {
        self.tree.redirect.as_deref()
    }
}

/// 렌더러가 무엇을 물었는지 기록만 하는 컨텍스트.
///
/// [`WikiContext`]는 동기 trait이라 렌더 도중에 DB(async)를 볼 수 없다. 대신 한 번
/// 렌더시켜 렌더러가 실제로 묻는 제목을 모으고, 그 답을 미리 채운 스냅샷으로 다시
/// 렌더한다 — AST를 따로 순회하지 않으므로 수집 결과가 렌더 의미론과 어긋나지 않는다.
struct RecordingContext<'a> {
    current_title: String,
    rendered_at: namumark_render::DateTime,
    known: &'a Snapshot,
    asked_existence: RefCell<HashSet<String>>,
    asked_source: RefCell<HashSet<String>>,
    asked_file: RefCell<HashSet<String>>,
}

#[derive(Default)]
struct Snapshot {
    existence: HashMap<String, bool>,
    sources: HashMap<String, String>,
    file_urls: HashMap<String, String>,
    /// 조회를 마친 제목. 없는 문서·파일은 위 맵에 값을 남기지 않으므로, 이것이 없으면
    /// "아직 모른다"와 "찾아봤는데 없더라"를 구별하지 못해 같은 것을 영원히 되묻는다.
    resolved_sources: HashSet<String>,
    resolved_files: HashSet<String>,
}

impl WikiContext for RecordingContext<'_> {
    fn document_exists(&self, title: &str) -> bool {
        self.asked_existence.borrow_mut().insert(title.to_owned());
        self.known.existence.get(title).copied().unwrap_or(false)
    }

    fn current_title(&self) -> Option<String> {
        Some(self.current_title.clone())
    }

    fn include_source(&self, title: &str) -> Option<String> {
        self.asked_source.borrow_mut().insert(title.to_owned());
        self.known.sources.get(title).cloned()
    }

    fn file_url(&self, file_name: &str) -> Option<String> {
        self.asked_file.borrow_mut().insert(file_name.to_owned());
        self.known.file_urls.get(file_name).cloned()
    }

    /// `[date]`·`[age(…)]` 등이 실제 시각을 보게 한다. None이면 원문 표기로 남는다.
    fn now(&self) -> Option<namumark_render::DateTime> {
        Some(self.rendered_at)
    }
}

/// 렌더 한 회차의 산출물. 구문 트리(rowan)는 `Send`가 아니므로 이 함수 안에서 수명이
/// 끝나야 한다 — 그래야 호출자의 async 흐름이 `Send`로 남는다.
///
/// [`RenderTree`]는 rowan을 가리키지 않는 순수 값이라 이 경계를 넘어도 된다.
struct RenderPass {
    tree: RenderTree,
    asked_existence: HashSet<String>,
    asked_source: HashSet<String>,
    asked_file: HashSet<String>,
}

fn render_once(
    source: &str,
    current_title: &str,
    rendered_at: namumark_render::DateTime,
    known: &Snapshot,
) -> RenderPass {
    let document = namumark_parser::parse(source);
    let context = RecordingContext {
        current_title: current_title.to_owned(),
        rendered_at,
        known,
        asked_existence: RefCell::new(HashSet::new()),
        asked_source: RefCell::new(HashSet::new()),
        asked_file: RefCell::new(HashSet::new()),
    };

    RenderPass {
        tree: namumark_render::build_render_tree(&document, &context),
        asked_existence: context.asked_existence.into_inner(),
        asked_source: context.asked_source.into_inner(),
        asked_file: context.asked_file.into_inner(),
    }
}

/// 문서 원문을 렌더 트리로 만들고, 그 과정에서 드러난 참조를 함께 낸다.
///
/// 렌더 자체는 동기라 도중에 DB를 볼 수 없으므로, 먼저 원문을 훑어([`scan`]) 물어볼
/// 법한 제목을 배치로 받아 스냅샷을 채운다. 그러면 대개 첫 렌더가 아무것도 새로 묻지
/// 않아 그 자리에서 끝난다.
///
/// 그래도 루프는 남긴다. 스캔은 추측이라 상대 제목·틀 인자로 만들어지는 제목을 놓칠 수
/// 있는데, 그때 답 없이 렌더하면 있는 문서가 빨간 링크로 나가는 식으로 **조용히 틀린다**.
/// 루프가 그 경우를 받아내므로 정확성이 스캔의 정확도에 매달리지 않는다 — 스캔은 순수한
/// 최적화이고, 놓치면 예전처럼 한 바퀴 더 돌 뿐이다.
pub async fn render_document(
    pool: &PgPool,
    title: &DocumentTitle,
    source: &str,
) -> Result<RenderedDocument> {
    let namespaces = namespace_names(pool).await?;
    let current_title = title.to_string();
    let rendered_at = current_date_time();

    let mut snapshot = prefetch(pool, source, &current_title, &namespaces).await?;

    loop {
        let pass = render_once(source, &current_title, rendered_at, &snapshot);

        let existence: Vec<String> = pass
            .asked_existence
            .iter()
            .filter(|asked| !snapshot.existence.contains_key(*asked))
            .cloned()
            .collect();
        let sources: Vec<String> = pass
            .asked_source
            .iter()
            .filter(|asked| !snapshot.resolved_sources.contains(*asked))
            .cloned()
            .collect();
        let files: Vec<String> = pass
            .asked_file
            .iter()
            .filter(|asked| !snapshot.resolved_files.contains(*asked))
            .cloned()
            .collect();

        if existence.is_empty() && sources.is_empty() && files.is_empty() {
            let references = collect_references(&namespaces, &pass);
            return Ok(RenderedDocument {
                tree: pass.tree,
                references,
            });
        }

        learn(
            pool,
            &mut snapshot,
            &namespaces,
            &existence,
            &sources,
            &files,
        )
        .await?;
    }
}

/// 원문이 물어볼 법한 것을 미리 받아 둔다.
///
/// include로 끌어온 원문 안의 링크는 원문 스캔에 보이지 않으므로, 받아온 틀 원문을 한 번
/// 더 훑어 2차로 받는다. 중첩 include는 확장되지 않으므로(설계 확정) 여기서 끝난다.
async fn prefetch(
    pool: &PgPool,
    source: &str,
    current_title: &str,
    namespaces: &[String],
) -> Result<Snapshot> {
    let mut snapshot = Snapshot::default();

    let found = scan(source, current_title);
    learn(
        pool,
        &mut snapshot,
        namespaces,
        &found.links,
        &found.includes,
        &found.files,
    )
    .await?;

    let mut nested = Candidates::default();
    // 틀 안에서도 상대 제목은 **부른 쪽** 문서를 기준으로 풀린다 — resolve가 보는
    // `current_title`이 틀이 아니라 이 문서이기 때문이다.
    for template_source in snapshot.sources.values() {
        let found = scan(template_source, current_title);
        nested.links.extend(found.links);
        nested.files.extend(found.files);
    }
    nested.dedup();
    // 1차에서 이미 답을 받은 것은 빼고 묻는다.
    nested
        .links
        .retain(|title| !snapshot.existence.contains_key(title));
    nested
        .files
        .retain(|name| !snapshot.resolved_files.contains(name));
    learn(
        pool,
        &mut snapshot,
        namespaces,
        &nested.links,
        &[],
        &nested.files,
    )
    .await?;

    Ok(snapshot)
}

/// 제목 묶음의 답을 배치로 받아 스냅샷에 채운다. 종류마다 왕복 한 번씩이다.
async fn learn(
    pool: &PgPool,
    snapshot: &mut Snapshot,
    namespaces: &[String],
    existence: &[String],
    sources: &[String],
    files: &[String],
) -> Result<()> {
    if !existence.is_empty() {
        let parsed: Vec<DocumentTitle> = existence
            .iter()
            .map(|asked| DocumentTitle::parse(asked, namespaces))
            .collect();
        let present = documents_that_exist(pool, &parsed).await?;
        for (asked, title) in existence.iter().zip(&parsed) {
            snapshot
                .existence
                .insert(asked.clone(), present.contains(title));
        }
    }

    if !sources.is_empty() {
        let parsed: Vec<DocumentTitle> = sources
            .iter()
            .map(|asked| DocumentTitle::parse(asked, namespaces))
            .collect();
        let mut found = read_sources(pool, &parsed).await?;
        for (asked, title) in sources.iter().zip(&parsed) {
            if let Some(included) = found.remove(title) {
                snapshot.sources.insert(asked.clone(), included);
            }
            snapshot.resolved_sources.insert(asked.clone());
        }
    }

    if !files.is_empty() {
        let parsed: Vec<DocumentTitle> = files
            .iter()
            .map(|asked| DocumentTitle::new(Namespace::new(Namespace::FILE), asked.clone()))
            .collect();
        let present = documents_that_exist(pool, &parsed).await?;
        for (asked, title) in files.iter().zip(&parsed) {
            if present.contains(title) {
                snapshot.file_urls.insert(asked.clone(), file_url_of(asked));
            }
            snapshot.resolved_files.insert(asked.clone());
        }
    }

    Ok(())
}

/// 바이너리 서빙 경로 (docs/architecture.md의 URL 설계).
fn file_url_of(file_name: &str) -> String {
    format!("/file/{file_name}")
}

fn current_date_time() -> namumark_render::DateTime {
    use chrono::{Datelike, Timelike};

    let now = chrono::Utc::now();
    namumark_render::DateTime {
        date: namumark_render::Date {
            year: now.year(),
            month: now.month(),
            day: now.day(),
        },
        time: namumark_render::Time {
            hour: now.hour(),
            minute: now.minute(),
            second: now.second(),
        },
    }
}

fn collect_references(namespaces: &[String], pass: &RenderPass) -> Vec<ReferenceTarget> {
    let mut references = Vec::new();

    for asked in &pass.asked_existence {
        references.push(ReferenceTarget {
            title: DocumentTitle::parse(asked, namespaces),
            kind: ReferenceKind::Link,
        });
    }

    for asked in &pass.asked_source {
        references.push(ReferenceTarget {
            title: DocumentTitle::parse(asked, namespaces),
            kind: ReferenceKind::Include,
        });
    }

    for asked in &pass.asked_file {
        references.push(ReferenceTarget {
            title: DocumentTitle::new(Namespace::new(Namespace::FILE), asked.clone()),
            kind: ReferenceKind::Image,
        });
    }

    for category in &pass.tree.categories {
        references.push(ReferenceTarget {
            title: DocumentTitle::new(Namespace::new(Namespace::CATEGORY), category.clone()),
            kind: ReferenceKind::Category,
        });
    }

    if let Some(target) = &pass.tree.redirect {
        references.push(ReferenceTarget {
            title: DocumentTitle::parse(target, namespaces),
            kind: ReferenceKind::Redirect,
        });
    }

    references
}
