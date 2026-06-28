use crate::AppState;
use crate::handlers::error_response;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use tower_cookies::Cookies;

// GET /artists - Return dropdown options for all artists
pub async fn list_artists(State(state): State<AppState>, cookies: Cookies) -> Response {
    let user_id = match get_user_id_from_cookies(&cookies) {
        Some(id) => id,
        None => return Html("<option>Not logged in</option>".to_string()).into_response(),
    };

    match crate::db::queries::get_all_artists(&state.pool).await {
        Ok(mut artists) => {
            // Apply user edits (renames, deletes)
            let user_renames = sqlx::query!(
                r#"SELECT artist_id, new_name FROM user_artist_edits
                   WHERE user_id = ? AND action = 'rename'
                   ORDER BY created_at DESC"#,
                user_id
            )
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();

            let user_deletes = sqlx::query!(
                r#"SELECT artist_id FROM user_artist_edits
                   WHERE user_id = ? AND action = 'delete'"#,
                user_id
            )
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();

            // Get artists with saved cards
            let saved_artists = sqlx::query!(
                r#"SELECT DISTINCT artist_id FROM user_mtg_selections
                   WHERE user_id = ?"#,
                user_id
            )
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();

            // Build rename map (artist_id -> new_name)
            let mut rename_map = std::collections::HashMap::new();
            for r in user_renames {
                if let (id, Some(name)) = (r.artist_id, r.new_name) {
                    rename_map.insert(id, name);
                }
            }

            // Build delete set
            let mut delete_set = std::collections::HashSet::new();
            for d in user_deletes {
                delete_set.insert(d.artist_id);
            }

            // Build saved set
            let mut saved_set = std::collections::HashSet::new();
            for s in saved_artists {
                saved_set.insert(s.artist_id);
            }

            // Apply renames and filter deletes
            artists.retain(|a| !delete_set.contains(&a.id));

            let mut options = vec![r#"<option value="">Choose an artist...</option>"#.to_string()];
            options.extend(artists.iter().map(|a| {
                let display_name = rename_map.get(&a.id).unwrap_or(&a.name);
                let has_saved = saved_set.contains(&a.id);
                format!(
                    r#"<option value="{}" data-saved="{}">{}</option>"#,
                    a.id, has_saved, display_name
                )
            }));

            Html(options.join("\n")).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch artists: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "<option>Error loading artists</option>")
        }
    }
}

