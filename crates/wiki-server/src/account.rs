use axum::Json;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use chrono::Duration;
use serde::Deserialize;
use tower_sessions::Session;
use wiki_account::VerificationPurpose;
use wiki_document::{DocumentTitle, Namespace, RevisionKind};

use crate::ServerError;
use crate::security::verify_header;
use crate::session::{Requester, log_in, log_out};
use crate::state::AppState;

type HandlerResult = Result<Response, ServerError>;

/// 가입 확인 링크가 살아 있는 기간.
const VERIFICATION_VALID_HOURS: i64 = 24;

#[derive(Deserialize)]
pub struct VerifyQuery {
    token: String,
}

/// 메일의 확인 링크가 곧장 닿는 자리라 GET이지만 상태를 바꾼다 — 링크는 한 번만
/// 쓰이므로(`consume`) 두 번째부터는 기한 만료와 같은 답을 낸다.
pub async fn verify_api(
    State(state): State<AppState>,
    Query(parameters): Query<VerifyQuery>,
) -> Result<Response, ServerError> {
    let consumed =
        wiki_account::consume(&state.pool, &parameters.token, VerificationPurpose::Signup).await?;

    let verified = match consumed {
        Some(verification) => {
            if let Some(credential) = verification.credential_id {
                wiki_account::mark_verified(&state.pool, credential).await?;
            }
            true
        }
        None => false,
    };

    Ok(Json(serde_json::json!({ "verified": verified })).into_response())
}

/// 가입하면 사용자 문서를 만들어 둔다 (the seed와 같은 동작).
async fn create_user_document(
    state: &AppState,
    user: &wiki_account::WikiUser,
) -> Result<(), ServerError> {
    let title = DocumentTitle::new(Namespace::new(Namespace::USER), user.name.clone());
    if wiki_document::find_document(&state.pool, &title)
        .await?
        .is_some()
    {
        return Ok(());
    }

    let actor = wiki_account::ensure_user_actor(&state.pool, user.identifier).await?;
    wiki_document::record_revision(
        &state.pool,
        &title,
        actor,
        RevisionKind::Create,
        Some(""),
        "계정 생성",
        None,
    )
    .await?;

    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequestBody {
    name: String,
    password: String,
}

/// 로그인. 성공·실패 어느 쪽도 계정이 있는지는 알리지 않는다.
pub async fn login_api(
    State(state): State<AppState>,
    jar: CookieJar,
    session: Session,
    requester: Requester,
    headers: HeaderMap,
    axum::Json(submission): axum::Json<LoginRequestBody>,
) -> HandlerResult {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    match wiki_account::authenticate(&state.pool, &submission.name, &submission.password).await? {
        Some(user) => {
            wiki_account::record_login_attempt(
                &state.pool,
                user.identifier,
                &requester.ip_address,
                user_agent,
                true,
            )
            .await?;
            log_in(&session, &user).await?;
            Ok(axum::Json(serde_json::json!({ "ok": true })).into_response())
        }
        None => {
            // 실패한 시도도 기록에 남긴다 — 다만 계정이 있는지는 응답으로 구분하지 않는다.
            if let Some(existing) =
                wiki_account::find_user_by_name(&state.pool, &submission.name).await?
            {
                wiki_account::record_login_attempt(
                    &state.pool,
                    existing.identifier,
                    &requester.ip_address,
                    user_agent,
                    false,
                )
                .await?;
            }

            Ok((
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({ "error": "invalid_credentials" })),
            )
                .into_response())
        }
    }
}

/// 로그아웃. 세션을 통째로 버린다.
pub async fn logout_api(
    jar: CookieJar,
    session: Session,
    requester: Requester,
    headers: HeaderMap,
) -> HandlerResult {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }
    if !requester.is_member() {
        return Ok(crate::api::unauthorized());
    }

    log_out(&session).await?;
    Ok(axum::Json(serde_json::json!({ "ok": true })).into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupRequestBody {
    name: String,
    email: String,
    password: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupPayload {
    name: String,
}

/// 계정 만들기. 검증 순서와 문구는 폼 경로와 같다.
pub async fn signup_api(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    axum::Json(submission): axum::Json<SignupRequestBody>,
) -> HandlerResult {
    if !verify_header(&jar, &headers) {
        return Ok(crate::api::forbidden());
    }

    let name = submission.name.trim();
    if name.is_empty() || name.contains('/') || submission.password.len() < 8 {
        return Ok(rejected_api(
            "사용자 이름에는 '/'를 쓸 수 없고, 비밀번호는 여덟 자 이상이어야 합니다.",
        ));
    }

    if wiki_account::find_user_by_name(&state.pool, name)
        .await?
        .is_some()
        || wiki_account::email_taken(&state.pool, &submission.email).await?
    {
        return Ok(rejected_api("이미 쓰이고 있는 이름이나 이메일입니다."));
    }

    let user = wiki_account::create_user(&state.pool, name).await?;
    wiki_account::set_password(&state.pool, user.identifier, &submission.password).await?;
    let credential =
        wiki_account::add_email(&state.pool, user.identifier, &submission.email).await?;

    let issued = wiki_account::issue(
        &state.pool,
        user.identifier,
        Some(credential),
        VerificationPurpose::Signup,
        Duration::hours(VERIFICATION_VALID_HOURS),
    )
    .await?;

    // 메일 발송기는 아직 없다. 링크를 로그로 남겨 운영자가 전달할 수 있게 한다.
    println!(
        "[가입 확인] {} <{}> → /verify?token={}",
        user.name, submission.email, issued.token
    );

    create_user_document(&state, &user).await?;

    Ok(axum::Json(SignupPayload {
        name: user.name.clone(),
    })
    .into_response())
}

fn rejected_api(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}
