# K 轴 · 设备审批卡→IM 推送链 —— 推送时序图 / 按钮闭合语义 / 测试证据

> `Status: 📋 接线落位（2026-10-18，K2/K3 收尾缝批）`
> 衔接 [`K-device-control-three-beat.md`](K-device-control-three-beat.md) 七节
> 报备的两条未接线缝：**IM 审批卡推送面**（本批由 gateway 消费
> `DeviceApprovalCard` 卡载荷）+ **`tool.self_status` 设备节探针**（
> `DeviceControlStatus` 接线归口本批）。先例同链：提案卡推送面
> `im_proposal`（同链同信封 / 推送幂等 / 失败留账）。

## 一、战场与各件

| 件 | 路径 | 是什么 |
|---|---|---|
| 审批卡载荷真参数 | `crates/capabilities/tools/src/device/device_control.rs` | `PreviewFrame` 携动作参数（与 plan 拍同值），`DeviceApprovalCard::from_preview` 卡载荷自足（参数不藏，执行重建凭卡） |
| 两本账 | `crates/capabilities/tools/src/device/approval.rs` | 推送账 `device-approval-pushes.jsonl`（plan_id 幂等 + 失败留账）+ 按钮决定账 `device-approval-decisions.jsonl`（一计划一决定 + 如实署名） |
| 设备审批面 | `crates/adapters/gateway/src/im_device.rs` | 卡→信封映射 / 回调路由标记 / 按钮批准·拒绝落账（批准即执行） |
| 桥接线 | `crates/adapters/gateway/src/im_bridge.rs` | 出站推送 + 失败重试 + 按钮路由 + `DeviceDecided`/`DeviceRefused` 回执帧 |
| 自述探针 | `crates/engine/runtime-assembly/src/canonical/self_status_source.rs` | `device_control_status_probe`（与工具面同一台执行核心同账同源） |
| 自述节 | `crates/capabilities/tools/src/self_status.rs` | `device_control` 节入二十一节冻结合同（控制计数/最近动作真值） |
| 装配接线 | `tool_modules.rs` `DeviceControlModule::runner()`、`production.rs` 注册点 | 面注册才接探针，缺源显式 null |

## 二、推送时序图（批准即执行 · money 档 IM 卡闭环）

```text
调用方(模型/面板)  tool.device_control      设备审批面+两本账        IM 渠道       人(手机)
  |  preview(设备,动作,参数)   |                  |                    |             |
  |-------------------------->| 计划绑定 + 预览票 (进程内, 零副作用)  |             |
  |  预览帧 + 审批卡载荷        |                  |                    |             |
  |  (DeviceApprovalCard:      |                  |                    |             |
  |   plan_id/参数/效果/分级)   |                  |                    |             |
  |<--------------------------|                  |                    |             |
  |  push_device_approval_cards(card)            |                    |             |
  |--------------------------------------------->| 幂等查账: plan_id   |             |
  |                            |                  |  已送达 → 跳过(0出站)|             |
  |                            |                  |  同链同信封出站      |             |
  |                            |                  |------------------->| 卡到手机     |
  |                            |                  |  (三拍纪律+效果行+   |             |
  |                            |                  |   money IM 卡标注)  |             |
  |                            |                  |<-------------------|             |
  |                            |                  | 推送账 append:      |             |
  |                            |                  |  pushed 行(plan_id  |             |
  |                            |                  |  +卡载荷+送达回执)  |             |
  |                            |                  |                    |             |
  |      [推送失败: failed 行带真因留账; retry 凭账本行载荷重建出站 (重启可续)]       |
  |                            |                  |                    | 点批准/拒绝  |
  |                            |                  |<-------------------|<------------|
  |                            |                  | ①决定账查一计划一决定|             |
  |                            |                  |  ②推送账查送达记录   |             |
  |                            |                  |  (无回执=无IM卡,拒) |             |
  |   (批准) execute(设备,动作,参数,plan_id,证据)  |                    |             |
  |                            |<-----------------| 证据: approved_by=  |             |
  |                            |                  |  im:人@渠道,        |             |
  |                            |                  |  im_card_id=送达回执|             |
  |                            | 三拍绑定核对 + 预览票核销 + 分级审批校验 |             |
  |                            | 适配器真执行 + 状态账 append (谁批署名) |             |
  |                            |----------------->| 决定账 append       |             |
  |                            |                  |  (approved, 署名)   |             |
  |                            |                  |------------------->| 回执文本     |
  |                            |                  |  (署名+状态账记录id)|<------------|
  |      (拒绝) 决定账 append (rejected, 署名), 不执行, 回执「拒绝即不执行」          |
```

## 三、硬语义（全部有测试钉）

1. **同链同信封**：设备审批卡 = 既有 `ImApprovalCard` 信封（同一条
   `deliver_card` 出站链，0 第二套卡信封；出站体逐层键集与工具审批卡同构）。
   `approval_ref = pair_id = plan_id`（推送幂等键 = 一计划一决定闭合身份）；
   动作摘要 = 卡载荷规范字节 SHA-256（载荷被改 → 摘要不符）。
