# DeepSeek Harness 前端参考研究（2026-09，供 companion-desktop 借鉴）

> 素材：本机 dsh-web-app@0.1.0-rc.8 实装（pnpm 库内全部包）+ GitHub deepseek-ai/deepseek-harness
> 文档（README / architecture.md / docs/user/guide/index.md + providers.md / packages/web README）。
> 三个子代理并行解剖 UI 交互层 / 会话编排层 / 安全治理工具层，本文件汇总其报告并映射到
> Apeireth 桌面前端的采纳建议。

## 1. DSH 总览（事实）

- **一切皆插件**（Cordis 组合框架）：模型适配器、工具注册表、会话日志、agent loop 本身都是
  可替换插件；profile = 有序 bundle 层的命名组合；web profile 支持**热补丁重载**。
- **Web UI** 默认 `http://127.0.0.1:3080`；**桌面版（Electron）不监听任何回环端口**——用
  `dsh-app://` 安全协议 + 版本匹配的 framed byte pipes 通信（对比：我们侧车监听回环 HTTP）。
- **配置即生效**：模型/设置改动**下一次请求生效，不重启服务**（web 与桌面皆然）。
- **会话日志即真相**：`session/event` 是 append-only 事实日志；"**模型可见即已入日志**"是不变量；
  UI 从日志渲染，流式增量走 `agent/assistant-stream` 实时事件。UI 扩展 = 注册
  `ConversationNodeDefinition` + keyed renderer（聊天面是节点化的）。
- 回合模型：**step** = 一次模型请求 + 其工具调用；**turn** = 0..n step。会话事件耐久、agent 事件
  实时，两者分域。

## 2. 已确认的借鉴点（本代理直接取证）

1. **设置免重启**：DSH 明确"Model changes take effect on the next request without restarting
   the server"。我们现状：Settings 保存 → 重启侧车（端口变更 → 前端重解析）。建议：P0 级改造
   方向——provider 配置走运行时可重载的配置读取（env 为启动参数是当前约束，需要加配置层）。
2. **密钥写保护 + 持久引用**：DSH key 只写不读（保存后回显脱敏描述符），存
   `$DSH_HOME/.credentials.yaml`，settings 只存引用——**重启后 key 仍在**。我们现状：key 不落盘
   → 每次启动重填（今晚已实锤为摩擦点）。建议：复用既有 keyring 机制
   （`APEIRETH_KEYRING_BACKEND`）把 provider key 存 OS 钥匙串，supervisor 启动时解析。
3. **模型发现选择器**：DSH 自定义 provider 表单有"Fetch available models"→ 可搜索多选 picker。
   我们已有 `/v1/models`，但设置页模型列表是硬编码预设。建议：设置页接 gateway `/v1/models`
   做可搜索多选。
4. **表单刻意做小**：DSH Models 页只有"路由存在所需"的字段；高级字段（reasoning 档位、图片输入、
   兼容开关）放 `settings.yaml`，页面提供"Open configuration file"直达。我们 SettingsView 的
   高级能力/密钥弹窗分裂问题可参考此分层哲学。
5. **兼容开关（compat）**：`supportsDeveloperRole` / `maxTokensField` / `thinkingFormat`
   等针对"网关兼容 OpenAI 却拒收某些字段"的一键开关 + 故障排查表（错误码化：
   MISSING_CREDENTIAL / UNKNOWN_MODEL / 401 等）。我们今晚踩的 reasoning 空响应/错误帧被吞，
   正是缺这类"请求形状兼容"与"错误码→解法"的体系。
6. **Workspace 前置选择**：会话输入框在选定工作区前不可用（"session composer remains
   unavailable until a workspace is selected"）。对应我们：shell/filesystem 工具的 cwd 语义
   需要一个用户可见的"工作区"概念。
7. **会话保留所选模型**：切换模型只影响新会话；已发过请求的会话记自己日志里的模型。我们的
   config.model 是全局单值——建议按会话记 model。
