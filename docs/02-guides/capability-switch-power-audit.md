# 开关权力审计表（能力开关名实相符核查）

> 核查命题：**开关名 = 开关权**。每个能力开关的「声称范围」与「实际范围」必须一致——
> 摆设开关（名称管的事实际没管住）与幽灵权力（名称外的实际权力）都是病灶。
> 本表为内测整改（文件写入总闸批）的核查产出，逐行给出处置；行内证据指向真实代码坐标。
> 复查机制：`frontend/companion-desktop/tests/switch-power-audit.mjs` 锁本表结构与行覆盖，
> `frontend/companion-desktop/tests/switch-scope-copy.mjs` 锁全责文案，Rust 侧写意图/总闸
> 测试锁行为（见各行「锁定测试」列）。

## 一、权力面开关审计

| 开关名（env / 配置字段） | 声称范围 | 实际范围（核查结论） | 处置 |
|---|---|---|---|
| `APEIRETH_ENABLE_FILE_WRITE`（file_write） | 受控文件写入 | **整改前有洞**：只管 `tool.apply_patch` 的注册/授权/风险档位；shell 的 `echo xxx > file` 等写命令不经此闸照样落盘（开关名与权力不符）。**整改后 = 唯一写总闸**：apply_patch 三档风险映射 + shell 写意图词法面（重定向 `>`/`>>`/`N>`/`<>`、删除/移动/复制/改名/建目录族、脚本引擎等值命令面）关闸即拒绝即帧（`pipeline.pre_deny`），冻结时与执行时双查 | **已整改**：写意图扫描（`crates/capabilities/tools/src/write_intent.rs`）+ shell 总闸接线（`shell.rs`）+ 组装根同源注入（`production.rs`）；词法面之外的写入（解释器内写入/编码负载/解包工具）由 shell 人工审批链 + 沙箱兑底（明示边界，见 `write_intent.rs` 白名单注释） |
| `APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS`（file_write_auto_pass） | 自动放行已读文件的修改 | 一致：仅**修改类**补丁免逐次审批；创建/删除永不自动放行（仍进人工审批四态闭合）；依赖主开关（单独设=无效果）；未读文件仍被读前观测门禁拒绝 | **保持**（名实相符） |
| `APEIRETH_ENABLE_SHELL`（shell） | Shell 命令工具开关 | 一致（注册 + grant + require_approval，每次调用人工审批）；但 shell 工具的权力由三个开关分层：注册/授权（本开关）、沙箱姿态（`APEIRETH_SHELL_SANDBOX`）、写权力（file_write 总闸）——单说"shell 开关"不等于 shell 全部行为 | **保持 + 描述补全**：设置页 shell 行明示"写命令另受文件写入总闸管辖"（开关分层如实声明） |
| `APEIRETH_SHELL_SANDBOX`（shellSandbox） | shell 沙箱开关 | 一致：开 = 文件限定工作区 + 断网隔离（平台不可实施即拒绝执行，绝不裸跑）；关 = 显式裸跑。只决定隔离要求集，不涉注册/授权/审批 | **保持** |
| `APEIRETH_ENABLE_FETCH`（fetch） | 网络读取工具开关 | 一致：注册 + grant + require_approval；工具内受控出站（目的校验/本地回环拒绝）。**核查问题"fetch 管不管住 fetch？"→ 管住**：未注册即无网络权力；其余网络面归属清晰——外部工具桥的网络行为归 `APEIRETH_ENABLE_MCP` 辖，爬取引擎是库代码、未注册为能力（模型无此权力面） | **保持**（网络面归属入档） |
| `APEIRETH_ENABLE_MCP`（mcp） | 外部工具桥开关 | 一致：装配外部工具桥 + 动态注册外部工具；动态工具经桥接治理映射进授权/风险/拒绝守卫，未知能力默认不授权（fail-closed） | **保持** |
| `APEIRETH_ENABLE_LOCAL_READ_TOOLS` / `APEIRETH_DISABLE_LOCAL_READ_TOOLS`（localReadTools） | 本地只读工具（file / search / repo） | **小口不符**：开关只决定 `tool.filesystem` / `tool.search` 的 grant；`tool.repo` **恒 grant 无开关**（只读合同）。声称含 repo、实际管两件 | **已整改（描述对齐）**：设置页 desc 改为「file / search 两件随开关；repo 恒授只读」；行为面维持只读合同不动 |
| （无开关）`tool.repo` | —— | 只读 git 仓库探查，恒注册（默认）+ 恒 grant；写操作不提供工具（设计边界） | **保持**（表内注明无开关事实） |
| （无开关）`tool.self_status` | —— | 自省只读面，恒注册 + 恒 grant + 零审批；只读取自身状态计数 | **保持** |
| `APEIRETH_ENABLE_EDUCATION`（education） | 换元检查工具开关 | **反向缺口**：开 = 注册 `tool.education`，但默认治理授权面未随开（无对应 grant），开关开了权力未必到手（会话权限档可另授） | **挂账**（本轮不扩权：授权面扩张需单独评审；已在本表如实登记） |

## 二、不属工具权力面的开关（核查后划出本表）

认知/治理层开关（`APEIRETH_ENABLE_ORGANS` / `PREFERENCE_LEARNING` / `COGNITIVE_JUDGE` / `COGNITIVE_COUNCIL` /
`ENABLE_ONION_LAYER` / 记忆核心族 / `WORKTREE_SANDBOX` 等）不向模型授予任何工具执行权力，
属认知模块装配或治理层收紧旋钮，不产生"开关名 ≠ 开关权"形态，不入权力审计面。

## 三、全责文案锚点（同一句话三处落点，测试锁定）

**「文件写入总闸：apply_patch 与 shell 写命令同受此闸」** 落在：

1. 前端能力开关描述（`frontend/companion-desktop/src/lib/views/SettingsView.svelte`，fileWrite 行）；
2. env 注释（`crates/adapters/cli/src/lib.rs`，`ENABLE_FILE_WRITE_ENV` 与 `file_write_enabled_from_env`）；
3. 用户手册（`docs/02-guides/user-manual.md`，tool.apply_patch 节；`docs/03-reference/api.md` 工具表同口径）。

拒绝帧文案（shell 写命令撞总闸时的原话）：
**「文件写入开关未开——shell 写命令受同一总闸管辖」**（帧 code `pipeline.pre_deny`，拒绝信息即帧）。

## 四、行级锁定测试指针

| 审计行 | 锁定测试 |
|---|---|
| file_write（唯一写总闸） | `crates/capabilities/tools/src/write_intent.rs` 扫描矩阵（重定向/删除类/管道链/白名单误拒注释）、`shell.rs` 写闸四测（关闸拒绝即帧/开闸行为不变/只读零误伤/冻结载荷复核）、`crates/engine/runtime-assembly/tests/file_write_registration.rs` 组装根接线 |
| file_write_auto_pass | `crates/adapters/cli/tests/file_write_knob.rs`（三档风险映射） |
| shell / fetch / mcp / localReadTools | `crates/adapters/cli/tests/production_knobs.rs`、`crates/engine/runtime-assembly/tests/canonical_shell_approval_e2e.rs`、`canonical_fetch_e2e.rs`、`mcp_bridge_runtime_e2e.rs` |
| 全责文案三落点 | `frontend/companion-desktop/tests/switch-scope-copy.mjs` |
| 本表入档（结构 + 行覆盖） | `frontend/companion-desktop/tests/switch-power-audit.mjs` |
