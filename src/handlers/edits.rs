use crate::AppState;
use crate::handlers::error_response;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Form,
};
use serde::Deserialize;
use tower_cookies::Cookies;

#[derive(Deserialize)]
pub struct RenameForm {
    new_name: String,
}

#[derive(Deserialize)]
pub struct MergeForm {
    merge_into: String,
}

// POST /artists/:id/rename - Rename artist for current user
pub async fn rename_artist(
    Path(artist_id): Path<String>,
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<RenameForm>,
) -> Response {
    let user_id = match get_user_id_from_cookies(&cookies) {
        Some(id) => id,
        None => return Html("<p>Not logged in</p>".to_string()).into_response(),
    };

    match crate::db::queries::create_artist_rename(&state.pool, user_id, &artist_id, &form.new_name)
        .await
    {
        Ok(_) => {
            // Return updated artist detail HTML
            // Re-fetch artist data with new name applied
            match crate::db::queries::get_artist_by_id(&state.pool, &artist_id).await {
                Ok(_artist) => {
                    // Return simple confirmation for now
                    // TODO: Return full artist detail HTML instead
                    Html(format!("<p>Artist renamed to: {}</p>", form.new_name)).into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to fetch artist after rename: {}", e);
                    // Rename succeeded, so still return 200
                    Html("<p>Rename saved but error refreshing</p>".to_string()).into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to rename artist: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Error saving rename")
        }
    }
}

// POST /artists/:id/merge - Merge artist into another for current user
pub async fn merge_artist(
    Path(artist_id): Path<String>,
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<MergeForm>,
) -> Response {
    let user_id = match get_user_id_from_cookies(&cookies) {
        Some(id) => id,
        None => return Html("<p>Not logged in</p>".to_string()).into_response(),
    };

    match crate::db::queries::create_artist_merge(
        &state.pool,
        user_id,
        &artist_id,
        &form.merge_into,
    )
    .await
    {
        Ok(_) => Html("<p>Artist merged successfully</p>".to_string()).into_response(),
        Err(e) => {
            tracing::error!("Failed to merge artist: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Error merging artist")
        }
    }
}

// POST /artists/:id/delete - Soft delete artist for current user
pub async fn delete_artist(
    Path(artist_id): Path<String>,
    State(state): State<AppState>,
    cookies: Cookies,
) -> Response {
    let user_id = match get_user_id_from_cookies(&cookies) {
        Some(id) => id,
        None => return Html("<p>Not logged in</p>".to_string()).into_response(),
    };

    match crate::db::queries::create_artist_delete(&state.pool, user_id, &artist_id).await {
        Ok(_) => Html("<p>Artist deleted from your view</p>".to_string()).into_response(),
        Err(e) => {
            tracing::error!("Failed to delete artist: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Error deleting artist")
        }
    }
}

#[derive(Deserialize)]
pub struct MetadataForm {
    metadata_json: String,
}

// POST /artists/:id/metadata - Save user-specific metadata
pub async fn save_metadata(
    Path(artist_id): Path<String>,
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<MetadataForm>,
) -> Response {
    let user_id = match get_user_id_from_cookies(&cookies) {
        Some(id) => id,
        None => return Html("<p>Not logged in</p>".to_string()).into_response(),
    };

    // Validate JSON
    if serde_json::from_str::<serde_json::Value>(&form.metadata_json).is_err() {
        return error_response(StatusCode::BAD_REQUEST, "Invalid JSON format");
    }

    match crate::db::queries::set_user_artist_metadata(
        &state.pool,
        user_id,
        &artist_id,
        &form.metadata_json,
    )
    .await
    {
        Ok(_) => Html("<p>Metadata saved</p>".to_string()).into_response(),
        Err(e) => {
            tracing::error!("Failed to save metadata: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Error saving metadata")
        }
    }
}

// Helper function to extract user_id from cookies
fn get_user_id_from_cookies(cookies: &Cookies) -> Option<i64> {
    cookies
        .get("user_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
}
