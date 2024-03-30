use crate::error::AppError;
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    middleware,
    response::{IntoResponse, Response},
};
use ipnet::IpNet;
use reqwest::StatusCode;
use std::net::SocketAddr;

pub async fn allowed_ips(
    State(allowed): State<Vec<IpNet>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: middleware::Next,
) -> Result<Response, AppError> {
    if !allowed
        .iter()
        .any(|ip| ip.contains(&IpNet::from(addr.ip())))
    {
        tracing::info!("Denying connection from IP: {addr}");
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    Ok(next.run(request).await)
}
