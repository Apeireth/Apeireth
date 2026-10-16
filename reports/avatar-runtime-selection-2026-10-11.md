# 桌面常驻伙伴 2D 形象层渲染路线选型报告（2026-10-11）

> 用词纪律：正文分析使用中性类别名（"某类专有形象格式"“开放骨骼动画格式”等）；具体产品名只出现在对比表、候选清单、许可证据与素材生态证据中。
> 证据纪律：每条许可结论附本次实际抓取的 URL 与关键条款摘录；抓取失败或未覆盖的一律写"未核实"（web_search 当前 401 故障，未做社区口碑检索，性能口碑均为工程推断并已标注）。
> 许可白名单（已拍板）：T1 公有领域等价（CC0/Unlicense）；T2 允许商用署名类（MIT/Apache-2.0/BSD/CC-BY-4.0）。拒收：NC / ND / 禁二次分发 / 许可不明 / "个人免费、商用需另行授权"的专有运行时。运行时与资产分开审。

## ① 一句话结论

**主线走"开放许可的 2D 骨骼动画运行时（JSON+图集、MIT 类）+ 自制或委托的形象资产"，开放 3D 人形格式 + 卡通渲染（MIT 运行时）作为参数面更全的备选，帧序列精灵图兜底；分层 2D 网格变形的专有形象生态仅作对照评估、不进产品——其运行时与素材许可均落在白名单外。**（初始假设大体证实，修正点见 §⑤。）

## ② 四路线对比矩阵

| 维度 | A 分层 2D 网格变形（专有格式/运行时） | B 开放/半开放 2D 骨骼动画 | C 开放 3D 人形格式+卡通渲染 | D 帧序列/精灵图 |
|---|---|---|---|---|
| 运行时许可 | 专有双许可（专有核心 + 厂商自定"开放软件"许可）；年营收 ≥1000 万日元须签出版许可并付费 → **白名单外，拒收** | 两极分化：行业主流格式的运行时**绑定编辑器授权**（自定义许可）→ 拒收；MIT 类 JSON+图集运行时 → **白名单内**；另有运行时 MIT 但格式为闭源二进制的"半开放"方案 | 渲染库与 VRM 加载库均 MIT → **白名单内**；但格式规范文本许可未核实 | 零第三方运行时（CSS/Canvas 自绘）→ 零许可风险 |
| 资产生态许可分布 | 生态最大但白名单占比低：模型市场逐作品授权，多为"仅个人使用/自定许可"；官方免费样本也是专有免费素材许可 + 禁改设定，合作角色禁商用禁改禁再分发 | 主流格式素材市场丰富但随编辑器授权体系流转；MIT 路线素材稀少，**首个形象需自制/委托** | 模型内嵌商用许可与再分发许可元数据（CC0/CC-BY/…/禁再分发），**可机器筛选**；白名单子集=CC0/CC-BY 且商用允许 | CC0/CC-BY 精灵库可得但风格受限；角色级连续帧需自制/委托 |
| WebView 性能 | 2D 网格变形负载低、内存小，官方 Web 运行时可在 WebView 跑（口碑好，**工程推断/未核实**） | 2D 骨骼+图集，负载低、内存小，Canvas/WebGL 双可；失焦暂停自管渲染循环即可 | WebGL 必需，内存/CPU 显著高于 2D；<150MB 预算下需实测；老机器必须回落零 WebGL 光态形象；失焦暂停需自行节流渲染循环 | 最低负载、内存可控；天然就是零 WebGL 回落方案 |
| 表情·口型·视线参数面 | 分层网格参数（口型开合/形状、眨眼、视线、物理摆动）最贴合 2D 二次元表现，参数由模型自带参数 ID 驱动 | 骨骼变换 + 网格形变 + 附件切换；口型靠形变/换图，视线靠眼部骨骼，呼吸靠循环动画或代码摆动；**够用但需在制作规范里约定参数命名** | **最全**：标准化表情（a-i-u-e-o 口型、眨眼、情绪）、视线三种驱动（骨骼/BlendShape/纹理 UV）、发型物理、卡通材质光照直接映射"光色" | 离散帧组合：口型/眨眼/视线=预渲染状态；呼吸=CSS/变换；光色=滤镜/混合近似 |
| 维护活跃度 | 商业公司持续维护（许可文本 2025-02 修订、SDK 页面 2026 年在线） | 主流商业格式持续维护；MIT 龙骨系运行时版权年到 2025 且工具链转向新编辑器（精确活跃度未核实）；半开放方案运行时持续维护（未核实细节） | three-vrm 提交至 2026-09-09（已核实）；three.js 持续发版（工程推断） | 无依赖 |
| 结论 | 对照评估，不进产品 | **主线**（选 MIT 运行时分支） | **备选**（参数面升级项） | **兜底** |

