use axum::{
    extract::{multipart::Field, Multipart},
    response::Html,
    Extension, Json,
};
use chrono::{SecondsFormat, Utc};
use hyper::StatusCode;
use serde::Serialize;
use sha2::Digest;
use std::{io::Write, sync::Arc};

use crate::{app_config::AppConfig, av::AvContext, av_engine::AvScanResult};

#[derive(Serialize)]
pub struct AvResponse {
    #[serde(rename = "avVersion")]
    av_version: String,
    #[serde(rename = "dbVersion")]
    db_version: u32,
    #[serde(rename = "dbSignatureCount")]
    db_sig_count: u32,
    #[serde(rename = "dbDate")]
    db_date: String,
    results: Vec<AvResult>,
}

#[derive(Serialize)]
pub struct AvResult {
    name: Option<String>,
    size: u64,
    crc32: String,
    md5: String,
    sha256: String,
    #[serde(rename = "contentType")]
    content_type: Option<String>,
    #[serde(rename = "dateScanned")]
    date_scanned: String,
    result: &'static str,
    signature: Option<String>,
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
    while let Some(mut field) = mp.next_field().await.map_err(map_mp_error_to_400)? {
        let mut tmp = tempfile::Builder::new()
            .rand_bytes(12)
            .tempfile()
            .map_err(map_io_error_to_500)?;
        let mut size = 0;
        let mut crc32 = crc32fast::Hasher::new();
        let mut md5 = md5::Context::new();
        let mut sha256 = sha2::Sha256::new();
        while let Some(chunk) = field.chunk().await.map_err(map_mp_error_to_400)? {
            size += tmp.write(&chunk).map_err(map_io_error_to_500)? as u64;
            crc32.update(&chunk);
            md5.consume(&chunk);
            sha256.update(&chunk);
        }
        tmp.as_file().sync_data().map_err(map_io_error_to_500)?;
        results.push(
            map_result(
                &ctx,
                &field,
                &tmp,
                size,
                format!("{:08x?}", crc32.finalize()),
                format!("{:032x?}", md5.compute()),
                hex::encode(sha256.finalize()),
            )
            .map_err(map_io_error_to_500)?,
        );
    }
    Ok(Json(AvResponse {
        av_version: ctx.clamav_version.to_owned(),
        db_version: ctx.db_version,
        db_sig_count: ctx.db_sig_count,
        db_date: ctx.db_date.to_rfc3339_opts(SecondsFormat::Millis, true),
        results,
    }))
}

fn map_result(
    ctx: &AvContext,
    field: &Field<'_>,
    tmp: &tempfile::NamedTempFile,
    size: u64,
    crc32: String,
    md5: String,
    sha256: String,
) -> Result<AvResult, std::io::Error> {
    let name = field.name().map(|f| f.to_string());
    let path = tmp.path().to_str().expect("temp path should be valid");
    let content_type = infer::get_from_path(path)?.map(|t| t.mime_type().to_string());
    match ctx.engine.scan(path, &mut ctx.settings.to_owned()) {
        Ok(r) => Ok(AvResult {
            name,
            size,
            crc32,
            md5,
            sha256,
            content_type,
            date_scanned: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
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
        }),
        Err(err) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("[{}] {}", err.code(), err.string_error()),
        )),
    }
}

#[inline]
fn map_mp_error_to_400(err: axum::extract::multipart::MultipartError) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, err.to_string())
}

#[inline]
fn map_io_error_to_500(err: std::io::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
