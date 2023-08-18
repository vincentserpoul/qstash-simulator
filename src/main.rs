use axum::{
    error_handling::HandleErrorLayer, extract::State, http::StatusCode, response::IntoResponse,
    routing::post, Json, Router,
};
use serde::Serialize;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::mpsc::Sender;
use tower::{BoxError, ServiceBuilder};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "qstash=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

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
                .timeout(Duration::from_secs(10))
                .into_inner(),
        )
        .with_state(tx);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3033));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn publish(
    State(tx): State<Sender<(String, String)>>,
    uri: axum::extract::Path<String>,
    body: String,
) -> impl IntoResponse {
    // print the request uri without v1/publish
    let target = uri.as_str().replace("/v1/publish/", "");

    tx.send((target, body)).await.unwrap();

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