### 候选清单（产品名仅列于此与证据节）

| 候选 | 归属路线 | 运行时许可 | 白名单判定 |
|---|---|---|---|
| Cubism Web（Live2D Cubism SDK for Web） | A | 专有许可 + 出版许可制度 | 拒收 |
| Spine runtimes（spine-ts 等） | B | 自定义许可，绑定编辑器授权 | 拒收 |
| DragonBonesJS（JSON+图集） | B | MIT | 通过（编辑器许可另核） |
| Rive（rive-cpp / rive-wasm 系运行时） | B（半开放） | 运行时 MIT，格式为闭源二进制 | 有条件通过，格式不开放 |
| three-vrm + three.js（VRM 1.0） | C | MIT + MIT | 通过（规范文本未核实） |
| UniVRM（Unity 侧参考实现） | C | MIT | 通过（备查） |
| 自制帧序列（精灵图 + CSS/Canvas） | D | 无第三方依赖 | 通过 |

## ③ 各候选许可证据（URL + 关键条款摘录）

**A 路线 · 分层 2D 网格专有生态**
1. Cubism Web Samples LICENSE（https://raw.githubusercontent.com/Live2D/CubismWebSamples/develop/LICENSE.md）：
   - "Live2D Cubism Core is available under Live2D Proprietary Software License."
   - "*All business* users must obtain a Cubism SDK Release License. 'Business' means an entity with the annual gross revenue more than ten million (10,000,000) JPY for the most recent fiscal year."
   - 官方样本模型 "available under Free Material License"（另附逐角色条款）。
2. 组件许可全文（https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html）：名为 "Open Software License" 实为专有合同——§2.3 "the Customer may not Distribute or Publish Derivative Works … unless the 'Live2D Publication License Agreement' is separately executed"；§5.7 "may not use the Derivative Work or the Output File with software(s) or middleware(s) that compete with … Live2D"；§5.5 样本素材仅限 "Internal evaluation … and Training"。→ 不满足 T1/T2（既非公有领域也非 MIT/Apache/BSD/CC-BY）。
3. 核心许可全文（https://www.live2d.com/eula/live2d-proprietary-software-license-agreement_en.html）：§2.1 仅授予"为出版派生作品而使用"的有限许可；§2.2 一般用户/小规模事业者（年营收 <1000 万日元）可免签出版许可，超额须补签并追溯；§6.4 禁止逆向工程。→ 属"个人/小规模免费、规模化商用需另行授权"的专有运行时，**按红线默认拒**。
4. 出版许可页（https://www.live2d.com/en/download/cubism-sdk/release-license/ ，重定向至 /en/sdk/license/）："License only required upon releasing your content… Individuals and Small-Scale Enterprises are exempted from the license and payment (except Expandable Application)"；VTuber 直播场景另有 "annual sales … exceed 20 million JPY" 门槛与 "Expandable Applications" 事前审批制。

