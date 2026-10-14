# E-P2 审批族归一 · 星港线接入契约

> `Status: ✅ 已交付（E 分轴 P2，威胁模型 G2 销账批）`
> 一事：命令级审批从**逐字命令**升级为**语义族匹配**——审批/授权记录的对象是**族**，
> 本文是星港线两件（舵室智能体驱动 / 深望智能体放行）的**接入契约**。
> 纪律：[CONSTRUCTION-DISCIPLINE.md](CONSTRUCTION-DISCIPLINE.md)（进场第一课）；
> 上游：[E-threat-model.md](E-threat-model.md)（G2 缺口）、[E-security-depth.md](E-security-depth.md)（E-P2 事项）。
> 诚实条款：族词表是**冻结词表**（新增族/族词 = 契约变更，须过本文 §2 全表口径）；
> 边界逐条如实，词法面之外的形态不假称覆盖。

## 1. 病灶与整改口径（G2 → 销账）

**病灶（威胁模型 G2）**：命令级审批名单逐字精确匹配（`tool.shell:<命令>`）——
批一次 `Remove-Item x`，换个等价写法（`[System.IO.File]::Delete` / `rm` / 变体）重放，
字面外形态不进名单 = 白拿授权（孪生 API 形态不同权）。

**整改口径（审批族归一）**：

1. **命令→族**：族归一函数进治理层（单一来源），写意图族词表与它**同源引用**；
2. **授权对象 = 族**：批"删除族" = 该族**所有形态**生效；同族换形重放 = 同族授权覆盖
   （不重批）；**跨族仍逐族审批**（批删除≠批网络）；
3. **变体防护**：短形态 / 前缀缩写 / 可执行后缀 / 大小写换形逃不出族（沿 git_write
   变形防护经验——前缀重叠命中、逐形态收口）；
4. **兼容（二选一钉死 = 迁移生效）**：存量逐字审批记录按**族解释**（老记录的命令→族
   映射后生效），不是失效重批；词表外记录退回逐字精确语义。

## 2. 族词表全表（冻结面）

族 id 即人读族名（审批名单条目 / 审批卡 / 记录三面同一词）；命中面标签 = `族id + "命令"`。

| 族 id | 命中面标签 | 族词（大小写不敏感；短形态别名在列） | 成员词头（`::成员` / `.成员(` 前缀命中） |
|---|---|---|---|
| `删除族` | `删除族命令` | `del` `erase` `rd` `rmdir` `rm` `unlink` `shred` `ri` `Remove-Item` | `delete` |
| `移动/复制/改名族` | `移动/复制/改名族命令` | `move` `copy` `xcopy` `robocopy` `ren` `rename` `mv` `cp` `ln` `mi` `cpi` `Move-Item` `Copy-Item` `Rename-Item` | `move` `copy` `rename` |
| `建目录/建文件族` | `建目录/建文件族命令` | `mkdir` `md` `touch` `New-Item` `New-TemporaryFile` `install` `ni` | `create` |
| `文件写入族` | `文件写入族命令` | `tee` `dd` `truncate` `Set-Content` `Add-Content` `Out-File` `Clear-Content` `Set-Item` `Export-Csv` `Export-Clixml` `Start-Transcript` | `write` `append` `replace` `set` `encrypt` `decrypt` |
| `网络族` | `网络族命令` | `curl` `wget` `Invoke-WebRequest` `Invoke-RestMethod` `Start-BitsTransfer` `bitsadmin` `ftp` `sftp` `scp` `ssh` `rsync` `nc` `ncat` `telnet` `iwr` | `download` `upload` |
| `进程族` | `进程族命令` | `start` `stop` `kill` `taskkill` `wmic` `nssm` `Start-Process` `Stop-Process` `Invoke-Expression` `iex` | `kill` `start` `stop` |

**词表口径（逐条）**：

* **写四族**（删除 / 移动复制改名 / 建目录建文件 / 文件写入）= 写意图总闸的执法面；
  网络族 / 进程族**同源在表**但不入写闸（它们不是文件写入意图）。
* **`Rename-Item` 判移动/复制/改名族**（与 `ren` / `rename` / `.Rename()` 同族）——
  词表一致性修正：改名同族同权，批"移动/复制/改名族"覆盖 `Rename-Item` 形态。
* **变体匹配**（全表同一套）：
  * 大小写：一律 ASCII 不敏感；
  * 短形态：短别名词表内逐字命中（`rm` / `rd` / `del` / `ri` …）；
  * 前缀缩写：族词前缀 ≥5 字符命中（`Remove-I` ≡ `Remove-Item`；短于此不收，
    挡 `rem` / `set` / `stat` 类短词误伤）；
  * 可执行后缀：剥一层 `.exe` / `.cmd` / `.bat` / `.com` / `.ps1`（`rm.exe` ≡ `rm`）；
  * 成员词头：前缀命中（`DeleteFile` / `DeleteFolder` ∈ `delete`）；
  * **精确命中优先于缩写命中**（`start` 精确判进程族，不因 `Start-Transcript`
    缩写面误入文件写入族）。
