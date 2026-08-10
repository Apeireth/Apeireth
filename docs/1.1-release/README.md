# Apeireth-rust 1.1 Release Index (R38, 2026-08-09)

> **性质**: workspace 1.1.0 主轴 release 章节索引
> **来源**: R38 9 B-stage 一气呵成, commit range `1f23b28f` → `a64fe197`
> **配套**: `APEIRETH-VERSIONING.md` + `CHANGELOG.md` v1.1.0 entry + `reports/r38-batch-1.1-rc-2026-08-09.md`

---

## 1.1.0 主报告

- `reports/r38-batch-1.1-rc-2026-08-09.md` — R38 主报告 (B1-B9 + 1.1 真合并 verdict)
- `reports/r40-r45-1.2-source-sync-2026-08-09.md` — R40-R45 1.2 follow-up (同步)
- `reports/r35-batch-final-2026-08-09.md` — R35 1.1 真合并前奏

## 9 B-stage 索引

| Stage | 主题 | Commit | 报告段落 |
|---|---|---|---|
| **B1** | telemetry 4→1 1.1 真合并 | `1f23b28f` | reports/b1-telemetry-merge-2026-08-09.md |
| **B2** | pipeline tool_loop 实战 (web+tauri-stub) | `dc00f6d7` | reports/r38 § B2 |
| **B5** | eval-live.yml + rust-ci 加 ci-summary | `cb2f2ab4` + R51 patch | reports/r38 § B5 |
| **B6** | memory mini-redis RESP mock (R46) | TBD | reports/r46-r53-... md |
| **B7** | oauth device_code HTTP polling RFC 8628 wiremock | `9becfbf0` | reports/r40-r45 § B7 |
| **B8** | cognition_graph 24 dim + 1 asi + 1 decide = 26 节点 | `990c0d5e` | reports/r38 § B8 |
| **B9** | workspace 1.0 → 1.1 | `a64fe197` | reports/r38 § B9 |
| **R36** | 5 老 provider 真删 (89→84 members) | (R36 batch) | reports/r35 § R36 |
| **R37** | Protocol bridge + organ partial merge | (R37 batch) | reports/r35 § R37 |

## API 引用 (1.1 适配)

- `docs/api/` — 9 organ + 6 provider + 6 tool HTTP 端点
- `crates/apeireth-api/src/` — 实现
- `docs/api/v1-websocket.md` — 流式 LLM
- `crates/apeireth-oauth/src/transport.rs` — RFC 8628 device_code HTTP polling (B7)

## 哲学锚 + 锁定 (1.1 0 触锚定)

- **S-1 北极星**: 24 维 + 9 organ + 8 LOCKED — 0 触碰
- **S-2 实事求是**: R44 wiremock 真接 RFC 8628 §3.1/§3.5
- **O-2 走在前人尖上**: R35/R36 借 LangGraph/AutoGen/MCP; R44 借 RFC 8628 + Golutra #2
- **6 LOCKED + 24 LOCKED crate + 8 项承诺 + R11 baseline + TUI 9 organ UI** — 100% 0 触

## 版本系统 (R42 一次性落档)

`APEIRETH-VERSIONING.md` 7 子系统全 R38 同步:
主代码 1.1.0-R38 / 设计 Design-5.0-R38 / 修正链 Fix-3..Fix-12-R38 / R 周期 R38 /
指标 V0.5-24d-R38 + V1136-R38 + V1331-R38 / 基线 snap-a64fe197 + snap-1f23b28f / 手册 Manual-Rev-H.

---

## 1.2.0 follow-up (R40-R45 已落 master)

| Stage | 主题 | Commit | 报告 |
|---|---|---|---|
| **R40** | README Document-Meta + Version badge 1.1 | `133bfb8b` | reports/r40-r45 § R40 |
| **R41** | src-tauri/Cargo.toml 1.1.0 | `133bfb8b` | reports/r40-r45 § R41 |
| **R42** | APEIRETH-VERSIONING.md 7 子系统 R38 | `133bfb8b` | reports/r40-r45 § R42 |
| **R43** | test 升级 1.1.0 (apeireth-core + integration-e2e) | `133bfb8b` | reports/r40-r45 § R43 |
| **R44** | oauth device_code HTTP polling wiremock 4 步真接 | `9becfbf0` | reports/r40-r45 § R44 |
| **R45** | reports/r40-r45 报告 + final 验证 | `5f58f798` | reports/r40-r45 full |


---

## 1.1.2 patch (R54 续升级, 2026-08-09)

> 1.1.1 之后的 patch, 主轴: backend wire-up + cognition_graph 数据流闭环到 TUI memory organ。

| Stage | 主题 | Commit | 报告 |
|---|---|---|---|
| **R54-a** | apeireth-graph 加 apeireth-tui Cargo.toml deps | 后续提交 | reports/r54-batch-1.1.2-patch-2026-08-09.md § R54-a |
| **R54-b** | backend.rs::compute_main_ai_status 中段接 3 个新调用: record_mid_term_count (last 24h 真接 SQLite) + record_long_term_count (近似 total/5) + record_cognition_summary (run_cognition_graph_sync 真跑 26 节点) | 同上 | reports/r54 § R54-b |
| **R54-c** | render() 0 假装小修 (mid/long 标 stub -> 真接 (real query) / 近似; readiness 1/3 真接 -> 2/3 真接 + long_term 近似标注) + tests 守门 | 同上 | reports/r54 § R54-c |
| **R55** | APEIRETH-VERSIONING.md 7 子系统 R54 同步: 主代码 1.1.0-R38 -> 1.1.2-R54; Fix-3..Fix-12 -> Fix-3..Fix-13-R54 (Fix-13-R54 主题); R 周期 R38 -> R54 (+ R46-R53 归档); 手册 Manual-Rev-H -> Manual-Rev-I | 同上 | reports/r54 § R55 |
| **R56** | CHANGELOG.md R54 1.1.2 patch entry + docs/1.1-release/README.md 本节 | 同上 | reports/r54 § R56 |

## 哲学锚 (R54 续 100% 穿透)

- **S-1 北极星**: 24 LOCKED + 9 organ + 8 LOCKED + 1.1 workspace — 0 触 (R54 续 backend + memory.rs 仅 touched render labels, 0 触 LOCKED)
- **S-2 实事求是**: render() 0 假装 (stub 标删除); mid_term 真接 (last 24h SQLite query); long_term 近似 (total/5 标注 vector store 未上)
- **O-2 走在前人尖上**: 借 VCP / LangGraph cognition_graph (R38 B8); 借 EpisodeQuery::in_range 真接 mid_term
- **O-3 干到底**: R54 + R55 + R56 一 commit 一气呵成
- **O-4 任何人都能接手**: 本 README + APEIRETH-VERSIONING.md + CHANGELOG + reports/r54-batch-1.1.2-patch-2026-08-09.md (NEW)
- **O-5 不假装**: render 3 行 0 假装小修; long_term "近似" 明确标注; cognition_summary 测试守门

## 后续 follow-up (R54 续 不在)

- vector store 真接 long_term (当前 total/5 heuristic, 长期 vector store 是 1.3 路线)
- TUI 9 organ memory page 加 cognition summary 显示行 (需用户放行 UI 改; current render 已显示 2/3 真接)
- backend 真接 cognition_summary 频率: snapshot_organ_main only vs per-chat-cycle (per-chat-cycle 更精细, 但 block_on cost)
