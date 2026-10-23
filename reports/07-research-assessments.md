# 册 07 — 调研评估卷宗（12 篇合册）

> **一句话定位**：TP30 待评估清单的**逐件评估件**（10 篇候选，每篇统一「机制 / 对照 / 吸收建议 / 0 装 PASS 标注」四段式）+ 内置应用面参考套件拆解 + 记忆 donor 语义映射；一篇篇都写着「真用：否（未实测）」「未调研不写结论」——**调研件本身遵守与产品面同一套 0 装纪律**。
>
> **来源篇数**：12（分配表 §07，逐篇唯一归册）｜**原篇日期跨度**：2026-09-06 ~ 2026-10-11
> **合册生成日**：2026-10-23｜**复核口径（现行真源）**：[AGENTS.md](../AGENTS.md) §1/§2/§4.5 ／ [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) ／ [capabilities-matrix](../docs/03-reference/capabilities-matrix.md) §0.2 ／ [claims.yaml](../docs/status/claims.yaml)
>
> **用词与引注说明**：本册各篇原标题含研究对象专名，属仓内**研究引注豁免**（依 [embedded-app-suite-research-2026-10-11.md](embedded-app-suite-research-2026-10-11.md) 文末「引注豁免表」与施工纪律 §三.4 豁免通道）；本册正文分析一律用中性类别名，**不新增**任何专名引用。

## 目录

| # | 原标题 | 原日期 | 类型 |
|---|---|---|---|
| 1 | Agent-S 评估 | 未标日期 | TP30 候选 |
| 2 | airllm 评估 | 未标日期 | TP30 候选 |
| 3 | exo 评估 | 未标日期 | TP30 候选 |
| 4 | GitNexus 评估（**实测评估**） | 未标日期 | TP30 候选（已实测） |
| 5 | maigret 评估 | 未标日期 | TP30 候选 |
| 6 | OCR 类评估（Tesseract / PaddleOCR / EasyOCR） | 未标日期 | TP30 候选 |
| 7 | OpenSquilla 评估 | 未标日期 | TP30 候选 |
| 8 | project-nomad 评估 | 未标日期 | TP30 候选 |
| 9 | Scrapling 评估 | 未标日期 | TP30 候选 |
| 10 | taipy 评估 | 未标日期 | TP30 候选 |
| 11 | 基地内置应用面调研——上游参考套件逐件拆解 | 2026-10-11 | 架构调研 |
| 12 | Yanshu / Memory donor mapping — vNext | 2026-09-06 | 语义映射 |

---

### Agent-S 评估 (原篇未标日期)

**来源**：`reports/tp30/agent-s-assessment.md`（约 45 行）｜TP30 待评估清单成员。
**结论/要点**
- **机制**：电脑使用 Agent（computer-use agent），AI 操作 GUI 自动化（鼠标/键盘/截图）；解决传统 RPA 贵且僵化 → AI 驱动 GUI 自动化。关键技术 4 项：视觉理解（截图→元素识别）/ 动作执行（PyAutoGUI / xdotool）/ 任务分解 + 多步规划 / 安全边界（白名单 + 危险操作拦截）。
- **对照**：相似能力 = `apeireth-tool-browser`（浏览器自动化）/ `apeireth-action`（行动器官）/ `apeireth-tool-approval`（R173 ApprovalBridge）/ `OpenHands`（已 clone 在 `research/source/OpenHands/`）/ `morphic`（已 clone）/ `hermes-agent-rs`（已 clone，Rust agent）。差异化 = 通用 GUI vs 浏览器专属（场景略错位）、Python vs Rust、安全敏感（GUI 全权）而 `apeireth-guard`（R173 Privacy Guard）有边界。
- **吸收建议**：P0 不做 / P1 待评估 / P2 观察；**不做（重复/价值低）**：当前主路径是「伴侣 + 工具调用」，GUI 自动化是边缘场景。
- **0 装 PASS 标注**：**真用：否（未实测 Agent-S 本体）**；**源：未下载实测**——`research/source/` 无源码；本评估基于 GitHub README + 同类项目（已 clone 的三项）对照推理；**未调研不写结论**——视觉理解算法 / 任务分解模型 / 安全边界实现均为推理判断，如需落地建议**必须先实测 + 在 Rust 端做 POC**；**同类参考优先级：已 clone 的更成熟者 > 本候选**，本候选降级为观察项。
**现状复核（2026-10-23）**：**已过时（对照对象全部迁出）**——`apeireth-tool-browser` / `apeireth-action` / `apeireth-tool-approval` / `apeireth-guard` 现位于 [legacy/donor/apeireth-tool-browser](../legacy/donor/apeireth-tool-browser)、[apeireth-action](../legacy/donor/apeireth-action)、[apeireth-tool-approval](../legacy/donor/apeireth-tool-approval)、[apeireth-guard](../legacy/donor/apeireth-guard)；按 [legacy/README.md](../legacy/README.md)「不注册能力」。**已过时（现行能力面）**——现行浏览器/工具面是 [crates/capabilities/tools](../crates/capabilities/tools) 的 `tool.repo_map` / `tool.apply_patch` / `tool.git_write` / `tool.guardrail` / `tool.mcp` / `tool.stealth_crawler` 与 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0.2 的默认开关表（只读面默认 ON；shell / fetch / MCP 默认 OFF）。**仍成立（方法论，本册共性）**：① **「真用：否 / 源：未下载实测 / 未调研不写结论」三段式 0 装标注**——与现行 [AGENTS.md](../AGENTS.md) §1 零装诚实纪律逐字一致，是本册 9 篇 TP30 评估件的共同骨架；② **「P0 不做 / P1 评估后做 / P2 长期调研 / 不做（重复·价值低）」四级动作词表**——与现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) 的 ✅/🔧/📋 三态 + 「处置建议」写法同构；③ **「场景错位就不吸收」的判断纪律**——避免为凑数引入不相关能力；④ **「同类参考优先级：已在库的更成熟者优先」**——与现行取材纪律（[AGENTS.md](../AGENTS.md) §4.5「先测活再取材」）同向。**未核实**：本候选与其余 9 件的实际现况（**未实测，不作断言**）。
**归属链接**：../legacy/donor/apeireth-tool-browser、../legacy/donor/apeireth-action、../legacy/donor/apeireth-tool-approval、../legacy/donor/apeireth-guard、../crates/capabilities/tools、../docs/03-reference/capabilities-matrix.md、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md、../legacy/README.md

### airllm 评估 (原篇未标日期)

