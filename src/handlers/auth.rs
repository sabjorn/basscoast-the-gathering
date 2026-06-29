use crate::AppState;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;
use tower_cookies::{Cookie, Cookies};

#[derive(Deserialize)]
pub struct LoginForm {
    name: String,
}

// GET / - Landing page, redirect to login if no session
pub async fn index(cookies: Cookies) -> impl IntoResponse {
    if let Some(cookie) = cookies.get("user_id") {
        if cookie.value().parse::<i64>().is_ok() {
            return Redirect::to("/app");
        }
    }
    Redirect::to("/login")
}

// GET /login - Login page
pub async fn login_page() -> impl IntoResponse {
    Html(
        r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Bass Coast: The Gathering - Login</title>
    <link rel="stylesheet" href="/static/css/style.css?v=2">
</head>
<body>
    <div class="login-container">
        <h1>Bass Coast: The Gathering</h1>
        <form method="POST" action="/login">
            <label for="name">Enter your name:</label>
            <input type="text" name="name" id="name" required autofocus>
            <button type="submit">Start</button>
        </form>
    </div>
</body>
</html>
    "#,
    )
}

// POST /login - Create/get user and set cookie
pub async fn login_submit(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    match crate::db::queries::get_or_create_user(&state.pool, &form.name).await {
        Ok(user) => {
            let mut cookie = Cookie::new("user_id", user.id.to_string());
            cookie.set_path("/");
            cookies.add(cookie);

            Redirect::to("/app")
        }
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            Redirect::to("/login")
        }
    }
}

