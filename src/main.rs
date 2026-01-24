mod app_config;
mod av;
mod controller;

use axum::{
    Extension, Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use axum_prometheus::PrometheusMetricLayer;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    main,
    net::TcpListener,
    select, signal,
    sync::{
        Mutex,
        oneshot::{self, Receiver},
    },
};
use tower_http::trace::TraceLayer;

#[main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("libclamav formpost service is starting...");

    let cfg = app_config::load();
    tracing::info!("Loaded config\n{}", cfg);

    let ctx = av::load_context().await;
    tracing::info!("Loaded context\n{}", ctx);

    let (max_file_size, port) = (cfg.max_file_size, cfg.port);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .route("/", get(controller::index_html))
        .route("/index.htm", get(controller::index_html))
        .route("/index.html", get(controller::index_html))
        .route("/shutdown", post(controller::shutdown))
        .route("/upload", post(controller::upload))
        .layer(Extension(Arc::new(cfg)))
        .layer(Extension(Arc::new(ctx)))
        .layer(Extension(Arc::new(Mutex::new(Some(shutdown_tx)))))
        .layer(DefaultBodyLimit::max(max_file_size))
        .layer(TraceLayer::new_for_http())
        .layer(prometheus_layer);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await.unwrap();
    tracing::info!("Bound to {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_rx))
        .await
        .unwrap();
}

