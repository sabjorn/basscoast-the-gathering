use crate::db::models::*;
use sqlx::{Result, SqlitePool};

// User operations
pub async fn get_or_create_user(pool: &SqlitePool, name: &str) -> Result<User> {
    // Try to get existing user
    if let Ok(user) = sqlx::query_as!(
        User,
        r#"SELECT id as "id: i64", name, created_at as "created_at: String" FROM users WHERE name = ?"#,
        name
    )
    .fetch_one(pool)
    .await
    {
        return Ok(user);
    }

    // Create new user
    sqlx::query!("INSERT INTO users (name) VALUES (?)", name)
        .execute(pool)
        .await?;

    sqlx::query_as!(
        User,
        r#"SELECT id as "id: i64", name, created_at as "created_at: String" FROM users WHERE name = ?"#,
        name
    )
    .fetch_one(pool)
    .await
}

// Artist operations
pub async fn get_all_artists(pool: &SqlitePool) -> Result<Vec<Artist>> {
    sqlx::query_as!(
        Artist,
        r#"SELECT id as "id: String", name, created_at as "created_at: String" FROM artists ORDER BY name"#
    )
    .fetch_all(pool)
    .await
}

pub async fn get_artist_by_id(pool: &SqlitePool, artist_id: &str) -> Result<Artist> {
    sqlx::query_as!(
        Artist,
        r#"SELECT id as "id: String", name, created_at as "created_at: String" FROM artists WHERE id = ?"#,
        artist_id
    )
    .fetch_one(pool)
    .await
}

pub async fn get_artist_appearances(
    pool: &SqlitePool,
    artist_id: &str,
) -> Result<Vec<ArtistAppearance>> {
    sqlx::query_as!(
        ArtistAppearance,
        r#"SELECT id, artist_id, year
         FROM artist_appearances
         WHERE artist_id = ?
         ORDER BY year"#,
        artist_id
    )
    .fetch_all(pool)
    .await
}

pub async fn get_artist_metadata(
    pool: &SqlitePool,
    artist_id: &str,
) -> Result<Option<ArtistMetadata>> {
    sqlx::query_as!(
        ArtistMetadata,
        r#"SELECT artist_id as "artist_id: String",
                found_musicbrainz as "found_musicbrainz: i64",
                found_discogs as "found_discogs: i64",
                found_lastfm as "found_lastfm: i64",
                tags, area,
                type as "artist_type: String"
         FROM artist_metadata
         WHERE artist_id = ?"#,
        artist_id
    )
    .fetch_optional(pool)
    .await
}

// User edit operations
pub async fn create_artist_rename(
    pool: &SqlitePool,
    user_id: i64,
    artist_id: &str,
    new_name: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO user_artist_edits (user_id, artist_id, action, new_name)
         VALUES (?, ?, 'rename', ?)",
        user_id,
        artist_id,
        new_name
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_artist_merge(
    pool: &SqlitePool,
    user_id: i64,
    artist_id: &str,
    merge_into: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO user_artist_edits (user_id, artist_id, action, merge_into_artist_id)
         VALUES (?, ?, 'merge', ?)",
        user_id,
        artist_id,
        merge_into
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_artist_delete(pool: &SqlitePool, user_id: i64, artist_id: &str) -> Result<()> {
    sqlx::query!(
        "INSERT INTO user_artist_edits (user_id, artist_id, action)
         VALUES (?, ?, 'delete')",
        user_id,
        artist_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

// User MTG selection operations
pub async fn get_user_mtg_selection(
    pool: &SqlitePool,
    user_id: i64,
    artist_id: &str,
) -> Result<Option<UserMTGSelection>> {
    sqlx::query_as!(
        UserMTGSelection,
        r#"SELECT id as "id: i64", user_id as "user_id: i64",
                artist_id, scryfall_id,
                created_at as "created_at: String",
                updated_at as "updated_at: String"
         FROM user_mtg_selections
         WHERE user_id = ? AND artist_id = ?"#,
        user_id,
        artist_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn set_user_mtg_selection(
    pool: &SqlitePool,
    user_id: i64,
    artist_id: &str,
    scryfall_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO user_mtg_selections (user_id, artist_id, scryfall_id, updated_at)
         VALUES (?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(user_id, artist_id) DO UPDATE SET
         scryfall_id = excluded.scryfall_id,
         updated_at = CURRENT_TIMESTAMP",
        user_id,
        artist_id,
        scryfall_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

// User artist metadata operations
pub async fn get_user_artist_metadata(
    pool: &SqlitePool,
    user_id: i64,
    artist_id: &str,
) -> Result<Option<String>> {
    let result = sqlx::query!(
        r#"SELECT metadata_json FROM user_artist_metadata
         WHERE user_id = ? AND artist_id = ?"#,
        user_id,
        artist_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|r| r.metadata_json))
}

pub async fn set_user_artist_metadata(
    pool: &SqlitePool,
    user_id: i64,
    artist_id: &str,
    metadata_json: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO user_artist_metadata (user_id, artist_id, metadata_json, updated_at)
         VALUES (?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(user_id, artist_id) DO UPDATE SET
         metadata_json = excluded.metadata_json,
         updated_at = CURRENT_TIMESTAMP",
        user_id,
        artist_id,
        metadata_json
    )
    .execute(pool)
    .await?;

    Ok(())
}
