//! HTTP 조립 계층.
//!
//! 유스케이스는 핸들러가 하위 크레이트 호출 몇 개를 이어 붙이는 얇은 조립이고,
//! 규칙과 데이터는 각 부분영역 크레이트가 소유한다 (docs/design/07).

mod account;
mod admin;
mod browse;
mod discussion;
mod edit;
mod error;
mod file;
mod handler;
mod history;
mod operate;
mod security;
mod session;
mod session_store;
mod state;
mod template;

pub use error::ServerError;
pub use state::{AppState, open_database, open_state, rebuild_index};

use axum::Router;
use axum::routing::{get, post};
use session_store::PostgresSessionStore;
use tower_sessions::{Expiry, SessionManagerLayer};

/// docs/design/07의 URL 설계를 그대로 옮긴 라우팅.
///
/// 문서 동작은 동사 접두사(`/w/`, `/raw/`), 그 밖은 소문자 kebab-case다.
pub fn router(state: AppState) -> Router {
    // 세션 쿠키는 자바스크립트가 읽을 이유가 없고, 다른 사이트에서 따라오는 요청에는
    // 실리지 않아야 한다 (docs/design/06 보안 표준).
    let sessions = SessionManagerLayer::new(PostgresSessionStore::new(state.pool.clone()))
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(30)));

    Router::new()
        .route("/", get(handler::index))
        .route(
            "/login",
            get(account::login_form).post(account::login_submit),
        )
        .route("/logout", post(account::logout_submit))
        .route(
            "/signup",
            get(account::signup_form).post(account::signup_submit),
        )
        .route("/verify", get(account::verify))
        .route("/w/{*title}", get(handler::view))
        .route("/raw/{*title}", get(handler::raw))
        .route(
            "/edit/{*title}",
            get(edit::edit_form).post(edit::edit_submit),
        )
        .route("/history/{*title}", get(history::history))
        .route("/diff/{*title}", get(history::diff))
        .route(
            "/revert/{*title}",
            get(history::revert_form).post(history::revert_submit),
        )
        .route("/recent-changes", get(history::recent_changes))
        .route(
            "/discuss/{*title}",
            get(discussion::document_threads).post(discussion::create_thread),
        )
        .route("/thread/{id}", get(discussion::view_thread))
        .route("/thread/{id}/comment", post(discussion::add_comment))
        .route("/thread/{id}/status", post(discussion::change_status))
        .route("/recent-discussions", get(discussion::recent_discussions))
        .route("/edit-requests", get(discussion::edit_requests))
        .route("/edit-request/{id}", get(discussion::view_edit_request))
        .route(
            "/edit-request/{id}/accept",
            post(discussion::accept_edit_request),
        )
        .route(
            "/edit-request/{id}/close",
            post(discussion::close_edit_request),
        )
        .route(
            "/new-edit-request/{*title}",
            post(discussion::submit_edit_request),
        )
        .route("/search", get(handler::search))
        .route("/random", get(handler::random))
        .route("/needed-pages", get(handler::needed_pages))
        .route("/orphaned-pages", get(browse::orphaned_pages))
        .route("/uncategorized-pages", get(browse::uncategorized_pages))
        .route("/old-pages", get(browse::old_pages))
        .route("/shortest-pages", get(browse::pages_by_length))
        .route("/longest-pages", get(browse::pages_by_length))
        .route("/backlink/{*title}", get(browse::backlinks))
        .route("/starred", get(browse::starred_documents))
        .route("/star/{*title}", post(browse::toggle_star))
        .route("/notifications", get(browse::notifications))
        .route("/api/suggest", get(browse::suggest_titles))
        .route("/api/w/{*title}", get(browse::document_api))
        .route(
            "/move/{*title}",
            get(operate::move_form).post(operate::move_submit),
        )
        .route(
            "/delete/{*title}",
            get(operate::delete_form).post(operate::delete_submit),
        )
        .route("/blame/{*title}", get(operate::blame))
        .route("/hide-revision/{*title}", post(operate::hide_revision))
        .route("/upload", get(file::upload_form).post(file::upload_submit))
        .route("/file/{*name}", get(file::serve_file))
        .route("/block-history", get(admin::block_history))
        .route(
            "/admin/config",
            get(operate::config_form).post(operate::config_submit),
        )
        .route(
            "/admin/batch-revert",
            get(operate::batch_revert_form).post(operate::batch_revert_submit),
        )
        .route(
            "/admin/grant",
            get(admin::grant_form).post(admin::grant_submit),
        )
        .route("/users/{name}", get(admin::user_profile))
        .route("/users/{name}/contributions", get(admin::contributions))
        .route("/license", get(handler::license))
        .route("/style.css", get(handler::stylesheet))
        .layer(sessions)
        .with_state(state)
}
