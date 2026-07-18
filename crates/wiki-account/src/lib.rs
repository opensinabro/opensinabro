//! 위키의 행위 주체(actor)와 인증을 소유한다.
//!
//! 리비전·토론·권한이 모두 [`ActorIdentifier`]로 "누가 했는가"를 가리키므로, 이
//! 크레이트는 다른 위키 크레이트를 참조하지 않는 의존 그래프의 뿌리다.

mod actor;
mod credential;
mod error;
mod notification;
mod user;
mod verification;

/// 암호학적으로 안전한 임의 바이트. 소금·토큰이 이것으로 만들어진다.
pub(crate) fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    for byte in &mut bytes {
        *byte = rand::random();
    }
    bytes
}

/// 바이트를 소문자 16진 문자열로. 토큰 표기와 해시 표기에 쓴다.
pub(crate) fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

pub use actor::{ActorIdentifier, ensure_ip_actor, ensure_user_actor};
pub use credential::{
    CredentialKind, add_email, authenticate, email_taken, mark_verified, record_login_attempt,
    set_password,
};
pub use error::{AccountError, Result};
pub use notification::{
    Notification, NotificationKind, mark_all_read, notifications, notify, unread_count,
};
pub use user::{
    UserIdentifier, WikiUser, create_user, ensure_system_user, find_user_by_external_id,
    find_user_by_name,
};
pub use verification::{IssuedVerification, VerificationPurpose, consume, issue};
