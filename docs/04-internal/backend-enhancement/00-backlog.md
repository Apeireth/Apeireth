# 后端增强待办（想法级索引）

> `Status: 📋 待办（2026-10-02）`
> 纪律：本页**只列想法**（一行一事）；细节一律折进同目录小文件（每文件一主题）。
> 开工时认领想法 → 拆批派工 → 细节文件随批更新；想法完成就在本页标 ✅ 并把细节文件归档。
> 状态列（2026-10-13 文档同步批刷新）词表：**✅ 已交付**（指针到交付记录）／**🔧 在飞**（在建或部分交付，含他线在途）／**📋 未开工**。

## 后续方向（2026-10-15 立）

**[V1-V6 施工序](V-forward-direction.md)**：上屏面兑现 → 星群正身 → 活在生活里 → 记忆升维 → 安全守望 → 编排绑定（调研综合裁定，见总裁决篇）。

## 想法清单

| # | 想法 | 一句话 | 状态（2026-10-13 刷新） | 细节文件 |
|---|---|---|---|---|
| A | **配额真接口** | 四维配额从"只有类型"变"真执行"——烧钱闸数值化 | 🔧 在飞：P1 token/步数 + P2 花费维**已交付**（配额三维计量/超限即帧/自述真值，见细节文件 P1、P2 实施记录；数据面合同 [frontend-data-contract.md](../../02-guides/frontend-data-contract.md) §10.2）；深度维**暂无接口**（如实），P3 待开工 | [A-quota-real-interface.md](A-quota-real-interface.md) |
| B | **性能极致** | 找写得不好的开刀：冷启动红行、热点、并行 | 📋 未开工（横切纪律随波验收；file_lock 负载 flake 已定根 2026-10-12、锁竞争仍列热点测量对象——见细节文件现状） | [B-performance.md](B-performance.md) |
| C | **记忆深化** | consolidation/reflexion/自调校/图谱向量上生产 | 🔧 在飞：随手记入口面**已交付**（[quick-capture-delivery.md](quick-capture-delivery.md)）；夜间整编/教训回路/图谱向量生产待开工 | [C-memory-deepening.md](C-memory-deepening.md) |
| D | **干活能力** | worktree_apply 受控写 + git 写分级（commit 级走审批） | 🔧 在飞：P1 git 写分级 ✅（[git-write-tiering-delivery.md](git-write-tiering-delivery.md)）、P2 worktree_apply ✅（[worktree-apply-delivery.md](worktree-apply-delivery.md)）；P3 任务编排、P4 MCP 深化待 | [D-doing-work.md](D-doing-work.md) |
| E | **安全纵深** | 沙箱对象级判定、审批族归一、多用户权限 | 🔧 在飞：P1 威胁模型 ✅（[E-threat-model.md](E-threat-model.md)）+ W1-③ 令牌加固/W2-② 认证面/W3-① 域数据门/W3-② 角色门**已交付**（销账对齐见 [E-threat-model.md](E-threat-model.md) §4/§5 处置注）；E-P2 审批族归一、E-P3 沙箱对象级判定**在飞** | [E-security-depth.md](E-security-depth.md) |
| F | **服务端形态** | headless daemon、多客户端复用、远程访问（T2 中枢） | 🔧 在飞：W2-① daemon 化 ✅、W2-② 远程认证面 ✅（[W2-2-remote-access-auth-delivery.md](W2-2-remote-access-auth-delivery.md)）、W2-③ 多客户端会话复用 ✅（[W2-3-multi-client-session-reuse.md](W2-3-multi-client-session-reuse.md)）、W3-① 跨户数据面 ✅（[W3-1-multi-user-data-isolation.md](W3-1-multi-user-data-isolation.md)）、W3-② 角色门 ✅（[W3-2-user-identity-roles.md](W3-2-user-identity-roles.md)）、W3-③ 星群逐域伙伴实例 ✅（[W3-3-domain-partner-instances.md](W3-3-domain-partner-instances.md)：每户伙伴运行时实例 + 域级模型/人设 + chat/事件面按域开放）；TLS 真接线（现接口桩）、域面余面（面板/审批按域）、P4 部署形态待 | [F-server-form.md](F-server-form.md) |
| G | **观测与可靠** | 指标数据源、崩溃恢复、备份 | 🔧 在飞：P1 指标数据源 ✅（`GET /v1/panel/metrics`，见细节文件文末交付记录；**前端消费待上屏**→债栏 D9）；P2 tracing 时间线、P3 崩溃恢复、P4 备份待 | [G-observability.md](G-observability.md) |
| K | **物理在场与设备控制** | 五层：L1 感知（传感器/摄像头/环境读）→L2 提醒（天气/推测预警）→L3 受控执行（家电白名单）→L4 代理代办（购物/下单——动钱永远人批）→L5 守护（看家/异常告警）。四底线：设备白名单/动钱人批/不可逆零自主/先模拟后执行。能力宪章=owner 与友人对话实录（2026-10-18） | [K-life-presence.md](K-life-presence.md) |
| L | **会话经验沉淀** | 431 轮/104 子代理的工程经验→项目资产（协同模式/死亡解剖/设计裁定/方法论/产品洞察五矿脉） | [L-experience-distillation.md](L-experience-distillation.md) |
| P | **吸收自动化（P 轴 · Practice 沉淀）** | 书→技能/随手记/历史即记忆——把『吸收优秀实践』变成产品本能 | 🔧 在建：P1 书→技能**已交付**（见 [P 轴分期](I-absorption-automation.md) 分期表；交付记录随线落盘）；随手记入口面 ✅（[quick-capture-delivery.md](quick-capture-delivery.md)）；P2-P4 待排期 | [P 轴分期](I-absorption-automation.md)（P1 书→技能已交付；候选池 [吸收候选池](../absorption-candidates-2026-10.md)） |
| H | **模型面** | 多模型路由/fallback、成本分级 | 🔧 在飞：P1 模型画像表 ✅（见细节文件 P1 实施记录；单价表来源已供 A-P2 花费维）；P2 fallback、P3 路由、P4 成本分级待 | [H-model-plane.md](H-model-plane.md) |
| I | **内置应用面（星港）** | 多标签壳+贡献点注册表+内置件面（伴星/星笺/深望等 15 面），批次按工程速度 | ✅ 17 面全部点亮（2026-10-11，批次台账见细节文件）；🔧 增强位随批清（最末两件等 W1/E 契约） | [I-starport-builtins.md](I-starport-builtins.md) |
| J | **生活信息流（信风）** | 多源订阅+摘要+陪伴共看，接入现代生活与网络 | 🔧 在建：J1 内容流接入在途（`tool.content_feed` 订阅/拉取/摘要/入记忆流 + 自述订阅节，治理档位已钉死；交付记录随线落盘） | [J-tradewind.md](J-tradewind.md) |
| K | **便携随身（星槎）** | U 盘随身：绿色单体+数据随身+现场收紧放权 | 🔧 在飞：打包随身脚本/数据随身/真判定已点亮（I 星港批次记录「星槎便携」）；现场收紧放权（E 联动）、自愈备份（G 联动）待 | [K-starraft.md](K-starraft.md) |

