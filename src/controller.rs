use crate::{app_config::AppConfig, av::AvContext, av_engine::AvScanResult};
use std::{io::Write, sync::Arc};

use axum::{
    extract::{multipart::Field, Multipart},
    response::Html,
    Extension, Json,
};
use hyper::StatusCode;
use serde::Serialize;

#[derive(Serialize)]
pub struct AvResponse {
    #[serde(rename = "avVersion")]
    av_version: String,
    #[serde(rename = "dbVersion")]
    db_version: u32,
    #[serde(rename = "dbSignatureCount")]
    db_sig_count: u32,
    #[serde(rename = "dbTimestampSeconds")]
    db_timestamp_sec: u32,
    results: Vec<AvResult>,
}

#[derive(Serialize)]
pub struct AvResult {
    name: Option<String>,
    #[serde(rename = "contentType")]
    content_type: Option<String>,
    result: &'static str,
    signature: Option<String>,
    #[serde(rename = "errorCode")]
    error_code: Option<u32>,
    #[serde(rename = "errorMessage")]
    error_msg: Option<String>,
}

const INDEX_HTML: &'static [u8] = include_bytes!("index.html");

pub async fn index_html() -> Html<&'static [u8]> {
    Html(INDEX_HTML)
}

type ShutdownTx = tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>;

pub async fn shutdown(
    Extension(cfg): Extension<Arc<AppConfig>>,
    Extension(tx): Extension<Arc<ShutdownTx>>,
) -> StatusCode {
    match cfg.enable_shutdown_endpoint {
        false => StatusCode::NOT_FOUND,
        true => {
            let ptr = Arc::clone(&tx);
            let mut lock = ptr.lock().await;
            lock.take().map(|sender| sender.send(()));
            StatusCode::ACCEPTED
        }
    }
}

pub async fn upload(
    Extension(ctx): Extension<Arc<AvContext>>,
    mut mp: Multipart,
) -> Result<Json<AvResponse>, (StatusCode, String)> {
    let mut results = Vec::new();
    while let Some(mut field) = mp.next_field().await.map_err(map_mp_error_to_403)? {
        let mut tmp = tempfile::Builder::new()
            .rand_bytes(12)
            .tempfile()
            .map_err(map_io_error_to_500)?;
        while let Some(chunk) = field.chunk().await.map_err(map_mp_error_to_403)? {
            tmp.write(&chunk).map_err(map_io_error_to_500)?;
        }
        tmp.as_file().sync_data().map_err(map_io_error_to_500)?;
        results.push(map_result(&ctx, &field, &tmp));
    }
    Ok(Json(AvResponse {
        av_version: ctx.clamav_version.to_owned(),
        db_version: ctx.db_version,
        db_sig_count: ctx.db_sig_count,
        db_timestamp_sec: ctx.db_timestamp_sec,
        results,
    }))
}

fn map_result(ctx: &AvContext, field: &Field<'_>, tmp: &tempfile::NamedTempFile) -> AvResult {
    let name = field.name().map(|f| f.to_string());
    let content_type = field.content_type().map(|c| c.to_string());
    let scan_result = tmp
        .path()
        .to_str()
        .map(|p| ctx.engine.scan(p, &mut ctx.settings.to_owned()));
    match scan_result {
        Some(Ok(r)) => AvResult {
            name,
            content_type,
            result: match r {
                AvScanResult::Clean => "CLEAN",
                AvScanResult::Whitelisted => "WHITELISTED",
                AvScanResult::Virus(_) => "VIRUS",
            },
            signature: match r {
                AvScanResult::Clean => None,
                AvScanResult::Whitelisted => None,
                AvScanResult::Virus(sig) => Some(sig),
            },
            error_code: None,
            error_msg: None,
        },
        Some(Err(err)) => AvResult {
            name,
            content_type,
            result: "ERROR",
            signature: None,
            error_code: Some(err.code()),
            error_msg: Some(err.string_error()),
        },
        None => AvResult {
            name,
            content_type,
            result: "ERROR",
            signature: None,
            error_code: None,
            error_msg: None,
        },
    }
}

fn map_mp_error_to_403(err: axum::extract::multipart::MultipartError) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, err.to_string())
}

fn map_io_error_to_500(err: std::io::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
