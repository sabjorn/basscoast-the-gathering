mod db;
mod handlers;
mod services;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub scryfall: Arc<services::mtg_service::ScryfallClient>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "basscoast_web=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database connection
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    tracing::info!("Database migrations complete");

    // Create Scryfall API client
    let scryfall_client = Arc::new(services::mtg_service::ScryfallClient::new());

    // Create app state
    let state = AppState {
        pool,
        scryfall: scryfall_client,
    };

    // Build our application with routes
    // Must call with_state BEFORE layers in Axum 0.7
    let app = Router::new()
        .route("/", get(handlers::auth::index))
        .route("/login", get(handlers::auth::login_page))
        .route("/login", post(handlers::auth::login_submit))
        .route("/logout", get(handlers::auth::logout))
        .route("/app", get(handlers::auth::app_page))
        .route("/artists", get(handlers::artists::list_artists))
        .route("/artists/:id", get(handlers::artists::artist_detail))
        .route("/artists/:id/rename", post(handlers::edits::rename_artist))
        .route("/artists/:id/merge", post(handlers::edits::merge_artist))
        .route("/artists/:id/delete", post(handlers::edits::delete_artist))
        .route("/artists/:id/metadata", post(handlers::edits::save_metadata))
        .route("/mtg/search", get(handlers::mtg::search_cards))
        .route("/mtg/cards/:scryfall_id", get(handlers::mtg::get_card_json))
        .route("/mtg/select", post(handlers::mtg::select_card))
        .route(
            "/artists/:id/mtg/recommendations",
            get(handlers::mtg::get_recommendations),
        )
        .with_state(state)
        .layer(CookieManagerLayer::new())
        .layer(TraceLayer::new_for_http())
        .nest_service("/static", ServeDir::new("static"));

    // Run server
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;

    // Bind to 0.0.0.0 by default to allow network access
    // Set SERVER_HOST=127.0.0.1 to restrict to localhost only
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
