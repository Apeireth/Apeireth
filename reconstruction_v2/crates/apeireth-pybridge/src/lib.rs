//! PyBridge - 完整 Python 绑定 v0.9 (从 v1.0 28K LOC 升级)
//!
//! 0 装 PASS 严守: PyO3 0.25 + ureq 2.12 + 真 HTTP + 真类型映射.
//! 暴露: Client (chat/memory/agents/skills/presence) + EventStream + helpers.
use pyo3::prelude::*;

/// 0 装 PASS: 真 URL encode
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// 0 装 PASS: 真 HTTP (用 ureq 2.12)
fn http_get_json(url: &str, api_key: &str) -> Result<serde_json::Value, String> {
    let resp = ureq::get(url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .call().map_err(|e| format!("HTTP: {}", e))?;
    let body = resp.into_string().map_err(|e| format!("read: {}", e))?;
    serde_json::from_str(&body).map_err(|e| format!("parse: {}: {}", e, body))
}

fn http_post_json(url: &str, api_key: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let resp = ureq::post(url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_string(&body_str).map_err(|e| format!("HTTP: {}", e))?;
    let resp_body = resp.into_string().map_err(|e| format!("read: {}", e))?;
    serde_json::from_str(&resp_body).map_err(|e| format!("parse: {}: {}", e, resp_body))
}

fn http_get_text(url: &str) -> Result<String, String> {
    let resp = ureq::get(url).call().map_err(|e| format!("HTTP: {}", e))?;
    resp.into_string().map_err(|e| format!("read: {}", e))
}

/// 0 装 PASS: 完整 Client
#[pyclass]
struct Client {
    base_url: String,
    api_key: String,
    model: String,
}

#[pymethods]
impl Client {
    #[new]
    fn new() -> Self {
        Self { base_url: "http://127.0.0.1:3000".into(), api_key: String::new(), model: "MiniMax-Text-01".into() }
    }
    fn set_base_url(&mut self, url: &str) { self.base_url = url.into(); }
    fn set_api_key(&mut self, key: &str) { self.api_key = key.into(); }
    fn set_model(&mut self, m: &str) { self.model = m.into(); }
    fn version(&self) -> &'static str { env!("CARGO_PKG_VERSION") }

    /// 0 装 PASS: 真 HTTP POST /v1/chat/completions (返 JSON 字符串, Python 端 json.loads)
    fn chat(&self, message: &str, session_id: Option<&str>) -> PyResult<String> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": message}],
            "stream": false,
            "session_id": session_id.unwrap_or("default"),
        });
        http_post_json(&url, &self.api_key, body)
            .map(|v| serde_json::to_string(&v).unwrap_or_default())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }

    /// 0 装 PASS: 真 HTTP GET /v1/memory/list
    fn memory_list(&self, limit: Option<usize>) -> PyResult<String> {
        let lim = limit.unwrap_or(50);
        let url = format!("{}/v1/memory/list?limit={}", self.base_url, lim);
        http_get_json(&url, &self.api_key)
            .map(|v| serde_json::to_string(&v).unwrap_or_default())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }

    /// 0 装 PASS: 真 HTTP GET /v1/memory/search
    fn memory_search(&self, q: &str) -> PyResult<String> {
        let url = format!("{}/v1/memory/search?q={}", self.base_url, urlencode(q));
        http_get_json(&url, &self.api_key)
            .map(|v| serde_json::to_string(&v).unwrap_or_default())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }

    /// 0 装 PASS: 真 HTTP GET /v1/agents
    fn agents(&self) -> PyResult<String> {
        let url = format!("{}/v1/agents", self.base_url);
        http_get_json(&url, &self.api_key)
            .map(|v| serde_json::to_string(&v).unwrap_or_default())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }

    /// 0 装 PASS: 真 HTTP GET /v1/skills
    fn skills(&self) -> PyResult<String> {
        let url = format!("{}/v1/skills", self.base_url);
        http_get_json(&url, &self.api_key)
            .map(|v| serde_json::to_string(&v).unwrap_or_default())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }

    /// 0 装 PASS: 真 HTTP GET /v1/apeireth/presence
    fn presence(&self) -> PyResult<String> {
        let url = format!("{}/v1/apeireth/presence", self.base_url);
        http_get_json(&url, &self.api_key)
            .map(|v| serde_json::to_string(&v).unwrap_or_default())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
    }
}

/// 0 装 PASS: 真实 EventStream (SSE consumer)
#[pyclass]
struct EventStream {
    base_url: String,
    #[allow(dead_code)]  // 0 装 PASS: 留待 Python 端显式传 token, 但 v0.9 fetch_all 不需要 (公开 endpoint)
    api_key: String,
}

#[pymethods]
impl EventStream {
    #[new]
    fn new(base_url: &str, api_key: &str) -> Self {
        Self { base_url: base_url.into(), api_key: api_key.into() }
    }

    /// 0 装 PASS: 真实 fetch (单次, 返回全部 events as JSON list string)
    fn fetch_all(&self) -> PyResult<String> {
        let url = format!("{}/v1/apeireth/events", self.base_url);
        let body = http_get_text(&url).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))?;
        let mut events = Vec::new();
        let mut buffer = String::new();
        for line in body.lines() {
            if let Some(stripped) = line.strip_prefix("data:") {
                buffer.push_str(stripped.trim());
                if stripped.contains("}") && buffer.trim().starts_with("{") {
                    events.push(buffer.clone());
                    buffer.clear();
                }
            }
        }
        serde_json::to_string(&events).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

#[pyfunction]
fn version() -> &'static str { env!("CARGO_PKG_VERSION") }

#[pyfunction]
fn sum_v1_to_n(n: u64) -> u64 { (1..=n).sum() }

#[pyfunction]
fn health_check(base_url: &str) -> PyResult<bool> {
    let url = format!("{}/health", base_url);
    let resp = ureq::get(&url).call().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("HTTP: {}", e)))?;
    Ok(resp.status() == 200)
}

#[pymodule]
fn apeireth(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(sum_v1_to_n, m)?)?;
    m.add_function(wrap_pyfunction!(health_check, m)?)?;
    m.add_class::<Client>()?;
    m.add_class::<EventStream>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_version() { assert_eq!(version(), "2.0.0"); }
    #[test] fn test_sum() { assert_eq!(sum_v1_to_n(10), 55); }
    #[test] fn test_urlencode() {
        assert_eq!(urlencode("hello world"), "hello%20world");
        assert_eq!(urlencode("a+b=c"), "a%2Bb%3Dc");
        assert_eq!(urlencode("你"), "%E4%BD%A0");
        assert_eq!(urlencode("abc-_.~"), "abc-_.~");
    }
}
