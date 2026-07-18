//! 문서와 그 역사를 소유한다.
//!
//! 나무마크 렌더 파이프라인과의 접점도 여기다 — 렌더러가 묻는 것(링크 존재·include
//! 원문·파일 URL)이 곧 이 크레이트의 데이터이기 때문이다.

mod error;
mod file;
mod reference;
mod render;
mod repository;
mod revise;
mod title;

pub use error::{DocumentError, Result};
pub use file::{
    ContentHash, SUPPORTED_MEDIA_TYPES, StoredFile, attach_file, is_supported_media_type, licenses,
    read_file, store_content,
};
pub use reference::{ReferenceKind, ReferenceTarget};
pub use render::{RenderedDocument, render_document};
pub use repository::{
    BlameLine, DocumentIdentifier, DocumentRecord, RecentChange, RevisionKind, RevisionRecord,
    backlinks, blame, content_before_author, delete_document, document_titles,
    documents_last_edited_by, find_document, is_starred, latest_revision, move_document,
    orphaned_titles, random_title, read_source, recent_changes, record_revision,
    replace_references, revision_content, revision_history, set_revision_hidden, stale_titles,
    starred_titles, subscribers, titles_by_length, titles_missing, titles_starting_with,
    toggle_star, uncategorized_titles,
};
pub use revise::{DiffLine, DiffLineKind, MergeOutcome, diff_lines, merge_edits};
pub use title::{DocumentTitle, Namespace};
