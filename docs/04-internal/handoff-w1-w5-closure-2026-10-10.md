# 交接报告 — W1 沙箱 / W2 接线 / W3 真缺口 / W5 SDK 全线收官 (handoff-w1-w5-closure-2026-10-10.md)

> **给谁看**: 接续本工作线的人或 AI。**你不需要读过任何对话记录 —— 本文自足**。
> **上一份交接**: `handoff-w2-wiring-2026-10-06.md`（W2 线已宣告完成，本文是其后的
> 全 Span 收官报告：W1 沙箱本体 → W2 全线 → 子代理运行时 → W3 真缺口 → 守夜人 →
> W5 SDK → 小件队列）。
> **状态总纲**: 主线全部闭环，剩余 = 需外部条件或专门批次的中件（§5 逐项列阻塞点）。

---

## 1. 本 Span 交付总账（16 批，台账 #45–#64，全部推送 0/0）

| # | 批次 | 一线 |
|---|---|---|
| 45 | dreaming 接线 + 凭据收口 + W3 报告 §9 修正 | W2 §4.1 |
| 46 | 真缺陷 #4（thinking max_tokens fail-loud）+ 挂账 #9 核销 | 质量 |
| 47 | council 改造（每轮评审器 → 决策环节顾问，7→3 旋钮）| 认知 |
| 48 | W1 P3 纵深（凭据绊线 + 口令模式 + 系统命令守门）| W1 |
| 49 | **W1 沙箱本体**（AppContainer 全链路 + 双探针墙实证）| W1 |
| 50 | W2 §4.2 partner 接线 + principles 有意不接 + W1 涟漪断言反转 | W2 |
| 51 | W2 §4.3 morphology + education 双件 | W2 |
| 52 | W2 §4.4 吸收批四算法（认知体操）| W2 |
| 53 | worktree_sandbox 装饰器 + **W2 全线收口宣告** | W2 |
| 54 | 生产 Orchestrator（`LlmSubagentOrchestrator` + `apeireth subagent`）| 子代理 |
| 55 | 三洋葱判定模型（`onion_gate`，v1 donor 移植 + r177 证明平移）| W3 |
| 56 | 三洋葱物理执行面（`OnionLayerHook` 接入生产治理管线）| W3 |
| 57 | community 移植（三件 + 9 测）| W3 |
| 58 | **onering 考古纠错**（本体=context_ledger 打捞件，误判修正）+ CLI 消费 | W3 |
| 59 | experiment_field 机制移植（状态机 + VMRunner 0 装 + 学习 sink）| W3 |
| 60 | 守夜人 Nightwatch（五件组合审计器 + `apeireth nightwatch`）| 守夜人 |
| 61 | W5 SDK HTTP 真传输（invoke_tool 换 reqwest，R21 半场）| W5 |
| 62 | 小件：审批卡 sandbox 徽标前端渲染 | W1 §2.4 |
| 63 | 守夜人空闲调度（`NightwatchIdleScheduler` + `--watch` 后台守护）| 守夜人 |
| 64 | community 生产消费（`all_facts` 契约扩展 + 检索前置，§1 全闭环）| W3 |

## 2. 各战线四级口径终局

| 战线 | IMPLEMENTED | PRODUCTION WIRED | DEFAULT ENABLED | 备注 |
|---|---|---|---|---|
| W1 shell 沙箱 | ✅ | ✅ | **默认开**（`APEIRETH_SHELL_SANDBOX=0` 裸跑）| AppContainer；用户空间零访问已双探针实证 |
| W2 认知模块 | ✅ | ✅ | 默认关（逐模块 `APEIRETH_ENABLE_*` 旋钮）| #53 已宣告收口 |
| 子代理运行时 | ✅ | ✅ | 显式命令（`apeireth subagent`）| plan 步人工审批 fail-closed |
| 三洋葱 | ✅ | ✅ | 默认关（`APEIRETH_ENABLE_ONION_LAYER`）| 真 Ed25519 留 v2.1 |
| community | ✅ | ✅ | 默认关（`APEIRETH_ENABLE_COMMUNITY_TRIAGE`）| #64 全闭环 |
| onering | ✅ | ✅（CLI 前端）| 默认关（`APEIRETH_ENABLE_ONERING_LEDGER`）| 多前端见 §5 |
| experiment_field | ✅（机制） | ⏳（机制层）| — | 真 VM 后端 0 装（smol-vm/libkrun）|
| 守夜人 | ✅ | ✅ | 显式命令 / `--watch` | report-only 红线 |
| W5 SDK | ✅（HTTP） | ✅（HTTP） | — | WS 半场 stub（服务端端点未建）|

