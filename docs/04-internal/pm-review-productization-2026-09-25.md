# Apeireth 产品化审视：离小白「好用省心」还有多远

```text
[Document-Meta]
Document:    docs/04-internal/pm-review-productization-2026-09-25.md
性质:        产品经理视角的产品化差距评审（内部文档）
评审日期:    2026-09-25
评审方式:    README/INSTALL/用户文档走读 + CLI 实机实测 + 桌面端/分发链/可靠性三路深挖
评审对象:    v2.0.0-rc.1（18-crate workspace + companion-desktop）
```

---

## 一句话结论

**这是一个内核工程 9 分、产品外壳 2 分的倒挂产品。** 离「好用」差的不是代码量，而是**接线、默认值和叙事**——集中做 4–6 周产品化即可让小白 10 分钟装上、3 分钟体验到「它记得你」；离「省心」还差**一整个成熟度周期**（备份/自愈/升级三件套 + 发版节奏 + 平台覆盖），约 3–6 个月专注工作。

## 1. 核心发现：产品有两副面孔，而且只把「工程师那副」放在了门面上

| | 面孔 A：门面（README/INSTALL） | 面孔 B：实际产品（companion-desktop） |
|---|---|---|
| 形态 | 「AGI 操作系统」+ 数学公式 + 微秒基准 | Tauri 桌面伴侣 App，有火焰开场动画 |
| 获取方式 | clone 源码 → 装 Rust/VS Build Tools/cmake → 编译 18 crate（**1.5–4 小时**） | **NSIS 安装包已构建**（`Apeireth Companion_2.0.0-rc.1_x64-setup.exe`），双击即装 |
| 配置 | 15+ 个环境变量旋钮 | 首启向导 GUI 选 provider 填 key（`frontend/companion-desktop/src/lib/components/FirstRunWizard.svelte`） |

**但发布链把面孔 B 藏起来了**：`.github/workflows/publish-release.yml` 创建 GitHub Release 时只写 notes、**挂 0 个二进制资产**；`INSTALL.md` 通篇没有「下载安装包」。小白来到门口，只看得到编译工具链。

## 2. 小白旅程逐环节打分

| 环节 | 现状 | 小白期望 | 差距 |
|---|---|---|---|
| **获取** | Release 无安装包；packaging/ 九种包全无出口 | 下载 → 安装 | 🔴 断 |
| **安装** | 12–15 步工具链编译，验证要跑 3418 项测试 | 双击下一步 | 🔴 断 |
| **首次运行** | 实测：`apeireth session` 打印两行 `canonical runtime ready` 就退出；README 的 `chat` 实测报 `chat requires a prompt`；`bundle` 实测 `unknown command`（README 五步上手 **2 步必败**） | 看到「它」，说上话 | 🔴 断 |
| **配置模型** | README 快速上手**只字未提 API key**；Docker 路线的 `.env.example` 变量名（`APEIRETH_LLM_*`）代码根本不读，还强制填产品用不到的 `POSTGRES_PASSWORD` | 填一个 key | 🔴 断 |
| **核心价值时刻** | 没有任何文档写「第一次对话应看到什么」；核心记忆能力（主动召回/偏好学习/器官）**默认全关**（「默认拒绝」旋钮） | 3 分钟内感到「它记得我」 | 🔴 断 |
| **日常使用** | 桌面 GUI 不错（设置页即配置源、审批卡、Ember 呼吸微光）；但 key 每次启动重输、语音默认不接线 | 顺手 | 🟡 半通 |
| **长期/升级** | 无更新提示；rc.1 后近 30 条关键修复全躺在 `[Unreleased]`（装了 rc.1 就带着「开始菜单启动必失败」等 3 个已知 bug 跑）；无备份入口；卸载勾「删除数据」= 记忆永久蒸发 | 不丢数据、自动更 | 🔴 断 |

## 3. 五个最伤的问题（按对小白的伤害排序）

**① 终身记忆没有安全网——对这个定位是致命组合。** 库里明明有导出 API（`crates/engine/memory/src/lib.rs:512` `export_streams_jsonl`）但 CLI/UI **零入口**；数据散落三处（桌面 `%LOCALAPPDATA%\Apeireth`、CLI 在「运行目录/.apeireth」相对路径——`crates/adapters/cli/src/lib.rs:481`，根目录那坨 `apeireth_gateway.db` 就是事故化石）、卸载可一键删光且不可逆。一个卖点是「帮你记住一辈子」的产品，用户却不知道记忆存在哪、无法备份、换机即失忆。

**② 上手文档「照做必败」。** README 五步上手里 `bundle` 命令不存在（`crates/adapters/cli/src/main.rs` 无此派发，已实测）、`chat` 不是「交互式会话」是单轮命令、缺 API key 步骤；hello-world 三处指引两处路径过期；`custom-llm.md` 还在教跑 legacy/ 里的 v1 命令。**小白按文档走完的成功率接近于零。**

**③ 宣称与实现脱节——信任债务。** README 卖点「P2P Mesh/Noise_XX 加密漫游」，`crates/foundation/protocol/src/p2p_mesh.rs` 模块头自述是**明文 stub**；「因果世界模型/主动关怀」的实现在 legacy/ 未移植（`docs/02-guides/user-manual.md:73`）；SDK 导出函数走 `unimplemented!()`。讽刺的是这个产品自己的哲学锚写着「S-2 实事求是、O-5 不假装」——对一个品牌即信任的伴侣产品，这是最贵的债。