**来源**：`reports/tp30/airllm-assessment.md`（约 35 行）｜TP30 待评估清单成员。
**结论/要点**
- **机制**：家用单 GPU 跑 70B 模型（**层交换 layer swapping** 让单卡跑 70B）；解决 70B 需多卡 → 单卡家用设备跑不动。关键技术 4 项：层交换（逐层 swap 到 GPU/CPU 的前向/后向推理）/ 模型并行层调度 / 内存映射 + 懒加载 / HuggingFace transformers 集成。
- **对照**：相似能力 = `apeireth-local-inference`（本地推理，**待评估是否有占位**）/ `apeireth-provider`（5 Provider 真合并）；差异化 = 解决「单卡跑大模型」而现行策略是「云端 + 小模型本地」，Python vs Rust。
- **可借鉴**：**层交换思想**（在 Rust 端用 `candle` 或 `tch-rs` 重写核心调度）／**内存映射**（参考 mmap 模型加载，加速冷启动）。
- **吸收建议**：P0 不做；P1 需先实测源码 + 在 Rust 端 POC；P2 列入观察项，每季度回看 candle-rs 进展；**不做（重复/价值低）**——与同类家用推理候选重复，后者更适合多设备场景，本候选更适合单卡极限场景；**当前不主推家用推理**。
- **0 装 PASS 标注**：**真用：否（未实测）**；**源：未下载实测**（`research/source/` 无源码；基于 GitHub README 公开信息）；**未调研不写结论**——层交换算法具体实现 / 性能数据 / 与 candle 兼容性均为推理判断。
**现状复核（2026-10-23）**：**已过时（对照对象已不存在）**——原文对照的 `apeireth-local-inference` 在现行树**不存在**（无同名 crate；本地推理相关现行面是 [crates/engine/provider](../crates/engine/provider) 的 provider 路由 + `rt.provider_router` claim）；`apeireth-provider` 现行是 [crates/engine/provider](../crates/engine/provider)。**已过时（模型面口径）**——现行模型面口径见 [claims.yaml](../docs/status/claims.yaml) `rt.provider_router`（跨 provider fallback + 健康态，PRODUCTION WIRED，默认 ON）与 `core.cost_router`（成本分级路由四档，PRODUCTION WIRED，**默认 OFF**，`APEIRETH_COST_ROUTING`），本地大模型推理**无现行登记面**。**仍成立（方法论）**：① **「层交换 / mmap 加载」作为可移植语义而非代码的判断**（用 `candle` / `tch-rs` 重写而非 fork Python）——与现行 [legacy/README.md](../legacy/README.md)「移植语义，不移植架构」及 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D19（新增依赖闭包 84–85 包 ⇒ 暂不立项）**完全同源**：先算依赖闭包再决定立项；② **「P2 观察项，每季度回看」**的长期观察机制；③ 0 装三段式标注。**未核实**：Rust 端 `candle` / `tch-rs` 的现行依赖面是否存在——**未核实**（现行 [Cargo.toml](../Cargo.toml) members 与依赖中未见，但未逐项比对），**不作断言**。
**归属链接**：../crates/engine/provider、../docs/status/claims.yaml、../docs/04-internal/backend-enhancement/00-backlog.md、../legacy/README.md、../Cargo.toml、../AGENTS.md

### exo 评估 (原篇未标日期)

**来源**：`reports/tp30/exo-assessment.md`（约 35 行）｜TP30 待评估清单成员。
**结论/要点**
- **机制**：家用 GPU 集群推理引擎，把多台家用设备的 GPU/NPU 聚合为统一推理池；解决单设备显存上限 → 多设备水平扩展 → 在家跑 70B+。关键技术：模型分区（tensor/pipeline parallelism）+ 设备发现 + RDMA/NVLink/USB-net 自适应 + MLX 后端。
- **对照**：相似能力 = `apeireth-local-inference`（待评估是否有占位）/ `apeireth-provider`（5 Provider 真合并，模型路由）；差异化 = 设备异构聚合 vs 云端/本地模型路由；强调 Apple Silicon（MLX）而现行 ROCm/CUDA 为主。
- **可借鉴**：**模型分区策略**（若后期想在家跑 70B，可参考 pipeline parallelism，**不直接 fork，因为是 Python**）／**设备发现协议**（mDNS / 手动 IP 列表的轻量级发现，可借鉴到多机调度）。
- **吸收建议**：**P0 立即做：不动**（Python 项目与 Rust 主力栈不兼容）；**P1 评估后做**——仅当「想在家跑 70B+ 且有多设备条件」时，用 `candle` 或 `cudarc` 在 Rust 端重写核心调度（设备发现 + 分区策略），或把 MLX 后端思路 fork 到 Rust（社区已有 `candle-metal` 实验）；P2 观察项，每季度回看；**不做（重复/价值低）**——现行主力是「云端 API + 小模型本地」，家用集群推理需求未触发。
- **0 装 PASS 标注**：**真用：否（未实测）**；**源：未下载实测**（基于 GitHub README 公开信息）；**未调研不写结论**——MLX 后端实现细节 / 分区算法具体步骤 / 设备发现协议均为推理判断。
**现状复核（2026-10-23）**：**已过时（对照对象与语境）**——`apeireth-local-inference` 在现行树不存在；`apeireth-provider` 现为 [crates/engine/provider](../crates/engine/provider)。**已过时（推理栈语境）**——原文称「现行 ROCm/CUDA 为主」；现行能力面见 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §1：端侧推理在**系统级能力缺口台账**（SG-09 端侧推理）登记为**待排期缺口**，非已交付能力。**仍成立（方法论）**：① **「触发条件未满足 → P0 不动」**——即**需求未触发就不立项**，这是本册最健康的一条判断纪律，与现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D19（技术可行但暂不立项，理由写明依赖闭包冲突）同构；② **「用 Rust 生态重写语义而非 fork Python」的取材纪律**；③ **「先确认能力缺口是否真存在（『待评估是否有占位』）」**——现行对应机制是 `scripts/arch_guard.py --strict` 与 `docs_truth.py --verify` 的证据核对。**未核实**：端侧推理（SG-09）在 2026-10-22 之后的排期状态——**未核实**。
**归属链接**：../crates/engine/provider、../docs/03-reference/capabilities-matrix.md、../docs/04-internal/backend-enhancement/00-backlog.md、../scripts/arch_guard.py、../scripts/docs_truth.py、../legacy/README.md、../AGENTS.md

### GitNexus 评估（**实测评估**，源码在 research/source/GitNexus/） (原篇未标日期)

