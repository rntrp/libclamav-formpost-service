mod app_config;
mod av;
mod av_bindings;
mod av_engine;
mod av_settings;
mod controller;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Extension, Router,
};
use axum_prometheus::PrometheusMetricLayer;
use std::{net::SocketAddr, sync::Arc};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("libclamav formpost service is starting...");

    let cfg = app_config::load();
    tracing::info!("Loaded config\n{}", cfg);

    let ctx = av::load_context();
    tracing::info!("Loaded context\n{}", ctx);

    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .route("/", get(controller::index_html))
        .route("/index.htm", get(controller::index_html))
        .route("/index.html", get(controller::index_html))
        .route("/upload", post(controller::upload))
        .layer(Extension(Arc::new(ctx)))
        .layer(DefaultBodyLimit::max(cfg.max_file_size))
        .layer(TraceLayer::new_for_http())
        .layer(prometheus_layer)
        .into_make_service();
    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
    tracing::info!("Bound to {}", addr);

    axum::Server::bind(&addr).serve(app).await.unwrap();
}
