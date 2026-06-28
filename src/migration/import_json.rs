use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Import Bass Coast artist data from JSON into SQLite database
#[derive(Parser, Debug)]
#[command(name = "import-json")]
#[command(about = "Import artist data from JSON to SQLite", long_about = None)]
struct Cli {
    /// Path to the artist history JSON file
    #[arg(value_name = "FILE")]
    file_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct BassCoastArtistsHistory {
    artists: HashMap<String, ArtistData>,
}

#[derive(Debug, Deserialize)]
struct ArtistData {
    appearances: Vec<AppearanceData>,
    metadata: Option<MetadataEntry>,
}

#[derive(Debug, Deserialize)]
struct AppearanceData {
    year: String,
}

#[derive(Debug, Deserialize)]
struct MetadataEntry {
    found_musicbrainz: Option<bool>,
    found_discogs: Option<bool>,
    found_lastfm: Option<bool>,
    tags: Option<Vec<String>>,
    area: Option<String>,
    #[serde(rename = "type")]
    artist_type: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment
    dotenvy::dotenv().ok();

    println!("Starting JSON to SQLite migration...");

    // Connect to database
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = SqlitePool::connect(&database_url).await?;

    println!("Connected to database");

    // Run migrations first
    println!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    println!("Migrations complete");

    // Parse CLI arguments
    let cli = Cli::parse();

    // Load artist data
    println!("\n=== Importing Bass Coast artists ===");
    println!("Loading data from: {}", cli.file_path.display());

    let data: BassCoastArtistsHistory =
        serde_json::from_str(&std::fs::read_to_string(&cli.file_path)?)?;

    let mut artist_count = 0;
    let mut appearance_count = 0;
    let mut metadata_count = 0;

    for (artist_name, artist_data) in data.artists {
        let artist_id = Uuid::new_v4().to_string();

        // Insert artist
        sqlx::query!(
            "INSERT INTO artists (id, name) VALUES (?, ?)",
            artist_id,
            artist_name
        )
        .execute(&pool)
        .await?;

        artist_count += 1;

        // Insert appearances
        for appearance in artist_data.appearances {
            sqlx::query!(
                r#"
                INSERT INTO artist_appearances (artist_id, year)
                VALUES (?, ?)
                "#,
                artist_id,
                appearance.year
            )
            .execute(&pool)
            .await?;

            appearance_count += 1;
        }

        // Insert metadata if available
        if let Some(metadata) = artist_data.metadata {
            let tags_json = metadata
                .tags
                .map(|t| serde_json::to_string(&t).unwrap_or_default());

            sqlx::query!(
                r#"
                INSERT INTO artist_metadata (
                    artist_id, found_musicbrainz, found_discogs, found_lastfm,
                    tags, area, type
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
                artist_id,
                metadata.found_musicbrainz,
                metadata.found_discogs,
                metadata.found_lastfm,
                tags_json,
                metadata.area,
                metadata.artist_type
            )
            .execute(&pool)
            .await?;

            metadata_count += 1;
        }
    }

    println!("\n=== Migration complete! ===");
    println!("Summary:");
    println!("  - {} artists", artist_count);
    println!("  - {} appearances", appearance_count);
    println!("  - {} metadata entries", metadata_count);

    Ok(())
}
