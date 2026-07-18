use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use chrono::Utc;
use sqlx::PgPool;

use crate::{AccountError, Result, UserIdentifier, WikiUser, find_user_by_name};

/// 인증 수단의 종류. DB의 `credential_kind` 열거와 짝이다.
///
/// 이메일도 여기 한 종류다 — 가입 검증·복구·수신이 모두 인증 흐름의 역할이라
/// 별도 테이블을 두지 않는다 (docs/design/08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    Password,
    Totp,
    Passkey,
    OAuth,
    Email,
}

impl CredentialKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Totp => "totp",
            Self::Passkey => "passkey",
            Self::OAuth => "oauth",
            Self::Email => "email",
        }
    }
}

async fn kind_id(pool: &PgPool, kind: CredentialKind) -> Result<i64> {
    sqlx::query_as::<_, (i64,)>("SELECT id FROM credential_kind WHERE name = $1")
        .bind(kind.as_str())
        .fetch_optional(pool)
        .await?
        .map(|(id,)| id)
        .ok_or(AccountError::MissingCredentialKind(kind.as_str()))
}

/// 비밀번호를 해시해 저장한다. 사용자당 하나라는 제약은 부분 유니크 인덱스가 지킨다.
pub async fn set_password(pool: &PgPool, user: UserIdentifier, password: &str) -> Result<()> {
    let salt = SaltString::encode_b64(&crate::random_bytes::<16>())
        .map_err(|_| AccountError::PasswordHash)?;
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AccountError::PasswordHash)?
        .to_string();

    let kind = kind_id(pool, CredentialKind::Password).await?;

    sqlx::query(
        "INSERT INTO user_credential (user_id, kind_id, secret, created_at)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id, kind_id) WHERE is_primary DO NOTHING",
    )
    .bind(user.as_raw())
    .bind(kind)
    .bind(&hash)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

/// 이름과 비밀번호가 맞으면 그 사용자를 돌려준다.
///
/// 이름이 없든 비밀번호가 틀리든 같은 결과(None)를 내어, 어느 쪽이 틀렸는지
/// 밖에서 알 수 없게 한다.
pub async fn authenticate(pool: &PgPool, name: &str, password: &str) -> Result<Option<WikiUser>> {
    let Some(user) = find_user_by_name(pool, name).await? else {
        return Ok(None);
    };

    // 기계 주체는 로그인 수단을 갖지 않는다.
    if user.is_system {
        return Ok(None);
    }

    let kind = kind_id(pool, CredentialKind::Password).await?;
    let stored = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, secret FROM user_credential
         WHERE user_id = $1 AND kind_id = $2 AND secret IS NOT NULL",
    )
    .bind(user.identifier.as_raw())
    .bind(kind)
    .fetch_optional(pool)
    .await?;

    let Some((credential_id, hash)) = stored else {
        return Ok(None);
    };

    let Ok(parsed) = PasswordHash::new(&hash) else {
        return Ok(None);
    };

    if Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_err()
    {
        return Ok(None);
    }

    sqlx::query("UPDATE user_credential SET last_used_at = $1 WHERE id = $2")
        .bind(Utc::now())
        .bind(credential_id)
        .execute(pool)
        .await?;

    Ok(Some(user))
}

/// 이메일 주소를 인증 수단으로 등록한다 (아직 검증 전).
pub async fn add_email(pool: &PgPool, user: UserIdentifier, email: &str) -> Result<i64> {
    let kind = kind_id(pool, CredentialKind::Email).await?;

    let (id,) = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO user_credential (user_id, kind_id, identifier, is_primary, created_at)
         VALUES ($1, $2, $3, true, $4)
         RETURNING id",
    )
    .bind(user.as_raw())
    .bind(kind)
    .bind(email)
    .bind(Utc::now())
    .fetch_one(pool)
    .await?;

    Ok(id)
}

/// 이 주소를 이미 누가 쓰고 있는가 — 가입 폼이 미리 걸러 준다.
pub async fn email_taken(pool: &PgPool, email: &str) -> Result<bool> {
    let kind = kind_id(pool, CredentialKind::Email).await?;
    let found = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM user_credential WHERE kind_id = $1 AND identifier = $2",
    )
    .bind(kind)
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(found.is_some())
}

/// 검증이 끝난 자격 증명에 표시를 남긴다.
pub async fn mark_verified(pool: &PgPool, credential_id: i64) -> Result<()> {
    sqlx::query("UPDATE user_credential SET verified_at = $1 WHERE id = $2")
        .bind(Utc::now())
        .bind(credential_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 로그인 시도를 남긴다 — 성공·실패 모두가 요청 제한과 다중 계정 검사의 자료다.
pub async fn record_login_attempt(
    pool: &PgPool,
    user: UserIdentifier,
    ip_address: &str,
    user_agent: &str,
    succeeded: bool,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO login_record (user_id, ip_address, user_agent, succeeded, created_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user.as_raw())
    .bind(ip_address)
    .bind(user_agent)
    .bind(succeeded)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}
