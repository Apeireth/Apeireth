# K 轴 L5 守护流 · 时序图 + 测试证据（K2/L5 批产出）

> `[Document-Meta]`
> Document:        docs/04-internal/backend-enhancement/K-guard-watch-flow.md
> Version:         1.1（K2/L5 批：提醒闭环 + L5 守护骨架；2026-10-18 总装批：卷宗从 artifacts/ 归档入 docs/）
> Last-Modified:   2026-10-18
> Status:          🟢 活跃
> Author:          K2/L5 批施工线
>
> 口径注记：本文只写本批交付面与实测证据；测试数字以本批实测为准，
> 全量口径见 `docs/03-reference/test-baseline.md`（本文不抄写全量数字）。

## 一、守护流时序图（只看不动：看到 ≠ 动手）

```text
┌────────┐   ┌───────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ 设备表  │   │ tool.guard_watch│  │  守护规则表   │   │  告警账       │   │ care 引擎     │
│devices.│   │  (只读感知面)   │   │guard-rules. │   │guard-alerts. │   │(同源纪律)     │
│ json   │   │               │   │ json (拒开)  │   │ jsonl        │   │              │
└───┬────┘   └──────┬────────┘   └──────┬───────┘   └──────┬───────┘   └──────┬───────┘
    │               │                   │                  │                  │
    │ ① 白名单读     │                   │                  │                  │
    │◀──────────────│ (StoredDoc 拒开;   │                  │                  │
    │               │  表外/停用规则跳过  │                  │                  │
    │               │  不编造读数)       │                  │                  │
    │ ② 适配器读状态 │                   │                  │                  │
    │──────────────▶│ (只读; mock fixture│                  │                  │
    │               │  如实标注数据源)   │                  │                  │
    │               │                   │                  │                  │
    │               │ ③ 规则判定 (纯判定) │                  │                  │
    │               │──────────────────▶│ 生效时段(时序②)   │                  │
    │               │   阈值条件(严格不等 │                  │                  │
    │               │   缺读数=无法判定)  │                  │                  │
    │               │   连续命中窗(时序①) │                  │                  │
    │               │◀──────────────────│                  │                  │
    │               │                   │                  │                  │
    │               │ ④ 命中 → 告警入账   │                  │                  │
    │               │─────────────────────────────────────▶│ append-only      │
    │               │   (恒带「守护告警」标注)               │ 逐行 StoredDoc   │
    │               │                   │                  │                  │
    │               │ ⑤ 帧面输出 (watch) │                  │                  │
    │               │──▶ 告警帧 + 「只看不动」注             │                  │
    │               │   **零设备动作** (闭合词表无执行档)     │                  │
    │               │                   │                  │                  │
    │               │        ⑥ 提醒侧消费 (care 心跳拍)      │                  │
    │               │                   │                  │ 未消费信号        │
    │               │                   │                  │─────────────────▶│
    │               │                   │                  │  ReminderCandidate│
    │               │                   │                  │  (guard_alert 档) │
    │               │                   │                  │                  │
    │               │                   │        ⑦ 同源打扰纪律四件             │
    │               │                   │        勿扰 → 专注 → 频率 → 预算      │
    │               │                   │        **守护例外档**:                │
    │               │                   │        默认可破勿扰(可配) +           │
    │               │                   │        「守护告警」标注 +             │
    │               │                   │        频率独立上限(默认 1h≤3)        │
    │               │                   │                  │                  │
    │               │                   │        ⑧ 发起 = 标准回合输入          │
    │               │                   │        [守护告警] [care:gwalert-*] …  │
    │               │                   │        → 会话流普通转写 (照常计费)     │
    │               │                   │                  │                  │
    │ ⑨ 动手？没有。 │                   │                  │                  │
    │   看到 ≠ 动手: 设备动作仍走 tool.device_control 三拍人批
    │   (plan → preview → execute, 不可逆零自主) —— 守护面零执行词表。          │
```

要点（对应四条安全底线 Q41-Q44 继承）：

1. **白名单制**：① 只读消费 L1 `devices.json` 同一本设备表；表外/停用规则
   跳过（`rules_skipped` 如实注因），不编造读数；
2. **只看不动**：⑤ 告警是唯一产出；`tool.guard_watch` 闭合动作词表
   `list_rules`/`watch`/`list_alerts` **零执行**；`device-actions.jsonl`
   （设备执行状态账）在守护巡检后**不得存在**（测试钉死）；
