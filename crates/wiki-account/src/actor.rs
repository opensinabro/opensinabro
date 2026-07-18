use sqlx::PgPool;

use crate::{Result, UserIdentifier};

/// 리비전·토론·권한이 참조하는 행위 주체의 내부 식별자.
///
/// 로그인 사용자와 IP 사용자를 한 타입으로 가리켜, 참조하는 쪽이 둘을 구분하지 않고
/// 같은 외래키를 쓴다. 내부 전용이라 URL·HTML로 나가지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorIdentifier(i64);

impl ActorIdentifier {
    pub fn from_raw(value: i64) -> Self {
        Self(value)
    }

    pub fn as_raw(self) -> i64 {
        self.0
    }
}

/// 사용자의 actor를 확보한다 (없으면 만든다).
pub async fn ensure_user_actor(pool: &PgPool, user: UserIdentifier) -> Result<ActorIdentifier> {
    let raw = user.as_raw();
    if let Some((id,)) = sqlx::query_as::<_, (i64,)>("SELECT id FROM actor WHERE user_id = $1")
        .bind(raw)
        .fetch_optional(pool)
        .await?
    {
        return Ok(ActorIdentifier(id));
    }

    let (id,) = sqlx::query_as::<_, (i64,)>("INSERT INTO actor (user_id) VALUES ($1) RETURNING id")
        .bind(raw)
        .fetch_one(pool)
        .await?;
    Ok(ActorIdentifier(id))
}

/// 비로그인 편집자의 actor를 확보한다 (없으면 만든다).
pub async fn ensure_ip_actor(pool: &PgPool, ip_address: &str) -> Result<ActorIdentifier> {
    if let Some((id,)) = sqlx::query_as::<_, (i64,)>("SELECT id FROM actor WHERE ip_address = $1")
        .bind(ip_address)
        .fetch_optional(pool)
        .await?
    {
        return Ok(ActorIdentifier(id));
    }

    let (id,) =
        sqlx::query_as::<_, (i64,)>("INSERT INTO actor (ip_address) VALUES ($1) RETURNING id")
            .bind(ip_address)
            .fetch_one(pool)
            .await?;
    Ok(ActorIdentifier(id))
}
