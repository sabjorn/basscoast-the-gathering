use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::time::Duration;

/// Enrich artist metadata from MusicBrainz API
#[derive(Parser, Debug)]
#[command(name = "enrich-metadata")]
#[command(about = "Fetch artist metadata from MusicBrainz API", long_about = None)]
struct Cli {
    /// Only process N artists (for testing)
    #[arg(short, long)]
    limit: Option<usize>,

    /// Skip artists that already have metadata
    #[arg(long, default_value = "true")]
    skip_existing: bool,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzResponse {
    artists: Vec<MusicBrainzArtist>,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzArtist {
    #[allow(dead_code)]
    name: String,
    #[serde(rename = "type")]
    artist_type: Option<String>,
    area: Option<MusicBrainzArea>,
    tags: Option<Vec<MusicBrainzTag>>,
    #[serde(default)]
    genres: Vec<MusicBrainzGenre>,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzArea {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzTag {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzGenre {
    name: String,
}

struct ArtistRow {
    id: String,
    name: String,
}

const MUSICBRAINZ_API: &str = "https://musicbrainz.org/ws/2";
const RATE_LIMIT_DELAY: Duration = Duration::from_millis(1000); // 1 req/sec

async fn search_musicbrainz(client: &reqwest::Client, artist_name: &str) -> Result<Option<MusicBrainzArtist>> {
    let url = format!("{}/artist", MUSICBRAINZ_API);

    let response = client
        .get(&url)
        .query(&[
            ("query", format!("artist:\"{}\"", artist_name)),
            ("fmt", "json".to_string()),
            ("limit", "1".to_string()),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        tracing::warn!("MusicBrainz API error: {}", response.status());
        return Ok(None);
    }

    let data: MusicBrainzResponse = response.json().await?;

    Ok(data.artists.into_iter().next())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    println!("=== Artist Metadata Enrichment ===\n");

    // Connect to database
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = SqlitePool::connect(&database_url).await?;

    // Query artists that need metadata
    let query = if cli.skip_existing {
        "SELECT a.id, a.name FROM artists a
         LEFT JOIN artist_metadata am ON a.id = am.artist_id
         WHERE am.artist_id IS NULL OR am.found_musicbrainz = 0
         ORDER BY a.name"
    } else {
        "SELECT id, name FROM artists ORDER BY name"
    };

    let artists: Vec<ArtistRow> = sqlx::query_as::<_, (String, String)>(query)
        .fetch_all(&pool)
        .await?
        .into_iter()
        .map(|(id, name)| ArtistRow { id, name })
        .collect();

    let total = if let Some(limit) = cli.limit {
        artists.len().min(limit)
    } else {
        artists.len()
    };

    println!("Processing {} artists...\n", total);

    // Create HTTP client with user agent
    let client = reqwest::Client::builder()
        .user_agent("BassCoastMTGMapper/1.0 (https://github.com/sabjorn/basscoast)")
        .timeout(Duration::from_secs(10))
        .build()?;

    let mut stats = Stats::default();

    for (idx, artist) in artists.iter().take(total).enumerate() {
        if (idx + 1) % 10 == 0 || idx == 0 {
            println!("Progress: {}/{} - {}", idx + 1, total, &artist.name);
        }

        // Rate limiting
        if idx > 0 {
            tokio::time::sleep(RATE_LIMIT_DELAY).await;
        }

        // Search MusicBrainz
        match search_musicbrainz(&client, &artist.name).await {
            Ok(Some(mb_artist)) => {
                // Merge tags and genres
                let mut all_tags: Vec<String> = mb_artist.genres.iter().map(|g| g.name.clone()).collect();
                if let Some(tags) = &mb_artist.tags {
                    all_tags.extend(tags.iter().map(|t| t.name.clone()));
                }

                let tags_json = if all_tags.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&all_tags)?)
                };

                let area = mb_artist.area.as_ref().map(|a| a.name.clone());

                // Upsert metadata
                sqlx::query!(
                    r#"
                    INSERT INTO artist_metadata (artist_id, found_musicbrainz, found_discogs, found_lastfm, tags, area, type)
                    VALUES (?, 1, 0, 0, ?, ?, ?)
                    ON CONFLICT(artist_id) DO UPDATE SET
                        found_musicbrainz = 1,
                        tags = COALESCE(excluded.tags, tags),
                        area = COALESCE(excluded.area, area),
                        type = COALESCE(excluded.type, type)
                    "#,
                    artist.id,
                    tags_json,
                    area,
                    mb_artist.artist_type
                )
                .execute(&pool)
                .await?;

                stats.found_musicbrainz += 1;
                if !all_tags.is_empty() {
                    stats.total_with_tags += 1;
                }
            }
            Ok(None) => {
                // Not found - still insert a record so we don't query again
                sqlx::query!(
                    r#"
                    INSERT INTO artist_metadata (artist_id, found_musicbrainz, found_discogs, found_lastfm, tags, area, type)
                    VALUES (?, 0, 0, 0, NULL, NULL, NULL)
                    ON CONFLICT(artist_id) DO UPDATE SET found_musicbrainz = 0
                    "#,
                    artist.id
                )
                .execute(&pool)
                .await?;
            }
            Err(e) => {
                tracing::warn!("Error fetching metadata for {}: {}", artist.name, e);
            }
        }

        stats.processed += 1;
    }

    // Print summary
    println!("\n{}", "=".repeat(70));
    println!("METADATA ENRICHMENT COMPLETE");
    println!("{}", "=".repeat(70));
    println!("\nTotal artists processed: {}", stats.processed);
    println!("Found in MusicBrainz: {} ({:.1}%)",
        stats.found_musicbrainz,
        (stats.found_musicbrainz as f64 / stats.processed as f64) * 100.0
    );
    println!("Artists with tags: {} ({:.1}%)",
        stats.total_with_tags,
        (stats.total_with_tags as f64 / stats.processed as f64) * 100.0
    );
    println!("{}", "=".repeat(70));

    Ok(())
}

#[derive(Default)]
struct Stats {
    processed: usize,
    found_musicbrainz: usize,
    total_with_tags: usize,
}