8. **Web 工具家族**：web_search + web_fetch 可互换后端（Exa / Perplexity / DeepSeek 原生），
   模型侧工具行为一致。我们只有 GET-only fetch——搜索能力是明确的下一代补位。

## 3. UI/交互层调研（子代理报告摘要）

**技术底座**：React 18 + Vite，Cordis 插件按 **slot** 组装（`conversation.chat.node`、
`tool.call.toolview`、`conversation.input.dock` 等）；三栏 AppFrame（sidebar/conversation/
details 可拖拽）；`dsh-client-locale` 双语；富文本 = MarkdownText 原语（GFM + KaTeX + Shiki
语法高亮）。

**特性清单**：①三栏框架（侧栏可收 56px、details 零宽自动关）②侧栏会话浏览（标题即时搜索 +
内容 250ms 防抖后端搜索、拖拽排序、Fork 标题自增）③聊天流（step 摘要流、流式 tail 隔离、
compaction 折叠行、Think 行、输入坞栈 Todo/Goal/Queue）④工具卡片（递归 ToolCallTree、按工具
名分派原子视图、running/success/failed/interrupted 生命周期）⑤审批/提问卡（完全替换输入框；
单选/多选/推荐徽章/跳过；plan-review 审批卡 Chat about it/Refuse/Approve）⑥后台任务 header
图标 + live/settled 排序 ⑦轨迹台账（turn 感知事件流、区间框选聚焦、token/耗时 inspector、
虚拟滚动）⑧产物 chips（≤6 + "+N files"、"Show in folder"、inline-code 文件提及可点）⑨附件
64px 缩略图 rail + 拖拽 + lightbox ⑩输入触发（`/` 命令模糊菜单、`@file`/`@session` 原子引用、
`+` Command launcher）⑪slot 化设置页（Host 持久化 settings.yaml + schema 校验）⑫消息反馈
Like/Dislike + note ⑬主题 light/dark/system 启动前注入防闪白 ⑭HMR：SSE rebuilt 帧单插件
串行热重载。

**交互细节**：流式 = 已结算内容与 streaming tail 分离；审批 = amber strip + 理由标题 + 命令
+ 一次性 allow/refuse；AUTH 错误不回显凭据片段；连续 retry 折叠为单条静音行；pendingInteraction
分类（Waiting for approval / Plan awaiting review / Waiting for answer）用 amber 点优先于
运行指示；Enter=Queue（可配 Steer）、Cmd+Enter 反向、Shift+Enter 换行。

**亮点**：slot 可组合（删配置条目即关功能）；单一权威不乐观（goal/plan/todos 全读 Host 投影，
客户端不本地猜测，迟到 frame 能纠正）；复制语义诚实（仅在剪贴板真写入后报"已复制"）；无障碍
（aria-activedescendant、prefers-reduced-motion）；文件提及仅"精确路径或唯一 basename"可点；
启动前同步注入主题防闪白。

## 4. 会话/编排层调研（子代理报告摘要）

- **agent-loop 是唯一驱动者**：turn 内多 step（每 step = 一次模型调用 + 一组工具），并行安全
  调用滚动池上限 10；send 原语 followup/steer/inject；插件失败只结束当前 turn 不杀 loop。
- **session 事件溯源为唯一权威**：模型历史从 surface 层派生；压缩/替换只遮蔽 surface 不删
  原始日志；崩溃恢复合成 TOOL_NOT_STARTED/TOOL_OUTCOME_UNKNOWN。
- **goal**：同会话持久目标（单个当前目标、phase active/paused/blocked/completed、round 预算
  默认 256）；空闲时自动注入 `<goal_round>` 续跑；blocked 需连续 3 轮才可上报；恢复会话需显式
  resume。**用户可见：目标后台自动推进。**
- **plan-mode**：`/plan` 软约束规划态；模型提计划 → 「计划审查」审批卡请用户批准。
- **subagent**：fork 继承父前缀 / spawn 全新；前台/后台/continuable；后台结束发
  "Background subagent finished" 通知；list_agents 显示 running/idle/ready；send_message/
  interrupt_agent 控制。
