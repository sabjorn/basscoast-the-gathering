use crate::AppState;
use crate::handlers::{error_response, success_response};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    Form,
};
use serde::Deserialize;
use tower_cookies::Cookies;

#[derive(Deserialize)]
pub struct SearchParams {
    rarity: Option<String>,
    colors: Option<String>,
    cmc: Option<f64>,
    q: Option<String>,
    mechanic: Option<String>,
    artist_id: Option<String>,
}

#[derive(Deserialize)]
pub struct SelectForm {
    artist_id: String,
    scryfall_id: String,
}

// GET /mtg/search - Search MTG cards with filters via Scryfall API
pub async fn search_cards(
    Query(params): Query<SearchParams>,
    State(state): State<AppState>,
) -> Response {
    // Build Scryfall query string
    let query = crate::services::mtg_service::build_search_query(
        params.rarity.as_deref(),
        params.colors.as_deref(),
        params.cmc,
        params.q.as_deref(),
        params.mechanic.as_deref(),
    );

    if query.is_empty() {
        return Html("<li>Enter search criteria</li>".to_string()).into_response();
    }

    tracing::info!("Scryfall query: {}", query);

    // Call Scryfall API
    match state.scryfall.search_cards(&query).await {
        Ok(cards) => {
            if cards.is_empty() {
                return Html("<li>No cards found</li>".to_string()).into_response();
            }

            let html = cards
                .iter()
                .map(|card| {
                    if let Some(ref artist_id) = params.artist_id {
                        format!(
                            r##"<li class="card-item" data-scryfall-id="{}" data-artist-id="{}">
                                {} ({})
                                <br><small>{}</small>
                            </li>"##,
                            card.id, artist_id, card.name, card.rarity, card.type_line
                        )
                    } else {
                        format!(
                            r##"<li class="card-item" data-scryfall-id="{}">
                                {} ({})
                                <br><small>{}</small>
                            </li>"##,
                            card.id, card.name, card.rarity, card.type_line
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            Html(format!("<ul class=\"card-list\">{}</ul>", html)).into_response()
        }
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Failed to search cards: {}", error_msg);

            // Provide user-friendly error messages with proper HTTP status codes
            if error_msg.contains("404") {
                error_response(
                    StatusCode::NOT_FOUND,
                    "<li>No cards found matching your search criteria</li>",
                )
            } else if error_msg.contains("400") {
                error_response(
                    StatusCode::BAD_REQUEST,
                    "<li>Invalid search query. Try adjusting your filters</li>",
                )
            } else if error_msg.contains("503") || error_msg.contains("500") {
                error_response(
                    StatusCode::BAD_GATEWAY,
                    "<li>Scryfall API is temporarily unavailable. Try again in a moment</li>",
                )
            } else {
                error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "<li>Unable to search cards. Please try again</li>",
                )
            }
        }
    }
}

// GET /mtg/cards/:scryfall_id - Get card details as JSON from Scryfall API
pub async fn get_card_json(
    Path(scryfall_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.scryfall.get_card(&scryfall_id).await {
        Ok(card) => {
            // Extract normal image URL
            let image_uri = card
                .image_uris
                .as_ref()
                .and_then(|uris| uris.normal.clone())
                .unwrap_or_default();

            // Return card data for the 3D viewer
            let response = serde_json::json!({
                "scryfall_id": card.id,
                "name": card.name,
                "rarity": card.rarity,
                "type_line": card.type_line,
                "image_uri": image_uri
            });
            Json(response).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch card from Scryfall: {}", e);
            (axum::http::StatusCode::NOT_FOUND, "Card not found").into_response()
        }
    }
}

// POST /mtg/select - Save user's card selection (scryfall_id only)
pub async fn select_card(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<SelectForm>,
) -> Response {
    let user_id = match get_user_id_from_cookies(&cookies) {
        Some(id) => id,
        None => return Html("<p>Not logged in</p>".to_string()).into_response(),
    };

    match crate::db::queries::set_user_mtg_selection(
        &state.pool,
        user_id,
        &form.artist_id,
        &form.scryfall_id,
    )
    .await
    {
        Ok(_) => success_response("Selection saved"),
        Err(e) => {
            tracing::error!("Failed to save selection: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Error saving selection")
        }
    }
}

// GET /artists/:id/mtg/recommendations - Get top 5 community picks (cards selected by other users)
pub async fn get_recommendations(
    Path(artist_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    // Get top 5 cards selected by users for this artist, ordered by popularity
    let popular_picks = match sqlx::query!(
        r#"SELECT scryfall_id, COUNT(*) as pick_count
         FROM user_mtg_selections
         WHERE artist_id = ?
         GROUP BY scryfall_id
         ORDER BY pick_count DESC
         LIMIT 5"#,
        artist_id
    )
    .fetch_all(&state.pool)
    .await
    {
        Ok(picks) => picks,
        Err(e) => {
            tracing::error!("Failed to fetch community picks: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error loading community picks",
            );
        }
    };

    if popular_picks.is_empty() {
        return Html("<p>No community picks yet. Be the first to select a card!</p>".to_string()).into_response();
    }

    // Fetch card details from Scryfall API for each pick
    let mut cards_html = Vec::new();
    for pick in popular_picks {
        match state.scryfall.get_card(&pick.scryfall_id).await {
            Ok(card) => {
                let pick_count = pick.pick_count;
                let pick_text = if pick_count == 1 {
                    "1 pick".to_string()
                } else {
                    format!("{} picks", pick_count)
                };

                cards_html.push(format!(
                    r##"<li class="card-item" data-scryfall-id="{}" data-artist-id="{}">
                        {} ({})
                        <br><small>👥 {}</small>
                    </li>"##,
                    card.id, artist_id, card.name, card.rarity, pick_text
                ));
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch card {} from Scryfall: {}",
                    pick.scryfall_id,
                    e
                );
                // Skip this card and continue
            }
        }
    }

    if cards_html.is_empty() {
        Html("<p>Error loading community picks</p>".to_string()).into_response()
    } else {
        Html(format!(
            "<ul class=\"card-list\">{}</ul>",
            cards_html.join("\n")
        )).into_response()
    }
}

// Helper function
fn get_user_id_from_cookies(cookies: &Cookies) -> Option<i64> {
    cookies
        .get("user_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
}
