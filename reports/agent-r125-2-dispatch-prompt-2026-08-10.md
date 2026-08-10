# R125-2 Sub-Agent Dispatch Prompt (clap derive 重构 commands.rs)

**Date**: 2026-08-10 17:28
**Author**: R125 P0 supervisor
**Receiving agent**: R125-2 sub-agent

---

## 任务

**主题**: clap derive 重构 `apeireth-cli/src/commands.rs` (26.5KB → 12KB, -55%)

**借鉴 ID**: `R124-1-BORROW-clap-rs/clap-4f7a2c1-2026-08-10`

**借鉴源码**: `.openclaw\workspace\borrowed-repos\clap\`

**目标文件**:
- `Apeireth-rust/crates/apeireth-cli/src/commands.rs` (26.5KB → 12KB, 重构)
- `Apeireth-rust/crates/apeireth-cli/src/main.rs` (0 改入口签名, 仅 update imports)
- `Apeireth-rust/crates/apeireth-cli/src/lib.rs` (按需 re-export)

**B1 24 LOCKED 持续更新 (per 主人 17:22)**: commands.rs **不在 24 LOCKED 名单** (apeireth-cli 是工具, 实施可改, 0 触碰入口)

**估时**: 4-6h

**截止**: 8/11 8:00 (过夜)

---

## 0 装解除 (主人 17:22)

**借鉴源码 verify**:
```bash
Test-Path '.openclaw\workspace\borrowed-repos\clap\.git'
```

- ✅ cloned = 真重构
- ⏳ 限流中 = 0 重构, 报告"借鉴 ID 索引完成, src 0 改"
- ❌ 永久失败 = 报 supervisor

---

## 8 硬墙 (B1-B7 升级版 + A1-A3 + C1-C3)

| # | 必守 |
|---|------|
| 1 | B2 0 触碰 workspace.version |
| 2 | A1 0 触碰 R11 baseline 3 值 |
| 3 | B1 commands.rs 不在 24 LOCKED, 实施可改. **入口签名 0 改** (apeireth_cli::run() 等) |
| 4-7 | B3-B6 0 改原实质 (5/6/8/25/13 维是扩展) |
| 8 | C1 0 commit, C2 0 装解除 (借鉴 cloned 才真实施), C3 0 装 5 项升 6 重 v6, 0 push |

---

## 实施步骤 (5 阶段)

### 阶段 1: 现状盘点 (15 min)
- 读 commands.rs 全部 26.5KB, 列出所有 8-10 个 subcommand struct (R17 战略 1-3 6 commands + R30 tool upgrade + R32 langgraph 续)
- 标出每个 struct 的字段 (clap 原生 builder vs derive)

### 阶段 2: 借鉴 clap derive (20 min)
- 读 clap 仓库 `examples/tutorial.rs` + `examples/derive_ref/` 
- 提取 3 pattern:
  1. `#[derive(Parser)]` + `#[command(name, version, about)]`
  2. `#[derive(Subcommand)]` + `#[derive(Args)]`
  3. value_enum / arg_enum (clap 4.x 是 `#[derive(ValueEnum)]`)

### 阶段 3: 重构 (3-4h)
**目标结构** (8-10 个 subcommand → 1 file, derive):
```rust
use clap::{Parser, Subcommand, Args, ValueEnum};

#[derive(Parser)]
#[command(name = "apeireth", version, about = "Apeireth CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// R17 战略 1-3 pipeline commands
    Pipeline(PipelineCmd),
    /// R30 tool upgrade commands
    Tool(ToolCmd),
    /// R32 langgraph commands
    Langgraph(LanggraphCmd),
    /// R125 续 ... (新增)
    ...
}

#[derive(Args)]
pub struct PipelineCmd {
    #[arg(long)]
    pub input: String,
    #[arg(long, value_enum, default_value_t = Format::Json)]
    pub format: Format,
    ...
}

#[derive(ValueEnum, Clone)]
pub enum Format { Json, Yaml, Toml, Text }
```

**重构约束**:
- 0 改 subcommand 名字 (外部 API 兼容)
- 0 改 --flag 长名 (兼容旧调用)
- 0 改 default 值
- 仅用 clap derive (4.x), 0 混用 builder API

### 阶段 4: 跑通 (1h)
```bash
cd .openclaw\workspace\promethean\Apeireth-rust
cargo build -p apeireth-cli
cargo test -p apeireth-cli
cargo run -p apeireth-cli -- pipeline --help  # verify help 文案
cargo run -p apeireth-cli -- pipeline --input test.json  # 真跑 1 个
```

### 阶段 5: final 报告 (30 min)
- `Apeireth-rust/reports/agent-r125-2-final-2026-08-10.md`

---

## 0 主动 commit (C1)

❌ **0 commit, 0 push**. final 报告写完 = done. mavis 整合 #3 17:30 拍板 (0 含 R125, R125 续 8/15-9/10).

---

## final 报告 必含 6 段 (同 R125-1 template)

---

**派活完成 17:28. 截止 8/11 8:00 (跑过夜).**
