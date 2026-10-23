# K 轴 · 设备受控执行骨架 —— 三拍时序图 / 分级词表 / 测试证据

> `Status: 📋 骨架落位（2026-10-18，K 轴 L3/L4 批）`
> 面向 [`K-life-presence.md`](../K-life-presence.md) 四条安全底线（Q41-Q44）的
> L3 受控执行 / L4 代理代办骨架。能力宪章口径：**它没有手，但它有插头**。
> 本批只落骨架 + mock 适配器（真实协议族后插，**未接线**）。

## 一、战场与各件

| 件 | 路径 | 是什么 |
|---|---|---|
| 受控执行骨架 | `crates/capabilities/tools/src/device/device_control.rs` | 三拍执行 + 三档分级 + 白名单 + 状态账 + 治理钩子 + `tool.device_control` |
| 模块登记 | `crates/capabilities/tools/src/device/mod.rs` | device 目录登记（K1 感知件 registry/adapter/sense/audit/weather 同目录并行落位） |
| 装配 | `runtime-assembly/.../tool_modules.rs` `DeviceControlModule`、`production.rs` 注册点、`self_status_source.rs` 名册行 | 注册即闸 |
| 生产接线 | `adapters/cli/src/lib.rs` | 配置注入（双闸）+ 授权面 grant + `DeviceControlApprovalHook` 治理链 |

## 二、三拍时序图（底线④ 先模拟后执行）

```text
调用方            工具面 (tool.device_control)   白名单/词表           mock 适配器        状态账
  |  plan(设备,动作,参数)  |                      |                   |                |
  |---------------------->| 校验 (登记? 启用? 词表? 参数?)            |                |
  |      计划帧 plan_id    |                      |                   |                |
  |<----------------------|                      |                   |                |
  |  preview(plan_id)     |                      |                   |                |
  |---------------------->| snapshot(只读) ------->| 只读              |                |
  |   将发生什么 (before→after 确定性投影)          |                   |                |
  |<----------------------| (零副作用: 不写设备、不落账)               |                |
  |  execute(plan_id, 谁批[, im_card_id])          |                   |                |
  |---------------------->| 计划绑定核对 + 预览票 + 分级审批证据       |                |
  |                       | apply(真执行) ------->| 变更状态           |                |
  |                       | append (何时/何设备/何动作/谁批) --------->| append-only    |
  |    执行帧 (before→after + 记录 id)             |                   |                |
  |<----------------------|                      |                   |                |
```

三拍硬语义（测试钉死）：

1. **plan / preview 零副作用**：不改设备状态、不写状态账（预览票是进程内记忆，
   不落盘）；preview 的 after 是**投影**不是执行。
2. **三拍同计划绑定**：preview / execute 必须带 plan 拍的 `plan_id`（FNV-1a 64
   摘要，非密码学），现算不符拒绝即帧（载荷被改即拒）。
3. **先模拟后执行**：没预览过的计划拒绝执行；预览票一次性（逐次语义）——
   通过分级审批证据校验后才核销，缺人批/缺 IM 卡的拒绝**零副作用**（不烧票）。
4. **execute = 真执行**：适配器调用 + 状态账 `device-actions.jsonl` 追加。

## 三、分级词表（底线②③，闭合；词表外拒绝即帧）

| 档 | 词表（闭合） | 审批语义 |
|---|---|---|
| `reversible` | `power_on` `power_off` `set_brightness` `set_temperature` `set_mode` `set_timer` `start` `stop` | 审批档：默认每次人批；**自动档只此档可配**（`APEIRETH_ENABLE_DEVICE_AUTO_PASS_REVERSIBLE`，放行如实记 `auto-pass-reversible`） |
| `irreversible` | `ignite`（开火） `unlock_door`（开门） `physical_delete`（物理删除） | **永远逐次人批**（无自动档，自动档开关开了也不放） |
| `money` | `purchase`（购物） `place_order`（下单） | **永远逐次人批 + IM 审批卡**（`im_card_id` 缺位拒绝即帧，无自动档） |

