pub type Result<T> = std::result::Result<T, DocumentError>;

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("문서 저장소 오류")]
    Database(#[from] sqlx::Error),

    #[error("계정 오류")]
    Account(#[from] wiki_account::AccountError),

    #[error("파일을 저장하지 못했다")]
    FileStorage(#[source] std::io::Error),

    #[error("알 수 없는 이름공간: {0}")]
    UnknownNamespace(String),

    #[error("열거 값이 없다: {table}.{name} — 마이그레이션 시드를 확인할 것")]
    MissingEnumeration { table: &'static str, name: String },
}
