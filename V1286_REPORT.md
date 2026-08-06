# V1286 Security Fix Priority Queue — Run `v1286-1785926826`

- Run timestamp: `1785926826.100` (unix)
- Build: `2026-08-05-1840+08` version: `0.1.0`
- ASI NS current: `0.7905` (display 92.91%)
- Source audit: `V1285_all42_security_audit` (V1285 1173 hotspots)
- Promethean dir: `.openclaw\workspace\promethean`
- Crates scored: **42**
- **P0 (立即修)**: **23**
- **P1 (本 sprint)**: **9**
- **P2 (本季度)**: **4**
- **OK (无需修, clean)**: **6**
- Total score across all crates: **10735**
- Crates with unsafe blocks: **1**
- Elapsed: `292.1 ms`

## V3 Philosophy Gate (主 17:58 + 主 20:46 + 主 17:43 不假装)

- ✅ `v1285_inherited_gate_0` = True
- ✅ `v1285_inherited_gate_1` = True
- ✅ `v1285_inherited_gate_2` = True
- ✅ `v1285_inherited_gate_3` = True
- ✅ `v1285_inherited_gate_4` = True
- ✅ `v1285_inherited_gate_5` = True
- ✅ `v1285_inherited_gate_6` = True
- ✅ `v1285_inherited_gate_7` = True
- ✅ `v1285_inherited_gate_8` = True
- ✅ `v1285_inherited_gate_9` = True
- ✅ `v1285_inherited_gate_10` = True
- ✅ `v1285_inherited_gate_11` = True
- ✅ `v1285_inherited_gate_12` = True
- ✅ `v1285_inherited_gate_13` = True
- ✅ `v1285_inherited_gate_14` = True
- ✅ `v1285_inherited_gate_15` = True
- ✅ `v1285_inherited_gate_16` = True
- ✅ `v1285_inherited_gate_17` = True
- ✅ `v1285_inherited_gate_18` = True
- ✅ `v1285_inherited_gate_19` = True
- ✅ `v1285_inherited_gate_20` = True
- ✅ `v1285_inherited_gate_21` = True
- ✅ `v1285_inherited_gate_22` = True
- ✅ `v1285_inherited_gate_23` = True
- ✅ `v1285_inherited_gate_24` = True
- ✅ `v1285_inherited_gate_25` = True
- ✅ `v1285_inherited_gate_26` = True
- ✅ `v1286_extends_v1285_not_replaces` = True
- ✅ `v1286_priority_only_no_fix` = True
- ✅ `v1286_governance_weight_advisory` = True

## 🔴 P0 (立即修) — 23 crates

| Rank | Crate | Score | Critical | Important | Unsafe | Gov | Base | Unsafe-bonus |
|------|-------|-------|----------|-----------|--------|-----|------|--------------|
| 1 | `apeireth-memory` | **1270** | 123 | 8 | 0 - | -0 | 1270 | +0 |
| 2 | `apeireth-evolution` | **1070** | 104 | 2 | 0 - | +20 | 1050 | +0 |
| 3 | `apeireth-upgrade` | **1035** | 101 | 1 | 0 - | +20 | 1015 | +0 |
| 4 | `apeireth-sovereignty` | **910** | 84 | 10 | 0 - | +20 | 890 | +0 |
| 5 | `apeireth-agent` | **770** | 76 | 2 | 0 - | -0 | 770 | +0 |
| 6 | `apeireth-tui` | **615** | 50 | 23 | 0 - | -0 | 615 | +0 |
| 7 | `apeireth-tools` | **590** | 14 | 90 | 0 - | -0 | 590 | +0 |
| 8 | `apeireth-http-client` | **470** | 46 | 2 | 0 - | -0 | 470 | +0 |
| 9 | `apeireth-api` | **430** | 41 | 4 | 0 - | -0 | 430 | +0 |
| 10 | `apeireth-tool-runtime` | **345** | 11 | 47 | 0 - | -0 | 345 | +0 |
| 11 | `apeireth-mcp` | **315** | 29 | 5 | 0 - | -0 | 315 | +0 |
| 12 | `apeireth-extension` | **280** | 28 | 0 | 0 - | -0 | 280 | +0 |
| 13 | `apeireth-bus` | **270** | 26 | 2 | 0 - | -0 | 270 | +0 |
| 14 | `apeireth-core` | **270** | 27 | 0 | 0 - | -0 | 270 | +0 |
| 15 | `apeireth-tool-registry` | **270** | 27 | 0 | 0 - | -0 | 270 | +0 |
| 16 | `apeireth-vector` | **260** | 26 | 0 | 0 - | -0 | 260 | +0 |
| 17 | `apeireth-value` | **210** | 21 | 0 | 0 - | -0 | 210 | +0 |
| 18 | `apeireth-pipeline` | **175** | 14 | 7 | 0 - | -0 | 175 | +0 |
| 19 | `apeireth-central` | **155** | 14 | 3 | 0 - | -0 | 155 | +0 |
| 20 | `apeireth-pybridge` | **125** | 12 | 1 | 0 - | -0 | 125 | +0 |
| 21 | `apeireth-consciousness` | **110** | 11 | 0 | 0 - | -0 | 110 | +0 |
| 22 | `apeireth-asi` | **100** | 8 | 0 | 0 - | +20 | 80 | +0 |
| 23 | `apeireth-web` | **95** | 3 | 3 | 1 ⚠️ | -0 | 45 | +50 |