3. **告警 = 关怀候选高优先**（⑥⑦⑧）：走 care 同源打扰纪律，唯一例外档 =
   守护告警（默认可破勿扰 + 破窗必带「守护告警」标注 + 频率独立上限，
   `care_rules.json` discipline 可配收起例外档）；
4. **动手仍走人批**（⑨）：设备动作永远走 `tool.device_control` 三拍
   （plan → preview → execute，reversible 审批档 / irreversible 永逐次人批 /
   money 永逐次人批 + IM 卡）。

## 二、提醒闭环时序（K2 消费端，同源纪律）

```text
tool.weather (只交信号)          weather-signals.jsonl        care 引擎
      │  读数 → ReminderSignal        │                        │
      │──────────────────────────────▶│ append-only            │
      │  (不决定发不发)                │                        │
      │                               │  心跳拍: 未消费信号      │
      │                               │───────────────────────▶│
      │                               │   ReminderCandidate     │
      │                               │   (reminder 档)         │
      │                               │                        │
      │                同源纪律四件 (与规则同账):                │
      │                勿扰窗口 / 专注抑制 / 频率上限 / 关怀预算  │
      │                               │                        │
      │                发起 → 标准回合输入 [care:wxsig-*] → 会话流
      │                说不 → 台账留痕 (裁决完即消费)
      │                拦下 → 候选不丢 (不标消费, 窗口滑过重试)
      │                已裁决 → 台账去重 (不重发不重复计数)
```

## 三、测试证据（本批新增 22 例, 2026-10-18 实测）

| 套件 | 例数 | 钉住的行为 |
|---|---|---|
| `crates/capabilities/tools/src/device/guard_watch.rs`（单元） | 5 | 阈值矩阵（温度上/下限严格不等 + 恰等边界 + 缺读数三态）/ 门磁 + 异常离线 / 时序①连续命中窗（跨度不够/断过 = 不命中）/ 时序②生效时段（跨午夜 = 起 > 止）/ 规则整条判定（时段外不命中, 时序未满不告警） |
| `crates/capabilities/tools/tests/guard_watch.rs` | 6 | 规则 StoredDoc 拒开（畸形/串档/未来版本/坏条目/重复 id 一律拒开拒写）/ **只看不动断言**（命中只产告警 + `device-actions.jsonl` 不得存在 + 词表零执行 + 执行词拒绝即帧）/ 开关 fail-closed（依赖 DEVICE_SENSE, 双闸显式 `"1"` 才开）/ 白名单外跳过不编造 / 告警账 round-trip（逐行 StoredDoc）/ 设备表拒开整帧拒绝 |
| `crates/engine/organ/tests/care_reminder.rs` | 7 | 信号→候选→标准回合全链 / **打扰纪律同源**（提醒照拦勿扰 + 与规则同账频率）/ **守护例外档**（破勿扰 + 「守护告警」标注 + 台账双留痕）/ 例外档可配（`guard_alert_breaks_quiet=false` 收起）/ **频率独立上限**（不占普通预算 + 自己有界）/ 消费位（发起标消费 / 拦下不丢 / 台账去重）/ 说不也裁决消费 |
| `crates/engine/runtime-assembly/tests/care_reminder_face.rs` | 4 | 跨 crate 全链（信号账→消费桥→引擎→标准回合输入）/ 守护告警高优先候选（标注 + 破窗前缀）/ 跨重启台账去重 / **自述关怀节天气提醒计数**（真值计数 + 缺账 0 如实） |
| `crates/capabilities/tools/src/self_status.rs`（既有自述测试随行） | — | 关怀节扩 `weather_reminders` 列（fixture 真值 + 缺席显式 null + 原因） |

既有套件零回归：本批对既有测试只做**机械补位**（`CareRecord` 台账新增来源档列
`kind` 的字面量补位 4 处：organ tests/care.rs 2 处、rules.rs 单测 1 处、
cli panel_care_traits_real_chain.rs 1 处；`CareDiscipline` 新增守护例外档字段
全走 `..default()` 缺省, 既有字面量零改动）；全量口径见
`docs/03-reference/test-baseline.md`（本批不代改真源）。

## 四、宪章门禁证据（2026-10-18 实测, 工具链 rustc 1.97.1 / cargo 1.97.1）