**来源**：`reports/tp30/gitnexus-assessment.md`（约 110 行）｜TP30 清单中**唯一实测**的一件。
**结论/要点**
- **机制**：把任意 codebase 索引成知识图谱（依赖/调用链/集群/执行流），通过 MCP tools 暴露给 AI agent；解决「AI agent 写代码时看不到依赖、改一处破一片」。
- **关键技术（实测来源 `research/source/GitNexus/ARCHITECTURE.md` + `gitnexus/src/`）7 组件表**：Ingestion pipeline（`core/run-analyze.ts` + `pipeline.ts`，**15-phase DAG**，构建 `KnowledgeGraph`）／Graph store（`core/lbug/` + `ladybug-db`，图数据库类 Neo4j，持久化到 `.gitnexus/`）／Query layer（`core/search/`：`bm25-index.ts` / `hybrid-search.ts` / `fts-indexes.ts`，BM25 + 向量混合检索 + Cypher ad-hoc）／MCP server（`mcp/server.ts` + `tools.ts`，stdio + HTTP 双 transport）／Embeddings（`core/embeddings/embedder.ts` + `hf-env.ts`）／Wiki generator（`core/wiki/generator.ts` + `graph-queries.ts`，自动生成 Mermaid 图代码 wiki）／Staleness（`core/git-staleness.ts` + `index-freshness.ts`，增量更新检测 lastCommit 对比）／Web UI（`gitnexus-web/`，Vite + React，只读 thin client）。
- **17 个 MCP tools 实测清单**：`list_repos` / `query` / `cypher` / `context` / `impact` / `detect_changes` / `rename` / `api_impact` / `trace` / `route_map` / `tool_map` / `shape_check` / `explain` / `pdg_query` / `group_list` / `group_sync`；**关键差异化**：`query`/`context`/`impact` 支持 `repo: "@<group>"` 的 group-aware 跨仓库操作。
- **0 装 PASS 标注（本册最诚实的一件）**：**真用：部分是（仅借鉴模式，不 fork 代码）**；**源：已实测**（源码 + `ARCHITECTURE.md` + `src/` 实读）；**实测深度**：ARCHITECTURE.md 全文 + src/ 目录结构 + 关键文件（`hybrid-search.ts` / `mcp/tools.ts` / `core/wiki/generator.ts` / `cjk-segmentation.ts`）**文件名级确认**；**未实测**：实际代码**行级** review（17 个 MCP tools 的具体参数 schema、`embedder.ts` 的具体 model 选型）；**风险**：License = **PolyForm Noncommercial（非商业）**，如借鉴代码片段需注意 license 边界——**建议仅借鉴算法思路，不复制代码段**。
**现状复核（2026-10-23）**：**部分成立（结论与许可边界仍可引用，源码检出不在现行树）**——① `research/source/GitNexus/` **当前不存在**（`research/` 区在 [evidence-retention-policy.md](../docs/04-internal/evidence-retention-policy.md) 语境下属仓库外证据面），故「已实测」的证据面**不可就地复核**；② **「仅借鉴模式，不 fork 代码」+「非商业许可 → 只借鉴思路不复制代码段」**的处置结论**仍成立**，且与现行 [AGENTS.md](../AGENTS.md) §2 中性表述 + §4.5 取材通道「Rust crate 源码 → crates.io tarball、通用网页 → web_fetch、GitHub 内容 → 节点池」一致。**已过时（对照对象）**——原文对照的 `apeireth-tool-codesearch` / `apeireth-repo-tools` 等现位于 [legacy/donor/apeireth-tool-codesearch](../legacy/donor/apeireth-tool-codesearch) / [legacy/donor/apeireth-repo-tools](../legacy/donor/apeireth-repo-tools)。**仍成立（方法论，本册最值得抄的一条）**：① **「实测深度分级：文件名级确认 ≠ 行级 review，未实测项逐条列出」**——这是现行 [claims.yaml](../docs/status/claims.yaml) `sources` / `tests` / `verify_cmd` / `notes` 四段结构的原型；② **「真用：部分是」这一中间档**（既非「是」也非「否」）——现行四级口径中的 **IMPLEMENTED**（库级完备）与 **PRODUCTION WIRED**（生产接线）之间正是这个区间；③ **「许可边界先判再谈借鉴」**；④ 15-phase DAG ingestion、BM25+向量混合检索、staleness 增量检测三套语义可独立重建。**未核实**：17 个 MCP tools 的参数 schema 与 embeddings 模型选型——原文已自述未实测，本轮同样**未核实**。
**归属链接**：../docs/04-internal/evidence-retention-policy.md、../docs/status/claims.yaml、../legacy/donor/apeireth-tool-codesearch、../legacy/donor/apeireth-repo-tools、../AGENTS.md、../docs/04-internal/backend-enhancement/00-backlog.md

### maigret 评估 (原篇未标日期)

**来源**：`reports/tp30/maigret-assessment.md`（约 35 行）｜TP30 待评估清单成员。
**结论/要点**
- **机制**：用户画像 OSINT，给定 username 在 **2500+ 网站**上检索账号存在；解决单一站点手动查 → 跨站点统一用户名占用扫描 → 用户画像拼图。关键技术 4 项：2500+ 站点 site adapter（每站点独立 URL 模板 + 抓取策略）/ 启发式 username 变体生成 / 本地报告 HTML/PDF 输出 / 反指纹规避（每站点独立 cookie/header）。
- **对照**：相似能力 = `apeireth-tool-fetch`（统一 fetch 引擎）/ `apeireth-tool-browser`（浏览器自动化）/ `apeireth-tool-search`（R145 VSearch + aggregate）；**缺失：APEIRETH 无 OSINT 套件**。差异化 = 解决「OSINT」而现行套件是「AI 伴侣基础设施」（场景错位）、Python CLI vs Rust。
- **可借鉴**：**跨站点查询编排模式**（site adapter 抽象 = 每站点一个 adapter + 统一调度，可参考为通用 pattern）／**隐私边界**——涉及隐私/合规问题，现行 `apeireth-guard`（R173 Privacy Guard）有边界，OSINT 集成**需要 review**。
- **吸收建议**：P0 不做；P1 需实测 + 合规 review；P2 观察项；**不做（重复/价值低）**——当前主人场景未触发 OSINT 需求。
- **0 装 PASS 标注**：**真用：否（未实测）**；**源：未下载实测**（基于 GitHub README 公开信息）；**未调研不写结论**——2500+ 站点 adapter 维护现状 / 反指纹技术细节 / 合规边界均为推理判断。
**现状复核（2026-10-23）**：**已过时（对照对象已迁出 + 需求未触发）**——`apeireth-tool-fetch` / `apeireth-tool-browser` / `apeireth-tool-search` / `apeireth-guard` 现位于 [legacy/donor/apeireth-tool-fetch](../legacy/donor/apeireth-tool-fetch) 等；现行 fetch 面是 [crates/capabilities/tools](../crates/capabilities/tools) + [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0.2 的 `APEIRETH_ENABLE_FETCH=1`（**默认 OFF**）。**仍成立（方法论，本篇的价值在合规判断）**：① **「涉及隐私/合规的集成必须先过合规 review，再谈技术」**——与现行 [AGENTS.md](../AGENTS.md) §2 中性表述、§4.5 取材纪律，以及现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) **E 安全纵深**（E-P2 审批族归一、E-P3 沙箱对象级判定在飞）配套；② **「需求未触发 → 不做」**；③ **「adapter 抽象可作通用 pattern 借鉴」**——与现行工具面 `CapabilityRegistry` / `BehaviorRegistry` 的注册-投影范式同构。**未核实**：合规 review 在现行是否已有对应面——**未核实**。
**归属链接**：../legacy/donor/apeireth-tool-fetch、../legacy/donor/apeireth-tool-browser、../legacy/donor/apeireth-tool-search、../legacy/donor/apeireth-guard、../crates/capabilities/tools、../docs/03-reference/capabilities-matrix.md、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md

### OCR 类评估（Tesseract / PaddleOCR / EasyOCR） (原篇未标日期)

