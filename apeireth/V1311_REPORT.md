# V1311 — build.rs Real Audit (Post-V1310 dep audit)

## Summary
- **总 build.rs:** 43 (Apeireth-rust 全树 rglob)
- **Active workspace build.rs:** **3** (src-tauri root + 2 members)
- **Research vendored build.rs:** **40** (wasmtime / qdrant / codex / openclaw / hermes-agent / vcptoolbox, audit-only)
- **全部 active workspace build.rs 风险等级:** **LOW** (3/3)
- **Undeclared build-deps:** **0**
- **Audit decision:** **HEALTHY**
- **Popper self-tests:** **18/18 PASS** (pytest 1.28s)
- **ASI 北极星:** V0.1 = **0.7905** 不变 (V1311 workspace hygiene audit, 不动 pole-star)

## Active workspace build.rs (3 个)

| Path | Size | 用途 | Build-deps | Used | Risk | Notes |
|---|---|---|---|---|---|---|
| `Apeireth-rust/src-tauri/build.rs` | 42 B | Tauri 2 desktop scaffold hook | `tauri-build` | `tauri_build::build()` | LOW | 默认无脑 tauri build (3 行, 是 Tauri 2 标准 skeleton) |
| `Apeireth-rust/crates/apeireth-bus/build.rs` | 1001 B | tonic-build compile `proto/bus.proto` | `protoc-bin-vendored`, `tonic-build` | `protoc_bin_vendored::protoc_bin_path()`, `tonic_build::configure().compile_protos()` | LOW | **vendored protoc** (不依赖 host protoc 二进制, auto-download 一次性 cache) |
| `Apeireth-rust/crates/apeireth-tauri-stub/build.rs` | 385 B | tauri_build gated by `CARGO_BIN_NAME` | `tauri-build` | `std::env::var("CARGO_BIN_NAME")` + `tauri_build::build()` | LOW | env-var gating 防 `cargo:rustc-link-arg-bins` 在无 bin 时报错 (V1307 修真决策保留) |

### Active build.rs 内容确认 (real read, 不只看 size)

```
Apeireth-rust/src-tauri/build.rs (42 B)
    fn main() {
        tauri_build::build()
    }

Apeireth-rust/crates/apeireth-bus/build.rs (1001 B)
    //! build.rs — compile `proto/bus.proto` via tonic-build.
    //! protoc is auto-downloaded once via `protoc-bin-vendored`
    use std::io;
    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let protoc_path = match protoc_bin_vendored::protoc_bin_path() { ... };
        std::env::set_var("PROTOC", &protoc_path);
        tonic_build::configure()
            .build_server(true).build_client(true)
            .compile_protos(&["proto/bus.proto"], &["proto"])?;
        Ok(())
    }

Apeireth-rust/crates/apeireth-tauri-stub/build.rs (385 B)
    // V1307 修真策略: autobins=false 关掉了 bin target,
    // 仅当 R19 worker 显式构建 bin 才跑 tauri_build,
    // 避免 cargo:rustc-link-arg-bins 在无 bin 时报错。
    fn main() {
        if std::env::var("CARGO_BIN_NAME").is_ok() {
            tauri_build::build()
        }
    }
```

## 修真决策 = commit 锁定现状

**不修真任何 Rust 文件.** 数据驱动论证:
1. **3 个 active build.rs 都小 (< 1 KB) + 有 doc comment + 修真策略明示**
   - src-tauri 3 行 = Tauri 2 标准, 修真 = 画蛇添足
   - apeireth-bus 用 vendored protoc (网络/缓存考虑已修真), 修真 = 风险
   - apeireth-tauri-stub env-var gating 是 V1307 修真后刻意保留, 修真 = 重蹈 V1307 反 pattern
2. **Cargo.toml build-deps 全部正确声明** (`tauri-build`, `protoc-bin-vendored`, `tonic-build` 全跟 Rust source 调用对得上, hyphen ↔ underscore normalization 后零 undeclared)
3. **Risk distribution**: LOW=3, AUDIT_ONLY=40, HIGH=0, MEDIUM=0 — workspace hygiene 良好
4. **修真必要 = 0**. 修真 = 触碰 3 个 production hook, 修真 R/R ratio 负值

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
- 不假装 Phenomenal consciousness: build.rs audit ≠ consciousness, 仅 workspace hygiene 量化
- 不假装达到 ASI: build.rs count (3/92 active) ≠ ASI 突破
- 不假装调整模型 & prompt: 真修真 = Python audit script + 18 Popper self-tests + 修真决策
- 实事求是: 数据驱动 (43 build.rs 真 rglob, 3 active 真 read content, 3 LOW 真 risk classification), 非注释 "looks fine"