**B 路线 · 开放/半开放 2D 骨骼**
5. 主流格式运行时许可（https://raw.githubusercontent.com/EsotericSoftware/spine-runtimes/4.2/LICENSE）："Integration of the Spine Runtimes … is permitted under the terms and conditions of Section 2 of the Spine Editor License Agreement"；否则 "each user of the Products must obtain their own Spine Editor license"。→ **运行时许可 ≠ 编辑器许可的经典陷阱：运行时本身不开放**。
6. 编辑器许可（https://esotericsoftware.com/spine-editor-license）：§2.1(b)/2.4(b) "You have a valid Spine Editor license at the time the Spine Runtimes are integrated into each Product"；§1.1.1 收入门槛 "less than $500,000 USD … in the last 12 months"；§1.4.1 试用版 "does not grant rights to integrate, distribute, or otherwise make use of the Spine Runtimes"。→ 拒收（运行时绑定付费编辑器授权）。
7. 龙骨系 JS 运行时 LICENSE（https://raw.githubusercontent.com/DragonBones/DragonBonesJS/master/LICENSE）："The MIT License (MIT) … Copyright (c) 2012-2025 The DragonBones team" → **T2 白名单内**。README（https://raw.githubusercontent.com/DragonBones/DragonBonesJS/master/README.md）提供 PixiJS/Phaser/Cocos 等集成，"Highly suggest use LoongBones … to create animation"（工具链已转向新编辑器，其许可未核实）。
8. 半开放方案运行时（https://raw.githubusercontent.com/rive-app/rive-cpp/master/LICENSE）："MIT License … Copyright (c) 2020 Rive" → 运行时 T2 内；但文件格式为闭源二进制、编辑器为订阅制网页工具（条款页未核实，见 §⑥）。

**C 路线 · 开放 3D 人形格式 + 卡通渲染**
9. three-vrm LICENSE（https://raw.githubusercontent.com/pixiv/three-vrm/dev/LICENSE）："MIT License … Copyright (c) 2019-2026 pixiv Inc." → T2 内。
10. three.js LICENSE（https://raw.githubusercontent.com/mrdoob/three.js/dev/LICENSE）："The MIT License … three.js authors" → T2 内。
11. UniVRM LICENSE.txt（https://raw.githubusercontent.com/vrm-c/UniVRM/master/LICENSE.txt）："MIT License … Copyright (c) 2020 VRM Consortium … Copyright (c) 2018 Masataka SUMI for MToon" → T2 内（卡通材质实现亦 MIT）。
12. 格式性质与参数面（https://vrm.dev/en/vrm/vrm_features/）："cross-platform glTF based format"；标准化 "pose, facial expression, and gaze direction"，表情含 "blinking" 与 "a-i-u-e-o"（口型五音），视线支持 "Bone / BlendShape / TextureUV" 三种驱动，材质含 MToon，物理含 SpringBone。
13. 模型内嵌许可元数据（https://vrm.dev/en/vrm/meta/license/）："For commercial use: Disallow / Allow"；再分发许可类型枚举 "CC0 / CC BY / CC BY NC / CC BY SA / CC BY NC SA / CC BY ND / CC BY NC ND / Redistribution Prohibited / Other"。→ 该生态的资产许可**可机器审计**，白名单子集清晰。
14. 规范仓库 README（https://raw.githubusercontent.com/vrm-c/vrm-specification/master/README.md）仅两行、无许可声明；根目录 LICENSE / LICENSE.md / LICENSE.txt 在 master、main、dev 分支实抓均 404 → **规范文本许可未核实**（见 §⑥）。

**D 路线 · 帧序列兜底**：无第三方运行时，无许可证据需求；素材侧证据见 §④。

## ④ 资产生态许可分布（附证据）

