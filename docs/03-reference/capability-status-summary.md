# Apeireth v2 · 能力大汇总表（拥有哪些能力 · 什么状态）

[Document-Meta]
Document: docs/03-reference/capability-status-summary.md
Version: 1.0
Last-Modified: 2026-11-08
Class: LEDGER
Status: 🟢 活跃
Author: lead（插件/组件/能力开发线 总指挥）

> **本页 = 机器可再生的能力全景台账**。生成块由 `scripts/gen_capability_summary.py --gen` 重算生成，
> `--check` 守卫判它与真源是否一致（不一致 ⇒ 非 0 退出）⇒ **手改即漂移 ⇒ 漂移必红**。

---

## 0. 口径（四条，先读这一节，后面的表才成立）

| 口径 | 含义 |
|---|---|
| **四级阶梯** | `NOT IMPLEMENTED` < `IMPLEMENTED` < `PRODUCTION WIRED` < `HARDWARE VALIDATED`（承 `docs/01-architecture/system-capabilities.md`） |
| **DEFAULT ENABLED 不是一级** | 它是**横切属性**：一个能力可以「PRODUCTION WIRED 但默认关」⇒ **本表默认列单独标注** |
| **注册 ≠ 可调用** | 「在能力表里」只说明注册面；**授权 / 审批**是另一层（族授权 + 逐次人批） |
| **库级面 ≠ 能力 id** | `guardrail` / `spill` / `object_access` 这类**有实现、无能力 id** ⇒ **不得**当能力宣称 |

### 0.1 三条真源（表里每一格的出处）

| 真源 | 提供什么 |
|---|---|
| `crates/engine/runtime-assembly/src/canonical/tool_taxonomy.rs` | **能力 id 策展表**（生产注册面的权威清单）+ 库级保留面 + 动态族根 |
| `docs/architecture/env-registry.toml` | 每个 `APEIRETH_*` 开关的登记（`default` / `level` / `purpose` / `evidence`） |
| `docs/status/claims.yaml` | 对外承诺台账（`level` / `default` / `switch_env` / `verify_cmd` / `verified_at`） |

### 0.2 怎么复核本页（**守卫**）

```bash
# ① 生成块与真源一致性（表漂了必红）
python3 scripts/gen_capability_summary.py --check      # ⇒ 期望 exit 0
# ② 只看统计（不判成败）
python3 scripts/gen_capability_summary.py --report
# ③ 重算并重写（改能力面后跑）
python3 scripts/gen_capability_summary.py --gen
# ④ 声明真源（claims 级口径）
python3 scripts/docs_truth.py --verify --check         # ⇒ claims=N fail=0 stale=0
```

> **每次生成还会回核四件事**（不一致 ⇒ `--check` 非 0 退出）：
> ① 能力 id **必须在策展表里**（动态族除外）⇒ 新增能力不登记策展表 ⇒ 红；
> ② 开关名**必须在 env 登记表里**且**默认档与登记表一致** ⇒ 口径打架 ⇒ 红；
> ③ 台账等级**必须在四级口径词表内**；
> ④ **证据 file:line 必须仍指向活文件**（文件存在 + 行号在范围内）⇒ 既有面被搬走 ⇒ 红。

---

<!-- BEGIN:generated -->
### ① 生产装配根可达 · 静态能力 id

