# README 术语血统镊子批 · 术语血统对照表 + 镊子改动清单（2026-10-18）

```text
[Document-Meta]
范围:  docs/design/README-draft-pending.md + README.zh-CN-draft-pending.md（只动术语）。
纪律:  镊子原则（每处 ≤2 行、只换词不重写句子）；中性表述扫描；不 commit；
       审计修正 / 门面批 / 小巧思成果零删改；LOCKED 锚本体 0 触碰。
上限:  自研术语新增 = 6 处（3 词 × 2 文件，已用满）；术语密度不升反降（见 §五）。
```

## 一、血统判定依据（证据面）

- `docs/04-internal/absorption-ledger.md` —— 机制吸收战役逐项台账（条目为中性自含命名，台账正文不记外部来源；机制来龙去脉以本台账为总指针）；
- `crates/engine/memory/src` 各数学模块头注 —— `residual_pyramid.rs` / `river_topology.rs` / `semantic_axis.rs` 头注明写「本模块为独立实现，数学基础均为公开文献中的经典方法/经典模型」并点名公开教材章节；
- `crates/engine/memory/src/{betti_hole_detector,kuramoto_resonance,chronicle_crystallizer,three_tier_vault}.rs` 头注 —— 公开经典模型 / 公开设计启发（吸收改造）；
- `crates/foundation/governance/src/{approval_family,action_binding}.rs`、`crates/engine/runtime-assembly/src/canonical/production.rs` —— 审批族 / 动作绑定 / 写总闸 / 四拍受控写 / 恒定注入 / 保鲜回路的冻结语义注释；
- `docs/04-internal/backend-enhancement/` 交付实录（V3-a 关怀双引擎 / V4-a 恒定注入 / V4-b 保鲜回路 / V5-c 守望漏斗 / W3-1 零迁移零丢失 / E-P2 审批族归一 / G2.5 动作绑定 / worktree-apply 四拍）。

## 二、术语血统对照表（术语 / 血统 / 处置）

血统四分：**自研** / **吸收改造**（公开成果的假说改造、工程改造）/ **公开经典**（公开算法·模型·概念）/ **通用**（行业通用词）。

### A. 记忆数学类（吸收改造主区，本次主要修点）

| 术语 | 血统 | 处置 |
|---|---|---|
| MGS 残差金字塔（正交残差分层提炼记忆） | 公开经典（修正 Gram-Schmidt 正交化，头注点名公开数值线性代数教材）+ 吸收改造（多层能量级联 / 新颖度门控工程化）；**非自研** | **改②③⑩⑪**：首现改通俗转述 + 诚实定语「公开数值方法的应用改造」，二现补同款定语 |
| Betti 洞探测 | 公开经典（持续同调 / Betti 数）+ 吸收改造（简化/实验级，β₂ 恒 0 占位如实标） | **改①⑨**：括注「公开经典模型的应用改造」 |
| Kuramoto 相锁 | 公开经典（非线性耦合振子同步模型）+ 吸收改造 | **改①⑨**（同括注） |
| Chronicle 结晶 | 公开经典概念（自传体记忆整编 + 幂律衰减）+ 吸收改造 | **改①⑨**（同括注） |
| DualScaled 场（双标度连续场） | 公开经典（LIF 脉冲传导等公开模型）+ 吸收改造 | **改①⑨**（同括注） |
| 三层知识库 / three-tier vault | 吸收改造（模块头注写明公开设计启发） | **改④⑫**：加一词定语「公开设计的应用改造」 |
| 记忆曲线 | 公开经典概念（遗忘曲线）应用 | 保留（"像人一样" 白话在位） |
| 哈希链 / SHA-256 / Merkle 链 | 公开经典 | 保留 |

### B. 记忆机制类

