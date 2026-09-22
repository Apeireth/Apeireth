# 工程日志 2026-09-22 (Engineering Log)

> **给谁看**: 协作者 / 接手人。**口径**: 2026-09-22 至 2026-09-23 发生在 `main` 上的
> K3 前端产品化批次全部工程动作, 以 commit 为索引。七项任务全部经主会话逐项
> 视觉验收 (Edge CDP 无头截图亲审, 真后端为主), 验收登记以
> `live-verification-ledger.md` #29/#30 为准。
> **基线**: 起点 `85c7f857` (2026-09-22 侦察批 v2 状态更正收口), 终点 `e7a6e9ff`。
> **施工组织**: 主会话 (团队负责人/验收人) + 三名子代理 (A=后端 presence, B=聊天壳,
> C=治理卷宗), 开工提示词与控场手册见 `frontend/companion-desktop/docs/k3-*.md`。

## 0. 批次总览

| 项 | 任务 | 主 commit | 验收 |
|---|---|---|---|
| ① | `presence_state` 后端事件 (heuristic_v0) | `fa011d07` | 台账 #29: 起 gateway 实收 60s 心跳帧, shape 逐字段核契 |
| ② | T0 聊天壳 + 三栏主从 + 个性化背景 | `5021157e`…`63557256` | 台账 #30: 两轮打回返工后复核通过 |
| ③ | 治理卷宗四 tab | `a2ecd244`/`dc821e55`/`40f78845` | 真后端实拍 (grants 3 / traces 46 / audit 100) |
| ④ | Ctrl+K 命令面板 + 打断按钮 | `d1252744`/`24c862ec` | 拼音过滤实测; 打断走 mock 慢流式 (诚实标注) |
| ⑤ | 底部状态条 | `f9fecc02`/`6f4160b8` | 安静态 + 杀后端断线级联双状态实测 |
| ⑥ | 记忆卷宗主从化 + 日记纸面调 | `104d9a2b`/`5497ae2a`/`78e01e36` | 真后端 34 条 episode; Archive 调首次实拍 |
| ⑦ | 显影接线 (presence.ts 重写) | `7026a3a7` | 真后端 60s 心跳驱动余烬 amplitude 0.65→0.2 |

**质量门 (批次终点复核)**: `cargo test --workspace` 3406 passed + `cargo clippy --all-targets -- -D warnings` 零警告; 前端 `npm run check` 0/0 + `npm run test` 15/15。

## 1. ① presence_state 后端事件 (00-PHILOSOPHY §10 契约 v0)

| 动作 | commit |
|---|---|
| 契约先行: gateway 契约文档 §8a 补 `presence_state` 事件 | `20cf3d1b` |
| `EmberCognitiveStance` 序列化定为契约值 (小写下划线串, 复用前提) | `487b7141` |
| `crates/adapters/gateway/src/presence.rs` (821 行): 状态合成模块 + 事件上现有 SSE 总线; 60s 心跳 + 无交互衰减 baseline + initiative 预算 ≤3 次/天 + `heuristic_v0` 诚实标注; 12 单测覆盖 shape/分级/频率 | `fa011d07` |
| 台账 #29 登记 (主会话 live 探针: `curl -sN -m 70 /v1/apeireth/events` 实收 2 帧, 空闲态 `dreaming_consolidation` + baseline PAD) | `20a99703` |

## 2. ② T0 聊天壳 (00-PHILOSOPHY §3, 微信式长寿骨骼)

按施工时序:

| 动作 | commit |
|---|---|
| T0 壳骨骼纯逻辑: 会话账本归并 / 审批事件归一化 / 治理通报分流 | `5021157e` |
| 消息列表是主页: 会话列表首页落地 (§3.1) | `1e1e12ec` |
| 审批卡对话内化: 待签文书在发生处浮起, 不跳窗 (§3.2/§6.4) | `e25e349a` |
| 守卫通报卡 (琥珀) 对话内呈现 + 星尘/presence 断点诚实注释 | `acb85d86` |
| svelte-check 清零 (基线 5 条警告修复) | `a614f805` |
| 待签文书按 session 归属渲染 (切换会话不串场) | `4bb59b00` |
| **打回修复①**: 会话列表页面层承托 (01-DESIGN-SYSTEM §5.1) | `95dc5e6a` |
| **打回修复②**: 会话头横排 / 左缘金晕呼吸 2.8s / 引语级行宽衬线 (22px/2.1/0.13em) | `61629a19` |
| 规范 §8 增补文档 (主人 2026-09-22 拍板个性化背景与配色) | `481a7d6a` |
| heritage-void 静态默认主题 (主人供图入库 `public/assets/themes/`; 黑洞实时场景降级为 night 主题) | `a92f8c53` |
| 三栏主从骨架: 图标轨｜320px 常驻列表栏｜聊天区, 取消页面跳转 | `31de87a2` |
| 自定义上传背景 (IndexedDB 持久化; 换原点需重传, 诚实标注) | `f9575064` |
| UI 配色方案 accent 体系 (`--ap-accent-ui` 个性化 / `--ap-accent-tab` 恒主题金; 机器断言不占存在金家族) | `597645a0` |
| persistedConfig 白名单漏 accent/customBg 修复 (保存后 reload 失忆) | `63557256` |

