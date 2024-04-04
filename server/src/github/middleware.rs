use axum::{
    body::{self, Body},
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::{self},
    response::IntoResponse,
};
use hmac::{digest::MacError, Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use sha2::Sha256;

use crate::error::AppError;

pub async fn validate_request_signature(
    State(secret): State<SecretString>,
    request: Request<Body>,
    next: middleware::Next,
) -> Result<impl IntoResponse, AppError> {
    let request = do_validate_signature(&secret, request).await?;
    Ok(next.run(request).await)
}

fn extract_signature(headers: &HeaderMap) -> Result<Vec<u8>, AppError> {
    let header = headers
        .get("X-Hub-Signature-256")
        .and_then(|value| value.to_str().ok())
        .ok_or({
            (
                StatusCode::BAD_REQUEST,
                "Missing X-Hub-Signature-256 header",
            )
        })?;
    let hex = header.strip_prefix("sha256=").ok_or({
        (
            StatusCode::BAD_REQUEST,
            "Invalid signature format, expected sha256=...",
        )
    })?;
    Ok(hex::decode(hex).map_err(|err| (StatusCode::BAD_REQUEST, format!("Invalid hex: {err}")))?)
}

fn validate_signature(
    secret: &SecretString,
    body: &[u8],
    signature: &[u8],
) -> Result<(), MacError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.expose_secret().as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(body);
    mac.verify_slice(signature)
}

// the trick is to take the request apart, buffer the body, do what you need to do, then put
// the request back together
async fn do_validate_signature(
    secret: &SecretString,
    request: Request<Body>,
) -> Result<Request<Body>, AppError> {
    let (parts, body) = request.into_parts();
    let signature = extract_signature(&parts.headers)?;
    let body = body::to_bytes(body, 1024 * 1024)
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

    if validate_signature(secret, &body, &signature).is_ok() {
        Ok(Request::from_parts(parts, Body::from(body)))
    } else {
        Err((StatusCode::BAD_REQUEST, "Invalid signature").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;
    use axum::routing::post;
    use axum::{body::Body, Router};
    use tower::ServiceExt;

    #[axum::debug_handler]
    async fn ok() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn valid_signature() {
        let app = Router::new().route(
            "/",
            post(ok).layer(middleware::from_fn_with_state(
                SecretString::new("It's a Secret to Everybody".to_string()),
                super::validate_request_signature,
            )),
        );

        let res = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .header(
                        "X-Hub-Signature-256",
                        "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17",
                    )
                    .uri("/")
                    .body(Body::from("Hello, World!"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
