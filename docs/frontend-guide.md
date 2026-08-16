# 前端接入指南 — 用现成 Chat 前端连 Apeireth（2026-08-16）

> 主人不会写前端 + 不急着自研 UI → **路线 A：接现成开源 Chat 前端**，我们的
> `apeireth-api` 提供 **OpenAI 兼容端点**（`POST /v1/chat/completions`），主流前端原生支持。
> 产品形态：**先验证伙伴感**（对话可用 → 记忆可视 → 主动送达），界面从简。

## 一、后端：起 Apeireth API（OpenAI 兼容）

```powershell
# 真 LLM 后端（MiniMax-M3）—— 注意: chat 端点总是走真 pipeline, 必须真 key
$env:APEIRETH_API_KEY = (Get-Content apikey-ultra.txt -Raw).Trim()
cargo run -p apeireth-api --example serve          # 默认 :8080
```

> **实测发现（2026-08-16）**：`APEIRETH_LLM_BACKEND=scripted` 只作用于 `/council/advise`
> （LlmProvider 路径）；`/v1/chat/completions` 永远走 5 步 Pipeline 真接 MiniMax ——
> 无 key 时返回 MiniMax 401（链路通但未授权）。前端对接必须配真 key。

验证端点活着（2026-08-16 真机已验证 ✅）：

```powershell
curl http://127.0.0.1:8080/health          # → {"status":"ok","protocols":[...4 协议...]}
curl http://127.0.0.1:8080/v1/models       # → {"object":"list","data":[{"id":"MiniMax-M3",...}]} (2026-08-16 新增)
curl -X POST http://127.0.0.1:8080/v1/chat/completions `
  -H "Authorization: Bearer $env:APEIRETH_API_KEY" `
  -H "Content-Type: application/json" `
  -d '{"model":"MiniMax-M3","messages":[{"role":"user","content":"用一句话介绍你自己"}]}'
# → MiniMax-M3 真回话 ✅ (max_tokens 给小了会被 <think> 吃掉, 建议 ≥200)
```

端点清单：`/health` · `/health/deps` · `/v1/models`（OpenAI 兼容模型列表）
· `/v1/chat/completions`（OpenAI Chat）· `/v1/responses`（OpenAI Responses）
· `/v1/messages`（Anthropic）· `/v1beta/models/{model}:generateContent`（Gemini, schema bug 已知不修）
· V2 端点（/tools/list /memory/episodes /organs /asi /sovereignty 等）。

## 二、前端选型（2026-08-16 调研）

| 前端 | 形态 | 优点 | 缺点 | 适配度 |
|---|---|---|---|---|
| **LobeChat** | 桌面版 (Windows) / Docker / Web | UI 现代, 中文生态, 自定义 OpenAI 兼容 provider, 知识库 | 桌面版需下载安装包 | ⭐⭐⭐ 首选（主人零部署） |
| **NextChat** (ChatGPT-Next-Web) | Docker / Vercel / 静态页 | 最轻量, 配置 1 分钟 | UI 朴素 | ⭐⭐⭐ 次选（最简） |
| **Open WebUI** | Docker（较重） | 功能最全 (RAG/多用户/工具) | 个人场景过重, 部署门槛高 | ⭐⭐ 以后再说 |
| 自研面板 (Vite+React) | — | 完全定制 (每日摘要/记忆侧栏/涌现日志) | 要写前端 | 路线 C, 产品验证后 |

## 三、对接步骤（以 LobeChat 为例）

1. 起 Apeireth serve（§一, 默认 :8080）
2. LobeChat（桌面版或 Docker）→ 设置 → 语言模型 → 添加自定义服务商：
   - **API 地址**：`http://127.0.0.1:8080/v1`
   - **API Key**：任意非空串（本地 serve 校验存在即可；真 key 由 serve 持有）
   - **模型名**：`MiniMax-M3`
3. 开始对话。

> 注意：OpenAI 兼容前端会拉 `GET /v1/models` 列模型 —— 若前端要求该端点而 serve 未提供，
> 在 Apeireth 侧补一个静态 models 列表端点即可（见 §五 待办）。

## 四、"伙伴感"验证清单（产品形态：先验证伙伴感）

按最小前提（「打开就感觉他在、他记得我」）逐项验证（2026-08-16 真机实测）：

- [x] **P0-1 对话可用**：前端 ↔ serve ↔ MiniMax 真回话（§三）
- [x] **P0-2 记忆感**：companion_serve 记忆注入 → 问"我高数换元错在哪" → 准确答出
      两个错点 + 主动提到周五考试/作息 + 提议帮助 ✅（实测原文见 §六）
- [x] **P0-4 工具透明**：AI 主动调 simulate ×3 完成沙盘推演并基于结果回答 ✅
- [ ] **P0-3 今日感**：今日摘要注入已在链路（build_daily_summary），需真实积累一天数据后验证
- [ ] **P1-1 主动性**：Lark/飞书实发早安 + 每日摘要（通道 B, 需凭据）
- [ ] **P1-2 记忆可视**：前端侧栏展示"他记得你什么"（路线 C 定制面板）
- [ ] **P1-3 工具透明 UI**：前端展示 tool_calls（前端原生支持, 无需后端改）