动钱词表闭合（底线②）：`money` 档只认 `purchase` / `place_order` 两词；
词表外动作（含大小写变体、带空白变体）一律拒绝即帧。

### 白名单（底线①，复用）

- 判定语义：**未登记拒绝即帧、已登记未启用拒绝即帧、默认零设备**。
- 复用接缝（K1 感知批设备表**只读消费**）：`RegistryFileWhitelist` 现读
  `devices.json`（StoredDoc 严格打开，与 `tool.device_sense` **同一本设备表**）；
  表拒开/坏表 = 拒绝即帧（fail-closed，不静默回退空表）。
- 类目推动作词表（闭合、保守）：`appliance` = reversible 家电面；
  `sensor` / `camera` / `other` = 只读感知面**零执行动作**。
  irreversible / money 两档动作只随显式逐设备登记挂接（mock/测试 harness 与
  未来 L4 渠道族注册面）；L4 动钱/不可逆面的类目扩表归口 L4 渠道接入批。
- 进程内注册表 `DeviceRegistry`（mock fixture：灯 / 空调 / 微波炉三件显式登记
  启用）与文件桥实现同一 `DeviceWhitelist` 合同。

### mock 适配器（真实协议族后插）

内置假设备三件（灯 / 空调 / 微波炉）真走三拍 + 状态账；状态变更 =
`project_action` 的确定性投影（preview 与 execute 同一投影，"将发生什么"即
"会发生什么"）。真实协议族（本地桥 / 云桥 / 直连）与购买渠道族
**未接线**（0 装诚实）：接线走 `DeviceAdapter` trait 后插，三拍语义不变。

## 四、开关（双闸 fail-closed）

| 开关 | 语义 |
|---|---|
| `APEIRETH_ENABLE_DEVICE_SENSE` | 上游感知闸（K 轴 L1 感知族总闸，与感知批同一常量） |
| `APEIRETH_ENABLE_DEVICE_CONTROL` | 受控执行总闸：**双闸齐开**（都显式 `"1"`）才注册即闸 |
| `APEIRETH_ENABLE_DEVICE_AUTO_PASS_REVERSIBLE` | 子档：**只对 reversible 档**自动放行；irreversible / money 永逐次人批不入子档 |

任一未设 / `"0"` / 杂值 = 关（`DeviceControlConfig::from_env` = `None`，面不注册，
名册 `device_control` 行照实 false）。

## 五、状态账（append-only）

`device-actions.jsonl`：何时（`at_ms`）/ 何设备 / 何动作 / 谁批（`approved_by`，
自动档如实记 `auto-pass-reversible`）+ 分级 + 计划 id + before/after +
money 档 IM 卡 id。逐行 StoredDoc 信封，追加写 + 落盘，不重写不删行；
坏行读作不存在（计入 `skipped`，不砖化整本）。

## 六、测试证据

单一命令（日期 2026-10-18，工具链 rustc 1.97.1 / cargo，`-j 2` 口径）：

```powershell
cargo test -p apeireth-tools-canonical --lib -j 2
# => test result: ok. 529 passed; 0 failed (device_control 20 项全绿; 同库其余为既有/K1/sheet 面测试)
```