**来源**：`reports/tp30/ocr-assessment.md`（约 55 行）｜TP30 待评估清单成员。
**结论/要点**
- **机制**：图片/扫描件 → 文字识别，含版面分析 + 多语言 + 表格识别；解决图像中的文字无法直接被 LLM 消费 → OCR 转文本后才能检索/摘要/翻译。
- **三库对比表（引擎 / 语言 / 优势 / 劣势）**：Tesseract（LSTM 传统 OCR，100+ 语言，**离线、纯 CPU、轻量**，劣势复杂版面/手写识别差）／PaddleOCR（PP-OCR Transformer，80+ 语言，中文识别强、版面分析、表格识别，劣势 Python + PaddlePaddle 重）／EasyOCR（CRAFT + CRNN，80+ 语言，易用 API，劣势性能慢于 PaddleOCR）。
- **对照**：相似能力 = `apeireth-tool-image-process`（图像处理工具套件）/ `apeireth-tool-approval`（含审批流，可处理 OCR 后的人工校对）；**缺失：APEIRETH 无 OCR 套件**；差异化 = 三库覆盖「轻量/中文/易用」三角可按场景挑选，而现行若需 OCR 只能调外部 API（**不满足本地/隐私场景**）。
- **可借鉴**：**本地 OCR 套件骨架**（在 `apeireth-tool-image-process` 加 `OcrEngine` trait，三后端，主用 Tesseract——Rust 端有 `leptess` crate 绑定）／**隐私优先**（本地 OCR 满足「数据不出本地」原则）。
- **吸收建议**：P0 需先 clone 实测再重写建议；P1 待评估；P2 观察项；**不做**：EasyOCR 单独集成（PaddleOCR 已覆盖其优势场景）。
- **0 装 PASS 标注**：**真用：否（未实测）**；**源：未下载实测**（基于各项目 GitHub README + `leptess` crate 文档推理）；**未调研不写结论**——实际准确率 / 性能 benchmark / 中文版识别效果均为推理判断；**强烈建议**先 clone 三个库到 `research/source/ocr/` 实测后重写 P0 建议（特别是 `leptess` API 是否真覆盖 Tesseract 全部功能）；**注：与 `apeireth-tool-image-process` 实际接口契约冲突时，以其现状为准**。
**现状复核（2026-10-23）**：**已过时（对照对象已迁出 + 图像面已换代）**——`apeireth-tool-image-process` 现位于 [legacy/donor/apeireth-tool-image-process](../legacy/donor/apeireth-tool-image-process)；**现行图像面已完全换代**——2026-10-18 许可搬运线第一批把图像核心 vendored 家族 **10 crate** + 接口层 1 crate 搬入主仓（现 [crates/engine/image/](../crates/engine/image) 与 `apeireth-image-core` 接口层，见 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D17 与 [AGENTS.md](../AGENTS.md) §3 crate 口径），现行 `image_ops` 工具面闭合操作词表 = `resize/crop/rotate/flip/format_convert/brightness/contrast/grayscale/blur/sharpen/watermark_text/layers_composite/psd_read_info`（见 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0.2），**不含 OCR**——故「缺失：APEIRETH 无 OCR 套件」**在现行仍成立**。**仍成立（方法论）**：① **「与现有工具面契约冲突时以现状为准」的越权防护**（避免调研件擅自改动他线工作）；② **「本地优先 = 隐私边界」**——与现行 `sec.egress_gate`（模型出网闸，默认 OFF，检出 PII→真值换不可逆占位，闸不可用→拒绝即帧 fail-closed）的方向一致；③ **「强建议先实测再重写建议」的取证纪律**；④ **「不单独集成能力重叠项」**（EasyOCR 场景被 PaddleOCR 覆盖）。**未核实**：`leptess` crate 与 OCR 后端在现行依赖面是否存在——**未核实**，不作断言。
**归属链接**：../crates/engine/image、../legacy/donor/apeireth-tool-image-process、../docs/03-reference/capabilities-matrix.md、../docs/04-internal/backend-enhancement/00-backlog.md、../docs/status/claims.yaml、../AGENTS.md

### OpenSquilla 评估 (原篇未标日期)

