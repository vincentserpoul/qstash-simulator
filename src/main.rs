use axum::{
    error_handling::HandleErrorLayer, extract::State, http::StatusCode, response::IntoResponse,
    routing::post, Json, Router,
};
use serde::Serialize;
use std::env;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::mpsc::Sender;
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::{self, TraceLayer};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    tx: Sender<(String, String)>,
    in_container: bool,
}

#[tokio::main]
async fn main() {
    // init env
    let in_container = env::var("IN_CONTAINER").is_ok();
    let port: u16 = if let Ok(p) = env::var("PORT") {
        p.parse().unwrap()
    } else {
        3033
    };

    // init tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "qstash=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    let state = AppState { tx, in_container };

    tokio::spawn(async move {
        while let Some((target, body)) = rx.recv().await {
            // sleep 1 second
            tokio::time::sleep(Duration::from_secs(1)).await;

            tracing::info!("sending request to {}", target);
            // send an POST request to the target with the body
            let res = reqwest::Client::new()
                .post(target)
                .body(body)
                .send()
                .await
                .unwrap();

            tracing::info!("res: {}", res.status());
        }
    });

    // Compose the routes
    let app = Router::new()
        .route("/v2/publish/*url", post(publish))
        .route("/v1/publish/*url", post(publish))
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {}", error),
                        ))
                    }
                }))
                .timeout(Duration::from_secs(1))
                .into_inner(),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state);

    let app = app.fallback(handler_404);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}, in container: {}", addr, in_container);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn publish(
    State(state): State<AppState>,
    uri: axum::extract::Path<String>,
    body: String,
) -> impl IntoResponse {
    // print the request uri without v1/publish
    let target = uri.as_str().replace("v1/publish/", "");
    let mut target = target.replace("v2/publish/", "");

    if state.in_container {
        target = target.replace("localhost", "host.docker.internal");
    }

    state.tx.send((target, body)).await.unwrap();

    (
        StatusCode::CREATED,
        Json(Message {
            message_id: Uuid::new_v4().to_string(),
        }),
    )
}

#[derive(Debug, Serialize)]
struct Message {
    #[serde(rename(serialize = "messageId"))]
    message_id: String,
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}
