pub type Result<T> = std::result::Result<T, AccountError>;

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("계정 저장소 오류")]
    Database(#[from] sqlx::Error),

    #[error("비밀번호를 해시하지 못했다")]
    PasswordHash,

    #[error("인증 수단 종류가 없다: {0} — 마이그레이션 시드를 확인할 것")]
    MissingCredentialKind(&'static str),

    #[error("알림 종류가 없다: {0} — 마이그레이션 시드를 확인할 것")]
    MissingNotificationKind(&'static str),

    #[error("검증 목적이 없다: {0} — 마이그레이션 시드를 확인할 것")]
    MissingVerificationPurpose(&'static str),
}
