use askama::Template;
use axum::extract::{Multipart, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use wiki_authorization::AclAction;
use wiki_document::{DocumentTitle, Namespace, RevisionKind};

use crate::ServerError;
use crate::handler::{escape, shell};
use crate::security::{issue_token, verify_token};
use crate::session::Requester;
use crate::state::AppState;

type HandlerResult = Result<Response, ServerError>;

/// 업로드 폼. 파일명·라이선스·분류가 모두 있어야 올릴 수 있다 (docs/design/06).
pub async fn upload_form(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
) -> HandlerResult {
    let licenses = wiki_document::licenses(&state.pool).await?;
    let options = licenses
        .into_iter()
        .map(|(name, display)| {
            format!(
                "<option value=\"{}\">{}</option>",
                escape(&name),
                escape(&display)
            )
        })
        .collect::<String>();

    let (jar, csrf_token) = issue_token(jar);
    let body = format!(
        "<form method=\"post\" action=\"/upload\" enctype=\"multipart/form-data\">\
         <input type=\"hidden\" name=\"csrf_token\" value=\"{csrf_token}\">\
         <label>파일 <input type=\"file\" name=\"file\" required></label>\
         <label>파일명 <input type=\"text\" name=\"name\" required></label>\
         <label>라이선스 <select name=\"license\" required>{options}</select></label>\
         <label>분류 <input type=\"text\" name=\"category\" required \
           placeholder=\"예: 야구 사진\"></label>\
         <label>설명 <textarea name=\"description\" rows=\"4\"></textarea></label>\
         <button type=\"submit\">올리기</button>\
         </form>",
        csrf_token = escape(&csrf_token)
    );

    let page = shell(&state, &requester, "파일 올리기", body, &csrf_token)
        .await?
        .render()?;
    Ok((jar, Html(page)).into_response())
}

#[derive(Default)]
struct UploadFields {
    csrf_token: String,
    name: String,
    license: String,
    category: String,
    description: String,
    bytes: Vec<u8>,
    media_type: String,
}

/// 업로드 처리. 파일도 문서이므로 리비전을 남기고, 바이너리만 따로 저장한다.
pub async fn upload_submit(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    mut multipart: Multipart,
) -> HandlerResult {
    let mut fields = UploadFields::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ServerError::Upload)?
    {
        let name = field.name().unwrap_or_default().to_owned();
        if name == "file" {
            fields.media_type = field.content_type().unwrap_or_default().to_owned();
            fields.bytes = field
                .bytes()
                .await
                .map_err(|_| ServerError::Upload)?
                .to_vec();
            continue;
        }

        let text = field.text().await.map_err(|_| ServerError::Upload)?;
        match name.as_str() {
            "csrf_token" => fields.csrf_token = text,
            "name" => fields.name = text,
            "license" => fields.license = text,
            "category" => fields.category = text,
            "description" => fields.description = text,
            _ => {}
        }
    }

    if !verify_token(&jar, &fields.csrf_token) {
        return Ok((StatusCode::FORBIDDEN, "요청을 확인할 수 없습니다.").into_response());
    }

    let title = DocumentTitle::new(Namespace::new(Namespace::FILE), fields.name.trim());
    if title.name.is_empty() || fields.license.is_empty() || fields.category.trim().is_empty() {
        return rejected(
            &state,
            &requester,
            "파일명·라이선스·분류는 모두 있어야 합니다.",
        )
        .await;
    }
    if !wiki_document::is_supported_media_type(&fields.media_type) {
        return rejected(&state, &requester, "지원하지 않는 형식입니다.").await;
    }
    if !requester.may(&state, &title, AclAction::Edit).await? {
        return rejected(&state, &requester, "파일을 올릴 권한이 없습니다.").await;
    }

    let hash = wiki_document::store_content(
        &state.pool,
        &state.file_root,
        &fields.bytes,
        &fields.media_type,
    )
    .await?;

    // 파일 문서의 본문은 설명과 분류다 — 분류가 본문에 있어야 분류 목록에 걸린다.
    let source = format!(
        "{description}\n[[분류:{category}]]",
        description = fields.description.trim(),
        category = fields.category.trim(),
    );

    let actor = requester.actor(&state).await?;
    let revision = wiki_document::record_revision(
        &state.pool,
        &title,
        actor,
        RevisionKind::Create,
        Some(&source),
        "파일 올림",
        None,
    )
    .await?;

    wiki_document::attach_file(&state.pool, revision, &hash, &fields.license).await?;
    crate::edit::apply_side_effects(&state, &title, &source).await?;

    Ok(Redirect::to(&format!("/w/{title}")).into_response())
}

/// 바이너리 서빙. 문서 보기는 `/w/파일:이름`이고 여기서는 내용만 낸다.
pub async fn serve_file(State(state): State<AppState>, Path(name): Path<String>) -> HandlerResult {
    let Some(file) = wiki_document::read_file(&state.pool, &state.file_root, &name).await? else {
        return Ok((StatusCode::NOT_FOUND, "그런 파일이 없습니다.").into_response());
    };

    let Ok(bytes) = std::fs::read(&file.path) else {
        return Ok((StatusCode::NOT_FOUND, "파일 내용을 찾을 수 없습니다.").into_response());
    };

    Ok((
        [
            (header::CONTENT_TYPE, file.media_type),
            // 내용이 바뀌면 파일명도 바뀌는 구조가 아니라 짧게만 캐시한다.
            (header::CACHE_CONTROL, "public, max-age=300".to_owned()),
        ],
        bytes,
    )
        .into_response())
}

async fn rejected(state: &AppState, requester: &Requester, message: &str) -> HandlerResult {
    let body = format!(
        "<p>{}</p><p><a href=\"/upload\">다시 시도</a></p>",
        escape(message)
    );
    let page = shell(state, requester, "파일 올리기", body, "")
        .await?
        .render()?;
    Ok((StatusCode::BAD_REQUEST, Html(page)).into_response())
}
