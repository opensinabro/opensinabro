pub type Result<T> = std::result::Result<T, DiscussionError>;

#[derive(Debug, thiserror::Error)]
pub enum DiscussionError {
    #[error("토론 저장소 오류")]
    Database(#[from] sqlx::Error),

    #[error("문서 계층 오류")]
    Document(#[from] wiki_document::DocumentError),

    #[error("열거 값이 없다: {table}.{name} — 마이그레이션 시드를 확인할 것")]
    MissingEnumeration { table: &'static str, name: String },

    #[error("문서가 없다: {0}")]
    MissingDocument(String),
}
