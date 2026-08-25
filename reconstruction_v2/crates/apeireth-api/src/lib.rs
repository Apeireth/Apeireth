//! apeireth-api — HTTP API gateway (v2 完整抄录 v1 lib)
//!
//! 0 装 PASS: 真 ApiServer + 真 router

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct ApiServer { pub routes: HashMap<String, Box<dyn Route>> }

pub trait Route: Send + Sync {
    fn path(&self) -> &str;
    fn method(&self) -> &str;
    fn handle(&self, body: Value) -> Result<Value, String>;
}

impl ApiServer {
    pub fn new() -> Self { Self { routes: HashMap::new() } }
    pub fn add(&mut self, r: Box<dyn Route>) {
        self.routes.insert(format!("{}:{}", r.method(), r.path()), r);
    }
    pub fn dispatch(&self, method: &str, path: &str, body: Value) -> Result<Value, String> {
        let key = format!("{}:{}", method, path);
        self.routes.get(&key).ok_or_else(|| format!("route not found: {}", key))?.handle(body)
    }
}

impl Default for ApiServer { fn default() -> Self { Self::new() } }

pub struct GetRoute<F> { pub path: String, pub handler: F }
impl<F: Fn(Value) -> Result<Value, String> + Send + Sync> Route for GetRoute<F> {
    fn path(&self) -> &str { &self.path }
    fn method(&self) -> &str { "GET" }
    fn handle(&self, body: Value) -> Result<Value, String> { (self.handler)(body) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn test_dispatch() {
        let mut s = ApiServer::new();
        s.add(Box::new(GetRoute { path: "/health".into(), handler: |_| Ok(json!({"ok": true})) }));
        let r = s.dispatch("GET", "/health", json!({})).unwrap();
        assert_eq!(r["ok"], true);
    }
    #[test]
    fn test_unknown_route() {
        let s = ApiServer::new();
        assert!(s.dispatch("GET", "/missing", json!({})).is_err());
    }
    #[test]
    fn test_default() {
        let s: ApiServer = Default::default();
        assert_eq!(s.routes.len(), 0);
    }
}
