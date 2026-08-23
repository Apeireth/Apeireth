use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::process::Stdio;
use tokio::process::{Child, Command};

use super::protocol::{JsonRpcRequest, JsonRpcResponse, JsonRpcNotification};

#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send_request(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, String>;
    async fn send_notification(&self, notif: JsonRpcNotification) -> Result<(), String>;
}

// -----------------------------------------------------------------------------
// In-Memory Direct Channel Transport (for ultra-low latency & integration tests)
// -----------------------------------------------------------------------------

pub struct MemoryTransport {
    req_tx: mpsc::Sender<JsonRpcRequest>,
    resp_rx: Arc<Mutex<mpsc::Receiver<JsonRpcResponse>>>,
}

impl MemoryTransport {
    pub fn pair(buffer_size: usize) -> (Self, mpsc::Receiver<JsonRpcRequest>, mpsc::Sender<JsonRpcResponse>) {
        let (req_tx, req_rx) = mpsc::channel(buffer_size);
        let (resp_tx, resp_rx) = mpsc::channel(buffer_size);

        let transport = Self {
            req_tx,
            resp_rx: Arc::new(Mutex::new(resp_rx)),
        };

        (transport, req_rx, resp_tx)
    }
}

#[async_trait]
impl McpTransport for MemoryTransport {
    async fn send_request(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, String> {
        self.req_tx.send(req).await.map_err(|e| format!("Memory transport send error: {}", e))?;
        let mut rx = self.resp_rx.lock().await;
        rx.recv().await.ok_or_else(|| "Memory transport channel closed".into())
    }

    async fn send_notification(&self, _notif: JsonRpcNotification) -> Result<(), String> {
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Stdio Child Process Transport (for spawning standard npx / uvx / binary MCP servers)
// -----------------------------------------------------------------------------

pub struct StdioTransport {
    #[allow(dead_code)]
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,

    reader: Arc<Mutex<BufReader<tokio::process::ChildStdout>>>,
}

impl StdioTransport {
    pub fn spawn(cmd: &str, args: &[&str]) -> Result<Self, String> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn MCP process '{}': {}", cmd, e))?;

        let stdin = child.stdin.take().ok_or("Failed to capture MCP child stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to capture MCP child stdout")?;

        Ok(Self {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            reader: Arc::new(Mutex::new(BufReader::new(stdout))),
        })
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send_request(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, String> {
        let req_id = req.id.clone();
        let mut req_str = serde_json::to_string(&req).map_err(|e| e.to_string())?;

        req_str.push('\n');

        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(req_str.as_bytes()).await.map_err(|e| format!("Stdio write error: {}", e))?;
            stdin.flush().await.map_err(|e| format!("Stdio flush error: {}", e))?;
        }

        let mut reader = self.reader.lock().await;
        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line).await.map_err(|e| format!("Stdio read error: {}", e))?;
            if bytes_read == 0 {
                return Err("MCP process closed stdout stream prematurely".into());
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(trimmed) {
                if resp.id == req_id {
                    return Ok(resp);
                }
            }
        }
    }


    async fn send_notification(&self, notif: JsonRpcNotification) -> Result<(), String> {
        let mut notif_str = serde_json::to_string(&notif).map_err(|e| e.to_string())?;
        notif_str.push('\n');

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(notif_str.as_bytes()).await.map_err(|e| format!("Stdio write error: {}", e))?;
        stdin.flush().await.map_err(|e| format!("Stdio flush error: {}", e))?;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// SSE / HTTP Streamable Transport (for remote MCP web services)
// -----------------------------------------------------------------------------

pub struct SseTransport {
    endpoint_url: String,
    client: reqwest::Client,
}

impl SseTransport {
    pub fn new(endpoint_url: impl Into<String>) -> Self {
        Self {
            endpoint_url: endpoint_url.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl McpTransport for SseTransport {
    async fn send_request(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, String> {
        let resp = self.client.post(&self.endpoint_url)
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("SSE transport HTTP error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("SSE transport returned HTTP {}", resp.status()));
        }

        resp.json::<JsonRpcResponse>().await.map_err(|e| format!("Failed to decode SSE JSON-RPC response: {}", e))
    }

    async fn send_notification(&self, notif: JsonRpcNotification) -> Result<(), String> {
        let _ = self.client.post(&self.endpoint_url)
            .json(&notif)
            .send()
            .await;
        Ok(())
    }
}
