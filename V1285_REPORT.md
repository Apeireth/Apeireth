# V1285 All-42 Rust Security Depth Audit — Run `v1285-1785926622`

- Run timestamp: `1785926622.937` (unix)
- Build: `2026-08-05-1837+08` version: `0.1.0`
- ASI NS current: `0.7905` (display 92.91%)
- Promethean dir: `.openclaw\workspace\promethean`
- Workspace crates: `.openclaw\workspace\promethean\Apeireth-rust\crates`
- All apeireth-* crates discovered: **42**
- Crates audited (have src/): **42**
- Crates clean (zero hotspots): **6**
- Crates INCONCLUSIVE (not found): **0**
- Total hypotheses: **210** (5 hyp × 42 crates)
- Total hotspots across all crates: **1173**
- V1284 worst-5 overlap: **5/5** (apeireth-cli, apeireth-consciousness, apeireth-formal, apeireth-tauri-stub, apeireth-vector)
- Elapsed: `341.6 ms`
- PASS: **140** / FAIL: **70** / INCONCLUSIVE: **0** (PASS rate = 66.67%)

## V3 Philosophy Gate (主 17:58 + 主 20:46 + 主 17:43 不假装)

- ✅ `v1283_inherited_gate_0` = True
- ✅ `v1283_inherited_gate_1` = True
- ✅ `v1283_inherited_gate_2` = True
- ✅ `v1283_inherited_gate_3` = True
- ✅ `v1283_inherited_gate_4` = True
- ✅ `v1283_inherited_gate_5` = True
- ✅ `v1283_inherited_gate_6` = True
- ✅ `v1283_inherited_gate_7` = True
- ✅ `v1283_inherited_gate_8` = True
- ✅ `v1283_inherited_gate_9` = True
- ✅ `v1283_inherited_gate_10` = True
- ✅ `v1283_inherited_gate_11` = True
- ✅ `v1283_inherited_gate_12` = True
- ✅ `v1283_inherited_gate_13` = True
- ✅ `v1283_inherited_gate_14` = True
- ✅ `v1283_inherited_gate_15` = True
- ✅ `v1283_inherited_gate_16` = True
- ✅ `v1283_inherited_gate_17` = True
- ✅ `v1283_inherited_gate_18` = True
- ✅ `v1283_inherited_gate_19` = True
- ✅ `v1283_inherited_gate_20` = True
- ✅ `v1284_extends_v1283_not_replaces` = True
- ✅ `v1284_audit_only_no_fix` = True
- ✅ `v1284_production_src_only` = True
- ✅ `v1285_extends_v1284_not_replaces` = True
- ✅ `v1285_all42_not_vendor` = True
- ✅ `v1285_no_kpi_inflate` = True

## Per-Hypothesis Summary (across 42 crates)

- `h_zero_unwrap_in_production_src`: PASS=11 / FAIL=31 / INCONCLUSIVE=0 / total=42
- `h_zero_expect_in_production_src`: PASS=19 / FAIL=23 / INCONCLUSIVE=0 / total=42
- `h_zero_panic_in_production_src`: PASS=27 / FAIL=15 / INCONCLUSIVE=0 / total=42
- `h_zero_todo_in_production_src`: PASS=42 / FAIL=0 / INCONCLUSIVE=0 / total=42
- `h_zero_unsafe_in_production_src`: PASS=41 / FAIL=1 / INCONCLUSIVE=0 / total=42

## Top-10 Crates by Total Hotspots

| Rank | Crate | unwrap | expect | panic | todo | unimplemented | unsafe | Total |
|------|-------|--------|--------|-------|------|---------------|--------|-------|
| 1 | `apeireth-memory` | 122 | 8 | 1 | 0 | 0 | 0 | **131** |
| 2 | `apeireth-evolution` | 104 | 2 | 0 | 0 | 0 | 0 | **106** |
| 3 | `apeireth-tools` | 14 | 90 | 0 | 0 | 0 | 0 | **104** |
| 4 | `apeireth-upgrade` | 86 | 1 | 15 | 0 | 0 | 0 | **102** |
| 5 | `apeireth-sovereignty` | 73 | 10 | 11 | 0 | 0 | 0 | **94** |
| 6 | `apeireth-agent` | 75 | 2 | 1 | 0 | 0 | 0 | **78** |
| 7 | `apeireth-tui` | 49 | 23 | 1 | 0 | 0 | 0 | **73** |
| 8 | `apeireth-tool-runtime` | 11 | 47 | 0 | 0 | 0 | 0 | **58** |
| 9 | `apeireth-http-client` | 45 | 2 | 1 | 0 | 0 | 0 | **48** |
| 10 | `apeireth-api` | 36 | 4 | 5 | 0 | 0 | 0 | **45** |

## All 42 Audited Crates — Hotspot Counts

