use chrono::{Duration, Utc};
use sqlx::PgPool;

use crate::{AccountError, Result, UserIdentifier};

/// 검증이 무엇을 위한 것인가. DB의 `verification_purpose` 열거와 짝이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationPurpose {
    Signup,
    PasswordReset,
    EmailChange,
    Device,
}

impl VerificationPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Signup => "signup",
            Self::PasswordReset => "password_reset",
            Self::EmailChange => "email_change",
            Self::Device => "device",
        }
    }
}

/// 발급한 검증 토큰. 원문은 이 순간에만 존재하고 DB에는 해시만 남는다 —
/// 저장소가 새더라도 링크를 되살릴 수 없게 한다.
pub struct IssuedVerification {
    pub token: String,
    pub credential_id: Option<i64>,
}

fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    crate::to_hex(&Sha256::digest(token.as_bytes()))
}

fn generate_token() -> String {
    crate::to_hex(&crate::random_bytes::<32>())
}

/// 검증 토큰을 발급한다. 유효 기간이 지나면 쓸 수 없다.
pub async fn issue(
    pool: &PgPool,
    user: UserIdentifier,
    credential_id: Option<i64>,
    purpose: VerificationPurpose,
    valid_for: Duration,
) -> Result<IssuedVerification> {
    let (purpose_id,) =
        sqlx::query_as::<_, (i64,)>("SELECT id FROM verification_purpose WHERE name = $1")
            .bind(purpose.as_str())
            .fetch_optional(pool)
            .await?
            .ok_or(AccountError::MissingVerificationPurpose(purpose.as_str()))?;

    let token = generate_token();

    sqlx::query(
        "INSERT INTO user_verification
           (user_id, credential_id, purpose_id, token_hash, expires_at, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(user.as_raw())
    .bind(credential_id)
    .bind(purpose_id)
    .bind(hash_token(&token))
    .bind(Utc::now() + valid_for)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(IssuedVerification {
        token,
        credential_id,
    })
}

/// 토큰을 한 번만 쓰이게 소비한다. 이미 쓴 토큰과 기한이 지난 토큰은 받지 않는다.
pub async fn consume(
    pool: &PgPool,
    token: &str,
    purpose: VerificationPurpose,
) -> Result<Option<IssuedVerification>> {
    let row = sqlx::query_as::<_, (i64, Option<i64>)>(
        "UPDATE user_verification
         SET consumed_at = $1
         WHERE token_hash = $2
           AND consumed_at IS NULL
           AND expires_at > $1
           AND purpose_id = (SELECT id FROM verification_purpose WHERE name = $3)
         RETURNING id, credential_id",
    )
    .bind(Utc::now())
    .bind(hash_token(token))
    .bind(purpose.as_str())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(_, credential_id)| IssuedVerification {
        token: token.to_owned(),
        credential_id,
    }))
}
