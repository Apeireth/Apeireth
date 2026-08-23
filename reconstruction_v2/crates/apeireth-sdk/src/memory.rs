use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct MemoryClient {
    base_url: String,
    http: Client,
}

#[derive(Serialize)]
struct MemoryAppendRequest {
    data: String,
    importance: Option<f64>,
}

#[derive(Deserialize)]
struct MemoryEpisode {
    data: String,
}

impl MemoryClient {
    pub(crate) fn new(base_url: String, http: Client) -> Self {
        Self { base_url, http }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn search(&self, query: &str) -> Result<Vec<String>, crate::Error> {
        let url = format!("{}/v1/panel/memory/episodes", self.base_url);
        let resp = self.http.get(&url)
            .send()
            .await
            .map_err(|e| crate::Error::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(crate::Error::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let episodes: Vec<MemoryEpisode> = resp.json()
            .await
            .unwrap_or_default();

        let q_lower = query.to_lowercase();
        let matches: Vec<String> = episodes.into_iter()
            .map(|e| e.data)
            .filter(|d| query.is_empty() || d.to_lowercase().contains(&q_lower))
            .collect();

        Ok(matches)
    }

    pub async fn append(&self, fact: &str, importance: Option<f64>) -> Result<(), crate::Error> {
        let url = format!("{}/v1/memory/append", self.base_url);
        let body = MemoryAppendRequest {
            data: fact.to_string(),
            importance,
        };

        let resp = self.http.post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(crate::Error::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }
}