| # | 能力 id | 模型面工具名 | 所属族 | 四级口径 | 默认 | 开关 | 证据（file:line，回核过） |
|---:|---|---|---|---|:---:|---|---|
| 1 | `tool.filesystem` | filesystem | Filesystem | PRODUCTION WIRED | ON | 无 env 开关（APEIRETH_DISABLE_LOCAL_READ_TOOLS=1 只撤授权，不撤注册） | 装配 production.rs:531-538；默认 production_config.rs:364；组合根 cli/src/lib.rs:2261(..default()) |
| 2 | `tool.search` | search | Search | PRODUCTION WIRED | ON | 同上 | 装配 production.rs:540-546；默认 production_config.rs:365 |
| 3 | `tool.repo` | repo | Repo | PRODUCTION WIRED | ON | 同上 | 装配 production.rs:548-554；默认 production_config.rs:366 |
| 4 | `tool.repo_map` | repo_map | Repo（子族） | PRODUCTION WIRED | ON | 无 env 开关（组合根硬写 repo_map: true） | 装配 production.rs:560-566；组合根 cli/src/lib.rs:2152 |
| 5 | `tool.self_status` | self_status | SelfStatus | PRODUCTION WIRED | ON | 无 env 开关（默认 true，组合根不覆盖） | 装配 production.rs:852-984；默认 production_config.rs:393 |
| 6 | `tool.trait_report` | trait_report | TraitReport | PRODUCTION WIRED | ON | 无 env 开关（组合根恒注入成长账落位） | 装配 production.rs:721-725；组合根 cli/src/lib.rs:2210-2212 |
| 7 | `tool.shell` | shell | Shell | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_SHELL=1（只认 "1"） | 装配 production.rs:645-649；默认 production_config.rs:367(shell: None)；env cli/src/lib.rs:117,2048-2050,2157-2160 |
| 8 | `tool.fetch` | fetch | Fetch | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_FETCH=1 | 装配 production.rs:651-662；默认 production_config.rs:368(fetch: None)；env cli/src/lib.rs:118,2051-2053,2161 |
| 9 | `tool.apply_patch` | apply_patch | ApplyPatch（受控写） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_FILE_WRITE=1 且 工作区根已注入 | 装配 production.rs:573-579；默认 production_config.rs:394(file_write: false)；env cli/src/lib.rs:176,626-630,2183 |
| 10 | `tool.git_write` | git_write | GitWrite | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_GIT_WRITE=1 且工作区根 | 装配 production.rs:604-610；默认 production_config.rs:396；env cli/src/lib.rs:186,668-672,2185 |
| 11 | `tool.worktree_plan` | worktree_plan | WorktreePlan | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_WORKTREE_PLAN=1 且 APEIRETH_ENABLE_FILE_WRITE=1（子依赖）且工作区根 | 装配 production.rs:585-591（config.worktree_plan && config.file_write）；env cli/src/lib.rs:196,686-691,2190 |
| 12 | `tool.worktree_apply` | worktree_apply | WorktreeApply | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_WORKTREE_APPLY=1 且 …WORKTREE_PLAN=1 且 …FILE_WRITE=1 且工作区根 | 装配 production.rs:593-599；env cli/src/lib.rs:197,696-701,2191 |
| 13 | `tool.skill_distill` | skill_distill | SkillDistill | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_SKILL_DISTILL=1 且 …FILE_WRITE=1 且工作区根 且 技能库目录 | 装配 production.rs:616-622；env cli/src/lib.rs:202,706-711,2194-2197 |
| 14 | `tool.skill_search` | skill_search | SkillSearch | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_SKILL_SEARCH=1 且 技能库目录 | 装配 production.rs:624-633；env cli/src/lib.rs:203,714-718,2195-2197 |
| 15 | `tool.image_ops` | image_ops | ImageOps | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_IMAGE_OPS=1 且 …FILE_WRITE=1 且工作区根 | 装配 production.rs:750-757（config.image_ops && config.file_write）；env cli/src/lib.rs:2228 + tools/src/image_ops.rs:107,2094-2098 |
| 16 | `tool.doc_ops` | doc_ops | DocOps | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_DOC_OPS=1（写动作另受 …FILE_WRITE=1） | 装配 production.rs:764-770；env tools/src/doc_ops.rs:131,384-388 + cli/src/lib.rs:2233 |
| 17 | `tool.sheet_ops` | sheet_ops | SheetOps | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_SHEET_OPS=1（写动作另受 …FILE_WRITE=1） | 装配 production.rs:778-784；env tools/src/sheet_ops.rs:142,411-415 + cli/src/lib.rs:2238 |
| 18 | `tool.content_feed` | content_feed | ContentFeed | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_CONTENT_FEED=1 | 装配 production.rs:667-671；默认 production_config.rs:369；env cli/src/lib.rs:123,732-736,2163 |
| 19 | `tool.care_schedule` | care_schedule | CareSchedule | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_CARE=1 | 装配 production.rs:678-682；默认 production_config.rs:370；env cli/src/lib.rs:128,741-745,2167 |
| 20 | `tool.backup_export` | backup_export | BackupExport | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_BACKUP_EXPORT=1 | 装配 production.rs:687-691；默认 production_config.rs:372；env cli/src/lib.rs:132,749-753,1899-1902,2170 |
| 21 | `tool.voice_speak` | voice_speak | VoiceSpeak | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_VOICE_SPEAK=1 | 装配 production.rs:697-704；默认 production_config.rs:371；env cli/src/lib.rs:138,759-763,2175 |
| 22 | `tool.task_board` | task_board | TaskBoard | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_TASK_BOARD=1 | 装配 production.rs:711-715；默认 production_config.rs:373；env cli/src/lib.rs:142,769-773,2179 |
| 23 | `tool.exposure_audit` | exposure_audit | ExposureAudit | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_EXPOSURE_AUDIT=1 | 装配 production.rs:730-734；默认 production_config.rs:404；env cli/src/lib.rs:148,399-418,2216 |
| 24 | `tool.proposal_draft` | proposal_draft | Proposals | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_PROPOSALS=1 | 装配 production.rs:740-743；默认 production_config.rs:405；env tools/src/proposal.rs:69,1134-1137 + cli/src/lib.rs:2221-2223 |
| 25 | `tool.education` | education | Education | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_EDUCATION=1 | 装配 production.rs:636-640；默认 production_config.rs:391(education: false)；env cli/src/lib.rs:3055-3061,2151 |
| 26 | `tool.device_sense` | device_sense | DeviceSense | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_DEVICE_SENSE=1 | 装配 production.rs:791-796；默认 production_config.rs:409；env tools/src/device/sense.rs:74,794-798 + cli/src/lib.rs:2242-2244 |
| 27 | `tool.weather` | weather | Weather | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_DEVICE_SENSE=1（同族同闸） | 装配 production.rs:797-801；env tools/src/device/weather.rs:768-771 + cli/src/lib.rs:2245-2247 |
| 28 | `tool.device_control` | device_control | DeviceControl | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_DEVICE_SENSE=1 且 APEIRETH_ENABLE_DEVICE_CONTROL=1（双闸） | 装配 production.rs:809-815；env tools/src/device/device_control.rs:126,130,145-147,196-204 + cli/src/lib.rs:2251-2254 |
| 29 | `tool.guard_watch` | guard_watch | GuardWatch | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_DEVICE_SENSE=1 且 APEIRETH_ENABLE_GUARD_WATCH=1（双闸） | 装配 production.rs:822-826；env tools/src/device/guard_watch.rs:74,1105-1119 + cli/src/lib.rs:2258-2260 |
| 30 | `tool.play_propose` | play_propose | BioPlay（install_bio_play） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_BIO_PLAY=1 且工作区根 | 装配 production.rs:831-837 → bio_production_wiring.rs:249-267；id 常量 canonical/play_tools.rs:61,70；env play_tools.rs:58 |
| 31 | `tool.play_start` | play_start | BioPlay | PRODUCTION WIRED | OFF | 同上 | 同上；id play_tools.rs:63,72 |
| 32 | `tool.play_finish` | play_finish | BioPlay | PRODUCTION WIRED | OFF | 同上 | 同上；id play_tools.rs:65,74 |
| 33 | `tool.play_abort` | play_abort | BioPlay | PRODUCTION WIRED | OFF | 同上 | 同上；id play_tools.rs:67,76 |
| 34 | `tool.ritual_done` | ritual_done | 仪式面（install_bio_ritual_tools） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1 且 关怀数据目录已注入 | 装配 production.rs:842-847 → bio_production_wiring.rs:281-299；id canonical/ritual_tools.rs:77,82；env care_reminder.rs:255 |
| 35 | `tool.ritual_skip` | ritual_skip | 仪式面 | PRODUCTION WIRED | OFF | 同上 | 同上；id ritual_tools.rs:79,84 |
| 36 | `tool.recon.pack` | re.pack | Recon | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_RECON=1 且工作区根（+ APEIRETH_TOOL_SCOPE） | 装配 production.rs:1023-1029；id tools/src/recon_tool.rs:33；env tools/src/recon/wiring.rs:32,48-50 + cli/src/lib.rs:2204 |
| 37 | `tool.recon.binfmt` | re.binfmt | Recon | PRODUCTION WIRED | OFF | 同上 | production.rs:1023-1029；id recon_tool.rs:278 |
| 38 | `tool.recon.yara` | re.yara | Recon | PRODUCTION WIRED | OFF | 同上 | production.rs:1023-1029；id recon_tool.rs:359 |
| 39 | `tool.recon.net` | re.net.recon | Recon（网络子族） | PRODUCTION WIRED | OFF | 同上（scope 为空 ⇒ 目标全拒） | production.rs:1023-1029；id recon_tool.rs:462；网络子族 id 表 tools/src/recon/wiring.rs（RECON_NETWORK_TOOL_IDS） |
| 40 | `tool.recon.dotnet` | re.dotnet | Recon | PRODUCTION WIRED | OFF | 同上 | production.rs:1023-1029；id recon_tool.rs:586 |
| 41 | `tool.recon.js` | re.js | Recon | PRODUCTION WIRED | OFF | 同上 | production.rs:1023-1029；id recon_tool.rs:694 |
| 42 | `tool.recon.pcap` | re.pcap | Recon | PRODUCTION WIRED | OFF | 同上 | production.rs:1023-1029；id recon_tool.rs:784 |
| 43 | `tool.recon.web` | re.web.probe | Recon（网络子族） | PRODUCTION WIRED | OFF | 同上 | production.rs:1023-1029；id recon_tool.rs:876 |
| 44 | `tool.recon.tls` | re.tls | Recon（网络子族） | PRODUCTION WIRED | OFF | 同上 | production.rs:1023-1029；id recon_tool.rs:969 |
| 45 | `tool.external.nmap` | ext.nmap | ExternalTools | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_EXTERNAL=1 | 装配 production.rs:1034-1038；id tools/src/external.rs:165；env tools/src/recon/wiring.rs:34,53-55 + cli/src/lib.rs:2205 |
| 46 | `tool.external.exiftool` | ext.exiftool | ExternalTools | PRODUCTION WIRED | OFF | 同上 | production.rs:1034-1038；id external.rs:481 |
| 47 | `tool.external.tshark` | ext.tshark | ExternalTools | PRODUCTION WIRED | OFF | 同上 | production.rs:1034-1038；id external.rs:593 |
| 48 | `tool.external.yara` | ext.yara | ExternalTools | PRODUCTION WIRED | OFF | 同上 | production.rs:1034-1038；id external.rs:730 |
| 49 | `tool.external.binwalk` | ext.binwalk | ExternalTools | PRODUCTION WIRED | OFF | 同上 | production.rs:1034-1038；id external.rs:874 |
| 50 | `tool.external.radare2` | ext.radare2 | ExternalTools | PRODUCTION WIRED | OFF | 同上 | production.rs:1034-1038；id external.rs:991 |
| 51 | `tool.external.jadx` | ext.jadx | ExternalTools | PRODUCTION WIRED | OFF | 同上 | production.rs:1034-1038；id external.rs:1156 |
| 52 | `tool.external.apktool` | ext.apktool | ExternalTools | PRODUCTION WIRED | OFF | 同上 | production.rs:1034-1038；id external.rs:1278 |
| 53 | `tool.external.ghidra` | ext.ghidra | ExternalTools | PRODUCTION WIRED | OFF | 同上 | production.rs:1034-1038；id external.rs:1433 |
| 54 | `tool.external.openssl` | ext.openssl | ExternalTools | PRODUCTION WIRED | OFF | 同上 | production.rs:1034-1038；id external.rs:1591 |
| — | **tool.mcp.<server>.<tool>（动态族，逐个外部工具生成；不存在固定 tool.mcp 能力 id）** | mcp:<server>:<tool> | Mcp | IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立（docs/status/claims.yaml:124：2026-11-07 起四环已在真第三方 stdio MCP 服务器端到端成立，但仍有 5 类性质未覆盖 ⇒ 整体口径不成立） | OFF | APEIRETH_ENABLE_MCP=1（+ 服务器列表可加载） | 装配 production.rs:999-1017；id 生成 tools/src/mcp_bridge/catalog.rs:137-143；族根 tools/src/mcp_bridge/family.rs:20；env cli/src/lib.rs:212,2054-2056,2200-2201 |

