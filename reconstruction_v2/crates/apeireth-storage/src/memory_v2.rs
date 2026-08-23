use crate::{pool::SqliteConnectionPool, StorageError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum QueryMode {
    CurrentOnly,
    Historical,
    All,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MemoryOperation {
    Add,
    Update { content_override: String },
    Delete,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemoryItem {
    pub id: String,
    pub data: String,
    pub importance: f64,
    pub access_count: u32,
    pub access_times: Vec<i64>, // store timestamps in seconds for ACT-R
    pub created_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub is_tombstone: bool,
    pub artifact_sig: Option<String>,
}

impl MemoryItem {
    pub fn calculate_importance_score(&self, recency_weight: f64) -> f64 {
        self.importance * 3.0 + (self.access_count as f64) * 0.3 + recency_weight
    }

    pub fn calculate_act_r_activation(&self, current_time: i64, decay: f64, beta: f64) -> f64 {
        let mut sum = 0.0;
        for &t_j in &self.access_times {
            let diff = current_time - t_j;
            if diff > 0 {
                sum += (diff as f64).powf(-decay);
            }
        }
        if sum > 0.0 {
            sum.ln() + beta
        } else {
            beta
        }
    }
}

pub struct MemoryStore {
    pool: SqliteConnectionPool,
}

impl MemoryStore {
    pub fn new(pool: SqliteConnectionPool) -> Self {
        Self { pool }
    }

    pub async fn apply_operation(&self, mut item: MemoryItem, op: MemoryOperation) -> Result<(), StorageError> {
        match op {
            MemoryOperation::Add => {
                item.is_tombstone = false;
            }
            MemoryOperation::Update { content_override } => {
                item.data = content_override;
                item.is_tombstone = false;
            }
            MemoryOperation::Delete => {
                item.is_tombstone = true;
            }
        }

        if item.artifact_sig.is_none() {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(item.data.as_bytes());
            item.artifact_sig = Some(format!("{:x}", hasher.finalize()));
        }

        let json = serde_json::to_string(&item)?;
        let id = item.id.clone();
        self.pool.write(move |conn| {
            conn.execute(
                "INSERT INTO facts (id, data) VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
                (&id, &json),
            )?;
            Ok(())
        }).await
    }

    pub async fn query(&self, as_of: DateTime<Utc>, mode: QueryMode) -> Result<Vec<MemoryItem>, StorageError> {
        let conn = self.pool.get_reader()?;
        let mut stmt = conn.prepare("SELECT data FROM facts")?;
        let rows = stmt.query_map([], |row| {
            let data: String = row.get(0)?;
            Ok(data)
        })?;

        let mut results = Vec::new();
        for row in rows {
            let data = row?;
            if let Ok(item) = serde_json::from_str::<MemoryItem>(&data) {
                if item.is_tombstone {
                    continue;
                }
                
                let is_valid = match mode {
                    QueryMode::CurrentOnly => {
                        item.valid_from <= as_of && item.valid_until.map_or(true, |until| as_of < until)
                    }
                    QueryMode::Historical | QueryMode::All => true, // Simplified
                };

                if is_valid {
                    results.push(item);
                }
            }
        }
        
        results.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
        Ok(results)
    }

    pub fn cjk_bigram_tokenize(text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let mut bigrams = Vec::new();
        if chars.len() < 2 {
            if chars.len() == 1 {
                bigrams.push(chars[0].to_string());
            }
            return bigrams;
        }
        for i in 0..chars.len() - 1 {
            let bigram = format!("{}{}", chars[i], chars[i + 1]);
            bigrams.push(bigram);
        }
        bigrams
    }
    
    pub fn calculate_jaccard_similarity(text_a: &str, text_b: &str) -> f64 {
        let set_a: std::collections::HashSet<String> = Self::cjk_bigram_tokenize(text_a).into_iter().collect();
        let set_b: std::collections::HashSet<String> = Self::cjk_bigram_tokenize(text_b).into_iter().collect();

        if set_a.is_empty() && set_b.is_empty() {
            return 1.0;
        }
        if set_a.is_empty() || set_b.is_empty() {
            return 0.0;
        }

        let intersection_count = set_a.intersection(&set_b).count();
        let union_count = set_a.union(&set_b).count();

        intersection_count as f64 / union_count as f64
    }

    pub fn greedy_clustering(items: &[MemoryItem], similarity_threshold: f64) -> Vec<Vec<MemoryItem>> {
        let mut clusters: Vec<Vec<MemoryItem>> = Vec::new();
        for item in items {
            let mut best_match = None;
            let mut highest_sim = 0.0;

            for (i, cluster) in clusters.iter().enumerate() {
                if let Some(representative) = cluster.first() {
                    let sim = Self::calculate_jaccard_similarity(&item.data, &representative.data);
                    if sim >= similarity_threshold && sim > highest_sim {
                        highest_sim = sim;
                        best_match = Some(i);
                    }
                }
            }

            if let Some(idx) = best_match {
                clusters[idx].push(item.clone());
            } else {
                clusters.push(vec![item.clone()]);
            }
        }
        clusters
    }
}