## 3. 验证状态（接手后先跑这些）

```bash
cargo test -p apeireth-tools-canonical -p apeireth-memory -p apeireth-core \
  -p apeireth-orchestration -p apeireth-runtime-assembly -p apeireth-cli -p apeireth-sdk
# 本 Span 终点实测: 上述全绿（tools 229 / memory 830+ / assembly / cli 68 / sdk 61 等，
# 累计数千测 0 failed; clippy --all-targets -D warnings 0 警告）
cd frontend/companion-desktop && pnpm check && pnpm test   # svelte-check 0 错 + 15/15 套件
python scripts/check_doc_caliber.py docs/04-internal/live-verification-ledger.md <新文档路径>
```

**git 状态**: origin/main = HEAD（0/0）；唯一未提交区 = `research/`（主人的 leak-probe
实验在途工作，**全程未动，不要提交**）。

## 4. 推送与提交纪律（会咬人）

- **推送必须走代理**: `git -c http.proxy=http://127.0.0.1:7900 -c https.proxy=http://127.0.0.1:7900 -c http.version=HTTP/1.1 push origin main`；代理会瞬断，重试循环（本轮亲历 16 连败后恢复）。
- **`git add` 显式路径**；新 `scripts/*.py` 需 `-f`（`.gitignore:208`  blanket `*.py`）。
- **ledger 只追加不替换**（曾误替换行 #44，已复原；append-only 是红线）。
- **commit message 三段式**（为什么/做了什么/测试结果）。
- **依赖变动必带 `Cargo.lock`**（曾漏 → CI `--locked` 破）。
- **fmt 重排后 edit 会被"文件已变"挡** → 重读再贴（本 Span 出现 10+ 次，规律稳定）。

## 5. 剩余队列与阻塞点（按可操作性排序）

| 项 | 阻塞点 / 解锁条件 |
|---|---|
| onering Web/SSE 前端 | 需 GatewayState 管线改造（runtime 的 store 句柄不透出）；Lark/Telegram = 新 adapter crate |
| SDK WS 半场 | 需 gateway WS 端点（`/v1/stream` 契约服务端）先行 |
| 真 Ed25519 多签 | v2.1 计划项（`onion.rs` 0 装占位注释已载明）|
| smol-vm/libkrun VM | 外部依赖；`VMRunner` trait 口已备 |
| experiment_field CLI 命令面 | 需 orchestrator 触发链设计（机制件已在 assembly）|
| 审批卡 sandbox 徽标目检 | 主人真机 click-through（`frontend/companion-desktop/docs/first-run-click-through-checklist.md`）|

## 6. 防复蹈教训（本 Span 真机咬出的坑）

1. **AppContainer 进程创建校验 profile 注册**：纯派生 SID → 误导性 os error 2；`CreateAppContainerProfile` 幂等修复（12 组合诊断矩阵，`appcontainer.rs` 模块头注是正史）。
2. **环境块三连 203**：乱序→排序 / 缺 `CREATE_UNICODE_ENVIRONMENT`→补旗 / 瘦块→档案六件变量。
3. **`\\?\` 前缀 cwd 掉 %WINDIR%** → lpCurrentDirectory 剥前缀。
4. **win.ini 自带 AAP (I)(RX)**：系统文件自我放行，墙边界 = 用户空间（零 AAP ACE）。
5. **thinking 模型 max_tokens 预算**（真缺陷 #4）：reasoning_content 吃光预算 → 留空 content；omit max_tokens 或 ≥2048。
6. **考古纠错 ×2**：v1 沙箱（real.rs 是真 Docker）/ onering（context_ledger 早是打捞件）——"未移植"判定必须全量扫模块，不能凭印象。
7. **evidence 推断语义**：Inference + confidence<0.7 是**合法通过**（PassInferred），不是失败。
8. **`forbid(unsafe_code)` crate 里 `std::env::set_var` 违禁** → 测试用纯函数/唯一标记值。
9. **wiremock 测试翻面**：stub→真传输时，旧断言（NotImplemented）必须翻，别留自相矛盾。

## 7. 接手第一步（建议顺序）

1. 跑 §3 全量验证 + `git status` 确认只有 `research/` 脏。
2. 读 `live-verification-ledger.md` #45–#64（每行的复现命令可直接跑）。
3. 从 §5 表第一行开工（onering Web 前端：GatewayState 加 ledger 句柄 → native_chat 完成后记账，CLI 侧 `onering_record_turn` 是现成模板）。
4. 任何新模块按**五件验收门**：默认关旋钮 / 默认关不变测试 / 效果可见测试 / 四级口径行 / 台账条目。
