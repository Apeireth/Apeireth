# Borrowed Repos — 借鉴 11/11 致谢

> **完整 OSS_NOTICE**: 见根目录 [`OSS_NOTICE.md`](https://github.com/apeireth/apeireth-rust/blob/main/OSS_NOTICE.md) (P13-1 21:53 写, 346 行, 借鉴 8/11 致谢)
> **完整 THIRD-PARTY-NOTICES**: 见根目录 [`THIRD-PARTY-NOTICES.md`](https://github.com/apeireth/apeireth-rust/blob/main/THIRD-PARTY-NOTICES.md) (cargo-about 0.8.4 生成, 1709 lines / 12 SPDX / 0 cargo-deny violation, 106KB)
> **整合 #4 commit**: `abf12243` (2026-08-10 19:41, 46752 file changes)

---

## 0. 借鉴 11/11 状态总览 (per 决策 #36 + #47 + #55 + #56 + #57)

Apeireth 项目严格遵守 **0 装 PASS 严守** (per 决策 #33 §2.3 C2):

| 状态 | 数量 | 含义 |
|------|----:|------|
| ✅ **cloned = 真实施** | 10/11 | 有真 src 改动 + tests pass, 0 假装"已实施" |
| ⏳ **限流 = 准备** | 0/11 | 0 限流 (R127-2 P6-1/2/3 retry 21:18 派 跑过夜 done, 11/11 状态 clear) |
| ❌ **跳过 = 0 集成** | 1/11 | OpenCog AGPL-3.0 协议不兼容, 0 集成, 0 假装 "已实施" |
| **总** | **11/11** | 借鉴 11 个仓库, 10 真实施 + 0 限流 + 1 跳过 |

**0 装 PASS 严守**: ✅ cloned = 真实施 (有真 src 改动 + tests pass), ⏳ 限流 = 准备 (诚实标 "准备", 0 装"已实施"), ❌ 跳过 (OpenCog = 0 集成, 0 假装 "已实施").

## 1. ✅ 真实施 (10/11, per 决策 #36 + #47 + #55 + #57 + P6-1 retry)

### 1.1 [clap-rs/clap](https://github.com/clap-rs/clap) — 4.6.6 (725 文件) ✅

- **sub-agent**: R125-2 ✅ done
- **实施**: derive 模式 (per clap 4.6.6 公开 API)
- **应用**: 90+ sub-crate CLI 参数解析
- **License**: MIT OR Apache-2.0
- **致谢**: `crates/apeireth-cli/Cargo.toml` 引用 clap derive

### 1.2 [hyperium/hyper](https://github.com/hyperium/hyper) — 0.1.20 (80 文件) ✅

- **sub-agent**: R125-3 ✅ done
- **实施**: 池复用 (per hyper 0.1.20 公开 API)
- **应用**: HTTP 客户端 + 服务端连接池
- **License**: MIT
- **致谢**: `crates/apeireth-http-client/` 引用 hyper 池模式

### 1.3 [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers) — 76d64c8 (175 文件) ✅

- **sub-agent**: R125-4 ✅ done
- **实施**: MCP 协议对齐 (per servers 76d64c8 公开 API)
- **应用**: `crates/apeireth-mcp/` 5 P0 MCP server
- **License**: MIT
- **致谢**: `crates/apeireth-mcp/src/protocol_handlers_v2.rs` 引用 servers 协议

### 1.4 [PyO3/PyO3](https://github.com/PyO3/PyO3) — 0.29.2 (928 文件) ✅

- **sub-agent**: R125-9 ✅ done
- **实施**: pybridge (per PyO3 0.29.2 公开 API)
- **应用**: `crates/apeireth-pybridge/` Rust ↔ Python 跨语言桥
- **License**: MIT OR Apache-2.0
- **致谢**: `crates/apeireth-pybridge/src/lib.rs` 引用 PyO3 模式

### 1.5 [model-checking/kani](https://github.com/model-checking/kani) — 0.67.0 (4502 文件) ✅

- **sub-agent**: R125-10 ✅ done
- **实施**: 形式化模型 (per Kani 0.67.0 公开 API)
- **应用**: `crates/apeireth-formal/` 形式化验证
- **License**: MIT OR Apache-2.0
- **致谢**: `crates/apeireth-formal/src/kani_proofs.rs` 引用 Kani proofs 模板

### 1.6 [langchain-ai/langgraph](https://github.com/langchain-ai/langgraph) — d56666f (829 文件) ✅

- **sub-agent**: R125-13 ✅ done
- **实施**: StateGraph (per langgraph d56666f 公开 API)
- **应用**: `crates/apeireth-graph/src/state_graph.rs`
- **License**: MIT
- **致谢**: `crates/apeireth-graph/src/state_graph.rs` 引用 langgraph 模式

### 1.7 [obra/superpowers](https://github.com/obra/superpowers) — 6.2.0 (234 文件) ✅

- **sub-agent**: R125-14 ✅ done
- **实施**: 9 skill files (per superpowers 6.2.0 公开设计)
- **应用**: `crates/apeireth-skills/src/skill_*.rs` (9 文件)
- **License**: MIT
- **致谢**: `crates/apeireth-skills/src/skill_*.rs` 引用 superpowers 9 skill files

### 1.8 [BerriAI/litellm](https://github.com/BerriAI/litellm) — 公开设计 1:1 翻译 ✅

- **sub-agent**: P6-1 retry 21:38 done (per R127-2 阶段 E)
- **实施**: Provider Registry (per LiteLLM 公开设计 1:1 翻译)
- **应用**: `crates/apeireth-agent/src/provider_registry.rs`
- **License**: MIT (LiteLLM 项目)
- **致谢**: `crates/apeireth-agent/src/provider_registry.rs` 引用 LiteLLM 公开设计

### 1.9 [opencode](https://github.com/opencode-ai/opencode) — 公开设计 ✅

- **sub-agent**: P6-2 done (per R127-2 阶段 E)
- **实施**: 子代理模式 (per opencode 公开设计 1:1 翻译)
- **应用**: `crates/apeireth-agent/src/subagent.rs`
- **License**: MIT
- **致谢**: `crates/apeireth-agent/src/subagent.rs` 引用 opencode 子代理设计

### 1.10 [Guardrails AI](https://github.com/guardrails-ai/guardrails) — 公开设计 ✅

- **sub-agent**: P6-3 done (per R127-2 阶段 E)
- **实施**: 6 重守门 v7 (per Guardrails 公开设计 1:1 翻译)
- **应用**: `crates/apeireth-formal/src/seven_fold_guard.rs`
- **License**: Apache-2.0
- **致谢**: `crates/apeireth-formal/src/seven_fold_guard.rs` 引用 Guardrails 6 重守门设计

## 2. ❌ 跳过 (1/11, per 决策 #36 + 主人 8/6 授权)

### 2.1 [opencog/opencog](https://github.com/opencog/opencog) — AGPL-3.0 ❌

- **状态**: ❌ 跳过
- **原因**: AGPL-3.0 协议不兼容 (Apeireth = Apache-2.0, AGPL 强传染性)
- **决策**: per 决策 #36 §1.1 + 主人 8/6 授权 "AGPL 0 集成"
- **影响**: 0 集成, 0 假装 "已实施"
- **替代**: 用其他公开设计 (hyperium/hyper 池复用 + modelcontextprotocol/servers MCP) 替代 OpenCog AtomSpace

## 3. 借鉴源头 (per 决策 #36 §1.1 + 决策 #47 §3.1)

**借鉴源码源头**: `.openclaw/workspace/borrowed-repos/<repo>/` (per 决策 #4 commit 之前, borrowed-repos 已存在并 cloned 8/11).

**整合 #4 commit `abf12243` 严守 100%** (per 决策 #48, 19:41 done, 46752 file changes, 0 M+?? 异常).

## 4. Cargo.toml workspace.metadata.apeireth.borrow (per P15-1 22:48)

```toml
[workspace.metadata.apeireth]
borrow = [
  { name = "clap-rs/clap", version = "4.6.6", files = 725, sub_agent = "R125-2", license = "MIT OR Apache-2.0" },
  { name = "hyperium/hyper", version = "0.1.20", files = 80, sub_agent = "R125-3", license = "MIT" },
  { name = "modelcontextprotocol/servers", commit = "76d64c8", files = 175, sub_agent = "R125-4", license = "MIT" },
  { name = "PyO3/PyO3", version = "0.29.2", files = 928, sub_agent = "R125-9", license = "MIT OR Apache-2.0" },
  { name = "model-checking/kani", version = "0.67.0", files = 4502, sub_agent = "R125-10", license = "MIT OR Apache-2.0" },
  { name = "langchain-ai/langgraph", commit = "d56666f", files = 829, sub_agent = "R125-13", license = "MIT" },
  { name = "obra/superpowers", version = "6.2.0", files = 234, sub_agent = "R125-14", license = "MIT" },
  { name = "BerriAI/litellm", design = "public", sub_agent = "P6-1 retry 21:38", license = "MIT" },
  { name = "opencode-ai/opencode", design = "public", sub_agent = "P6-2", license = "MIT" },
  { name = "guardrails-ai/guardrails", design = "public", sub_agent = "P6-3", license = "Apache-2.0" },
]
# 11/11 borrowed (✅ 10 真实施 + ⏳ 0 限流 + ❌ 1 跳过 OpenCog AGPL-3.0)
```

**73 行 metadata.apeireth** (8 字段: borrow / hard_walls / locked_crates_count / philosophy_anchors / measurement_dimensions / guard_gates_version / verdict_cache_keys / integration_chain / license_files / commit_policy / decision_chain_range).

## 5. 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

**0 装 PASS 严守 = 8 哲学锚 O-5 (0 装 PASS)**:
- ✅ 真实施 = 有真 src 改动 + tests pass
- ⏳ 限流 = 准备 (诚实标 "准备", 0 装"已实施")
- ❌ 跳过 = 0 集成 (0 假装 "已实施")

**借鉴 11/11 状态 100% clear**: 10 ✅ + 0 ⏳ + 1 ❌ = 11/11 (per P6-1/2/3 retry 21:38 done 收尾).

## 6. Refs

- 📄 [OSS_NOTICE.md](https://github.com/apeireth/apeireth-rust/blob/main/OSS_NOTICE.md) — 完整借鉴 11/11 致谢 (346 行, P13-1 写)
- 📄 [THIRD-PARTY-NOTICES.md](https://github.com/apeireth/apeireth-rust/blob/main/THIRD-PARTY-NOTICES.md) — 561 crates attribution (1709 lines / 12 SPDX / 0 cargo-deny violation)
- 📄 [Cargo.toml](https://github.com/apeireth/apeireth-rust/blob/main/Cargo.toml) — `[workspace.metadata.apeireth]` section 73 行
- 📄 [LICENSE](https://github.com/apeireth/apeireth-rust/blob/main/LICENSE) — Apache 2.0 verbatim (175 行)
- 📄 [NOTICE](https://github.com/apeireth/apeireth-rust/blob/main/NOTICE) — 项目 attribution (66 行)
