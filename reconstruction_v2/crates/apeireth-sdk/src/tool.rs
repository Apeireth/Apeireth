pub struct ToolClient {
    #[allow(dead_code)]
    base_url: String,
}

impl ToolClient {
    pub(crate) fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn list_tools(&self) -> Result<Vec<String>, crate::Error> {
        Ok(vec![])
    }
}