2. **批准即执行**：按钮批准走与模型侧/面板侧**同一台执行核心**
   （`DeviceControlRunner::execute`，0 第二套执行语义）——三拍同计划绑定、
   先模拟后执行（预览票一次性）、分级审批证据照常生效；执行落状态账，
   `approved_by` = IM 用户名 + 渠道标注（`im:人@渠道`，如实署名）。
3. **money 档必须 IM 卡**（三面断言）：① 面板侧执行缺 `im_card_id` 拒绝即帧
   （零副作用）；② IM 批准落地的账行必带**送达回执**作 `im_card_id`
   （IM 审批卡标识 = 卡到手机的回执）；③ 查无送达记录的点击不可信（无回执 =
   无 IM 卡），拒绝即帧零执行。**irreversible 可面板或 IM**：面板侧带
   `approved_by` 执行与 IM 批准互为独立入口，同一执行核心同一语义。
4. **一计划一决定**：拒绝 = 不执行 + 决定账关账；拍过板的计划再点拒绝即帧
   （真因如实，不可逆动作真的不会发生）。批准执行失败 = 决定不落账，排障后
   可再点（如实回执）。
5. **推送幂等键 = 动作 plan_id**：推送账 `pushed` 行是真值幂等集，同计划不
   重复推送（含换桥/重启后同账生效）。
6. **失败留账重试**：推送失败写 `failed` 行（真因 + **卡载荷随行留痕**）待
   重试；`retry_device_approval_pushes` 凭账本行载荷重建出站（不需要外部
   记忆，重启可续）；IM 失败不丢卡。
7. **回调路由分家**：设备动作计划卡 `subject = tool.device_control.plan`，
   与 `tool.device_control` 的**工具审批卡**（subject = 能力 id，闭合身份 =
   approval id，走四态闭合链）无歧义——两条路由各有各的闭合语义。
8. **卡不设超时**：点按本身就是底线②③的那次逐次人批（卡面动作摘要钉死批的
   是什么）；一次性闸门在执行面（预览票一次性）。信封超时戳取远期哨兵
   `DEVICE_CARD_NO_DEADLINE_MS`（同提案卡口径）。

## 四、自述探针接线（设备节扩：控制计数/最近动作真值）

- `device_control_status_probe(runner)` 绑 `tool.device_control` **同一台执行
  核心**（同状态账同白名单），`ProductionSelfStatusSource::with_device_control_probe`
  真接；装配根在面注册时接线（注册即闸）。
- `self_status` 新增 `device_control` 节（**二十一节冻结合同**的形状冻结演进，
  附理由）：`devices_total/devices_enabled`（白名单真值）、
  `actions_executed/executed_reversible/executed_irreversible/executed_money`
  （控制计数 = 状态账行数）、`last_action`（最近动作：设备/动作/时刻/谁批）、
  `note/reason`。缺源/读失败 = 全列显式 `null` + 原因（0 装诚实）。
- 探针面核对（本批真值审计）：`ImageOpsStatus` / `DocOpsStatus` /
  `SheetOpsStatus` 三节探针已由创作/表格批接线（同计数器同账），本批只补
  `DeviceControlStatus` 这一口——四节现全走 `self_status_source` 统一探测。

## 五、测试证据

单一命令（日期 2026-10-18，工具链 rustc 1.97.1 / cargo，`-j 2` 口径）：

```powershell
cargo test -p apeireth-gateway --test im_device_cards --test im_proposal_cards --test im_approval_cards -j 2
# => im_device_cards 8 passed / im_proposal_cards 9 passed / im_approval_cards 9 passed; 0 failed
cargo test -p apeireth-tools-canonical --lib -j 2
# => 561 passed; 0 failed (本批新增 device::approval 4 项 + self_status 缺源 null 1 项,
#    既有 every_section_arrives_with_real_values 扩 device_control 节真值断言)
cargo test -p apeireth-runtime-assembly --test device_control_status_probe -j 2
# => 2 passed; 0 failed
```