### ② 有能力 id 形态，但生产装配根**不可达**

| # | 能力 id | 模型面工具名 | 所属族 | 四级口径 | 默认 | 证据（file:line） | 为什么不可达 |
|---:|---|---|---|---|:---:|---|---|
| 1 | `tool.memory.retrieve` | （无模型面名，MemoryTool 内部能力） | MemoryModule | grep -v "/tests/" ⇒ 只命中定义处） | OFF | id 生成 runtime-assembly/src/canonical/memory_module.rs:57；impl ToolCapability :276；全仓无 MemoryModule::new 调用（rg -n "\bMemoryModule\b" crates/ --glob '*.rs' \ | 构造点全仓只在 `tests/` 被调 |
| 2 | `tool.memory.write` | 同上 | MemoryModule | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 同上（OPERATIONS 常量 memory_module.rs:25-27） | 构造点全仓只在 `tests/` 被调 |
| 3 | `tool.memory.update` | 同上 | MemoryModule | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 同上 | 构造点全仓只在 `tests/` 被调 |
| 4 | `tool.memory.delete` | 同上 | MemoryModule | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 同上 | 构造点全仓只在 `tests/` 被调 |
| 5 | `tool.memory.search` | 同上 | MemoryModule | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 同上 | 构造点全仓只在 `tests/` 被调 |
| 6 | `tool.memory.compact` | 同上 | MemoryModule | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 同上 | 构造点全仓只在 `tests/` 被调 |
| 7 | `tool.memory.organize` | 同上 | MemoryModule | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 同上 | 构造点全仓只在 `tests/` 被调 |
| 8 | `tool.mesh.connect` | （MeshTool 内部能力） | Mesh | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立（docs/status/claims.yaml:1094-1110 rt.mesh_local_worker 等级 = IMPLEMENTED、default OFF，与本表一致） | OFF | id 生成 runtime-assembly/src/mesh.rs:26；Kind::suffix :301-307；全仓 MeshModule::new 只在 tests/mesh_execution.rs | 构造点全仓只在 `tests/` 被调 |
| 9 | `tool.mesh.status` | 同上 | Mesh | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 同上 | 构造点全仓只在 `tests/` 被调 |
| 10 | `tool.mesh.exchange` | 同上 | Mesh | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 同上 | 构造点全仓只在 `tests/` 被调 |
| 11 | `tool.process` | 无（库级面，非工具） | Process（库级保留面） | IMPLEMENTED（库级）｜PRODUCTION WIRED 不成立（永不可达）｜HARDWARE VALIDATED 不成立 | 不适用 | 无 ToolCapability 实现（rg -c "impl ToolCapability for" crates/capabilities/tools/src/process/ ⇒ 0）；仅被分类器引用 permission_preset.rs:61；仓内显式登记为「未注册为工具」tool_taxonomy.rs:92-104 | 库级保留面（**无 `ToolCapability` 实现**） |
| 12 | `tool.supervisor` | 无 | Supervisor（库级保留面） | IMPLEMENTED（库级）｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | 不适用 | 同上（tools/src/supervisor.rs 无 ToolCapability）；tool_taxonomy.rs:97-104 | 库级保留面（**无 `ToolCapability` 实现**） |
| 13 | `tool.std_sub_supervisor` | 无 | Supervisor（库级保留面） | IMPLEMENTED（库级）｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | 不适用 | 同上（tools/src/std_sub_supervisor.rs 无 ToolCapability）；tool_taxonomy.rs:95-104 | 库级保留面（**无 `ToolCapability` 实现**） |