**来源**：`reports/tp30/opensquilla-assessment.md`（约 40 行）｜TP30 待评估清单成员。
**结论/要点**
- **机制**：分布式多 Agent 协作框架，强调 Agent 间消息路由 / 任务分发 / 状态共享；解决单 Agent 能力上限 → 多 Agent 分工 → 复杂任务分解。关键技术：消息总线 + Agent 注册中心 + 任务调度器 + 共享 context store。
- **对照**：相似能力 = `apeireth-bus`（消息总线基础设施）/ `apeireth-agent`（单 Agent 调度）/ `apeireth-runtime`（含 **7 模块编排**：heartbeat / task / bus / arbitration / search / group_chat / emotion）/ `apeireth-council`（多 Agent 治理）/ `MetaGPT`（已 clone 在 `research/source/MetaGPT/`，SOP 化多 Agent 协作参考）。差异化 = 强调「分布式」（跨节点）而现行暂无跨节点 Agent 编排（仅单进程）；任务分发走 message passing 而现行走 bus + arbitration。
- **可借鉴**：Agent 注册中心模式（按 capability 统一检索）/ 任务分解 + 回滚（TxId + rollback 机制可参考 sub-task 回退设计）。
- **吸收建议**：**P0 立即做：不动**（解决的是「跨节点分布式」，现行战场是「单进程高内聚」，**错位**）；P1/P2 观察；**不做（重复/价值低）**——现行已有 runtime 的 7 模块端到端编排 + council 多 Agent 治理，本候选不补核心缺位。
- **0 装 PASS 标注**：**真用：否（未实测）**；**源：未下载实测**（基于 GitHub README 公开信息 + 同类项目对照推理）；**未调研不写结论**——「差异化优势」「可借鉴」「不做」**均为推理判断，非实测结论**，如需落地建议必须先 clone 实测 README + src/ 后重写本评估。
**现状复核（2026-10-23）**：**已过时（对照对象全部迁出 + 分布式面已换代）**——`apeireth-bus` / `apeireth-agent` / `apeireth-council` 现位于 [legacy/donor/apeireth-bus](../legacy/donor/apeireth-bus) / [apeireth-agent](../legacy/donor/apeireth-agent) / [apeireth-council](../legacy/donor/apeireth-council)；`apeireth-runtime` 现行是 [crates/engine/runtime](../crates/engine/runtime)。**已过时（多 Agent 治理面）**——现行治理与总线口径见 [../docs/adr/0009-r1b-nervous-bus.md](../docs/adr/0009-r1b-nervous-bus.md)（R1b 事件总线）与 [claims.yaml](../docs/status/claims.yaml) `gov.chain_skeleton` / `gov.chain_hook_adapter`（R8 六段治理骨架 + 生产单入口）；**跨节点 Agent 编排仍是未闭合缺口**（现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) F 行「TLS 真接线（现接口桩）、域面余面、P4 部署形态待」；D22 SG-03/04/05/06 等系统级缺口待排期）。**仍成立（方法论）**：① **「战场错位 → P0 不动」**的判断纪律（本篇与 exo、taipy、OpenSquilla 三件同款）——与现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D19「技术可行但暂不立项」的处置法同构；② **「能力注册中心按 capability 检索」的抽象**——与现行 `CapabilityRegistry` / `BehaviorRegistry` 投影面（[capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0）同构；③ **「可借鉴项必须先实测再落地」**；④ **「已有能力不重复建设」的显式核对**（7 模块编排 + council 双保险）。**未核实**：跨节点编排缺口的当前排期——**未核实**。
**归属链接**：../crates/engine/runtime、../docs/adr/0009-r1b-nervous-bus.md、../docs/status/claims.yaml、../docs/03-reference/capabilities-matrix.md、../docs/04-internal/backend-enhancement/00-backlog.md、../legacy/donor/apeireth-bus、../legacy/donor/apeireth-agent、../legacy/donor/apeireth-council、../AGENTS.md

### project-nomad 评估 (原篇未标日期)

**来源**：`reports/tp30/project-nomad-assessment.md`（约 40 行）｜TP30 待评估清单成员。
**结论/要点**
- **机制**：AI 数字游民助手，个人助理场景的端到端 AI 伴侣；解决数字游民跨时区 / 多任务 / 低预算的生活管理需求。关键技术：行程规划 + 预算管理 + 跨时区通信协调 + 本地化服务整合。
- **对照**：相似能力 = `apeireth-companion`（伙伴端点，任何 OpenAI 兼容前端接入主链路）/ `apeireth-action`（行动器官）/ `apeireth-motivation`（动机/价值器官）/ `claude-code`（已 clone）/ `hermes-agent-rs`（已 clone）。差异化 = 专注「数字游民」细分场景（签证、跨境、远程协作工具链）而现行通用伴侣架构不锁定具体场景。
- **可借鉴**：**场景化 Skill 包**（把行程/预算/签证做成专用 Skill，可参考在 `apeireth-wiki` 上建一个场景子树）／**跨时区调度**（时区感知任务调度可参考）。
- **吸收建议**：P0 不做；P1 待评估；P2 观察；**不做（重复/价值低）**——通用伴侣架构已覆盖「个人助理」基底，数字游民特化不补缺位。
- **0 装 PASS 标注**：**真用：否（未实测）**；**源：未下载实测**（基于 GitHub README 公开信息 + 现行现状推理）；**未调研不写结论**——具体 skill 列表 / 行程规划算法 / 预算模型均为推理判断。
**现状复核（2026-10-23）**：**已过时（对照对象已迁出）**——`apeireth-companion` / `apeireth-action` / `apeireth-motivation` / `apeireth-wiki` 现位于 [legacy/donor/apeireth-companion](../legacy/donor/apeireth-companion) / [apeireth-action](../legacy/donor/apeireth-action) / [apeireth-motivation](../legacy/donor/apeireth-motivation) / [apeireth-wiki](../legacy/donor/apeireth-wiki)；现行内置应用面为 17 面全部点亮（[00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) **I 星港内置面：✅ 17 面全部点亮（2026-10-11）**），**与本篇「场景化 Skill 包」建议方向一致**。**仍成立（方法论）**：① **「通用架构不锁定细分场景 → 细分只作为 Skill 包 / 场景子树」的定位纪律**——与现行内置应用面「统一宿主 + 贡献点注册表 + 多标签壳，每个子应用只实现 `mount(container, context)` 协议」的形态一致（见本册 §11）；② **「已有基底不重复建设」**；③ **0 装三段式标注**。**未核实**：17 面内置面中是否已有场景化 Skill 包形态——**未核实**（本轮未逐面核对）。
**归属链接**：../legacy/donor/apeireth-companion、../legacy/donor/apeireth-action、../legacy/donor/apeireth-motivation、../legacy/donor/apeireth-wiki、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md；同册相关 本册 §11

### Scrapling 评估 (原篇未标日期)

**来源**：`reports/tp30/scrapling-assessment.md`（约 45 行）｜TP30 待评估清单成员。
**结论/要点**
- **机制**：反爬网页爬取（adaptive web scraping），自动绕过主流反爬栈；解决 requests/BeautifulSoup 在现代反爬站点（CF challenge、JS 渲染）直接 403 → 自动选择爬取策略。关键技术 4 项：启发式指纹（自动探测反爬栈）/ 多策略回退（HTTP fetch → headless browser → Spider-like fetch）/ 自动 retry + 退避 / 与 Scrapy 类似的 selector（CSS/XPath）。
- **对照**：相似能力 = `apeireth-tool-fetch`（R149 Tier 1.5 unified fetch engine，吸收 VCP 7 plugins）/ `apeireth-tool-browser`（浏览器自动化）/ `apeireth-tool-web` / `apeireth-web`（Web 套件）/ `composio-next`（已 clone）/ `playwright-mcp`（已 clone）。差异化 = 解决「反爬」而 `apeireth-tool-fetch` 是「统一 fetch 引擎」但**未必处理反爬**；Python 实现 vs Rust 实现需重写或 Python sidecar。
- **吸收建议**：P0 需先实测 + 在 `apeireth-tool-fetch` 上做 POC 验证；P1 待评估；P2 观察；**不做（重复/价值低）**：完整 fork（Python 不兼容 Rust）。
- **0 装 PASS 标注**：**真用：否（未实测）**；**源：未下载实测**（基于 GitHub README + 同类参考对照推理）；**未调研不写结论**——反爬特征库 / 退避算法具体参数 / 与 CF challenge 交互细节均为推理判断；**注：与 `apeireth-tool-fetch` 实际能力冲突时，以其现状为准（避免越权修改其它 agent 工作）**。
**现状复核（2026-10-23）**：**已过时（对照对象已迁出）**——`apeireth-tool-fetch` / `apeireth-tool-browser` / `apeireth-web` 现位于 [legacy/donor/apeireth-tool-fetch](../legacy/donor/apeireth-tool-fetch) / [apeireth-tool-browser](../legacy/donor/apeireth-tool-browser) / [apeireth-web](../legacy/donor/apeireth-web)；原文所列 `apeireth-tool-web` 在现行树**无同名 crate**。**已过时（现行 fetch/爬取面）**——现行工具面见 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §1：`tool.stealth_crawler`（StealthCrawlerEngine::parse_scraped_document，「高反爬异步无头浏览器 + 指纹伪装 + 短视频多模态提取」）已登记为 canonical 能力；fetch 面为 `APEIRETH_ENABLE_FETCH=1`（**默认 OFF**，注册即闸）。**仍成立（方法论）**：① **「多策略回退（HTTP → headless → spider-like）+ 自动 retry + 退避」**是可独立重建的语义——与现行 `tool.stealth_crawler` 的存在互补；② **「Python 实现 → Rust 重写或 sidecar」的移植语义路线**；③ **「与现有能力冲突时以其现状为准（避免越权改他线工作）」**——与本册多篇共有的越权防护一致；④ **「先在既有面上做 POC 验证再谈吸收」**。**未核实**：`tool.stealth_crawler` 与本篇所述反爬能力是否重叠——**未核实**（本轮未逐文件比对），**不作断言**。
**归属链接**：../legacy/donor/apeireth-tool-fetch、../legacy/donor/apeireth-tool-browser、../legacy/donor/apeireth-web、../crates/capabilities/tools、../docs/03-reference/capabilities-matrix.md、../AGENTS.md、../legacy/README.md

### taipy 评估 (原篇未标日期)

**来源**：`reports/tp30/taipy-assessment.md`（约 40 行）｜TP30 待评估清单成员。
**结论/要点**
- **机制**：Python 低代码 dashboard / 数据应用构建（data-driven web apps）；解决业务人员想用 Python 快速搭 dashboard → Streamlit/Gradio 受限 → 提供更结构化方案。关键技术 4 项：声明式 UI（Markdown-like 语法）/ 数据流编排（scenario management）/ 内置图表 + 表单组件 / 与 Pandas/NumPy 深度集成。
- **对照**：相似能力 = `apeireth-tui`（TUI 套件）/ `apeireth-action`（含 StructuredOutput，但非 dashboard）；**缺失：APEIRETH 无 Web dashboard 套件**。差异化 = Python Web dashboard vs 现行主战场是终端 + 命令行；面向业务人员 vs 面向开发者。
- **可借鉴**：**数据流编排思想**（scenario management 的参数 + 数据双向绑定可参考到上下文折叠）/ **声明式 UI**（若后期要做 Web 端点，companion 已有 OpenAI 兼容，可参考声明式范式）。
- **吸收建议**：P0 不做；P1 需先实测 + 在 `apeireth-companion` 上做 POC；P2 观察项；**不做（重复/价值低）**：Python 与 Rust 栈不兼容，直接 fork 价值低。
- **0 装 PASS 标注**：**真用：否（未实测）**；**源：未下载实测**（基于 GitHub README + 现状推理）；**未调研不写结论**——具体 API 设计 / 与 axum 兼容性均为推理判断。
**现状复核（2026-10-23）**：**已过时（对照对象已迁出 + 仪表盘面已变化）**——`apeireth-tui` 现位于 [legacy/donor/apeireth-tui](../legacy/donor/apeireth-tui)，TUI 现行是 [crates/adapters/tui](../crates/adapters/tui)；**「APEIRETH 无 Web dashboard 套件」已不成立**——现行桌面壳 [frontend/companion-desktop](../frontend/companion-desktop) 含 **MetricsPanel.svelte**（消费 `GET /v1/panel/metrics`，自动刷新，挂 settings 视图）与 **UserAdminPanel.svelte**（住户卡 + owner 管理面）、`daemon-ops.ts`（侧车状态/归属/端点/版本/重启计数，只读零动作面），且**前端套件 82/82 suites**（见 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D9 / D10 / D12 状态注记）。**仍成立（方法论）**：① **「场景化参数↔数据双向绑定」的编排语义**可迁移到上下文折叠（现行 [crates/engine/memory](../crates/engine/memory) 侧）；② **「声明式 UI 范式」**——与本册 §11 提出的「贡献点注册表以纯数据定义（`id / title / kind / mount()`），注册表校验字段白名单，**显式禁止注入 HTML/URL/通道字段**」**方向一致**（数据优先的注册面，禁注入面）；③ **「Python 栈不兼容 → 不 fork，只借语义」**；④ **0 装三段式标注**。**未核实**：现行面板面与本篇所述低代码编排的差距——**未核实**，不作断言。
**归属链接**：../crates/adapters/tui、../frontend/companion-desktop、../crates/engine/memory、../docs/04-internal/backend-enhancement/00-backlog.md、../legacy/donor/apeireth-tui、../AGENTS.md；同册相关 本册 §11

### 基地内置应用面调研——上游参考套件「内置浏览器 / 富文档 / 文本编辑」逐件拆解 (2026-10-11)

**来源**：`reports/embedded-app-suite-research-2026-10-11.md`（约 260 行）｜状态 🔍 调研完成（2026-10-11，本线施工前奏）｜**口径纪律**：全文中性措辞，不使用任何第三方产品自称词汇；真实来源仅登记在文末「引注豁免表」｜**合规红线**：上游双仓均为 **CC BY-NC-SA 4.0**（署名-非商业性使用-相同方式共享）——**本调研只取架构思想，代码零复制**；我们的实现必须独立重建，命名、结构、措辞全部中性化。
**结论/要点**
- **结论速览 5 条**：① 上游是「中枢服务 + 桌面壳」双仓形态（中枢管模型接入/记忆/插件/工具审批，桌面壳管交互面），其最新桌面端核心卖点是一套**内置应用面**（内置浏览器、富文档工作台、文本/源码编辑器、协同画板、内置终端、图表等 **27+ 内置子应用**）；② 「内置」不是各自为政的独立软件，而是**统一宿主 + 贡献点注册表 + 多标签壳**，每个子应用只实现 `mount(container, context)` 协议即可挂载；③ 三大内置件中**富文档工作台技术含金量最高**——一条「源码保持型」渲染编辑管线（**编译时产出字符区间索引 → 渲染态直改 → 偏移映射回源码 → 带原文校验的区间事务写回**），值得独立重建；④ 我们的基地（Tauri + Svelte 桌面壳 + Rust 引擎工作区）具备做同样事情的全部底座，且 Rust 侧可把上游放在 JS 里的重活（编译器、语义树）做成更强的引擎件；⑤ **安全面必须先想清楚**：终端、浏览器桥、可编程脚本岛都是「权限爆炸面」，必须走既有审批分级与安全纵深（00-backlog E），**默认拒绝、人工审批兜底**。
- **上游全景表（只述事实，中性称呼）**：参考中枢服务（Node.js 单体）272 MB、插件面 92 MB / 1252 文件、管理面板 31 MB、多代标签记忆引擎 + 河流式记忆引擎、100+ 插件目录 + 工具审批配置 + 结果隐私过滤、托管浏览器运行时（spawn 系统浏览器 + 托管配置 + 调试端口 + 令牌鉴权）、近期动向 = 统一插件格式 / 配置中心规范化 / 脚本安全强化 / 审核机制强化；参考桌面壳（Electron）模块面 22 MB / 480 文件 + 5 个 Rust 辅助引擎子目录、神经拓扑可视化 / 记忆中心、人类工具箱（插件自动转 GUI 表单）、插件管理中心、语义化网页操控内核（多宿主适配器）+ 侧栏共享浏览器、文档工作台 V3 / 流式渲染器 V4 / 多标签壳。**镜鉴**：上游正在还早期扩张的债，我们零债起建正好绕开。
- **可借鉴 6 条（逐条给机制）**：① **贡献点注册表（数据优先）**——子应用/命令以纯数据定义注册（`id / title / kind / mount()`），注册表校验字段白名单，**显式禁止注入 HTML/URL/通道字段**，防注册面成为注入面；注册/注销有 owner 生命周期（owner 失活即拒绝注册）；② **mount/unmount 协议**——子应用只拿容器与上下文，销毁时必须释放；浮层走 acquire/release 会话配对；③ **多标签壳**——内建与嵌入应用统一进标签体系，可拖出为独立物理窗口；快照与「安定回调」方便测试断言；④ **流式渲染纪律**——消息流式渲染采用**会话 ID + 代次序号 + 消息身份三维校验**防旧任务串写；预览/终稿双阶段提交；⑤ **配置中心化 + 热更新监控**；⑥ **工具审批分级**——高危调用走审批配置；工具结果出站前过隐私过滤。
- **对我们基地的映射**：现状 = 桌面壳 [frontend/companion-desktop](../frontend/companion-desktop)（Tauri + Svelte，已有三栏聊天面、伙伴形象面、治理面、桥接层 `lib/bridge`）+ `crates/` 工作区（记忆/工具/治理/验证面，验证面已有证明件积累）+ 纪律底座（审批分级、能力开关权力审计、沙箱、中性词门禁，全部可复用）；目标形态代号建议 **I**（基地内置应用面），四层结构 = 应用宿主层（贡献点注册表 + 多标签壳 + mount/unmount + 浮层会话，Svelte）/ 浏览面（受控内嵌 webview 标签 + 语义树操控内核，Rust 引擎件）/ 文档面（源码保持型文档管线）/ 文本面 / 终端·画板·图表（后续批次）；权限面全部挂既有审批分级与能力开关（默认拒绝、审计留痕）。
- **分批施工建议（零债、每批过门禁）7 批**：I-1 宿主层（贡献点注册表 + 多标签壳 + 一个 hello 内置应用，钉行为测试，无依赖）→ I-2 文本面（代码编辑组件 + 文本查看器，最轻，依赖 I-1）→ I-3 文档面（唯一真源模型 + 编译索引 + 渲染态直改 + 区间事务写回，MVP 只支持 Markdown+代码围栏，依赖 I-1）→ I-4 浏览面（受控 webview 标签：HTTP/HTTPS 白名单、令牌、空闲回收，依赖 I-1 + E 审批面）→ I-5 语义操控内核（页面→语义树→句柄操作，Rust 实现，挂工具审批，依赖 I-4）→ I-6 文档进阶（可编程岛 + 版本分支 + 提案-合入协议 + 导出，依赖 I-3）→ I-7 终端/画板/图表（视 W 波序与 owner 优先级，各自前置）。
- **与既有账目的关系**：I-4/I-5 深度绑定 **E 安全纵深**（对象级判定、审批族归一）；文档面的「提案-合入」与 **D 干活能力**（受控写 + git 分级）同构，可共用审批链；I-5 受益于 **B 性能**纪律（语义树构建要基准）。
- **owner 决策位（待 owner 拍板）**：含「文档面的格式与扩展名命名（**命名即承诺**）」「浏览面对接哪条审批族（E 的对象级判定现状需要先盘）」等。
- **引注豁免表（真实来源登记，纪律三.4 豁免通道）**：参考中枢服务 = 某 GitHub 仓库 @ `d40dc44e`（2026-10-10 最新提交）；参考桌面壳 = 某 GitHub 仓库 @ `a795428`；本地检出 `_research_mem/upstream-suite-ref/`、`_research_mem/upstream-client-ref/`（gitignore 区，浅克隆）；拉取方式 = 公共加速节点转发（直连 TLS 被网络环境阻断；节点清单实测 32 节点中 26 可用）；许可 = 双仓 LICENSE 均为 CC BY-NC-SA 4.0（NOASSERTION SPDX）→ **零代码复制红线**。
**现状复核（2026-10-23）**：**部分成立（方案仍在现行账面上，编号已变 + 执行面已换代）**——① **内置应用面已落地并改称「星港」**：现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) **I 行**记「内置应用面（星港）：多标签壳 + 贡献点注册表 + 内置件面（伴星/星笺/深望等 **15 面**），批次按工程速度 → **✅ 17 面全部点亮（2026-10-11）**」——**本篇的分批建议已被执行**，但编号从 **I**（基地内置应用面）变为现行 **I 轴 = 星港内置面**（[AGENTS.md](../AGENTS.md) §3 与债栏「命名注记（审计修 D13）」明确：吸收自动化原为 I 轴，与星港内置面字母双占，**统一改名 P 轴**，星港线保留 **I**）；② **「E 安全纵深」绑定已在现行兑现一部分**：E 行记 W1-③ 令牌加固 / W2-② 认证面 / W3-① 域数据门 / W3-② 角色门**已交付**，E-P2 审批族归一与 E-P3 沙箱对象级判定**在飞**；③ **「D 干活能力」同构**已在现行兑现：D 行 P1 git 写分级 ✅ / P2 worktree_apply ✅ / P3 任务编排 ✅（V6-a 持久任务队列 + `tool.task_board`，默认关）；④ **「B 性能」纪律**现行 B 行仍为 **📋 未开工**（横切纪律随波验收）。⑤ **已过时（技术栈语境）**——本篇的「Tauri + Svelte 桌面壳」仍在（[frontend/companion-desktop](../frontend/companion-desktop)），但**现行桌面 CI 已执行面补齐**（`companion-desktop-ci.yml`：tauri-shell `cargo test` + svelte-ui `pnpm test` 82/82 + node 20→22，PR #36 起 3/3 success）。⑥ **合规红线仍成立**：「CC BY-NC-SA 4.0 → 只取架构思想、代码零复制、命名/结构/措辞全部中性化」——与现行 [AGENTS.md](../AGENTS.md) §2 中性表述 + 许可搬运线「独立重建、只搬语义」的纪律一致（[00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D19 与 [legacy/README.md](../legacy/README.md)）。**仍成立（方法论，本篇是全仓最完整的「借思想不借代码」样本）**：① **「统一宿主 + 贡献点注册表 + 多标签壳，`mount(container, context)` 协议」**的架构形态（已被星港 17 面验证）；② **「注册表字段白名单 + 显式禁止注入 HTML/URL/通道字段，防注册面成为注入面」**——与现行 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0 的 `supported`/`available`/`reason` 三元表达、[claims.yaml](../docs/status/claims.yaml) 每条 claim 的 `sources` 路径白名单同构；③ **「流式渲染三维校验（会话 ID + 代次序号 + 消息身份）防旧任务串写」**——与现行 `gov.reflection_queue` 的「仅库内注入面、生产零行为变化」和 R1b 总线订阅的「订阅者与模块共享同一状态、开关关 = 零发布」同属「防串写/防重复推进」的纪律；④ **「owner 失活即拒绝注册」的 owner 生命周期**；⑤ **「源码保持型管线：编译期区间索引 → 渲染态直改 → 偏移映射回源码 → 带原文校验的区间事务写回」**四段式——可独立重建；⑥ **「分批施工 + 每批过门禁 + 依赖前置显式列出」**的批次纪律。**未核实**：17 面内置面的当前具体清单与本篇 7 批建议的逐批对应关系——**未核实**（本轮未逐面核对）。
**归属链接**：../frontend/companion-desktop、../docs/04-internal/backend-enhancement/00-backlog.md、../docs/03-reference/capabilities-matrix.md、../docs/status/claims.yaml、../AGENTS.md、../legacy/README.md、../crates/engine/runtime-assembly/src/canonical/production.rs；同册相关 本册 §10

