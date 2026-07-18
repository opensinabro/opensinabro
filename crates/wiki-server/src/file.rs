use axum::extract::{Multipart, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::Serialize;
use wiki_authorization::AclAction;
use wiki_document::{DocumentTitle, Namespace, RevisionKind};

use crate::ServerError;
use crate::security::verify_token;
use crate::session::Requester;
use crate::state::AppState;

type HandlerResult = Result<Response, ServerError>;

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseEntry {
    name: String,
    display_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadOptionsPayload {
    licenses: Vec<LicenseEntry>,
    media_types: Vec<&'static str>,
}

/// 업로드 폼이 고를 수 있는 것들.
pub async fn upload_api(State(state): State<AppState>) -> Result<Response, ServerError> {
    let licenses = wiki_document::licenses(&state.pool)
        .await?
        .into_iter()
        .map(|(name, display_name)| LicenseEntry { name, display_name })
        .collect();

    Ok(axum::Json(UploadOptionsPayload {
        licenses,
        media_types: wiki_document::SUPPORTED_MEDIA_TYPES.to_vec(),
    })
    .into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadPayload {
    title: String,
}

/// 업로드 처리. 본문은 multipart 그대로 받고 결과만 JSON으로 낸다.
pub async fn upload_submit_api(
    State(state): State<AppState>,
    requester: Requester,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Result<Response, ServerError> {
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

    // multipart는 헤더에 토큰을 실을 수 없어 폼 경로와 같이 필드로 받는다.
    if !verify_token(&jar, &fields.csrf_token) {
        return Ok(crate::api::forbidden());
    }

    let title = DocumentTitle::new(Namespace::new(Namespace::FILE), fields.name.trim());
    if title.name.is_empty() || fields.license.is_empty() || fields.category.trim().is_empty() {
        return Ok(rejected_api("파일명·라이선스·분류는 모두 있어야 합니다."));
    }
    if !wiki_document::is_supported_media_type(&fields.media_type) {
        return Ok(rejected_api("지원하지 않는 형식입니다."));
    }
    // 폼 경로는 권한 거부도 400으로 내는데, 고쳐 다시 낼 수 있는 입력 문제와 섞이는
    // 결함이다 — 권한은 403으로 낸다.
    if !requester.may(&state, &title, AclAction::Edit).await? {
        return Ok(crate::api::forbidden());
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

    Ok(axum::Json(UploadPayload {
        title: title.to_string(),
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
