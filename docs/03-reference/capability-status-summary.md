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

> 另有三处**实现级**真源被逐格回核：生产装配根
> `crates/engine/runtime-assembly/src/canonical/production.rs`（可达性 + 装配点行号）、
> 配置默认档 `production_config.rs`、能力名册 `self_status_roster.rs`（行为面开关全表）。

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
> ① 表内每一格**都能在真源里指到出处**（策展表 / 装配点 / env 登记表 / 台账）；
> ② 开关名**必须在 env 登记表里**且**默认档与登记表一致** ⇒ 口径打架 ⇒ 红；
> ③ 台账等级**必须在四级口径词表内**；
> ④ **证据 file:line 必须仍指向活文件**（文件存在 + 行号在范围内）⇒ 既有面被搬走 ⇒ 红。
>
> 另有一类**不判红**的提示：策展表自身与生产面的出入（漏登 / 幽灵条目）。
> 那不是本表的漂移（表已如实反映现状），守卫在 Rust 侧
> `cargo test -p apeireth-runtime-assembly --lib tool_taxonomy`。

---

<!-- BEGIN:generated -->
### ① 生产装配根可达 · 静态能力 id

| # | 能力 id | 模型面工具名 | 所属族/域 | 四级口径 | 默认 ON/OFF | 开关名 | 证据（file:line，回核过） | 真源出处 |
|---:|---|---|---|:---:|:---:|---|---|---|
| 1 | `tool.apply_patch` | apply_patch | apply_patch（域：filesystem） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_FILE_WRITE | 注册 crates/capabilities/tools/src/apply_patch_tool.rs:50；装配 production.rs:646 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_FILE_WRITE；装配根 production.rs:646 |
| 2 | `tool.backup_export` | （未在源中派生） | backup_export（域：document） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/backup.rs:70；装配 production.rs:811 | 策展表 tool_taxonomy.rs；装配根 production.rs:811 |
| 3 | `tool.calendar` | calendar | calendar（域：companion） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_CALENDAR | 注册 crates/capabilities/tools/src/calendar/mod.rs:113；装配 production.rs:757 | 策展表 tool_taxonomy.rs；台账 claims.yaml#tool.calendar；env 登记表 APEIRETH_ENABLE_CALENDAR；装配根 production.rs:757 |
| 4 | `tool.care_schedule` | care_schedule | care_schedule（域：companion） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_CARE | 注册 crates/engine/organ/src/care/tool.rs:40；装配 production.rs:802 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_CARE；装配根 production.rs:802 |
| 5 | `tool.content_feed` | content_feed | content_feed（域：network） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/content_feed/tool.rs:93；装配 production.rs:791 | 策展表 tool_taxonomy.rs；装配根 production.rs:791 |
| 6 | `tool.device_control` | device_control | device_control（域：device） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_DEVICE_SENSE | 注册 crates/capabilities/tools/src/device/device_control.rs:116；装配 production.rs:935 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_DEVICE_SENSE；装配根 production.rs:935 |
| 7 | `tool.device_sense` | device_sense | device_sense（域：device） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_DEVICE_SENSE | 注册 crates/capabilities/tools/src/device/sense.rs:59；装配 production.rs:916 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_DEVICE_SENSE；装配根 production.rs:916 |
| 8 | `tool.doc_ops` | doc_ops | doc_ops（域：document） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_DOC_OPS | 注册 crates/capabilities/tools/src/doc_ops.rs:123；装配 production.rs:890 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_DOC_OPS；装配根 production.rs:890 |
| 9 | `tool.education` | education | education（域：document/companion） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/education.rs:302；装配 production.rs:708 | 策展表 tool_taxonomy.rs；装配根 production.rs:708 |
| 10 | `tool.exposure_audit` | exposure_audit | exposure_audit（域：governance） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/exposure_audit/tool.rs:22；装配 production.rs:854 | 策展表 tool_taxonomy.rs；装配根 production.rs:854 |
| 11 | `tool.external.apktool` | ext.apktool | external（域：filesystem/compute） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/external.rs:1278；装配 production.rs:1160 | 策展表 tool_taxonomy.rs；装配根 production.rs:1160 |
| 12 | `tool.external.binwalk` | ext.binwalk | external（域：filesystem/compute） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/external.rs:874；装配 production.rs:1160 | 策展表 tool_taxonomy.rs；装配根 production.rs:1160 |
| 13 | `tool.external.exiftool` | ext.exiftool | external（域：filesystem） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/external.rs:481；装配 production.rs:1160 | 策展表 tool_taxonomy.rs；装配根 production.rs:1160 |
| 14 | `tool.external.ghidra` | ext.ghidra | external（域：filesystem/compute） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/external.rs:1433；装配 production.rs:1160 | 策展表 tool_taxonomy.rs；装配根 production.rs:1160 |
| 15 | `tool.external.jadx` | ext.jadx | external（域：filesystem/compute） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/external.rs:1156；装配 production.rs:1160 | 策展表 tool_taxonomy.rs；装配根 production.rs:1160 |
| 16 | `tool.external.nmap` | ext.nmap | external（域：network） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/external.rs:165；装配 production.rs:1160 | 策展表 tool_taxonomy.rs；装配根 production.rs:1160 |
| 17 | `tool.external.openssl` | ext.openssl | external（域：network） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/external.rs:1591；装配 production.rs:1160 | 策展表 tool_taxonomy.rs；装配根 production.rs:1160 |
| 18 | `tool.external.radare2` | ext.radare2 | external（域：filesystem/compute） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/external.rs:991；装配 production.rs:1160 | 策展表 tool_taxonomy.rs；装配根 production.rs:1160 |
| 19 | `tool.external.tshark` | ext.tshark | external（域：filesystem/network） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/external.rs:593；装配 production.rs:1160 | 策展表 tool_taxonomy.rs；装配根 production.rs:1160 |
| 20 | `tool.external.yara` | ext.yara | external（域：filesystem/search） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/external.rs:730；装配 production.rs:1160 | 策展表 tool_taxonomy.rs；装配根 production.rs:1160 |
| 21 | `tool.fetch` | fetch | fetch（域：network） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/fetch.rs:210；装配 production.rs:723 | 策展表 tool_taxonomy.rs；装配根 production.rs:723 |
| 22 | `tool.filesystem` | filesystem | filesystem（域：filesystem） | PRODUCTION WIRED | ON | （无 env 开关） | 注册 crates/capabilities/tools/src/filesystem.rs:84；装配 production.rs:605 | 策展表 tool_taxonomy.rs；装配根 production.rs:605 |
| 23 | `tool.git_write` | git_write | git_write（域：filesystem/process） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_GIT_WRITE | 注册 crates/capabilities/tools/src/git_write.rs:86；装配 production.rs:677 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_GIT_WRITE；装配根 production.rs:677 |
| 24 | `tool.guard_watch` | guard_watch | guard_watch（域：governance） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_GUARD_WATCH | 注册 crates/capabilities/tools/src/device/guard_watch.rs:60；装配 production.rs:946 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_GUARD_WATCH；装配根 production.rs:946 |
| 25 | `tool.http_write` | http_write | http_write（域：network） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_HTTP_WRITE | 注册 crates/capabilities/tools/src/http_write.rs:345；装配 production.rs:740 | 策展表 tool_taxonomy.rs；台账 claims.yaml#tool.http_write；env 登记表 APEIRETH_ENABLE_HTTP_WRITE；装配根 production.rs:740 |
| 26 | `tool.image_ops` | image_ops | image_ops（域：media） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_IMAGE_OPS | 注册 crates/capabilities/tools/src/image_ops.rs:101；装配 production.rs:876 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_IMAGE_OPS；装配根 production.rs:876 |
| 27 | `tool.object_grant` | object_grant | object_grant（域：memory/governance） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/engine/runtime-assembly/src/canonical/object_grant.rs:63；装配 production.rs:782 | 策展表 tool_taxonomy.rs；台账 claims.yaml#tool.object_grant；装配根 production.rs:782 |
| 28 | `tool.play_abort` | play_abort | play_abort（域：companion） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_BIO_PLAY | 注册 crates/engine/runtime-assembly/src/canonical/play_tools.rs:67；装配 production.rs:949 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_BIO_PLAY；装配根 production.rs:949 |
| 29 | `tool.play_finish` | play_finish | play_finish（域：companion） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_BIO_PLAY | 注册 crates/engine/runtime-assembly/src/canonical/play_tools.rs:65；装配 production.rs:949 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_BIO_PLAY；装配根 production.rs:949 |
| 30 | `tool.play_propose` | play_propose | play_propose（域：companion） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_BIO_PLAY | 注册 crates/engine/runtime-assembly/src/canonical/play_tools.rs:61；装配 production.rs:949 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_BIO_PLAY；装配根 production.rs:949 |
| 31 | `tool.play_start` | play_start | play_start（域：companion） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_BIO_PLAY | 注册 crates/engine/runtime-assembly/src/canonical/play_tools.rs:63；装配 production.rs:949 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_BIO_PLAY；装配根 production.rs:949 |
| 32 | `tool.process_tree` | process_tree | process_tree（域：process） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_SHELL | 注册 crates/capabilities/tools/src/process_tree_tool.rs:71；装配 production.rs:768 | 策展表 tool_taxonomy.rs；台账 claims.yaml#tool.process_tree；env 登记表 APEIRETH_ENABLE_SHELL；装配根 production.rs:768 |
| 33 | `tool.proposal_draft` | proposal_draft | proposal_draft（域：governance） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_PROPOSALS | 注册 crates/capabilities/tools/src/proposal.rs:49；装配 production.rs:863 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_PROPOSALS；装配根 production.rs:863 |
| 34 | `tool.recon.binfmt` | re.binfmt | recon（域：filesystem/compute） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/recon_tool.rs:257；装配 production.rs:1151 | 策展表 tool_taxonomy.rs；装配根 production.rs:1151 |
| 35 | `tool.recon.dotnet` | re.dotnet | recon（域：filesystem/compute） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/recon_tool.rs:586；装配 production.rs:1151 | 策展表 tool_taxonomy.rs；装配根 production.rs:1151 |
| 36 | `tool.recon.js` | re.js | recon（域：filesystem/compute） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/recon_tool.rs:694；装配 production.rs:1151 | 策展表 tool_taxonomy.rs；装配根 production.rs:1151 |
| 37 | `tool.recon.net` | re.net.recon | recon（域：network） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/recon_tool.rs:462；装配 production.rs:1151 | 策展表 tool_taxonomy.rs；装配根 production.rs:1151 |
| 38 | `tool.recon.pack` | re.pack | recon（域：filesystem/compute） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/recon_tool.rs:33；装配 production.rs:1151 | 策展表 tool_taxonomy.rs；装配根 production.rs:1151 |
| 39 | `tool.recon.pcap` | re.pcap | recon（域：filesystem/network） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/recon_tool.rs:784；装配 production.rs:1151 | 策展表 tool_taxonomy.rs；装配根 production.rs:1151 |
| 40 | `tool.recon.tls` | re.tls | recon（域：network） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/recon_tool.rs:969；装配 production.rs:1151 | 策展表 tool_taxonomy.rs；装配根 production.rs:1151 |
| 41 | `tool.recon.web` | re.web.probe | recon（域：network） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/recon_tool.rs:876；装配 production.rs:1151 | 策展表 tool_taxonomy.rs；装配根 production.rs:1151 |
| 42 | `tool.recon.yara` | re.yara | recon（域：filesystem/search） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/recon_tool.rs:359；装配 production.rs:1151 | 策展表 tool_taxonomy.rs；装配根 production.rs:1151 |
| 43 | `tool.repo` | repo | repo（域：filesystem/search） | PRODUCTION WIRED | ON | （无 env 开关） | 注册 crates/capabilities/tools/src/plugin.rs:106；装配 production.rs:621 | 策展表 tool_taxonomy.rs；装配根 production.rs:621 |
| 44 | `tool.repo_map` | repo_map | repo_map（域：filesystem/compute） | PRODUCTION WIRED | ON | （无 env 开关） | 注册 crates/capabilities/tools/src/repo_map_tool.rs:51；装配 production.rs:633 | 策展表 tool_taxonomy.rs；装配根 production.rs:633 |
| 45 | `tool.ritual_done` | ritual_done | ritual_done（域：companion） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/engine/runtime-assembly/src/canonical/ritual_tools.rs:77；装配 production.rs:1423 | 策展表 tool_taxonomy.rs；装配根 production.rs:1423 |
| 46 | `tool.ritual_skip` | ritual_skip | ritual_skip（域：companion） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/engine/runtime-assembly/src/canonical/ritual_tools.rs:79；装配 production.rs:1423 | 策展表 tool_taxonomy.rs；装配根 production.rs:1423 |
| 47 | `tool.search` | search | search（域：search） | PRODUCTION WIRED | ON | （无 env 开关） | 注册 crates/capabilities/tools/src/plugin.rs:95；装配 production.rs:613 | 策展表 tool_taxonomy.rs；装配根 production.rs:613 |
| 48 | `tool.self_status` | self_status | self_status（域：self） | PRODUCTION WIRED | ON | （无 env 开关） | 注册 crates/capabilities/tools/src/self_status.rs:2159；装配 production.rs:1107 | 策展表 tool_taxonomy.rs；装配根 production.rs:1107 |
| 49 | `tool.sheet_ops` | sheet_ops | sheet_ops（域：document） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_SHEET_OPS | 注册 crates/capabilities/tools/src/sheet_ops.rs:134；装配 production.rs:904 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_SHEET_OPS；装配根 production.rs:904 |
| 50 | `tool.shell` | shell | shell（域：process） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/plugin.rs:126；装配 production.rs:717 | 策展表 tool_taxonomy.rs；装配根 production.rs:717 |
| 51 | `tool.skill_distill` | skill_distill | skill_distill（域：memory） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_SKILL_DISTILL | 注册 crates/capabilities/tools/src/skill/distill.rs:70；装配 production.rs:689 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_SKILL_DISTILL；装配根 production.rs:689 |
| 52 | `tool.skill_search` | skill_search | skill_search（域：search/memory） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_SKILL_DISTILL | 注册 crates/capabilities/tools/src/skill/search.rs:26；装配 production.rs:700 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_SKILL_DISTILL；装配根 production.rs:700 |
| 53 | `tool.task_board` | task_board | task_board（域：governance） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/engine/runtime-assembly/src/canonical/task_board_tool.rs:44；装配 production.rs:835 | 策展表 tool_taxonomy.rs；台账 claims.yaml#tool.task_board；装配根 production.rs:835 |
| 54 | `tool.trait_report` | trait_report | trait_report（域：self/governance） | PRODUCTION WIRED | ON | （无 env 开关） | 注册 crates/engine/runtime-assembly/src/canonical/trait_report_tool.rs:44；装配 production.rs:845 | 策展表 tool_taxonomy.rs；装配根 production.rs:845 |
| 55 | `tool.voice_speak` | voice_speak | voice_speak（域：media） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/voice/speak.rs:83；装配 production.rs:824 | 策展表 tool_taxonomy.rs；台账 claims.yaml#tool.voice_speak；装配根 production.rs:824 |
| 56 | `tool.weather` | weather | weather（域：network） | PRODUCTION WIRED | OFF | （无 env 开关） | 注册 crates/capabilities/tools/src/device/weather.rs:45；装配 production.rs:921 | 策展表 tool_taxonomy.rs；装配根 production.rs:921 |
| 57 | `tool.web_search` | web_search | web_search（域：network/search） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_WEB_SEARCH | 注册 crates/capabilities/tools/src/web_search.rs:87；装配 production.rs:749 | 策展表 tool_taxonomy.rs；台账 claims.yaml#tool.web_search；env 登记表 APEIRETH_ENABLE_WEB_SEARCH；装配根 production.rs:749 |
| 58 | `tool.worktree_apply` | worktree_apply | worktree_apply（域：filesystem） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_WORKTREE_APPLY | 注册 crates/capabilities/tools/src/worktree_apply.rs:87；装配 production.rs:666 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_WORKTREE_APPLY；装配根 production.rs:666 |
| 59 | `tool.worktree_plan` | worktree_plan | worktree_plan（域：filesystem） | PRODUCTION WIRED | OFF | APEIRETH_ENABLE_WORKTREE_PLAN | 注册 crates/capabilities/tools/src/worktree_plan.rs:36；装配 production.rs:658 | 策展表 tool_taxonomy.rs；env 登记表 APEIRETH_ENABLE_WORKTREE_PLAN；装配根 production.rs:658 |
| — | **`tool.mcp.<server>.<tool>`**（动态族，逐个外部工具生成） | mcp:<server>:<tool> | Mcp | IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立 | OFF | APEIRETH_ENABLE_MCP | 族根 tool_taxonomy.rs | 策展表 tool_taxonomy.rs（DYNAMIC_FAMILY_ROOTS） |

