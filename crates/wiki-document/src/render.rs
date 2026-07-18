use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use namumark_render::WikiContext;
use sqlx::PgPool;

use crate::reference::{ReferenceKind, ReferenceTarget};
use crate::repository::{document_exists as lookup_exists, namespace_names, read_source};
use crate::{DocumentTitle, Namespace, Result};

/// 렌더 한 번의 결과.
pub struct RenderedDocument {
    pub html: String,
    pub redirect: Option<String>,
    /// 이 문서가 가리킨 것들 — 역링크 테이블을 이 값으로 재작성한다.
    pub references: Vec<ReferenceTarget>,
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
struct RenderPass {
    html: String,
    redirect: Option<String>,
    categories: Vec<String>,
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

    let tree = namumark_render::build_render_tree(&document, &context);
    let html = namumark_backend_namuwiki::namuwiki_markup(&tree).to_string();

    RenderPass {
        html,
        redirect: tree.redirect.clone(),
        categories: tree.categories.clone(),
        asked_existence: context.asked_existence.into_inner(),
        asked_source: context.asked_source.into_inner(),
        asked_file: context.asked_file.into_inner(),
    }
}

/// 문서 원문을 HTML로 렌더하고, 그 과정에서 드러난 참조를 함께 낸다.
///
/// include로 끌어온 원문 안의 링크는 첫 렌더에서 보이지 않으므로, 새로 묻는 것이
/// 없을 때까지 반복한다. 중첩 include는 확장하지 않으므로(설계 확정) 곧 수렴한다.
pub async fn render_document(
    pool: &PgPool,
    title: &DocumentTitle,
    source: &str,
) -> Result<RenderedDocument> {
    let namespaces = namespace_names(pool).await?;
    let current_title = title.to_string();
    let rendered_at = current_date_time();

    let mut snapshot = Snapshot::default();

    loop {
        let pass = render_once(source, &current_title, rendered_at, &snapshot);
        let mut learned_something = false;

        for asked in &pass.asked_existence {
            if !snapshot.existence.contains_key(asked) {
                let parsed = DocumentTitle::parse(asked, &namespaces);
                let exists = lookup_exists(pool, &parsed).await?;
                snapshot.existence.insert(asked.clone(), exists);
                learned_something = true;
            }
        }

        for asked in &pass.asked_source {
            if !snapshot.sources.contains_key(asked) {
                let parsed = DocumentTitle::parse(asked, &namespaces);
                if let Some(included) = read_source(pool, &parsed).await? {
                    snapshot.sources.insert(asked.clone(), included);
                    learned_something = true;
                }
            }
        }

        for asked in &pass.asked_file {
            if !snapshot.file_urls.contains_key(asked) {
                let parsed = DocumentTitle::new(Namespace::new(Namespace::FILE), asked.clone());
                if lookup_exists(pool, &parsed).await? {
                    snapshot.file_urls.insert(asked.clone(), file_url_of(asked));
                    learned_something = true;
                }
            }
        }

        if !learned_something {
            let references = collect_references(&namespaces, &pass);
            return Ok(RenderedDocument {
                html: pass.html,
                redirect: pass.redirect,
                references,
            });
        }
    }
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

    for category in &pass.categories {
        references.push(ReferenceTarget {
            title: DocumentTitle::new(Namespace::new(Namespace::CATEGORY), category.clone()),
            kind: ReferenceKind::Category,
        });
    }

    if let Some(target) = &pass.redirect {
        references.push(ReferenceTarget {
            title: DocumentTitle::parse(target, namespaces),
            kind: ReferenceKind::Redirect,
        });
    }

    references
}
