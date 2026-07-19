//! 프론트엔드 프록시.
//!
//! 브라우저의 진입점은 언제나 axum이고, API가 아닌 요청은 Next.js가 그린다
//! (docs/architecture.md). 오리진이 하나라 세션 쿠키가 그대로 흐르고 CORS가 필요 없다.

use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderName, HeaderValue, StatusCode, Uri, header};
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

/// 프론트엔드가 그리지 못하는 상태에서 브라우저가 화면 요청으로 들어왔을 때.
///
/// 오류 화면은 원래 프론트엔드의 몫이지만 그쪽에 닿지 못해 난 오류다 — 여기서 JSON을
/// 내면 주소창에 중괄호가 뜨고, 평문을 내면 브라우저 기본 서식으로 선다. axum이 직접
/// 그려야 하는 유일한 화면이므로 셸 없이 홀로 서는 한 장을 여기 갖춘다.
fn outage(status: StatusCode, title: &str, description: &str) -> Response {
    let body = format!(
        "<!doctype html><html lang=\"ko\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{title}</title></head>\
         <body style=\"margin:0;min-height:100dvh;display:flex;flex-direction:column;\
         justify-content:center;gap:12px;padding:0 24px;\
         font-family:system-ui,sans-serif;color:#121a18\">\
         <h1 style=\"margin:0;font-size:30px;font-weight:800;letter-spacing:-0.02em;\
         color:#000\">{title}</h1>\
         <p style=\"margin:0;font-size:14.5px;color:#24302d\">{description}</p>\
         </body></html>"
    );

    (
        status,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        body,
    )
        .into_response()
}

pub async fn forward(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
) -> Response {
    let Some(frontend) = state.frontend_origin.as_deref() else {
        return outage(
            StatusCode::NOT_FOUND,
            "위키를 열지 못했습니다",
            "프론트엔드 주소(OPENSINABRO_FRONTEND)가 설정되지 않았습니다.",
        );
    };

    let path_and_query = request
        .uri()
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");

    let Ok(target) = format!("{frontend}{path_and_query}").parse::<Uri>() else {
        return outage(
            StatusCode::BAD_GATEWAY,
            "위키를 열지 못했습니다",
            "요청한 주소를 프론트엔드로 넘기지 못했습니다.",
        );
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
            return outage(
                StatusCode::BAD_GATEWAY,
                "위키를 열지 못했습니다",
                "프론트엔드에 연결하지 못했습니다. 잠시 뒤 다시 시도해 주세요.",
            );
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
