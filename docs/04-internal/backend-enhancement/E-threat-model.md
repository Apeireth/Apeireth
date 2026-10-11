# E 轴 P1 · 威胁模型（W1-①）

> `Status: ✅ 已交付（E 分期 P1，纯文档批）`
> 一事：为 E 轴安全纵深立**资产—边界—敌人—防点—缺口—对抗测试**六表，供 P2-P4 派工与对抗测试选题。
> 纪律：[CONSTRUCTION-DISCIPLINE.md](CONSTRUCTION-DISCIPLINE.md)（进场第一课）；上游：[E-security-depth.md](E-security-depth.md)（P1=本文）；波次口径：[00-backlog.md](00-backlog.md)。
> 诚实条款：已设防**必带实现指针**并附边界；缺口只记现实、不美化；不存在的防点不写。

## 1. 资产清单

| 资产 | 现实落点 | 敏感级 | 泄露/失守后果 |
|---|---|---|---|
| 记忆库 | `crates/engine/memory`（SQLite 记忆栈 + 六流历史 + 日记档） | 高（个人隐私） | 「它只记得你」破防：往事/习惯/关系被画像、被社工 |
| 凭据 | `crates/foundation/credentials`（系统钥匙串 / 加密文件后端） | 极高 | 冒名调用 provider、烧钱、密钥扩散至全链 |
| 会话日志 | `crates/engine/runtime` 回合/审批记录 + `%LOCALAPPDATA%\Apeireth\data\sessions.sqlite3` | 高 | 全部对话原文外泄（含隐私与提示注入面） |
| 工具执行权 | `crates/capabilities/tools`（shell/受控写/联网/MCP）+ 能力开关 | 极高（=本机权力） | 任意命令、任意写、数据外带——等价本机沦陷 |
| 配置 | `crates/adapters/gateway/src/im_channels.rs` 渠道配置（含共享秘密）、桌面能力开关、`onboarding.json`（零机密） | 中 | 渠道秘密泄露=审批回执可伪造；开关被改=防线静默拆除 |
| 审计流 | `crates/foundation/governance/src/approval_closure.rs` 配对审计 + `credentials` 审计（name_hash）+ `crates/engine/memory/src/agent_trace.rs` | 中高（问责链） | 篡改=追责断链可抵赖；泄露=行动轨迹全曝光 |

## 2. 信任边界图（文字图）

```text
本机用户（主人 / 物理接触者）
  ├─① 人机面：输入 / 查看 / 审批点击
  │    设防：密钥只进系统钥匙串、引导档案零机密、审批卡「批准前见墙」沙箱徽标
  │    缺口：无应用锁、无本地身份——摸到机器≈当主人（敌人⑥）
  ▼
桌面壳（frontend/companion-desktop）
  ├─② 本机回环 HTTP/SSE + 能力开关 env 注入 → 侧车
  │    设防：开关同源注入、启动日志脱敏、会话级工作区、审计 masked 不可还原、
  │         回环令牌自铸注入（G10 处置：每次启动一枚 32 字节随机令牌进侧车 env）
  │    缺口：来源白名单只挡浏览器跨源；**令牌门已扩到全 API 面（W2-②，仅 /health
  │         探活豁免）**，桌面 WebView 全调用面带令牌（残余 (d) 已收口：全 fetch
  │         统一鉴权头 + SSE 带令牌走 fetch 流式读取）
  ▼
侧车网关 + 运行时（adapters/gateway + engine/runtime + 治理管线）
  ├─③ → provider API（出：提示+召回记忆 / 入：回复）
  │      设防：出口允许清单（tools/src/egress.rs）、https 强制（环回例外）、缺凭据 fail-closed 不出网、出站凭据绊线脱敏
  │      缺口：记忆原文随提示托付第三方——隐私只靠契约不靠技术
  ├─④ ↔ IM 渠道（入：跨源消息 / 出：审批卡）
  │      设防：入站不可信封套（orchestration/src/untrusted_envelope.rs）、审批四态闭合+审计配对、共享秘密脱敏
  │      缺口：跨源内容=提示注入面（G8）；回执身份仅共享秘密级
  ├─⑤ ↔ MCP 外部工具（出：调用 / 入：结果与工具描述）
  │      设防：审批级接入（tools/src/mcp_bridge/）、输出归一+脱敏、坏配置拒开、启动日志脱敏
  │      缺口：外部服务器自带权限=供应链面（G6）；判定非对象级（G3）
  └─⑥ ↔ TUI / 远程客户端（W2 起：会话与工具权远程化）
         设防（W2-②）：认证面全网关化（全 API 面令牌门, 审计带客户端名）+ 每客户端
              令牌表（name→token, env/json 配置面, StoredDoc 拒开）+ 非回环绑定
              强制令牌（fail-closed 拒启动）+ `APEIRETH_LISTEN` 绑定旋钮（缺省仅本机）
         缺口：TLS 面仍是接口桩（未接线, 明文）；每客户端**权限**分级未做（W3, G1）；
              未设口径/逃生阀下仍无认证（残余 (a)(b)，审计可见）
沙箱执行面包住工具层（tools/process 应用容器 + 作业对象 + 出口管制）：
  设防：工作区 ACL 授予、零 capability=断网、能力不达 fail-closed 不裸跑
  缺口：词面/命令级判定≠对象级（G3）；仓外拦截层不对称（见 §4 沙箱行）
```

