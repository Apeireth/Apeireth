# P1.2 SDK 骨架 crate 砍/填决策表 (R131 2026-08-12)

> 评估 17-20 个 SDK 翻译型/工具型 crate 的真实度, 给出 **砍/填/留** 决策.
> 决策原则: 0 假装 LLM (LARK/VOICE/LIVEKIT 真接) > 留骨架 (i18n 数据驱动) > 砍 (高 stub 标记).

## 1. 现状清单 (R131 评估)

| # | Crate | 状态 | 实现度 | 决策 | 理由 |
|---|---|---|---|---|---|
| 1 | `apeireth-credentials` | ❌ 已删 | 0% | **已砍** | R20 阶段 6 1:1 翻译商业版, 0 真接 |
| 2 | `apeireth-cache` | ❌ 已删 | 0% | **已砍** | LRU/TTL 骨架, 仅 Memory 真接 |
| 3 | `apeireth-tracing` | ❌ 已删 | 0% | **已砍** | 分布式追踪骨架, 仅 stdout/file |
| 4 | `apeireth-metrics` | ❌ 已删 | 0% | **已砍** | Prometheus 骨架, 0 真接 |
| 5 | `apeireth-oauth` | ❌ 已删 | 0% | **已砍** | 3 Provider trait stub |
| 6 | `apeireth-update` | ❌ 已删 | 0% | **已砍** | autoupdate 骨架, 0 GitHub Releases |
| 7 | `apeireth-sandbox` | ❌ 已删 | 0% | **已砍** | Container/Process/WASM 骨架, bollard 占位 |
| 8 | `apeireth-mcp-ssh` | ✅ 保留 | 25% | **冻结** | 16 stub 标记, 4 fn, 0 真接 ssh2 lib |
| 9 | `apeireth-mcp-winrm` | ✅ 保留 | 25% | **冻结** | 24 stub 标记, 7 fn, 0 真接 winrm |
| 10 | `apeireth-mcp-relay-image` | ✅ 保留 | 50% | **冻结** | 17 stub 标记, 18 fn, 11 impl, 部分真接 |
| 11 | `apeireth-lark` | ✅ 保留 | 80% | **保留 + 填** | 4 stub 标记, 7 impl, 9 测试, SDK stub |
| 12 | `apeireth-voice` | ✅ 保留 | 85% | **保留 + 填** | 5 stub 标记, 17 impl, 15 测试, TTS SDK |
| 13 | `apeireth-livekit` | ✅ 保留 | 95% | **保留 + 填** | 0 stub 标记, 29 impl, 22 fn, 16 测试, 真接 |
| 14 | `apeireth-keyring` | ❌ 已删 | 0% | **已砍** | OS keyring 基础设施 |
| 15 | `apeireth-machine-id` | ❌ 已删 | 0% | **已砍** | 机器 ID 基础设施 |
| 16 | `apeireth-repo-scan` | ❌ 已删 | 0% | **已砍** | 仓库扫描工具 |
| 17 | `apeireth-repo-analyzer` | ❌ 已删 | 0% | **已砍** | 仓库分析工具 |
| 18 | `apeireth-i18n` | ✅ 保留 | 60% | **保留 + 填** | 17 stub 标记, 10 fn, 13 测试, 数据驱动 |
| 19 | `apeireth-image-prompt` | ❌ 已删 | 0% | **已砍** | 生图 prompt 工具 |
| 20 | `apeireth-plugin` | ❌ 已删 | 0% | **已砍** | 插件系统骨架 |

## 2. 现状: 13/20 已砍, 7/20 保留

**已砍 (13 个):** credentials / cache / tracing / metrics / oauth / update / sandbox / keyring / machine-id / repo-scan / repo-analyzer / image-prompt / plugin

**R131 决策 (7 个):**
- **冻结 (3 个, 等真接需求):** mcp-ssh / mcp-winrm / mcp-relay-image — 0 假装路径, 等业务真需
- **保留 + 填 (4 个, R132-R134 续):** lark / voice / livekit / i18n

## 3. 详细决策

### 3.1 冻结 (3 个 MCP 远程管理) - 理由

**apeireth-mcp-ssh** (16 stub, 4 fn):
- 0 真接 ssh2 lib (Rust 主流 ssh crate)
- 仅类型定义 + 错误枚举
- 实际路径: WS-MCP 暴露 ssh_connect / ssh_exec / ssh_keepalive 工具
- 当前状态: Apeireth **不通过 MCP 远程 shell** — 主人直跑 daemon
- 决策: 冻结, 等真正"远程 MCP 控制"业务需求

**apeireth-mcp-winrm** (24 stub, 7 fn):
- 与 SSH 类似, Windows 远程管理
- 0 真接 winrm crate
- 决策: 冻结, 同 SSH 理由

**apeireth-mcp-relay-image** (17 stub, 18 fn, 11 impl):
- 图像中转 MCP Server
- 实现度 50% (有 fn + impl)
- 决策: 冻结, 等图像中转场景

### 3.2 保留 + 填 (4 个真接 provider) - 理由

**apeireth-livekit** (29 impl, 22 fn, 16 test, 0 stub) **优先级 P0**:
- 这是 **真接 SDK** — `livekit` crate 真的集成
- 0 stub 标记 → 真实可用
- 决策: 保留 + R132 续填 (TUI 视频/音频需要)

**apeireth-voice** (17 impl, 29 fn, 15 test, 5 stub) **优先级 P0**:
- SDK stub 多但 fn/impl 充足
- TTS / 语音播报路径
- 决策: 保留 + R132 续填

**apeireth-lark** (7 impl, 10 fn, 9 test, 4 stub) **优先级 P1**:
- 飞书 SDK stub
- 决策: 保留 + R133 续填 (需要飞书消息场景)

**apeireth-i18n** (7 impl, 10 fn, 13 test, 17 stub) **优先级 P1**:
- 数据驱动 i18n (locale TOML)
- 5 语言 × 12 类别 × 69 keys = 4140 翻译条
- 决策: 保留 + R133 续填 (TUI 需要多语言)

## 4. 8 硬墙 0 越界严守

- ✅ 0 改任何 LOCKED entry signature
- ✅ 0 主动 commit 改动
- ✅ 0 增 workspace member (13 个砍的已经删)
- ✅ 0 改 version 1.2.0

## 5. 落地时间表

| 阶段 | 任务 | R |
|---|---|---|
| 即时 | 7 MCP crate freeze 标记 (R131) | R131 (本次) |
| R132 | livekit + voice 真接 TUI 视频/音频 | R132 |
| R133 | lark + i18n 续填 | R133 |
| R134+ | 按业务需求重新启动 SSH/WinRM | R134+ |

## 6. 不假装登记

- 这 7 个保留 crate **仅是工具型**, 0 涉及 L0/L1 守门
- Self-Disable / 8 重守门 / 9 重守门 0 触碰
- 24 LOCKED entry 0 触碰
- 不假装标注: R131 决策仅是"砍/填"清单, 0 实施删除 (保留 workspace member 状态)

---

_R131 (2026-08-12) 决策. 主人拍板后续 R132+ 续填. 详见 `docs/conventions/16-crate-merge-policy.md`._