### ② 有能力 id 形态，但生产装配根**不可达**

| # | 能力 id | 模型面工具名 | 所属族/域 | 四级口径 | 默认 ON/OFF | 开关名 | 证据（file:line） | 为什么不可达 | 真源出处 |
|---:|---|---|---|:---:|:---:|---|---|---|---|
| 1 | `tool.process` | 无（库级面，非工具） | Process（库级保留面） | IMPLEMENTED（库级）｜PRODUCTION WIRED 不成立（永不可达）｜HARDWARE VALIDATED 不成立 | 不适用 | 无（策略分类器按 id 形态引用） | crates/capabilities/tools/src/process/mod.rs:360 | 库级保留面（**无 `ToolCapability` 实现**） | 策展表 tool_taxonomy.rs（LIBRARY_LEVEL_FACES） |
| 2 | `tool.std_sub_supervisor` | 无（库级面，非工具） | Supervisor（库级保留面） | IMPLEMENTED（库级）｜PRODUCTION WIRED 不成立（永不可达）｜HARDWARE VALIDATED 不成立 | 不适用 | 无（策略分类器按 id 形态引用） | crates/capabilities/tools/src/std_sub_supervisor.rs:46 | 库级保留面（**无 `ToolCapability` 实现**） | 策展表 tool_taxonomy.rs（LIBRARY_LEVEL_FACES） |
| 3 | `tool.supervisor` | 无（库级面，非工具） | Supervisor（库级保留面） | IMPLEMENTED（库级）｜PRODUCTION WIRED 不成立（永不可达）｜HARDWARE VALIDATED 不成立 | 不适用 | 无（策略分类器按 id 形态引用） | crates/capabilities/tools/src/supervisor.rs:158 | 库级保留面（**无 `ToolCapability` 实现**） | 策展表 tool_taxonomy.rs（LIBRARY_LEVEL_FACES） |
| 4 | `tool.memory.compact` | compact（运行期生成） | tool.memory（运行期生成） | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | id 生成 crates/engine/runtime-assembly/src/canonical/memory_module.rs:57 | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | 运行期拼装 + 装配根无构造点 |
| 5 | `tool.memory.delete` | delete（运行期生成） | tool.memory（运行期生成） | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | id 生成 crates/engine/runtime-assembly/src/canonical/memory_module.rs:57 | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | 运行期拼装 + 装配根无构造点 |
| 6 | `tool.memory.organize` | organize（运行期生成） | tool.memory（运行期生成） | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | id 生成 crates/engine/runtime-assembly/src/canonical/memory_module.rs:57 | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | 运行期拼装 + 装配根无构造点 |
| 7 | `tool.memory.retrieve` | retrieve（运行期生成） | tool.memory（运行期生成） | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | id 生成 crates/engine/runtime-assembly/src/canonical/memory_module.rs:57 | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | 运行期拼装 + 装配根无构造点 |
| 8 | `tool.memory.search` | search（运行期生成） | tool.memory（运行期生成） | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | id 生成 crates/engine/runtime-assembly/src/canonical/memory_module.rs:57 | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | 运行期拼装 + 装配根无构造点 |
| 9 | `tool.memory.update` | update（运行期生成） | tool.memory（运行期生成） | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | id 生成 crates/engine/runtime-assembly/src/canonical/memory_module.rs:57 | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | 运行期拼装 + 装配根无构造点 |
| 10 | `tool.memory.write` | write（运行期生成） | tool.memory（运行期生成） | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | id 生成 crates/engine/runtime-assembly/src/canonical/memory_module.rs:57 | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | 运行期拼装 + 装配根无构造点 |
| 11 | `tool.mesh.connect` | connect（运行期生成） | tool.mesh（运行期生成） | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | id 生成 crates/engine/runtime-assembly/src/mesh.rs:26 | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | 运行期拼装 + 装配根无构造点 |
| 12 | `tool.mesh.exchange` | exchange（运行期生成） | tool.mesh（运行期生成） | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | id 生成 crates/engine/runtime-assembly/src/mesh.rs:26 | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | 运行期拼装 + 装配根无构造点 |
| 13 | `tool.mesh.status` | status（运行期生成） | tool.mesh（运行期生成） | IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | id 生成 crates/engine/runtime-assembly/src/mesh.rs:26 | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | 运行期拼装 + 装配根无构造点 |