- **workflow**：模型写 JS 编排脚本 fan-out 子代理（前台阻塞）；ralph = 每轮全新 agent、
  共享工作区为记忆的固定脚本。
- **skill**：会话开头注入 `<available_skills>` 目录；模型调 skill 工具加载内容；用户可
  `/技能名` 直接内联触发。
- **compaction**：token 压力（默认阈值 0.8）或 `/compact` 手动 → 旧区间摘要为一条
  `<compacted-summary>` user 消息。
- **spill**：超大纯文本工具结果落盘，模型只见预览 + `(Omitted N bytes… stored at: 路径)`。
- **todo/jobs/schedule/feedback/context**：todo_write 全量替换任务清单（UI 计划条）；jobs 后台
  任务注册表（job id/read/kill/wait）；schedule 定时提醒以 followup 注入；/feedback 只写日志
  不进模型；time-context 每步注入时区；session-reference 提供跨会话 @提及快照。

## 5. 安全/治理/工具层调研（子代理报告摘要）

- **权限模型两参数**：`sandbox/mode`（read-only / workspace-write / danger-full-access，仅文件
  操作）+ `approval/policy`（ask / never）；`permission-presets` 打包成单一选择器（默认
  workspace-write+ask 与 danger-full-access+never）。**会话事件日志是唯一状态**；预设与策略在
  会话创建时冻结，之后改动不影响已存在会话。**批准严格一次性**：allowed-once / rejected /
  cancelled / unavailable，无 allow-always、无记住规则、无撤销。默认 fail-safe（read-only +
  ask；无应答者 → unavailable → 拒绝关闭）。
- **审批交互**：审批是通道无关 seam；Web 端 = **composer 接管**卡片——「等待审批」+ 理由标题
  （缺省"工具 X 请求越权执行"）+ **提取出的 shell 命令原文** + 【拒绝 / 允许一次】。无超时；
  断线不丢 pending（重连同 rpcId 回放）；**无批量批准**（并行调用按 callId 逐条作答）。
  ask_user_question 同为 composer 接管：多问题分页、radio/checkbox + 自定义文本 + Skip/Submit/
  Next；plan-review 渲染成【确认执行 / 拒绝】专用卡。
- **沙箱与执行**：只限制文件写入，与宿主共享 FS/内核（bwrap / Landlock / Seatbelt / Windows
  受限令牌）；无后端 → SANDBOX_UNAVAILABLE fail-closed。**拒绝是"结果事实"**：模型看到
  `[sandbox: file access denied under <mode> mode]` + `[sandbox: escalation available — retry
  with sandbox_permissions + justification]`；升权 = 同轮次以最窄更宽模式 + justification 重试
  一次，执行前过审批。fs：write/edit 前必须先 read（观察策略），版本冲突拒绝，read 有行数/行长/
  字节上限。web：web_search + web_fetch（无审批，参数为部署配置）。
- **凭据 UX**：配置只存**引用**（`apiKeyEnv`）；值在进程环境 + `.credentials.yaml` + .env；
  每操作 resolve() 不缓存（改密下次即生效）；describe() 只报 configured/source/writable、
  **绝不返回值**；空值 = 不存在。
- **亮点**：审批审计事件只写日志，模型只见工具结果、审批 UI 不进上下文；策略变化以
  "运行时上下文快照" markdown 追加（KV-cache 稳定）；invariants 提供运行时断言（审批
  asked/decided 配对、沙箱词汇、preset 引用）；匿名用户 id 随机生成不派生 hostname；MCP 工具
  按 server 命名空间隔离、世代整体替换、图片按模型路由门禁。

## 6. 采纳优先级（定稿）