// GET /app - Main application page
pub async fn app_page(cookies: Cookies) -> impl IntoResponse {
    // Check if user is logged in
    if cookies.get("user_id").is_none() {
        return Redirect::to("/login").into_response();
    }

    let html = r##"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Bass Coast: The Gathering - Artist Mapper</title>
    <link rel="stylesheet" href="/static/css/style.css?v=2">
    <script src="https://unpkg.com/htmx.org@2.0.3"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/three.js/r128/three.min.js"></script>
</head>
<body>
    <div class="app-container">
        <!-- Welcome Modal -->
        <div id="welcome-modal" class="modal-overlay">
            <div class="modal-content">
                <h2>Welcome to Bass Coast: The Gathering! 🎵✨</h2>
                <p>Create your personalized artist collection by selecting Magic: The Gathering cards that represent each musician.</p>

                <div class="welcome-steps">
                    <div class="welcome-step">
                        <span class="step-number">1</span>
                        <div class="step-text">
                            <strong>Select an Artist</strong>
                            <p>Choose from the dropdown in the left panel</p>
                        </div>
                    </div>
                    <div class="welcome-step">
                        <span class="step-number">2</span>
                        <div class="step-text">
                            <strong>Pick a Card</strong>
                            <p>Browse recommendations or search for the perfect card</p>
                        </div>
                    </div>
                    <div class="welcome-step">
                        <span class="step-number">3</span>
                        <div class="step-text">
                            <strong>Save & Repeat</strong>
                            <p>Your selections are saved automatically</p>
                        </div>
                    </div>
                </div>

                <button id="get-started-btn" class="primary-button">Get Started</button>
                <p class="modal-hint">Tip: Use ← → arrow keys or swipe to navigate between artists</p>
            </div>
        </div>

        <header>
            <div class="header-left">
                <h1>Bass Coast: The Gathering</h1>
            </div>
            <div class="header-right">
                <button id="logout-button" class="logout-button">Logout</button>
            </div>
        </header>

        <main>
            <div class="left-panel">
                <div class="artist-selector">
                    <label for="artist-select">Select Artist:</label>
                    <select id="artist-select">
                        <option value="">Choose an artist...</option>
                    </select>
                </div>
                <div id="artist-detail">
                    <div class="empty-state">
                        <div class="empty-state-icon">🎵</div>
                        <h3>Ready to Start?</h3>
                        <p>Select an artist from the dropdown above to begin building your collection.</p>
                        <div class="empty-state-hint">
                            👆 Click the dropdown to see all artists
                        </div>
                    </div>
                </div>
            </div>

            <div class="right-panel">
                <div id="card-viewer-container">
                    <button id="prev-artist" class="nav-button nav-left" title="Previous artist (←)" style="display: none;">
                        <span class="nav-arrow">←</span>
                    </button>
                    <div id="card-viewer"></div>
                    <button id="next-artist" class="nav-button nav-right" title="Next artist (→)" style="display: none;">
                        <span class="nav-arrow">→</span>
                    </button>
                    <div class="viewer-controls">
                        <button id="toggle-rotation">Play</button>
                    </div>
                </div>
            </div>
        </main>
    </div>

    <script>
        // Helper function to get cookie value
        function getCookie(name) {
            const value = `; ${document.cookie}`;
            const parts = value.split(`; ${name}=`);
            if (parts.length === 2) return parts.pop().split(';').shift();
            return null;
        }

        // Set global user ID for use by other scripts (like card-viewer.js)
        window.currentUserId = getCookie('user_id');
    </script>

    <script src="/static/js/card-viewer.js"></script>

    <script>

        // Welcome Modal - show on first visit per user
        (function() {
            const userId = getCookie('user_id');
            if (!userId) {
                console.warn('No user_id cookie found');
                return;
            }

            const welcomeKey = `basscoast_has_seen_welcome_${userId}`;
            const hasSeenWelcome = localStorage.getItem(welcomeKey);
            const modal = document.getElementById('welcome-modal');

            if (!hasSeenWelcome) {
                modal.style.display = 'flex';
            }

            document.getElementById('get-started-btn').addEventListener('click', function() {
                localStorage.setItem(welcomeKey, 'true');
                modal.style.display = 'none';
            });

            // Allow clicking overlay to close
            modal.addEventListener('click', function(e) {
                if (e.target === modal) {
                    localStorage.setItem(welcomeKey, 'true');
                    modal.style.display = 'none';
                }
            });
        })();

        // Stub checkForChanges to prevent errors if called before artist loads
        window.checkForChanges = function() {
            console.debug('checkForChanges stub called (no artist loaded yet)');
        };

        // Get user ID for user-specific localStorage keys
        const userId = getCookie('user_id');

        // LocalStorage helpers - all user-specific (if userId available)
        const STORAGE_KEYS = userId ? {
            SELECTED_ARTIST: `basscoast_selected_artist_${userId}`,
            ROTATION_STATE: `basscoast_rotation_state_${userId}`
        } : {
            SELECTED_ARTIST: 'basscoast_selected_artist',
            ROTATION_STATE: 'basscoast_rotation_state'
        };

        // Clean up old localStorage keys (now all user-specific)
        localStorage.removeItem('basscoast_selected_artist_name');
        localStorage.removeItem('basscoast_has_seen_welcome');
        localStorage.removeItem('basscoast_selected_artist');
        localStorage.removeItem('basscoast_rotation_state');
        localStorage.removeItem('cardAutoRotate');

        let artistMap = new Map(); // Map of artist name -> artist id
        let artistIdToNameMap = new Map(); // Map of artist id -> current name (including renames)
        let savedArtists = new Set(); // Set of artist IDs that have saved cards

        // Load artist list on page load
        fetch('/artists')
            .then(response => response.text())
            .then(html => {
                // Parse options from HTML
                const tempDiv = document.createElement('div');
                tempDiv.innerHTML = html;
                const options = tempDiv.querySelectorAll('option');

                const select = document.getElementById('artist-select');

                options.forEach(option => {
                    if (option.value) {
                        const artistId = option.value;
                        const artistName = option.textContent;
                        const hasSaved = option.dataset.saved === 'true';

                        artistMap.set(artistName, artistId);
                        artistIdToNameMap.set(artistId, artistName);
                        if (hasSaved) {
                            savedArtists.add(artistId);
                        }

                        const optionEl = document.createElement('option');
                        optionEl.value = artistId;
                        optionEl.textContent = (hasSaved ? '✓ ' : '') + artistName;
                        select.appendChild(optionEl);
                    }
                });

                // Restore selected artist from localStorage
                const savedArtistId = localStorage.getItem(STORAGE_KEYS.SELECTED_ARTIST);

                if (savedArtistId) {
                    select.value = savedArtistId;

                    htmx.ajax('GET', '/artists/' + savedArtistId, {
                        target: '#artist-detail',
                        swap: 'innerHTML'
                    });
                }

                // Restore rotation state from localStorage
                const savedRotationState = localStorage.getItem(STORAGE_KEYS.ROTATION_STATE);
                if (savedRotationState === 'true' && viewer) {
                    viewer.toggleRotation();
                }

                // Initialize navigation with loaded artist data
                window.artistNavigation.init(artistIdToNameMap, savedArtists);
            });

        // Handle artist selection from dropdown
        document.getElementById('artist-select').addEventListener('change', function(e) {
            const artistId = e.target.value;
            const artistSelector = document.querySelector('.artist-selector');

            if (artistId) {
                // Remove pulse hint when artist is selected
                artistSelector.classList.remove('pulse-hint');

                // Mark that user has learned to use the selector (show guidance once only)
                const hasUsedKey = `basscoast_has_used_artist_selector_${userId}`;
                localStorage.setItem(hasUsedKey, 'true');

                // Save to localStorage
                localStorage.setItem(STORAGE_KEYS.SELECTED_ARTIST, artistId);

                // Update navigation state
                window.artistNavigation.updateCurrentArtist(artistId);

                // Load artist details
                htmx.ajax('GET', '/artists/' + artistId, {
                    target: '#artist-detail',
                    swap: 'innerHTML'
                });
            }
        });

        // Add pulse hint on page load if user hasn't used selector before
        window.addEventListener('load', function() {
            const select = document.getElementById('artist-select');
            const artistSelector = document.querySelector('.artist-selector');
            const hasUsedKey = `basscoast_has_used_artist_selector_${userId}`;
            const hasUsedBefore = localStorage.getItem(hasUsedKey);

            if (select && artistSelector && !select.value && !hasUsedBefore) {
                artistSelector.classList.add('pulse-hint');
            }
        });

        // Save rotation state to localStorage when toggled
        document.getElementById('toggle-rotation').addEventListener('click', function() {
            setTimeout(() => {
                if (viewer) {
                    localStorage.setItem(STORAGE_KEYS.ROTATION_STATE, viewer.autoRotate);
                }
            }, 100);
        });

        // Artist Navigation System - persists across HTMX swaps
        window.artistNavigation = {
            artistsArray: [],
            currentIndex: -1,
            artistIdToNameMap: null,
            savedArtists: null,

            init: function(idToNameMap, savedSet) {
                this.artistIdToNameMap = idToNameMap;
                this.savedArtists = savedSet;
                this.artistsArray = Array.from(idToNameMap.keys());
            },

            updateCurrentArtist: function(artistId) {
                this.currentIndex = this.artistsArray.indexOf(artistId);
                this.updateButtons();
            },

            navigateToArtist: function(direction) {
                const newIndex = this.currentIndex + direction;
                if (newIndex < 0 || newIndex >= this.artistsArray.length) return;

                // Check for unsaved changes
                const saveButton = document.getElementById('save-changes');
                if (saveButton && !saveButton.disabled) {
                    const choice = confirm('You have unsaved changes. Click OK to save them, or Cancel to discard.');

                    if (choice) {
                        // Save and navigate
                        saveButton.click();
                        setTimeout(() => {
                            this.loadArtist(newIndex);
                        }, 500);
                    } else {
                        // Discard and navigate
                        if (confirm('Are you sure you want to discard your changes?')) {
                            this.loadArtist(newIndex);
                        }
                    }
                } else {
                    this.loadArtist(newIndex);
                }
            },

            loadArtist: function(index) {
                const artistId = this.artistsArray[index];
                this.currentIndex = index;

                // Update artist select dropdown
                document.getElementById('artist-select').value = artistId;

                // Load artist details via HTMX
                htmx.ajax('GET', '/artists/' + artistId, {
                    target: '#artist-detail',
                    swap: 'innerHTML'
                });

                // Save to localStorage
                localStorage.setItem(STORAGE_KEYS.SELECTED_ARTIST, artistId);
            },

            updateButtons: function() {
                const prevBtn = document.getElementById('prev-artist');
                const nextBtn = document.getElementById('next-artist');

                if (prevBtn && nextBtn) {
                    prevBtn.style.display = 'flex';
                    nextBtn.style.display = 'flex';

                    prevBtn.disabled = this.currentIndex <= 0;
                    nextBtn.disabled = this.currentIndex >= this.artistsArray.length - 1;
                }
            }
        };

        // Navigation button event listeners
        document.getElementById('prev-artist')?.addEventListener('click', () => {
            window.artistNavigation.navigateToArtist(-1);
        });

        document.getElementById('next-artist')?.addEventListener('click', () => {
            window.artistNavigation.navigateToArtist(1);
        });

        // Keyboard shortcuts
        document.addEventListener('keydown', (e) => {
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT') return;

            if (e.key === 'ArrowLeft') {
                window.artistNavigation.navigateToArtist(-1);
            } else if (e.key === 'ArrowRight') {
                window.artistNavigation.navigateToArtist(1);
            }
        });

        // Touch swipe support for mobile
        let touchStartX = 0;
        document.addEventListener('touchstart', (e) => {
            touchStartX = e.changedTouches[0].screenX;
        }, { passive: true });

        document.addEventListener('touchend', (e) => {
            const touchEndX = e.changedTouches[0].screenX;
            const diff = touchStartX - touchEndX;

            if (Math.abs(diff) > 100) {
                window.artistNavigation.navigateToArtist(diff > 0 ? 1 : -1);
            }
        }, { passive: true });

        // Listen for HTMX afterSwap to update current artist
        document.body.addEventListener('htmx:afterSwap', (e) => {
            if (e.detail.target.id === 'artist-detail') {
                const artistId = document.querySelector('.artist-info')?.dataset.artistId;
                if (artistId) {
                    window.artistNavigation.updateCurrentArtist(artistId);
                }
            }
        });

        // Logout button
        document.getElementById('logout-button')?.addEventListener('click', () => {
            if (confirm('Are you sure you want to logout?')) {
                // No need to clear localStorage - data is user-specific via user_id
                // When user logs back in, their data will be loaded automatically
                window.location.href = '/logout';
            }
        });
    </script>
</body>
</html>
    "##;
    Html(html).into_response()
}

// GET /logout - Logout and clear cookie
pub async fn logout(cookies: Cookies) -> impl IntoResponse {
    // Remove user_id cookie
    let mut cookie = Cookie::new("user_id", "");
    cookie.set_path("/");
    cookie.set_max_age(tower_cookies::cookie::time::Duration::seconds(0));
    cookies.add(cookie);

    Redirect::to("/login")
}
