//! `apeireth-tools::github_accel` — GitHub 加速节点池 (调研驱动: xiake.pro 聚合池).
//!
//! 调研 (docs/ref-gh-accel.md, 2026-08-16):
//! - [xiake.pro](https://xiake.pro) 是 GitHub 加速镜像聚合站: 节点池 JSON API
//!   `https://xiake.pro/static/node.json` → { code, data: [{url, server, ip, location, latency, speed}] }
//! - 用法 = 前缀式代理: `https://{节点}/https://{github链接}`
//! - 站侧 latency 是**服务器视角基线**, 实测本机 10 节点仅 3 可用 (最快 0.82s,
//!   站标 17ms 的节点反而超时) → **每次使用必须本机实测选最快** (主人要求: ping 选最快)
//!
//! 设计:
//! - `fetch_mirror_pool`: 拉节点池 + 去重 + 按站侧延迟排序 (探测优先级)
//! - `probe_top`: 并发实测 (真实请求 raw.githubusercontent.com 测试文件, 6s 超时)
//! - `pick_fastest`: 选最快可用节点 (HTTP 2xx 才算可用; 404 = 活着但不支持 raw, 如实标注)
//!
//! 0 假装 (诚实):
//! - 节点全是第三方免费服务, 随时可能失效; 结果只代表「本次实测时刻」
//! - 测的是 HTTP 整链路耗时 (DNS+TCP+TLS+下载), 不是 ICMP ping (代理常禁 ping)
//! - 本工具只**探测与返回命令**, 不执行命令、不改环境 (执行是 ShellExec 的职责, 受审批守护)

use std::sync::Arc;
use std::time::Duration;

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};

/// 节点池 API (xiake.pro 聚合).
pub const POOL_API: &str = "https://xiake.pro/static/node.json";
/// 探测测试文件 (与 xiake.pro 前端同款: vscode 图标, 小文件).
pub const PROBE_FILE: &str = "https://raw.githubusercontent.com/microsoft/vscode/main/resources/linux/code.png";
/// 单节点探测超时.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(6);
/// 最大并发探测.
pub const PROBE_CONCURRENCY: usize = 4;
/// 默认探测节点数 (站侧延迟最低的前 N 个).
pub const DEFAULT_PROBE_LIMIT: usize = 12;
/// 探测上限 (防滥用).
pub const MAX_PROBE_LIMIT: usize = 30;

/// 节点池条目 (站侧元数据, 直通 node.json 字段).
#[derive(Debug, Clone, Deserialize)]
pub struct MirrorNode {
    pub url: String,
    #[serde(default)]
    pub server: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub location: String,
    /// 站侧基准延迟 ms (仅作探测优先级, 不可信为本机延迟).
    #[serde(default)]
    pub latency: u64,
    #[serde(default)]
    pub speed: f64,
}

/// 单节点实测结果.
#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub node: MirrorNode,
    /// 整链路耗时 ms (None = 失败/超时).
    pub latency_ms: Option<u64>,
    /// HTTP 状态码 (None = 网络层失败).
    pub http_status: Option<u16>,
    /// 内容级验证: 2xx 且 body 是真实测试文件 (PNG 魔数) — 防 HTML 包装页充数.
    pub verified: bool,
    /// 补充说明 (如 "html_wrapper" / "read body 失败").
    pub note: String,
}

impl ProbeResult {
    pub fn ok(&self) -> bool {
        self.verified
    }
}

/// 拉取节点池: GET node.json → 去重 (按 url) → 按站侧延迟升序 (探测优先级).
pub async fn fetch_mirror_pool(client: &reqwest::Client) -> Result<Vec<MirrorNode>, String> {
    let resp = client
        .get(POOL_API)
        .send()
        .await
        .map_err(|e| format!("拉取节点池失败 ({POOL_API}): {e}"))?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(format!("节点池 API 返回 {status}"));
    }
    let text = resp.text().await.map_err(|e| format!("读节点池响应失败: {e}"))?;
    parse_pool(&text)
}

/// 解析节点池 (纯函数, 可单测无网络): 校验 code==200 + data 数组 + 去重.
pub fn parse_pool(text: &str) -> Result<Vec<MirrorNode>, String> {
    #[derive(Deserialize)]
    struct Pool {
        code: i64,
        #[serde(default)]
        data: Vec<MirrorNode>,
    }
    let pool: Pool = serde_json::from_str(text).map_err(|e| format!("节点池 JSON 解析失败: {e}"))?;
    if pool.code != 200 {
        return Err(format!("节点池 code != 200: {}", pool.code));
    }
    let mut seen = std::collections::HashSet::new();
    let mut nodes: Vec<MirrorNode> = pool
        .data
        .into_iter()
        .filter(|n| n.url.starts_with("https://") || n.url.starts_with("http://"))
        .filter(|n| seen.insert(n.url.clone()))
        .collect();
    nodes.sort_by_key(|n| n.latency);
    if nodes.is_empty() {
        return Err("节点池为空".to_string());
    }
    Ok(nodes)
}