* **族词不跨族重复**（测试钉死）；新增族词必须过"跨族不串"矩阵。

## 3. 族词表 API（单一来源，星港线只准从这里取词）

单一来源 = `crates/foundation/governance/src/approval_family.rs`（`apeireth_governance::approval_family`）。
写意图扫描（`crates/capabilities/tools/src/write_intent.rs`）的族词表与它同源引用
（同一份族词 / 同一命中标签 / 同一变体匹配），两处族词一致由
`write_intent.rs::write_intent_family_vocabulary_agrees_with_the_governance_source` 钉死。

| API | 语义 |
|---|---|
| `CommandFamily`（`ALL` / `id()` / `hit_label()` / `words()` / `is_write_family()` / `from_id()`） | 冻结族词表；族 id 由 `from_id` 解析，**不手写** |
| `word_families(word)` / `member_family(member)` | 命令词 / 成员名 → 族（变体匹配；精确命中优先） |
| `command_families(command) -> Vec<CommandFamily>` | **族归一函数**（归属面，授权对象用）：段首命令动词 + 成员调用 |
| `touched_families(command) -> Vec<CommandFamily>` | 触达面（审批门用）：词面任意命中 + 成员调用（宁误拒不漏放） |
| `family_ids_for_arguments(args)` | 调用参数（`command` / `command1..N`）→ 授权族 id 列表（词表序去重）；空 = 非命令面 |
| `interpret_recorded_command(command) -> ApprovalTarget` | 存量逐字记录的**迁移解释**（命令→族映射后生效；词表外 → 逐字对象） |
| `ApprovalTarget`（`Capability` / `Families` / `Command`） | 一条审批/授权记录的对象词表 |

**两个判定面的方向分工**（同一词表、两个方向各自 fail-closed）：

* **归属面**（`command_families`，授权对象用）：只认**段首命令动词**与**成员调用**——
  段内"提到"族词（`echo del is a word`）不构成授权归属，宁可归属不足（重批）不白拿授权；
* **触达面**（`touched_families`，审批门用）：词面任意命中 + 成员调用——宁可误拒不漏放
  （与写意图扫描同一误拒口径）。

## 4. 审批名单语法（规则引擎面）

名单条目 = `<能力>` / `<能力>:family:<族id>` / `<能力>:<命令>`（存量逐字形态），
可带 `::SilentReject` 后缀。实现：`governance/src/approval_policy.rs`
（`ParsedApprovalEntry::target_for` / `best_approval_match` / `ApprovalPolicyEngine::evaluate`）。

| 条目形态 | 对象 | 匹配语义 |
|---|---|---|
| `tool.shell` | 整个能力 | 能力级（specificity 1，既有语义不变） |
| `tool.shell:family:删除族` | 族 | **该族所有形态**：命令触达面与族相交即命中（specificity 2） |
| `tool.shell:Remove-Item x.txt`（存量） | 族（迁移解释） | 命令→族映射后按族匹配（迁移生效）；同族孪生形态同受管辖 |
| `tool.shell:run-report --weekly`（存量·词表外） | 逐字命令 | 退回逐字精确匹配（既有语义） |
| `tool.shell:family:未知族`（配置错误） | 逐字命令 | **不给族级语义**（宁可无效不放宽）；族 id 必须来自 `CommandFamily::from_id` |

命中回传 `PolicyMatch { matched_command, matched_family, silent, .. }`——
`matched_family` 是审批卡族标注的规则面来源之一。

## 5. 审批卡族标注（授权对象 = 族，人见族而批）

被批命令的族在冻结时算好、随审批记录持久化、随视图与卡片给出：

| 面 | 字段 | 取值 |
|---|---|---|
| 审批记录 `PendingApproval` | `authorization_families: Vec<String>` | 族 id 列表（`family_ids_for_arguments`，词表序） |
| 审批视图 `PendingApprovalView` | `authorization_families` | 同上（存量记录按迁移语义映射后给出） |
| 网关载荷 `CanonicalPendingApproval` / `ImApprovalNotice` | `authorization_families` | 同上 |
| IM 审批卡 `ImApprovalCard` | `authorization_families` + 渲染文本 `授权族: …` | 族 id 以 `、` 连接；空 = `（无族·逐操作授权）` |

**语义提示**：卡片写明 `授权族: 删除族` = 本次批准的授权对象是**删除族**（该族所有形态）；
`（无族·逐操作授权）` = 非命令面操作，授权只对这一次操作。**跨族必批**：另一族的调用
会另开审批卡（卡上族标注不同）。

