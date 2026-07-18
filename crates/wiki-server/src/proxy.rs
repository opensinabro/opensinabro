//! 프론트엔드 프록시.
//!
//! 브라우저의 진입점은 언제나 axum이고, API가 아닌 요청은 Next.js가 그린다
//! (docs/design/07). 오리진이 하나라 세션 쿠키가 그대로 흐르고 CORS가 필요 없다.

use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderName, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;

use crate::state::AppState;

/// 비로그인 편집자는 IP가 곧 신원이라(actor), 프록시를 거쳐도 원래 주소가 남아야
/// 한다. 이 값이 루프백으로 뭉개지면 `ip:CIDR` 조건과 차단이 통째로 무너진다.
///
/// axum이 바깥과 맞닿은 유일한 지점이므로 들어온 헤더에 **덧붙이지 않고 갈아 끼운다** —
/// 클라이언트가 미리 넣어 둔 값을 그대로 흘려보내면 헤더 한 줄로 신원을 위조할 수 있다.
fn forwarded_for(peer: SocketAddr) -> Option<HeaderValue> {
    HeaderValue::from_str(&peer.ip().to_string()).ok()
}

pub async fn forward(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
) -> Response {
    let Some(frontend) = state.frontend_origin.as_deref() else {
        return (
            StatusCode::NOT_FOUND,
            "프론트엔드 주소(OPENSINABRO_FRONTEND)가 설정되지 않았습니다.",
        )
            .into_response();
    };

    let path_and_query = request
        .uri()
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");

    let Ok(target) = format!("{frontend}{path_and_query}").parse::<Uri>() else {
        return StatusCode::BAD_GATEWAY.into_response();
    };

    let mut outgoing = request;
    let header = HeaderName::from_static("x-forwarded-for");
    match forwarded_for(peer) {
        Some(client) => {
            outgoing.headers_mut().insert(header, client);
        }
        None => {
            outgoing.headers_mut().remove(header);
        }
    }
    *outgoing.uri_mut() = target;

    // 프로토콜 전환 요청이면 양쪽의 전환이 끝난 뒤 두 연결을 잇는다. 개발 모드의
    // 갱신 채널(HMR)이 웹소켓이라, 이것이 없으면 프론트엔드 스크립트가 시작하다 멎고
    // 화면이 서버가 그린 상태 그대로 굳는다.
    let client_upgrade = hyper::upgrade::on(&mut outgoing);

    let mut response = match state.http.request(outgoing).await {
        Ok(response) => response,
        Err(_) => {
            return (StatusCode::BAD_GATEWAY, "프론트엔드에 연결할 수 없습니다.").into_response();
        }
    };

    if response.status() == StatusCode::SWITCHING_PROTOCOLS {
        let frontend_upgrade = hyper::upgrade::on(&mut response);

        tokio::spawn(async move {
            let (Ok(client), Ok(frontend)) = (client_upgrade.await, frontend_upgrade.await) else {
                return;
            };

            let mut client = TokioIo::new(client);
            let mut frontend = TokioIo::new(frontend);
            let _ = tokio::io::copy_bidirectional(&mut client, &mut frontend).await;
        });
    }

    response.map(Body::new)
}
