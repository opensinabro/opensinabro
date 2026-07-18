//! 문서를 둘러싼 대화를 소유한다 — 토론 스레드와 편집요청.
//!
//! 상태 변경·주제 변경 같은 관리 조작도 스레드 안의 항목으로 남긴다. the seed가
//! 타임라인에 그것들을 끼워 보이기 때문이고, 그래야 "무슨 일이 있었나"가 한 줄기로
//! 읽힌다 (docs/architecture.md).

mod edit_request;
mod error;
mod thread;

pub use edit_request::{
    EditRequest, EditRequestStatus, accept_edit_request, close_edit_request, edit_request_by_id,
    open_edit_requests, submit_edit_request,
};
pub use error::{DiscussionError, Result};
pub use thread::{
    Comment, CommentKind, Thread, ThreadStatus, add_comment, change_status, change_topic,
    create_thread, hide_comment, recent_threads, thread_by_id, thread_comments, threads_of,
};