### ③ 库级模块面（**有实现、无能力 id ⇒ 不是能力**）

| 库级面 | 用途 | 证据 |
|---|---|---|
| `tool.guardrail` | 前置守门 / 凭据绊线（消费方，不是能力） | tools/src/guardrail.rs（impl ToolCapability for ⇒ 0） |
| `tool.stealth_crawler` | 3 个纯函数，零 IO，未接线 | tools/src/stealth_crawler.rs（无 ToolCapability） |
| `tool.spill` | 会话级有界大文本溢出存储 | tools/src/spill.rs（无 ToolCapability） |
| `tool.object_access` | 对象级访问判定（E-P3） | tools/src/object_access.rs（无 ToolCapability） |
| `tool.redaction_bridge` | 出境脱敏双向闸 | tools/src/redaction_bridge.rs（无 ToolCapability） |
| `tool.write_intent` | shell 侧写意图词法扫描 | tools/src/write_intent.rs（无 ToolCapability） |
| `tool.live_policy` | 进程内共享 live 权限策略 | tools/src/live_policy.rs（无 ToolCapability） |
| `tool.observed_gate` | 读前观测门禁 | tools/src/observed_gate.rs（无 ToolCapability） |
| `tool.egress` | 受控出网策略 | tools/src/egress.rs（无 ToolCapability） |
| `tool.escalation` | 沙箱升级阶梯提示 | tools/src/escalation.rs（无 ToolCapability） |
| `tool.ecosystem_health` | 工具生态健康度内核（纯内核未接线） | tools/src/ecosystem_health.rs（无 ToolCapability） |
| `tool.proposal_push` | 提案推送账 | tools/src/proposal_push.rs（无 ToolCapability） |
| `tool.exec_pipeline` | 五段流水线基础设施 | tools/src/exec_pipeline/（PipelinedCapability 是包装器，不是新能力 id） |
| `tool.evidence_faces` | 证据面 | tools/src/evidence_faces.rs（无 ToolCapability） |
| `tool.accountability_view` | 只读视图 | tools/src/accountability_view.rs（无 ToolCapability） |
| `tool.memory_ledger_view` | 只读视图 | tools/src/memory_ledger_view.rs（无 ToolCapability） |

