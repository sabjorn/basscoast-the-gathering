use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ArtistAppearance {
    pub id: Option<i64>,
    pub artist_id: String,
    pub year: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ArtistMetadata {
    pub artist_id: String,
    pub found_musicbrainz: i64,
    pub found_discogs: i64,
    pub found_lastfm: i64,
    pub tags: Option<String>,   // JSON array as string
    pub area: Option<String>,
    #[sqlx(rename = "type")]
    pub artist_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserMTGSelection {
    pub id: i64,
    pub user_id: i64,
    pub artist_id: String,
    pub scryfall_id: String,
    pub created_at: String,
    pub updated_at: String,
}