> **命名注记（审计修 D13，2026-10-16 口径尾账批）**：吸收自动化原为 **I 轴**（分期 I1-I4），与星港内置面（**I**）字母双占——统一改名 **P 轴（Practice 沉淀）**，分期同步 **P1-P4**（原 I1-I4）；文件名 `I-absorption-automation.md` / `I1-skill-distill-delivery.md` **保留旧名避免断链**，内容带别名注记。星港线保留 **I**（含「I 批」批次号），0 撞名。

## 架构定案（2026-10-02 owner 拍板）

**星群为骨、共享为翼**：每户伙伴实例独立（隐私硬边界、性格独立养成），中枢选择性共享**匿名化经验**（教训/技能，不含记忆原文）——既守「它只记得你」的魂，又吃群体智慧。详见 [F-server-form.md](F-server-form.md)。

## 施工纪律

**[CONSTRUCTION-DISCIPLINE.md](CONSTRUCTION-DISCIPLINE.md) 为进场第一课**（多 AI 共同施工宪章：零债/叠加不覆盖/口径诚实/门禁/验证）。

## 排序建议（owner 可改）

施工波序（全都要，分波交付）：**W1 看见与管住**（A 配额+G 指标+E 威胁模型）→ **W2 站住 7×24**（F daemon/多端/远程）→ **W3 多户多端**（E 多用户纵深）→ **W4 能干活**（D）→ **W5 更聪明更省**（C+H）。B 性能与 G 备份为横切纪律随波验收。