### ③ 库级模块面（**有实现、无能力 id ⇒ 不是能力**）

| 库级面 | 所属族/域 | 用途 | 证据（file:line） | 真源出处 |
|---|---|---|---|---|
| `accountability_view` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/accountability_view.rs:28（无 ToolCapability 实现） | tools crate `pub mod accountability_view` |
| `apply_patch` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/apply_patch.rs:37（无 ToolCapability 实现） | tools crate `pub mod apply_patch` |
| `ecosystem_health` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/ecosystem_health.rs:111（无 ToolCapability 实现） | tools crate `pub mod ecosystem_health` |
| `egress` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/egress.rs:35（无 ToolCapability 实现） | tools crate `pub mod egress` |
| `escalation` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/escalation.rs:34（无 ToolCapability 实现） | tools crate `pub mod escalation` |
| `evidence_faces` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/evidence_faces.rs:61（无 ToolCapability 实现） | tools crate `pub mod evidence_faces` |
| `guardrail` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/guardrail.rs:36（无 ToolCapability 实现） | tools crate `pub mod guardrail` |
| `live_policy` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/live_policy.rs:43（无 ToolCapability 实现） | tools crate `pub mod live_policy` |
| `mcp` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/mcp.rs:22（无 ToolCapability 实现） | tools crate `pub mod mcp` |
| `memory_ledger_view` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/memory_ledger_view.rs:23（无 ToolCapability 实现） | tools crate `pub mod memory_ledger_view` |
| `object_access` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/object_access.rs:65（无 ToolCapability 实现） | tools crate `pub mod object_access` |
| `observed_gate` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/observed_gate.rs:39（无 ToolCapability 实现） | tools crate `pub mod observed_gate` |
| `proposal_push` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/proposal_push.rs:39（无 ToolCapability 实现） | tools crate `pub mod proposal_push` |
| `recon` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/recon:1（无 ToolCapability 实现） | tools crate `pub mod recon` |
| `redaction_bridge` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/redaction_bridge.rs:88（无 ToolCapability 实现） | tools crate `pub mod redaction_bridge` |
| `repo_map` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/repo_map.rs:19（无 ToolCapability 实现） | tools crate `pub mod repo_map` |
| `spill` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/spill.rs:52（无 ToolCapability 实现） | tools crate `pub mod spill` |
| `stealth_crawler` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/stealth_crawler.rs:22（无 ToolCapability 实现） | tools crate `pub mod stealth_crawler` |
| `write_intent` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | crates/capabilities/tools/src/write_intent.rs:34（无 ToolCapability 实现） | tools crate `pub mod write_intent` |