**0 装边界**: 星尘卡蛰伏 (总线无 memory_recall 事件); prompt-overlay 召回命中不上总线。

## 3. ③ 治理卷宗 (事后卷宗, Deep-Ops 调首次实拍)

| 动作 | commit |
|---|---|
| 数据层与纯逻辑: trace 列表 fetcher + 审批账本 / 守卫映射 / 轨迹树 (契约 §6/§7/§8) | `a2ecd244` |
| 视图四 tab (审批/授权/守卫/审计) + 左轨「治理」入口 | `dc821e55` |
| 能力清单时序两修: 异步到达触发首载 + 未到达显加载而非不支持 | `40f78845` |

## 4. ④⑤ 命令面板 / 打断 / 状态条

| 动作 | commit |
|---|---|
| Ctrl+K 注册表纯逻辑: 筛选 / 别名 / 拼音 / 最近优先 + Node 单测 | `d1252744` |
| 面板上板 (19 条命令) + 打断按钮诚实化 (title 写明「打断的是收听, 不是他——后端回合仍会跑完」) | `24c862ec` |
| 状态条纯逻辑: SSE/回合/守卫/记忆四指标三态推导 + Node 单测 | `f9fecc02` |
| StatusBar 上壳 + **修复 presence 订阅 mount 门恒假真 bug** | `6f4160b8` |

## 5. ⑥ 记忆卷宗 + 日记 (Archive 纸面调首次实拍)

| 动作 | commit |
|---|---|
| 纯逻辑: 过滤 / 计数 / 缺省标注 / rev 账本 / 409 分类 / 图谱链接 + Node 单测 | `104d9a2b` |
| 主从化重写 + Archive 调落地: 左列表右详情 / forget 红边内联确认 / 409 黄边冲突卡 + `appendMemoryEpisode` 0 装修正 | `5497ae2a` |
| 日记纸面空态契约页 (后端无日记端点, 空态即契约) + rail/drawer/命令面板接线 | `78e01e36` |

**实拍校准产出**: `archive.inkGold = #8a6d1f` (纸白上 #ffd27a 对比度不足, 文字级强调下沉金; 🔵 提案待主人拍板)。

## 6. ⑦ 显影接线 (presence.ts 重写)

| 动作 | commit |
|---|---|
| 前端 presence 消费链改订 `presence_state` 契约: 具名 `addEventListener` 订阅 (`onmessage` 收不到具名帧) / 显影分级 (heartbeat 只动余烬, turn 动光环, ritual 无生产者留契约空间) / T2 黑洞 PAD 映射 (§4.1, 增益 0.75 保守值) | `7026a3a7` |
| 台账 #30 登记 (批次验收) | `e7a6e9ff` |

## 7. 验收与打回记录 (主人约束: 前端效果必须主会话亲自视觉验证)

- **验收设施**: Edge `--headless=new` + CDP 截图脚本 (`artifacts/k3-*.mjs`, scratch 未入库), 真后端 (`apeireth.exe gateway serve --port 8080`) 为主; 打断场景用 `artifacts/gov-shots/mock-gateway.mjs` 慢流式 mock (诚实标注)。
- **打回两轮** (② 聊天壳): ① 会话列表无页面层承托, 文字浮在背景上不可读; ② 会话头竖排 / 缺金晕呼吸 / 行宽不符引语级规范。返工后复核通过。
- **真后端亮点证据**: 治理卷宗 grants 3 条真实授权 / audit 46 traces + 100 事件; 状态条安静态「SSE 已连接 · 回合空闲 · 守卫 0 · 记忆 34」; 杀后端后「重连中」黄 + 守卫/记忆「不可用」级联; 记忆卷宗 34 条真 episode; 60s 心跳帧驱动余烬 `--ember-amp` 0.65→0.2 (真实 breath.amplitude, 周期 4s)。

## 8. 遗留与挂账 (0 装, 不声称已验)

1. 台账 §2 挂账 #2/#4 未消: 审批卡对话内**真后端**全闭环 (需真 LLM key 触发工具审批) + UI 点击流人工实测 —— 留主人真机。
2. `empathetic_care` / `ritual` 无生产者 (契约空间保留); initiative 只有预算器。
3. `subscribeCompanionEvents` (legacy 伴随体订阅) 同患 mount 门恒假 —— 点亮会唤醒 [他说] 主动开口整条链, **需主人拍板**。
4. 「批准当前待签文书」命令不做跨会话批发 (原则 4)。
5. 行宽 31ch → 实现取 35em (ch 对 CJK 只有约 15 字/行, 偏离意图); 严格 31 字需会话列约 830px, 待主人看阅读感 (01-DESIGN-SYSTEM §6.1 实现注)。
6. 00-PHILOSOPHY §10 原「kebab-case 序列化」与契约 JSON 实例矛盾, 已按实现更正为小写下划线串 (2026-09-23 文档批)。
7. `inspiration-2026-09/SYNTHESIS` 全仓缺失 (开工时已报主人, 不阻塞)。

## 9. 下一梯队 (未开工, 等主人指令)

桌宠 (常驻投影, 00-PHILOSOPHY §5 🟡 提案待校准) → 会话设置抽屉 (P2, `session_settings.rs` 后端已就绪) → 审批卡真闭环联调 (等主人给 key) → 会话分支 (需后端数据模型先行, gap-plan §5) → 看板/终端 (独立立项)。