**④ 「省心三件套」全缺**：备份/导出无入口；侧车崩溃后 UI 停在 Failed 不自愈、无 crash report（崩了只能翻日志文件）；无更新检查 + 发版停滞（`RC1_HANDOFF.md:293` 承认 brew/scoop 的 SHA256 是占位符）+ NSIS 同版本静默安装不覆盖。

**⑤ 文档与仓库是「开发现场」不是「发布产品」**：crate 数三套口径（13/17/18）、测试数一行四个口径（3418/3406/3120/23806）、`cd Apeireth` vs `cd apeireth-rust` 打架、内部黑话外泄（「leader 亲自产出」「W2 批」「台账 #47」）、`start_tauri_dev.ps1:3` 硬编码 `D:\apx\` 必失败、reports/ 里 1058 个 AI 工作日志被 git 跟踪。
> ✅ **2026-09-25 更新**：评审时发现的另一项风险——`artifacts/_attic/` 内与本产品无关的第三方安装介质及派生工作文件（外来重物）——**已完成清理**（连同相关工作区、过时工作文档与内部代号一并整理；来源归属见 NOTICE/THIRD-PARTY-NOTICES）。

## 4. 距离量化

| 维度 | 评分 | 依据 |
|---|---|---|
| 内核工程质量 | 9/10 | 3418 测试 0 失败、clippy 0 警告、安全审计 13 High 全修、审批崩溃安全 |
| 产品可达性 | 2/10 | 安装包存在但没人拿得到 |
| 首次体验 | 2/10 | 5 步上手 2 步坏，验证输出无情感回报 |
| 日常好用 | 5/10 | 桌面 GUI/向导/审批卡是真材实料 |
| 省心度 | 2/10 | 备份/自愈/升级三缺 |
| 文档可信度 | 3/10 | 矛盾密集、宣称与实现脱节 |

- **离「好用」≈ 4–6 周 P0 工作**：把已有的东西接线、默认化、讲清楚（安装包挂上 Release、README 重写成人话、修掉必败命令、首启向导补完人工点击流、核心记忆默认开）。
- **离「省心」≈ 3–6 个月**：备份/导出/恢复产品化、崩溃自愈与诊断包、更新检查与稳定发版节奏、macOS/Linux、shell 沙箱（目前开启后全盘零拦截，台账 #44「非沙箱实锤」）。

## 5. 建议路线（P0 → P2）

**P0（让小白 10 分钟可用，先做这些）**

1. 把 `Apeireth Companion_2.0.0-rc.1_x64-setup.exe` 挂上 GitHub Release（修 publish-release.yml 传资产）——**这是 ROI 最高的单件事**
2. README 重写为用户视角：一句话定义（「接上 API key 就能对话，它会记住你」）→ 下载安装 → 首次对话预期输出；数学公式/基准表下沉到 docs
3. 修掉全部「照做必败」命令（bundle 要么实现派发要么删宣称），建一条「文档命令 → CI 实跑」一致性检查
4. 首启向导是产品的心脏：补完挂账 #2 的人工点击流（一次测试就抓出过 5 个真 bug），key 走钥匙串持久化
5. 默认体验反转：核心记忆能力默认开或向导里给「推荐配置」一键预设——「默认拒绝」应只针对危险能力（shell/fetch），不应对产品核心价值

**P1（让日常好用，1–2 个月）**

6. 备份三件套：设置页「导出我的记忆」（接线现成的 `export_streams_jsonl`）+ 卸载前自动备份提示 + 换机导入
7. 侧车自愈 + 崩溃诊断包（一键打包日志）；数据位置统一并在设置页明示「你的记忆在哪」
8. 把 `[Unreleased]` 的修复发成 rc.2；加更新检查；补 brew/scoop 真哈希或先撤掉这两条渠道
9. 宣称对齐：P2P Mesh/世界模型/主动关怀降级为「路线图」或补实现——把「0 装」纪律从代码注释推广到门面文案

**P2（让省心，一个季度）**

10. macOS/Linux 打包验证、自动更新、shell AppContainer 沙箱、opt-in 崩溃上报、仓库卫生（.harness 痕迹、1058 个 AI 日志；~~_attic 重物~~ 已清理）

## 结语

值得强调的好消息：**原料全都存在**——安装包、首启向导、导出 API、九种打包脚本、装机 E2E、诚实的验证台账（live-verification-ledger）、修复率 100% 的安全审计。团队的工程纪律和诚实文化是同类项目罕见的资产。现在缺的不是能力，是**一次从「开发者仓库」到「用户产品」的视角切换**：把面孔 B 摆上门面，把已有的半成品接完线，把宣称收敛到实现——这恰好也是这个产品自己的哲学锚（实事求是、不假装）要求的事。

---

## 附录：评审中的实测证据（节选）

| 实测命令 | 文档声称 | 实际结果 |
|---|---|---|
| `apeireth session` | 「欢迎信息 + 启动 session」（INSTALL.md:72） | 打印 `canonical runtime ready / providers: …` 两行即退出 |
| `apeireth chat` | 「启动本地命令行交互式伴侣」（README） | `chat requires a prompt`，单轮即退，无 REPL |
| `apeireth bundle --output-dir …` | 「一键生成随身 U 盘生命体」（README §5） | `unknown command`，派发不存在 |
| `apeireth --help` | 全部命令 | 缺 `dream/council/subagent/nightwatch`（实现了但未列入帮助） |
| Docker 路线 `.env.example` | 5 个配置项 | `APEIRETH_LLM_*` 代码 0 命中；`POSTGRES_PASSWORD` 强制必填但产品用 SQLite |
| Release 资产 | 发布分发 | `publish-release.yml` 只写 notes，0 个二进制资产 |
