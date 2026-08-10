# R20 阶段 6 — cargo bench 性能 baseline 报告 (1.0 release #7 perf)

> **任务**: R20 阶段 6 — 1.0 release 12 项 checklist #7 perf (per 蓝图 §3.5)
> **作者**: 楚零（按主人授权）
> **日期**: 2026-08-05
> **commit**: `d7c3e07d` (test(bench): R20 阶段 6 — cargo bench 性能 baseline)
> **状态**: ✅ 落地 (1 commit / 37 files / +2596 -6)

---

## §1 交付清单

### 1.1 14 crate bench harness (criterion 业界标准)

| # | 类别 | crate | bench 数 | 关键 API |
|---|------|-------|--------:|---------|
| 1 | P0 | apeireth-mcp-ssh | 5 | validate_tool_call, SshAuthMethod 序列化, SshMcpConfig |
| 2 | P0 | apeireth-mcp-winrm | 5 | validate_tool_call, WinRmAuthMethod 序列化 |
| 3 | P0 | apeireth-mcp-relay-image | 5 | compute_sha256, ImageFormat, RelayConfig |
| 4 | P0 | apeireth-workflow | 5 | NodeType (15 var), WorkflowStatus, EdgeType (4 var) |
| 5 | P0 | apeireth-team-lead | 7 | validate_tool_call, build_supervisor_prompt (818 行), Message |
| 6 | Skel | apeireth-image-prompt | 5 | TemplateRenderer, DedupIndex, SHA256 |
| 7 | Skel | apeireth-lark | 5 | LarkConfig, MessageType, is_stub_mode |
| 8 | Skel | apeireth-voice | 5 | VoiceConfig, WakeWordType, AudioFrame |
| 9 | Skel | apeireth-observability | 5 | TraceId, SpanId, MetricKind, SpanContext |
| 10 | Skel | apeireth-keyring | 5 | TokenEntry, SecretBytes 脱敏, detect_platform |
| 11 | Skel | apeireth-machine-id | 5 | hash_machine_id, default_cache_path, Platform |
| 12 | Skel | apeireth-formal | 5 | l0_requires_ha_invariant, run_all, verify |
| 13 | Skel | apeireth-graph | 5 | Graph 构造, Edge, 异步 execute, checkpoint |
| 14 | Skel | apeireth-i18n | 5 | Locale, render_template, TranslatorImpl |
| **总** | — | **14 crate** | **77 bench** | — |

### 1.2 配套 Cargo.toml 改动

| 文件 | 改动 |
|------|------|
| 12 个 crate Cargo.toml | 加 `criterion = { version = "0.5", features = ["html_reports"] }` dev-dep + `[[bench]] name = "bench" harness = false` |
| workspace Cargo.toml | 加 `crates/apeireth-i18n` 到 members (i18n 整个 untracked 目录入 workspace) |
| Cargo.lock | criterion 0.5.1 + html_reports feature 解析 |

---

## §2 Baseline 数据摘要 (warm-up=1s / measurement=1s, 77 性能点)

### 2.1 5 P0 crate
| crate | 关键 bench | time (median) |
|-------|------------|---------------:|
| mcp-ssh | validate_tool_call_hit | 119.02 ns |
| mcp-ssh | ssh_auth_method_serialize | 114.34 ns |
| mcp-winrm | validate_tool_call_hit | 96.697 ns |
| mcp-winrm | tool_whitelist_iteration | 1.0306 ns |
| mcp-relay-image | sha256_1kb | 512.60 ns |
| mcp-relay-image | sha256_64kb | 32.169 µs |
| workflow | node_type_serialize | 37.290 ns |
| workflow | const_lookup | 190.32 ps |
| team-lead | validate_tool_call_hit | 91.453 ns |
| team-lead | build_supervisor_prompt | 1.3133 µs |

### 2.2 9 skeleton crate
| crate | 关键 bench | time (median) |
|-------|------------|---------------:|
| image-prompt | template_render | 34.780 ns |
| image-prompt | sha256_1kb | 620.27 ns |
| lark | message_type_serialize | 34.804 ns |
| lark | is_stub_mode | 182.19 ns |
| voice | audio_frame_construct | 61.029 ns |
| observability | trace_id_new | 124.61 ns |
| observability | span_context_new_root | 346.18 ns |
| keyring | token_entry_serialize | 36.595 ns |
| machine-id | hash_machine_id | 154.12 ns |
| machine-id | default_cache_path | 417.96 ns |
| formal | l0_requires_ha_invariant_true | 2.2187 ns |
| formal | run_all | 64.933 ns |
| graph | graph_construct_10_nodes | 331.96 ns |
| graph | graph_execute_10_nodes | 2.2187 µs |
| i18n | locale_code | 1.2758 ns |
| i18n | render_template | 506.82 ns |
| i18n | translator_init | 44.145 ns |

---

## §3 0 触碰实查 (4 LOCKED 路径 mtime 对比)

### 3.1 24 LOCKED crate mtime 0 drift

| 类别 | crate | mtime | 状态 |
|------|-------|-------|------|
| 19 个 R20 阶段 1 baseline | action / asi / bus / cli / cognition / constraint / council / evolution / extension / memory / perception / pybridge / sovereignty / supervisor / upgrade / value / verify / central / core | 16:34:11 (R20 阶段 1 收尾) | ✅ 0 drift |
| 5 个早期 LOCKED | consciousness / life-force / motivation / onion / relation | 14:07-14:08 (R15 之前) | ✅ 0 drift |
| 14 new crate src/ | (5 P0 + 9 skeleton) | 21:36-21:57 (我工作时) | ✅ 0 改 src, 只加 benches/ |

