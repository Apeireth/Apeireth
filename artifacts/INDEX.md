# artifacts/ 验收截图索引

验收证据总账：[`docs/04-internal/live-verification-ledger.md`](../docs/04-internal/live-verification-ledger.md)（台账按行引用本目录路径——**已引用文件不得移动/改名**）。

## 使用约定

- 命名：`cap<批>-<内容>.png`（真机）/ `dev<批>-`（dev 环境）/ `<轮次前缀>-`（早期批次）。
- 新截图直接入根目录；一次性杂件进 `_attic/`。
- 看图工具：ReadMediaFile；大图先看缩略，细节用 region 裁剪。

## 2026-10-11 批（台账 #66–#75，主人逐截图批复验收）

| 批 | 主题 | 验收图 | 验证点 |
|---|---|---|---|
| #66 | 能力中心设置页三页重构 | `cap-cognition-off/on`、`cap-governance-council/off`、`cap-tools-off/warn`（6 张） | 默认全关 / 开三件 / 议会顾问滑杆 / 沙箱关红标警告 |
| #67 | 默认能力说明卡 + 运行时诊断去保存栏 + 开发者选项 Beta 场 | `cap2-dev-beta(-on)`、`cap2-runtime-nosavebar`、`cap2-essence-{cognition,dev,governance,tools}`（7 张） | Beta 开/关双态、运行时页无保存栏、essence 浅色四页不破版 |
| #68 | rc.1 真机安装验收 | `inst-home`、`inst-runtime-probe`、`inst-gov-{approvals,grants,guard,audit}(-tab)`（9 张） | 治理卷宗四页徽标真实数据、深度诊断双绿 |
| #70 | 对话栏位常驻控件（权限/模型迁入输入栏） | `cap3-composer`、`cap3-preset-pop`、`cap3-model-pop`（3 张） | 弹层向上翻、四档菜单 checkbox 语义 |
| #72 | 视觉打磨（六截图四问题） | 台账引用：`cap7-composer`、`cap7-scrolled-bottom`、`cap7-workbench`；同批过程迭代：`cap4/4b/4c`、`cap5/6` | 台宽于列、AI 字号 14.5px、对话收进框架、工具轨迹归位 |
| #73 | 窗框融合实底条 + 工具状态三面翻面 | 台账引用：`cap8-head-frame`、`cap8-head-mid`、`cap9-workbench-fixed` | 全出血暗带实底 rgb(11,13,18)、22 工具「已结束」诚实标注 |
| #74 | 会话分组（项目/联系人）+ 行内菜单 | `cap10-groups/menu/collapsed`（dev 注入全排列）、`cap11-real-groups`（真机兜底组 + 菜单四件） | 折叠区收起持久化、置顶/重命名/归档/删除、双区菜单去重 |
| #75 | 主页窗框融合 + 工作区默认回退 | dev：`dev75-home-{full,top,scrolled}`、`dev75-seeded(-scrolled)`；真机：`cap75-real-{full,top,scrolled}` | 栏顶无黑框、sticky 实底头、旧会话归当前项目组（组头「31683」= 侧车根路径末段） |

## 更早批次速查（台账 #45–#65 时代）

| 前缀 | 内容 |
|---|---|
| `p1`–`p6` | 设置页/主题卡/模型面板/保存栏修复轮（`p1-home-empty` … `p6-night-savebar`） |
| `p7`–`p8` | essence 浅色主题全页面走查（治理卷宗/日记/历史/日志/记忆/状态） |
| `p9`、`p9d`–`p9g` | 主题引擎诊断与修复（forest/ocean/paper/day 四主题 × home/memory/settings） |
| `p10` | 主题 × 日记/记忆页矩阵 |
| `k3-` | K3 前端批：三栏主从骨架 T0、会话视图、presence 接线（`k3-k7`）、治理卷宗 |
| `rev-`、`fix1`–`fix4` | 走查轮（夜间 hour2 / essence hour22 实弹截图与修复对照） |
| `e2e-`、`inst-` | 装机 E2E 与真机验收 |
| `gov-shots/`、`presence-wiring/` | 治理 / presence 专项过程证据 |
| `guardon`、`w2*.txt`、`baseline-cargo-test.txt` | 守卫联合训练数据清单与指标、后端 cargo 测试基线（文本证据） |
| 各 `*.mjs` | 对应批次 CDP 截图/探针驱动脚本（可复跑） |

## `_attic/`

外来项目（前代产品 安装包 442MB / 补丁 / 脚本）与本项目一次性杂件（窗口调试 ps1、点击探测截图、`.tmp` json 等），共 36 件。**无任何台账引用**，可整目录清空；暂留防万一。