| 生态 | 许可分布现状 | 白名单判定 | 证据 |
|---|---|---|---|
| 分层 2D 网格模型市场（官方+第三方） | 逐作品自定许可为主；"仅个人使用"、商用另议常见。官方免费样本亦非开源许可：免费素材许可 + 指定版权标记 + 逐角色约束（部分角色 "No changes of any kind to the design"）；合作角色 "You may neither use … for commercial purposes nor alter nor distribute them" | 白名单占比低，逐件审查成本高 | https://www.live2d.com/en/learn/sample/model-terms/ ；https://docs.nizima.com/en/ （官方市场，买卖 Live2D 数据）；https://docs.nizima.com/en/account/license/ （逐作品"License management"，比例未核实） |
| 商业素材商店（2D 游戏素材） | 自定商业许可：可商用嵌入产品，但 "Distribution of source files is NOT permitted"、"You can NOT resell the source files … or slightly modified version"，另禁 AI 训练 | **禁二次分发类，按红线拒收**（除非走例外流程） | https://craftpix.net/file-licenses/ |
| 开放素材库（通用游戏素材） | 混合许可并存，逐件标注：CC0、CC-BY 3.0/4.0、CC-BY-SA、GPL 2.0/3.0、OGA-BY 等（示例单件为 GPL 3.0） | **白名单子集 = CC0 与 CC-BY 件**；SA/GPL/OGA-BY 件需拒或另审 | https://opengameart.org/art-search-advanced （许可筛选项与总量 36091）；https://opengameart.org/content/dude-with-arms （逐件许可示例） |
| 独立素材工作室（CC0 系） | 全站素材单件标 CC0（例：2D 像素包 "License \| Creative Commons CC0"） | **明确可商用 ✓（T1）** | https://kenney.nl/assets/tiny-farm |
| 3D 虚拟形象生态 | 模型内嵌商用许可字段与再分发许可类型（CC0/CC-BY/…/禁再分发/Other）；个人向模型常标"禁止商用/仅个人" | 白名单子集 = 元数据为 CC0/CC-BY 且"商用允许"的模型，可脚本筛选 ✓ | https://vrm.dev/en/vrm/meta/license/ |
| 模型转卖/分享平台 | 平台条款+逐商品许可混杂；BOOTH 条款页跳转 pixiv 政策域；VRoid Hub 条款页抓取被拦 | 大面积"许可不明"，默认不入库 | https://booth.pm/terms （仅验证到跳转，正文未核实）；https://hub.vroid.com/terms （403，未核实） |

**明确可商用的三个具体例子**：① Kenney 素材（单件 CC0，证据见上）；② OpenGameArt 中按 CC0/CC-BY 4.0 筛出的子集（筛选页证据见上）；③ 3D 虚拟形象生态中元数据为"CC0/CC-BY + 商用允许"的模型（许可字段规范证据见上）。

## ⑤ 推荐组合与风险

**主线（现在做）**：B 路线 MIT 运行时（JSON+图集 2D 骨骼，PixiJS 等渲染）+ 形象资产**自制或委托**（合同锁定 CC0 或 CC-BY-4.0 交稿，含源分层文件与二次修改权）。在制作规范中约定契约映射：姿态=根/躯干骨骼，呼吸=循环轨或代码摆动，口型=嘴部网格形变两通道（开合/形状），视线=眼部骨骼，光色=着色器/滤镜参数。
**备选（并行小原型）**：C 路线 three-vrm + three.js（均 MIT）+ 元数据筛出的 CC0/CC-BY 模型；当需要完整口型五音、视线三驱动、发丝物理且实测内存达标时升级为主线。MToon 光照参数天然映射"光色"。
**兜底（必须保）**：D 路线帧序列光态形象（CC0/CC-BY 素材或自制），承担零 WebGL 老机器回落与 WebGL 初始化失败降级；与主线共用同一状态帧契约（离散化）。
**对照评估（不进产品）**：A 路线专有分层网格生态——只用于表现力标杆对照。