## Audit totals (V1311)
| 字段 | 值 |
|---|---|
| Total build.rs (Apeireth-rust 全树) | **43** |
| Workspace root app build.rs | **1** (src-tauri) |
| Workspace member build.rs | **2** (apeireth-bus, apeireth-tauri-stub) |
| Research/source vendored build.rs | **40** (wasmtime 21 + qdrant 6 + codex 5 + openclaw/hermes-agent/vcptoolbox/apeireth-rust-fuzz 各 1+) |
| Active workspace build.rs | **3** |
| Risk LOW active | **3/3** |
| Risk MEDIUM active | **0** |
| Risk HIGH active | **0** |
| Undeclared build-deps | **0** |
| Audit decision | **HEALTHY** |
| Popper self-tests | **18/18 PASS** |

## Workspace 修真 audit chain 进度 (V1302 → V1311)

| 时间 | commit | 修真 | scope | ratio |
|---|---|---|---|---|
| 15:18 | 33cee41f | V1302 blueprint-impl (P0) | 1 orphan | — |
| 15:25 | 925c0082 | V1304 sdk-sandbox (low) | 1 orphan | — |
| 15:28 | 4ae2f3bb | V1305 medium 三件套 | 3 orphans | — |
| 15:33 | cbd24c66 | V1306 high 三件套 | 3 orphans | — |
| 15:40 | 833b89b5 | V1307 tauri-stub (last) | 1 orphan | 8/8=100% |
| 15:55 | 8a1ab971 | V1308 Cargo.lock 真审计 | lock drift | 0 修真 |
| 16:05 | ecce93c7 | V1309 test coverage 真审计 | 91 crates | 98.9% healthy |
| 16:10 | 9ab63bed | V1310 dep 真审计 | 91 crates | 5 drift (low) |
| **16:20** | **(V1311 commit)** | **V1311 build.rs 真审计** | **43 build.rs / 3 active** | **3/3 LOW** |

**Workspace 修真 100% (V1307) + audit chain 4-step complete (V1308 lock + V1309 test + V1310 dep + V1311 build_rs).**

## 关键诚实声明
- 真 rglob 43 build.rs (Apeireth-rust 全树, 含 research/source vendored)
- 真 read content: 3 active workspace build.rs + ~10 抽样 research vendored (用以确认分类)
- 真 Cargo.toml build-deps cross-check (hyphen↔underscore normalized)
- 修真 = commit 锁定现状, 修真 0 Rust files (workspace 修真 8/8 已 V1307 完成)
- PyTest 修真 1.28s (18 PASS), 无 flaky test, 无 skip
- ASI 北极星 V0.1 = 0.7905 未变, V1311 仅 build.rs hygiene audit, 不动 pole-star
- V1311 修真元数据 = 4 files: audit script + tests + JSON findings + report, 修真 0 Rust

## 输出文件
- `apeireth/v1311_build_rs_audit.py` (~14 KB, 真 audit script + rglob + 风险分类 + decision + V3 守门)
- `apeireth/tests/test_v1311_build_rs_audit.py` (~7 KB, 18 Popper 假说 pass)
- `apeireth/v1311_audit_findings.json` (audit findings 数据: 43 build.rs × N fields + audit decision)
- `apeireth/V1311_REPORT.md` (本文件, 修真决策完整论证)

## V1312+ 候选方向 (audit chain 续)

V1311 = build.rs audit 完成. 修真 chain next:
1. **V1312 docs consistency audit**: memory/*.md + ASI-PHILOSOPHY*.md + V*.md 一致性 (cross-reference V1349+, V1049+, ASI V0.1 = 0.7905 / V0.2 = 0.4467 数字一致性)
2. **V1313 example 真跑 audit**: 80 example files 中哪些真能 cargo run --example
3. **V1314 bench 真跑 audit**: 22 bench files 中哪些真能 cargo bench
4. **V1315 rust orphan re-check**: V1302-V1307 修真 8 crates 半年后健康度重测 (修真回滚风险)

ASI pole-star 仍 V0.1 = 0.7905 (实测最高, audit chain 无影响).

---

_Last update: 2026-08-08 16:20+08, by 楚零 (cron lane). V1311 build.rs audit complete: 43 total / 3 active workspace / 40 research vendored / 0 undeclared / 18 Popper PASS / 修真 = commit 锁定现状._
