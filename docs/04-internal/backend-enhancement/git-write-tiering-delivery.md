# D-P1「git 写分级上架」交付记录（分级词表全表 + 接线图 + 测试证据）

> `Status: ✅ 已交付（D 轴 P1，后端批，零前端触碰）`
> 施工纪律：`CONSTRUCTION-DISCIPLINE.md` 全款遵守（零债 / 叠加不覆盖 / 口径诚实 / 门禁全过 / 对抗测试）。
> 来源纪律：代码、注释、测试名、文案全中性工程表述；`scripts/check-neutral-terms.ps1` 零命中。

## 1. 交付物一览

| 面 | 内容 | 位置 |
|---|---|---|
| 工具面 | `tool.git_write`（声明名 `git_write`）：分级词表 + 词法分级 + 受控通道执行 | `crates/capabilities/tools/src/git_write.rs` |
| 治理面 | 分级风险钩子（L1 档位 / L2 永人批 / L3 拒绝即帧）+ 授权映射 + 单调边界 Guard | 同上（`GitWriteTierApprovalHook` / `GitWriteRiskMappingHook` / `GitWriteBoundaryGuard`） |
| 预设面 | `read_only` 拒 / `standard` 审批 / `full` L1 放行·L2 照审批（**L2 地板：永不因预设放行**） | `crates/engine/runtime-assembly/src/canonical/permission_preset.rs`（`human_approval_floor_applies`） |
| 装配面 | `GitWriteModule` + `ProductionModulesConfig.git_write / git_write_auto_pass_l1` + 自省名册两行 | `crates/engine/runtime-assembly/src/canonical/{tool_modules,production,self_status_source}.rs` |
| 描述面 | `tool.git_write` 入 canonical 能力描述（写面，L3 无工具面如实 destructive=false） | `crates/engine/guard/src/semantics.rs` |
| 旋钮面 | `APEIRETH_ENABLE_GIT_WRITE` / `APEIRETH_ENABLE_GIT_WRITE_AUTO_PASS_L1`（子档依赖主开关，fail-closed） | `crates/adapters/cli/src/lib.rs`（`GitWriteRiskLevel` 三档） |
| 文案面 | `GIT_WRITE_BOUNDARY_NOTE` 改分级语义（「L3 永无工具」保留）+ user-manual / api / 能力矩阵 / 开关审计表同步 | `apply_patch_tool.rs` + `docs/02-guides/user-manual.md` + `docs/03-reference/api.md` 等 |
| 脱敏面 | 秘密面脱敏原语（凭据 URL / 密钥赋值 / 凭据形状键值）+ 审批载荷全链脱敏 | `crates/foundation/governance/src/input_security.rs`（`redact_secret_surfaces`）+ `crates/engine/runtime/src/canonical/approval.rs` |

## 2. 分级词表全表（写意图分级）

**总则**：命令文本只认 git 调用段（首词 `git`），段间只认 `&&` / `;` / 换行；**组合命令逐段分级，整次取最严**（一段 L2 = 整次 L2；一段 L3/词表外/变形面 = 整次拒绝即帧）。长选项按**前缀重叠**命中（`--ha` ≡ `--hard` 缩写、`--force-with-lease=x` 撞 `--force` 族），短选项束**逐字符**查（`-qf` 里的 `f` 也算强推）。

