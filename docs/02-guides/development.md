# Apeireth Development Guide

> 对齐实际工作流（2026-08-19 post-1.0.0）。给想参与开发的人：构建/测试/代码地图/纪律。

## 构建与测试

```bash
cargo build --workspace              # 85 + 1 desktop crates 全量构建
cargo check --workspace --all-targets  # 编译全 target（含 examples/bins/tests）— 必跑
cargo test --workspace               # 23,874 组 0 失败（含 post-1.0.0 增量）
cargo test -p apeireth-cron --test integration_cron  # cron 25 case integration tests
cargo test -p apeireth-companion --lib  # 伙伴器官 644 测试（最快的核心反馈环）
cargo fmt --all --check              # 格式
```

## 前端开发 (companion-desktop, post-1.0.0 新增)

`frontend/companion-desktop/` 是**独立 [workspace]** (Svelte 5 + Tauri 2), 不在 root cargo workspace.
其 CI 守门在 `.github/workflows/companion-desktop-ci.yml` 单独跑 (cargo check Tauri shell + pnpm svelte-check + 8 硬墙).

```bash
# 前置: Node 20+ + pnpm 9+ (Windows: WebView2 runtime)
cd frontend/companion-desktop
pnpm install
pnpm dev                            # Vite + Svelte (http://localhost:1420)
pnpm check                          # svelte-check (类型 + 语法)
```

> 真实 LLM 流式 (CoT + tool_call + tool_result SSE) 仍 deferred (TP34 v1.5 中期). 当前 `stream: false` 写死,
> 前端 6 种 RuntimeEvent 中部分不可触发, mock SSE e2e 跑通. 详见 `docs/04-internal/next-team-handbook.md` TP34.

**注意**：`cargo test --workspace` 不编译 examples——改公共结构后必须 `--all-targets`。

## 代码地图（从哪开始读）

### 伙伴器官（apeireth-companion，~25K 行）

| 读序 | 文件 | 内容 |
|---|---|---|
| 1 | `src/assemble.rs` | CompanionApp 装配器——所有机制怎么接起来 |
| 2 | `src/context.rs` | 注入管线（L0/L1 常驻 + 预算截断）|
| 3 | `src/memory_extractor.rs` | 记忆 v2 核心（importance/对账/排名）|
| 4 | `src/memory_graph.rs` | 双时态事实图 + crawl |
| 5 | `src/world_model.rs` → `src/causal_world_model.rs` | 世界模型 W1 → W2/W3 |
| 6 | `src/curiosity.rs` → `src/hypothesis.rs` → `src/emotion_memory.rs` → `src/value_cases.rs` | 她本身（E4/F4/F1/F6）|
| 7 | `src/emergence.rs` | 开口策略（E7）|
| 8 | `src/oracle.rs` + `src/intent_brier.rs` | 校准 + 自我诊断 |

### 工具链

| 读序 | crate | 内容 |
|---|---|---|
| 1 | `apeireth-tool-runtime` | parser/executor/record |
| 2 | `apeireth-tool-approval` | 5 规则审批 |
| 3 | `apeireth-tools` | schema/guardrail/yaml_spec |

### 安全

| crate | 内容 |
|---|---|
| `apeireth-http-client::egress` | 出站默认拒绝 + 审计链 |
| `apeireth-guard` | PII 脱敏 |
| `apeireth-companion::job_object` | Windows Job Object 沙箱 |

### CI 守门 (post-1.0.0 加固)

| workflow | 守门内容 |
|---|---|
| `rust.yml` | cargo nextest (3 OS matrix) + 8 硬墙 job (LOCKED / version / R11 baseline / 13 键 / V1136) |
| `companion-desktop-ci.yml` | cargo check (Tauri shell) + pnpm svelte-check + 8 硬墙 |
| `pii-leak-detection.yml` | 8 关键词 grep (防前轮 11 轮 filter-repo 清洗回潮) |
| `release-1.0.0.yml` | 8 包齐发矩阵 (deb/rpm/brew/scoop/tarball/msi/docker×2) + 5/5 gate |

详见 `docs/04-internal/ci-fix-log-2026-08.md` 历史 + `docs/04-internal/next-team-handbook.md` 排期.

## 机制设计模式（本项目特色）

1. **trait 策略注入**：lib 零 LLM 依赖——`MemoryExtractor`/`DreamSummarizer`/`ReflectionReflector`/`ConstitutionLlm` 等全是 trait，测试用 Mock 实现，生产注入真 LLM。新机制照此模式。
2. **确定性机制件**：curiosity/hypothesis/emotion_memory/value_cases 全是确定性无 LLM——可单测、可复现（固定种子 LCG）。LLM 行为是下游消费方的事（0 装标注）。
3. **集成而非分立**：新需求挂既有机制（oracle/memory/bus/approval 链），不造平行系统。
4. **0 装 PASS**：未实现标 `trait 口已备未接`；无环境标"待实测"；真实 API 测试带限流退避。

## 常见陷阱（前人踩过）

| 陷阱 | 规则 |
|---|---|
| std Mutex 不可重入 | 持 guard 期间禁止调用会再取同一把锁的方法（migrate_subject 死锁教训）|
| Windows cmd 嵌套引号 | 子进程测试直接 spawn，不经 `cmd /c`（powershell 脚本会被解析坏）|
| Job Object 内存限制语义 | 超限 = 拒绝分配（OOM），不是杀进程（与 CPU 时间限制不同）|
| HashMap 迭代序 | 确定性测试要求排序后再比较（curiosity 采样教训）|
| 并行测试静态原子 | 共享状态必须共享锁（TUI hand 竞态教训）|
| 真实 API 压测 | 必须带退避，否则限流自造失败 |

## 提交规范

- 改码必改对应 README/docs（文档同步自觉）
- 改公共结构（enum/struct/签名）→ grep 所有构造点 + all-targets
- 验收标准：全量测试绿 + all-targets 干净 + 文档同步
- 分支：开发分支 → 全量验证 → 合入 integration → 发布时同步 master
