# 协同模式矿 · 多 AI 并行施工的实战协同模式（批 L1 沉淀）

> `Status: ✅ 批 L1 交付（2026-10-15，纯文档批 · 未 commit）`
> 一事：从本战役各批交付文档的「报备/接棒/协同注」与 git 提交历史提炼**可复用的协同模式**；
> 每模式 =【情境→做法→效果】三段，每条注明素材出处（文内链接）。
> 矿脉图：[L-experience-distillation.md](../../archive/backend-enhancement/L-experience-distillation-2026-10-15.md)（本批挖第一矿）；
> 施工宪章：[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md)。
> 素材口径：全部可指回仓内交付文档 / 债栏注记 / git log；会话记录侧的补充批注仅作旁证并如实标注。

## 模式 1 · 叠加不覆盖：共树并行的呼吸边界

【情境】10+ 施工线共用一棵工作树（同一 `Cargo.lock`、同一批共享文件如 `production.rs`），任何一线的半成品都会被他人踩到；宪章 §二 立了「叠加不覆盖」，但真实边界要靠实战画出来。

【做法】三件套落地：① **edit 前先读后改**，文件冲突后到者 rebase，不覆盖他人在途工作；② **共享收敛点不手改**——`Cargo.lock` 交给包管理器收敛（并行线的依赖边入锁由 cargo 收敛），以 `--locked` 门禁复验一致性；③ **战场禁入**——门禁取证期间撞见他线在途半成品（编译错误/格式不净），零触碰零代改，报备待归口线自检收口，绝不"顺手帮它修好"。

【效果】多线同树施工全程零互相覆盖事故；取而代之的只是可定性的瞬时红（见模式 6），交付报告可以同时写出"本批全绿"与"他线在途红非本批引入"两句真话。

> 出处：[zero-debt-opening-clearance-2026-10-12.md](../backend-enhancement/04-delivery/zero-debt-opening-clearance-2026-10-12.md) §6 收尾备注（叠加不覆盖 + `Cargo.lock` 共享收敛点）；[git-write-tiering-delivery.md](../backend-enhancement/git-write-tiering-delivery.md) §6 共树说明；[quick-capture-delivery.md](../backend-enhancement/quick-capture-delivery.md) §4.3（他线战场禁入、零代改）。

## 模式 2 · 跨线补位：编译断点按"对方口径"补位

【情境】W3-② 取证期间，W3-① 并行线的在途结构演进**两次造成全树编译中断**（① `SelfStatusSnapshot` 增字段后构造器未跟上；② `ProductionBackends` 增字段后装配字面量未跟上）——不是本批的战场，但全树红会卡死所有人的门禁。

【做法】补位不越权：按**对方自己声明的口径**（「未接线 = 显式 null/None + 原因」）补最小占位（`DATA_PROBE_NOT_WIRED` / `None`）并在代码里带报备注释，域探测口真接线**归口 W3-① 线**；顺手做的 `cargo fmt` 格式补平明确声明为"机械空白，无语义改动"，与语义改动严格分开。

【效果】全树编译当场恢复、门禁继续取证；W3-① 线随后真接线**无缝覆盖占位**（占位语义与其真实接线同形），零返工零冲突、零口径漂移。

> 出处：[W3-2-user-identity-roles.md](../backend-enhancement/04-delivery/home-W-axis-delivery.md)「共树协同注（W3-① 并行线）」段（两次编译中断的补位记录）。

## 模式 3 · 占位互保：为在途线留位，而不是替它完工

【情境】多线同时演进同一装配面/自述面，一线的交付必然撞上另一线尚未落地的接口字段。

【做法】为在途面落**显式占位**（`None` + 原因 + 报备注释），且占位必须"可被识别并喊破"——不做假旋钮。正例一：W3-① 为 J1 内容流节落 `None` 占位（J1 真接线随后无缝覆盖）、`cli/lib.rs` 装配字面量补 `self_status_content_feed: None` 占位并注明"探测接线归口 J1"。正例二：W2-② 的 TLS 接口桩（占位旋钮）配了专门测试 `tls_placeholder_knobs_are_reported_as_unwired`，让占位被识别时**大声自报未接线**。占位的真接线归口原线，谁占位谁登记。

