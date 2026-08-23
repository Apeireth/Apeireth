pub struct MemoryClient {
    #[allow(dead_code)]
    base_url: String,
}

impl MemoryClient {
    pub(crate) fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn search(&self, _query: &str) -> Result<Vec<String>, crate::Error> {
        Ok(vec![])
    }
}
