use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tower_sessions::SessionStore;
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::{Error, Result};

/// 세션을 PostgreSQL에 담는 저장소.
///
/// 세션 크레이트가 제공하는 sqlx 저장소는 우리와 다른 sqlx 판을 쓰므로 직접 구현한다
/// — 테이블 하나에 넣고 빼는 일이라 구현이 짧고, 스키마도 우리 마이그레이션이 쥔다.
#[derive(Debug, Clone)]
pub struct PostgresSessionStore {
    pool: PgPool,
}

impl PostgresSessionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn backend(error: sqlx::Error) -> Error {
    Error::Backend(error.to_string())
}

fn encode(record: &Record) -> Result<Vec<u8>> {
    rmp_serde::to_vec(record).map_err(|error| Error::Encode(error.to_string()))
}

fn decode(bytes: &[u8]) -> Result<Record> {
    rmp_serde::from_slice(bytes).map_err(|error| Error::Decode(error.to_string()))
}

fn expiry(record: &Record) -> DateTime<Utc> {
    DateTime::from_timestamp(record.expiry_date.unix_timestamp(), 0).unwrap_or_else(Utc::now)
}

#[async_trait::async_trait]
impl SessionStore for PostgresSessionStore {
    async fn save(&self, record: &Record) -> Result<()> {
        sqlx::query(
            "INSERT INTO session (id, data, expires_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE
               SET data = excluded.data, expires_at = excluded.expires_at",
        )
        .bind(record.id.to_string())
        .bind(encode(record)?)
        .bind(expiry(record))
        .execute(&self.pool)
        .await
        .map_err(backend)?;

        Ok(())
    }

    async fn load(&self, id: &Id) -> Result<Option<Record>> {
        let row = sqlx::query_as::<_, (Vec<u8>,)>(
            "SELECT data FROM session WHERE id = $1 AND expires_at > $2",
        )
        .bind(id.to_string())
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;

        row.map(|(data,)| decode(&data)).transpose()
    }

    async fn delete(&self, id: &Id) -> Result<()> {
        sqlx::query("DELETE FROM session WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(backend)?;

        Ok(())
    }
}