/// 并发实测节点 (取前 limit 个; 站侧延迟低者优先, 仅作探测顺序).
pub async fn probe_top(
    client: &reqwest::Client,
    nodes: &[MirrorNode],
    limit: usize,
) -> Vec<ProbeResult> {
    let limit = limit.clamp(1, MAX_PROBE_LIMIT);
    let batch: Vec<MirrorNode> = nodes.iter().take(limit).cloned().collect();
    let results = futures::stream::iter(batch)
        .map(|node| {
            let client = client.clone();
            async move { probe_one(&client, &node).await }
        })
        .buffer_unordered(PROBE_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;
    results
}

/// 实测单节点: GET {节点}/{测试文件}, 计时整链路 + **内容级验证** (PNG 魔数).
///
/// 实测教训 (gh_accel_demo 首跑): 某节点返回 200 但 body 是 HTML 包装页 —
/// 光看状态码会选到"假可用"节点, 所以必须验内容 (0 装 PASS: 不以假充真).
pub async fn probe_one(client: &reqwest::Client, node: &MirrorNode) -> ProbeResult {
    let url = format!("{}/{}", node.url.trim_end_matches('/'), PROBE_FILE);
    let start = std::time::Instant::now();
    match client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let ct = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            match resp.bytes().await {
                Ok(bytes) => {
                    let latency = start.elapsed().as_millis() as u64;
                    let is_png = bytes.len() >= 4
                        && bytes[0] == 0x89
                        && bytes[1] == b'P'
                        && bytes[2] == b'N'
                        && bytes[3] == b'G';
                    let verified = (200..300).contains(&status) && is_png;
                    let note = if verified {
                        String::new()
                    } else if (200..300).contains(&status) && !is_png {
                        format!("2xx 但非真实文件 (content-type: {ct}, 前 {} 字节非 PNG)", bytes.len().min(16))
                    } else {
                        format!("HTTP {status}")
                    };
                    ProbeResult {
                        node: node.clone(),
                        latency_ms: Some(latency),
                        http_status: Some(status),
                        verified,
                        note,
                    }
                }
                Err(_) => ProbeResult {
                    node: node.clone(),
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    http_status: Some(status),
                    verified: false,
                    note: "读 body 失败".to_string(),
                },
            }
        }
        Err(_) => ProbeResult {
            node: node.clone(),
            latency_ms: None,
            http_status: None,
            verified: false,
            note: "网络层失败/超时".to_string(),
        },
    }
}

/// 选最快可用节点 (HTTP 2xx 中耗时最小者).
pub fn pick_fastest(probes: &[ProbeResult]) -> Option<&ProbeResult> {
    probes
        .iter()
        .filter(|p| p.ok())
        .min_by_key(|p| p.latency_ms.unwrap_or(u64::MAX))
}

/// 生成加速 URL: `https://{节点}/{github链接}` (去协议前缀).
pub fn accelerate_url(node_url: &str, github_url: &str) -> String {
    let bare = github_url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    format!("{}/https://{bare}", node_url.trim_end_matches('/'))
}

/// GitHub 加速工具: 拉池 → 实测 → 选最快 → 返回可用命令.
pub struct GhAccelTool;