| 门禁 | 命令 | 实测 |
|---|---|---|
| 测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | **322 套件目标 / 8508 passed / 1 failed / 52 ignored**（321 套件全绿；唯一失败 = `object_access_judgment::shell_mounted_object_policy_replaces_the_default_face`, **负载 flake**——并行负载下 AppContainer 起 `cmd.exe` 报 os error 2, **隔离复跑 13/13 全绿**（flake 定性, 非本批回归；该面非本批战场）） |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | **0 警告** |
| 格式 | `cargo fmt --check` | **0 diff**（收尾时复测; 施工期间他线在途文件的 fmt 债已代跑 rustfmt 清零, 见报备） |
| 中性表述 | `scripts/check-neutral-terms.ps1` | **零命中**（40 changed files 全中性; 他线一行来源措辞（含禁词，打码不引述）已按零债纪律改为「旧实现」） |
| LOCKED 5 项 | 9 锚本体 / 13 键 / 3 不可变脊柱 / workspace.version / R11 baseline | **0 触碰** |
| 新外部依赖 | — | **0 引新**（`Cargo.lock` 0 行 diff 由本批代码面产生; 他线换核批的依赖变更不在本批） |

本批新增测试 22 例（5+6+7+4, 见上表）；既有套件零回归（321 套件全绿, 唯一失败为
他线面的负载 flake 且隔离复绿）；全量口径以 `docs/03-reference/test-baseline.md`
为单一真源, 本批实测值如实上报不代改真源。

## 五、0 装诚实注记

- 时序条件的历史观测与消费位是**进程内值**（重启清零）：`sustain_minutes > 0`
  的规则重启后重新累计；已裁决信号由台账去重兜底不重发，台账有界（32 条）之外的
  极老信号理论上可重裁决，由频率上限兜底不刷屏（如实注明，不冒充完美去重）；
- mock 适配器 fixture 无门磁读数：门磁规则对 mock 设备 = 无法判定（不告警
  不编造），真实协议适配器后插；
- 守护例外档默认破勿扰是**设计裁定**（安全告警要到得了），可配收起；
  专注抑制/关怀预算照常生效（例外档只破勿扰一项）；
- 真实协议族（本地桥/云桥/直连）与 live 天气档仍**留插槽不接**（本批零改动）。

## 六、跨线报备（最小改动 + 理由）

1. **机械补位（编译断点, 零语义）**: `CareRecord` 台账新增来源档列 `kind`
   （旧账缺省 `rule` 语义不变）——字面量补位 4 处：
   `crates/engine/organ/tests/care.rs`（2）/ `crates/engine/organ/src/care/rules.rs`
   （单测 1）/ `crates/adapters/cli/tests/panel_care_traits_real_chain.rs`（1）。
2. **代补他线在途编译断点（按对方引用口径的最小面）**:
   `crates/engine/runtime-assembly/tests/device_control_status_probe.rs`
   （根级 `SelfStatusSource` 引用 + `last_action.clone()` 不移出）+
   `crates/engine/runtime-assembly/src/lib.rs` 根级补 `SelfStatusSource` 再导出。
3. **同形镜像随行（合同演进, 断言旁注理由）**: `tool.care_schedule` care_list 帧
   纪律节随 `CareDiscipline` 加守护例外档三列、台账加 `kind` 列 —— 镜面照跟:
   `crates/adapters/gateway/src/care_panel.rs`（`CareDisciplineDto` +3 / `CareRecordDto`
   +1）+ `crates/adapters/cli/src/gateway_panels.rs`（两个 mapper 补列）+
   `crates/adapters/gateway/tests/panel_care_face.rs`（2 fixture + 2 键面清单演进,
   演进理由注在断言旁）。**status 六列契约不动**（`weather_reminders` 归
   `tool.self_status` 关怀节, 面板 DTO 暂不投影, 差列文档注记如实）。
4. **中性表述硬规则**: `crates/capabilities/tools/Cargo.toml`（他线换核批注释
   一行来源措辞（含禁词，打码不引述）→「旧实现」, 扫描零命中）。
5. **代跑 rustfmt（机械空白, 零语义）**: 他线在途文件的 fmt 债随门禁清零
   （`gateway/src/im_bridge.rs` / `im_device.rs` / `lib.rs` / `tests/im_device_cards.rs` /
   `tools/src/image_ops.rs` / `tests/image_ops_core_swap.rs` 等）——收尾复测
   `cargo fmt --check` 0 diff。