## 🟡 P1 (本 sprint) — 9 crates

| Rank | Crate | Score | Critical | Important | Unsafe | Gov | Base | Unsafe-bonus |
|------|-------|-------|----------|-----------|--------|-----|------|--------------|
| 1 | `apeireth-constraint` | **80** | 7 | 2 | 0 - | -0 | 80 | +0 |
| 2 | `apeireth-graph` | **70** | 6 | 2 | 0 - | -0 | 70 | +0 |
| 3 | `apeireth-motivation` | **60** | 6 | 0 | 0 - | -0 | 60 | +0 |
| 4 | `apeireth-relation` | **60** | 6 | 0 | 0 - | -0 | 60 | +0 |
| 5 | `apeireth-supervisor` | **60** | 6 | 0 | 0 - | -0 | 60 | +0 |
| 6 | `apeireth-council` | **50** | 2 | 2 | 0 - | +20 | 30 | +0 |
| 7 | `apeireth-protocol` | **50** | 5 | 0 | 0 - | -0 | 50 | +0 |
| 8 | `apeireth-tool-approval` | **50** | 5 | 0 | 0 - | -0 | 50 | +0 |
| 9 | `apeireth-verify` | **50** | 5 | 0 | 0 - | -0 | 50 | +0 |

## 🟢 P2 (本季度) — 4 crates

| Rank | Crate | Score | Critical | Important | Unsafe | Gov | Base | Unsafe-bonus |
|------|-------|-------|----------|-----------|--------|-----|------|--------------|
| 1 | `apeireth-cognition` | **45** | 0 | 9 | 0 - | -0 | 45 | +0 |
| 2 | `apeireth-life-force` | **10** | 0 | 2 | 0 - | -0 | 10 | +0 |
| 3 | `apeireth-action` | **5** | 0 | 1 | 0 - | -0 | 5 | +0 |
| 4 | `apeireth-tauri-stub` | **5** | 0 | 1 | 0 - | -0 | 5 | +0 |

## ✅ OK (clean, 无需修) — 6 crates

| Rank | Crate | Score | Critical | Important | Unsafe | Gov | Base | Unsafe-bonus |
|------|-------|-------|----------|-----------|--------|-----|------|--------------|
| 1 | `apeireth-bench` | **0** | 0 | 0 | 0 - | -0 | 0 | +0 |
| 2 | `apeireth-cli` | **0** | 0 | 0 | 0 - | -0 | 0 | +0 |
| 3 | `apeireth-formal` | **0** | 0 | 0 | 0 - | -0 | 0 | +0 |
| 4 | `apeireth-onion` | **0** | 0 | 0 | 0 - | -0 | 0 | +0 |
| 5 | `apeireth-perception` | **0** | 0 | 0 | 0 - | -0 | 0 | +0 |
| 6 | `apeireth-sdk` | **0** | 0 | 0 | 0 - | -0 | 0 | +0 |

