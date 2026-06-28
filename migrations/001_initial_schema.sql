-- Bass Coast: The Gathering - Initial Database Schema

-- Artists table (base artist data)
CREATE TABLE IF NOT EXISTS artists (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Artist appearances table (festival performances)
CREATE TABLE IF NOT EXISTS artist_appearances (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    artist_id TEXT NOT NULL,
    year TEXT NOT NULL,
    FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE
);

-- Users table (simple name-based user tracking)
CREATE TABLE IF NOT EXISTS users (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Artist metadata table (MusicBrainz data)
CREATE TABLE IF NOT EXISTS artist_metadata (
    artist_id TEXT NOT NULL PRIMARY KEY,
    found_musicbrainz INTEGER NOT NULL DEFAULT 0,
    found_discogs INTEGER NOT NULL DEFAULT 0,
    found_lastfm INTEGER NOT NULL DEFAULT 0,
    tags TEXT,  -- JSON array
    area TEXT,
    type TEXT,
    FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE
);

-- User artist edits table (user-specific artist modifications)
CREATE TABLE IF NOT EXISTS user_artist_edits (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    artist_id TEXT NOT NULL,
    action TEXT NOT NULL,
    new_name TEXT,
    merge_into_artist_id TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE,
    FOREIGN KEY (merge_into_artist_id) REFERENCES artists(id) ON DELETE SET NULL
);

-- User MTG selections table (user's selected MTG card per artist)
-- Note: scryfall_id is NOT a foreign key - all card data comes from Scryfall API
CREATE TABLE IF NOT EXISTS user_mtg_selections (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    artist_id TEXT NOT NULL,
    scryfall_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE,
    UNIQUE(user_id, artist_id)
);

-- User-specific artist metadata overrides
CREATE TABLE IF NOT EXISTS user_artist_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    artist_id TEXT NOT NULL,
    metadata_json TEXT NOT NULL,  -- JSON object with user's metadata fields
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE,
    UNIQUE(user_id, artist_id)
);

-- Indices for better query performance

-- Artist indices
CREATE INDEX IF NOT EXISTS idx_artist_name ON artists(name);
CREATE INDEX IF NOT EXISTS idx_artist_metadata_artist ON artist_metadata(artist_id);

-- Appearance indices
CREATE INDEX IF NOT EXISTS idx_appearance_artist ON artist_appearances(artist_id);
CREATE INDEX IF NOT EXISTS idx_appearance_year ON artist_appearances(year);

-- User edit indices
CREATE INDEX IF NOT EXISTS idx_edit_user ON user_artist_edits(user_id);
CREATE INDEX IF NOT EXISTS idx_edit_artist ON user_artist_edits(artist_id);

-- User selection indices
CREATE INDEX IF NOT EXISTS idx_selection_user ON user_mtg_selections(user_id);
CREATE INDEX IF NOT EXISTS idx_selection_artist ON user_mtg_selections(artist_id);

-- User metadata indices
CREATE INDEX IF NOT EXISTS idx_user_artist_metadata_user ON user_artist_metadata(user_id);
CREATE INDEX IF NOT EXISTS idx_user_artist_metadata_artist ON user_artist_metadata(artist_id);