## 3. 敌人画像

| 敌人 | 主攻边 | 典型路径 | 成事条件 |
|---|---|---|---|
| ① 恶意提示注入（跨源内容带毒） | ③④⑤入站 | 消息/网页/工具输出夹带指令，诱模型自证、越权申请、外带记忆 | 模型服从内容指令且执行面无审批/沙箱兜底 |
| ② 工具滥用（越权写/数据外带） | ②→工具层 | 绕写总闸落盘、shell 输出夹带数据、等价 API 换形态重放 | 判定停在词面/命令级——换 API 即换权力 |
| ③ 本地恶意进程 | ①②本机面 | 读数据目录/密钥材料、冒充桌面壳调回环端点、拷钥匙串导出物 | 数据明文在盘、端点身份可伪造 |
| ④ 供应链（依赖/外部工具服务器） | ⑤依赖链 | 依赖被投毒、外部工具服务器或工具描述带私货 | 第三方代码在授权面内运行、行为审计拦不住 |
| ⑤ 多户越权（W3 起） | ⑥多客户端 | 访客会话读主人记忆库、复用他人审批、改全局开关 | 单机单主假设被多端/多户打破 |
| ⑥ 物理接触 | ①机器本身 | 锁屏外翻会话、直接拷数据目录、现场改配置关防线 | 无应用锁 + 明文数据 + 配置可写 |

## 4. 已设防清单（对齐现实，每条带实现指针）

