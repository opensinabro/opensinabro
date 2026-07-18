use askama::Template;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use chrono::Duration;
use serde::Deserialize;
use tower_sessions::Session;
use wiki_account::VerificationPurpose;
use wiki_document::{DocumentTitle, Namespace, RevisionKind};

use crate::ServerError;
use crate::handler::escape;
use crate::security::{issue_token, verify_token};
use crate::session::{Requester, log_in, log_out};
use crate::state::AppState;
use crate::template::Shell;

type HandlerResult = Result<Response, ServerError>;

/// 가입 확인 링크가 살아 있는 기간.
const VERIFICATION_VALID_HOURS: i64 = 24;

pub async fn login_form(
    State(state): State<AppState>,
    jar: CookieJar,
    requester: Requester,
) -> HandlerResult {
    if requester.is_member() {
        return Ok(Redirect::to("/").into_response());
    }

    let (jar, csrf_token) = issue_token(jar);
    let body = format!(
        "<form method=\"post\" action=\"/login\">\
         <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
         <label>사용자 이름 <input type=\"text\" name=\"name\" autocomplete=\"username\" required></label>\
         <label>비밀번호 <input type=\"password\" name=\"password\" autocomplete=\"current-password\" required></label>\
         <button type=\"submit\">로그인</button>\
         </form>\
         <p><a href=\"/signup\">계정 만들기</a></p>",
        csrf_token = escape(&csrf_token)
    );

    let page = Shell::new(&state.settings, "로그인", body).render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct LoginSubmission {
    csrf_token: String,
    name: String,
    password: String,
}

pub async fn login_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    session: Session,
    requester: Requester,
    headers: HeaderMap,
    axum::Form(submission): axum::Form<LoginSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
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
            Ok(Redirect::to("/").into_response())
        }
        None => {
            // 어느 쪽이 틀렸는지 알리지 않는다 — 계정 존재 여부가 새지 않게.
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

            let body = "<p>사용자 이름이나 비밀번호가 맞지 않습니다.</p>\
                        <p><a href=\"/login\">다시 시도</a></p>"
                .to_owned();
            let page = Shell::new(&state.settings, "로그인", body).render()?;
            Ok((StatusCode::UNAUTHORIZED, Html(page)).into_response())
        }
    }
}

#[derive(Deserialize)]
pub struct LogoutSubmission {
    csrf_token: String,
}

pub async fn logout_submit(
    jar: CookieJar,
    session: Session,
    axum::Form(submission): axum::Form<LogoutSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    log_out(&session).await?;
    Ok(Redirect::to("/").into_response())
}

pub async fn signup_form(
    State(state): State<AppState>,
    jar: CookieJar,
    requester: Requester,
) -> HandlerResult {
    if requester.is_member() {
        return Ok(Redirect::to("/").into_response());
    }

    let (jar, csrf_token) = issue_token(jar);
    let body = format!(
        "<form method=\"post\" action=\"/signup\">\
         <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
         <label>사용자 이름 <input type=\"text\" name=\"name\" autocomplete=\"username\" required></label>\
         <label>이메일 <input type=\"email\" name=\"email\" autocomplete=\"email\" required></label>\
         <label>비밀번호 <input type=\"password\" name=\"password\" autocomplete=\"new-password\" required minlength=\"8\"></label>\
         <button type=\"submit\">가입</button>\
         </form>",
        csrf_token = escape(&csrf_token)
    );

    let page = Shell::new(&state.settings, "계정 만들기", body).render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Deserialize)]
pub struct SignupSubmission {
    csrf_token: String,
    name: String,
    email: String,
    password: String,
}

pub async fn signup_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::Form(submission): axum::Form<SignupSubmission>,
) -> HandlerResult {
    if !verify_token(&jar, &submission.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let name = submission.name.trim();
    if name.is_empty() || name.contains('/') || submission.password.len() < 8 {
        return rejected(
            &state,
            "사용자 이름에는 '/'를 쓸 수 없고, 비밀번호는 여덟 자 이상이어야 합니다.",
        );
    }

    if wiki_account::find_user_by_name(&state.pool, name)
        .await?
        .is_some()
        || wiki_account::email_taken(&state.pool, &submission.email).await?
    {
        return rejected(&state, "이미 쓰이고 있는 이름이나 이메일입니다.");
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

    let body = format!(
        "<p>{}님, 계정을 만들었습니다.</p>\
         <p>보내 드린 확인 링크로 이메일을 인증한 뒤 로그인하세요.</p>\
         <p><a href=\"/login\">로그인하러 가기</a></p>",
        escape(&user.name)
    );
    let page = Shell::new(&state.settings, "계정 만들기", body).render()?;
    Ok(Html(page).into_response())
}

#[derive(Deserialize)]
pub struct VerifyQuery {
    token: String,
}

/// 가입 확인 링크. 토큰은 한 번만 쓰인다.
pub async fn verify(
    State(state): State<AppState>,
    Query(parameters): Query<VerifyQuery>,
) -> HandlerResult {
    let consumed =
        wiki_account::consume(&state.pool, &parameters.token, VerificationPurpose::Signup).await?;

    let body = match consumed {
        Some(verification) => {
            if let Some(credential) = verification.credential_id {
                wiki_account::mark_verified(&state.pool, credential).await?;
            }
            "<p>이메일을 확인했습니다. 이제 로그인할 수 있습니다.</p>\
             <p><a href=\"/login\">로그인</a></p>"
                .to_owned()
        }
        None => "<p>링크가 이미 쓰였거나 기한이 지났습니다.</p>".to_owned(),
    };

    let page = Shell::new(&state.settings, "이메일 확인", body).render()?;
    Ok(Html(page).into_response())
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

fn rejected(state: &AppState, message: &str) -> HandlerResult {
    let body = format!(
        "<p>{}</p><p><a href=\"/signup\">다시 시도</a></p>",
        escape(message)
    );
    let page = Shell::new(&state.settings, "계정 만들기", body).render()?;
    Ok((StatusCode::BAD_REQUEST, Html(page)).into_response())
}
