use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("문서 계층 오류")]
    Document(#[from] wiki_document::DocumentError),

    #[error("검색 오류")]
    Search(#[from] wiki_search::SearchError),

    #[error("계정 오류")]
    Account(#[from] wiki_account::AccountError),

    #[error("권한 오류")]
    Authorization(#[from] wiki_authorization::AuthorizationError),

    #[error("토론 오류")]
    Discussion(#[from] wiki_discussion::DiscussionError),

    #[error("저장소 오류")]
    Database(#[from] sqlx::Error),

    #[error("업로드를 처리하지 못했다")]
    Upload,

    #[error("세션을 읽고 쓰지 못했다")]
    Session,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        // 내부 오류 내용은 사용자에게 노출하지 않는다. 이 오류가 나가는 경로는 전부
        // JSON API이므로 형식도 다른 오류와 같은 {"error": ...}로 맞춘다.
        eprintln!("요청 처리 실패: {self}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "internal" })),
        )
            .into_response()
    }
}
