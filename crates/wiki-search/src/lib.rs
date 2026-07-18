//! 전문 검색 색인.
//!
//! 다른 위키 크레이트를 참조하지 않는다 — 색인할 내용은 호출자가 문자열로 공급하고,
//! 여기서는 색인 구축과 질의만 맡는다.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{
    Field, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions, Value,
};
use tantivy::tokenizer::NgramTokenizer;
use tantivy::{Index, IndexWriter, TantivyDocument, Term, doc};

pub type Result<T> = std::result::Result<T, SearchError>;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("검색 색인 오류")]
    Index(#[from] tantivy::TantivyError),

    #[error("질의를 해석할 수 없다")]
    Query(#[from] tantivy::query::QueryParserError),

    #[error("색인 잠금이 깨졌다")]
    LockPoisoned,
}

/// 한국어는 어절이 조사로 붙어 공백 분리만으로는 부분 일치가 안 된다. 형태소 사전을
/// 두면 정확도가 오르지만 외부 사전 파일이 생겨 "단일 바이너리로 곧장 실행"이라는
/// 요구사항(docs/design/06)과 어긋나므로, 사전 없이 동작하는 문자 n-gram을 쓴다.
const KOREAN_TOKENIZER: &str = "korean_ngram";

pub struct SearchHit {
    pub namespace: String,
    pub title: String,
    pub score: f32,
}

struct Fields {
    /// 삭제·갱신용 정확 일치 키. 검색용 title은 n-gram으로 쪼개져 term 삭제가 안 되므로
    /// 토큰화하지 않는 필드를 따로 둔다.
    key: Field,
    namespace: Field,
    title: Field,
    content: Field,
}

pub struct SearchIndex {
    index: Index,
    writer: Mutex<IndexWriter>,
    fields: Fields,
}

impl SearchIndex {
    /// 색인 디렉터리를 열거나 새로 만든다.
    pub fn open_or_create(directory: &Path) -> Result<Self> {
        let mut builder = Schema::builder();

        let searchable = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer(KOREAN_TOKENIZER)
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        let key = builder.add_text_field("key", STRING);
        let namespace = builder.add_text_field("namespace", STRING | STORED);
        let title = builder.add_text_field("title", searchable.clone());
        let content = builder.add_text_field("content", searchable);
        let schema = builder.build();

        std::fs::create_dir_all(directory).map_err(tantivy::TantivyError::from)?;
        let index = Index::open_or_create(
            tantivy::directory::MmapDirectory::open(directory)
                .map_err(tantivy::TantivyError::from)?,
            schema,
        )?;

        index
            .tokenizers()
            .register(KOREAN_TOKENIZER, NgramTokenizer::new(2, 3, false)?);

        let writer = index.writer(50_000_000)?;

        Ok(Self {
            index,
            writer: Mutex::new(writer),
            fields: Fields {
                key,
                namespace,
                title,
                content,
            },
        })
    }

    fn writer(&self) -> Result<MutexGuard<'_, IndexWriter>> {
        self.writer.lock().map_err(|_| SearchError::LockPoisoned)
    }

    /// 문서를 색인에 반영한다 (같은 제목의 옛 항목은 지운다).
    pub fn put(&self, namespace: &str, title: &str, content: &str) -> Result<()> {
        let writer = self.writer()?;
        writer.delete_term(Term::from_field_text(
            self.fields.key,
            &qualified(namespace, title),
        ));
        writer.add_document(doc!(
            self.fields.key => qualified(namespace, title),
            self.fields.namespace => namespace,
            self.fields.title => title,
            self.fields.content => content,
        ))?;
        Ok(())
    }

    pub fn remove(&self, namespace: &str, title: &str) -> Result<()> {
        let writer = self.writer()?;
        writer.delete_term(Term::from_field_text(
            self.fields.key,
            &qualified(namespace, title),
        ));
        Ok(())
    }

    /// 지금까지의 변경을 확정한다 — 이 호출 뒤에야 질의에 보인다.
    pub fn commit(&self) -> Result<()> {
        self.writer()?.commit()?;
        Ok(())
    }

    /// 색인에 아무것도 없는가 — 서버가 시작할 때 재색인이 필요한지 판단한다.
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.index.reader()?.searcher().num_docs() == 0)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        let parser =
            QueryParser::for_index(&self.index, vec![self.fields.title, self.fields.content]);
        let parsed = parser.parse_query(query)?;

        let found = searcher.search(&parsed, &TopDocs::with_limit(limit).order_by_score())?;
        let mut hits = Vec::with_capacity(found.len());

        for (score, address) in found {
            let document: TantivyDocument = searcher.doc(address)?;
            hits.push(SearchHit {
                namespace: text_of(&document, self.fields.namespace),
                title: text_of(&document, self.fields.title),
                score,
            });
        }

        Ok(hits)
    }
}

/// 이름공간이 다른 같은 이름을 구별하는 색인 키.
fn qualified(namespace: &str, title: &str) -> String {
    format!("{namespace}:{title}")
}

fn text_of(document: &TantivyDocument, field: Field) -> String {
    document
        .get_first(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 색인한_문서를_부분_문자열로_찾는다() {
        let directory = tempdir("부분-문자열");
        let index = SearchIndex::open_or_create(&directory).expect("색인 생성");

        index
            .put("문서", "나무위키", "나무위키는 한국어 위키입니다.")
            .expect("색인 추가");
        index.commit().expect("커밋");

        // 조사가 붙은 어절("위키입니다") 안의 부분 문자열도 찾아야 한다.
        let hits = index.search("위키", 10).expect("검색");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "나무위키");
        assert_eq!(hits[0].namespace, "문서");

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn 지운_문서는_검색되지_않는다() {
        let directory = tempdir("삭제");
        let index = SearchIndex::open_or_create(&directory).expect("색인 생성");

        index.put("문서", "임시", "지워질 내용").expect("색인 추가");
        index.commit().expect("커밋");
        index.remove("문서", "임시").expect("색인 삭제");
        index.commit().expect("커밋");

        assert!(index.search("지워질", 10).expect("검색").is_empty());

        std::fs::remove_dir_all(&directory).ok();
    }

    /// 테스트마다 다른 디렉터리를 준다.
    ///
    /// 색인 쓰기 잠금은 디렉터리당 하나뿐이라, 시각만으로 이름을 지으면 나란히 도는
    /// 테스트가 같은 이름을 잡아 잠금 충돌이 난다. 프로세스와 순번을 함께 섞는다.
    fn tempdir(label: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        let path = std::env::temp_dir().join(format!(
            "wiki-search-test-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir_all(&path).expect("임시 디렉터리");
        path
    }
}