| # | 测试（`device::device_control::tests::`） | 钉住的行为 |
|---|---|---|
| ① | `plan_beat_is_pure_and_binds_the_plan_id` | plan 零副作用（不落账、不动设备）+ 计划绑定摘要 |
| ② | `preview_beat_projects_effects_with_zero_side_effects` | preview 零副作用 + "key: 旧 → 新" 效果行 |
| ③ | `execute_beat_changes_mock_state_and_appends_the_ledger` | execute 真执行（mock 状态真变更 + 账本追加） |
| ④ | `execute_refuses_a_plan_that_was_never_previewed` | 先模拟后执行（底线④） |
| ⑤ | `execute_rejects_plan_id_drift` | 三拍同计划绑定（载荷被改即拒）+ 预览票一次性 |
| ⑥ | `reversible_grade_is_approval_tier_with_optional_auto_pass` | reversible 审批档 + 自动档可配（如实记谁批） |
| ⑦ | `irreversible_grade_always_needs_per_call_human_approval` | irreversible 永逐次人批（自动档开了也不放） |
| ⑧ | `money_grade_always_needs_per_call_human_approval_plus_im_card` | money 永逐次人批 + IM 卡（缺卡拒） |
| ⑨ | `grading_vocabulary_is_closed_and_disjoint` | 三档词表闭合互斥、词表外拒绝即帧 |
| ⑩ | `whitelist_refuses_unregistered_and_disabled_devices` | 白名单拒（默认零设备 / 未登记 / 未启用） |
| ⑪ | `device_action_whitelist_and_params_are_closed` | 每设备动作白名单 + 参数键集闭合 |
| ⑫ | `ledger_is_append_only_and_ordered` | 状态账 append-only（既有行字节原样、保序） |
| ⑬ | `mock_fixtures_walk_the_three_beats` | 灯/空调/微波炉 fixture 真走三拍 + 状态账 |
| ⑭ | `approval_card_carries_preview_effects_and_im_card_flag` | 审批卡载荷（预览效果 + money IM 卡标注） |
| ⑮ | `switch_is_fail_closed_on_the_double_gate` | 开关双闸 fail-closed（杂值全拒 + 跨件开关常量一致） |
| ⑯ | `governance_hook_grades_the_three_tiers` | 治理钩子分级矩阵（零副作用拍零审批 / 三档分级 / 词表外 deny） |
| ⑰ | `boundary_guard_denies_whitelist_and_vocabulary_violations` | 单调边界 Guard 只拒不放 |
| ⑱ | `status_section_reports_honest_counts` | 自述节（设备计数 / 分级计数 / 最近动作真值） |
| ⑲ | `tool_face_walks_the_three_beats` | 工具面三拍走通 + 缺 plan_id / 词表外拍拒绝即帧 |
| ⑳ | `file_registry_bridge_reuses_the_l1_whitelist_read_only` | 白名单复用（L1 `devices.json` 只读消费 + 类目推词表 + 坏表拒开） |

既有套件零回归（本批实测值，2026-10-18，`-j 2` 口径，不代改 test-baseline 真源）：

```powershell
cargo test --workspace --exclude apeireth-image-algo --all-targets --locked --no-fail-fast -j 2
# => exit 0: 300 套件 / 7857 passed / 0 failed
cargo test -p apeireth-image-algo --lib --locked -j 2
# => exit 0: 572 passed / 0 failed / 24 ignored (隔离面)
```

注记：全量 `-j 2` 并行负载下 `apeireth-image-algo` (lib test) 两次撞 MSVC 链接器
内存断点（`link.exe` 1104），隔离复跑全绿 —— 按宪章 §四.1「链接器内存口径；
负载 flake 隔离复跑定性」归类为负载 flake（该面属图像引擎批, 本批只定性不代改）。

## 七、0 装诚实（本批现状）

- **已接线**：三拍执行、三档分级、白名单（内存 + `devices.json` 只读消费桥）、
  mock 适配器（灯/空调/微波炉）、状态账、审批卡载荷、双闸开关、生产装配
  （注册即闸 + 授权面 + 治理钩子）。
- **未接线**（如实）：真实家居协议族 / 购买渠道族（`DeviceAdapter` trait 留插槽）。
- **收尾缝报备更新（2026-10-18，设备审批卡→IM 批）**：本节原报备的两条未接线
  已由 [`K-device-approval-im-push.md`](../K-device-approval-im-push.md) 接线 ——
  IM 审批卡推送面（gateway 消费 `DeviceApprovalCard` 卡载荷，同链同信封 +
  plan_id 幂等 + 失败留账重试 + 按钮批准/拒绝落账）与 `tool.self_status`
  设备节探针（`DeviceControlStatus` 接 `self_status_source` 统一探测口）。