| # | 测试 | 钉住的行为 |
|---|---|---|
| ① | `im_device_cards::device_cards_reach_the_phone_through_the_same_envelope_as_approval_cards` | 卡出站**信封同构**（与工具审批卡逐层键集一致）+ 按钮闭合身份 = plan_id |
| ② | `im_device_cards::approve_click_executes_the_action_signed_by_the_im_user_and_channel` | **按钮落账署名**（状态账 `approved_by = im:人@渠道` + `im_card_id` + 真执行） |
| ③ | `im_device_cards::money_grade_execute_demands_an_im_card` | **money 档必须 IM 卡**三面断言（面板缺卡拒 / IM 账行带回执 / 无送达点击拒） |
| ④ | `im_device_cards::the_same_plan_is_never_pushed_twice` | **幂等**（plan_id 键, 含跨桥同账生效） |
| ⑤ | `im_device_cards::a_failed_push_stays_on_the_books_and_the_retry_rebuilds_from_the_ledger` | **失败留账** + 凭账本载荷重试送达 |
| ⑥ | `im_device_cards::reject_click_lands_a_signed_decision_and_closes_the_plan` | 拒绝关账 + **一计划一决定**（不可逆动作零执行） |
| ⑦ | `im_device_cards::without_the_device_face_the_bridge_reports_unwired_honestly` | 未接线照实报 + 出站 inert（0 假旋钮） |
| ⑧ | `im_device_cards::tool_approval_cards_for_device_control_still_close_through_the_four_state_chain` | 路由零碰撞（工具审批卡照走四态闭合） |
| ⑨⑩ | `im_device::tests::`（信封映射 / 路由分家） | 信封字段映射即承诺 + subject 分家 |
| ⑪-⑭ | `device::approval::tests::` | 推送账 append-only + 幂等集 / 重试集 / 决定账一计划一决定 / 两词表闭合 |
| ⑮ | `self_status::tests::device_control_section_without_a_source_is_explicit_null_with_a_reason` | `device_control` 节缺源显式 null + 原因（既有全值测试另扩本节真值断言） |
| ⑯⑰ | `device_control_status_probe`（探针真值 / 缺源 null） | 探针与工具面同账同源（控制计数/最近动作真值）/ 缺探测口显式 null |

既有套件零回归（本批实测值，见 §六）：`im_proposal_cards` 9 项、
`im_approval_cards` 9 项、`device_face` 3 项原样全绿。

## 六、门禁实测（本批实测值 + 日期，不代改 test-baseline 真源）

单一命令族（2026-10-18，`-j 2` 口径，日志 `artifacts/k23-device-im-*.log`）：

```powershell
cargo test --workspace --all-targets --locked --no-fail-fast -j 2
# => 322 套件 / 8509 passed / 0 failed / 52 ignored, exit 0 (全量实测收卷)
cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings   # 0 警告, exit 0
cargo fmt --check                                                     # 0 diff, exit 0
pwsh -File scripts/check-neutral-terms.ps1                            # 本批新增行 0 命中
pwsh -File scripts/secret-scan.ps1                                    # No secrets found
```

注记（负载口径）：中途两次全量被他线在途红打断（`image_ops_core_swap` 换核
容差自检、`care/engine.rs` clippy），两归口线收口后本次收卷全绿；本批文件
全程 0 failed。neutral-terms 扫描的命中均为他线新档引述禁词（见扫描输出），
本批新增行 0 命中。

本批增量 = 新增测试 **17 例**（gateway `im_device_cards` 套件 8 例 + `im_device`
单测 2 例 + tools `device::approval` 4 例 + self_status 缺源 null 1 例 +
runtime-assembly `device_control_status_probe` 套件 2 例；既有
`every_section_arrives_with_real_values` 另扩 `device_control` 节真值断言）。
既有套件零回归（同次全量实测）：`im_proposal_cards` 9/9、`im_approval_cards`
9/9、`device_face` 3/3、tools lib 全绿。全量收卷数（322 套件 / 8509 passed /
0 failed / 52 ignored）是否回填 test-baseline 归口真源维护线（本批不代改真源）。

## 七、0 装诚实（本批现状）

- **库级已实现、生产装配未接线〔四级口径：IMPLEMENTED〕**：设备审批卡→IM 推送链（同链同信封 + plan_id 幂等 +
  失败留账重试）、按钮批准即执行/拒绝关账（两本账落盘）、money 档 IM 卡证据闭环、回调路由分家 ——
  `with_device_approval_face` / `push_device_approval_cards` / `retry_device_approval_pushes`
  全仓**仅测试调用、生产装配无注入点**，桥内 face 缺位照实回「未接线」，故 money 档 IM 卡出卡来源在生产装配下**未闭合**
  （与[能力矩阵 §0.2](../../03-reference/capabilities-matrix.md) 同口径）。
  > 口径校注（2026-10-24 文档治理批）：本条原标 `PRODUCTION WIRED` 属**划错边界**（把"库级有实现+有测试"当成"已接生产装配"），
  > 已按真源改为 `IMPLEMENTED`。这正是口径纪律要防的过度声称。
- **已接线〔四级口径：PRODUCTION WIRED〕**：`self_status` `device_control` 节 + `device_control_status_probe` 装配接线
  （`runtime-assembly/src/canonical/production.rs:1349` 实证，生产装配根确有调用）。
- **未接线**（如实）：真实 IM 端点的 live 往返（本批 0 真实网络，端点全 mock，
  **未实测** live）；运行时侧「预览拍产卡 → 自动提交推送」的触发器——本批交付
  `ImBridge::push_device_approval_cards` 显式提交口（谁看见卡谁提交），随 IM
  装配批挂生产桥时接；真实家居协议族/购买渠道族仍为 mock 适配器（K2 报备不变）。