## Top-10 Crates by Total Score (overall)

| Rank | Crate | Score | Critical | Important | Unsafe | Priority |
|------|-------|-------|----------|-----------|--------|----------|
| 1 | `apeireth-memory` | **1270** | 123 | 8 | 0 | P0 |
| 2 | `apeireth-evolution` | **1070** | 104 | 2 | 0 | P0 |
| 3 | `apeireth-upgrade` | **1035** | 101 | 1 | 0 | P0 |
| 4 | `apeireth-sovereignty` | **910** | 84 | 10 | 0 | P0 |
| 5 | `apeireth-agent` | **770** | 76 | 2 | 0 | P0 |
| 6 | `apeireth-tui` | **615** | 50 | 23 | 0 | P0 |
| 7 | `apeireth-tools` | **590** | 14 | 90 | 0 | P0 |
| 8 | `apeireth-http-client` | **470** | 46 | 2 | 0 | P0 |
| 9 | `apeireth-api` | **430** | 41 | 4 | 0 | P0 |
| 10 | `apeireth-tool-runtime` | **345** | 11 | 47 | 0 | P0 |

## P0 Detail — Top Findings (first 5 critical per P0 crate)

### `apeireth-memory` (score=1270, n_critical=123)

- 🔴 `unwrap_call` at `lib.rs:277` — `        let store = SqliteMemoryStore::open_in_memory().unwrap();`
- 🔴 `unwrap_call` at `episode.rs:278` — `        let store = SqliteMemoryStore::open_in_memory().unwrap();`
- 🔴 `unwrap_call` at `lib.rs:278` — `        let migrations = store.applied_migrations().unwrap();`
- 🔴 `unwrap_call` at `episode.rs:280` — `        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &ep).unwrap();`
- 🔴 `unwrap_call` at `episode.rs:282` — `            .unwrap()`

### `apeireth-evolution` (score=1070, n_critical=104)

- 🔴 `unwrap_call` at `fail.rs:206` — `            .unwrap();`
- 🔴 `unwrap_call` at `fail.rs:208` — `            .unwrap();`
- 🔴 `unwrap_call` at `fail.rs:269` — `            .unwrap();`
- 🔴 `unwrap_call` at `fail.rs:282` — `            .unwrap();`
- 🔴 `unwrap_call` at `fail.rs:296` — `            .unwrap();`

### `apeireth-upgrade` (score=1035, n_critical=101)

- 🔴 `panic_macro` at `governance.rs:157` — `            _ => panic!("expected Failed variant"),`
- 🔴 `unwrap_call` at `intent.rs:294` — `        sm.submit().unwrap();`
- 🔴 `unwrap_call` at `intent.rs:296` — `        sm.approve().unwrap();`
- 🔴 `unwrap_call` at `intent.rs:298` — `        sm.withdraw().unwrap();`
- 🔴 `unwrap_call` at `intent.rs:306` — `        sm.submit().unwrap();`

### `apeireth-sovereignty` (score=910, n_critical=84)

- 🔴 `unwrap_call` at `physical_multisig.rs:229` — `            .unwrap();`
- 🔴 `unwrap_call` at `physical_multisig.rs:231` — `            .unwrap();`
- 🔴 `unwrap_call` at `reflection.rs:235` — `        clock.begin("d1", "test".to_string()).unwrap();`
- 🔴 `panic_macro` at `physical_multisig.rs:240` — `            _ => panic!("应 Approved"),`
- 🔴 `unwrap_call` at `multi_human.rs:241` — `            .unwrap();`

### `apeireth-agent` (score=770, n_critical=76)

- 🔴 `unwrap_call` at `lib.rs:188` — `        mgr.register(coder).unwrap();`
- 🔴 `unwrap_call` at `lib.rs:198` — `        mgr.register(mavis).unwrap();`
- 🔴 `unwrap_call` at `lib.rs:206` — `        let a1 = mgr.resolve("@chuling").unwrap();`
- 🔴 `unwrap_call` at `lib.rs:208` — `        let a2 = mgr.resolve("@ai").unwrap();`
- 🔴 `unwrap_call` at `lib.rs:212` — `        let a3 = mgr.resolve("coder").unwrap();`

