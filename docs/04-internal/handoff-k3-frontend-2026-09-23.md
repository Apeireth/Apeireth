# K3 前端产品化批次交接文档（主会话写，2026-09-23 额度收官）

> **给谁看**：接手 Apeireth 前端产品化的下一批工程人员 / 下一个 AI 会话。你不需要读过 K3 的任何过程——这一份就是全部上下文。
> **HEAD 状态**：`main` @ `ffffac8f`（2026-09-23 文档批）。远端只有 `jimmy`（github.com/Jimmyxiao2009/Apeireth-rust.git），`main` 未设上游跟踪——**是否推送由主人决定，接手人不要擅自 push**。
> **何时写**：K3 七项任务全部落地并逐项视觉验收通过、台账 #29/#30 登记、文档批回写完毕之后；主会话额度将尽，主人可能换 AI 继任。
> **关系文档**（按读序）：本文 → `engineering-log-2026-09-22.md`（30 支 commit 全索引）→ `live-verification-ledger.md` #29/#30（验收真账）→ `frontend/design-preview/peer-gap-and-frontend-plan.md` §7（施工结果对照）→ `frontend/companion-desktop/docs/k3-team-kickoff-prompt.md` + `k3-steering-playbook.md`（K3 的施工组织方式，下一批可复用）。

```
[Document-Meta]
Document:        docs/04-internal/handoff-k3-frontend-2026-09-23.md
Version:         1.0
Last-Modified:   2026-09-23
Status:          🟢 活跃（K3 → 下一批次的接手入口）
Author:          K3 施工集群主会话（团队负责人/验收人）
```

---

## 0. 先说三句实话

1. **这个仓库的 ✅ 全是真的，挂账全是明写的**——`live-verification-ledger.md` 是权威。开工前查它：别重测已绿的，别把挂账的当已验的。
2. **主人有一条硬约束：前端最终效果必须经过视觉验证**——Edge CDP 截图 + 亲眼审图，不通过就打回重做（K3 打过两轮，都返工通过了）。接手人继承了这条约束，设施是现成的（§4），别绕。
3. **主人的偏好是「宁可慢，不能错，更不能误解原意」**——方向性不确定就停下来问，别替主人拍板；主人拍了板的方向（如 §5 清单）要回写文档防漂移。

---

## 1. K3 批次总结报告（2026-09-22/23）

**一句话**：把前端从原型推进到产品化壳——微信式三栏聊天壳 + 治理/记忆两卷宗 + presence 数据契约全线贯通，七项任务全绿。

| # | 任务 | 主 commit | 验收方式（全部主会话亲自截图） |
|---|---|---|---|
| ① | `presence_state` 后端事件（heuristic_v0） | `20cf3d1b`/`487b7141`/`fa011d07` | curl 实收 60s 心跳帧逐字段核契（台账 #29） |
| ② | T0 聊天壳：三栏主从 + 个性化背景 + 审批卡对话内化 | `5021157e`…`63557256`（14 支） | 两轮打回返工后通过；金光晕呼吸双帧实测 opacity 差 149 |
| ③ | 治理卷宗四 tab（Deep-Ops 调首次实拍） | `a2ecd244`/`dc821e55`/`40f78845` | 真后端 grants 3 / traces 46 / audit 100 |
| ④ | Ctrl+K 命令面板（19 条）+ 打断按钮 | `d1252744`/`24c862ec` | 拼音过滤实测；打断走 mock 慢流式（诚实标注） |
| ⑤ | 底部状态条四指标 | `f9fecc02`/`6f4160b8` | 安静态 + 杀后端断线级联双状态实测 |
| ⑥ | 记忆卷宗主从化 + 日记纸面空态（Archive 调首次实拍） | `104d9a2b`/`5497ae2a`/`78e01e36` | 真后端 34 条 episode；forget 确认 + 409 冲突卡 |
| ⑦ | 显影接线（presence.ts 重写订 presence_state） | `7026a3a7` | 真后端 60s 心跳驱动余烬 amplitude 0.65→0.2 |

**批次共 30 支 commit**：前端 24 + 后端 2 + 文档 4（另有 2026-09-23 台账 `e7a6e9ff` 与文档批 `ffffac8f` 两支收官）。**质量门（终点复核）**：`cargo test --workspace` 3406 passed / clippy 零警告 / 前端 `npm run check` 0/0 / `npm run test` 15/15。逐支索引见工程日志。

**产品形态现状**：默认主题 heritage-void（主人供的静态星空山脉图，已入库 `frontend/companion-desktop/public/assets/themes/`）；黑洞实时场景降级为 `night` 主题可选；背景支持自定义上传（IndexedDB，换原点需重传——诚实标注）；accent 配色四套（gold 默认/deep-space/sage/bone，机器断言不占存在金家族）。

---

## 2. 仓库现状真账

**已入库**（K3 全部成果 + 文档批）：代码、设计文档（`docs/design/00-PHILOSOPHY.md`、`01-DESIGN-SYSTEM.md`）、令牌 JSON、gap 计划、工程日志、K3 开工提示词与控场手册。

**未入库（刻意留的 scratch，接手人可自行清理或入库）**：

