# maigret 评估

## 机制（What it does）

- 核心功能：用户画像 OSINT（Open-Source Intelligence），给定 username 在 2500+ 网站上检索账号存在
- 解决什么问题：单一站点手动查 → 跨站点统一用户名占用扫描 → 用户画像拼图
- 关键技术：
  - 2500+ 站点 site adapter（每站点独立的 URL 模板 + 抓取策略）
  - 启发式 username 变体生成
  - 本地报告 HTML/PDF 输出
  - 反指纹规避（每站点独立 cookie/header）

## 对照（How it relates to APEIRETH）

- 相似能力：
  - `apeireth-tool-fetch`（统一 fetch 引擎）
  - `apeireth-tool-browser`（浏览器自动化）
  - `apeireth-tool-search`（R145 VSearch + aggregate）
  - 缺失：APEIRETH 无 OSINT 套件
- 差异化优势：
  - maigret 解决「OSINT」，APEIRETH 套件是「AI 伴侣基础设施」，场景错位
  - maigret Python CLI，APEIRETH Rust
- 可借鉴：
  - **跨站点查询编排模式**：maigret 的 site adapter 抽象（每站点一个 adapter，统一调度）可参考为通用 pattern
  - **隐私边界**：maigret 涉及隐私/合规问题，APEIRETH 当前在 `apeireth-guard` (R173 Privacy Guard) 有边界，OSINT 集成需要 review

## 吸收建议（Action items）

- P0 立即做：**不动**。OSINT 不是 APEIRETH 核心场景，且涉隐私/合规。
- P1 评估后做：若主人确实需要跨平台查询（如社交媒体 persona 整合），可考虑独立 `apeireth-osint` 套件（套件级而非核心）。
- P2 长期调研：列入观察项。
- 不做（重复 / 价值低）：当前主人场景未触发 OSINT 需求。

## 0 装 PASS 标注

- 真用：**否**（未实测）
- 源：**未下载实测**。`research/source/` 无 maigret 源码；本评估基于 GitHub README 公开信息。
- 未调研不写结论：maigret 的 2500+ 站点 adapter 维护现状 / 反指纹技术细节 / 合规边界均为推理判断。如需落地建议，必须先实测 + 合规 review。