| 术语 | 血统 | 处置 |
|---|---|---|
| 双时态（事实流 / 事实图谱） | 概念公开经典（时态数据库）；「双时态」机制名为自研用法 | 保留（首现白话「何时发生 / 何时记录」在位） |
| 五层混合检索 | 自研集成机制名（内含公开经典件 BM25 / 向量 RRF，文内已点名=诚实） | 保留（机制段有词法/语义/激活度白话分层） |
| 保鲜回路（散列增量门 + 可恢复摄取队列 + 增量折入） | 自研命名 + 吸收改造（哲学研究假说改造，V4-b 交付） | **漏用未补**（受 6 处上限）；建议落点：记忆段或默认 ON 表 |
| 恒定注入（恒预算自我叙事注入） | 自研（V4-a） | **漏用未补**；建议落点：「性格与记忆」步或特性表 💛 行 |
| 逐字保留区（核心信念/底线绕过压缩） | 自研（保真分级，persona_layer 冻结语义） | **漏用未补**；建议落点：上下文段「压缩检查点」行 |
| 记忆注入 / 偏好学习 / 主动召回 / consolidation | 自研模块名 | 保留（默认表如实标 ON/OFF） |

### C. 治理 / 安全类

| 术语 | 血统 | 处置 |
|---|---|---|
| 三洋葱（E/S/A/M/O ⊥ L0-L5 + DSL 层）/ 双洋葱 | 自研 | 保留（白话「原则与权限是两把独立的锁」在位） |
| 命令审批族（批哪族算哪族：同族写法同权、跨族逐族批） | 自研（E-P2 审批族归一，governance 冻结词表） | **补用③⑮**（本次 3 词之一） |
| 族词表 / 冻结词表 | 自研（审批族冻结词表 + `owner / member / guest` 冻结角色词表） | 部分在文（zh「冻结角色词表」/ en "frozen role vocabulary" 保留）；「族词表」字面随补用③隐含，未单独占新词额度 |
| 动作绑定（批准绑定到这一次的动作摘要） | 自研（G2.5） | **漏用未补**；建议落点：「双手」段「批准只管这一次」括注（与 owner 口径「批一次就是这一次」同魂） |
| 守望漏斗（筛查→精查→复核→报告） | 自研命名 + 吸收改造（公开漏斗评估思路，V5-c 交付） | **漏用未补**（且默认 OFF，补用须带默认关注记） |
| 零丢失迁移（零迁移零丢失采用语义） | 自研（W3-1 交付，对抗断言在案） | **漏用未补**；建议落点：星群段存量原位收编 |
| 风险分级 / 审批状态机 / 三条铁律 | 自研 | 保留（承诺表引文一字未动） |
| 人格张力（矛盾双侧保留待裁，不静默覆盖） | 自研 | **漏用未补**；建议落点：星群段「自己的性格」旁 |

### D. 预算 / 写入类

| 术语 | 血统 | 处置 |
|---|---|---|
| 三维预算 → **三维烧钱闸**（tokens→tool_steps→cost_micros） | 自研（A 轴配额真接口「真执行的烧钱闸」） | **补用⑤⑬**（本次 3 词之一）：节首冠「三维烧钱闸」，固定维序原样保留 |
| **写总闸**（file_write 唯一写总闸） | 自研（宪法级开关语义） | **补用⑥⑭**（本次 3 词之一）：en 首现补 "sole write master gate 写总闸"；zh「总闸」升为「写总闸」 |
| 四拍受控写（计划→改→验证→落，全有或全无） | 自研（worktree_apply 交付；owner 口径「四拍原子写」≈ 落拍持久档原子写） | **漏用未补**（受 6 处上限）；建议落点：默认 OFF 表 `worktree_*` 括注或「双手」段原子写行（须带 opt-in 注记） |
| 原子写两档 / 读前观测门禁 / 沙箱升级阶梯 / 五段执行流水线 / 压缩检查点 / 溢出落盘取回 / 溢出自愈 / 审批闭合词表 | 吸收改造（机制吸收战役逐项在案 = absorption-ledger） | 保留——中性自含命名本就是通俗转述，且「机制来龙去脉 → 吸收台账」总指针在位 |

### E. 多户 / 关怀 / 人格 / 纪律类

