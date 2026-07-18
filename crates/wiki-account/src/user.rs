use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::Result;

/// 사용자의 내부 식별자. 외부에는 [`WikiUser::external_id`]나 이름으로만 나간다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserIdentifier(i64);

impl UserIdentifier {
    pub fn from_raw(value: i64) -> Self {
        Self(value)
    }

    pub fn as_raw(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct WikiUser {
    pub identifier: UserIdentifier,
    pub external_id: Uuid,
    pub name: String,
    pub is_system: bool,
}

/// 덤프 임포트·자동 편집의 주체가 되는 시스템 사용자를 확보한다.
///
/// 인증 수단이 없어 로그인할 수 없다 — 기계가 남긴 리비전도 사람 리비전과 같은
/// actor 참조를 쓰게 하려고 존재한다.
pub async fn ensure_system_user(pool: &PgPool, name: &str) -> Result<WikiUser> {
    if let Some(user) = find_user_by_name(pool, name).await? {
        return Ok(user);
    }

    let external_id = Uuid::new_v4();
    let (id,) = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO wiki_user (external_id, name, is_system, created_at)
         VALUES ($1, $2, true, $3)
         RETURNING id",
    )
    .bind(external_id)
    .bind(name)
    .bind(Utc::now())
    .fetch_one(pool)
    .await?;

    Ok(WikiUser {
        identifier: UserIdentifier(id),
        external_id,
        name: name.to_owned(),
        is_system: true,
    })
}

/// 가입으로 만들어지는 보통 사용자.
pub async fn create_user(pool: &PgPool, name: &str) -> Result<WikiUser> {
    let external_id = Uuid::new_v4();
    let (id,) = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO wiki_user (external_id, name, is_system, created_at)
         VALUES ($1, $2, false, $3)
         RETURNING id",
    )
    .bind(external_id)
    .bind(name)
    .bind(Utc::now())
    .fetch_one(pool)
    .await?;

    Ok(WikiUser {
        identifier: UserIdentifier(id),
        external_id,
        name: name.to_owned(),
        is_system: false,
    })
}

pub async fn find_user_by_external_id(
    pool: &PgPool,
    external_id: Uuid,
) -> Result<Option<WikiUser>> {
    let row = sqlx::query_as::<_, (i64, Uuid, String, bool)>(
        "SELECT id, external_id, name, is_system FROM wiki_user WHERE external_id = $1",
    )
    .bind(external_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, external_id, name, is_system)| WikiUser {
        identifier: UserIdentifier(id),
        external_id,
        name,
        is_system,
    }))
}

pub async fn find_user_by_name(pool: &PgPool, name: &str) -> Result<Option<WikiUser>> {
    let row = sqlx::query_as::<_, (i64, Uuid, String, bool)>(
        "SELECT id, external_id, name, is_system FROM wiki_user WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, external_id, name, is_system)| WikiUser {
        identifier: UserIdentifier(id),
        external_id,
        name,
        is_system,
    }))
}
