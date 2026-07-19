use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

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

impl ServerError {
    /// 어느 계층에서 났는지까지만 알린다. 화면은 이걸로 "문서를 읽는 중"인지
    /// "검색하는 중"인지를 말할 수 있고, 그 이상의 내부 사정은 나가지 않는다.
    fn token(&self) -> &'static str {
        match self {
            Self::Document(_) => "document_failed",
            Self::Search(_) => "search_failed",
            Self::Account(_) => "account_failed",
            Self::Authorization(_) => "authorization_failed",
            Self::Discussion(_) => "discussion_failed",
            Self::Database(_) => "storage_failed",
            Self::Upload => "upload_failed",
            Self::Session => "session_failed",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            // 읽을 수 없는 multipart는 서버가 고장 난 것이 아니라 요청이 어긋난 것이다.
            // 500으로 내면 다시 보내도 소용없다는 뜻이 되어, 고쳐 올릴 수 있는 사람을
            // 돌려세운다.
            Self::Upload => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        // 내부 오류 내용은 사용자에게 노출하지 않는다. 대신 추적 번호를 화면과 기록
        // 양쪽에 같이 남긴다 — 이것이 없으면 신고받은 화면과 서버 기록을 이을 길이 없다.
        let trace = Uuid::new_v4();
        eprintln!("요청 처리 실패 [{trace}]: {self}");

        // 이 오류가 나가는 경로는 전부 JSON API이므로 형식도 다른 오류와 같은
        // {"error": ...}로 맞춘다.
        (
            self.status(),
            Json(serde_json::json!({
                "error": self.token(),
                "trace": trace.to_string(),
            })),
        )
            .into_response()
    }
}
