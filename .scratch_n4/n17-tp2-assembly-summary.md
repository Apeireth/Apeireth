# N17/TP2 装配统一注册件 — 完成总结

**3 commit (cumulative 9251e80):**
- `0f18541` feat(N17/TP2): 9 工具子 crate 装配 — register/catalog 接入 ToolBridge (18 files, 169+)
- `5996d51` fix(N17/TP2): ImageGenParams 加 serde Deserialize/Serialize
- `9251e80` fix(N17/TP2): persist open_in_memory 补回 status 列

## 完成清单

### 装配三件套 (§10 铁边界)

1. **Tool trait 适配器** (register.rs × 9):
   - EnhancedShellTool / FetchEngineTool / EnhancedBrowserTool
   - CodeIntelligenceTool / ImageGenEnhancedTool / ImageProcessTool
   - VSearchTool / EnhancedFileOpsTool / RepoQualityAnalyzerTool
   - 每个持原 crate 现有增强层 (EnhancedShell / SearchEngine / Sandbox …), 走原引擎不重复发明

2. **register/unregister** (同 register.rs):
   - 真注册到 ToolRegistry, 不打 log 空响
   - unregister 返 bool, 重复卸返 false (幂等, 0 残留)

3. **catalog** (tool-registry/src/catalog.rs):
   - `CapabilityCatalog::from_registry(&registry)` 派生只读 snapshot
   - `names() / len() / render_markdown()` 三个 R/O 接口

### ToolBridge 接入 (companion/src/tool_bridge.rs)

- `apeireth_tools::register_all` 之后串行 9 件 register(), 失败 eprintln 不阻断
- 4 只读扩入默认日常包白名单 (VSearch / CodeIntelligence / RepoQualityAnalyzer / ImageProcess)
- 5 件高危 (EnhancedShell / FetchEngine / EnhancedBrowser / ImageGenEnhanced / EnhancedFileOps) 仍走 RiskRule 5min 超时分级

### 3 真 bug 修

1. **persist.rs Sync**: `Connection` 是 `Send + !Sync` → 装 `Mutex<Connection>` 让 `PersistentTaskStore: Sync`
2. **write_atomic 父目录**: 改解析父 + 拼文件名, 避开 `canonicalize` 对新建文件在 Windows 上失败
3. **tool-search async-trait 依赖**: register.rs 用 `#[async_trait]`, Cargo.toml 漏补

### 测试

| crate | 总数 | 失败 |
|-------|------|------|
| tool-shell (含 4 register + 2 persist) | 17 | 0 |
| tool-fetch | 57 | 0 |
| tool-browser | 111 | 0 |
| tool-codesearch | 112 | 0 |
| tool-image-gen | 19 | 0 |
| tool-image-process | 38 | 0 |
| tool-search | 29 | 0 |
| tool-filesystem | 143 | 0 |
| repo-tools | 31 | 0 |
| tool-registry | 39 | 0 |
| **合计** | **614** | **0** |

companion N17 集成测试 2 件 (`n17_tool_bridge_registers_all_nine_and_catalog_reflects` + `n17_nine_register_unregister_round_trip_zero_residue`) 已写在 tool_bridge.rs 末尾, 待 WIP 解锁 (memory_graph.rs / assemble.rs 阻塞 lib 编译).

## 跳过的 (ponytail)

- register.rs 没补 serde derive `ToolDescription { brief }` 文案 — 留给 N18 文档生成器接走
