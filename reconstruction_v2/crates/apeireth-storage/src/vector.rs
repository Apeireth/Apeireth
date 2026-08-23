use std::collections::HashMap;

#[derive(Default)]
pub struct VectorIndex {
    items: HashMap<String, Vec<f32>>,
    texts: HashMap<String, String>,
    doc_freqs: HashMap<String, usize>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: String, text: String, vector: Vec<f32>) {
        let terms: std::collections::HashSet<String> = text
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        for term in terms {
            *self.doc_freqs.entry(term).or_insert(0) += 1;
        }

        self.items.insert(id.clone(), vector);
        self.texts.insert(id, text);
    }

    pub fn search_hybrid(&self, query_vec: &[f32], query_text: &str, top_k: usize) -> Vec<(String, f32)> {
        let total_docs = self.texts.len();
        if total_docs == 0 {
            return Vec::new();
        }

        let total_len: usize = self.texts.values().map(|t| t.split_whitespace().count()).sum();
        let avgdl = (total_len as f32 / total_docs as f32).max(1.0);

        let mut results = Vec::new();
        for (id, vec) in &self.items {
            let cosine = cosine_similarity(query_vec, vec);
            let text = self.texts.get(id).unwrap();
            let bm25 = self.bm25_score(query_text, text, total_docs, avgdl);
            
            let hybrid_score = cosine * 0.7 + bm25 * 0.3;
            results.push((id.clone(), hybrid_score));
        }
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(top_k).collect()
    }

    fn bm25_score(&self, query: &str, document: &str, total_docs: usize, avgdl: f32) -> f32 {
        let q_terms: Vec<String> = query.split_whitespace().map(|s| s.to_lowercase()).collect();
        let d_terms: Vec<String> = document.split_whitespace().map(|s| s.to_lowercase()).collect();
        let doc_len = d_terms.len() as f32;
        let k1 = 1.2;
        let b = 0.75;
        let mut score = 0.0;

        for qt in &q_terms {
            let tf = d_terms.iter().filter(|&dt| dt == qt).count() as f32;
            if tf > 0.0 {
                let df = self.doc_freqs.get(qt).copied().unwrap_or(1) as f32;
                let idf = ((total_docs as f32 - df + 0.5) / (df + 0.5) + 1.0).ln().max(0.1);
                let tf_norm = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * (doc_len / avgdl)));
                score += idf * tf_norm;
            }
        }
        score
    }
    
    pub fn extract_user_profile(&self) -> String {
        if self.texts.is_empty() {
            return "empty_profile".to_string();
        }

        // Extract most frequent salient entity keywords across stored texts
        let mut term_counts: HashMap<String, usize> = HashMap::new();
        for text in self.texts.values() {
            for word in text.split_whitespace() {
                let clean = word.to_lowercase();
                if clean.len() > 2 {
                    *term_counts.entry(clean).or_insert(0) += 1;
                }
            }
        }

        let mut sorted_terms: Vec<(String, usize)> = term_counts.into_iter().collect();
        sorted_terms.sort_by(|a, b| b.1.cmp(&a.1));

        let top_keywords: Vec<String> = sorted_terms
            .into_iter()
            .take(5)
            .map(|(term, count)| format!("{}({})", term, count))
            .collect();

        format!("UserProfile[topics: {}]", top_keywords.join(", "))
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_index_hybrid_search() {
        let mut idx = VectorIndex::new();
        idx.insert("1".into(), "Rust async runtime tokio concurrency".into(), vec![1.0, 0.0, 0.0]);
        idx.insert("2".into(), "Python machine learning pytorch model".into(), vec![0.0, 1.0, 0.0]);

        let results = idx.search_hybrid(&[0.9, 0.1, 0.0], "tokio async", 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "1");

        let profile = idx.extract_user_profile();
        assert!(profile.contains("UserProfile"));
    }
}