| 级 | 借鉴项 | DSH 做法（事实） | 我们现状 | 建议动作 |
|---|---|---|---|---|
| P0 | key 持久化 | 写保护密钥存 `.credentials.yaml`，settings 只存引用，重启仍在 | key 每次启动重填（已实锤为摩擦） | 复用 `APEIRETH_KEYRING_BACKEND` 存 OS 钥匙串，supervisor 启动解析 |
| P0 | 模型选择接真值 | Models 页 catalog + "Fetch available models" 可搜索多选 picker | 设置页硬编码预设；gateway `/v1/models` 已存在 | 设置页接 `/v1/models` 做可搜索选择；预设降级为快捷模板 |
| P0 | 会话级模型记忆 | 已发请求的会话记自己日志里的模型；默认 provider 被删 → composer 阻塞提示 "Select model" | config.model 全局单值 | 会话表加 model 字段；composer 无可用模型时阻塞 |
| P0 | 审批卡信息量 | composer 接管卡：理由标题 + **命令原文** + 一次性【拒绝/允许一次】；plan-review 专用卡 | 弹窗已修卡死/自愈，但没显示要执行的命令 | 弹窗内展示 frozen invocation 的命令/URL（后端已随审批返回） |
| P0 | 错误码→解法 | MISSING_CREDENTIAL / UNKNOWN_MODEL / 401 等错误码 + 排查表 | error frames 已上抛，提示仍是原始文案 | 建立错误码→中文解法映射（前端 + 文档） |
| P1 | 设置免重启 | 改动下一次请求生效，不重启服务 | 每次保存重启侧车（端口变更+重解析） | 运行时配置重载层（大改：env 注入模型 → 配置读取模型） |
| P1 | 会话级权限预设冻结 | 预设（沙箱模式+审批策略）在会话创建时冻结，改动不影响已存在会话 | 旋钮全局生效 | 会话创建时快照能力旋钮（低冲突、语义清晰） |
| P1 | 拒绝结果文案 | `denied under <mode>` + `escalation available — retry with sandbox_permissions + justification` | 拒绝 = 纯错误文案 | 工具拒绝结果附带"升权路径"提示（设置旋钮） |
| P1 | 工作区概念 | 未选 workspace 前 composer 不可用；目录选择器（browse/native） | shell/filesystem 的 cwd 语义无用户可见工作区 | 设置加"工作区"选择器，注入 `TrustedShellConfig` cwd |
| P1 | 工具卡生命周期 | 递归 ToolCallTree + 按工具原子视图 + running/success/failed/interrupted | 工具调用展示较弱（仅 tool-call/tool-result 事件） | 工具卡片组件化 + 生命周期态（事件流已齐） |
| P1 | 输入触发 | `/` 命令模糊菜单、`@file`/`@session` 引用、`+` 命令启动器 | 无 | `/` 菜单（人设/开能力/compact 类）+ @文件提及（配合 filesystem 工具） |
| P2 | 沙箱执行层 | Windows 受限令牌 / Landlock / Seatbelt；read-before-write 观察策略 | shell 以用户账户权限运行（文档已声明） | 远期（安全路线，与 P5 沙箱强化合并） |
| P2 | 轨迹台账 | turn 感知事件流 + 区间框选 + token/耗时 inspector + 虚拟滚动 | trace 面板已有基础（span 树） | 增强为时间轴 + 虚拟滚动 |
| P2 | 消息反馈 | Like/Dislike + note 弹窗 | 无（有 preference_learning 后端） | 反馈事件接入偏好学习写回 |
| P2 | 附件/图片 | 64px 缩略图 rail、拖拽、lightbox；vision 模型声明制 | 无附件；provider Vision 未声明 | 远期（先 provider 声明 Vision） |
| P2 | 子代理/后台任务/工作流 UI | subagent 卡片 + 后台完成通知 + jobs 徽章 + workflow 进度 | 后端有 OrganOrchestrator 等，无对应 UI | 远期（配合后端能力路线） |
| P2 | 桌面端不走回环端口 | Electron `dsh-app://` + framed pipes，零端口暴露 | 侧车回环 HTTP + CORS（已闭合） | 架构级改造，暂不做 |
