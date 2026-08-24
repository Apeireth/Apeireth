pub mod pool;
pub mod migrations;
pub mod memory_v2;
pub mod graph_ops;
pub mod graph_primitive;
pub mod cosine;
pub mod graph;
pub mod vector;
pub mod fold;
pub mod memory_episode;
pub mod memory_dedup;
pub mod memory_continuity;

pub use pool::SqliteConnectionPool;
pub use migrations::run_migrations;

#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Pool error: {0}")]
    Pool(#[from] r2d2::Error),
    #[error("Write queue error")]
    WriteQueue,
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Not found")]
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Utc, Duration};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_concurrent_read_write() {
        let db_path = format!("test_pool_{}.db", Uuid::new_v4());
        let pool = SqliteConnectionPool::new(&db_path).await.unwrap();
        
        {
            let mut conn = pool.get_reader().unwrap();
            migrations::run_migrations(&mut conn).unwrap();
        }

        let p1 = pool.clone();
        let p2 = pool.clone();
        
        let t1 = tokio::spawn(async move {
            for i in 0..50 {
                p1.write(move |conn| {
                    conn.execute("INSERT INTO facts (id, data) VALUES (?1, ?2)", (&format!("id{}", i), "data"))?;
                    Ok(())
                }).await.unwrap();
            }
        });

        let t2 = tokio::spawn(async move {
            for _ in 0..50 {
                let _ = p2.get_reader().unwrap().query_row("SELECT count(*) FROM facts", [], |row| {
                    let count: i64 = row.get(0)?;
                    Ok(count)
                });
            }
        });

        let _ = tokio::join!(t1, t2);
        std::fs::remove_file(db_path).unwrap_or_default();
    }

    #[tokio::test]
    async fn test_memory_v2_importance_and_temporal() {
        let db_path = format!("test_mem_{}.db", Uuid::new_v4());
        let pool = SqliteConnectionPool::new(&db_path).await.unwrap();
        
        {
            let mut conn = pool.get_reader().unwrap();
            migrations::run_migrations(&mut conn).unwrap();
        }

        let store = memory_v2::MemoryStore::new(pool);
        let now = Utc::now();

        store.apply_operation(memory_v2::MemoryItem {
            id: "1".to_string(),
            data: "past".to_string(),
            importance: 0.5,
            access_count: 1,
            access_times: vec![now.timestamp() - 10000],
            created_at: now,
            valid_from: now - Duration::days(10),
            valid_until: Some(now - Duration::days(5)),
            is_tombstone: false,
            artifact_sig: None,
        }, memory_v2::MemoryOperation::Add).await.unwrap();

        store.apply_operation(memory_v2::MemoryItem {
            id: "2".to_string(),
            data: "current".to_string(),
            importance: 0.8,
            access_count: 5,
            access_times: vec![now.timestamp() - 1000],
            created_at: now,
            valid_from: now - Duration::days(2),
            valid_until: Some(now + Duration::days(5)),
            is_tombstone: false,
            artifact_sig: None,
        }, memory_v2::MemoryOperation::Add).await.unwrap();

        let active = store.query(now, memory_v2::QueryMode::CurrentOnly).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "2");
        assert!(active[0].artifact_sig.is_some());
        
        // test ACT-R
        let act_r = active[0].calculate_act_r_activation(now.timestamp(), 0.5, 0.0);
        assert!(act_r > -10.0);
        
        std::fs::remove_file(db_path).unwrap_or_default();
    }

    #[test]
    fn test_graph_traversal() {
        use graph::{CausalGraph, FactNode, Edge, MctsCausalSimulator};
        let mut g = CausalGraph::default();
        g.add_edge(Edge { from: FactNode("A".into()), to: FactNode("B".into()), weight: 1.0 });
        g.add_edge(Edge { from: FactNode("B".into()), to: FactNode("C".into()), weight: 1.0 });

        let sim = MctsCausalSimulator::new(g);
        let res = sim.simulate(&FactNode("A".into()), 5);
        assert!(res.len() >= 2);
    }
    
    #[test]
    fn test_cjk_bigram() {
        let text = "你好世界";
        let bigrams = memory_v2::MemoryStore::cjk_bigram_tokenize(text);
        assert_eq!(bigrams, vec!["你好", "好世", "世界"]);
    }

    #[test]
    fn test_jaccard_greedy_clustering() {
        let now = Utc::now();
        let items = vec![
            memory_v2::MemoryItem {
                id: "1".into(),
                data: "苹果手机和苹果电脑".into(),
                importance: 0.9,
                access_count: 1,
                access_times: vec![],
                created_at: now,
                valid_from: now,
                valid_until: None,
                is_tombstone: false,
                artifact_sig: None,
            },
            memory_v2::MemoryItem {
                id: "2".into(),
                data: "苹果电脑性能很强".into(),
                importance: 0.8,
                access_count: 1,
                access_times: vec![],
                created_at: now,
                valid_from: now,
                valid_until: None,
                is_tombstone: false,
                artifact_sig: None,
            },
            memory_v2::MemoryItem {
                id: "3".into(),
                data: "今天天气真好晴空万里".into(),
                importance: 0.5,
                access_count: 1,
                access_times: vec![],
                created_at: now,
                valid_from: now,
                valid_until: None,
                is_tombstone: false,
                artifact_sig: None,
            },
        ];

        let clusters = memory_v2::MemoryStore::greedy_clustering(&items, 0.2);
        assert_eq!(clusters.len(), 2); // Items 1 and 2 cluster together, Item 3 in separate cluster
        assert_eq!(clusters[0].len(), 2);
        assert_eq!(clusters[1].len(), 1);
    }
}