| 术语 | 血统 | 处置 |
|---|---|---|
| 星群 | 自研 | 保留（白话在位） |
| 关怀双引擎 / 心跳宿主 | 自研（V3-a / 心跳宿主接线） | 已用，保留 |
| 随手记 / 审批卡 | 自研 | 已用，保留 |
| 十七节自省合同（自述节合同） | 自研；**「十七节」是历史口径**——节合同是「形状冻结的演进」，`self_status.rs` 现值已演进至十九节 | **不补**（数字会过时）；若补用应写「自述节合同（形状冻结的演进）」，或按历史口径带日期 |
| 0 装 PASS | 自研纪律名 | README 已用白话「不假装 / No pretending」承载（footer、宣言）——保留不动 |
| 「命名即承诺」 | 自研宪章条款 | **漏用未补**；建议落点：诚实边界段首或 Verify 段 |
| 三条铁律 / 六锚 / 四公理 /「日志是唯一权威」等 | 自研哲学本体（LOCKED） | **0 触碰** |
| reflexion / Judge / Council / 9 器官 | 自研模块名（reflexion 与公开方法同名，实现含自研件） | 保留（默认表如实标 ON/OFF） |
| Kani / TLA/TLC / Noise 握手 / 洋葱路由 / BM25 / RRF / OWASP ASI-01 / Job Object / AppContainer / rlimits | 公开经典（工具、协议、方法、平台机制名） | 保留——如实指称外部之物，非自家发明宣称 |

## 三、镊子改动清单（逐处：原文 → 改文；每处 ≤2 行）

### README.zh-CN-draft-pending.md（8 处）