**零债开场白**：W1 前先清既有债（src-tauri 告警/登记欠账）。

## 债栏（登记债落账，宪章 §一.2「见债必还」的登记面）

> 清偿/留账全文见 [zero-debt-opening-clearance-2026-10-12.md](zero-debt-opening-clearance-2026-10-12.md)（清偿对照表 + 门禁证据）。
> **零债开场白已收**（2026-10-12）：src-tauri clippy 55→0、ratatui 0.30 迁移（P11 根治）、file_lock 负载 flake 定根、全局槽竞态复核闭合、孤儿/死面清理。

| # | 登记债 | 卡点 / 理由 | 落点 |
|---|---|---|---|
| D1 | 台账 §2 挂账族（真机点击流 / live E2E 换 key / embedding 端点） | 需主人人工、有效凭证、可用端点 | `live-verification-ledger.md` §2 原行 |
| D2 | stage123-gap-matrix 挂账行（物理多签 v2.1 / UpgradeCycle 留白 / 阈值校准 / Kani 真接线） | 拍板推迟或需阶段实测校准；产品语义红线禁本批动 | 各行理由在册 |
| D3 | 前端欠账 6 项 + 设计系统仍断面 | 前端产品/审美工作，观感红线 | `docs/design/frontend-handoff.md` §6 |
| D4 | reflexion 文件并发偶发 flake（台账 #57） | 疑与 file_lock 同根，本批定根后**待复跑观察定性**，不虚报已消 | 台账 #57 顺记 |
| D5 | deny.toml `[bans]` v1 时代 duplicate skip 条目理由对账 | 非本批造成；deps 治理批复核 | `deny.toml` |
| D6 | `TunableParam::effective()` 宿主 env 测试敏感面 | 非竞态；测试侧减敏待测试治理批统一做 | 清偿表 §5 N6 |
| D7 | 认知调度器任务淘汰序非严格 FIFO（按 `task_id` 字典序，非最旧提交者） | M1 有界化时如实注记的边界；严格 FIFO 需给 TCB 加 `submit_seq`，属调度器内部改动（W1-② 进场读到，按宪章 §一.2 登记） | `cognitive_quota_scheduler.rs` 淘汰注记 |
| D8 | **治理流程"智囊团审议 + 物理多签"整机未合练**（2026-10-11 PHL-08 直加时审计发现：`multi_sign` 全库调用点均为 `None`、审议→多签→反思期串联零执行记录、反思期审计默认不运行） | 流程零件实装但从未被实践验证——与 D2（缺件）不同，D8 是**有件未合练**；建议排一次治理演习（模拟变更走完整链）再宣称可用，别等真变更才首演 | 本条即审计记录；相邻账 D2 / stage123-gap-matrix §7e 相关行 |
| D9 | 【待上屏】指标端点 `GET /v1/panel/metrics` **已建成但无 UI 面**（驾驶舱 CACHE/BUDGET 面板与聊天头遥测条仍是"-"） | G-P1 只供数据、前端消费明注为后续批（零假旋钮：不上屏不编数）；数据面合同已备好等前端接 | `G-observability.md` P1 交付记录 + 缺口登记③；合同 [frontend-data-contract.md](../../02-guides/frontend-data-contract.md) §10.1 |
| D10 | 【待上屏】用户域/用户管理**仅有 CLI**（`apeireth user add\|list\|role\|remove`；域开通/改显示名/删域「暂无接口」） | W3-①/② 后端面已建成，管理 UI 未做；域管理 API 已备（`DomainRegistry::create`）等管理面批 | `W3-1-multi-user-data-isolation.md` §7 残余表 / `W3-2-user-identity-roles.md` |
| D11 | 【待上屏】客户端令牌表**无管理 UI**（env `APEIRETH_GATEWAY_TOKENS` + 数据目录 `client-tokens.json` 配置面） | W2-② 令牌体系已建成；增删客户端/换令牌只能改配置面，UI 管理面未做 | `W2-2-remote-access-auth-delivery.md` |
| D12 | 【待上屏】daemon/多客户端**运维面无 UI**（服务生命周期、同会话写排队审计、会话归属审计只在日志/审计面） | W2-①/③ 已建成；驾驶舱/设置页未接运维读数，主人看不到"谁在线、谁在排队" | `F-server-form.md` 现状 + `W2-3-multi-client-session-reuse.md` |
| D13 | 00-backlog 想法清单 **#I 字母双占**（吸收自动化与星港内置面同用 I） | 命名跨线（I1 施工线注释/分期文档同用「I 轴」口径），单方改名会撞他线命名——待在途线收口后统一改或另立字母。**状态注记（2026-10-16 口径尾账批，审计修 D13）：✅ 已清**——吸收自动化统一改名 **P 轴（Practice 沉淀）**、分期 I1-I4 → P1-P4，全库引用同步（src 注释/用户手册/数据合同/能力矩阵/台账/候选池同批）；`I-absorption-automation.md` / `I1-skill-distill-delivery.md` 文件名保留 + 内容别名注记防断链；星港线保留 I，双占解除 | 本页想法清单两行；`I-absorption-automation.md` / `I-starport-builtins.md`；对照表 [audit-gap-caliber-hygiene-2026-10-16.md](audit-gap-caliber-hygiene-2026-10-16.md) §4 |
| D15 | 【审计修 B-04】守门级数口径 5 重 / 6 重 v6 / 7 重 v7 并存（外部审计清单 B-04） | **状态注记（2026-10-16 口径尾账批）：✅ 已清**——现值统一 **7 重守门 v7**（含 Colang DSL + Superpowers Skill Guard，ROADMAP §13 唯一现值出处）；历史文档（CHANGELOG / commit-msg 档案 / 审计卷宗 / OSS_NOTICE / library v1.0 档）逐处加「历史口径（日期）」标注、原值不动；v1.0 时代 9 重 v9 留痕（Cargo.toml metadata 已加历史口径标注；philosophy.md O-1 锚描述与 core 源注释属 LOCKED/留痕 0 触碰）登记见对照表 | ROADMAP §13 + [audit-gap-caliber-hygiene-2026-10-16.md](audit-gap-caliber-hygiene-2026-10-16.md) §2 |
| D16 | 【审计修 B-05】12→13 键行与「13 键 LOCKED」表述不一；R11 baseline 3 值只有标签无可复现入口（外部审计清单 B-05） | **状态注记（2026-10-16 口径尾账批）：✅ 已清**——ROADMAP §13 现值行统一为「**13 键**（LOCKED）」（12 键降为历史口径注记）；R11 baseline 3 值如实标注「**无公开复现入口，仅供内部对照**」（const 源仅存 legacy 档，数值 LOCKED 0 触碰） | ROADMAP §13 + [audit-gap-caliber-hygiene-2026-10-16.md](audit-gap-caliber-hygiene-2026-10-16.md) §3 |
| D17 | 【审计修 F-02】大体量非源码资产随库：reports/（1071 tracked / 39.9MB）+ previews/（34 PNG / 40.9MB）+ `scripts/guard_ml/*.jsonl`（4 件 / 21.1MB） | **状态注记（2026-10-16 口径尾账批）：🟡 留账（方案已出、待 owner 拍板）**——构成清单 + 四选项（迁 release 资产 / 迁 docs/archive/ / 外置 / git-lfs）去留方案表已出，**本批只出方案表不动文件**，owner 拍板后另批执行；已查明 `scenarios.jsonl` + `fixture_artifact.json` 是 `crates/engine/guard/tests/security_scenarios.rs` `include_str!` 编译期依赖、必留库，4 大 jsonl 为库内导出器可再生件（CI 每次重生成 + artifact 兜底）；**执行批注记（2026-10-18 F-02 出库批）**：reports/ v1.0 批量日志 + UUID 工作日志 878 件已打包迁 `release-assets/reports-archive-2026-07-08.tar`（git 历史不动、711 处 docs 引用不改、`reports/README.md` 加迁出说明），previews/ 34 PNG 走 **git-lfs**（保路径零断链，未硬装降级面未触发），guard_ml 4 大 jsonl 已外置打包迁 `release-assets/guard-ml-exports-archive-2026-10-18.tar`（`scenarios.jsonl` + `fixture_artifact.json` 等 9 件留库，`cargo test -p apeireth-guard` 迁移前后均绿）；**artifacts/（2026-10-18 实测 395 件 / 172MB on-disk）仅登记不处置、owner 拍板另批，D17 继续留账**；迁移清单 [f-02-migration-manifest-2026-10-18.tsv](f-02-migration-manifest-2026-10-18.tsv) | [audit-gap-caliber-hygiene-2026-10-16.md](audit-gap-caliber-hygiene-2026-10-16.md) §5 方案表 |
| D14 | 前端套件 `frontend/companion-desktop/tests/session-preset-persistence.mjs`「治理同源」断言与生产装配**形状漂移**（断言 `Arc::clone(&session_store)` / `with_session_store(session_store)` 字面，现装配已改为 `sessions.store()` / `with_session_manager(sessions)`；`pnpm test` 1/76 套件红） | 治理语义意图（钩子与运行时共用同一会话存储）在现装配仍成立，只是断言字面落后于会话写闸装配重构一系（`bddc8aa4`/`ae953e93`）；属治理断言演进，归口批准档持久化线按「演进断言时注明理由」换新短语，非本批代改（2026-10-16 落差清账·行为小修批实测登记，宪章 §六.3 标红不代改） | `session-preset-persistence.mjs:141-146` ↔ `crates/adapters/cli/src/lib.rs:897-905` |
| D18 | `LeakedCredentialKind::JwtToken` 变体已声明但无扫描器（guardrail 后置出站绊线只覆盖 OpenAI/AWS/GitHub PAT/PEM/Slack/口令赋值 6 族，JWT 形状零检测） | 「命名即承诺」的空槽：枚举承诺了 JWT 泄露类别、检测面没接；补 JWT 形状扫描（三段 base64url `eyJ` 头）属凭据面语义扩展，超 V5-a 最小面，按宪章 §六.3 登记不实施。**状态注记（2026-10-18 凭据绊线补空槽批）：✅ 已清**——`JwtToken` 检测面补齐（`eyJ` 头 + 恰好三段 base64url：头 ≥8 / 载荷 ≥4 / 签名 ≥8 或空=未签名 `alg=none` 形态，整体 ≥20；带 `find_jwt_span` 形状判据 + 6 例测试），同时发现并补齐**同类第二空槽**：`GenericBearer` 变体亦只有声明（本批一并收口：只认 `Authorization:` 上下文 + `Bearer <opaque 16..=4096>`，散文里的 "Bearer" 一词与短值不误伤；两族互不吞并，JWT 先收口）；`apeireth-tools-canonical` lib 单测 545 passed / 0 failed（含本批 6 例新增），`cargo clippy -p apeireth-tools-canonical --all-targets -- -D warnings` 0 警告，该文件 `cargo fmt --check` 0 diff。保守边界如实登记：JWT 只认 `eyJ` 前缀的 JSON 对象头形态、单次调用只收口首个命中（不声称全扫） | `crates/capabilities/tools/src/guardrail.rs` `LeakedCredentialKind` 声明处 |

## 与既有账目的关系

- 性能红行现状见 `reports/benchmark-reproduction.md`；开关权力现状见 `docs/02-guides/capability-switch-power-audit.md`；
- 验证面硬化（Kani 证明收口）持续进行，与 A/E 的安全语义互为弹药；
- 驾驶舱 P2 遥测、对话遥测条的"未接线"槽位=G 的数据源需求清单。
