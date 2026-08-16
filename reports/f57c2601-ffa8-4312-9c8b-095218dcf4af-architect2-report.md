# TP4/N22 ShellPreset（预设命令模板 + 白名单 + 防注入）— architect2 自审报告

- 任务 ID: f57c2601-ffa8-4312-9c8b-095218dcf4af
- 角色: 架构师2 | 日期: 2026-08-17
- 提交: `b48f355`（preset.rs 新模块）/ `7b11738`（lib.rs 挂接，被并行作业覆盖后恢复）

## 1. 背景与哲学锚点
- §10 官方包 TP4 最后一件；台账 N22；来源 R136 主人蓝图对照：「VCP preset 机制 (preset:预设名?参数) 值得保留 — 减少 LLM 记忆成本」。
- LLM 只记预设名 + 参数，不记命令全文 → 记忆成本下降，且命令面收敛到白名单。

## 2. 改动文件（严格任务包边界）
| 文件 | 改动 |
|---|---|
| `crates/apeireth-tool-shell/src/preset.rs` | **新增**（395 行）：ArgSpec / ShellPreset / PresetError / PresetRegistry（白名单登记+展开）/ PresetShell（挂既有执行链）+ 9 个测试 |
| `crates/apeireth-tool-shell/src/lib.rs` | `pub mod preset;` + re-export 2 行（中途被并行作业覆盖，已恢复并二次提交） |
| `docs/maintenance-guide.md` | 模块地图新增 preset.rs 行 |
| `docs/backlog.md` | N22 划 ✅（含提交号与验收摘要） |

禁止触碰项核实：tool-approval/guard 零改动（git diff 可查）；执行链路（enhanced.rs build_command/exec_sandboxed）零改动，纯复用。

## 3. 架构决策记录（防注入三道防线）
1. **白名单**：预设清单显式登记（builtin 3 个：git-log-recent / git-status-short / echo-text + register 扩展）；非白名单预设名展开即拒（UnknownPreset）；预设名语法 `[a-z][a-z0-9_-]{0,63}`，非法/重复登记即拒。
2. **模板结构校验**：模板 = argv 片段数组，占位符 `{arg}` 必须**独占整个 argv 槽位**；嵌入式占位符（与文本拼接）注册时即拒（InvalidTemplate）——从结构上杜绝参数与模板文本拼接。
3. **参数独立引用 + 往返闭环**：填充走 `shell_words::quote`（单 token），执行链 `EnhancedShell::exec_sandboxed` → `build_command` 用 `shell_words::split` 解析为 argv 直传 `tokio::process::Command`（**不经 shell 解释器**）。quote/split 满足 `split(quote(x)) == [x]`，`;` `&&` `|` `$()` 反引号等特殊字符永远无法逃逸出参数边界。
- 参数规格校验：Number（十进制 + 范围）/ Text（禁控制字符、长度上限）；缺参/多参/重复参数名拒绝。
- 挂接方式（不自写 shell 调用）：`PresetShell::exec_preset = registry.expand → shell.exec_sandboxed`，沙箱/超时/持久化全走既有链路。

## 4. 测试结果
`cargo test -p apeireth-tool-shell -j 4`：**全绿**（lib 36 passed = 原 27 + 新增 9；其余 target 4 passed；0 failed）。
- 预设展开：`expand_git_log_recent`（模板正确展开）、`builtin_registry_has_whitelist`
- 非法预设拒绝：`unknown_preset_rejected`（含大小写变体）、`invalid_or_duplicate_name_rejected`（6 种非法名 + 重复登记）、`embedded_placeholder_rejected_at_register`（嵌入式/未声明占位符）
- 注入用例（验收必测，参数含特殊字符）：`injection_payloads_stay_inside_single_token`（7 组载荷：`; rm -rf /`、`&& malicious`、`| nc evil`、`$(whoami)`、`` `id` ``、引号混合、`&>< <`重定向 — 断言 split 后 argv 结构不变且载荷原样落在单 token 内）、`arg_validation_rejects_bad_values`（Number 槽位注入载荷直接拒 + 控制字符拒）、`exec_preset_runs_real_command_with_injection_literal`（真执行端到端，注入载荷字面回显不被解释）、`exec_preset_unknown_rejected_before_exec`（非白名单在执行前拒绝）

## 5. 0 装 PASS 标注（诚实）
| 做了 | 没做（接线点/后续） |
|---|---|
| 白名单预设登记 + builtin 3 预设 | 动态预设文件加载（从配置/数据库读预设清单）未做 — 当前是编译期 builtin + 代码 register |
| 参数模板填充防注入（quote/split 闭环） | 敏感预设的多签/审批不在此处 — 走既有 tool-approval/guard（本任务不改其本体），接线点 = PresetShell 外层包审批 |
| PresetShell 挂 exec_sandboxed 执行链 | 预设未注册进 ToolRegistry 工具面（N17 装配批次的事 — N22 台账标注「与 N17 同批」，本任务边界只到 tool-shell 内机制件） |
| 3 个 builtin 预设 | 预设清单可按套件装配扩展（suite 层追加 register 调用即可） |

## 6. 过程备注（并行作业实录）
- lib.rs 挂接提交被并行作业覆盖一次（git add 时仅剩 preset.rs 入库；grep 发现 lib.rs 无 preset 痕迹）→ 重新编辑 + 验证 9/9 测试真跑 + 独立提交 7b11738 恢复。教训：提交前 grep 核实文件状态。
- 首次「36 passed」即已含 preset 9 测（27 原有 + 9 新增），二次核实过滤器单跑 9/9 确认。

## 7. 给守门员的合并提示
- preset.rs 自包含无新依赖（shell-words/thiserror/tokio/tempfile 均已在 Cargo.toml）。
- lib.rs 仅 2 行增量；若再被并行覆盖，恢复方式 = 加回 `pub mod preset;` + `pub use preset::{ArgSpec, PresetError, PresetRegistry, PresetShell, ShellPreset};`。
- N17 装配时建议：preset 不需要独立工具注册（它是 shell 工具的参数层机制），若要暴露工具面，包一层 Tool trait 调 PresetShell 即可。