## 6. 重放语义（对抗口径）

| 场景 | 语义 | 钉死测试 |
|---|---|---|
| 同族换形重放（`Remove-Item x` → `[System.IO.File]::Delete('x')` / `rm x` / 变体） | **同族授权覆盖，不重批**（重放不再"逃出授权面"，而是落进同一族授权） | `permission_preset.rs::same_family_form_change_replay_is_covered_without_reapproval` |
| 跨族重放（批删除族后调网络族） | **必批**（批删除≠批网络） | `permission_preset.rs::cross_family_replay_still_requires_separate_approval` |
| 跨族复合命令（`del a && curl b`） | 族组合 = 另一授权对象（不吞并单族授权、也不被吞并） | 同上 |
| 同族不同目标（批 `Remove-Item x` 后 `rm -rf <y>`） | **族集授权覆盖**（批"删除族" = 该族所有形态生效；审批卡族标注随卡明示授权族） | `permission_preset.rs::family_authorization_covers_every_form_of_the_family` |
| 无族命令（`echo hi`） | 逐字逐批（精确操作指纹，L 组口径原样） | `permission_preset.rs::approval_memory_is_scoped_to_session_capability_and_authorization_object` |
| `AllowedOnce` 一次性审批 | **不变**：一审批一动作恰好一次（`approval_closure.rs` 词表不动） | `approval_closure.rs` 既有测试 |
| git 写 L2 人工审批地板 | **不变**：免审记忆不越过 L2（`git_write_preset_semantics.rs::l2_approval_memory_never_skips_the_human`） | 既有测试 |

**边界（如实）**：族集授权在族内不绑目标——要目标级逐批走 `AllowedOnce` 一次性审批面，
或关闭会话 `approval_remember`（= 每次人批）。审批记忆**进程内**（重启即清，不落盘），
与既有口径一致。

## 7. 存量记录迁移语义（二选一钉死 = 迁移生效）

| 存量记录面 | 迁移解释 | 钉死测试 |
|---|---|---|
| 审批名单逐字条目（`tool.shell:<命令>`） | 命令→族映射后生效（同族孪生形态同受管辖，不失效重批） | `approval_policy.rs::legacy_verbatim_entries_migrate_to_family_interpretation` |
| 持久化审批记录（`authorization_families` 字段加入前的 session blob） | 读取时按 `tool_call` 逐字命令映射族生效 | `approval.rs::persisted_records_before_the_family_field_migrate_by_interpretation` |
| 族级记录构造（迁移函数本体） | `interpret_recorded_command`：词表外 → 逐字对象（不静默扩权） | `approval_family.rs::legacy_verbatim_records_migrate_by_family_interpretation` |

**未选方案（显式失效重批）**：不采用——老记录直接作废会把运维存量名单变摆设；
迁移生效 + 审批卡族标注（人见族而批）是更友好的口径，且更安全的方向一致
（族门比逐字门更宽 = 更多形态被管住）。

## 8. 星港线接线指引

**舵室智能体驱动 / 深望智能体放行两件按本契约接线，三不要**：

1. **不要自造族词**：族 id 与族词只从 `apeireth_governance::approval_family` 取
   （`CommandFamily::from_id` 解析；需要枚举全表用 `CommandFamily::ALL`）；
2. **不要逐字比对命令**：判"同一授权是否覆盖某调用"用 `family_ids_for_arguments`
   （授权对象键 = 族集；见 `runtime-assembly/src/canonical/permission_preset.rs::authorization_key`）；
3. **不要另起审批卡字段**：审批卡族标注 = `authorization_families`（SDK 卡面渲染
   `授权族: …`），读取走 `PendingApprovalView` / `CanonicalPendingApproval` 既有字段。

**接线点（真实坐标）**：

* 族词表 / 归一函数：`crates/foundation/governance/src/approval_family.rs`；
* 审批名单族匹配：`crates/foundation/governance/src/approval_policy.rs`；
* 授权记忆（同族覆盖 / 跨族必批）：`crates/engine/runtime-assembly/src/canonical/permission_preset.rs`；
* 审批卡族标注链：`crates/engine/runtime/src/canonical/approval.rs`（记录+视图）→
  `crates/adapters/gateway/src/canonical_entry.rs`（API 载荷）→
  `crates/adapters/gateway/src/im_approval.rs`（IM 通知）→ `crates/adapters/sdk/src/im/card.rs`（卡片渲染）；
* 写意图同源消费：`crates/capabilities/tools/src/write_intent.rs`。

## 9. 测试证据（E-P2 新增，全部 `cargo test --locked -j 2` 实跑）

