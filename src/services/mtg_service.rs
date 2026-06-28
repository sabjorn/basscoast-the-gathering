use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const SCRYFALL_API_BASE: &str = "https://api.scryfall.com";
const RATE_LIMIT_MS: u64 = 100; // Scryfall requires 100ms between requests

#[derive(Debug, Serialize, Deserialize)]
pub struct ScryfallCard {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub colors: Option<Vec<String>>,
    pub color_identity: Option<Vec<String>>,
    pub type_line: String,
    pub cmc: Option<f64>,
    pub oracle_text: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub image_uris: Option<ImageUris>,
    pub edhrec_rank: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUris {
    pub small: Option<String>,
    pub normal: Option<String>,
    pub large: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ScryfallSearchResponse {
    data: Vec<ScryfallCard>,
    has_more: bool,
    next_page: Option<String>,
}

/// Scryfall API client with rate limiting
pub struct ScryfallClient {
    client: reqwest::Client,
    last_request: std::sync::Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
}

impl ScryfallClient {
    pub fn new() -> Self {
        // Create client with required headers
        let client = reqwest::Client::builder()
            .user_agent("BasscoastApp/1.0")
            .build()
            .unwrap();

        Self {
            client,
            last_request: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Enforce rate limiting before making a request
    async fn rate_limit(&self) {
        let mut last = self.last_request.lock().await;
        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            let min_delay = Duration::from_millis(RATE_LIMIT_MS);
            if elapsed < min_delay {
                let sleep_duration = min_delay - elapsed;
                tokio::time::sleep(sleep_duration).await;
            }
        }
        *last = Some(std::time::Instant::now());
    }

    /// Search for cards with filters
    /// Example query: "rarity:uncommon color:U cmc:3"
    pub async fn search_cards(&self, query: &str) -> Result<Vec<ScryfallCard>> {
        self.rate_limit().await;

        let url = format!(
            "{}/cards/search?q={}",
            SCRYFALL_API_BASE,
            urlencoding::encode(query)
        );

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Scryfall API error: {} - {}", status, error_body);
        }

        let search_result: ScryfallSearchResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse Scryfall response: {}", e))?;
        Ok(search_result.data)
    }

    /// Get a single card by Scryfall ID
    pub async fn get_card(&self, scryfall_id: &str) -> Result<ScryfallCard> {
        self.rate_limit().await;

        let url = format!("{}/cards/{}", SCRYFALL_API_BASE, scryfall_id);

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Scryfall API error: {}", response.status());
        }

        let card: ScryfallCard = response.json().await?;
        Ok(card)
    }
}

/// Build a Scryfall search query from filter parameters
pub fn build_search_query(
    rarity: Option<&str>,
    colors: Option<&str>,
    cmc: Option<f64>,
    text_query: Option<&str>,
    mechanic: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    if let Some(r) = rarity {
        if !r.is_empty() {
            parts.push(format!("rarity:{}", r));
        }
    }

    if let Some(c) = colors {
        if !c.is_empty() {
            // colors param expected as comma-separated: "U,B"
            let color_filters: Vec<_> = c
                .split(',')
                .map(|color| format!("color:{}", color.trim()))
                .collect();
            parts.extend(color_filters);
        }
    }

    if let Some(cmc_val) = cmc {
        parts.push(format!("cmc={}", cmc_val));
    }

    if let Some(m) = mechanic {
        if !m.is_empty() {
            parts.push(format!("keyword:{}", m));
        }
    }

    if let Some(q) = text_query {
        if !q.is_empty() {
            parts.push(q.to_string());
        }
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_query() {
        let query = build_search_query(
            Some("uncommon"),
            Some("U,B"),
            Some(3.0),
            Some("artifact"),
            Some("flying"),
        );
        assert!(query.contains("rarity:uncommon"));
        assert!(query.contains("color:U"));
        assert!(query.contains("color:B"));
        assert!(query.contains("cmc=3"));
        assert!(query.contains("artifact"));
        assert!(query.contains("keyword:flying"));
    }
}