【效果】并行线互不阻塞；占位不会沉淀成"看起来能用"的假功能（零假旋钮纪律在并行面的延伸）。

> 出处：[W3-1-multi-user-data-isolation.md](../backend-enhancement/04-delivery/home-W-axis-delivery.md) §7「并行批接缝互保」段；[W2-2-remote-access-auth-delivery.md](../backend-enhancement/W2-2-remote-access-auth-delivery.md) 测试表 `tls_placeholder_knobs_are_reported_as_unwired` 行。

## 模式 4 · 报备先行、接棒兑现：接缝当合同写

【情境】两线的交付天然互为接缝：W3-① 域面**建立** `UserDomain.role`、W3-② 治理层**消费**它；`users.json` 的 `domain` 引用一方只携、另一方判域。接缝错位要到集成时才炸。

【做法】开工前互相**报备接缝口径**（宽接口形状、冻结词表、谁携引用谁判域），交付文档抬头显式写「接缝（X 并行线，已兑现其报备）」；接缝行为用测试钉死（如 `the_domain_role_feeds_the_governance_identity_wide_interface`、`the_wide_role_interface_refuses_unprovisioned_and_unknown_roles`）。交付顺序无关——后交付方在文档里写明"兑现了先交付方的报备注记"。

【效果】两线并行交付后接缝零错位、零事后对账改口径；接缝测试让"报备"从口头承诺变成可执行合同。

> 出处：[W3-1-multi-user-data-isolation.md](../backend-enhancement/04-delivery/home-W-axis-delivery.md) 文档头「接缝」注（"已兑现其报备"）+ §6 测试表；[W3-2-user-identity-roles.md](../backend-enhancement/04-delivery/home-W-axis-delivery.md) 文档头「接缝」注 + §4 测试表。

## 模式 5 · 协调者归并节奏：波次收卷，口径只在收卷点刷新

【情境】多线各自交付后成果散在各线文档与工作树里；若不定期归并，测试数/套件数/版本轴这类口径会漂移，后来者无从接手。

【做法】协调者按"波/战役"节奏做三件事：① **归并快照**——把并行线在途成果一次收拢（git log 实证：`c0867872` 并行工作流成果归并多线 + 增强文档、`c74786c1` 四批交付快照 + 并行线在途归集、`4beda58b` 总装批收口四波集成）；② **战役文档全量刷新**（`91b2b614` 存档点——战役文档全量更新）；③ **口径收卷**——套件/例数这类全量数字只在收卷点刷新（如「190 套件 / 5243 例」），单线交付文档只报自己批的增量与共树在途的归属。

【效果】任何时点接手都有一个"当前真相"文档面；104 个子代理规模的大战场没有出现口径分裂，00-backlog 的状态列可以在一次刷新里对齐全部在飞线。

> 出处：git log（`c0867872` / `c74786c1` / `4beda58b` / `91b2b614` 提交主题）；[00-backlog.md](../backend-enhancement/00-backlog.md) 状态列「2026-10-13 文档同步批刷新」注。

## 模式 6 · 同文件并行的呼吸与瞬断自愈：先定性再定罪

【情境】同一 build 目录/同一文件被多线同时使用时会出现"瞬时红"：一次退出码 101、一次格式不净、一次编译中断，复跑即绿。粗暴归因会冤枉好人，含糊放过会掩盖真凶。

【做法】**先定性再定罪**：瞬时红先隔离复跑——若隔离复跑即绿、且期间有并行扰动证据（同 build 目录出现并行 `cargo` 抢锁 "Blocking waiting for file lock"），按 flake 口径定性为「并行在途扰动」，非本批引入；报告写法固定三件：如实标红 + 明确归属线 + 定性理由。反过来，负载敏感的 flake 不许用"复跑绿了"收账——要定根（file_lock 案：标记瞬时失败被误判持有者已死 → 误删活跃锁，改为"接管只认确证"后才算治根）。

【效果】门禁报告保持诚实（红就是红），责任面清晰；随各线收口自然转绿，零冤案零甩锅；真 flake 被逼着定根而不是被复跑掩盖。

