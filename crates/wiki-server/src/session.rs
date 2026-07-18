use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::HeaderValue;
use axum::http::request::Parts;
use std::net::{IpAddr, SocketAddr};
use tower_sessions::Session;
use uuid::Uuid;
use wiki_account::WikiUser;

use crate::ServerError;
use crate::state::AppState;

/// 세션에 담는 로그인 표시. 내부 id가 아니라 외부 식별자를 넣는다 —
/// 쿠키로 나가는 값이 내부 키를 드러내지 않게 한다 (docs/architecture.md).
const SESSION_USER: &str = "user";

/// 이번 요청을 누가 보냈는가.
///
/// 로그인 사용자든 비로그인 IP든 위키에서는 모두 행위 주체라, 핸들러가 둘을
/// 구분하지 않고 쓸 수 있게 한 타입으로 싣는다.
#[derive(Clone)]
pub struct Requester {
    pub user: Option<WikiUser>,
    pub ip_address: String,
}

impl Requester {
    pub fn is_member(&self) -> bool {
        self.user.is_some()
    }

    /// 이 주체의 actor를 확보한다 (리비전·토론이 참조할 대상).
    pub async fn actor(
        &self,
        state: &AppState,
    ) -> Result<wiki_account::ActorIdentifier, ServerError> {
        Ok(match &self.user {
            Some(user) => wiki_account::ensure_user_actor(&state.pool, user.identifier).await?,
            None => wiki_account::ensure_ip_actor(&state.pool, &self.ip_address).await?,
        })
    }

    /// 권한 판정에 넘길 주체 — 소속 그룹과 계정 여부를 채운다.
    pub async fn subject(
        &self,
        state: &AppState,
    ) -> Result<wiki_authorization::RequestSubject, ServerError> {
        let user_id = self.user.as_ref().map(|user| user.identifier.as_raw());
        let groups =
            wiki_authorization::active_group_names(&state.pool, &self.ip_address, user_id).await?;

        Ok(wiki_authorization::RequestSubject {
            ip_address: Some(self.ip_address.clone()),
            is_member: self.is_member(),
            acl_groups: groups,
        })
    }

    /// 이 문서에 대해 그 동작이 허용되는가.
    pub async fn may(
        &self,
        state: &AppState,
        title: &wiki_document::DocumentTitle,
        action: wiki_authorization::AclAction,
    ) -> Result<bool, ServerError> {
        let rules = wiki_authorization::load_rules(
            &state.pool,
            title.namespace.as_str(),
            &title.name,
            action,
        )
        .await?;
        let subject = self.subject(state).await?;

        Ok(wiki_authorization::evaluate(&rules, action, &subject).is_allowed())
    }

    /// 이 사용자가 가진 운영 권한인가 (비로그인은 언제나 아니다).
    pub async fn has_permission(
        &self,
        state: &AppState,
        permission: &str,
    ) -> Result<bool, ServerError> {
        let Some(user) = &self.user else {
            return Ok(false);
        };
        let granted =
            wiki_authorization::granted_permissions(&state.pool, user.identifier.as_raw()).await?;
        Ok(granted.iter().any(|name| name == permission))
    }
}

/// 프록시를 거친 요청에서 원래 클라이언트 주소를 고른다.
///
/// 비로그인 편집자는 IP가 곧 신원이라(actor), 프론트엔드가 대신 부른 요청도 원래
/// 주소로 판정돼야 한다. 다만 `X-Forwarded-For`는 누구나 붙일 수 있으므로 **연결 상대가
/// 우리 프록시(루프백)일 때만** 믿는다 — 그러지 않으면 헤더 한 줄로 차단을 벗어난다.
fn client_address(peer: Option<IpAddr>, forwarded: Option<&HeaderValue>) -> String {
    let Some(peer) = peer else {
        return "unknown".to_owned();
    };

    if peer.is_loopback() {
        let original = forwarded
            .and_then(|value| value.to_str().ok())
            .and_then(|chain| chain.split(',').next())
            .map(str::trim)
            .filter(|candidate| !candidate.is_empty());

        if let Some(original) = original {
            return original.to_owned();
        }
    }

    peer.to_string()
}

impl FromRequestParts<AppState> for Requester {
    type Rejection = ServerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let peer = ConnectInfo::<SocketAddr>::from_request_parts(parts, state)
            .await
            .map(|ConnectInfo(address)| address.ip())
            .ok();

        let ip_address = client_address(peer, parts.headers.get("x-forwarded-for"));

        let user = match Session::from_request_parts(parts, state).await {
            Ok(session) => current_user(&session, state).await?,
            Err(_) => None,
        };

        Ok(Self { user, ip_address })
    }
}

async fn current_user(
    session: &Session,
    state: &AppState,
) -> Result<Option<WikiUser>, ServerError> {
    let Some(external_id) = session
        .get::<Uuid>(SESSION_USER)
        .await
        .map_err(|_| ServerError::Session)?
    else {
        return Ok(None);
    };

    Ok(wiki_account::find_user_by_external_id(&state.pool, external_id).await?)
}

/// 로그인 상태로 만든다. 세션 고정 공격을 막으려 식별자를 새로 발급한다.
pub async fn log_in(session: &Session, user: &WikiUser) -> Result<(), ServerError> {
    session.cycle_id().await.map_err(|_| ServerError::Session)?;
    session
        .insert(SESSION_USER, user.external_id)
        .await
        .map_err(|_| ServerError::Session)
}

pub async fn log_out(session: &Session) -> Result<(), ServerError> {
    session.flush().await.map_err(|_| ServerError::Session)
}

#[cfg(test)]
mod tests {
    use super::client_address;
    use axum::http::HeaderValue;
    use std::net::IpAddr;

    fn address(text: &str) -> IpAddr {
        text.parse().expect("주소를 읽을 수 있어야 한다")
    }

    #[test]
    fn 프록시가_전한_주소를_쓴다() {
        let forwarded = HeaderValue::from_static("203.0.113.42");
        let resolved = client_address(Some(address("127.0.0.1")), Some(&forwarded));
        assert_eq!(resolved, "203.0.113.42");
    }

    #[test]
    fn 바깥에서_붙인_헤더는_믿지_않는다() {
        let forged = HeaderValue::from_static("10.0.0.1");
        let resolved = client_address(Some(address("203.0.113.42")), Some(&forged));
        assert_eq!(resolved, "203.0.113.42");
    }

    #[test]
    fn 헤더가_없으면_연결_상대를_쓴다() {
        let resolved = client_address(Some(address("203.0.113.42")), None);
        assert_eq!(resolved, "203.0.113.42");
    }
}