**对初始假设的裁决**：①"开放骨骼动画格式为主"——**证实**，但必须剔除"运行时绑定编辑器授权"的主流格式（证据 §③-5/6），真正合规的是 MIT 运行时分支；②"专有生态为辅评估"——**证实**，其运行时与素材均白名单外（§③-1~4）；③"开放 3D 为备选"——**证实且上修**：该路线参数面最全、模型许可可机器审计，若 WebView 实测达标可升主线；④"帧序列兜底"——**证实**。修正点：白名单内 2D 骨骼的工具链与素材生态比假设更薄（主线素材必须自制/委托），而开放 3D 的合规性比假设更干净。

**最大的三个风险**：
1. **工具链与素材缺口**：主线运行时虽 MIT，但配套编辑器许可未核实、生态转向新工具，白名单内现成角色素材几乎为零；首个形象高度依赖自制/委托排期与合同条款（改编权、再分发、CC-BY 署名方式）。
2. **专有生态误用**：最大素材生态默认"仅个人使用/自定许可/禁改禁转"，官方免费样本也非白名单许可；任何"先用后换"都会污染发布物，需在 CI 加素材许可清单（SPDX/来源 URL）门禁。
3. **WebView 性能与回落未实测**：两条主线候选（2D 网格/骨骼渲染、3D 卡通渲染）在 WebView2/WKWebView 的真实内存、失焦暂停行为均无第一方数据；3D 路线强依赖 WebGL，必须先做"150MB 预算 + 失焦暂停 + 零 WebGL 回落"的原型实测再定终选。
次要风险：3D 路线的格式规范文本许可未核实（§⑥-1），若其非 T1/T2 需评估仅用运行时（MIT）+ 自产模型的隔离方案；半开放方案的闭源二进制格式带来长期锁定。

## ⑥ 未核实项清单（诚实边界）

1. **3D 人形格式规范文本的许可**：规范仓库根目录 LICENSE/LICENSE.md/LICENSE.txt 在 master/main/dev 分支实抓均 404（https://raw.githubusercontent.com/vrm-c/vrm-specification/master/README.md 无声明）→ 规范文本许可未核实；运行时许可（MIT×3）已核实。
2. **龙骨工具链编辑器许可**（DragonBones Pro / LoongBones）：官网 https://www.loongbones.app/ 为 JS 渲染仅返回标题，条款未核实；旧编辑器免费但闭源的印象**未核实**。
3. **半开放方案的编辑器订阅条款与文件格式规范开放性**：https://rive.app/pricing 仅返回标题（JS 渲染）；https://rive.app/terms-of-service 与 https://rive.app/terms 均 404 → 未核实。
4. **开源运行时仓库的精确维护活跃度**（龙骨系）：GitHub API 限流（403）、提交 Atom feed 多次抓取失败 → 仅确认 README 版权年 2012–2025 与工具链转向声明。
5. **平台市场内模型许可的具体分布比例**（nizima / BOOTH / VRoid Hub）：nizima 购买页正文截断、pixiv 政策页（https://policies.pixiv.net ）连接被重置、VRoid Hub 条款 403 → 仅确认"逐作品许可管理"机制存在，比例未核实。
6. **各路线在 WebView2/WKWebView 的实测内存/CPU/失焦口碑**：web_search 故障（401）导致无法做社区口碑检索，矩阵中 WebView 性能列为工程推断，无引用来源。
7. **Spine 素材市场、合作角色授权的分布**：未抓取，未核实。
8. **免费素材许可协议全文**：仅抓到引用页（样本数据条款页），协议正文 https://www.live2d.com/eula/live2d-free-material-license-agreement_en.html 未抓取 → 摘录以样本条款页为准。
9. glTF 2.0 基础规范（Khronos）的文本许可：未抓取，未核实。

---
*报告依据的全部 URL 见 §③/§④/§⑥ 内嵌链接；均为 2026-10-11 当日实际抓取或明确记录失败状态的地址。*