- `artifacts/k3-*.png` / `k3-*.mjs`（26 张验收截图 + CDP 脚本）——验收证据与复验设施，**建议保留**；
- `artifacts/gov-shots/`（mock gateway + 截图）、`artifacts/presence-wiring/`、`artifacts/k3-assets/`；
- `frontend/companion-desktop/.tmp-*.mjs`、`shot-*.png`、`.tmp-vite.log`——临时探针，可删；
- `artifacts/` 里还有一大批 K3 之前的 前代产品 装机/窗口调试 ps1/png——历史 scratch，与 K3 无关。

**别人的未提交改动（别碰）**：`research/llm_judge/src/main.rs`（+467 行）、`research/.gitignore`、`research/logs/llmjudge-probes-expanded-summary.md`、`list-orig.txt`、`@AutomationLog.txt`。K3 全程未动这些，接手人也先别动，等主人说。

---

## 3. 环境与工具链（血换来的事实，照抄别再踩）

- **cargo/rustc 1.97.1 在 `~/.cargo/bin`**——Git Bash 默认不在 PATH，每个 Bash 调用开头 `export PATH="$HOME/.cargo/bin:$PATH"`。
- node v24.15.0 / npm 11 在 PATH；pnpm 在 `C:\Users\31683\AppData\Roaming\npm\pnpm.cmd`（本仓库前端用 npm scripts 即可）。
- **质量门**：前端 `npm run check` / `npm run test`（cwd `frontend/companion-desktop`）；后端 `cargo test -p apeireth-gateway`、`cargo clippy -p apeireth-gateway --all-targets -- -D warnings`；全量 `cargo test --workspace`。
- **手动起 gateway**：`./target/debug/apeireth.exe gateway serve --port 8080`，约 4s 就绪，`/health` 返回 `{"status":"ok","execution_owner":"apeireth-runtime::canonical"}`。
- **前端存储键**：配置 `apeireth-config`、会话 `apeireth-conversations`、首启 `apeireth-first-run-done`（验收脚本常需预置/清理 localStorage）。
- **URL 开发参数**：`?theme=`（heritage-void 默认 / night / essence / day / ocean / forest / paper）、`?hour=`、`?mode=`、`?drawer=governance&govtab=approvals|grants|guard|audit`、`?drawer=diary`——验收截图全靠这些直达目标状态。
- **服务器纪律**：Git Bash 里 `&` 起的进程在工具调用结束后**不死**，必须用完按 PID `taskkill //F //T //PID <pid>` 精确清理（`netstat -ano | findstr :<port>` 查 PID）；**绝不碰 msedgewebview2.exe——那是 Kimi 本体**。

## 4. 视觉验收设施与流程（主人硬约束的落地方式）

- **套路**：Edge `--headless=new --remote-debugging-port=<端口>` + Node 原生 WebSocket CDP 脚本截图，ReadMediaFile 亲审。**Edge 简单 `--screenshot` 模式在带 drawer 参数的页面上会挂**（network service crashed）——一律走 CDP 脚本。
- **现成脚本**（`artifacts/`，未入库但可用）：`k3-k4-palette.mjs`（命令面板）、`k3-k4-interrupt.mjs`（打断，需 mock）、`k3-k5-statusbar.mjs <quiet|down>`、`k3-k6-memory.mjs` / `k3-k6-detail.mjs`、`k3-k7-presence.mjs`（等 60s 真实心跳）、`gov-shots/mock-gateway.mjs`（8080 慢流式 mock，400ms×20 帧）+ `gov-shots/gov-shot.mjs`。
- **CDP 脚本纪律**：脚本结尾必须自己 `ws.close(); edge.kill(); process.exit(0)`，否则 Bash 超时后 Edge 变孤儿（只能按调试端口查 PID 杀）。
- **验收口径**：能真后端就真后端；只有后端无法产生的状态（如慢流式打断）才用 mock，且必须在报告里诚实标注。截图存 `artifacts/k3-*.png` 命名传统。

## 5. 待主人拍板清单（接手人不要自行决定）

1. **附录 B-12 四个提案值**（`01-DESIGN-SYSTEM.md`）：Archive `inkGold #8a6d1f` / 深舱三色（紫 `#8f7ad9`、青 `#3fb0c4`、红 `#d95f55`）/ accentUi 目录三配色 / 行宽 31ch→35em 实现注。已落地经视觉验收，数值待主人确认后转 ✅。
2. **是否点亮 `subscribeCompanionEvents`**（legacy 伴随体订阅，与 ⑤ 修掉的 presence 订阅同患 mount 门恒假）——点亮会唤醒「他说」主动开口整条链，是产品行为变化，**必须主人点头**。
3. **台账挂账 #2/#4**：审批卡对话内**真后端**全闭环（需主人给真 LLM key 触发工具审批）+ UI 点击流人工实测——留主人真机。
4. 会话分支后端数据模型（gap-plan §5，第二梯队 backlog 的前置）。
5. 「批准当前待签文书」命令不做跨会话批发（原则 4 已定，若主人想改需明确）。
6. 00-PHILOSOPHY §10 措辞已按实现更正（kebab-case → 小写下划线串），主人若不同意可回退 `ffffac8f` 中该段。
7. `inspiration-2026-09/SYNTHESIS` 全仓缺失——不阻塞，主人若找到请补。
8. `main` 是否推送远端 `jimmy`。
9. **全局浅色/深色模式口径**（2026-09-23 主人真机反馈「最基础的浅色深色模式要有」）：接手人提议把 essence 定位为「浅色模式」并继续做到位（而非另起新主题）；记忆卷宗 Archive 纸面维持此前拍板的「纸面恒纸面」不动。等主人确认方向后连同梯队 6 一起做。

