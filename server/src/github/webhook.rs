use std::convert::Infallible;

use super::middleware::validate_request_signature;
use crate::error::AppError;
use crate::hydra::client::HydraClient;
use axum::extract::State;
use axum::middleware;
use axum::routing::post;
use secrecy::SecretString;

#[tracing::instrument(skip_all, err)]
async fn webhook(State(client): State<HydraClient>, event: String) -> Result<(), AppError> {
    tracing::info!("Received push event from {event:?}");
    tracing::trace!(?event);
    let response = client.push(event).await?;
    tracing::info!(?response);
    Ok(())
}

pub fn handler(secret: SecretString) -> axum::routing::MethodRouter<HydraClient, Infallible> {
    post(webhook).route_layer(middleware::from_fn_with_state(
        secret,
        validate_request_signature,
    ))
}
