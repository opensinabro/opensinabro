use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use uuid::Uuid;

/// CSRF 토큰을 담는 쿠키 이름.
const CSRF_COOKIE: &str = "csrf_token";

/// 상태를 바꾸는 폼에 쓰는 CSRF 방어 (docs/design/06 보안 표준).
///
/// 세션 저장소가 아직 없는 단계라 double-submit 쿠키 방식을 쓴다 — 같은 토큰을
/// 쿠키와 폼 필드에 함께 실어 보내고, 제출 때 둘이 같은지 본다. 다른 출처의 페이지는
/// 쿠키를 읽어 폼 값에 넣을 수 없으므로 위조 요청이 걸러진다.
pub fn issue_token(jar: CookieJar) -> (CookieJar, String) {
    if let Some(existing) = jar.get(CSRF_COOKIE) {
        let token = existing.value().to_owned();
        return (jar, token);
    }

    let token = Uuid::new_v4().to_string();
    let mut cookie = Cookie::new(CSRF_COOKIE, token.clone());
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");

    (jar.add(cookie), token)
}

/// 제출된 폼의 토큰이 쿠키의 것과 같은가.
pub fn verify_token(jar: &CookieJar, submitted: &str) -> bool {
    jar.get(CSRF_COOKIE)
        .is_some_and(|cookie| constant_time_equals(cookie.value(), submitted))
}

/// 토큰 비교는 길이·내용이 일찍 드러나지 않도록 상수 시간에 가깝게 한다.
fn constant_time_equals(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 같은_값만_통과한다() {
        assert!(constant_time_equals("abc", "abc"));
        assert!(!constant_time_equals("abc", "abd"));
        assert!(!constant_time_equals("abc", "abcd"));
    }
}