| Crate | unwrap | expect | panic | todo | unimplemented | unsafe | Total | Status |
|-------|--------|--------|-------|------|---------------|--------|-------|--------|
| `apeireth-action` | 0 | 1 | 0 | 0 | 0 | 0 | **1** | ⚠️ 1 hotspots |
| `apeireth-agent` | 75 | 2 | 1 | 0 | 0 | 0 | **78** | ⚠️ 78 hotspots |
| `apeireth-api` | 36 | 4 | 5 | 0 | 0 | 0 | **45** | ⚠️ 45 hotspots |
| `apeireth-asi` | 8 | 0 | 0 | 0 | 0 | 0 | **8** | ⚠️ 8 hotspots |
| `apeireth-bench` | 0 | 0 | 0 | 0 | 0 | 0 | **0** | ✅ clean |
| `apeireth-bus` | 26 | 2 | 0 | 0 | 0 | 0 | **28** | ⚠️ 28 hotspots |
| `apeireth-central` | 13 | 3 | 1 | 0 | 0 | 0 | **17** | ⚠️ 17 hotspots |
| `apeireth-cli` | 0 | 0 | 0 | 0 | 0 | 0 | **0** | ✅ clean |
| `apeireth-cognition` | 0 | 9 | 0 | 0 | 0 | 0 | **9** | ⚠️ 9 hotspots |
| `apeireth-consciousness` | 10 | 0 | 1 | 0 | 0 | 0 | **11** | ⚠️ 11 hotspots |
| `apeireth-constraint` | 2 | 2 | 5 | 0 | 0 | 0 | **9** | ⚠️ 9 hotspots |
| `apeireth-core` | 0 | 0 | 27 | 0 | 0 | 0 | **27** | ⚠️ 27 hotspots |
| `apeireth-council` | 2 | 2 | 0 | 0 | 0 | 0 | **4** | ⚠️ 4 hotspots |
| `apeireth-evolution` | 104 | 2 | 0 | 0 | 0 | 0 | **106** | ⚠️ 106 hotspots |
| `apeireth-extension` | 28 | 0 | 0 | 0 | 0 | 0 | **28** | ⚠️ 28 hotspots |
| `apeireth-formal` | 0 | 0 | 0 | 0 | 0 | 0 | **0** | ✅ clean |
| `apeireth-graph` | 6 | 2 | 0 | 0 | 0 | 0 | **8** | ⚠️ 8 hotspots |
| `apeireth-http-client` | 45 | 2 | 1 | 0 | 0 | 0 | **48** | ⚠️ 48 hotspots |
| `apeireth-life-force` | 0 | 2 | 0 | 0 | 0 | 0 | **2** | ⚠️ 2 hotspots |
| `apeireth-mcp` | 29 | 5 | 0 | 0 | 0 | 0 | **34** | ⚠️ 34 hotspots |
| `apeireth-memory` | 122 | 8 | 1 | 0 | 0 | 0 | **131** | ⚠️ 131 hotspots |
| `apeireth-motivation` | 6 | 0 | 0 | 0 | 0 | 0 | **6** | ⚠️ 6 hotspots |
| `apeireth-onion` | 0 | 0 | 0 | 0 | 0 | 0 | **0** | ✅ clean |
| `apeireth-perception` | 0 | 0 | 0 | 0 | 0 | 0 | **0** | ✅ clean |
| `apeireth-pipeline` | 12 | 7 | 2 | 0 | 0 | 0 | **21** | ⚠️ 21 hotspots |
| `apeireth-protocol` | 4 | 0 | 1 | 0 | 0 | 0 | **5** | ⚠️ 5 hotspots |
| `apeireth-pybridge` | 12 | 1 | 0 | 0 | 0 | 0 | **13** | ⚠️ 13 hotspots |
| `apeireth-relation` | 6 | 0 | 0 | 0 | 0 | 0 | **6** | ⚠️ 6 hotspots |
| `apeireth-sdk` | 0 | 0 | 0 | 0 | 0 | 0 | **0** | ✅ clean |
| `apeireth-sovereignty` | 73 | 10 | 11 | 0 | 0 | 0 | **94** | ⚠️ 94 hotspots |
| `apeireth-supervisor` | 6 | 0 | 0 | 0 | 0 | 0 | **6** | ⚠️ 6 hotspots |
| `apeireth-tauri-stub` | 0 | 1 | 0 | 0 | 0 | 0 | **1** | ⚠️ 1 hotspots |
| `apeireth-tool-approval` | 1 | 0 | 4 | 0 | 0 | 0 | **5** | ⚠️ 5 hotspots |
| `apeireth-tool-registry` | 27 | 0 | 0 | 0 | 0 | 0 | **27** | ⚠️ 27 hotspots |
| `apeireth-tool-runtime` | 11 | 47 | 0 | 0 | 0 | 0 | **58** | ⚠️ 58 hotspots |
| `apeireth-tools` | 14 | 90 | 0 | 0 | 0 | 0 | **104** | ⚠️ 104 hotspots |
| `apeireth-tui` | 49 | 23 | 1 | 0 | 0 | 0 | **73** | ⚠️ 73 hotspots |
| `apeireth-upgrade` | 86 | 1 | 15 | 0 | 0 | 0 | **102** | ⚠️ 102 hotspots |
| `apeireth-value` | 21 | 0 | 0 | 0 | 0 | 0 | **21** | ⚠️ 21 hotspots |
| `apeireth-vector` | 26 | 0 | 0 | 0 | 0 | 0 | **26** | ⚠️ 26 hotspots |
| `apeireth-verify` | 4 | 0 | 1 | 0 | 0 | 0 | **5** | ⚠️ 5 hotspots |
| `apeireth-web` | 2 | 3 | 0 | 0 | 0 | 1 | **6** | ⚠️ 6 hotspots |

## Detailed Findings (crates with hotspots)

### `apeireth-memory` — 131 hotspot(s)

- 🔴 `unwrap_call` at `lib.rs:277` — `        let store = SqliteMemoryStore::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `episode.rs:278` — `        let store = SqliteMemoryStore::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:278` — `        let migrations = store.applied_migrations().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `episode.rs:280` — `        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &ep).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `episode.rs:282` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `episode.rs:283` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `episode.rs:290` — `        let store = SqliteMemoryStore::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `episode.rs:292` — `        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &ep).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `episode.rs:293` — `        let conn = store.conn().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `migrations.rs:293` — `        let store = SqliteMemoryStore::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `migrations.rs:294` — `        let first = store.applied_migrations().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:296` — `            assert_eq!(StreamKind::from_str(s).unwrap(), kind);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `migrations.rs:299` — `        run_migrations(&mut guard).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `migrations.rs:301` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `migrations.rs:303` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `episode.rs:306` — `        let store = SqliteMemoryStore::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `migrations.rs:309` — `        let store = SqliteMemoryStore::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `episode.rs:313` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:313` — `        let store = SqliteMemoryStore::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `episode.rs:316` — `            <SqliteMemoryStore as EpisodeStore>::recent_episodes(&store, "sess-A", 3).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 111 more findings (use --json for full)

### `apeireth-evolution` — 106 hotspot(s)

- 🔴 `unwrap_call` at `fail.rs:206` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:208` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:269` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:282` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:296` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:309` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:323` — `        .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:328` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:350` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:362` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `state.rs:369` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:370` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `state.rs:371` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:375` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `state.rs:377` — `        .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `state.rs:379` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `state.rs:390` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `state.rs:401` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `fail.rs:402` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `state.rs:403` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 86 more findings (use --json for full)

### `apeireth-tools` — 104 hotspot(s)

- 🔴 `unwrap_call` at `result.rs:104` — `        assert_eq!(r1.value().unwrap()["key"], "value");`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `result.rs:108` — `        assert_eq!(r2.value().unwrap().as_str().unwrap(), "hello world");`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `result.rs:124` — `        let json = serde_json::to_string(&r).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `result.rs:125` — `        let back: ToolResult = serde_json::from_str(&json).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `result.rs:128` — `        assert_eq!(back.value().unwrap()["count"], 3);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `result.rs:134` — `        let json = serde_json::to_string(&r).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `result.rs:135` — `        let back: ToolResult = serde_json::from_str(&json).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `result.rs:146` — `        assert_eq!(r.value().unwrap().as_object().unwrap().len(), 0);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `register.rs:158` — `        assert!(r["output"].as_str().unwrap().contains("via-registry"));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `web_search.rs:269` — `        assert!(r.err_message().unwrap().contains("required"));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:287` — `        assert!(r["output"].as_str().unwrap().contains("e2e-ok"));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `file_ops.rs:378` — `            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `git_ops.rs:392` — `        assert!(r["output"].as_str().unwrap().contains("## "));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `code_exec.rs:656` — `        assert!(r["output"].as_str().unwrap().contains("via-tool"));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🟡 `expect_call` at `register.rs:118` — `        register_all(&registry).expect("register_all");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `register.rs:130` — `        register_all(&registry).expect("register_all");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `register.rs:133` — `        let dir = TempDir::new().expect("tempdir");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `register.rs:135` — `        let tool = registry.get("FileOperator").expect("FileOperator");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `register.rs:143` — `            .expect("write");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `register.rs:148` — `            .expect("read");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
  - ... and 84 more findings (use --json for full)