| 级 | 子命令（词表） | 允许形态 | 升级为更严档 / 拒绝即帧的变体 |
|---|---|---|---|
| **L1 可逆** | `git add` | 任意路径/选项（add 无毁灭面） | —— |
| **L1 可逆** | `git commit` | 必须显式带消息（`-m`/`-F`/`--message`/`--file`/`--reuse-message`/`--fixup`/`--squash`）；不开编辑器面 | `--amend` 族（缩写 `--am` 同拒）→ **L3 改史**；`-n`/`--no-verify` → **变形面拒**；`-s`/`-S`/`--sign`/`--gpg-sign` → 签名/密钥面拒；`-c`（开编辑器）拒 |
| **L1 可逆** | `git branch`（建） | 位置参 1-2 个（名字 [起点]）；选项只认 `-t/--track/--no-track/-q/--quiet/--` | `-d`/`-D`/`--delete` → **L3 引用删除族**（≡ `update-ref -d`）；`-f`/`--force` → **L3 强制覆盖**；列表/改名/其它 → 词表外拒 |
| **L1 可逆** | `git switch`（切） | 1 个分支名，或 `-c/--create <new>`（建并切）；`-t/--no-track/-q/--quiet` | `-C`/`--force-create` → **L3**（重置既有分支）；`--discard-changes` → **L3**（丢工作区变更）；`-d`/`--detach` → 词表外拒 |
| **L1 可逆** | `git tag`（本地建） | 名字 [commit]（1-2 个位置参）；`-a/-m/-F/--annotate/--message/--file/-q/--quiet` | `-d`/`-D`/`--delete` → **L3 引用删除族**；`-f`/`--force` → **L3 强制覆盖**；`-s`/`--sign`/`-u`/`--local-user` → 签名/密钥面拒 |
| **L2 出网** | `git push` | 非强推推送（`--all/--tags/-u/--quiet/--verbose/--dry-run` 等） | `--force` 族（含 `--force-with-lease`/`--force-if-includes`/缩写）→ **L3 强推**；refspec `+x` → **L3 强推变体**；`--delete`/`-d`/空侧 `:x`/`x:` → **L3 远端引用删除变体**；`--mirror`/`--exec`/`--receive-pack` → **L3**；`-o`/`--push-option`、`--no-verify` → 变形面拒；`--signed`/`--gpg-sign` → 签名面拒 |
| **L2 出网** | `git fetch` | 常规抓取（`--all/--tags/--quiet` 等） | `--force`/`-f` → **L3 强制覆盖**；`--prune`/`--prune-tags` → **L3 引用删除族**；`--upload-pack`/`--exec` → **L3 远端命令执行**；refspec 空侧 `:` → **L3 引用删除变体**；refspec 目的地 `refs/heads/*`/`refs/tags/*`/`HEAD` → **L3 本地引用覆盖变体** |
| **L2 出网** | `git remote` | 仅 `add` / `rename` / `set-url` / `update`（远端操作面） | `remove`/`rm`/`prune` → **L3 引用删除族**；`--mirror` → **L3**（推送即删除的镜像面）；查询面（`show`/`get-url`/`list`）→ 词表外拒（走 repo 工具） |
| **L3 毁灭（无工具面）** | `reset` / `clean` / `rebase` / `update-ref` / `filter-branch` / `filter-repo` / `gc` / `prune` / `reflog` / `replace` | **拒绝即帧**（`L3 毁灭级拒绝即帧 (无工具面)`） | 全部形态（含 `reset --ha`、`clean -fdx`、`git update-ref -d` 等变形）一律同帧 |
| **词表外（无工具面）** | 其余子命令（`checkout`/`restore`/`merge`/`stash`/`config`/`status`/别名…） | **拒绝即帧**（`词表外拒绝即帧 (无工具面)`）；`checkout`/`restore` 帧内指路 `switch` | —— |
| **变形面（无工具面）** | 危险全局选项 / shell 面 | 子命令前只放行 `--no-pager`/`--paginate`；`-c`/`--config-env`/`--exec-path`/`--git-dir`/`--work-tree`/`--namespace`/`-C`/任何未列全局选项拒绝即帧；管道/重定向/`$`/反引号/分组括号/单 `&`/引号未闭合拒绝即帧 | 命令按结构化参数执行 git，**不经 shell**；引号内字面量（如 `-m "$(literal)"`）按参数保留、不做插值 |

**执行语义（如实）**：逐段顺序落地（`ProcessExecutor` 受控通道：结构化 argv、`git -C <工作区根>` 限域、`ProcessLimits::default()` 30s 超时归 `timeout.*` 族、输出 64K 截断归一）；**中段失败不谎称回滚** —— 错误帧逐段报告「已落地段保留 / 失败段 / 失败因」（staged 残留照实留存）。**脱敏**：命令面、输出、审批理由、审批卡命令文本全链过 `redact_secret_surfaces`（凭据 URL / 密钥赋值 / 凭据形状键值换占位，远端 URL 凭据不入日志）。