| 防点 | 实现指针 | 边界（如实） |
|---|---|---|
| 全 API 面令牌门（G10 处置 W1-③ + G4 收口 W2-②） | 三态门口径 `crates/adapters/gateway/src/canonical_entry.rs`（`GatewayTokenGate`：令牌族在场=Bearer 恒定时间校验**全端点** / 两面皆空=放行+警告帧+审计 / `none`=逃生阀明示不设防留审计）+ 客户端令牌表 `crates/adapters/gateway/src/client_tokens.rs`（name→token, env `APEIRETH_GATEWAY_TOKENS` + `client-tokens.json` StoredDoc 拒开, 过门审计 `client=<名>`）+ 远程绑定闸 `crates/adapters/gateway/src/remote_access.rs`（`APEIRETH_LISTEN` 缺省仅本机; 非回环无令牌**拒启动** fail-closed）+ 桌面自铸注入 `frontend/companion-desktop/src-tauri/src/backend_supervisor.rs`（每次启动一枚 32 字节随机令牌, 进程 env 面, 不落盘）+ 客户端注入面（TUI 全请求带 Bearer、桌面 UI `get_gateway_token` IPC、验证脚本全端点透传） | 唯一豁免 `/health`（探活刚需, `UNAUTHENTICATED_PROBE_PATHS` 测试钉死）；未设口径=兼容路径放行（警告+审计可见）；逃生阀 `APEIRETH_GATEWAY_TOKEN=none` = 明示不设防（审计留痕、门体不设防, 与令牌表互斥拒开）；TLS 面接口桩**未接线**（`APEIRETH_TLS_CERT/KEY` 只触发显式警告, 流量明文）；桌面 WebView 全调用面带令牌（残余 (d) 已收口：`gatewayAuthHeaders()` 全 fetch 统一鉴权头 + SSE 带令牌走 fetch 流式读取, 前端测试 `gateway-auth-face.mjs` / `sse-fetch-stream.mjs` 钉死） |
| 三洋葱 | 双洋葱 `crates/foundation/core/src/onion.rs` + 判定层 `onion_gate.rs`（L0 人类权威恒需在场）+ 执行面 `crates/engine/runtime-assembly/src/canonical/onion_layer.rs`（末层纵深） | 执行面 hook 旋钮 `APEIRETH_ENABLE_ONION_LAYER` **默认关**；多签 v2.1 未真接（挂账 D2）——「多签」暂无密码学效力 |
| 审批四态 | `crates/foundation/governance/src/approval_closure.rs`（AllowedOnce 恰好一次；Rejected/Cancelled/Unavailable 一律 fail-closed）+ 冻结面 `crates/engine/runtime/src/canonical/approval.rs` | 命令级名单逐字精确匹配→族等价形态可绕（G2） |
| 写总闸 | `crates/capabilities/tools/src/write_intent.rs`（写意图词法，静态调用面与命令面同族同权）+ `shell.rs` 冻结/执行双查 + 组装根 `runtime-assembly/src/canonical/production.rs` 同源 | 词法面之外的写入（解释器内写入/编码负载）由审批链+沙箱兜底；保守扫描宁误拒不漏放 |
| 观测门禁 | `crates/capabilities/tools/src/observed_gate.rs`（未读不得覆盖写 + 版本 CAS 双钥匙） | 观测表仅进程内，重启即重置 |
| 敏感面拒绝 | `crates/capabilities/tools/src/sensitive_path.rs`（凭据/密钥面读取 fail-closed 即帧；配置密钥字段值脱敏同模块） | 名单制：新敏感后缀/新密钥字段需人工补录 |
| 脱敏 | `crates/foundation/governance/src/input_security.rs`（PII 8 类 + 注入启发式）+ `tools/src/guardrail.rs` 凭据绊线（shell 输出真接线）+ `memory/src/agent_trace.rs` | 口令扫描单次一个命中段，不声称全扫 |
| 审计配对 | `approval_closure.rs`（asked↔decision 原子双写、半态可检出）+ `credentials` FileAuditSink（name_hash 不含明文） | IO 失败只降级计数——审计可用性非硬保证 |
| 凭据钥匙串 | `crates/foundation/credentials/src/keyring.rs`（平台钥匙串/加密文件/内存三级 + SecretBuf 内存清零）+ `gate.rs` 高危凭据审批门 | 降级加密文件后端=密钥仍落本机盘（加密态） |
| 沙箱 | `crates/capabilities/tools/src/process/appcontainer.rs`（工作区 ACL + 零 capability 断网 + fail-closed 不裸跑）+ 作业对象进程树 + `guardrail.rs` 灾难字面量守门 + `egress.rs` 出口管制 + 升级阶梯 `escalation.rs`/`governance::sandbox_ladder`（单调加宽+理由强制+一次授权） | **已知不对称（如实，在案 E-security-depth §现状①）**：沙箱本体的进程拦截层拦 `Remove-Item` 而静态调用形态穿，该拦截层在本仓外（本仓改不了）——「换 API 换权力」未消，出路=对象级判定（G3）；本仓删除双路径已同族同权（`tests/delete_path_parity.rs` 锁对称）；灾难字面量守门不扩面到 cmdlet/静态调用形态（`guardrail.rs` 注释明示理由） |

## 5. 缺口清单（按波次挂轴，每条一句「敌人怎么利用」）