### ④ 认知 / 行为模块族（**不是 `tool.*` 能力 id**，登记在 `tool.self_status` 名册里）

| 模块 | 所属族/域 | 默认 ON/OFF | 开关名 | 证据（file:line） | 真源出处 |
|:---:|---|:---:|---|---|---|
| `absorption_insight` | 行为面（无 `tool.*` 能力 id） | OFF | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:206 | 名册 self_status_roster.rs |
| `community_triage` | 行为面（无 `tool.*` 能力 id） | OFF | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:202 | 名册 self_status_roster.rs |
| `consolidation` | 行为面（无 `tool.*` 能力 id） | OFF | APEIRETH_ENABLE_CONSOLIDATION | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:171 | 名册 self_status_roster.rs |
| `consolidation_hash_gate` | 行为面（无 `tool.*` 能力 id） | OFF | APEIRETH_ENABLE_CONSOLIDATION_HASH_GATE | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:185 | 名册 self_status_roster.rs |
| `council` | 行为面（无 `tool.*` 能力 id） | OFF | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:192 | 名册 self_status_roster.rs |
| `file_write_auto_pass` | 行为面（无 `tool.*` 能力 id） | OFF | APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:93 | 名册 self_status_roster.rs |
| `git_write_auto_pass_l1` | 行为面（无 `tool.*` 能力 id） | OFF | APEIRETH_ENABLE_GIT_WRITE_AUTO_PASS_L1 | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:116 | 名册 self_status_roster.rs |
| `judge` | 行为面（无 `tool.*` 能力 id） | OFF | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:191 | 名册 self_status_roster.rs |
| `memory_injection` | 行为面（无 `tool.*` 能力 id） | ON | APEIRETH_ENABLE_MEMORY_INJECTION | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:154 | 名册 self_status_roster.rs |
| `memory_recall` | 行为面（无 `tool.*` 能力 id） | ON | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:152 | 名册 self_status_roster.rs |
| `memory_writeback` | 行为面（无 `tool.*` 能力 id） | ON | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:153 | 名册 self_status_roster.rs |
| `morphology_recall` | 行为面（无 `tool.*` 能力 id） | OFF | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:196 | 名册 self_status_roster.rs |
| `organs` | 行为面（无 `tool.*` 能力 id） | OFF | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:193 | 名册 self_status_roster.rs |
| `partner_bond` | 行为面（无 `tool.*` 能力 id） | OFF | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:194 | 名册 self_status_roster.rs |
| `persona_consolidation` | 行为面（无 `tool.*` 能力 id） | OFF | APEIRETH_ENABLE_PERSONA_CONSOLIDATION | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:178 | 名册 self_status_roster.rs |
| `preference_learning` | 行为面（无 `tool.*` 能力 id） | OFF | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:168 | 名册 self_status_roster.rs |
| `preference_recall` | 行为面（无 `tool.*` 能力 id） | ON | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:167 | 名册 self_status_roster.rs |
| `proactive_recall` | 行为面（无 `tool.*` 能力 id） | OFF | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:158 | 名册 self_status_roster.rs |
| `reflexion` | 行为面（无 `tool.*` 能力 id） | OFF | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:189 | 名册 self_status_roster.rs |
| `self_assessment` | 行为面（无 `tool.*` 能力 id） | ON | （无 env 开关） | crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:190 | 名册 self_status_roster.rs |

