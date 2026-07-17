//! 코퍼스 로딩, the seed 문서를 공급하는 `WikiContext`, 그리고 구조 지문 diff.

use crate::normalize::Fragment;
use namumark_render::{WikiContext, build_render_tree};
use std::collections::HashMap;
use std::path::Path;

pub struct Case {
    pub document: String,
    pub source: String,
    pub rendered: String,
}

pub struct Corpus {
    cases: Vec<Case>,
    sources: HashMap<String, String>,
    /// 파일 이름 → 실제 이미지 URL. 파일 저장소는 위키 DB에 있으므로 알아낼 길이 없다.
    /// the seed가 렌더한 `<img alt='파일:X' src='...'>`에서 거꾸로 주워 온다.
    file_urls: HashMap<String, String>,
    /// 실제로 있는 문서. 링크 존재 여부도 위키 DB에 달렸으므로, the seed 렌더의
    /// `wiki-link-internal`(있음)과 `not-exist`(없음)에서 거꾸로 주워 온다.
    existing_documents: std::collections::HashSet<String>,
}

/// 렌더 HTML의 내부 링크에서 "있는 문서"를 모은다.
fn collect_existing_documents(rendered: &str, out: &mut std::collections::HashSet<String>) {
    for tag in rendered.split("<a ").skip(1) {
        let Some(tag) = tag.split('>').next() else {
            continue;
        };
        let Some(class) = attribute_of(tag, "class") else {
            continue;
        };
        if class != "wiki-link-internal" {
            continue; // `not-exist`가 붙었으면 없는 문서다
        }
        if let Some(title) = attribute_of(tag, "title") {
            out.insert(title.to_string());
        }
    }
}

/// 렌더 HTML의 `<img>`에서 `alt`(파일 이름)와 `src`(실제 URL) 짝을 모은다.
fn collect_file_urls(rendered: &str, out: &mut HashMap<String, String>) {
    for tag in rendered.split("<img ").skip(1) {
        let Some(tag) = tag.split('>').next() else {
            continue;
        };
        let (Some(alt), Some(source)) = (attribute_of(tag, "alt"), attribute_of(tag, "src")) else {
            continue;
        };
        let name = alt.strip_prefix("파일:").unwrap_or(alt);
        out.insert(name.to_string(), source.to_string());
    }
}

fn attribute_of<'tag>(tag: &'tag str, name: &str) -> Option<&'tag str> {
    let start = tag.find(&format!("{name}='"))? + name.len() + 2;
    let length = tag[start..].find('\'')?;
    Some(&tag[start..start + length])
}

impl Corpus {
    pub fn load(directory: &Path) -> std::io::Result<Corpus> {
        let mut cases = Vec::new();
        let mut sources = HashMap::new();
        let mut file_urls = HashMap::new();
        let mut existing_documents = std::collections::HashSet::new();
        for entry in std::fs::read_dir(directory)? {
            let path = entry?.path();
            if path.extension().is_none_or(|extension| extension != "namu") {
                continue;
            }
            let Some(slug) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let source = std::fs::read_to_string(&path)?;
            let document = unslug(slug);
            sources.insert(document.clone(), source.clone());

            let rendered_path = path.with_extension("html");
            if let Ok(rendered) = std::fs::read_to_string(&rendered_path) {
                collect_file_urls(&rendered, &mut file_urls);
                collect_existing_documents(&rendered, &mut existing_documents);
                cases.push(Case {
                    document,
                    source,
                    rendered,
                });
            }
        }
        cases.sort_by(|left, right| left.document.cmp(&right.document));
        Ok(Corpus {
            cases,
            sources,
            file_urls,
            existing_documents,
        })
    }

    /// 대조 대상 케이스. 틀 문서는 include 공급용이므로 그 자체를 대조하지는 않는다.
    ///
    /// `[date]`·`[pagecount]`처럼 값이 시점에 따라 달라지는 매크로는 문서를 통째로
    /// 버리지 않고, 그로 인한 차이만 `known-differences.txt`에서 걸러 낸다.
    pub fn comparable_cases(&self) -> Vec<&Case> {
        self.cases
            .iter()
            .filter(|case| !case.document.starts_with("틀:"))
            .collect()
    }

    pub fn render(&self, case: &Case, corpus: &Corpus) -> String {
        let document = namumark_parser::parse(&case.source);
        let context = CorpusContext {
            corpus,
            current: case.document.clone(),
        };
        let tree = build_render_tree(&document, &context);
        namumark_backend_namuwiki::namuwiki_markup(&tree).to_string()
    }
}

fn unslug(slug: &str) -> String {
    slug.replacen('_', ":", 1).replace('_', "/")
}

struct CorpusContext<'a> {
    corpus: &'a Corpus,
    current: String,
}