#[inline]
async fn shutdown_signal(shutdown_rx: Receiver<()>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    select! {
        _ = ctrl_c => {},
        _ = terminate => {},
        _ = shutdown_rx => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;
    use axum_test::multipart::{MultipartForm, Part};
    use axum_test::{TestServer, expect_json};
    use serde_json::json;

    #[tokio::test]
    async fn upload_eicar_com_virus() {
        let cfg = app_config::load();
        let ctx = av::load_context().await;
        let app = Router::new()
            .route("/upload", post(controller::upload))
            .layer(Extension(Arc::new(cfg)))
            .layer(Extension(Arc::new(ctx)));
        let srv = TestServer::builder().mock_transport().build(app).unwrap();
        let eicar =
            Bytes::from("X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*");
        let part = Part::bytes(eicar).file_name("eicar.com");
        let form = MultipartForm::new().add_part("name", part);
        let resp = srv.post("/upload").multipart(form).await;
        resp.assert_status_ok();
        resp.assert_header("Content-Type", "application/json");
        resp.assert_json(&json!({
            "avVersion": expect_json::string(),
            "dbVersion": expect_json::integer(),
            "dbSignatureCount": expect_json::integer(),
            "dbDate": expect_json::iso_date_time(),
            "results": [{
                "name": "eicar.com",
                "size": 68,
                "crc32": "6851cf3c",
                "md5": "44d88612fea8a8f36de82e1278abb02f",
                "sha256": "275a021bbfb6489e54d471899f7db9d1663fc695ec2fe2a2c4538aabf651fd0f",
                "contentType": null,
                "dateScanned": expect_json::iso_date_time(),
                "result": "VIRUS",
                "signature": expect_json::string(),
            }]
        }));
    }

    #[tokio::test]
    async fn upload_eicar_com_zip_virus() {
        let cfg = app_config::load();
        let ctx = av::load_context().await;
        let app = Router::new()
            .route("/upload", post(controller::upload))
            .layer(Extension(Arc::new(cfg)))
            .layer(Extension(Arc::new(ctx)));
        let srv = TestServer::builder().mock_transport().build(app).unwrap();
        let eicar_com_zip = Bytes::from_static(&[
            0x50, 0x4b, 0x03, 0x04, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x98, 0xb8, 0x28,
            0x3c, 0xcf, 0x51, 0x68, 0x44, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0x09, 0x00,
            0x00, 0x00, 0x65, 0x69, 0x63, 0x61, 0x72, 0x2e, 0x63, 0x6f, 0x6d, 0x58, 0x35, 0x4f,
            0x21, 0x50, 0x25, 0x40, 0x41, 0x50, 0x5b, 0x34, 0x5c, 0x50, 0x5a, 0x58, 0x35, 0x34,
            0x28, 0x50, 0x5e, 0x29, 0x37, 0x43, 0x43, 0x29, 0x37, 0x7d, 0x24, 0x45, 0x49, 0x43,
            0x41, 0x52, 0x2d, 0x53, 0x54, 0x41, 0x4e, 0x44, 0x41, 0x52, 0x44, 0x2d, 0x41, 0x4e,
            0x54, 0x49, 0x56, 0x49, 0x52, 0x55, 0x53, 0x2d, 0x54, 0x45, 0x53, 0x54, 0x2d, 0x46,
            0x49, 0x4c, 0x45, 0x21, 0x24, 0x48, 0x2b, 0x48, 0x2a, 0x50, 0x4b, 0x01, 0x02, 0x14,
            0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x98, 0xb8, 0x28, 0x3c, 0xcf, 0x51,
            0x68, 0x44, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00, 0xff, 0x81, 0x00, 0x00, 0x00, 0x00, 0x65,
            0x69, 0x63, 0x61, 0x72, 0x2e, 0x63, 0x6f, 0x6d, 0x50, 0x4b, 0x05, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x37, 0x00, 0x00, 0x00, 0x6b, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ]);
        let part = Part::bytes(eicar_com_zip).file_name("eicar.com.zip");
        let form = MultipartForm::new().add_part("name", part);
        let resp = srv.post("/upload").multipart(form).await;
        resp.assert_status_ok();
        resp.assert_header("Content-Type", "application/json");
        resp.assert_json(&json!({
            "avVersion": expect_json::string(),
            "dbVersion": expect_json::integer(),
            "dbSignatureCount": expect_json::integer(),
            "dbDate": expect_json::iso_date_time(),
            "results": [{
                "name": "eicar.com.zip",
                "size": 184,
                "crc32": "31db20d1",
                "md5": "6ce6f415d8475545be5ba114f208b0ff",
                "sha256": "2546dcffc5ad854d4ddc64fbf056871cd5a00f2471cb7a5bfd4ac23b6e9eedad",
                "contentType": "application/zip",
                "dateScanned": expect_json::iso_date_time(),
                "result": "VIRUS",
                "signature": expect_json::string(),
            }]
        }));
    }

    #[tokio::test]
    async fn upload_eicar_com2_zip_virus() {
        let cfg = app_config::load();
        let ctx = av::load_context().await;
        let app = Router::new()
            .route("/upload", post(controller::upload))
            .layer(Extension(Arc::new(cfg)))
            .layer(Extension(Arc::new(ctx)));
        let srv = TestServer::builder().mock_transport().build(app).unwrap();
        let eicar_com2_zip = Bytes::from_static(&[
            0x50, 0x4b, 0x03, 0x04, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x32, 0xac, 0xeb, 0x28,
            0xd1, 0x20, 0xdb, 0x31, 0xb8, 0x00, 0x00, 0x00, 0xb8, 0x00, 0x00, 0x00, 0x0d, 0x00,
            0x00, 0x00, 0x65, 0x69, 0x63, 0x61, 0x72, 0x5f, 0x63, 0x6f, 0x6d, 0x2e, 0x7a, 0x69,
            0x70, 0x50, 0x4b, 0x03, 0x04, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x98, 0xb8,
            0x28, 0x3c, 0xcf, 0x51, 0x68, 0x44, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0x09,
            0x00, 0x00, 0x00, 0x65, 0x69, 0x63, 0x61, 0x72, 0x2e, 0x63, 0x6f, 0x6d, 0x58, 0x35,
            0x4f, 0x21, 0x50, 0x25, 0x40, 0x41, 0x50, 0x5b, 0x34, 0x5c, 0x50, 0x5a, 0x58, 0x35,
            0x34, 0x28, 0x50, 0x5e, 0x29, 0x37, 0x43, 0x43, 0x29, 0x37, 0x7d, 0x24, 0x45, 0x49,
            0x43, 0x41, 0x52, 0x2d, 0x53, 0x54, 0x41, 0x4e, 0x44, 0x41, 0x52, 0x44, 0x2d, 0x41,
            0x4e, 0x54, 0x49, 0x56, 0x49, 0x52, 0x55, 0x53, 0x2d, 0x54, 0x45, 0x53, 0x54, 0x2d,
            0x46, 0x49, 0x4c, 0x45, 0x21, 0x24, 0x48, 0x2b, 0x48, 0x2a, 0x50, 0x4b, 0x01, 0x02,
            0x14, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x98, 0xb8, 0x28, 0x3c, 0xcf,
            0x51, 0x68, 0x44, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00, 0xff, 0x81, 0x00, 0x00, 0x00, 0x00,
            0x65, 0x69, 0x63, 0x61, 0x72, 0x2e, 0x63, 0x6f, 0x6d, 0x50, 0x4b, 0x05, 0x06, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x37, 0x00, 0x00, 0x00, 0x6b, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x50, 0x4b, 0x01, 0x02, 0x14, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x32, 0xac, 0xeb, 0x28, 0xd1, 0x20, 0xdb, 0x31, 0xb8, 0x00, 0x00, 0x00, 0xb8,
            0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20,
            0x00, 0xb6, 0x81, 0x00, 0x00, 0x00, 0x00, 0x65, 0x69, 0x63, 0x61, 0x72, 0x5f, 0x63,
            0x6f, 0x6d, 0x2e, 0x7a, 0x69, 0x70, 0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x01, 0x00, 0x3b, 0x00, 0x00, 0x00, 0xe3, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        let part = Part::bytes(eicar_com2_zip).file_name("eicar.com2.zip");
        let form = MultipartForm::new().add_part("name", part);
        let resp = srv.post("/upload").multipart(form).await;
        resp.assert_status_ok();
        resp.assert_header("Content-Type", "application/json");
        resp.assert_json(&json!({
            "avVersion": expect_json::string(),
            "dbVersion": expect_json::integer(),
            "dbSignatureCount": expect_json::integer(),
            "dbDate": expect_json::iso_date_time(),
            "results": [{
                "name": "eicar.com2.zip",
                "size": 308,
                "crc32": "045a2cdd",
                "md5": "e4968ef99266df7c9a1f0637d2389dab",
                "sha256": "e1105070ba828007508566e28a2b8d4c65d192e9eaf3b7868382b7cae747b397",
                "contentType": "application/zip",
                "dateScanned": expect_json::iso_date_time(),
                "result": "VIRUS",
                "signature": expect_json::string(),
            }]
        }));
    }

    #[tokio::test]
    async fn upload_minpdf_clean() {
        let cfg = app_config::load();
        let ctx = av::load_context().await;
        let app = Router::new()
            .route("/upload", post(controller::upload))
            .layer(Extension(Arc::new(cfg)))
            .layer(Extension(Arc::new(ctx)));
        let srv = TestServer::builder().mock_transport().build(app).unwrap();
        let pdf = Bytes::from(concat!(
            "%PDF-1.\n",
            "1 0 obj<</Pages 2 0 R>>endobj\n",
            "2 0 obj<</Kids[3 0 R]/Count 1>>endobj\n",
            "3 0 obj<</Parent 2 0 R>>endobj\n",
            "trailer <</Root 1 0 R>>",
        ));
        let part = Part::bytes(pdf).file_name("min.pdf");
        let form = MultipartForm::new().add_part("name", part);
        let resp = srv.post("/upload").multipart(form).await;
        resp.assert_status_ok();
        resp.assert_header("Content-Type", "application/json");
        resp.assert_json(&json!({
            "avVersion": expect_json::string(),
            "dbVersion": expect_json::integer(),
            "dbSignatureCount": expect_json::integer(),
            "dbDate": expect_json::iso_date_time(),
            "results": [{
                "name": "min.pdf",
                "size": 130,
                "crc32": "d703e9d5",
                "md5": "f4e486fddb1f3d9d438926f053d53c6a",
                "sha256": "d18981866d1600d0f39eab26745e87335a1ee95a6fe5c82748d6d93604a8aa32",
                "contentType": "application/pdf",
                "dateScanned": expect_json::iso_date_time(),
                "result": "CLEAN",
                "signature": null,
            }]
        }));
    }

    #[tokio::test]
    async fn upload_multiple_files_multiple_results() {
        let cfg = app_config::load();
        let ctx = av::load_context().await;
        let app = Router::new()
            .route("/upload", post(controller::upload))
            .layer(Extension(Arc::new(cfg)))
            .layer(Extension(Arc::new(ctx)));
        let srv = TestServer::builder().mock_transport().build(app).unwrap();
        let part1 = Part::bytes(Bytes::from("Hello world!")).file_name("helloworld.txt");
        let part2 = Part::bytes(Bytes::from("Hallo Welt!")).file_name("hallowelt.txt");
        let part3 = Part::bytes(Bytes::from("Привет мир!")).file_name("приветмир.txt");
        let form = MultipartForm::new()
            .add_part("name1", part1)
            .add_part("name2", part2)
            .add_part("name3", part3);
        let resp = srv.post("/upload").multipart(form).await;
        resp.assert_status_ok();
        resp.assert_header("Content-Type", "application/json");
        resp.assert_json(&json!({
            "avVersion": expect_json::string(),
            "dbVersion": expect_json::integer(),
            "dbSignatureCount": expect_json::integer(),
            "dbDate": expect_json::iso_date_time(),
            "results": expect_json::array().len(3),
        }));
    }

    #[tokio::test]
    async fn index_html() {
        let cfg = app_config::load();
        let app = Router::new()
            .route("/index", get(controller::index_html))
            .layer(Extension(Arc::new(cfg)));
        let srv = TestServer::builder().mock_transport().build(app).unwrap();
        let resp = srv.get("/index").await;
        resp.assert_status_ok();
        resp.assert_header("Content-Type", "text/html; charset=utf-8");
        resp.assert_text_contains("<!DOCTYPE html>");
    }

    #[tokio::test]
    async fn shutdown_disabled_by_default_404() {
        let cfg = app_config::load();
        let (shutdown_tx, _) = oneshot::channel::<()>();
        let app = Router::new()
            .route("/shutdown", post(controller::shutdown))
            .layer(Extension(Arc::new(cfg)))
            .layer(Extension(Arc::new(Mutex::new(Some(shutdown_tx)))));
        let srv = TestServer::builder().mock_transport().build(app).unwrap();
        let resp = srv.post("/shutdown").await;
        resp.assert_status_not_found();
    }

    #[tokio::test]
    async fn shutdown_enabled_204() {
        let cfg = app_config::AppConfig {
            enable_shutdown_endpoint: true,
            max_file_size: 42,
            port: 8000,
        };
        let (shutdown_tx, _) = oneshot::channel::<()>();
        let app = Router::new()
            .route("/shutdown", post(controller::shutdown))
            .layer(Extension(Arc::new(cfg)))
            .layer(Extension(Arc::new(Mutex::new(Some(shutdown_tx)))));
        let srv = TestServer::builder().mock_transport().build(app).unwrap();
        let resp = srv.post("/shutdown").await;
        resp.assert_status_success();
    }
}