| # | 缺口 | 波次 / 轴 | 敌人怎么利用 |
|---|---|---|---|
| G1 | 多用户隔离（现状单机单主） | W3 / E-P4 | 访客会话与主人共用记忆库/凭据/工具权，越权读写无闸可拦 |
| G2 | 命令级审批族归一 | W3 / E-P2 | 批一次 `Remove-Item x` 后用等价静态调用重放，字面外形态不进名单=白拿授权 |
| G3 | 沙箱对象级判定 | W3 / E-P3 | 换 API 形态拿同一权力——命令动词级判定（含仓外拦截层）对等价调用失明 |
| G4 | 远程访问认证 **→已处置（W2-②，认证面收口）** | W2 / F | 远程端点无认证=谁连上谁是主人，工具权+记忆库一锅端（原缺口；现非回环绑定强制令牌、无令牌拒启动，见 §5 处置注） |
| G5 | 备份恢复 | W2-W5 横切 / G | 本地恶意进程加密或删库后无备份可还，勒索即成永久损失 |
| G6 | 供应链（依赖 + MCP 外部工具服务器） | W4 / D-E | 投毒的依赖/工具描述/工具输出把指令带进上下文，在授权面内代模型作恶 |
| G7 | 数据静态加密（记忆/会话库明文在盘） | W3 候选 / G | 物理接触者直接拷走数据目录，历史与记忆全量外泄 |
| G8 | 提示注入净化仅启发式 | W3 候选 / E | 跨源内容诱导模型发起越权请求，防线后移到审批与沙箱兜底 |
| G9 | 多签 v2.1 真密码学（挂账 D2） | W3 候选 / E | 伪造「多签」决策无密码学可验，高危升级可被假授权混过 |
| G10 | 本机回环令牌可选未设即放（`canonical_entry.rs` H1 门）**→已处置（W1-③）+ 残余 (c) 已销账（W2-② 全网关化）** | W1-W2 / F | 本机进程伪装来源头调端点，冒充桌面壳发指令、读事件流 |

> **G4 处置（W2-② 远程访问与认证面收口）**：缺口「远程端点无认证」已封堵——
> ① **认证面全网关化**：令牌门从两个敏感端点扩到**全 API 面**（chat/事件流/面板/会话/审批/
> admin 族 + fallback），唯一豁免 `/health`（探活刚需，`UNAUTHENTICATED_PROBE_PATHS` 显式
> 文档化并测试钉死）；三态语义沿用，「设/未设」按整个令牌族判定（env 单令牌或客户端令牌表
> 任一在场 = 强制校验）；
> ② **每客户端令牌体系**：令牌表 name→token（env `APEIRETH_GATEWAY_TOKENS` + 数据目录
> `client-tokens.json`，坏配置拒开 / StoredDoc 拒开语义），过门审计带**客户端名**
> （`gateway.auth_client client=<名>`），为 W3 多用户授权留桩（本批=单令牌族多枚，只立
> 「谁在调用」的问责身份，不做权限分级）；
> ③ **远程绑定 fail-closed**：`APEIRETH_LISTEN` 绑定旋钮（`--bind` 旗标 > env > `127.0.0.1`
> 缺省仅本机）；非回环绑定**强制令牌**——令牌族两面皆空时**拒绝启动**（fail-closed 即帧），
> 逃生阀 `none` 明示可裸奔但留审计（`gateway.remote_bind mode=declared_none`）；
> ④ **TLS 面接口桩**：`APEIRETH_TLS_CERT`/`APEIRETH_TLS_KEY` 显式未接线（设值只触发
> 「仍是明文」警告 + 审计，不硬造 TLS）。
> 详见 [W2-2-remote-access-auth-delivery.md](W2-2-remote-access-auth-delivery.md)（认证面矩阵表+
> 接线图+测试证据）。