// GET /artists/:id - Return artist detail panel HTML
pub async fn artist_detail(
    Path(artist_id): Path<String>,
    State(state): State<AppState>,
    cookies: Cookies,
) -> Response {
    let user_id = match get_user_id_from_cookies(&cookies) {
        Some(id) => id,
        None => return Html("<p>Not logged in</p>".to_string()).into_response(),
    };

    // Fetch artist data
    let artist = match crate::db::queries::get_artist_by_id(&state.pool, &artist_id).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Failed to fetch artist {}: {}", artist_id, e);
            return error_response(StatusCode::NOT_FOUND, "Artist not found");
        }
    };

    let appearances = crate::db::queries::get_artist_appearances(&state.pool, &artist_id)
        .await
        .unwrap_or_default();

    let metadata = crate::db::queries::get_artist_metadata(&state.pool, &artist_id)
        .await
        .ok()
        .flatten();

    // Check if user has a saved card selection for this artist
    let saved_card = crate::db::queries::get_user_mtg_selection(&state.pool, user_id, &artist_id)
        .await
        .ok()
        .flatten();

    // Check if user has renamed this artist
    let user_rename = sqlx::query!(
        r#"SELECT new_name FROM user_artist_edits
           WHERE user_id = ? AND artist_id = ? AND action = 'rename'
           ORDER BY created_at DESC
           LIMIT 1"#,
        user_id,
        artist_id
    )
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    // Load user-specific metadata
    let user_metadata_json = crate::db::queries::get_user_artist_metadata(&state.pool, user_id, &artist_id)
        .await
        .ok()
        .flatten();

    // Use renamed name if available, otherwise original name
    let display_name = user_rename
        .and_then(|r| r.new_name)
        .unwrap_or_else(|| artist.name.clone());

    // Build HTML with saved card if available
    let saved_card_id = saved_card
        .as_ref()
        .map(|c| c.scryfall_id.as_str())
        .unwrap_or("");

    let mut html = format!(
        r##"<div class="artist-info" data-artist-id="{}" data-original-name="{}" data-saved-card="{}">
            <div class="save-section">
                <button id="save-changes" class="save-button" disabled>Save Changes</button>
                <button id="delete-artist" class="delete-button">Delete Artist</button>
                <div id="save-status" class="save-status"></div>
            </div>

            <!-- Mobile Tabs -->
            <div class="mobile-tabs">
                <button class="tab-button active" data-tab="info">Info</button>
                <button class="tab-button" data-tab="metadata">Metadata</button>
                <button class="tab-button" data-tab="search">Card</button>
            </div>

            <!-- Tab Content: Info -->
            <div class="tab-content active" data-tab-content="info">
                <h2>Artist Details</h2>
                <input type="text"
                       id="artist-name-input"
                       value="{}"
                       placeholder="Artist name">
        "##,
        artist.id, display_name, saved_card_id, display_name
    );

    // Appearances - just show years
    html.push_str(r#"<h3>Appearances</h3><table><thead><tr><th>Year</th></tr></thead><tbody>"#);

    for app in &appearances {
        html.push_str(&format!(
            "<tr><td>{}</td></tr>",
            app.year
        ));
    }

    html.push_str("</tbody></table>");
    html.push_str("</div>"); // Close info tab

    // Metadata - merge base metadata with user metadata
    let mut metadata_map = std::collections::HashMap::new();

    // Start with base metadata
    if let Some(meta) = metadata {
        if let Some(tags) = meta.tags {
            metadata_map.insert("tags".to_string(), tags);
        }
        if let Some(area) = meta.area {
            metadata_map.insert("location".to_string(), area);
        }
    }

    // Override/extend with user metadata
    if let Some(user_json) = user_metadata_json {
        if let Ok(user_meta) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&user_json) {
            for (key, value) in user_meta {
                if let Some(v_str) = value.as_str() {
                    metadata_map.insert(key, v_str.to_string());
                }
            }
        }
    }

    // Track base metadata fields (can't be deleted)
    let base_fields = ["tags", "location"];

    // Display editable metadata
    html.push_str(r#"
            <!-- Tab Content: Metadata -->
            <div class="tab-content" data-tab-content="metadata">
                <h3>Metadata</h3>
                <div id="metadata-section">"#);

    // Show existing fields
    for (key, value) in &metadata_map {
        let is_base_field = base_fields.contains(&key.as_str());
        let delete_button = if !is_base_field {
            r#"<button class="delete-field-btn" title="Delete field">×</button>"#
        } else {
            ""
        };

        // For tags, convert JSON array to comma-separated list
        let display_value = if key == "tags" {
            // Try to parse as JSON array and convert to comma-separated
            if let Ok(tags_array) = serde_json::from_str::<Vec<String>>(value) {
                tags_array.join(", ")
            } else {
                value.clone()
            }
        } else {
            value.clone()
        };

        html.push_str(&format!(
            r#"<div class="metadata-field" data-field-name="{}">
                <label>{}</label>
                <input type="text" class="metadata-input" data-field="{}" value="{}">
                {}
            </div>"#,
            key, key, key, display_value.replace('"', "&quot;"), delete_button
        ));
    }

    // Add new field section
    html.push_str(r#"
        <div class="metadata-new-field">
            <h4>Add Custom Field</h4>
            <div class="metadata-new-field-row">
                <input type="text" id="new-field-name" placeholder="Field name (e.g., genre, country)">
                <input type="text" id="new-field-value" placeholder="Value">
            </div>
            <button id="add-metadata-field">Add Field</button>
        </div>
                </div>
            </div><!-- Close metadata tab -->"#);

    // MTG recommendations and search
    html.push_str(&format!(
        r##"
            <!-- Tab Content: Card -->
            <div class="tab-content" data-tab-content="search">
                <div class="mtg-section">
                    <h3>Community Picks</h3>
                    <div hx-get="/artists/{}/mtg/recommendations"
                 hx-trigger="load"
                 hx-target="this"
                 hx-swap="innerHTML"
                 hx-indicator="#rec-loading">
                <span id="rec-loading" class="htmx-indicator">
                    <span class="loading-spinner large"></span>
                    <span style="margin-left: 1rem;">Loading recommendations...</span>
                </span>
            </div>

            <h3>Search All Cards</h3>
            <form id="mtg-search-form" class="mtg-filters"
                  hx-get="/mtg/search"
                  hx-target="#search-results"
                  hx-trigger="submit"
                  hx-indicator="#search-loading">
                <input type="hidden" name="artist_id" value="{}">
                <div class="filter-row">
                    <div class="filter-group">
                        <label>Rarity:</label>
                        <select name="rarity">
                            <option value="">Any</option>
                            <option value="common">Common</option>
                            <option value="uncommon">Uncommon</option>
                            <option value="rare">Rare</option>
                            <option value="mythic">Mythic</option>
                        </select>
                    </div>

                    <div class="filter-group">
                        <label>Mana Cost:</label>
                        <input type="number" name="cmc" min="0" max="16" step="1" placeholder="0-16">
                    </div>
                </div>

                <div class="filter-group">
                    <label>Mechanic:</label>
                    <select name="mechanic">
                        <option value="">Any</option>
                        <option value="flying">Flying</option>
                        <option value="deathtouch">Deathtouch</option>
                        <option value="first strike">First Strike</option>
                        <option value="haste">Haste</option>
                        <option value="hexproof">Hexproof</option>
                        <option value="lifelink">Lifelink</option>
                        <option value="menace">Menace</option>
                        <option value="reach">Reach</option>
                        <option value="trample">Trample</option>
                        <option value="vigilance">Vigilance</option>
                        <option value="flash">Flash</option>
                        <option value="defender">Defender</option>
                    </select>
                </div>

                <div class="filter-group">
                    <label>Colors:</label>
                    <div class="color-checkboxes">
                        <label class="color-checkbox">
                            <input type="checkbox" class="color-check" value="W">
                            <span class="color-label white">W</span>
                        </label>
                        <label class="color-checkbox">
                            <input type="checkbox" class="color-check" value="U">
                            <span class="color-label blue">U</span>
                        </label>
                        <label class="color-checkbox">
                            <input type="checkbox" class="color-check" value="B">
                            <span class="color-label black">B</span>
                        </label>
                        <label class="color-checkbox">
                            <input type="checkbox" class="color-check" value="R">
                            <span class="color-label red">R</span>
                        </label>
                        <label class="color-checkbox">
                            <input type="checkbox" class="color-check" value="G">
                            <span class="color-label green">G</span>
                        </label>
                    </div>
                    <input type="hidden" name="colors" id="colors-input">
                </div>

                <div class="filter-group">
                    <label>
                        Card Name or Text:
                        <span class="search-help-icon" title="Search examples:&#10;• 'lightning bolt' - searches all text&#10;• 'name:jace' - card names only&#10;• 'o:draw' or 'oracle:draw' - rules text&#10;• 't:creature' - creature cards&#10;• 'o:destroy o:target' - cards with both words">ⓘ</span>
                    </label>
                    <input type="text" name="q" id="card-search-input" placeholder="e.g., lightning bolt, o:draw, t:creature">
                </div>

                <button type="submit">
                    Search Cards
                    <span id="search-loading" class="htmx-indicator loading-spinner"></span>
                </button>
            </form>

            <script>
            (function() {{
                document.getElementById('mtg-search-form').addEventListener('submit', function(e) {{
                    // Handle color checkboxes
                    const checked = Array.from(document.querySelectorAll('.color-check:checked')).map(cb => cb.value);
                    document.getElementById('colors-input').value = checked.join(',');

                    // Handle CMC - remove the parameter if empty so it doesn't filter by CMC=0
                    const cmcInput = this.querySelector('input[name="cmc"]');
                    if (cmcInput && cmcInput.value === '') {{
                        cmcInput.removeAttribute('name');
                        // Re-add name after submission for next search
                        setTimeout(() => cmcInput.setAttribute('name', 'cmc'), 100);
                    }}
                }});
            }})();
            </script>

            <div id="search-results" class="card-search-results">
                <p>Enter search criteria to find cards</p>
            </div>

            <script>
            (function() {{
                // Auto-load saved card if available
                const savedCardId = document.querySelector('.artist-info').dataset.savedCard;
                const artistId = document.querySelector('.artist-info').dataset.artistId;

                if (savedCardId && artistId) {{
                    // Wait for viewer with polling instead of fixed timeout
                    let attempts = 0;
                    const waitForViewer = setInterval(() => {{
                        if (window.viewer || attempts++ > 50) {{
                            clearInterval(waitForViewer);
                            if (window.viewer) {{
                                window.viewer.loadCard(savedCardId, artistId);
                                console.log('Auto-loaded saved card:', savedCardId);
                            }} else {{
                                console.error('Viewer not available after 5 seconds');
                            }}
                        }}
                    }}, 100); // Check every 100ms, max 5 seconds
                }}

                // Track changes and enable/disable save button
                const saveButton = document.getElementById('save-changes');
            const nameInput = document.getElementById('artist-name-input');
            const originalName = document.querySelector('.artist-info').dataset.originalName;

            // Store original metadata values
            const originalMetadata = {{}};
            document.querySelectorAll('.metadata-input').forEach(input => {{
                originalMetadata[input.dataset.field] = input.value.trim();
            }});

            window.checkForChanges = function() {{
                const nameChanged = nameInput.value.trim() !== originalName;

                // Always read current saved card ID from dataset (it may have been updated after save)
                const savedCardId = document.querySelector('.artist-info').dataset.savedCard;
                const cardChanged = viewer && viewer.currentCardId && viewer.currentCardId !== savedCardId;

                // Check if metadata has changed
                let metadataChanged = false;

                // Build current metadata from inputs
                const currentFields = new Set();
                document.querySelectorAll('.metadata-input').forEach(input => {{
                    const field = input.dataset.field;
                    currentFields.add(field);
                    const currentValue = input.value.trim();
                    const originalValue = originalMetadata[field] || '';
                    if (currentValue !== originalValue) {{
                        metadataChanged = true;
                    }}
                }});

                // Check if any fields were deleted
                Object.keys(originalMetadata).forEach(field => {{
                    if (!currentFields.has(field)) {{
                        metadataChanged = true;
                    }}
                }});

                const hasChanges = nameChanged || cardChanged || metadataChanged;
                saveButton.disabled = !hasChanges;
            }}

            // Check for changes on name input
            nameInput.addEventListener('input', checkForChanges);

            // Check for changes on metadata inputs
            document.querySelectorAll('.metadata-input').forEach(input => {{
                input.addEventListener('input', checkForChanges);
            }});

            // Handle save button
            document.getElementById('save-changes').addEventListener('click', async function() {{
                const artistId = document.querySelector('.artist-info').dataset.artistId;
                const originalName = document.querySelector('.artist-info').dataset.originalName;
                const currentName = document.getElementById('artist-name-input').value.trim();
                const statusDiv = document.getElementById('save-status');

                let savedItems = [];
                let errors = [];

                // Save artist name if changed
                if (currentName !== originalName && currentName !== '') {{
                    try {{
                        const params = new URLSearchParams();
                        params.append('new_name', currentName);
                        const response = await fetch(`/artists/${{artistId}}/rename`, {{
                            method: 'POST',
                            headers: {{
                                'Content-Type': 'application/x-www-form-urlencoded'
                            }},
                            body: params
                        }});
                        if (response.ok) {{
                            savedItems.push('name');
                            document.querySelector('.artist-info').dataset.originalName = currentName;
                        }} else {{
                            errors.push('name');
                        }}
                    }} catch (e) {{
                        errors.push('name');
                    }}
                }}

                // Save card selection if a card is loaded
                if (viewer && viewer.currentCardId && viewer.currentArtistId) {{
                    console.log('Saving card:', {{
                        artistId: viewer.currentArtistId,
                        scryfallId: viewer.currentCardId
                    }});
                    try {{
                        const params = new URLSearchParams();
                        params.append('artist_id', viewer.currentArtistId);
                        params.append('scryfall_id', viewer.currentCardId);
                        const response = await fetch('/mtg/select', {{
                            method: 'POST',
                            headers: {{
                                'Content-Type': 'application/x-www-form-urlencoded'
                            }},
                            body: params
                        }});
                        const responseText = await response.text();
                        console.log('Card save response:', {{ ok: response.ok, status: response.status, body: responseText }});
                        if (response.ok) {{
                            savedItems.push('card');
                            document.querySelector('.artist-info').dataset.savedCard = viewer.currentCardId;
                        }} else {{
                            console.error('Card save failed:', responseText);
                            errors.push('card');
                        }}
                    }} catch (e) {{
                        console.error('Card save error:', e);
                        errors.push('card');
                    }}
                }} else {{
                    console.log('Card save skipped:', {{
                        hasViewer: !!viewer,
                        currentCardId: viewer?.currentCardId,
                        currentArtistId: viewer?.currentArtistId
                    }});
                }}

                // Save metadata if changed
                let metadataChanged = false;
                const currentMetadata = {{}};
                document.querySelectorAll('.metadata-input').forEach(input => {{
                    const field = input.dataset.field;
                    let value = input.value.trim();

                    // Convert tags from comma-separated to JSON array format
                    if (field === 'tags' && value) {{
                        const tagsArray = value.split(',').map(t => t.trim()).filter(t => t);
                        value = JSON.stringify(tagsArray);
                    }}

                    if (value) {{
                        currentMetadata[field] = value;
                    }}
                    const originalValue = originalMetadata[field] || '';
                    if (value !== originalValue) {{
                        metadataChanged = true;
                    }}
                }});

                if (metadataChanged) {{
                    try {{
                        const params = new URLSearchParams();
                        params.append('metadata_json', JSON.stringify(currentMetadata));
                        const response = await fetch(`/artists/${{artistId}}/metadata`, {{
                            method: 'POST',
                            headers: {{
                                'Content-Type': 'application/x-www-form-urlencoded'
                            }},
                            body: params
                        }});
                        if (response.ok) {{
                            savedItems.push('metadata');
                            // Update original metadata with current input values (not the JSON-formatted versions)
                            Object.keys(originalMetadata).forEach(key => delete originalMetadata[key]);
                            document.querySelectorAll('.metadata-input').forEach(input => {{
                                const field = input.dataset.field;
                                const value = input.value.trim();
                                if (value) {{
                                    originalMetadata[field] = value;
                                }}
                            }});
                        }} else {{
                            errors.push('metadata');
                        }}
                    }} catch (e) {{
                        errors.push('metadata');
                    }}
                }}

                // Show status
                if (savedItems.length > 0) {{
                    statusDiv.textContent = `Saved: ${{savedItems.join(', ')}}`;
                    statusDiv.className = 'save-status success';
                }} else if (errors.length > 0) {{
                    statusDiv.textContent = `Error saving: ${{errors.join(', ')}}`;
                    statusDiv.className = 'save-status error';
                }} else {{
                    statusDiv.textContent = 'No changes to save';
                    statusDiv.className = 'save-status';
                }}

                setTimeout(() => statusDiv.textContent = '', 3000);

                // Re-check for changes to disable button if everything is saved
                checkForChanges();
            }});

            // Handle delete button
            document.getElementById('delete-artist').addEventListener('click', async function() {{
                const artistId = document.querySelector('.artist-info').dataset.artistId;
                const artistName = document.getElementById('artist-name-input').value.trim();

                if (!confirm(`Are you sure you want to delete "${{artistName}}" from your view? This will remove it from your artist list.`)) {{
                    return;
                }}

                try {{
                    const response = await fetch(`/artists/${{artistId}}/delete`, {{
                        method: 'POST'
                    }});

                    if (response.ok) {{
                        // Refresh artist list
                        htmx.ajax('GET', '/artists', {{target: '#artist-select', swap: 'innerHTML'}});
                        // Clear detail panel
                        document.getElementById('artist-detail').innerHTML = '<p>Artist deleted. Select another artist.</p>';
                    }} else {{
                        alert('Error deleting artist');
                    }}
                }} catch (e) {{
                    alert('Error deleting artist');
                }}
            }});

            // Handle metadata
            document.getElementById('add-metadata-field')?.addEventListener('click', function() {{
                const fieldName = document.getElementById('new-field-name').value.trim();
                const fieldValue = document.getElementById('new-field-value').value.trim();

                if (!fieldName || !fieldValue) {{
                    alert('Please enter both field name and value');
                    return;
                }}

                // Add new field with delete button
                const newFieldHtml = `
                    <div class="metadata-field" data-field-name="${{fieldName}}">
                        <label>${{fieldName}}</label>
                        <input type="text" class="metadata-input" data-field="${{fieldName}}" value="${{fieldValue}}">
                        <button class="delete-field-btn" title="Delete field">×</button>
                    </div>
                `;

                const newFieldSection = document.querySelector('.metadata-new-field');
                newFieldSection.insertAdjacentHTML('beforebegin', newFieldHtml);

                // Attach input listener to the new field
                const newInputs = document.querySelectorAll('.metadata-input');
                const newInput = newInputs[newInputs.length - 1];
                newInput.addEventListener('input', checkForChanges);

                // Mark as changed so Save Changes button enables
                checkForChanges();

                // Clear inputs
                document.getElementById('new-field-name').value = '';
                document.getElementById('new-field-value').value = '';
            }});

            // Handle metadata field deletion (event delegation)
            document.getElementById('metadata-section')?.addEventListener('click', function(e) {{
                if (e.target.classList.contains('delete-field-btn')) {{
                    const fieldDiv = e.target.closest('.metadata-field');

                    // Remove the field from DOM
                    fieldDiv.remove();

                    // Mark as changed (DO NOT delete from originalMetadata - that's the baseline!)
                    checkForChanges();
                }}
            }});

            // Mobile tab switching
            document.querySelectorAll('.tab-button').forEach(button => {{
                button.addEventListener('click', function() {{
                    const tabName = this.dataset.tab;

                    // Remove active class from all buttons and tabs
                    document.querySelectorAll('.tab-button').forEach(btn => btn.classList.remove('active'));
                    document.querySelectorAll('.tab-content').forEach(content => content.classList.remove('active'));

                    // Add active class to clicked button and corresponding tab
                    this.classList.add('active');
                    document.querySelector(`[data-tab-content="${{tabName}}"]`).classList.add('active');
                }});
            }});
            }})();
            </script>
                </div><!-- Close mtg-section -->
            </div><!-- Close search tab -->
        </div><!-- Close artist-info -->
        "##,
        artist.id, artist.id
    ));

    Html(html).into_response()
}

// Helper function to extract user_id from cookies
fn get_user_id_from_cookies(cookies: &Cookies) -> Option<i64> {
    cookies
        .get("user_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
}