### `apeireth-tui` (score=615, n_critical=50)

- 🔴 `unwrap_call` at `app.rs:44` — `        Self::from_u8(n).unwrap()`
- 🔴 `unwrap_call` at `app.rs:49` — `        Self::from_u8(n).unwrap()`
- 🔴 `unwrap_call` at `persistence.rs:227` — `        fs::create_dir_all(&tmp).unwrap();`
- 🔴 `unwrap_call` at `persistence.rs:255` — `                fs::create_dir_all(parent).unwrap();`
- 🔴 `unwrap_call` at `persistence.rs:257` — `            fs::write(p, "this is not json {{{").unwrap();`

### `apeireth-tools` (score=590, n_critical=14)

- 🔴 `unwrap_call` at `result.rs:104` — `        assert_eq!(r1.value().unwrap()["key"], "value");`
- 🔴 `unwrap_call` at `result.rs:108` — `        assert_eq!(r2.value().unwrap().as_str().unwrap(), "hello world");`
- 🔴 `unwrap_call` at `result.rs:124` — `        let json = serde_json::to_string(&r).unwrap();`
- 🔴 `unwrap_call` at `result.rs:125` — `        let back: ToolResult = serde_json::from_str(&json).unwrap();`
- 🔴 `unwrap_call` at `result.rs:128` — `        assert_eq!(back.value().unwrap()["count"], 3);`

### `apeireth-http-client` (score=470, n_critical=46)

- 🔴 `panic_macro` at `error.rs:84` — `            other => panic!("expected Request variant, got {other:?}"),`
- 🔴 `unwrap_call` at `lifo_pool.rs:236` — `        let g0 = pool.try_enter().unwrap();`
- 🔴 `unwrap_call` at `lifo_pool.rs:237` — `        let g1 = pool.try_enter().unwrap();`
- 🔴 `unwrap_call` at `lifo_pool.rs:238` — `        let g2 = pool.try_enter().unwrap();`
- 🔴 `unwrap_call` at `lifo_pool.rs:239` — `        let g3 = pool.try_enter().unwrap();`

### `apeireth-api` (score=430, n_critical=41)

- 🔴 `unwrap_call` at `protocol_handlers.rs:1280` — `        assert_eq!(out.usage_metadata.as_ref().unwrap().prompt_token_count, 10);`
- 🔴 `unwrap_call` at `v2_endpoints.rs:1876` — `        state.install_memory(Arc::new(V2Memory::open_in_memory().unwrap()));`
- 🔴 `unwrap_call` at `v2_endpoints.rs:1899` — `        state.install_memory(Arc::new(V2Memory::open_in_memory().unwrap()));`
- 🔴 `unwrap_call` at `v2_endpoints.rs:1916` — `        let store = V2Memory::open_in_memory().unwrap();`
- 🔴 `unwrap_call` at `v2_endpoints.rs:1924` — `        store.put_episode(&ep).unwrap();`

### `apeireth-tool-runtime` (score=345, n_critical=11)

- 🔴 `unwrap_call` at `lib.rs:245` — `            .unwrap()`
- 🔴 `unwrap_call` at `privacy.rs:250` — `                mask_secret(caps.get(0).unwrap().as_str(), config)`
- 🔴 `unwrap_call` at `executor.rs:276` — `        assert!(r.error.as_ref().unwrap().contains("Tool not found"));`
- 🔴 `unwrap_call` at `executor.rs:293` — `        assert!(r.error.as_ref().unwrap().contains("timeout"));`
- 🔴 `unwrap_call` at `executor.rs:317` — `        let echoed = r.output.get("echo").unwrap();`

### `apeireth-mcp` (score=315, n_critical=29)

- 🔴 `unwrap_call` at `tool_bridge.rs:192` — `        let s = serde_json::to_string(&def).unwrap();`
- 🔴 `unwrap_call` at `lib.rs:193` — `            .field("next_id", &*self.next_id.lock().unwrap())`
- 🔴 `unwrap_call` at `tool_bridge.rs:193` — `        let back: ToolDef = serde_json::from_str(&s).unwrap();`
- 🔴 `unwrap_call` at `lib.rs:194` — `            .field("initialized", &self.server_info.lock().unwrap().is_some())`
- 🔴 `unwrap_call` at `protocol.rs:211` — `        let s = serde_json::to_string(&req).unwrap();`