## 五、后端：companion_serve（伙伴端点, 已完成 2026-08-16）

**v2 全能力版**：连上后端 = 前端天然拥有后端全部能力（主人设想落地）。

```powershell
$env:APEIRETH_API_KEY = (Get-Content apikey-ultra.txt -Raw).Trim()
$env:APEIRETH_SEED_MEMORY = "可选: 种子记忆;分号分隔"   # 演示用, 不设则从零积累
$env:APEIRETH_GRANT = "FileOperator:24"                 # 可选: 显式扩权 (工具:小时;分号多个)
cargo run -p apeireth-companion --example companion_serve   # :8090, daemon 同进程常驻
```

- 端点：`/health` · `/v1/models` · `/v1/chat/completions`（OpenAI 兼容）
- **① 持久记忆**：`open_memory_store()` 文件库（%APPDATA%\apeireth\memory.sqlite, 重启不失忆）
- **② 全工具可见**：schema 由 registry 动态生成（含未手写 schema 的通用兜底）;
  执行由宪法硬门/权限包/主人批准约束（FileOperator 等默认需批准, APEIRETH_GRANT 显式扩权）
- **③ daemon 常驻 v3（全 LLM 接上）**：
  - 做梦: LLM 摘要器（MiniMaxDreamSummarizer, 合并记忆提炼 ≤50 字, 失败拼接降级）
    + **增量合并**（只合并上次做梦后的新记忆, 防旧记忆反复合并/摘要嵌套, 实测发现并修复）
  - 涌现: TonalUtterance LLM 润色（机制事实→自然问候, 30s 节流+退避兜底原文）
  - 宪法: LlmJudicator 真 LLM 评审（Medium+ 工具执行前按 E 层判案）
  - 对话自动喂节律（on_user_message）; 做梦安静期 `APEIRETH_DREAM_QUIET_SECONDS` 可调
- **预处理链**（对齐 VCP messagePreprocessors）：记忆注入（EMI/NEC）+ 今日摘要注入 + 工具桥
- 会话标签：`X-Apeireth-Continuity`（缺省 companion-main; 记忆本体统一 "me" 会话）
- 护栏：工具循环 ≤5 轮 + 结果 4000 字符截断 + 注入标注"以用户当前说法为准"

**0 假装**：FileOperator/ShellExec 可见但默认需批准; daemon 内部 RefCell 跨 await 非 Send → 与 HTTP 同 task 交替（select!）;
记忆很多时增量窗口（最近 20 条）可能漏旧条目（后续可做游标式）。

**实测（2026-08-16 v3, MiniMax-M3）**:
- save_memory 真执行 + **重启后仍记得**（持久化 ✅）
- **做梦全链路**: daemon tick → 安静期到 → 合并新记忆 → LLM 摘要(1.6s) → 写回【做梦摘要】✅
- 宪法评审: FileOperator 写文件成功（LLM 评审 ALLOW + 授权包）✅

**对 VCP 的改进点**（VCP 不好的我们改，VCP 没有的我们补）：

| VCP 原设计 | 问题 | companion_serve |
|---|---|---|
| chatCompletionHandler 59KB monolith | 不可维护 | 模块化: 注入/摘要/工具各自独立, 机制件在 companion |
| RAG 注入无反幻觉约束 | AI 可能把注入当事实 | EMI/NEC 编号列表 + 冲突以用户为准标注 |
| toolApprovalManager 简单审批 | 无宪法/审计 | ToolBridge 全链: 宪法硬门+权限包+留痕 |
| alias 映射复杂 | 重 | X-Apeireth-Continuity 一行 header |
| 无防失控 | 工具循环无限 | 5 轮上限 + 结果截断 |
| 无"今日感" | — | 今日摘要注入（VCP 没有, 我们补） |

## 六、实测记录（2026-08-16, MiniMax-M3 真机）

**记忆注入问答**（无工具, 纯注入）:
> 用户: 你还记得我高数换元法的问题吗?具体错在哪?
> AI: 记得的~ 根据记忆里的记录, 你换元法常犯两个错: ①认知漏洞: 根号里带平方的处理
> ②操作漏洞: 换元后忘记换 dx... 对了, 我记得你周五要考高数期中... 要不要我现在帮你
> 针对性出几道"根号带平方"的换元题?

**工具桥**（AI 主动调 simulate ×3 后回答）:
> 用户: 比较策略A(刷30题) vs 策略B(精练8题+套路卡), 推演3步
> AI: [simulate ×3] → 表格对比 (复习进度 A+24/B+9, 熟练度 A+10/B+23, 信心 A-2/B+19)
> → 判断选 B + 落地建议 (套 7-10 点高数时间: 8题精练/保温刷/口头复述)
> x_apeireth: {"tool_rounds":4, "tools_executed":["[simulate] 已执行"×3]}
