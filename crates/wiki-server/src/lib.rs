//! HTTP 조립 계층.
//!
//! 유스케이스는 핸들러가 하위 크레이트 호출 몇 개를 이어 붙이는 얇은 조립이고,
//! 규칙과 데이터는 각 부분영역 크레이트가 소유한다 (docs/architecture.md).

mod account;
mod admin;
mod api;
mod browse;
mod discussion;
mod edit;
mod error;
mod file;
mod handler;
mod history;
mod operate;
mod proxy;
mod security;
mod session;
mod session_store;
mod state;

pub use error::ServerError;
pub use state::{AppState, open_database, open_state, rebuild_index};

use axum::Router;
use axum::routing::{get, post};
use session_store::PostgresSessionStore;
use tower_sessions::{Expiry, SessionManagerLayer};

/// docs/architecture.md의 URL 설계를 따르는 라우팅.
///
/// **화면은 전부 프론트엔드가 그린다.** axum이 직접 처리하는 것은 `/api/` 아래의 JSON
/// API와 `/file/`(바이너리 서빙)·`/style.css`(본문 어휘)뿐이고, 나머지는 fallback으로
/// Next.js에 넘긴다 — 화이트리스트가 아니라 fallback이라 화면이 늘어도 이 파일을
/// 고치지 않는다.
pub fn router(state: AppState) -> Router {
    // 세션 쿠키는 자바스크립트가 읽을 이유가 없고, 다른 사이트에서 따라오는 요청에는
    // 실리지 않아야 한다 (docs/architecture.md의 보안 표준).
    let sessions = SessionManagerLayer::new(PostgresSessionStore::new(state.pool.clone()))
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(30)));

    Router::new()
        // ── 셸이 쓰는 것 ────────────────────────────────────────────────
        .route("/api/session", get(api::session_api))
        .route("/api/csrf", get(edit::csrf_api))
        .route("/api/suggest", get(browse::suggest_titles))
        // ── 문서 읽기·편집 ──────────────────────────────────────────────
        .route("/api/w/{*title}", get(browse::document_api))
        .route("/api/raw/{*title}", get(handler::raw_api))
        .route("/api/category/{*title}", get(browse::category_api))
        .route("/api/backlink/{*title}", get(browse::backlink_api))
        .route("/api/preview", post(edit::preview_api))
        .route(
            "/api/edit/{*title}",
            get(edit::edit_api).post(edit::edit_submit_api),
        )
        // ── 역사 ────────────────────────────────────────────────────────
        .route("/api/history/{*title}", get(history::history_api))
        .route("/api/recent-changes", get(history::recent_changes_api))
        .route("/api/diff/{*title}", get(history::diff_api))
        .route(
            "/api/revert/{*title}",
            get(history::revert_api).post(history::revert_submit_api),
        )
        .route("/api/blame/{*title}", get(operate::blame_api))
        .route(
            "/api/hide-revision/{*title}",
            post(operate::hide_revision_api),
        )
        // ── 토론·편집요청 ───────────────────────────────────────────────
        .route(
            "/api/discuss/{*title}",
            get(discussion::document_threads_api).post(discussion::create_thread_api),
        )
        .route("/api/thread/{id}", get(discussion::view_thread_api))
        .route("/api/thread/{id}/comment", post(discussion::add_comment_api))
        .route("/api/thread/{id}/status", post(discussion::change_status_api))
        .route(
            "/api/recent-discussions",
            get(discussion::recent_discussions_api),
        )
        .route("/api/edit-requests", get(discussion::edit_requests_api))
        .route(
            "/api/edit-request/{id}",
            get(discussion::view_edit_request_api),
        )
        .route(
            "/api/edit-request/{id}/accept",
            post(discussion::accept_edit_request_api),
        )
        .route(
            "/api/edit-request/{id}/close",
            post(discussion::close_edit_request_api),
        )
        .route(
            "/api/new-edit-request/{*title}",
            post(discussion::submit_edit_request_api),
        )
        // ── 특수 페이지 ─────────────────────────────────────────────────
        .route("/api/search", get(handler::search_api))
        .route("/api/random", get(handler::random_api))
        .route("/api/license", get(handler::license_api))
        .route("/api/needed-pages", get(handler::needed_pages_api))
        .route("/api/orphaned-pages", get(browse::orphaned_pages_api))
        .route(
            "/api/uncategorized-pages",
            get(browse::uncategorized_pages_api),
        )
        .route("/api/old-pages", get(browse::old_pages_api))
        .route("/api/pages-by-length", get(browse::pages_by_length_api))
        .route("/api/starred", get(browse::starred_documents_api))
        .route("/api/star/{*title}", post(browse::toggle_star_api))
        .route("/api/notifications", get(browse::notifications_api))
        .route(
            "/api/notifications/read",
            post(browse::notifications_read_api),
        )
        // ── 계정 ────────────────────────────────────────────────────────
        .route("/api/login", post(account::login_api))
        .route("/api/logout", post(account::logout_api))
        .route("/api/signup", post(account::signup_api))
        .route("/api/verify", get(account::verify_api))
        // ── 운영·관리 ───────────────────────────────────────────────────
        .route(
            "/api/move/{*title}",
            get(operate::move_api).post(operate::move_submit_api),
        )
        .route(
            "/api/delete/{*title}",
            get(operate::delete_api).post(operate::delete_submit_api),
        )
        .route(
            "/api/upload",
            get(file::upload_api).post(file::upload_submit_api),
        )
        .route("/api/block-history", get(admin::block_history_api))
        .route(
            "/api/users/{name}/contributions",
            get(admin::contributions_api),
        )
        .route(
            "/api/admin/grant",
            get(admin::grant_api).post(admin::grant_submit_api),
        )
        .route(
            "/api/admin/config",
            get(operate::config_api).post(operate::config_submit_api),
        )
        .route(
            "/api/admin/batch-revert",
            get(operate::batch_revert_api).post(operate::batch_revert_submit_api),
        )
        // ── axum이 직접 내주는 것 ───────────────────────────────────────
        .route("/file/{*name}", get(file::serve_file))
        .route("/style.css", get(handler::stylesheet))
        // 화면은 전부 프론트엔드가 그린다. 화이트리스트가 아니라 fallback이므로
        // 새 화면이 생겨도 이 파일을 고치지 않는다.
        .fallback(proxy::forward)
        .layer(sessions)
        .with_state(state)
}
