use std::sync::Arc;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Response {
    results: Vec<SearchResult>,
}

#[derive(Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub content: String,
}

#[derive(Clone)]
pub struct SearXng {
    http_client: reqwest::Client,
    api_url: Arc<str>,
}

impl SearXng {
    pub fn new(http_client: reqwest::Client, api_url: String) -> Self {
        Self {
            http_client,
            api_url: Arc::from(api_url.as_str()),
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, reqwest::Error> {
        self.http_client
            .get(format!("{}/search", self.api_url))
            .query(&[("q", query), ("format", "json")])
            .send()
            .await?
            .json::<Response>()
            .await
            .map(|mut response| response.results.drain(0..=5).collect())
    }
}
