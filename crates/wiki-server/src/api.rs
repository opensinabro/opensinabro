//! JSON API가 함께 쓰는 표현.
//!
//! 프론트엔드가 화면을 그리는 데 필요한 만큼만 담고, 내부 식별자는 내보내지 않는다
//! (docs/design/08의 내부·외부 식별자 분리).

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use uuid::Uuid;
use wiki_document::RevisionRecord;

/// 권한이 없을 때의 응답. 무엇이 막혔는지는 알리지 않는다.
pub fn forbidden() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({ "error": "forbidden" })),
    )
        .into_response()
}

pub fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "not_found" })),
    )
        .into_response()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionSummary {
    pub id: Uuid,
    pub sequence: i64,
    pub kind: &'static str,
    pub author: String,
    pub comment: String,
    pub content_bytes: i64,
    pub created_at: String,
}

impl From<&RevisionRecord> for RevisionSummary {
    fn from(record: &RevisionRecord) -> Self {
        Self {
            id: record.external_id,
            sequence: record.sequence,
            kind: record.kind.as_str(),
            author: record.author.clone(),
            comment: record.comment.clone(),
            content_bytes: record.content_bytes,
            created_at: record.created_at.to_rfc3339(),
        }
    }
}