## 6. 下一梯队（未开工，按 前代产品-gap §6/§7 顺序，等主人指令）

1. **桌宠**（常驻投影，00-PHILOSOPHY §5 🟡 提案待校准）——presence 数据已通，是四个投影里最后一个没点亮的；
2. **会话设置抽屉**（P2，`session_settings.rs` 390 行后端已就绪，纯前端活）；
3. **审批卡真闭环联调**（等主人给 key，挂账 #2）；
4. **会话分支**（需 §5 后端数据模型先行，第二梯队）；
5. 看板/终端（独立立项，超 companion 定位的边界问题先问主人）。
6. **主题系统补全**（2026-09-23 主人指示「加入要做的计划」）：day/ocean/forest/paper 四个主题**无实现**（点了无反应），当日已从 THEME_CATALOG/VALID_THEMES/Theme 联合类型删除（0 装底线），`base.css` 休眠的亮色令牌块一并清掉；后续要把它们**真做出来**（各自 data-theme 令牌体系 + 背景资产/实时场景），并与「全局浅色/深色模式」统一规划——主人 2026-09-23 明确要求最基础的双模式，essence 定位待主人拍板（见 §5 增补）。实现时以 essence 浅色令牌为基推导亮色令牌。

**v0 诚实边界（开工前必读，别假装它们存在）**：`empathetic_care` / `ritual` 无生产者（契约空间保留）；initiative 只有预算器（≤3 次/天）无生产者；prompt-overlay 召回命中不上总线（需 runtime 加 TraceEvent 变体）；星尘卡蛰伏（总线无 memory_recall）；warmth=f(d) v1.0 不实现。

## 7. 给下一批工程人员的话

- **0 装**：禁假数据、禁假成功、禁把挂账写成已验。契约没有的能力就做诚实空态（日记页是范本：后端无端点 → 纸面空态契约页）。
- **金色纪律**：金 = 他。UI accent 可个性化，但 `--ap-accent-tab` 跟随主题金家族不动摇，`theme.ts` 的机器断言是守门员，别拆。
- **视觉验收不是形式**：K3 的两轮打回（列表承托、会话头竖排+金晕+行宽）都是截图亲审抓出来的——代码 review 看不出「文字浮在背景上不可读」。交付前先截图给自己看。
- **小步 commit 写理由**：K3 每支 commit 都带规范出处（§x.y），追溯时救命。
- **文档同 PR 演进**：改了数值就回写规范与令牌 JSON（design-tokens.json 曾被写出尾随逗号语法错误——改完 JSON 必须解析校验）。
- **先查台账再动手**：`live-verification-ledger.md` §2 挂账清单是这个仓库的「未测真话集」。

## 8. 文件地图（K3 相关）

| 文件 | 角色 |
|---|---|
| `docs/design/00-PHILOSOPHY.md` | 范式层（主人定稿）；§10 presence_state 契约已落地 ✅ |
| `docs/design/01-DESIGN-SYSTEM.md` | 令牌层；§2.3 accent 分工、§3.1 presence 重接更正、§5.6 三调性实拍校准、§6.1 行宽实现注、附录 B-12 待拍板清单 |
| `docs/gateway-api-contract.md` §8a | `presence_state` 事件契约（✅ 2026-09-22） |
| `frontend/design-preview/peer-gap-and-frontend-plan.md` | 施工层；§7 = K3 七项结果对照表 |
| `frontend/design-preview/design-tokens.json` | 机器可读令牌镜像（语法已修复校验） |
| `docs/04-internal/engineering-log-2026-09-22.md` | K3 批次 30 支 commit 全索引 + 打回记录 + 挂账 |
| `docs/04-internal/live-verification-ledger.md` | 验收权威；#29 = presence_state 后端，#30 = K3 前端批次 |
| `frontend/companion-desktop/docs/k3-team-kickoff-prompt.md` / `k3-steering-playbook.md` | K3 施工组织（任务书 + 控场手册），下一批可复用此模式 |
| `crates/adapters/gateway/src/presence.rs` | presence_state 状态合成模块（821 行，12 单测） |
| `frontend/companion-desktop/src/lib/presence.ts` | 前端 presence 消费链（具名帧订阅 + 显影分级） |
| `frontend/companion-desktop/src/lib/design/tokens.css` | CSS 令牌实况（规范与 JSON 的运行时镜像） |
| `artifacts/k3-*.png` / `k3-*.mjs` | 26 张验收截图 + CDP 复验脚本（scratch 未入库） |
