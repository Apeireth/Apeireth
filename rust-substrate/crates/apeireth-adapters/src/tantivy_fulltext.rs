//! Tantivy fulltext adapter (借鉴 quickwit-oss/tantivy)

use async_trait::async_trait;
use apeireth_ports::{FullTextIndex, FullTextHit, PortError};
use std::sync::Arc;
use parking_lot::Mutex;

pub struct TantivyIndex {
    index: Arc<Mutex<tantivy::Index>>,
    schema: tantivy::schema::Schema,
}

impl TantivyIndex {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, PortError> {
        // Simplified — real impl needs schema + writers
        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("id", tantivy::schema::STRING | tantivy::schema::STORED);
        schema_builder.add_text_field("content", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();
        let dir = tantivy::directory::MmapDirectory::open(path)
            .map_err(|e| PortError::Backend(e.to_string()))?;
        let index = tantivy::Index::open_or_create(dir, schema.clone())
            .map_err(|e| PortError::Backend(e.to_string()))?;
        Ok(Self { index: Arc::new(Mutex::new(index)), schema })
    }
}

#[async_trait]
impl FullTextIndex for TantivyIndex {
    async fn index(&self, _id: &str, _text: &str, _metadata: serde_json::Value) -> Result<(), PortError> {
        // TODO: writer.add_document
        Ok(())
    }

    async fn search(&self, _query: &str, _top_k: usize) -> Result<Vec<FullTextHit>, PortError> {
        // TODO: real search
        Ok(vec![])
    }

    async fn delete(&self, _id: &str) -> Result<(), PortError> {
        Ok(())
    }
}