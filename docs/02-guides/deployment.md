# Apeireth Deployment Guide

> 对齐实际运行方式（2026-08-18）。目标：把伙伴端点跑起来、接前端、理解持久化与安全配置。

## 环境要求

| 项 | 要求 |
|---|---|
| OS | Windows 10+（Job Object 沙箱完整）；Linux/macOS 可运行（Windows 专属安全件如实降级）|
| Rust | 1.97.1（`rust-toolchain.toml` 锁定）|
| LLM API Key | 必需（真 LLM 功能）；MiniMax 兼容（OpenAI 协议）|

## 环境变量（完整清单，对齐代码）

| 变量 | 默认 | 说明 |
|---|---|---|
| `APEIRETH_API_KEY` | 无（必需）| LLM API key |
| `APEIRETH_MAX_TOKENS` | 见代码 | 单轮生成上限 |
| `PORT` | 8090 | 伙伴端点端口 |
| `APEIRETH_SEED_MEMORY` | 无 | 种子记忆（分号分隔；不设则从零积累）|
| `APEIRETH_GRANT` | 无 | 显式扩权，如 `FileOperator:24`（工具:小时）|
| `APEIRETH_EXTRACT_INTERVAL_SECONDS` | 600 | 记忆提炼节流 |
| `APEIRETH_DREAM_QUIET_SECONDS` | 21600 (6h) | 做梦安静期 |
| `APEIRETH_REFLECT_PERIOD_HOURS` | 24 | 反思周期 |
| `APEIRETH_MASTER_TOKEN` | 无 | 预留：管理令牌口 |
| `APEIRETH_CONTINUITY_ID` | 自动 | 连续性锚点（多前端同一叙事）|
| `APEIRETH_LARK_APP_ID/SECRET/RECEIVE_ID` | 无 | 飞书送达（可选）|
| `APEIRETH_TELEGRAM_BOT_TOKEN/CHAT_ID` | 无 | Telegram 送达（可选）|

## 启动

```powershell
# PowerShell:
$env:APEIRETH_API_KEY = (Get-Content C:\path\to\your-key.txt -Raw).Trim()
cargo run -p apeireth-companion --example companion_serve
```

预期输出：

```
[app] CompanionApp 装配完成: L0 Identity + L1 Essential 常驻, 提炼 600s 节流
✅ companion_serve v4 — 伙伴端点全能力版 (CompanionApp 机制装配)
http://127.0.0.1:8090/panel  (Web 面板 v2)
http://127.0.0.1:8090/v1  (模型 MiniMax-M3, Key 任意非空)
```

## 持久化

| 数据 | 位置 |
|---|---|
| 记忆库 | `%APPDATA%\apeireth\memory.sqlite`（跨重启持久）|
| 晋级候选 | `%APPDATA%\apeireth\promotion-candidates.md` |

## 前端接入（任何 OpenAI 兼容前端）

1. Base URL → `http://127.0.0.1:8090/v1`
2. API Key → 任意非空字符串
3. Model → `MiniMax-M3`

前端即获得：记忆注入、L0/L1 人格、工具桥（审批入队）、daemon 常驻（做梦/反思/涌现推送）。

## 生产注意事项

- **默认无认证**：伙伴端点接受任意 Bearer——本地单用户场景设计；公网部署前必须接真实令牌/网关
- **出站策略**：`apeireth-http-client::egress` 默认不启用（None=放行）；启用后白名单外域名默认拒绝 + 审计链——LLM 调用域名需入白名单
- **工具审批**：高危工具（FileOperator/ShellExec 等）默认需主人批准；`APEIRETH_GRANT` 显式扩权
- **Docker**：`Dockerfile` 已就绪（3-stage/multi-arch/distroless+nonroot/健康检查），**构建实测待补**（当前环境无 docker，如实标注）

## 故障排查

| 现象 | 检查 |
|---|---|
| 对话返回空 content | `[llm] 轮N 成功` 日志；API key 是否有效；`APEIRETH_MAX_TOKENS` 是否过小 |
| 工具执行"需要主人批准" | 预期行为——查 `/v1/apeireth/approval-requests` 队列 |
| 出站失败 | egress 白名单是否含目标域名 |
| 记忆不更新 | `APEIRETH_EXTRACT_INTERVAL_SECONDS` 节流；确认消息进入 build_injection 路径 |
