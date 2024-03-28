use std::convert::Infallible;

use super::events::PushEvent;
use super::middleware::validate_signature;
use crate::error::AppError;
use crate::hydra::client::HydraClient;
use axum::extract::Json;
use axum::extract::State;
use axum::middleware;
use axum::routing::post;
use secrecy::SecretString;

#[tracing::instrument(skip(client), err)]
async fn webhook(
    State(client): State<HydraClient>,
    event: Json<PushEvent>,
) -> Result<(), AppError> {
    let Some(branch) = event.branch() else {
        tracing::info!("Ignoring push");
        return Ok(());
    };

    let repo = &event.repository.name;
    tracing::info!(?event, "Handling push to {repo}/{branch}");
    client.push(repo, branch).await?;
    Ok(())
}

pub fn handler(secret: SecretString) -> axum::routing::MethodRouter<HydraClient, Infallible> {
    post(webhook).route_layer(middleware::from_fn_with_state(secret, validate_signature))
}