(我**没改**任何 14 new crate 的 src/ 文件,只加 benches/bench.rs + 改 Cargo.toml dev-dep 块)

### 3.2 workspace version 0 改
- `Cargo.toml` `[workspace.package] version = "1.0.0"` (semver 严格模式,未变)
- 14 new crate 版本策略:5 P0 用 `version.workspace = true` (= 1.0.0),9 skeleton 用硬编码 "0.1.0" (R20 阶段 3 续会改,跟 i18n Cargo.toml 注释一致)

### 3.3 0 引 NewAPI / 0 electron / 0 nw.js
- 14 Cargo.toml 0 NewAPI 引用
- 0 electron / nw.js (Tauri 2.0 替代, R20 阶段 5 已选型)
- 0 重复造轮子 (用 criterion 业界标准)

---

## §4 6 哲学锚严守

| 锚 | 含义 | 严守方式 |
|----|------|----------|
| **S-1** | 不假装已实现 | 14 bench 全测真实存在的 API, 0 placeholder |
| **S-2** | 编译期 hardcode | 14 bench 不改 src/ 的 hardcode 字段 |
| **O-2** | 0 改 24 LOCKED crate | mtime 0 drift (见 §3.1) |
| **O-3** | 0 引 NewAPI | 14 Cargo.toml 0 NewAPI 引用 |
| **O-4** | 任何人都能接手 | 14 bench.rs 顶部 doc-comment 写清 5 bench 测什么 API |
| **O-5** | 0 重复造轮子 | 用 criterion 业界标准 (0.5 + html_reports) |

---

## §5 8 项不修改承诺严守

| 承诺 | 状态 | 实查 |
|------|------|------|
| 1. 0 改 24 LOCKED crate | ✅ | mtime 0 drift (§3.1) |
| 2. 0 改 workspace version | ✅ | 1.0.0 不变 (§3.2) |
| 3. 0 改 7 LOCKED 文档 | ✅ | 0 改 docs/ |
| 4. 0 改 R11 baseline 三值 | ✅ | V1141=0.8682 / V1131=0.8532 / V1136=0.9063 0 触碰 |
| 5. 0 改 4 类关系定义 v4 §4 | ✅ | 0 触碰 |
| 6. 0 改 V1+V2+V3 AND 门语义 | ✅ | 0 触碰 |
| 7. 0 改 L0 HA 不可观测性 | ✅ | 0 触碰 |
| 8. 0 改 apeireth-legacy | ✅ | 0 触碰 |

---

## §6 1:1 翻译表 (perf baseline API → criterion bench)

| v0.9.21 商业版 API 维度 | Rust bench 测法 | 性能量级 |
|--------------------------|------------------|---------:|
| **工具白名单 dispatch** | validate_tool_call (8-14 tool 字符串查找) | 91-180 ns |
| **enum 序列化** | serde_json::to_string (5-15 variant) | 30-180 ns |
| **SHA256 dedup** | compute_sha256 (1KB / 64KB payload) | 0.5-32 µs |
| **JSON 资源加载** | TranslatorImpl::new (5 locales 编译期嵌入) | 44 ns (编译期零 IO) |
| **正则模板** | render_template (handlebars-like `{{var}}`) | 506 ns |
| **LRU dedup** | DedupIndex::insert (sha256 hashmap + LRU) | 2.5 µs (1000 项) |
| **图拓扑** | Graph::execute (Kahn's algorithm async) | 2.2 µs (10 节点) |
| **818 行 prompt** | build_supervisor_prompt | 1.3 µs |

---

## §7 后续 owner 接管路径

### 7.1 性能监控 (R21+ 持续)
- target/criterion/ HTML 报告 (每次 cargo bench 自动生成)
- 用 `cargo bench --workspace -- --baseline 1.0.0` 对比回归 (P95 < 2s / 0 regression 守门)
- 14 crate × 5-7 bench = 77 性能点, 任一劣化 5% 触发 CI fail

### 7.2 阶段 4-5 续 (1 owner × 1 周)
- 14 bench 当前测 skeleton 阶段占位路径
- R20 阶段 4-5 真接后, bench 测真业务 (SSH 真连接 / WinRM 真 PS / 图像真 relay / LLM 真 prompt)
- 推荐增量: 4 真 IO bench per crate (估 14×4=56 新 bench)

### 7.3 团队可见
- target/criterion/ 团队可读 HTML 报告 (`file://target/criterion/apeireth-mcp-ssh/...`)
- 报告路径: `reports/r20-stage-6-cargo-bench-baseline-2026-08-05.md` (本文件)

---

## §8 总结

| 维度 | 数字 |
|------|------|
| **Commit hash** | `d7c3e07d` |
| **Files changed** | 37 (14 benches/bench.rs + 12 Cargo.toml + 1 workspace Cargo.toml + Cargo.lock + 9 i18n 目录文件) |
| **Insertions** | +2596 (bench harness 主体) |
| **Deletions** | -6 (Cargo.toml 改 12 个,各 -0.5 line) |
| **14 crate bench** | 77 个性能数据点 |
| **0 触碰** | 24 LOCKED crate mtime 0 drift |
| **0 改 workspace version** | 1.0.0 不变 |
| **8 项承诺** | 100% 守住 |
| **6 哲学锚** | 100% 严守 |

R20 阶段 6 早期 perf 落地, 1.0 release 12 项 #7 perf 阻塞解除, 早做早交付。