| # | 位置 | 原文 | 改文 |
|---|---|---|---|
| ① | 行 188（A-12 记忆数学） | 随船的数学吸收模块（Betti 洞探测、Kuramoto 相锁、Chronicle 结晶、DualScaled 场）是**简化 / 实验级** | 随船的数学吸收模块（**公开经典模型的应用改造**：Betti 洞探测、Kuramoto 相锁、Chronicle 结晶、DualScaled 场）是**简化 / 实验级** |
| ② | 行 188（同句） | ；MGS 残差金字塔是完整实现。细节在各 crate 模块文档里。 | ；正交残差分层提炼记忆（MGS 残差金字塔，**公开数值方法的应用改造**）是完整实现。细节在各 crate 模块文档里。 |
| ③ | 行 256（诚实层级注记） | （见上文[诚实的边线](#boundaries)）；MGS 残差金字塔是完整实现。 | （见上文[诚实的边线](#boundaries)）；MGS 残差金字塔（公开数值方法的应用改造）是完整实现。 |
| ④ | 行 290（crate 地图） | 记忆曲线、三层知识库； | 记忆曲线、三层知识库（公开设计的应用改造）； |
| ⑤ | 行 228（三维预算节） | 固定维序 `tokens → tool_steps → cost_micros`，按会话记账、四个检查点查账： | **三维烧钱闸**（`tokens → tool_steps → cost_micros` 固定维序）按会话记账、四个检查点查账： |
| ⑥ | 行 197（默认 OFF 表） | （由 `APEIRETH_ENABLE_FILE_WRITE=1` 总闸把关） | （由**写总闸** `APEIRETH_ENABLE_FILE_WRITE=1` 把关） |
| ⑦ | 行 289（governance 行） | 双洋葱守门（原则 ⊥ 权限）、OWASP ASI-01 卫生、PII 脱敏。 | 双洋葱守门（原则 ⊥ 权限）、**命令审批族**（批哪族算哪族：该族所有写法同权，跨族仍逐族批）、OWASP ASI-01 卫生、PII 脱敏。 |
| ⑧ | 行 48 后（草稿头批注） | > （每条落点带（小巧思 n）标记）。 | > （每条落点带（小巧思 n）标记）。⏎> ⏎> **术语血统镊子批（2026-10-18）**：只动术语——吸收改造类加诚实定语、自研术语补用 3 处（详见术语血统对照表）；审计修正 / 门面批 / 小巧思成果零删改。 |

### README-draft-pending.md（8 处）

| # | 位置 | 原文 | 改文 |
|---|---|---|---|
| ⑨ | 行 189（Memory mathematics） | (Betti hole detector, Kuramoto resonance, Chronicle crystallizer, DualScaled field) are **simplified / experimental tier** | (**adapted from classic public models**: Betti hole detector, Kuramoto resonance, Chronicle crystallizer, DualScaled field) are **simplified / experimental tier** |
| ⑩ | 行 189（同句） | rather than a Merkle chain; the MGS residual pyramid is the fully-fledged one. Details live | rather than a Merkle chain; the layered orthogonal-residual memory distiller (the MGS residual pyramid, **an application of classic public numerical methods**) is the fully-fledged one. Details live |
| ⑪ | 行 257（Honest tier note） | (see [Honest edges](#boundaries) above); the MGS residual pyramid is the fully-fledged one. | (see [Honest edges](#boundaries) above); the MGS residual pyramid (an application of classic public numerical methods) is the fully-fledged one. |
| ⑫ | 行 291（crate map） | memory curve, three-tier vault; math absorption modules | memory curve, three-tier vault (**adapted from public designs**); math absorption modules |
| ⑬ | 行 229（three-dimensional budget） | Fixed dimension order `tokens → tool_steps → cost_micros`, ledger-kept per session | The three-dimensional burn gate (**三维烧钱闸**): fixed dimension order `tokens → tool_steps → cost_micros`, ledger-kept per session |
| ⑭ | 行 198（OFF table） | (gated by `APEIRETH_ENABLE_FILE_WRITE=1`) | (gated by the sole write master gate **写总闸**, `APEIRETH_ENABLE_FILE_WRITE=1`) |
| ⑮ | 行 290（governance row） | Double-onion gatekeeper (principles ⊥ permissions), OWASP ASI-01 hygiene, PII redaction. | Double-onion gatekeeper (principles ⊥ permissions), **command approval families** (approve a family and every spelling of it counts; other families stay unapproved), OWASP ASI-01 hygiene, PII redaction. |
| ⑯ | 行 49 后（草稿头批注） | > （每条落点带（小巧思 n）标记）。 | 同 ⑧（两文件同文批注） |

## 四、自研术语漏用候选（受「全文新增 ≤6 处」上限未补，建议下批按此序补）

1. **四拍受控写**（计划→改→验证→落）→ 默认 OFF 表 `worktree_*` 括注（须带 opt-in 注记，勿当默认现状）；
2. **保鲜回路** → 记忆段或默认 ON 表（散列增量门+可恢复摄取队列+增量折入的白话随行）；
3. **恒定注入** → 「性格与记忆」步或特性表 💛 行（每回合带一小段恒预算自我叙事）；
4. **人格张力** → 星群段「自己的性格」旁（矛盾双侧保留待裁，不静默改口）；
5. **动作绑定** → 「双手」段「批准只管这一次」括注（批准绑定这次的动作摘要）；
6. **守望漏斗** → 安全面（筛查→精查→复核→报告；默认 OFF，补用须带注记）；
7. **零丢失迁移** → 星群段存量原位收编（零迁移零丢失）；
8. **「命名即承诺」** → 诚实边界段首（名字 = 它的全部权力）；
9. **逐字保留区** → 上下文段「压缩检查点」行（核心信念/底线绕过压缩）。

## 五、口径注记与批内发现

1. **「十七节自省合同」= 历史口径**：自述节合同是「形状冻结的演进」，`crates/capabilities/tools/src/self_status.rs` 节合同注释已从七节逐节演进至**十九节**（十七节=创作节、十八节=文档操作节、十九节=在场节）；README 若补用此术语，应写「自述节合同（形状冻结的演进）」而非冻结「十七节」数字（历史不追溯改写，带日期口径另记）。
2. **「我们发明 / 独创」口气：全文零命中**——唯一近似句为 `missing prices are never invented`（反向诚实表述），无需改「我们的实现」。
3. **中性表述扫描**：本批 16 处新增行零命中。全场 `scripts/check-neutral-terms.ps1` 现报 FAIL 2 命中，**均在他线在途未跟踪文件 `crates/engine/image/NOTICE.md`**（2026-10-18 11:35 新增、许可署名类第三方标识，非本批触碰面）——按「他人在途红标红不代改」报备归口许可搬运线，本批不代改。
4. **测试数字**：Tests 徽章 2026-10-16 口径与 test-baseline §1 现行基线一致（该文件明文「对外引用保持上表」），本批零触碰、不代改真源。
5. **成果零删改自检**：（审计修 A-03…A-15）逐条注记、门面重排批注记、小巧思 ①②③④⑤⑧⑫ 落点、承诺+证据同行表引文、六锚/四公理（LOCKED）全部原样；结构（标题/表格/锚点/文气）零改动。
6. **术语密度**：自研术语新增 6 处（3 词 × 2 文件，用满上限）且均随半句白话；吸收改造类 8 处改为白话定语（通俗转述向）；无新增裸术语——净密度大致持平、白话占比上升。