### `apeireth-extension` (score=280, n_critical=28)

- 🔴 `unwrap_call` at `audit.rs:68` — `        let mut g = self.inner.lock().unwrap();`
- 🔴 `unwrap_call` at `audit.rs:74` — `        let mut g = self.inner.lock().unwrap();`
- 🔴 `unwrap_call` at `audit.rs:80` — `        self.inner.lock().unwrap().len()`
- 🔴 `unwrap_call` at `audit.rs:90` — `        self.inner.lock().unwrap().clone()`
- 🔴 `unwrap_call` at `audit.rs:97` — `            .unwrap()`

### `apeireth-bus` (score=270, n_critical=26)

- 🔴 `unwrap_call` at `l0.rs:240` — `        bus.watch_set("w", 1).await.unwrap();`
- 🔴 `unwrap_call` at `l0.rs:241` — `        bus.watch_set("w", 2).await.unwrap();`
- 🔴 `unwrap_call` at `l0.rs:242` — `        let v = bus.watch_get("w").await.unwrap();`
- 🔴 `unwrap_call` at `l2.rs:275` — `            PipeCodec::from_tag(PipeCodec::Json.tag()).unwrap(),`
- 🔴 `unwrap_call` at `l2.rs:279` — `            PipeCodec::from_tag(PipeCodec::MsgPack.tag()).unwrap(),`

### `apeireth-core` (score=270, n_critical=27)

- 🔴 `panic_macro` at `lib.rs:309` — `        panic!("12 键 hardcode 被破坏！必须保持 V3 9 键 + v4.1 新增 3 键 = 12");`
- 🔴 `panic_macro` at `lib.rs:327` — `            _ => panic!("未分组键"),`
- 🔴 `panic_macro` at `lib.rs:332` — `        panic!("12 键分组不匹配！3+3+3+1+1+1=12");`
- 🔴 `panic_macro` at `lib.rs:1968` — `        panic!("Q20: verify_sovereign_token('master') 必须返回 true");`
- 🔴 `panic_macro` at `lib.rs:1971` — `        panic!("Q20: verify_sovereign_token('slave') 必须返回 false");`

### `apeireth-tool-registry` (score=270, n_critical=27)

- 🔴 `unwrap_call` at `trait_def.rs:133` — `        let result = t.call(serde_json::json!({"x": 1})).await.unwrap();`
- 🔴 `unwrap_call` at `types.rs:355` — `        let s = serde_json::to_string(&ToolKind::MessagePreprocessor).unwrap();`
- 🔴 `unwrap_call` at `types.rs:357` — `        let h = serde_json::to_string(&ToolKind::Hybridservice).unwrap();`
- 🔴 `unwrap_call` at `types.rs:467` — `        let _ = serde_json::to_string(&custom).unwrap();`
- 🔴 `unwrap_call` at `registry.rs:502` — `        let got = r.get("echo").unwrap();`

### `apeireth-vector` (score=260, n_critical=26)

- 🔴 `unwrap_call` at `sqlite_backend.rs:327` — `        let mut b = SqliteVecBackend::open_in_memory().unwrap();`
- 🔴 `unwrap_call` at `sqlite_backend.rs:329` — `        b.set_dimension(4).unwrap();`
- 🔴 `unwrap_call` at `sqlite_backend.rs:332` — `        b.set_dimension(4).unwrap();`
- 🔴 `unwrap_call` at `sqlite_backend.rs:339` — `        let mut b = SqliteVecBackend::open_in_memory().unwrap();`
- 🔴 `unwrap_call` at `sqlite_backend.rs:340` — `        b.set_dimension(3).unwrap();`

### `apeireth-value` (score=210, n_critical=21)