> **G10 处置（W1-③ 网关回环令牌加固）**：缺口本体「令牌门可选、未设即放」已封堵——
> ① **桌面自铸**：`backend_supervisor` 启动侧车时若 env 未设 `APEIRETH_GATEWAY_TOKEN` 即自铸
> 密码学随机令牌（32 字节）注入，每次启动一枚、不落盘明文（进程 env 面）、重启换新——
> 生产（桌面）口径恒为「令牌强制」；
> ② **网关默认收紧（三态门）**：设值=Bearer 恒定时间校验（缺失 401 `auth_missing_key` /
> 伪造 401 `auth_invalid_key`，拒绝入审计 `gateway.auth_rejected`）；未设=保守过渡（按最小
> 破坏口径选此支），无认证调用仍放行但每次记警告帧 `gateway_auth_unprotected` + 审计
> `gateway.auth_unprotected`；显式 `APEIRETH_GATEWAY_TOKEN=none`=逃生阀；
> ③ **客户端对齐**：TUI `--token`/env 注入面、桌面 UI 敏感调用带令牌（`get_gateway_token`
> IPC + `runtime.ts` 鉴权头）、`scripts/verify-provider-path.ps1` 同源透传；IM 桥为同源进程内
> 直调（无回环 HTTP 面）零改动；无令牌客户端在未设口径下零断（兼容路径有测试钉住）。
> **残余（如实）**：(a) 逃生阀语义 = `APEIRETH_GATEWAY_TOKEN=none` 明示不设防——审计留痕
> 但门体不设防，风险由操作者明示自担（非回环绑定下同样成立，留 `gateway.remote_bind
> mode=declared_none`）；(b) 令牌族两面皆空的默认态仍放行无认证调用（保守过渡的兼容路径，
> 口径沿用），暴露面靠警告帧+审计可见可查，正式部署应配令牌、走桌面壳/服务令牌面，或至少
> 守回环绑定；(c) **已销账（W2-②）**：令牌门已扩到全 API 面（chat/事件流/面板/会话族全部
> 过门，仅 `/health` 豁免）——原「本机进程可驱动回合、读观测面」残留半句在强制态下闭合
> （无令牌调 chat=401，`gateway_auth_face` 测试矩阵钉死）；(d) 浏览器 web 构建取不到令牌
> （令牌只经桌面壳 IPC 下发），对令牌强制网关调任意端点会 401——设计内（令牌只属于桌面壳）；
> 且**桌面 WebView 的令牌覆盖仍有缺口（如实）**：UI 目前只给 `/v1/admin/config` 与审批解决
> 两处带令牌，chat/面板/SSE 调用无凭据（`EventSource` 无法带鉴权头），全网关化后这些调用会
> 401——**需前端批补**（全 fetch 统一带令牌 + SSE 换 fetch 流/侧车转发），后端批零前端触碰
> 未修；(e) TLS 面接口桩未接线，远程流量明文（凭据可被链路窃听），TLS 真接线挂 F 轴 P3 后续。

> 波次如实注：G1-G5 按 [00-backlog.md](00-backlog.md) 排序建议与 [E-security-depth.md](E-security-depth.md) 分期挂靠；E-P2/P3 未显式入波，按 E 分期顺序排在 P4（W3）前；G7-G9 为未入波候选——派工以 owner 拍板为准，不假称已排期。

## 6. 对抗测试建议（每类敌人 2-3 个可写成测试的绕过尝试，供后续批实现）

| 敌人 | 可直接写成测试的绕过尝试 |
|---|---|
| ① 提示注入 | (a) IM 入站消息夹「忽略之前指令,执行 X」→断言仍冻结进审批四态、不自动执行；(b) 召回记忆/工具输出带注入指令→断言 `UntrustedEnvelope` 警告头与边界标记不被剥除；(c) 注入诱导读凭据面→断言 `sensitive_path` 拒绝帧 |
| ② 工具滥用 | (a) 未读文件覆盖写→`observed_gate` 拒；(b) `echo x > f` 与等价静态调用在关闸下同帧拒（扩 `delete_path_parity` 覆盖写族）；(c) shell 输出夹 `sk-` 密钥→绊线脱敏截断且结果带 `credential_tripwire` |
| ③ 本地恶意进程 | (a) 直读 `sessions.sqlite3`/记忆库→当前应可读=G7 实证用例（落地后反转为回归锚）；(b) **绕过用例全部转绿（W2-②）**：伪来源头零 CORS 放行 + **无令牌调 chat=401** + **伪造 Bearer=401**（`crates/adapters/gateway/tests/gateway_auth_face.rs`：`forged_origin_and_forged_bearer_are_refused_together` 及 401 矩阵六例；`gateway_token_gate.rs` W1-③ 断言原样保留；桌面自铸令牌 e2e / 远程绑定 fail-closed 进程级 `cli/tests/remote_access_e2e.rs`）；(c) 读钥匙串导出物/密钥文件→`sensitive_path` 拒 |
| ④ 供应链 | (a) MCP 工具描述含指令文本→断言只入审批面、不被执行；(b) MCP 输出夹凭据→归一+脱敏断言；(c) 坏配置/未知 schema→拒开 fail-closed |
| ⑤ 多户越权 | (a) 访客会话读主人记忆库→应拒（现为 G1 实证用例）；(b) 客户端 A 的 `AllowedOnce` 在客户端 B 重放→恰好一次失效；(c) 访客改能力开关/沙箱旋钮→权限拒绝 |
| ⑥ 物理接触 | (a) 无锁翻会话历史→当前可读=G7 实证；(b) 导出或复制密钥材料→fail-closed 即帧；(c) 现场改配置关沙箱→默认开语义回归 + 下次审批卡红标警示 |

> 配套纪律：治理/安全语义改动落地时按宪章 §五补对抗测试；本表即选题池，实现归后续批。