### ④ 认知 / 行为模块族（**不是 `tool.*` 能力 id**，登记在 `tool.self_status` 名册里）

| 模块 | 默认 | 开关 | 证据 |
|:---:|:---:|---|---|
| `memory_recall` | **ON** | 无（默认 true） | production_config.rs:356；装配 production.rs:1117-1156 |
| `memory_writeback` | **ON** | 无（默认 true） | production_config.rs:359；装配 production.rs:1317-1361 |
| `preference_recall` | **ON** | 无（默认 true） | production_config.rs:360；装配 production.rs:1158-1168 |
| `self_assessment` | **ON** | 无（默认 true） | production_config.rs:361；装配 production.rs:1192-1206 |
| `preference_learning` | **ON（CLI 覆盖层默认开）** | APEIRETH_ENABLE_PREFERENCE_LEARNING=0 关 / APEIRETH_DISABLE_PREFERENCE_LEARNING=1 关（DISABLE 优先） | cli/src/lib.rs:360-368,2137；core_memory_knob_enabled cli/src/lib.rs:272-288 |
| `proactive_recall` | **ON** | APEIRETH_ENABLE_PROACTIVE_RECALL=0 关 / APEIRETH_DISABLE_PROACTIVE_RECALL=1 关 | cli/src/lib.rs:290-296,2138 |
| `memory_injection` | **ON** | APEIRETH_ENABLE_MEMORY_INJECTION=0 关 / APEIRETH_DISABLE_MEMORY_INJECTION=1 关 | cli/src/lib.rs:353-358,2139 |
| `consolidation` | **OFF** | APEIRETH_ENABLE_CONSOLIDATION=1 | cli/src/lib.rs:370-376,2140 |
| `persona_consolidation` | **OFF** | APEIRETH_ENABLE_PERSONA_CONSOLIDATION=1 | cli/src/lib.rs:378-386,2143 |
| `consolidation_hash_gate` | **OFF** | APEIRETH_ENABLE_CONSOLIDATION_HASH_GATE=1 | cli/src/lib.rs:388-395,2146 |
| `reflexion` | **OFF** | APEIRETH_ENABLE_REFLEXION=1 | cli/src/lib.rs:420-426,2147 |
| `partner_bond` | **OFF** | APEIRETH_ENABLE_PARTNER_BOND=1 | cli/src/lib.rs:3031-3037,2148 |
| `morphology_recall` | **OFF** | APEIRETH_ENABLE_MORPHOLOGY_RECALL=1 | cli/src/lib.rs:3039-3045,2149 |
| `community_triage` | **OFF** | APEIRETH_ENABLE_COMMUNITY_TRIAGE=1 | cli/src/lib.rs:3047-3053,2150 |
| `absorption_insight` | **OFF** | APEIRETH_ENABLE_ABSORPTION_INSIGHT=1 | cli/src/lib.rs:3063-3070,2153 |
| `judge` | **OFF** | APEIRETH_COGNITIVE_JUDGE=1 | cli/src/lib.rs:92,2038-2040,2131-2134 |
| `council` | **OFF** | APEIRETH_COGNITIVE_COUNCIL=1 | cli/src/lib.rs:93,2041-2043,2135 |
| `organs` | **OFF** | APEIRETH_ENABLE_ORGANS=1 | cli/src/lib.rs:213,2044-2046,2136 |
| `bio_play（4 个工具面）` | **OFF** | APEIRETH_ENABLE_BIO_PLAY=1 | production.rs:430,831-837；nervous_switchboard.rs:51-99 |
| `bio_care_candidates（2 个仪式工具面）` | **OFF** | APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1 | production.rs:431,842-847；nervous_switchboard.rs:51-99 |