> 出处：[quick-capture-delivery.md](../backend-enhancement/quick-capture-delivery.md) §4.3 注（抢锁扰动定性）；[W3-2-user-identity-roles.md](../backend-enhancement/04-delivery/home-W-axis-delivery.md) §6 门禁证据（"并行线两次瞬时红……均非本批引入"）；[zero-debt-opening-clearance-2026-10-12.md](../backend-enhancement/04-delivery/zero-debt-opening-clearance-2026-10-12.md) §1 第 3b 行 + §6（flake 定根与定性口径）。

## 模式 7 · "见债必还"的顺手修尺度：修 / 报 / 登记三判

【情境】施工路上不断撞见既有债（挂名不跑的测试、陈旧注记、孤儿面），顺手修是纪律（宪章 §一.2），但顺手过头就变成越界改他人战场。

【做法】三判分流：
1. **修**：确证级、在本批验收路径上、改动限最小集——例：`#[test]` 属性被并入上一行行尾注释而**静默失效**的陈旧断言（`dead_code = allow` 使其无警告漏网），顺手恢复挂测并钉本批新面，测试注释注明；
2. **报**：不在战场声明内、但本批验收必须经过的面——最小改动 + 报备 + 理由（如 read_only 名单 +1 项：名单缺项 = 只读会话静默放行的病灶，验收必须由它承载）；
3. **登记不实施**：超范围的"顺手重构"念头一律登记（R1/R2 式条目带理由）不实施；他人在途的红**标红不代改**，报备待归口线自检收口。

【效果】债在清、界不越、报告可审计；"路过就算"与"顺手大改"两种病都不得流行。

> 出处：[worktree-apply-delivery.md](../backend-enhancement/worktree-apply-delivery.md) §五「还债与报备」（修 + 报两判实例）；[zero-debt-opening-clearance-2026-10-12.md](../backend-enhancement/04-delivery/zero-debt-opening-clearance-2026-10-12.md) §4 逐条裁决（清 3 条 / 留账带理由）+ §5 新债与 R1/R2 登记不实施；[quick-capture-delivery.md](../backend-enhancement/quick-capture-delivery.md) §4.3（标红不代改）。

## 模式 8 · 接棒制遗嘱：接棒清单 + 就地收口标记

【情境】批不可能都一次做完，中道停机与跨批接续是常态（宪章 §二.3「半成品不弃尸」）；没有遗嘱的半成品就是弃尸。

【做法】交付文档固定一节「接棒清单（未完）」，每条写清四件：落点文件、注册条件、现成零件、验收形状——让下一位不用考古就能动手；接棒项完成后**就地标 ✅ 并注明收口批**（如"→ ✅ 已收口（2026-10-13 总装批件一）"，附注册条件兑现与新增测试名），不留悬空的"未完"。

【效果】跨批接续成本低（worktree 两级开关的组装根注册由总装批按清单原样收口）；接棒清单是活文档而非墓碑，"未完"列表随时反映真实残余。

> 出处：[worktree-apply-delivery.md](../backend-enhancement/worktree-apply-delivery.md) §六「接棒清单（未完）」（含 ✅ 收口注）；[D-doing-work.md](../backend-enhancement/02-axes/D-doing-work.md)「组装根注册接线见交付记录接棒清单」。

## 附：模式 ↔ 宪章条款对照

| 模式 | 宪章锚 | 本矿新增经验 |
|---|---|---|
| 1 叠加不覆盖 | §二.1/2 | 共享收敛点不手改 + `--locked` 复验；他线红标而不代改 |
| 2/3 补位与占位 | §二.4 | 补位按对方口径、占位可喊破、真接线归口原线 |
| 4 报备先行 | §二.4 | 接缝报备当合同写、接缝测试钉死 |
| 5 归并节奏 | §三.2 | 全量口径只在收卷点刷新 |
| 6 瞬断自愈 | §四.1 | 瞬时红先隔离复跑定性；真 flake 必须定根 |
| 7 顺手修尺度 | §一.2 | 修/报/登记三判（新增条款见宪章 §六.3） |
| 8 接棒遗嘱 | §二.3 | 接棒清单四要素 + 就地收口标记 |