#[async_trait]
impl Tool for GhAccelTool {
    fn name(&self) -> &str {
        "gh_accel"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Async
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: apeireth_tool_registry::types::TriggerAxis::OnDemand,
            awaiting: apeireth_tool_registry::types::AwaitingAxis::Immediate,
            resident: apeireth_tool_registry::types::ResidentAxis::Ephemeral,
            transport: apeireth_tool_registry::types::TransportAxis::Network,
            output: apeireth_tool_registry::types::OutputAxis::Value,
        }
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(DEFAULT_PROBE_LIMIT as u64) as usize;
        let github_url = args.get("github_url").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty());
        let client = reqwest::Client::builder()
            .timeout(PROBE_TIMEOUT)
            .user_agent("Mozilla/5.0 (Apeireth gh_accel)")
            .build()
            .map_err(|e| format!("HTTP 客户端构建失败: {e}"))?;
        let pool = fetch_mirror_pool(&client).await?;
        let probes = probe_top(&client, &pool, limit).await;
        let fastest = pick_fastest(&probes);
        // 输出排序: 可用优先, 再按实测延迟升序 (最快在最前, 方便人/AI 读表)
        let mut sorted = probes.clone();
        sorted.sort_by(|a, b| {
            b.ok()
                .cmp(&a.ok())
                .then_with(|| a.latency_ms.unwrap_or(u64::MAX).cmp(&b.latency_ms.unwrap_or(u64::MAX)))
        });
        let mut out = json!({
            "pool_total": pool.len(),
            "probed": probes.len(),
            "test_file": PROBE_FILE,
            "results": sorted.iter().map(|p| json!({
                "host": host_of(&p.node.url),
                "url": p.node.url,
                "server": p.node.server,
                "location": p.node.location.trim(),
                "site_latency_ms": p.node.latency,
                "measured_ms": p.latency_ms,
                "http_status": p.http_status,
                "verified": p.verified,
                "note": p.note,
                "ok": p.ok(),
            })).collect::<Vec<_>>(),
            "note": "节点为第三方免费服务, 结果只代表本次实测; 站侧延迟仅作探测顺序, 不可信为本机延迟; ok = 2xx 且内容验证通过 (PNG 魔数)"
        });
        match fastest {
            Some(f) => {
                out["fastest"] = json!({
                    "host": host_of(&f.node.url),
                    "url": f.node.url,
                    "measured_ms": f.latency_ms,
                    "http_status": f.http_status,
                });
                if let Some(gh) = github_url {
                    let acc = accelerate_url(&f.node.url, gh);
                    out["accelerated_url"] = json!(acc);
                    out["commands"] = json!({
                        "git_clone": format!("git clone {acc}"),
                        "curl": format!("curl -LO {acc}"),
                    });
                }
            }
            None => {
                out["fastest"] = json!(null);
                out["note"] = json!(format!("{} 个节点全部不可用 (2xx=可用) — 免费节点池常有死节点, 稍后重试或换源", probes.len()));
            }
        }
        Ok(out)
    }
}

fn host_of(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "code": 200,
        "msg": "success",
        "data": [
            {"url":"https://gh.a.com","server":"cloudflare","ip":"1.1.1.1","location":"  ","latency":500,"speed":0.3},
            {"url":"https://gh.b.com","server":"nginx","ip":"2.2.2.2","location":"Hong Kong","latency":100,"speed":0.9},
            {"url":"https://gh.a.com","server":"cloudflare","ip":"1.1.1.1","location":"  ","latency":500,"speed":0.3},
            {"url":"http://gh.c.com","server":"x","ip":"3.3.3.3","location":"","latency":999,"speed":0.0}
        ],
        "total": 4,
        "update_time": "2026-01-01 00:00:00"
    }"#;

    #[test]
    fn parse_pool_dedupes_and_sorts() {
        let nodes = parse_pool(FIXTURE).unwrap();
        assert_eq!(nodes.len(), 3, "去重 (gh.a.com ×2) + 只留 http(s)");
        assert_eq!(nodes[0].url, "https://gh.b.com", "按站侧延迟升序 (100 最前)");
    }

    #[test]
    fn parse_pool_rejects_bad_code() {
        assert!(parse_pool(r#"{"code":500,"data":[]}"#).is_err());
        assert!(parse_pool("not json").is_err());
        assert!(parse_pool(r#"{"code":200,"data":[]}"#).is_err(), "空池拒绝");
    }

    #[test]
    fn pick_fastest_prefers_ok_and_low_latency() {
        let mk = |url: &str, lat: Option<u64>, status: Option<u16>, verified: bool| ProbeResult {
            node: MirrorNode { url: url.into(), server: String::new(), ip: String::new(), location: String::new(), latency: 0, speed: 0.0 },
            latency_ms: lat,
            http_status: status,
            verified,
            note: String::new(),
        };
        let probes = vec![
            mk("https://a.com", Some(900), Some(200), true),
            mk("https://b.com", Some(300), Some(200), true),
            mk("https://c.com", Some(50), Some(404), false),   // 404 不可用
            mk("https://d.com", Some(50), Some(200), false),   // 200 但 HTML 包装页 → 不可用
            mk("https://e.com", None, None, false),            // 网络失败
        ];
        let best = pick_fastest(&probes).unwrap();
        assert_eq!(best.node.url, "https://b.com");
        assert_eq!(best.latency_ms, Some(300));
        // 全挂 → None
        let all_dead = vec![mk("https://c.com", Some(50), Some(404), false), mk("https://e.com", None, None, false)];
        assert!(pick_fastest(&all_dead).is_none());
    }

    #[test]
    fn accelerate_url_strips_scheme() {
        assert_eq!(
            accelerate_url("https://gh.a.com/", "https://github.com/user/repo.git"),
            "https://gh.a.com/https://github.com/user/repo.git"
        );
        assert_eq!(
            accelerate_url("https://gh.a.com", "http://raw.githubusercontent.com/x/y"),
            "https://gh.a.com/https://raw.githubusercontent.com/x/y"
        );
    }

    #[test]
    fn host_of_works() {
        assert_eq!(host_of("https://gh.a.com/"), "gh.a.com");
        assert_eq!(host_of("http://gh.b.com"), "gh.b.com");
    }
}
