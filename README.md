# Bass Coast: The Gathering

A **web application** for mapping [Bass Coast Festival](https://basscoast.ca) artists to Magic: The Gathering cards. Built with Rust, Axum, SQLite, and Three.js.

[![Docker Build](https://github.com/sabjorn/basscoast-the-gathering/actions/workflows/docker-release.yml/badge.svg)](https://github.com/sabjorn/basscoast-the-gathering/actions/workflows/docker-release.yml)

## Overview

Browse 600+ Bass Coast artists from multiple festival years and assign each one a Magic: The Gathering card that represents their musical style. Features:

- 🎨 **3D Card Viewer** - Interactive Three.js MTG card display with front/back faces
- 🔍 **Smart Card Search** - Filter by rarity, mana cost, colors, mechanics, and text
- 👥 **Community Picks** - See what cards other users have chosen for each artist
- ✏️ **Artist Customization** - Rename artists, add metadata, merge duplicates
- 💾 **User-Specific Data** - Your selections and edits are saved per-user
- 📱 **Mobile Responsive** - Full touch support with swipe navigation

## Quick Start

### Docker (Recommended)

```bash
# Using pre-built image
docker run -d \
  -p 3000:3000 \
  -v bctg-data:/app/data \
  --name bctg \
  ghcr.io/sabjorn/bctg:latest

# Open http://localhost:3000
```

See [DOCKER.md](DOCKER.md) for complete deployment guide.

### Docker Compose

```bash
# Clone the repository
git clone https://github.com/sabjorn/basscoast-the-gathering.git
cd basscoast-the-gathering

# Start the application
docker-compose up -d

# Open http://localhost:3000
```

### Build from Source

**Prerequisites:**
- Rust 1.75+
- SQLite 3.x

```bash
# Clone the repository
git clone https://github.com/sabjorn/basscoast-the-gathering.git
cd basscoast-the-gathering

# Build release binary
cargo build --release --bin bctg

# Import artist data
./target/release/import-json data/bass_coast_artists_history.json

# Run web server
./target/release/bctg
```

Environment variables:
- `DATABASE_URL` - SQLite database path (default: `sqlite:basscoast.db`)
- `SERVER_HOST` - Bind address (default: `0.0.0.0`)
- `SERVER_PORT` - Port number (default: `3000`)
- `RUST_LOG` - Log level (default: `info`)

## Features

### Artist Browser
- Select from 600+ Bass Coast artists across multiple years
- View appearance history (which years they performed)
- Navigate with arrow buttons, keyboard shortcuts (←/→), or touch swipes
- See checkmark (✓) next to artists with saved card selections

### 3D Card Viewer
- Interactive Three.js rendering of MTG cards
- View front and back of cards
- Auto-rotate or manual drag to spin
- Official MTG card back texture
- Responsive sizing for desktop and mobile

### Smart Card Search
Filter by:
- **Rarity:** Common, Uncommon, Rare, Mythic
- **Mana Cost:** 0-16 CMC
- **Colors:** White, Blue, Black, Red, Green (any combination)
- **Mechanics:** Flying, Deathtouch, Haste, Lifelink, and 10+ more
- **Card Text:** Search card names and rules text using Scryfall syntax

Example searches:
- `o:draw o:card` - Cards that draw cards
- `t:creature flying` - Creature cards with Flying
- `name:jace` - Cards with "Jace" in the name

### Community Picks
- See what cards other users have selected for each artist
- View most popular choices
- Get inspired by the community's interpretations

### Artist Customization
- **Rename:** Change artist names for personal preference
- **Add Metadata:** Track custom fields (genre, country, etc.)
- **Merge Artists:** Combine duplicate entries
- **Delete:** Remove artists from your view
- All edits are user-specific - doesn't affect other users

### User System
- Simple name-based login (no passwords)
- Each user has their own:
  - Card selections
  - Artist renames
  - Custom metadata
  - Deleted artists list

## Technology Stack

**Backend:**
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SQLx](https://github.com/launchbadge/sqlx) - Async SQL toolkit
- [SQLite](https://www.sqlite.org/) - Embedded database
- [Tokio](https://tokio.rs/) - Async runtime

**Frontend:**
- [HTMX](https://htmx.org/) - Dynamic HTML updates
- [Three.js](https://threejs.org/) - 3D card rendering
- Vanilla JavaScript - No framework bloat

**APIs:**
- [Scryfall](https://scryfall.com/docs/api) - MTG card data and images

**Deployment:**
- Docker + Docker Compose
- GitHub Actions (automated builds)
- GitHub Container Registry (image hosting)

## Database Schema

```sql
-- Artists (base festival data)
artists (id, name, created_at)
artist_appearances (id, artist_id, year)
artist_metadata (artist_id, tags, area, type)

-- Users
users (id, name, created_at)

-- User-specific data
user_mtg_selections (user_id, artist_id, scryfall_id)
user_artist_edits (user_id, artist_id, action, new_name, merge_into_artist_id)
user_artist_metadata (user_id, artist_id, metadata_json)
```

## Project Structure

```
basscoast-the-gathering/
├── Cargo.toml                 # Rust dependencies
├── Dockerfile                 # Multi-stage Docker build
├── docker-compose.yml         # Production deployment
├── docker-entrypoint.sh       # Container startup script
│
├── src/
│   ├── main.rs               # Web server entry point
│   ├── db/                   # Database queries and models
│   ├── handlers/             # HTTP request handlers
│   ├── services/             # Business logic (MTG API, etc.)
│   └── migration/            # Data import utilities
│
├── migrations/
│   └── 001_initial_schema.sql  # Database schema
│
├── static/
│   ├── css/style.css         # Application styles
│   ├── js/card-viewer.js     # Three.js 3D card viewer
│   └── images/               # MTG card back, etc.
│
└── data/
    └── bass_coast_artists_history.json  # Artist data (imported on first run)
```

## Development

### Local Development

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone repository
git clone https://github.com/sabjorn/basscoast-the-gathering.git
cd basscoast-the-gathering

# Create .env file
cat > .env <<EOF
DATABASE_URL=sqlite:basscoast.db
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=debug
EOF

# Import data
cargo run --bin import-json -- data/bass_coast_artists_history.json

# Run development server
cargo run --bin bctg

# Open http://localhost:3000
```

### Docker Development

```bash
# Build and run with docker-compose
docker-compose -f docker-compose.dev.yml up --build

# Rebuild after code changes
docker-compose -f docker-compose.dev.yml up --build --force-recreate
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint with clippy
cargo clippy --all-targets --all-features

# Run tests
cargo test
```

## API Endpoints

### Authentication
- `GET /` - Redirect to login or app
- `GET /login` - Login page
- `POST /login` - Create/login user
- `GET /logout` - Logout and clear session

### Artists
- `GET /app` - Main application page
- `GET /artists` - List all artists (HTML select options)
- `GET /artists/:id` - Artist detail page with tabs

### MTG Card Search
- `GET /mtg/search` - Search Scryfall API with filters
- `GET /mtg/cards/:scryfall_id` - Get specific card data
- `POST /mtg/select` - Save user's card selection
- `GET /artists/:id/mtg/recommendations` - Community picks

### Artist Edits (User-Specific)
- `POST /artists/:id/rename` - Rename artist
- `POST /artists/:id/metadata` - Save custom metadata
- `POST /artists/:id/delete` - Delete artist from view
- `POST /artists/:id/merge` - Merge into another artist

## Data Source

Artist data comes from a previous analysis project that:
1. Scraped Bass Coast Festival lineups (2015-2026)
2. Analyzed festival posters using OCR
3. Compiled 600+ unique artist appearances
4. Scraped music metadata from MusicBrainz

The data is included in `data/bass_coast_artists_history.json` and automatically imported on first run.

## Deployment

See [DOCKER.md](DOCKER.md) for detailed deployment instructions including:
- Production deployment with docker-compose
- Environment variable configuration
- Data persistence and backups
- Health monitoring
- CI/CD with GitHub Actions

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo fmt` and `cargo clippy`
5. Submit a pull request

## Roadmap

- [ ] User accounts with password authentication
- [ ] Public card selection gallery
- [ ] Export to printable proxy sheets
- [ ] Custom card frame generator
- [ ] Artist photo integration
- [ ] Search history and favorites
- [ ] Mobile app (React Native)

## License

MIT License - See [LICENSE](LICENSE) for details.

## Credits

- **Bass Coast Festival** - Artist lineup data
- **Scryfall** - MTG card data and images
- **Wizards of the Coast** - Magic: The Gathering™ trademark
- **Anthropic Claude** - AI pair programming assistant

## Disclaimer

This is a fan project for educational and entertainment purposes. Magic: The Gathering is a trademark of Wizards of the Coast LLC. Bass Coast Festival is a property of Bass Coast Festival Society. No affiliation or endorsement is implied.

---

**Built with ❤️ for the Bass Coast community and MTG nerds everywhere.**
