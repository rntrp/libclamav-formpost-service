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
use tokio::{
    main, select, signal,
    sync::{
        oneshot::{self, Receiver},
        Mutex,
    },
};
use tower_http::trace::TraceLayer;

#[main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("libclamav formpost service is starting...");

    let cfg = app_config::load();
    tracing::info!("Loaded config\n{}", cfg);

    let ctx = av::load_context();
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
        .layer(prometheus_layer)
        .into_make_service();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Bound to {}", addr);

    axum::Server::bind(&addr)
        .serve(app)
        .with_graceful_shutdown(shutdown_signal(shutdown_rx))
        .await
        .unwrap();
}

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