**风险档位**：L1 默认 **RequireApproval（每次人批）**；`APEIRETH_ENABLE_GIT_WRITE_AUTO_PASS_L1=1` 开「自动放行 L1」档（只放 L1）；**L2 永远人工审批**（无自动放行档，会话预设/免审记忆都不得越人）；**L3 永远无工具面**。

## 3. 接线图

```text
模型调用 git_write(command)
   │
   ▼
GitWriteTool (tool.git_write)
   ├─ classify_git_write 词法分级
   │    ├ split_command_segments：&& / ; / 换行切段；管道/重定向/$/`/分组/单 & → 变形面拒
   │    ├ tokenize_segment：引号成组（引号内字面，无插值）
   │    ├ grade_segment：git 起词 + 全局选项白名单（仅 --no-pager/--paginate）
   │    │                + 子命令词表 + 选项/束/refspec 变形防护（前缀重叠/逐字符）
   │    └ 逐段取最严 → Graded(L1|L2) 或 Refused(L3|词表外|变形面，拒绝即帧)
   │
   ├─ 五段流水线 git_write_pipeline
   │    pre    : GitWriteRiskMappingHook   授权映射（未授权停 require-approval 档）
   │    Guard  : GitWriteBoundaryGuard     单调边界（只拒不放，后阶段不得翻回）
   │    around : 30s 超时（timeout.* 族）  post: 透传
   │    contract: 输出归一（status/tier/commands/output 四字段冻结）
   │
   └─ execute_plan：ProcessExecutor 受控通道（结构化 argv / git -C <root> 限域 /
      无 shell / 逐段顺序落地 / 失败如实帧：已落地段 + 失败段 + 失败因，无回滚承诺）

治理链（组装根 build_production_governance_parts_with_dataset）
   PermissionGovernanceHook（grant = 工具面有人拍板；未开主开关 → 拒）
   → GitWriteTierApprovalHook（分级风险映射：L1 按档审批/放行、L2 永远审批、L3 拒绝即帧）
   → CredentialDisclosure / PromptInjection / BehaviorChainGuard（内容风险层）
外裹 PermissionPresetGovernanceHook（会话预设）
   ├ read_only ：tool.git_write ∈ 写/执行名单 → 拒（拒绝即帧，零审批实体）
   ├ standard  ：照 inner（L1/L2 都审批）；approval_remember 免审记忆**不越过 L2 地板**
   └ full      ：RequireApproval→Allow 改写，但 human_approval_floor_applies（L2）拦住
                 ——「L2 永不因预设放行」；L1 放行且真落地
审批实体（同一链）：PendingApproval 视图（command_text 全链脱敏 + 分级标注理由）
   → 本地审批面板 / IM 审批卡（手机上批 push，但每一次 push 都要人批）→ 四态闭合