| # | 测试 | 面 | 钉什么 |
|---|---|---|---|
| 1 | `approval_family.rs::family_normalization_keeps_all_forms_of_a_family_together` | 族归一矩阵 | 同族多形态（命令面/短别名/静态/实例调用）归一族 |
| 2 | `approval_family.rs::family_normalization_never_crosses_families` | 族归一矩阵 | 跨族不串 + 复合命令两族入账 |
| 3 | `approval_family.rs::variant_forms_short_aliases_prefix_abbreviations_and_case_stay_in_family` | 变体防护 | 短束/前缀/大小写/后缀变体逃不出族 |
| 4 | `approval_family.rs::exact_hits_win_and_read_only_forms_attribute_no_family` | 边界 | 精确命中优先 + 只读零归属 |
| 5 | `approval_family.rs::attribution_ignores_mentions_while_the_gate_still_catches_them` | 两面分工 | 提到≠归属（不白拿）/ 触达面保守 |
| 6 | `approval_family.rs::family_vocabulary_is_frozen_and_disjoint` | 词表冻结 | 六族齐全/族词不跨族重复/标签=id+命令 |
| 7 | `approval_policy.rs::family_entries_gate_every_form_of_the_family` | 重放对抗（门） | 族级条目管住同族所有形态 + 族标注 |
| 8 | `approval_policy.rs::cross_family_entries_stay_in_their_own_family` | 跨族 | 批删除≠批网络（双向） |
| 9 | `approval_policy.rs::legacy_verbatim_entries_migrate_to_family_interpretation` | 存量迁移 | 逐字条目按族解释后生效 |
| 10 | `approval_policy.rs::unattributable_entries_fall_back_to_exact_text_matching` | 边界 | 词表外逐字退路 + 未知族 id 不放宽 |
| 11 | `permission_preset.rs::same_family_form_change_replay_is_covered_without_reapproval` | 重放对抗 | 同族换形不重批 |
| 12 | `permission_preset.rs::cross_family_replay_still_requires_separate_approval` | 重放对抗 | 跨族必批 + 复合命令另批 |
| 13 | `permission_preset.rs::family_authorization_covers_every_form_of_the_family` | 授权对象 | 批"删除族"=该族所有形态生效 + 无族逐字 |
| 14 | `permission_preset.rs::approval_memory_is_scoped_to_session_capability_and_authorization_object` | 作用域 | L 组回归（echo 不免审 rm）+ 会话/能力隔离 |
| 15 | `approval.rs::approval_view_carries_the_family_annotation` | 审批卡族标注 | 视图带族 + 序列化面 |
| 16 | `approval.rs::persisted_records_before_the_family_field_migrate_by_interpretation` | 存量迁移 | 持久化老记录按族映射生效 |
| 17 | `sdk card.rs::approval_card_carries_the_family_annotation` | 审批卡族标注 | 三渠道卡片带授权族 + 空族明示 + 旧载荷兼容 |
| 18 | `delete_path_parity.rs::command_level_approval_entries_are_family_scoped_across_twin_forms` | 重放对抗 | 孪生换形不逃名单（G2 封堵）+ 跨族不串 |
| 19 | `write_intent.rs::write_intent_family_vocabulary_agrees_with_the_governance_source` | 同源断言 | 两处族词一致（族词+成员词头逐一核对） |
| 20 | `write_intent.rs::non_write_family_member_calls_stay_out_of_the_write_gate` | 边界 | 网络/进程族不入写闸 |
| 21 | `approval_family.rs::legacy_verbatim_records_migrate_by_family_interpretation` | 存量迁移 | 迁移函数本体 + 参数面同源提取 |

既有 186 套件 + 在飞各线零回归：`cargo test --workspace --locked -j 2` 全绿
（门禁口径见 [CONSTRUCTION-DISCIPLINE.md](CONSTRUCTION-DISCIPLINE.md) 第四节）。

## 10. 已知边界（如实）

1. **词法面边界同写意图口径**（`write_intent.rs` 白名单注释）：引号内提及族词 /
   只提到调用形态文本 = 触达面误拒（明示）；动态成员名/反射调用、编码负载、
   解释器内写入 = 词法外漏放（审批链 + 沙箱兜底，G3 对象级判定）；
2. **归属面是段首动词 + 成员调用**：经调度器转手的形态（`cmd /c del x`）归属不足
   → 授权面按"重批"收口（安全向）；触达面（审批门）仍按词面触达判到；
3. **族集授权不绑目标**（§6 边界）：同族不同目标不重批，要目标级逐批走
   `AllowedOnce` 或关闭 `approval_remember`；
4. **未知族 id / 词表外命令**：不获得族级语义（退回逐字，宁可无效不放宽）——
   接线方用 `CommandFamily::from_id` 取族即可杜绝此类配置错误；
5. **审批记忆进程内**：重启即清（既有口径）；跨进程持久授权不在本批。
