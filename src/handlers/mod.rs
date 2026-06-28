pub mod artists;
pub mod auth;
pub mod edits;
pub mod mtg;

use axum::{response::{Html, IntoResponse, Response}, http::StatusCode};

/// Helper function to return error responses with proper HTTP status codes
pub fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Html(format!("<p>{}</p>", message))).into_response()
}

/// Helper function to return success responses
pub fn success_response(message: &str) -> Response {
    Html(format!("<p>{}</p>", message)).into_response()
}
