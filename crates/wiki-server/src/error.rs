use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

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
        // 내부 오류 내용은 사용자에게 노출하지 않는다.
        eprintln!("요청 처리 실패: {self}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("<h1>500</h1><p>요청을 처리하지 못했습니다.</p>"),
        )
            .into_response()
    }
}