- 🔴 `unwrap_call` at `prioritization.rs:185` — `        let ranks = prioritize_values(&cands).unwrap();`
- 🔴 `unwrap_call` at `prioritization.rs:199` — `        let ranks = prioritize_values(&[a.clone(), b.clone()]).unwrap();`
- 🔴 `unwrap_call` at `prioritization.rs:201` — `        assert_eq!(ranks.last().unwrap().candidate_id, a.id);`
- 🔴 `unwrap_call` at `prioritization.rs:202` — `        assert_eq!(ranks.first().unwrap().candidate_id, b.id);`
- 🔴 `unwrap_call` at `prioritization.rs:209` — `        let ranks = prioritize_values(&[horizon.clone(), imm.clone()]).unwrap();`

### `apeireth-pipeline` (score=175, n_critical=14)

- 🔴 `unwrap_call` at `streaming.rs:124` — `        let c1 = rx.try_recv().unwrap();`
- 🔴 `unwrap_call` at `streaming.rs:125` — `        let c2 = rx.try_recv().unwrap();`
- 🔴 `panic_macro` at `streaming.rs:129` — `            _ => panic!("expected Error"),`
- 🔴 `unwrap_call` at `streaming.rs:156` — `        let c1 = rx.try_recv().unwrap();`
- 🔴 `unwrap_call` at `streaming.rs:157` — `        let c2 = rx.try_recv().unwrap();`

### `apeireth-central` (score=155, n_critical=14)

- 🔴 `unwrap_call` at `lib.rs:871` — `        central.transition_to(LifeStage::Birth).unwrap();`
- 🔴 `unwrap_call` at `lib.rs:872` — `        central.transition_to(LifeStage::Infancy).unwrap();`
- 🔴 `unwrap_call` at `lib.rs:873` — `        central.transition_to(LifeStage::Growth).unwrap();`
- 🔴 `unwrap_call` at `lib.rs:888` — `        central.transition_to(LifeStage::Birth).unwrap();`
- 🔴 `unwrap_call` at `lib.rs:889` — `        central.transition_to(LifeStage::Infancy).unwrap();`

### `apeireth-pybridge` (score=125, n_critical=12)

- 🔴 `unwrap_call` at `lib.rs:100` — `        let info = r11_lookup_module("apeireth.memory.v1141").unwrap();`
- 🔴 `unwrap_call` at `python_bindings.rs:169` — `        assert_eq!(r.unwrap(), "\"hello\"");`
- 🔴 `unwrap_call` at `python_bindings.rs:182` — `        let json = serde_json::to_string(&ep).unwrap();`
- 🔴 `unwrap_call` at `python_bindings.rs:183` — `        let back = py_episode_to_json(&json).unwrap();`
- 🔴 `unwrap_call` at `python_bindings.rs:184` — `        let parsed: Episode = serde_json::from_str(&back).unwrap();`

### `apeireth-consciousness` (score=110, n_critical=11)

- 🔴 `unwrap_call` at `lib.rs:339` — `        m.enter_reflecting().unwrap();`
- 🔴 `unwrap_call` at `lib.rs:340` — `        m.enter_dreaming().unwrap();`
- 🔴 `unwrap_call` at `lib.rs:341` — `        m.enter_meditating().unwrap();`
- 🔴 `unwrap_call` at `lib.rs:342` — `        m.enter_recovering().unwrap();`
- 🔴 `unwrap_call` at `lib.rs:343` — `        m.reset_to_awake().unwrap();`

### `apeireth-asi` (score=100, n_critical=8)

- 🔴 `unwrap_call` at `render.rs:167` — `        assert_eq!(line.chars().next().unwrap(), ' ');`
- 🔴 `unwrap_call` at `llm_judge.rs:168` — `            .unwrap();`
- 🔴 `unwrap_call` at `render.rs:168` — `        assert_eq!(line.chars().last().unwrap(), '▇');`
- 🔴 `unwrap_call` at `render.rs:176` — `        assert_eq!(line.chars().next().unwrap(), ' ');`
- 🔴 `unwrap_call` at `render.rs:177` — `        assert_eq!(line.chars().last().unwrap(), '▇');`

### `apeireth-web` (score=95, n_critical=3)