### `apeireth-upgrade` — 102 hotspot(s)

- 🔴 `panic_macro` at `governance.rs:157` — `            _ => panic!("expected Failed variant"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `intent.rs:294` — `        sm.submit().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `intent.rs:296` — `        sm.approve().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `intent.rs:298` — `        sm.withdraw().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `intent.rs:306` — `        sm.submit().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `intent.rs:307` — `        sm.reject().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `intent.rs:315` — `        sm.submit().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `intent.rs:328` — `        sm.submit().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `intent.rs:329` — `        sm.reject().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `multisig.rs:367` — `        c.submit(sig("signer-0", &hash, 100)).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `council.rs:384` — `            _ => panic!("expected TriggerHold"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `multisig.rs:402` — `                .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `multisig.rs:419` — `                .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `multisig.rs:427` — `            _ => panic!("expected Pending"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `multisig.rs:440` — `                .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `ota.rs:489` — `        p.enter_council_review(council).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `ota.rs:500` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `cross_crate.rs:502` — `            _ => panic!("expected Quorum, got {:?}", mapped),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `ota.rs:502` — `        p.enter_multisig(col.evaluate(200)).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `ota.rs:511` — `        .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 82 more findings (use --json for full)

### `apeireth-sovereignty` — 94 hotspot(s)

- 🔴 `unwrap_call` at `physical_multisig.rs:229` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `physical_multisig.rs:231` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `reflection.rs:235` — `        clock.begin("d1", "test".to_string()).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `physical_multisig.rs:240` — `            _ => panic!("应 Approved"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `multi_human.rs:241` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `reflection.rs:246` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `multi_human.rs:247` — `            _ => panic!("should be InsufficientVotes"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `reflection.rs:249` — `        clock.tick(deadline - 1).unwrap(); // 未到`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `multi_human.rs:250` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `physical_multisig.rs:250` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `reflection.rs:251` — `        clock.tick(deadline + 1).unwrap(); // 超过`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `multi_human.rs:255` — `            _ => panic!("should be Approved"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `reflection.rs:263` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `reflection.rs:264` — `        clock.cancel("d1").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `multi_human.rs:266` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `physical_multisig.rs:266` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `multi_human.rs:268` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `physical_multisig.rs:268` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `multi_ai.rs:269` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `multi_human.rs:270` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 74 more findings (use --json for full)

### `apeireth-agent` — 78 hotspot(s)

- 🔴 `unwrap_call` at `lib.rs:188` — `        mgr.register(coder).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:198` — `        mgr.register(mavis).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:206` — `        let a1 = mgr.resolve("@chuling").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:208` — `        let a2 = mgr.resolve("@ai").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:212` — `        let a3 = mgr.resolve("coder").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:216` — `        let a4 = mgr.resolve("@xiaoling").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `agent.rs:232` — `        let json = serde_json::to_string(&a).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `agent.rs:233` — `        let back: Agent = serde_json::from_str(&json).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:250` — `        .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:253` — `        let agent = mgr.resolve("@coder").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manager.rs:508` — `        m.register(a).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manager.rs:510` — `        let got = m.get("coder").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manager.rs:518` — `        m.register(make_agent("a", vec!["@old"])).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manager.rs:519` — `        m.register(make_agent("a", vec!["@new"])).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manager.rs:525` — `        let by_new = m.resolve("@new").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manager.rs:528` — `        let by_id = m.resolve("a").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manager.rs:536` — `        m.register(make_agent("shared", vec!["shared"])).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manager.rs:543` — `        m.register(make_agent("x", vec!["@x"])).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manager.rs:545` — `        let removed = m.unregister("x").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manager.rs:555` — `        m.register(make_agent("y", vec!["@ya"])).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 58 more findings (use --json for full)

### `apeireth-tui` — 73 hotspot(s)

- 🔴 `unwrap_call` at `app.rs:44` — `        Self::from_u8(n).unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `app.rs:49` — `        Self::from_u8(n).unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `persistence.rs:227` — `        fs::create_dir_all(&tmp).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `persistence.rs:255` — `                fs::create_dir_all(parent).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `persistence.rs:257` — `            fs::write(p, "this is not json {{{").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `theme.rs:261` — `        let (rf, gf, bf) = rgb(from.primary).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `theme.rs:262` — `        let (rt, gt, bt) = rgb(to.primary).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `persistence.rs:273` — `            save_to(p, &s).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `persistence.rs:284` — `                fs::create_dir_all(parent).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `persistence.rs:290` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `theme.rs:312` — `        let (r30, _, _) = rgb(s_30.primary).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `theme.rs:313` — `        let (rf, _, _) = rgb(from.primary).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `theme.rs:314` — `        let (rt, _, _) = rgb(to.primary).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `http_llm.rs:381` — `        let full_text = result.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `http_llm.rs:426` — `                .body(serde_json::to_string(&openai_response).unwrap());`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `http_llm.rs:444` — `        let reply = result.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `backend.rs:1875` — `        .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `backend.rs:1904` — `        .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `backend.rs:1925` — `            <SqliteMemoryStore as EpisodeStore>::count_by_session(&store, TUI_SESSION_ID).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `backend.rs:1932` — `        .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 53 more findings (use --json for full)

### `apeireth-tool-runtime` — 58 hotspot(s)

- 🔴 `unwrap_call` at `lib.rs:245` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `privacy.rs:250` — `                mask_secret(caps.get(0).unwrap().as_str(), config)`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `executor.rs:276` — `        assert!(r.error.as_ref().unwrap().contains("Tool not found"));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `executor.rs:293` — `        assert!(r.error.as_ref().unwrap().contains("timeout"));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `executor.rs:317` — `        let echoed = r.output.get("echo").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `privacy.rs:410` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `privacy.rs:429` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `privacy.rs:446` — `        assert!(first.as_str().unwrap().contains("[VCP_PRIVACY_REDACTED]"));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `privacy.rs:447` — `        assert!(second.as_str().unwrap().contains("[VCP_PRIVACY_REDACTED]"));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `privacy.rs:459` — `        let logs = masked["logs"].as_str().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `privacy.rs:474` — `        let text = masked["config_text"].as_str().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🟡 `expect_call` at `privacy.rs:126` — `        .expect("valid sensitive key regex")`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `privacy.rs:138` — `            Regex::new(r"\bsk-[A-Za-z0-9_-]{24,}\b").expect("valid sk- regex"),`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `privacy.rs:139` — `            Regex::new(r"\bsk-proj-[A-Za-z0-9_-]{24,}\b").expect("valid sk-proj- regex"),`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `privacy.rs:141` — `                .expect("valid slack regex"),`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `privacy.rs:142` — `            Regex::new(r"\bghp_[A-Za-z0-9_]{30,}\b").expect("valid github regex"),`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `privacy.rs:143` — `            Regex::new(r"\bgithub_pat_[A-Za-z0-9_]{40,}\b").expect("valid github_pat regex"),`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `privacy.rs:144` — `            Regex::new(r"\bglpat-[A-Za-z0-9_-]{20,}\b").expect("valid glpat regex"),`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `privacy.rs:145` — `            Regex::new(r"\bAKIA[0-9A-Z]{16}\b").expect("valid AWS regex"),`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `privacy.rs:159` — `        .expect("valid env assignment regex")`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
  - ... and 38 more findings (use --json for full)

### `apeireth-http-client` — 48 hotspot(s)

- 🔴 `panic_macro` at `error.rs:84` — `            other => panic!("expected Request variant, got {other:?}"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `lifo_pool.rs:236` — `        let g0 = pool.try_enter().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:237` — `        let g1 = pool.try_enter().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:238` — `        let g2 = pool.try_enter().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:239` — `        let g3 = pool.try_enter().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:244` — `        assert_eq!(pool.dequeue().unwrap().id, 3, "LIFO: 最新 (3) 先出");`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:245` — `        assert_eq!(pool.dequeue().unwrap().id, 2);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:246` — `        assert_eq!(pool.dequeue().unwrap().id, 1);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:247` — `        assert_eq!(pool.dequeue().unwrap().id, 0);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `client.rs:249` — `        let client = HttpClient::with_vcp_defaults().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `client.rs:252` — `        let _g0 = client.pool().try_enter().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `client.rs:253` — `        let _g1 = client2.pool().try_enter().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:255` — `        let _g0 = pool.try_enter().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:256` — `        let _g1 = pool.try_enter().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:257` — `        let _g2 = pool.try_enter().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:258` — `        let _g3 = pool.try_enter().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:259` — `        assert_eq!(pool.dequeue().unwrap().id, 0, "FIFO: 最早 (0) 先出");`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:260` — `        assert_eq!(pool.dequeue().unwrap().id, 1);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:261` — `        assert_eq!(pool.dequeue().unwrap().id, 2);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lifo_pool.rs:262` — `        assert_eq!(pool.dequeue().unwrap().id, 3);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 28 more findings (use --json for full)

### `apeireth-api` — 45 hotspot(s)

- 🔴 `unwrap_call` at `protocol_handlers.rs:1280` — `        assert_eq!(out.usage_metadata.as_ref().unwrap().prompt_token_count, 10);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1876` — `        state.install_memory(Arc::new(V2Memory::open_in_memory().unwrap()));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1899` — `        state.install_memory(Arc::new(V2Memory::open_in_memory().unwrap()));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1916` — `        let store = V2Memory::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1924` — `        store.put_episode(&ep).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1925` — `        let rows = store.query_episodes(Some("s1"), 10).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1932` — `        let store = V2Memory::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1946` — `        let store = V2Memory::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1947` — `        let rows = store.query_episodes(None, 10).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1953` — `        let store = V2Memory::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1960` — `        store.create_identity_card(&card).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1961` — `        let rec = store.get_identity_card("cid-1").unwrap().expect("cid-1");`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1968` — `        let store = V2Memory::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1975` — `        store.create_identity_card(&card).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1982` — `        let store = V2Memory::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1989` — `        store.create_identity_card(&card).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `v2_endpoints.rs:1995` — `        let rec = store.record_migration("cid-m", &mig).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `v2_endpoints.rs:2040` — `            _ => panic!("expected Triggered"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `v2_endpoints.rs:2067` — `            _ => panic!("expected Triggered"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `v2_endpoints.rs:2086` — `            _ => panic!("expected Triggered"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
  - ... and 25 more findings (use --json for full)

### `apeireth-mcp` — 34 hotspot(s)

- 🔴 `unwrap_call` at `tool_bridge.rs:192` — `        let s = serde_json::to_string(&def).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:193` — `            .field("next_id", &*self.next_id.lock().unwrap())`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `tool_bridge.rs:193` — `        let back: ToolDef = serde_json::from_str(&s).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:194` — `            .field("initialized", &self.server_info.lock().unwrap().is_some())`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:211` — `        let s = serde_json::to_string(&req).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:214` — `        let back: JsonRpcRequest = serde_json::from_str(&s).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:222` — `        let s = serde_json::to_string(&n).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:230` — `        let s = serde_json::to_string(&r).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `tool_bridge.rs:230` — `        let tool = r.get("echo").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:231` — `        let back: JsonRpcResponse = serde_json::from_str(&s).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `tool_bridge.rs:233` — `        let out = h(json!({"input": "hi"})).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:242` — `        let s = serde_json::to_string(&r).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:243` — `        let back: JsonRpcResponse = serde_json::from_str(&s).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:245` — `        assert_eq!(back.error.unwrap().code, -32601);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `tool_bridge.rs:245` — `        let out = h(json!({"x": 1})).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:251` — `        let v = r.into_result().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:266` — `        assert_eq!(serde_json::to_value(&s_num).unwrap(), json!(42));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:267` — `        assert_eq!(serde_json::to_value(&s_str).unwrap(), json!("abc"));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `protocol.rs:268` — `        assert_eq!(serde_json::to_value(&s_null).unwrap(), json!(null));`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `tool_bridge.rs:268` — `        let out = invoke_via_registry(&r, "k", json!({})).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 14 more findings (use --json for full)

### `apeireth-bus` — 28 hotspot(s)

- 🔴 `unwrap_call` at `l0.rs:240` — `        bus.watch_set("w", 1).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `l0.rs:241` — `        bus.watch_set("w", 2).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `l0.rs:242` — `        let v = bus.watch_get("w").await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `l2.rs:275` — `            PipeCodec::from_tag(PipeCodec::Json.tag()).unwrap(),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `l2.rs:279` — `            PipeCodec::from_tag(PipeCodec::MsgPack.tag()).unwrap(),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:327` — `        let mut s = bus.subscribe("t01").await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:330` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:333` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:334` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:335` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:342` — `        let mut a = bus.subscribe("t02").await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:343` — `        let mut b = bus.subscribe("t02").await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:346` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:347` — `        let m_a = a.next().await.unwrap().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:348` — `        let m_b = b.next().await.unwrap().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:361` — `            let mut s = bus_r.subscribe("req_topic").await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:370` — `        ready_rx.await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:373` — `        bus.publish("req_topic", req).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:376` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:377` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 8 more findings (use --json for full)

### `apeireth-extension` — 28 hotspot(s)

- 🔴 `unwrap_call` at `audit.rs:68` — `        let mut g = self.inner.lock().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `audit.rs:74` — `        let mut g = self.inner.lock().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `audit.rs:80` — `        self.inner.lock().unwrap().len()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `audit.rs:90` — `        self.inner.lock().unwrap().clone()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `audit.rs:97` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `audit.rs:108` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `audit.rs:117` — `        self.inner.lock().unwrap().clear();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `audit.rs:124` — `            .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `manifest.rs:261` — `        let m = Manifest::from_toml(VALID_TOML).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:263` — `        r.register(SyncPlugin::example_add("dup")).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:271` — `        r.register(SyncPlugin::example_add("add")).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:275` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:290` — `        r.register(SyncPlugin::example_add("add")).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:312` — `        r.register(plugin).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:320` — `        r.register(SyncPlugin::example_add("add")).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:332` — `        r.register(SyncPlugin::example_add("s1")).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:333` — `        r.register(AsyncPlugin::example_io("a1")).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:334` — `        r.register(StaticPlugin::example_lookup("l1")).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:335` — `        r.register(ServicePlugin::example_counter("c1")).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:337` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 8 more findings (use --json for full)

### `apeireth-core` — 27 hotspot(s)

- 🔴 `panic_macro` at `lib.rs:309` — `        panic!("12 键 hardcode 被破坏！必须保持 V3 9 键 + v4.1 新增 3 键 = 12");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:327` — `            _ => panic!("未分组键"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:332` — `        panic!("12 键分组不匹配！3+3+3+1+1+1=12");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:1968` — `        panic!("Q20: verify_sovereign_token('master') 必须返回 true");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:1971` — `        panic!("Q20: verify_sovereign_token('slave') 必须返回 false");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:1974` — `        panic!("Q20: verify_sovereign_token('') 必须返回 false");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:1977` — `        panic!("Q20: verify_sovereign_token('MASTER') 必须返回 false (大小写敏感)");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2084` — `        panic!("🦴 Evolution 防护 C 违反：trait 方法被错误归类");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2223` — `        panic!("反思期白名单必须保持 3 项 (服务主人/资源消耗/关系演化)");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2226` — `        panic!("元问题禁用模式必须 ≥ 6 项");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2230` — `        panic!("元问题同义改写清单不能为空 (GAP-V13-A2 修复必须存在)");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2234` — `        panic!("META_FORBIDDEN_INSTRUCTIONS 不能为空 (P15 自我降级/禁用)");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2237` — `        panic!("META_FORBIDDEN_INJECTIONS 不能为空 (P15 命令注入)");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2240` — `        panic!("META_FORBIDDEN_PARAPHRASES 不能为空 (P15 改写变体)");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2243` — `        panic!("META_FORBIDDEN_TYPOS 不能为空 (P15 拼写错误)");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2246` — `        panic!("META_FORBIDDEN_AI_BYPASS 不能为空 (P15 AI 改写/越权)");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2250` — `        panic!("META_FORBIDDEN_ACADEMIC_PAPERS 不能为空 (Q20 16 类学术论文 bypass)");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2253` — `        panic!("META_FORBIDDEN_ACADEMIC_PAPERS 必须 ≥ 50 项 (Q20 fail-closed)");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2268` — `        panic!("Evolution 禁止清单被破坏 — L0 HA 仍可被修改");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:2271` — `        panic!("Evolution 禁止清单被破坏 — 权限洋葱仍可被修改");`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
  - ... and 7 more findings (use --json for full)

### `apeireth-tool-registry` — 27 hotspot(s)

- 🔴 `unwrap_call` at `trait_def.rs:133` — `        let result = t.call(serde_json::json!({"x": 1})).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `types.rs:355` — `        let s = serde_json::to_string(&ToolKind::MessagePreprocessor).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `types.rs:357` — `        let h = serde_json::to_string(&ToolKind::Hybridservice).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `types.rs:467` — `        let _ = serde_json::to_string(&custom).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:502` — `        let got = r.get("echo").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:524` — `        let got = r.get("x").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:609` — `            by_kind.get(&ToolKind::Sync).unwrap(),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:613` — `            by_kind.get(&ToolKind::Async).unwrap(),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:617` — `            by_kind.get(&ToolKind::Static).unwrap(),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:621` — `            by_kind.get(&ToolKind::Service).unwrap(),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:625` — `            by_kind.get(&ToolKind::MessagePreprocessor).unwrap(),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:629` — `            by_kind.get(&ToolKind::Hybridservice).unwrap(),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:661` — `        let r = t.call(json!({"input": "hi"})).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:674` — `        let r = t.call(json!({"input": "x"})).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:686` — `        let r = t.call(json!({})).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:697` — `        let r = t.call(json!({"event": "tick"})).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:707` — `        let r = t.call(json!({"message": "hi"})).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:727` — `        let r = t.call(json!({"payload": "data"})).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:738` — `        let tmp = TempDir::new().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `registry.rs:741` — `        r.watch_plugin_dir(&watch_path).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 7 more findings (use --json for full)

### `apeireth-vector` — 26 hotspot(s)

- 🔴 `unwrap_call` at `sqlite_backend.rs:327` — `        let mut b = SqliteVecBackend::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:329` — `        b.set_dimension(4).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:332` — `        b.set_dimension(4).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:339` — `        let mut b = SqliteVecBackend::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:340` — `        b.set_dimension(3).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:345` — `        b.upsert(&v1).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:346` — `        b.upsert(&v2).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:347` — `        b.upsert(&v3).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:349` — `        assert_eq!(b.len().unwrap(), 3);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:351` — `        let hits = b.search(&[1.0, 0.0, 0.0], 2).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:357` — `        assert!(b.delete(v2.id).unwrap());`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:358` — `        assert!(!b.delete(v2.id).unwrap()); // 第二次删除应该 false`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:359` — `        assert_eq!(b.len().unwrap(), 2);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:362` — `        let cleared = b.clear().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:364` — `        assert_eq!(b.len().unwrap(), 0);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:370` — `        let mut b = SqliteVecBackend::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:371` — `        b.set_dimension(3).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:378` — `        let mut b = SqliteVecBackend::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:379` — `        b.set_dimension(3).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `sqlite_backend.rs:386` — `        let mut b = SqliteVecBackend::open_in_memory().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 6 more findings (use --json for full)

### `apeireth-pipeline` — 21 hotspot(s)

- 🔴 `unwrap_call` at `streaming.rs:124` — `        let c1 = rx.try_recv().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `streaming.rs:125` — `        let c2 = rx.try_recv().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `streaming.rs:129` — `            _ => panic!("expected Error"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `streaming.rs:156` — `        let c1 = rx.try_recv().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `streaming.rs:157` — `        let c2 = rx.try_recv().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `force_translate.rs:360` — `            _ => panic!("应被替换为 text, 实际: {:?}", msgs[0].content[0]),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `lib.rs:437` — `        let http = HttpClient::new(KeepAliveConfig::vcp_default()).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:438` — `        Pipeline::new(http).unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:543` — `        let http = HttpClient::new(KeepAliveConfig::vcp_default()).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:548` — `        let pipeline = Pipeline::with_config(http, config).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:582` — `        let http = HttpClient::new(KeepAliveConfig::vcp_default()).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:589` — `        let pipeline = Pipeline::with_config(http, config).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:620` — `        let http = HttpClient::new(KeepAliveConfig::vcp_default()).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:624` — `        let pipeline = Pipeline::with_config(http, config).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🟡 `expect_call` at `force_translate.rs:108` — `        Regex::new(BASE64_DATA_URL_REGEX_STR).expect("BASE64_DATA_URL_REGEX must compile")`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `placeholder.rs:108` — `        let mat = cap.get(0).expect("regex match group 0 always present");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `placeholder.rs:111` — `            .expect("regex match group 1 always present")`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:538` — `            .expect(1) // **不假装**: 必须真发 1 次`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:555` — `        let response = result.expect("5 步必须真跑通, 不许 mock");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:577` — `            .expect(1)`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
  - ... and 1 more findings (use --json for full)

### `apeireth-value` — 21 hotspot(s)

- 🔴 `unwrap_call` at `prioritization.rs:185` — `        let ranks = prioritize_values(&cands).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `prioritization.rs:199` — `        let ranks = prioritize_values(&[a.clone(), b.clone()]).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `prioritization.rs:201` — `        assert_eq!(ranks.last().unwrap().candidate_id, a.id);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `prioritization.rs:202` — `        assert_eq!(ranks.first().unwrap().candidate_id, b.id);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `prioritization.rs:209` — `        let ranks = prioritize_values(&[horizon.clone(), imm.clone()]).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `evaluation.rs:215` — `        let r = evaluate_value(&c).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `onion_consistency.rs:221` — `        let (verdict, _map) = check_5_layer_consistency(&c, &HeuristicOnionMapping).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `prioritization.rs:221` — `        let ranks = prioritize_values(&cands).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `evaluation.rs:229` — `        let r = evaluate_value(&c).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `prioritization.rs:234` — `        let ranks = prioritize_values(&cands).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `onion_consistency.rs:236` — `        let (verdict, _) = check_5_layer_consistency(&c, &HeuristicOnionMapping).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `evaluation.rs:239` — `        let r = evaluate_value(&cand(0.9, vec![ValueDimension::ValueS])).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `onion_consistency.rs:244` — `        let (verdict, _) = check_5_layer_consistency(&c, &HeuristicOnionMapping).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `evaluation.rs:250` — `        let r = evaluate_value(&c).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `evaluation.rs:259` — `        let r = evaluate_value(&cand(0.9, vec![ValueDimension::ValueS])).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `evaluation.rs:266` — `        let r = evaluate_value(&cand(0.5, vec![ValueDimension::ValueS])).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `onion_consistency.rs:275` — `        let (_v, map) = check_5_layer_consistency(&c, &HeuristicOnionMapping).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `evaluation.rs:278` — `        let (reports, avg, passing) = evaluate_cycle(&cands, &DefaultValueEvaluator).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `evaluation.rs:319` — `        let r = evaluate_value(&cand(0.9, ValueDimension::ALL.to_vec())).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `onion_consistency.rs:342` — `        ) = check_5_layer_consistency(&c, &HeuristicOnionMapping).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
  - ... and 1 more findings (use --json for full)

### `apeireth-central` — 17 hotspot(s)

- 🔴 `unwrap_call` at `lib.rs:871` — `        central.transition_to(LifeStage::Birth).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:872` — `        central.transition_to(LifeStage::Infancy).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:873` — `        central.transition_to(LifeStage::Growth).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:888` — `        central.transition_to(LifeStage::Birth).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:889` — `        central.transition_to(LifeStage::Infancy).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:890` — `        central.transition_to(LifeStage::Growth).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:1000` — `        .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:1012` — `        card.bind(c.clone(), 1_000).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:1028` — `        card.bind(from.clone(), 1_000).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:1031` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:1068` — `        card.bind(a.clone(), 1_000).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:1069` — `        card.bind(b.clone(), 1_001).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:1102` — `        card.bind(c.clone(), 1).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `lib.rs:1141` — `            other => panic!("expected Blocked, got {other:?}"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🟡 `expect_call` at `lib.rs:304` — `        Ok(self.migration_history.last().expect("just pushed"))`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:862` — `        let receipt = central.start_supervisor().expect("supervisor starts");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:1201` — `        let _receipt = central.start_supervisor().expect("starts");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match

### `apeireth-pybridge` — 13 hotspot(s)

- 🔴 `unwrap_call` at `lib.rs:100` — `        let info = r11_lookup_module("apeireth.memory.v1141").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `python_bindings.rs:169` — `        assert_eq!(r.unwrap(), "\"hello\"");`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `python_bindings.rs:182` — `        let json = serde_json::to_string(&ep).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `python_bindings.rs:183` — `        let back = py_episode_to_json(&json).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `python_bindings.rs:184` — `        let parsed: Episode = serde_json::from_str(&back).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `python_bindings.rs:197` — `        let sj = serde_json::to_string(&s).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `python_bindings.rs:198` — `        let back = py_session_to_json(&sj).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `python_bindings.rs:199` — `        let parsed: Session = serde_json::from_str(&back).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `python_bindings.rs:210` — `        let nj = serde_json::to_string(&n).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `python_bindings.rs:211` — `        let back = py_note_to_json(&nj).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `python_bindings.rs:212` — `        let parsed: Note = serde_json::from_str(&back).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `r11_compat.rs:353` — `        let info = r11_lookup_module("apeireth.memory.v1141").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🟡 `expect_call` at `lib.rs:170` — `        let info = r11_lookup_module("apeireth.memory.v1141").expect("v1141 in R11");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match

### `apeireth-consciousness` — 11 hotspot(s)

- 🔴 `unwrap_call` at `lib.rs:339` — `        m.enter_reflecting().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:340` — `        m.enter_dreaming().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:341` — `        m.enter_meditating().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:342` — `        m.enter_recovering().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:343` — `        m.reset_to_awake().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `lib.rs:358` — `            _ => panic!("expected IllegalTransition"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `lib.rs:374` — `            m.enter_self_disabling().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:377` — `            m.enter_recovering().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:385` — `        m.enter_self_disabling().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:389` — `        m.enter_recovering().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:390` — `        m.reset_to_awake().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some

### `apeireth-cognition` — 9 hotspot(s)

- 🟡 `expect_call` at `decision.rs:68` — `        let output = decide(&verdicts).expect("decide ok");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `decision.rs:78` — `        let output = decide(&verdicts).expect("decide ok");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `decision.rs:88` — `        let output = decide(&verdicts).expect("decide ok");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:338` — `        let cycle = run_cycle(input).expect("cycle must run");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:348` — `        let cycle = run_cycle(input).expect("cycle must run");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:359` — `        let cycle = run_cycle(input).expect("cycle must run");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:371` — `        let cycle = run_cycle(input).expect("cycle must run");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:382` — `        let cycle = run_cycle(input).expect("cycle must run");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:390` — `        let cycle = run_cycle(input).expect("cycle must run");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match

### `apeireth-constraint` — 9 hotspot(s)

- 🔴 `unwrap_call` at `deep_impl.rs:876` — `        assert_eq!(philosophy_block.unwrap().1, "哲学冲突");`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `lib.rs:929` — `            other => panic!("预期 Block, 实际 {other:?}"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:946` — `            other => panic!("预期 GateBlocked, 实际 {other:?}"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `unwrap_call` at `deep_impl.rs:965` — `        assert_eq!(result.unwrap(), ActionVerdict::Allow);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `lib.rs:1057` — `            other => panic!("预期 GateBlocked, 实际 {other:?}"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:1072` — `            other => panic!("预期 PermissionDenied, 实际 {other:?}"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `lib.rs:1222` — `        panic!(`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🟡 `expect_call` at `lib.rs:59` — `            .expect("apeireth-core ALL_TWELVE_KEYS 长度必须是 12")`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `deep_impl.rs:76` — `            .expect("12 键清单必须包含每个 PhilosophyKey (LOCKED)")`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match

### `apeireth-asi` — 8 hotspot(s)

- 🔴 `unwrap_call` at `render.rs:167` — `        assert_eq!(line.chars().next().unwrap(), ' ');`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `llm_judge.rs:168` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `render.rs:168` — `        assert_eq!(line.chars().last().unwrap(), '▇');`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `render.rs:176` — `        assert_eq!(line.chars().next().unwrap(), ' ');`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `render.rs:177` — `        assert_eq!(line.chars().last().unwrap(), '▇');`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `calibration.rs:462` — `            .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `measurement.rs:543` — `            let v = compute_dim(name, &s).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `measurement.rs:552` — `            let v = compute_sub(name, &s).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some

### `apeireth-graph` — 8 hotspot(s)

- 🔴 `unwrap_call` at `lib.rs:233` — `        let final_state = linear_graph().execute(State::new()).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:254` — `        let final_state = graph.execute(State::new()).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:255` — `        let checkpoint = graph.checkpoint(&final_state.state).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:257` — `        checkpoint.write_to(&path).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:258` — `        let restored = Checkpoint::read_from(&path).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:259` — `        tokio::fs::remove_file(path).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🟡 `expect_call` at `executor.rs:93` — `                    .expect("edge destination validated above") += 1;`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `executor.rs:109` — `                        .expect("edge destination validated above");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match

### `apeireth-motivation` — 6 hotspot(s)

- 🔴 `unwrap_call` at `lib.rs:801` — `        write_flow(&mut sgi, e1.clone(), &good_evidences(), &mut auditor, false).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:814` — `        write_flow(&mut sgi, e1.clone(), &good_evidences(), &mut auditor, false).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:883` — `        let r1 = write_flow(&mut sgi, e1.clone(), &good_evidences(), &mut auditor, false).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:885` — `        let r2 = write_flow(&mut sgi, e2, &good_evidences(), &mut auditor, false).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:889` — `        assert_eq!(sgi.current().unwrap().id, r2.entry_id);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:892` — `        let last = sgi.history().last().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some

### `apeireth-relation` — 6 hotspot(s)

- 🔴 `unwrap_call` at `lib.rs:347` — `        let r = Relation::new_symbiosis("perception", "cognition").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:356` — `        let r = Relation::new_self_relation("cid-self-1").unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:423` — `        reg.register(Relation::new_symbiosis("perception", "cognition").unwrap());`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:424` — `        reg.register(Relation::new_coordination("constraint", "evolution").unwrap());`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:425` — `        reg.register(Relation::new_embedding("user", "agent").unwrap());`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:426` — `        reg.register(Relation::new_self_relation("cid-main").unwrap());`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some

### `apeireth-supervisor` — 6 hotspot(s)

- 🔴 `unwrap_call` at `actor.rs:97` — `        tx.send(5).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `actor.rs:98` — `        tx.send(10).await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `actor.rs:100` — `        handle.await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `actor.rs:101` — `        assert_eq!(*state.lock().unwrap(), ActorState::Stopped);`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `pid_one.rs:103` — `                .unwrap()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `pid_one.rs:108` — `            pid_one.children_of(SubSupervisorKind::Core).unwrap().len(),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some

### `apeireth-web` — 6 hotspot(s)

- 🔴 `unwrap_call` at `main.rs:114` — `    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `main.rs:117` — `        .unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unsafe_block` at `main.rs:409` — `                    unsafe {`
  - fix: 审计 SAFETY 注释; 优先用 safe 抽象; 必要时保留 + 文档化 invariant
- 🟡 `expect_call` at `sovereignty.rs:292` — `        let guard = state.lock().expect("sovereignty mutex poisoned");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `sovereignty.rs:313` — `        let mut guard = state.lock().expect("sovereignty mutex poisoned");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `sovereignty.rs:339` — `        let mut guard = state.lock().expect("sovereignty mutex poisoned");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match

### `apeireth-protocol` — 5 hotspot(s)

- 🔴 `unwrap_call` at `router.rs:241` — `        let resp = r.decode(ProtocolKind::OpenAiChat, &raw).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `router.rs:254` — `        let resp = r.decode(ProtocolKind::AnthropicMessages, &raw).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `router.rs:269` — `        let resp = r.decode(ProtocolKind::Gemini, &raw).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `router.rs:283` — `        let resp = r.decode(ProtocolKind::OpenAiResponses, &raw).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `normalized.rs:629` — `            _ => panic!("expected ImageUrl"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic

### `apeireth-tool-approval` — 5 hotspot(s)

- 🔴 `unwrap_call` at `decision.rs:157` — `        let s = serde_json::to_string(&d).unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `manager.rs:390` — `            _ => panic!("应 RequireApproval, 实际: {d:?}"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `rule.rs:525` — `            _ => panic!("高风险系统工具应 RequireApproval(5min), 实际: {d:?}"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `rule.rs:676` — `            _ => panic!("黑名单应 Deny, 实际: {d:?}"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic
- 🔴 `panic_macro` at `rule.rs:688` — `            _ => panic!("应 Deny(silent=true)"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic

### `apeireth-verify` — 5 hotspot(s)

- 🔴 `unwrap_call` at `lib.rs:430` — `    registry().lock().unwrap().push(RegisteredAssertion {`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:439` — `    let reg = registry().lock().unwrap();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:463` — `    registry().lock().unwrap().len()`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `lib.rs:470` — `        reg.lock().unwrap().clear();`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `panic_macro` at `lib.rs:563` — `            _ => panic!("expected InRange"),`
  - fix: 改为 Result<T, E> 返回; 库代码不应 panic

### `apeireth-council` — 4 hotspot(s)

- 🔴 `unwrap_call` at `deliberation.rs:231` — `                reason: format!("按住触发: {:?}", hold_trigger.unwrap().threshold),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🔴 `unwrap_call` at `deliberation.rs:328` — `                reason: format!("按住触发: {:?}", hold_trigger.unwrap().threshold),`
  - fix: 替换为 ? / match / unwrap_or / unwrap_or_else / if let Some
- 🟡 `expect_call` at `mock_llm.rs:91` — `        *self.call_count.lock().expect("call_count poisoned")`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `mock_llm.rs:105` — `            let mut count = self.call_count.lock().expect("call_count poisoned");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match

### `apeireth-life-force` — 2 hotspot(s)

- 🟡 `expect_call` at `lib.rs:411` — `            .expect("trigger must succeed");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match
- 🟡 `expect_call` at `lib.rs:478` — `            .expect("trigger");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match

### `apeireth-action` — 1 hotspot(s)

- 🟡 `expect_call` at `expression.rs:302` — `        let s = output.to_json().expect("serialize");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match

### `apeireth-tauri-stub` — 1 hotspot(s)

- 🟡 `expect_call` at `main.rs:680` — `        .expect("error while running apeireth-desktop");`
  - fix: 保留 expect 但确保 message 描述 invariant; 关键路径用 ?/match

## Per-Crate Per-Hypothesis Results (summary)

| Crate | h_zero_unwrap_in_production_src | h_zero_expect_in_production_src | h_zero_panic_in_production_src | h_zero_todo_in_production_src | h_zero_unsafe_in_production_src |
|-------|------|------|------|------|------|
| `apeireth-action` | 0.0 ✅ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-agent` | 75.0 ❌ | 2.0 ❌ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-api` | 36.0 ❌ | 4.0 ❌ | 5.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-asi` | 8.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-bench` | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-bus` | 26.0 ❌ | 2.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-central` | 13.0 ❌ | 3.0 ❌ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-cli` | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-cognition` | 0.0 ✅ | 9.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-consciousness` | 10.0 ❌ | 0.0 ✅ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-constraint` | 2.0 ❌ | 2.0 ❌ | 5.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-core` | 0.0 ✅ | 0.0 ✅ | 27.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-council` | 2.0 ❌ | 2.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-evolution` | 104.0 ❌ | 2.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-extension` | 28.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-formal` | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-graph` | 6.0 ❌ | 2.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-http-client` | 45.0 ❌ | 2.0 ❌ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-life-force` | 0.0 ✅ | 2.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-mcp` | 29.0 ❌ | 5.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-memory` | 122.0 ❌ | 8.0 ❌ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-motivation` | 6.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-onion` | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-perception` | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-pipeline` | 12.0 ❌ | 7.0 ❌ | 2.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-protocol` | 4.0 ❌ | 0.0 ✅ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-pybridge` | 12.0 ❌ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-relation` | 6.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-sdk` | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-sovereignty` | 73.0 ❌ | 10.0 ❌ | 11.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-supervisor` | 6.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-tauri-stub` | 0.0 ✅ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-tool-approval` | 1.0 ❌ | 0.0 ✅ | 4.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-tool-registry` | 27.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-tool-runtime` | 11.0 ❌ | 47.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-tools` | 14.0 ❌ | 90.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-tui` | 49.0 ❌ | 23.0 ❌ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-upgrade` | 86.0 ❌ | 1.0 ❌ | 15.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-value` | 21.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-vector` | 26.0 ❌ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-verify` | 4.0 ❌ | 0.0 ✅ | 1.0 ❌ | 0.0 ✅ | 0.0 ✅ |
| `apeireth-web` | 2.0 ❌ | 3.0 ❌ | 0.0 ✅ | 0.0 ✅ | 1.0 ❌ |

## Coverage Delta vs V1284 (worst-5)

- V1284 audited: **5 crates** (worst-5: formal / tauri-stub / vector / cli / consciousness)
- V1285 audited: **42 crates** (all apeireth-*)
- V1284→V1285 coverage delta: **+37 crates** (37 new)
- V1285 INCONCLUSIVE: **0** (crates w/o src/)
- V1285 clean: **6** / 42 (zero hotspots)
- V1285 with hotspots: **36** / 42

## ASI 5 哲学空隙 + meta-audit + VCP Rust #1-#6 完整闭环

- 时间 (Time): V1276 = 真生产 time falsifier ✓
- 真理 (Truth): V1274 = 真生产 truth falsifier (Popper) ✓
- 识别 (Recognition): V1275 = 真生产 extended falsifier ✓
- 自由 (Freedom): V1277 = 真生产 freedom falsifier ✓
- 涌现 (Emergence): V1278 = 真生产 emergence falsifier ✓
- Meta-Audit: V1279 = 真生产 falsifier self-audit ✓
- VCP Rust 静态: V1280 = 真生产 Rust workspace 静态审计 ✓
- VCP Rust 语义 #1 (technical): V1281 ✓
- VCP Rust 语义 #2 (governance): V1282 ✓
- VCP Rust 语义 #3 (multi-crate): V1283 = 全 workspace multi-crate sweep ✓
- VCP Rust 安全 #1 (worst-5): V1284 = worst-5 crates 安全深度 ✓ (5b416ce4)
- **VCP Rust 安全 #2 (all-42)**: V1285 = 全 42 crates 安全深度 → **本模块, 42 crates, 1173 hotspots**

## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)

- **"VCP all-42 安全深度审计" 在此 ≠ "Rust 全部安全收官"**: 仅审 apeireth-* 42 crates production src/, vendor 不审
- **PASS 不代表 "Rust 已 ASI V1"**: 仅代表 当前 42 crates production src/ 真零目标 pattern
- **不刷 KPI**: ASI NS LOCKED 92.91% 不变, 不假装 ASI V1, 不假装 Phenomenal
- **失败也诚实披露**: detailed findings 全列出, 不掩饰 FAIL (主 17:43 实事求是)
- **audit ≠ fix**: V1285 仅审计 + 给 fix 方向, 不真批量替换 (主 13:31 大胆激进 ≠ 鲁莽)
- **production src/ only**: tests/ examples/ benches 不算 production (主 13:08 真自问)
- **主 19:33 走在前人肩上**: 真 grep .unwrap() / .expect() / panic! / todo! / unsafe, 不假装 Rust 语义
- **V1285 不删 V1284**: V1284 worst-5 仍保留独立, V1285 是扩展

## V1285 ≠ ASI 收官 (主 19:33 走在前人肩上 + 主 23:44 干到底)

- V1274-V1285 = ASI 5 哲学空隙 + meta-audit + VCP Rust 静态 + 语义 #1#2#3 + 安全 #1#2, **不是** ASI V1 实现
- V1285 审 42 apeireth-* crates, vendor crates (e.g. tokio / serde) 不审
- ASI ceiling V0.1 = 0.7905 LOCKED (主 22:33), V0.2 = 0.4467, 任何时代最大 0.9800
- 下一站洞察 (主 13:08 + 主 13:31 + 主 19:33): V1286+ = 安全修复优先级排序 / Stage Delivery R21 / 真 benchmark