### ⑤ 对外承诺台账（`docs/status/claims.yaml`）

| 等级 | 条数 |
|---|---:|
| PRODUCTION WIRED | 45 |
| IMPLEMENTED | 12 |
| NOT IMPLEMENTED | 1 |
| **合计** | **58** |

### ⑥ 统计块

| 指标 | 值 |
|---|---:|
| 生产可达静态能力 id | **59** |
| 动态能力族 | **1**（成员数随运行期发现而变） |
| 默认 ON | **6** |
| 默认 OFF | **53** |
| 有 id 但生产不可达 | **13** |
| 无能力 id 的库级模块面 | **19** |
| 认知 / 行为模块族 | **20** |
| 对外承诺（claims） | **58** |
| **HARDWARE VALIDATED 达成** | **0** |
| 策展表条目（`TOOL_TAXONOMY`） | **62** |
<!-- END:generated -->

---

## 附：与既有文档的分工（防两边口径混淆）

| 文档 | 回答什么 | 与本页的关系 |
|---|---|---|
| 本页 | **拥有哪些能力 · 每条什么状态**（全景台账） | — |
| [`docs/status/claims.yaml`](../status/claims.yaml) | **对外承诺到哪一级**（可核验声明，精选非穷举） | 承诺口径以 claims 为准；本页是**全景**，claims 是**承诺** |
| [`docs/03-reference/capabilities-matrix.md`](capabilities-matrix.md) | 能力域 × 契约 × 实现模块的**速查矩阵**（含契约细节与边界说明） | 本页给**全景 + 状态**，矩阵给**契约细节** |
| [`docs/status/workline-registry.md`](../status/workline-registry.md) | 工作线 ↔ 设计稿 ↔ 进度页 ↔ 守卫 | 指向「未来要造什么」；本页是「现在有什么」 |