### ⑤ 对外承诺台账（`docs/status/claims.yaml`）

| 等级 | 条数 |
|---|---:|
| PRODUCTION WIRED | 38 |
| IMPLEMENTED | 12 |
| NOT IMPLEMENTED | 1 |
| **合计** | **51** |

### ⑥ 统计块

| 指标 | 值 |
|---|---:|
| 生产可达静态能力 id | **54** |
| 动态能力族 | **1**（`tool.mcp.<server>.<tool>`，成员数随运行期发现而变） |
| 默认 ON | **6** |
| 默认 OFF | **48** |
| 有 id 但生产不可达 | **13** |
| 无能力 id 的库级模块面 | **16** |
| 认知 / 行为模块族 | **20** |
| 对外承诺（claims） | **51** |
| **HARDWARE VALIDATED 达成** | **0** |
| 策展表条目（`TOOL_TAXONOMY`） | **57** |
<!-- END:generated -->

---

## 附：与既有文档的分工（防两边口径混淆）

| 文档 | 回答什么 | 与本页的关系 |
|---|---|---|
| 本页 | **拥有哪些能力 · 每条什么状态**（全景台账） | — |
| [`docs/status/claims.yaml`](../status/claims.yaml) | **对外承诺到哪一级**（可核验声明，精选非穷举） | 承诺口径以 claims 为准；本页是**全景**，claims 是**承诺** |
| [`docs/03-reference/capabilities-matrix.md`](capabilities-matrix.md) | 能力域 × 契约 × 实现模块的**速查矩阵**（含契约细节与边界说明） | 本页给**全景 + 状态**，矩阵给**契约细节** |
| [`docs/status/workline-registry.md`](../status/workline-registry.md) | 工作线 ↔ 设计稿 ↔ 进度页 ↔ 守卫 | 指向「未来要造什么」；本页是「现在有什么」 |