```

## 4. 测试证据（≥12，全部新增钉行为）

| # | 测试（文件） | 钉住的行为 |
|---|---|---|
| 1 | `git_write_grading.rs::l1_word_table_grades_reversible_local_commands_and_variants` | L1 词表逐命令 + 变体（引号路径 / 附加消息值 / 组合短选项 / 长选项等值形态） |
| 2 | `git_write_grading.rs::l2_word_table_grades_network_commands_and_variants` | L2 词表逐命令 + 变体 + L2 地板判定（L1 不踩地板） |
| 3 | `git_write_grading.rs::destructive_and_disguised_forms_are_refused_with_the_l3_frame` | L3 矩阵 30+ 形态：`reset --hard` 变形（`-q` 组合 / `--ha` 缩写 / `=true`）/ `clean -f` 组合 / 强推族（`-qf`、`--force-with-lease`、`+refspec`）/ 改史（`--am`）/ 引用删除族 |
| 4 | `git_write_grading.rs::off_list_subcommands_have_no_tool_surface` | 词表外拒绝即帧（checkout/restore/merge/config/别名/空命令…） |
| 5 | `git_write_grading.rs::combined_commands_take_the_strictest_segment` | 组合命令逐段取最严（L1+L2→L2；含 L3 段→整次拒；管道/非 git 段→整次拒） |
| 6 | `git_write_grading.rs::dangerous_global_options_and_shell_shapes_are_refused` | 参数变体防绕过：`git -c alias` 族 / `--exec-path` / `--git-dir` / `--work-tree` / `--namespace` / `-C` 全拒；`$()`/反引号/重定向/单 `&` 全拒；`--no-pager`/`--paginate` 仅有的全局面 |
| 7 | `git_write_grading.rs::refusal_frames_are_stable_and_the_description_states_l3_has_no_tool_surface` | 三类拒绝帧稳定前缀 + 工具描述写明 L3 无工具面（防幻觉） |
| 8 | `git_write_grading.rs::destructive_invocations_never_touch_the_repository` | L3 伪装尝试（reset/clean/amend/force-push）拒绝即帧 + 仓库零变化 |
| 9 | `git_write_grading.rs::l1_execution_lands_add_and_commit_through_the_controlled_channel` | 真 git：add+commit 组合真落地，报告四字段归一 |
| 10 | `git_write_grading.rs::failed_segment_reports_landed_segments_without_rollback_claim` | 原子性如实语义：commit 失败 → 帧报告「第 2/2 段失败 + 已落地段保留」，staged 残留照实留存，不谎称回滚 |
| 11 | `git_write_grading.rs::remote_url_credentials_are_redacted_in_frames_plans_and_decisions` | 脱敏：remote URL 凭据不入命令面/审批理由/执行输出 |
| 12 | `git_write_grading.rs::tier_approval_hook_maps_the_three_release_semantics` | L1 三档语义（默认审批 / 自动放行放行）+ L2 两档都审批 + L3 拒绝不进审批 + 钩子不越权 |
| 13 | `git_write_grading.rs::boundary_guard_refuses_refused_plans_and_passes_graded_ones` | 单调边界 Guard 只拒不放 |
| 14 | `git_write_preset_semantics.rs::read_only_preset_refuses_the_git_write_tool` | read_only 档拒绝即帧：零提交、零审批实体 |
| 15 | `git_write_preset_semantics.rs::standard_preset_freezes_l1_at_a_tier_annotated_approval_card` | standard 档冻结待批；**审批卡载荷 = 命令文本 + 分级标注（L1/reversible-local）**；批准后真产生提交 |
| 16 | `git_write_preset_semantics.rs::approval_card_redacts_remote_url_credentials_and_labels_l2` | 审批卡脱敏 + L2 分级标注（手机上批 push 的同一张卡） |
| 17 | `git_write_preset_semantics.rs::full_preset_releases_l1_and_the_commit_lands` | full 档 L1 放行且真落地 |
| 18 | `git_write_preset_semantics.rs::l2_never_auto_passes_under_the_full_preset` | **L2 永远审批（full 档也不放）**，理由写明 never auto-passes |
| 19 | `git_write_preset_semantics.rs::l2_approval_memory_never_skips_the_human` | 免审记忆（approval_remember）也不越过 L2 地板 |
| 20 | `git_write_preset_semantics.rs::git_write_registers_only_when_enabled_and_reports_roster_switches` | 装配接线（opt-in / 无根不注册）+ 自省名册两开关如实 |
| 21 | `git_write_knob.rs`（cli）4 测 | 旋钮→档位映射（子档依赖主开关 fail-closed）/ 默认档全审批（理由带分级标注）/ 自动放行 L1 只放 L1 / 未开主开关未授权 |
| 22 | `file_write_knob.rs::docs_state_the_graded_git_write_boundary`（演进） | 边界文案更新：user-manual + api 与 `GIT_WRITE_BOUNDARY_NOTE` 同文（「none exists」按「L3 永无工具」保留） |
| 23 | `apply_patch_write.rs::git_write_boundary_is_stated_in_the_tool_description`（演进） | 边界短语演进：graded / L1 / L2 / L3 / always need a human decision / no tool / none exists |
| 24 | `approval.rs::approval_payloads_redact_remote_url_credentials`（runtime） | 审批载荷（命令文本 + 摘要）脱敏（先脱敏后截断） |
| 25 | `semantics.rs::structured_command_surface_merges_real_effects_without_the_shell_baseline`（guard） | 结构化调用面不套 shell 基线（受控 git 调用 ≠ shell 执行面），但真实效应照常合并（push→Publish/NetworkSend 仍进守卫观察） |
| 26 | `input_security.rs` 3 测（governance） | `redact_secret_surfaces`：凭据 URL/密钥赋值/凭据形状键值换占位、键名保留、无秘密面零误伤 |
| 27 | `permission_preset.rs::human_approval_floor_covers_only_l2_git_write_calls` | L2 地板只对 `tool.git_write` 的 L2 调用生效 |

断言演进留痕：`git_write_boundary_is_stated_in_the_tool_description` / `docs_state_the_graded_git_write_boundary` 两条旧断言（「git 写一律无工具」短语）按分级语义逐条换新短语，理由注明在测试注释（宪章「演进断言时注明理由」）。

## 5. 跨范围最小改动（报备 + 说明）

| 文件 | 改动 | 理由 |
|---|---|---|
| `crates/engine/runtime/src/canonical/approval.rs` | `approval_command_text` / `approval_arguments_summary` 成品文本过 `redact_secret_surfaces`（先脱敏后截断） | 「remote URL 凭据不入日志」的唯一收口面：审批卡 / 审批面板 / 审计记录同源；全工具受益，两条既有断言不受影响 |
| `crates/engine/guard/src/semantics.rs` | 命令效应合并对结构化调用面（`tool.git_write`）剔除 shell 基线效应；canonical 描述 + id 新增 | ① 否则受控 git 调用被误标 shell 执行面，「自动放行 L1」档名不实；② `tool.git_write` 入 canonical 描述（写面）为治理接线要求 |
| `crates/adapters/cli/src/{lib,onboarding}.rs` | git 写两旋钮 + 分级钩子接线 + 名册配置行 + 引导文案一处 | 旋钮面接线（与 file_write 旋钮同款结构） |
| `docs/02-guides/{user-manual,capability-switch-power-audit}.md`、`docs/03-reference/{api,capabilities-matrix}.md` | 分级语义同步 + 开关审计行 + 工具表行 | 文档随行（宪章第四节第 5 条） |

## 6. 门禁（宪章第四节）结果（全部 `-j 2`）

| 门禁 | 命令 | 结果 |
|---|---|---|
| 工作区测试 | `cargo test --workspace --locked -j 2` | ✅ 全绿 exit 0；套件 **171（批前基线）→ 177**：本批 +3 套件（`git_write_grading` 16 测 / `git_write_preset_semantics` 7 测 / `git_write_knob` 4 测）+ 6 测散入既有套件（`input_security` ×3 / `approval` ×1 / `semantics` ×1 / `permission_preset` ×1）= 33 新测；另 +3 套件来自共树他线在途改动（非本批文件）。**0 failed —— 既有 171 套件零回归** |
| clippy root | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 0 警告（exit 0） |
| clippy src-tauri（双 workspace 之二） | `frontend/companion-desktop/src-tauri` 下 `cargo clippy --all-targets --locked -j 2 -- -D warnings` | ✅ 0 警告（exit 0） |
| 格式 | `cargo fmt --all -- --check` | ✅ 干净 |
| 前端 | `pnpm check`（svelte-check）/ `pnpm test` | ✅ 0 errors 0 warnings / **71 of 71 suites passed** |
| 中性表述 | `pwsh -NoProfile -File scripts/check-neutral-terms.ps1` | ✅ 零命中 |
| secret-scan | `pwsh -NoProfile -File scripts/secret-scan.ps1`（含 `-Mode scan-all`） | ✅ No secrets found |

> 共树说明：本批与并行施工线共用工作树；`Cargo.lock` 为共享收敛点（并行线的 `fs-err` 依赖边入锁由 cargo 收敛，`--locked` 门禁复验一致）。本批零触碰禁区（frontend/**、CHANGELOG.md、`.github/workflows/companion-desktop-ci.yml`、`docs/04-internal/HANDOFF-NOTES.md`、`docs/design/**`、`crates/engine/memory/{reflexion,river_topology}.rs`）；gateway 零触碰。