- 🔴 `unwrap_call` at `main.rs:114` — `    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();`
- 🔴 `unwrap_call` at `main.rs:117` — `        .unwrap();`
- 🔴 `unsafe_block` at `main.rs:409` — `                    unsafe {`
- 🟡 `expect_call` at `sovereignty.rs:292` — `        let guard = state.lock().expect("sovereignty mutex poisoned");`
- 🟡 `expect_call` at `sovereignty.rs:313` — `        let mut guard = state.lock().expect("sovereignty mutex poisoned");`

## Scoring Formula (主 17:43 实事求是 + 主 19:33 走在前人肩上)

```
severity_weight = {critical: 10, important: 5, info: 1}
base_score = Σ(severity_weight × count_per_severity)
governance_bonus = +20 if crate ∈ {sovereignty, upgrade, evolution, asi, council}
unsafe_bonus = +50 if crate has any unsafe block
total_score = base_score + governance_bonus + unsafe_bonus

priority = P0 if total_score >= 100 OR n_unsafe > 0
         P1 if 50 <= total_score < 100
         P2 if 0 < total_score < 50
         OK if total_score == 0 (clean)
```

借鉴 CVSS 风险评分 (severity × volume) + governance 权重 (主 17:43 实事求是).

## Coverage vs V1285

- V1285 审计: 42 crates, 1173 hotspots, 140/210 PASS
- V1286 排序: 42 crates (同 V1285)
- P0: 23 (主 17:43 实事求是: 这些需立即修)
- P1: 9 (本 sprint)
- P2: 4 (本季度)
- OK: 6 (V1285 clean: bench / cli / formal / onion / perception / sdk)

## ASI 5 哲学空隙 + meta-audit + VCP Rust #1-#7 完整闭环

- 时间 (Time): V1276 = 真生产 time falsifier ✓
- 真理 (Truth): V1274 = 真生产 truth falsifier (Popper) ✓
- 识别 (Recognition): V1275 = 真生产 extended falsifier ✓
- 自由 (Freedom): V1277 = 真生产 freedom falsifier ✓
- 涌现 (Emergence): V1278 = 真生产 emergence falsifier ✓
- Meta-Audit: V1279 = 真生产 falsifier self-audit ✓
- VCP Rust 静态: V1280 = 真生产 Rust workspace 静态审计 ✓
- VCP Rust 语义 #1 (technical): V1281 ✓
- VCP Rust 语义 #2 (governance): V1282 ✓
- VCP Rust 语义 #3 (multi-crate): V1283 ✓
- VCP Rust 安全 #1 (worst-5): V1284 ✓ (5b416ce4)
- VCP Rust 安全 #2 (all-42): V1285 ✓ (16f48b94)
- **VCP Rust 安全 #3 (fix priority)**: V1286 = severity-weighted fix priority queue → **本模块, 23 P0 + 9 P1 + 4 P2 + 6 OK**

## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)

- **"Fix Priority Queue" 在此 ≠ "已修完"**: V1286 仅给 P0/P1/P2 + fix 方向
- **audit ≠ fix**: V1286 不真批量替换, 仅给排序 (主 13:31 大胆激进 ≠ 鲁莽)
- **不假装 ASI V1**: 1173 hotspots 仍待修, 不假装"已 ASI V1"
- **不刷 KPI**: PASS rate 不变, 修完 P0 后才能更新 audit (主 22:33)
- **governance 权重是启发式**: 不权威, 仅反映治理核心 (主 17:43 实事求是)
- **V1286 不删 V1285**: V1285 audit 仍独立, V1286 是 action queue
- **P0 阈值 (100) 是经验值**: 可调, 当前基于 1173 hotspots 分布

## V1286 ≠ ASI 收官 (主 19:33 走在前人肩上 + 主 23:44 干到底)

- V1286 = 真生产 fix priority queue, **不是** ASI V1 实现
- 修完 P0/P1/P2 后, V1287+ = 增量监控 (audit 减量, 验证修复)
- ASI ceiling V0.1 = 0.7905 LOCKED (主 22:33), V0.2 = 0.4467, 任何时代最大 0.9800
- 下一站洞察 (主 13:08 + 主 13:31 + 主 19:33): V1287+ = unsafe 块深度 audit / 修复增量监控 / Stage Delivery R21 / 真 benchmark