impl WikiContext for CorpusContext<'_> {
    fn document_exists(&self, title: &str) -> bool {
        self.corpus.existing_documents.contains(title)
    }

    fn current_title(&self) -> Option<String> {
        Some(self.current.clone())
    }

    fn include_source(&self, title: &str) -> Option<String> {
        if title == self.current {
            return None;
        }
        self.corpus.sources.get(title).cloned()
    }

    fn file_url(&self, file_name: &str) -> Option<String> {
        self.corpus.file_urls.get(file_name).cloned()
    }

    fn now(&self) -> Option<namumark_render::DateTime> {
        None
    }
}

pub struct Difference {
    pub expected: String,
    pub actual: String,
    pub context: String,
    /// 갈라진 자리. 앞뒤 조각을 펼쳐 보려면 여기서부터 본다.
    pub at: Position,
}

/// 차이가 난 자리의 조각 인덱스.
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub expected: usize,
    pub actual: usize,
}

impl Difference {
    /// 차이를 유형별로 묶는 키. 같은 원인에서 나온 차이를 한 줄로 모아 보기 위한 것이다.
    pub fn kind(&self) -> String {
        format!(
            "{} ↔ {}",
            summarize(&self.expected),
            summarize(&self.actual)
        )
    }
}

/// 조각을 유형 수준으로 요약한다: `<td style="...">` → `<td>`, 텍스트는 `"…"`.
fn summarize(rendered: &str) -> String {
    if !rendered.starts_with('<') {
        return "\"…\"".to_string();
    }
    let end = rendered
        .find(|character: char| character.is_whitespace() || character == '>')
        .unwrap_or(rendered.len());
    let head = &rendered[..end];
    if rendered[end..].starts_with(' ') {
        format!("{head} …>")
    } else {
        format!("{head}>")
    }
}

pub struct KnownDifferences {
    patterns: Vec<String>,
}

impl KnownDifferences {
    pub fn load(path: &Path) -> KnownDifferences {
        let patterns = std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_string)
            .collect();
        KnownDifferences { patterns }
    }

    /// 차이의 어느 쪽에든 등록된 패턴이 들어 있으면 알려진 차이로 본다.
    pub fn matches(&self, difference: &Difference) -> bool {
        self.patterns.iter().any(|pattern| {
            difference.expected.contains(pattern) || difference.actual.contains(pattern)
        })
    }
}

/// 두 구조 지문을 앞에서부터 맞춰 보며 어긋나는 지점을 모은다.
///
/// 한쪽이 삽입/삭제된 경우를 흡수하려고, 어긋나면 짧은 창 안에서 재동기화를 시도한다.
pub fn diff(expected: &[Fragment], actual: &[Fragment]) -> Vec<Difference> {
    const RESYNC_WINDOW: usize = 8;
    let mut differences = Vec::new();
    let mut left = 0;
    let mut right = 0;
    let mut recent = Vec::new();

    while left < expected.len() || right < actual.len() {
        match (expected.get(left), actual.get(right)) {
            (Some(expected_fragment), Some(actual_fragment))
                if expected_fragment == actual_fragment =>
            {
                if let Fragment::Open { name, .. } = expected_fragment {
                    recent.push(name.clone());
                    if recent.len() > 4 {
                        recent.remove(0);
                    }
                }
                left += 1;
                right += 1;
            }
            (Some(expected_fragment), Some(actual_fragment)) => {
                differences.push(Difference {
                    expected: expected_fragment.render(),
                    actual: actual_fragment.render(),
                    context: recent.join(" > "),
                    at: Position {
                        expected: left,
                        actual: right,
                    },
                });
                match resynchronize(expected, actual, left, right, RESYNC_WINDOW) {
                    Some((next_left, next_right)) => {
                        left = next_left;
                        right = next_right;
                    }
                    None => {
                        left += 1;
                        right += 1;
                    }
                }
            }
            (Some(expected_fragment), None) => {
                differences.push(Difference {
                    expected: expected_fragment.render(),
                    actual: "(없음)".to_string(),
                    context: recent.join(" > "),
                    at: Position {
                        expected: left,
                        actual: right,
                    },
                });
                left += 1;
            }
            (None, Some(actual_fragment)) => {
                differences.push(Difference {
                    expected: "(없음)".to_string(),
                    actual: actual_fragment.render(),
                    context: recent.join(" > "),
                    at: Position {
                        expected: left,
                        actual: right,
                    },
                });
                right += 1;
            }
            (None, None) => break,
        }
    }
    differences
}

/// 어긋난 지점 이후 가장 가까운 공통 조각을 찾아 양쪽 커서를 다시 맞춘다.
fn resynchronize(
    expected: &[Fragment],
    actual: &[Fragment],
    left: usize,
    right: usize,
    window: usize,
) -> Option<(usize, usize)> {
    for offset in 1..=window {
        for candidate in 0..=offset {
            let next_left = left + candidate;
            let next_right = right + offset - candidate;
            if expected.get(next_left)? == actual.get(next_right)? {
                return Some((next_left, next_right));
            }
        }
    }
    None
}
