use axum::response::{IntoResponse, Response};
use core::fmt;
use reqwest::StatusCode;
use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub enum AppError {
    StatusCodeMessage(StatusCode, Cow<'static, str>),
    InternalServerError(anyhow::Error),
}

impl<S> From<(StatusCode, S)> for AppError
where
    S: Into<Cow<'static, str>>,
{
    fn from((status, message): (StatusCode, S)) -> Self {
        AppError::StatusCodeMessage(status, message.into())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::InternalServerError(e)
    }
}

// Tell axum how `AppError` should be converted into a response.
//
// This is also a convenient place to log errors.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::InternalServerError(err) => {
                tracing::error!(%err, "Internal server error");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            AppError::StatusCodeMessage(status, message) => {
                tracing::error!(%status, %message);
                (status, message).into_response()
            }
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AppError::InternalServerError(err) => write!(f, "Internal server error: {}", err),
            AppError::StatusCodeMessage(status, message) => {
                write!(f, "{}: {}", status, message)
            }
        }
    }
}