### Yanshu / Memory donor mapping — vNext (2026-09-06)

**来源**：`reports/yanshu-memory-donor-mapping.md`（约 65 行）｜日期 2026-09-06｜**定位**：这份映射记录的是「**可吸收的语义**」和「**不能假装已经吸收的实现**」；当前仓库的 canonical owner 仍是 `crates/engine/memory`，旧实现只作为行为和兼容性参照。
**结论/要点**
- **能力映射表 11 行（能力 / 参照来源 / 处理 / 当前证据）**：① 情节·工作·语义·关系四层记忆（`legacy/donor/apeireth-companion` memory 相关模块 + 现有 `apeireth-memory`）→ **ADAPT**——`MemoryCoordinator` 保留现有 SQLite、偏好、图谱边界，并**增加 scope / provenance / ranking 合同**；② 关闭世界记忆注入（旧 `memory_injection` 语义）→ **ADAPT**——统一使用 `<governed_memory>…</governed_memory>`，旧无 coordinator 分支**也不再发出 `Retrieved memory context`**；③ 偏好与画像 → **ADAPT**——`PersonaMemoryProfile` + revision-checked `PersonaProfileDelta`，**规则提取器仅作离线降级，不声称等同模型提取**；④ 混合检索与中文回退（旧 BM25/vector 思路）→ **ADAPT**——`HybridRetrievalPipeline`：scope filter → lexical/vector union → dedup → centralized ranking → budget；**无 embedding 时可解释地 lexical fallback**；⑤ 向量持久化（旧 vector 仅内存语义）→ **ADAPT**——`VectorRecord` 保存 model / dimension / content hash / 更新时间；SQLite migration v3 + metadata store；⑥ 记忆压缩 / continuity → **ADAPT**——`ContinuityState` 扩展 rolling summary、goals、unresolved threads；`ContextWindowManager` **只改变 provider projection，不改 transcript**；⑦ 远程 embedding / LLM extraction → **DEFER**——已定义 `EmbeddingProvider` / `MemoryExtractor` trait，**没有把 HTTP、密钥或未验证模型写进 Memory owner**；⑧ DialoguePool / MemoryScope → **ADAPTED / production**——`MemoryScope`（Global/User/Project/Persona/Session）+ `ScopedMemoryBackend` + production `SqliteBackend` + SQLite `episode_memory_metadata` 索引；**无 metadata 严格 fail-narrow 到 Session(session_id)**，经由 T3..=T6 与 T12 垂直验证；⑨ 外部 memory provider（Mongo / remote 等）→ **REJECT / DEFER**——当前没有批准的 canonical repository contract，**不恢复旧 provider bridge**；⑩ Yanshuai-AI / OnDeviceAI 的 Windows C# UWP / D3D11 实现 → **REJECT**——与跨平台 Rust + provider capability 架构不匹配；**不移植 UI / 平台绑定代码**。
- **不变量 4 条**：① scope 默认是显式可见集合；旧 session-linked episode 没有 metadata 时**只按 session scope 解释**；② forget 仍由 `MemoryGovernanceStore` 决定，recall / context compile / consolidation **都先过滤 forgotten 状态**；③ provenance **只保存来源身份和 trace/request 引用**，不把 prompt、secret、chain-of-thought 放进 Guard feature 或 Memory control plane；④ embedding 与 reranker 是**可选适配器**——适配器不可用**不会让本地 lexical recall 伪装成 semantic recall**。
- **证据边界**：`crates/engine/runtime-assembly/tests/cognitive_vnext_production.rs` 驱动真实 production module assembly、runtime、scripted provider 和 coordinator-backed SQLite memory path，证明「**写入 → provider 请求注入 → 重启 → forget 后不再召回**」；`crates/engine/runtime-assembly/tests/cognitive_convergence_vertical.rs::test_t12_production_sqlite_backend_scope_and_legacy_boundary` 直接驱动 CLI 使用的 pooled `SqliteBackend`，证明 scope persistence、跨 session 查询与 legacy fail-narrow；**两者都没有证明远程 embedding 质量、模型抽取质量或跨平台桌面发布包质量——这些保持 DEFER / 待 CI**。
**现状复核（2026-10-23）**：**仍成立（本册唯一基本成立的一件，且是全册最成熟的语义映射范本）**——① **canonical owner 仍是 `crates/engine/memory`**（[crates/engine/memory](../crates/engine/memory)，现行存在）；② **「可吸收的语义 vs 不能假装已经吸收的实现」的二分**——与现行 [AGENTS.md](../AGENTS.md) §1 四级口径（IMPLEMENTED / PRODUCTION WIRED / DEFAULT ENABLED / HARDWARE VALIDATED）、[claims.yaml](../docs/status/claims.yaml) 每条 claim 的 `level` + `notes` 残余标注**逐字同构**；③ **「DEFER 明确写『没有把 HTTP、密钥或未验证模型写进 Memory owner』」**——与现行债栏 **D19** 的「不把未验证依赖写进主 lock」、**D18** 的凭据面边界同源；④ **「REJECT 并写明理由（架构不匹配 / 无批准的 canonical contract）」**——与现行 [legacy/README.md](../legacy/README.md)「不移植架构、只移植语义」一致；⑤ **「fail-narrow 到 Session(scope)」**——与本册 §5（Guard 3.1 fail-narrow）、[capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0 的 `reason=not_implemented` 同属「不假装」的统一动作；⑥ **「可选适配器不可用时不伪装 semantic recall」**——与现行 `sec.linux_containment`（partial 如实标注）、`mem.*` 族「简化/实验级」标注同构；⑦ **两个证据文件仍存在**（[cognitive_vnext_production.rs](../crates/engine/runtime-assembly/tests/cognitive_vnext_production.rs)、[cognitive_convergence_vertical.rs](../crates/engine/runtime-assembly/tests/cognitive_convergence_vertical.rs)），**证据边界声明可逐条复核**。**已过时（部分实现面）**——「`legacy/donor/apeireth-memory-extensions`」的映射结论（REJECT/DEFER）仍成立，但该 donor 目录现状（[legacy/donor/apeireth-memory-extensions](../legacy/donor/apeireth-memory-extensions) 现存）；「Yanshuai-AI / OnDeviceAI 的 C# UWP / D3D11 实现」对应的 [../docs/04-internal/borrow-from-jimmyxiao2009.md](../docs/04-internal/borrow-from-jimmyxiao2009.md) 引用面仍存在；**记忆面在现行已再进化**（现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) C 行：随手记入口面**已交付**，夜间整编/教训回路/图谱向量生产待开工；`mem.lesson_object` 已 PRODUCTION WIRED 但 **L3 反思步与 L4 上下文注入未做**，不注入 = 候选不注入上下文）。**未核实**：本篇列出的 scope / provenance / ranking 合同在现行记忆 crate 中的逐项实现状态——**未核实**（本轮未逐文件核对，仅核了 evidence 文件存在性）。
**归属链接**：../crates/engine/memory、../crates/engine/runtime-assembly/tests/cognitive_vnext_production.rs、../crates/engine/runtime-assembly/tests/cognitive_convergence_vertical.rs、../legacy/donor/apeireth-companion、../legacy/donor/apeireth-memory-extensions、../docs/04-internal/borrow-from-jimmyxiao2009.md、../docs/status/claims.yaml、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md、../legacy/README.md

---

## 现状对照（2026-10-23）— 本册 12 篇横向口径

| 维度 | 原篇口径 | 现行口径（真源） | 判定 |
|---|---|---|---|
| 0 装标注骨架 | 「真用：否 / 源：未下载实测 / 未调研不写结论」 | **仍成立**（AGENTS.md §1；claims 四级口径） | 仍成立 |
| 唯一实测件 | GitNexus（文件名级确认，17 MCP tools） | `research/source/` 检出**不在现行树**；结论与许可边界仍可引用 | 部分成立 |
| 对照对象（apeireth-tool-* / bus / agent / council / companion / action / motivation / wiki / context-fold / tui / web / onion / verify / asi / guard / tool-approval） | canonical crate | **legacy/donor/** 同名目录（不注册能力） | 已过时 |
| Provider 面 | `apeireth-provider`（5 Provider 真合并） | `crates/engine/provider` + `rt.provider_router`（PRODUCTION WIRED，默认 ON）+ `core.cost_router`（默认 OFF） | 已过时 |
| 内置应用面 | 提案代号 I，7 批施工建议 | **已执行并改称「星港」**（现行 I 轴 = 星港内置面，✅ 17 面全部点亮 2026-10-11）；原 I 轴已改名 **P 轴** | 部分成立 |
| 仪表盘 / Web 面板 | 「APEIRETH 无 Web dashboard 套件」 | **不成立**：MetricsPanel / UserAdminPanel / daemon-ops + 前端 82/82 | 已过时 |
| OCR 面 | 「APEIRETH 无 OCR 套件」 | **仍成立**（图像核心 11 crate 已入树但 `image_ops` 词表不含 OCR） | 仍成立 |
| 端侧推理 | 「家用集群推理需求未触发」 | SG-09 端侧推理仍登记为待排期缺口 | 部分成立 |
| 记忆面 | scope / provenance / ranking 合同（ADAPT） | canonical owner 仍是 `crates/engine/memory`；`mem.lesson_object` 已 PRODUCTION WIRED（L3/L4 未做） | 部分成立 |
| 「只借语义不借代码」+ 许可红线 | CC BY-NC-SA 4.0 / PolyForm Noncommercial → 零代码复制 | **仍成立**（AGENTS.md §2 + legacy/README + 许可搬运线纪律） | 仍成立 |
| 需求未触发 → 不立项 | P0 不动 / P2 观察 | **仍成立**（D19「技术可行但暂不立项」的同款处置） | 仍成立 |
| 越权防护 | 「与现状冲突时以现状为准」 | **仍成立**（多线并发施工纪律） | 仍成立 |

**本册复核统计**：12 篇中标「**已过时**」结论 **13 条**、「**仍成立**」**16 条**、「**部分成立**」**7 条**、「**未核实**」**8 条**。

*本册为 `reports/` 合册产物；来源 12 篇与分配表 `doc-audit/reports-assignment.txt` §07 一致（逐篇唯一归册）。历史内容按原文保留、不追溯改写；现行口径一律另起「现状对照（2026-10-23）」并指明真源。*
