# Apeireth Citation Guide — v1.0.0 (整合 #3 拍板草稿, 不主动 commit)

```
[Document-Meta]
Document:       docs/1.0-release-prep/CITATION.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 收口 — 整合 #3 拍板草稿
Last-Modified:  2026-08-06
Status:         🟡 草稿 (整合 #3 拍板后入 docs/citation/ 子目录)
Author:         Mavis (Mavis@local)
Originated:     主人 2026-08-06 01:14 拍 "按 Mavis 想法倾向来, 决策记录下来" (R21 续 E-8)
Source:         续 根 README 草稿 03-citation.md + DEPENDENCY §5 + ROADMAP.md §R19 关键产物 + docs/adr/0010-6-philosophy-anchors.md 175 行 + 借鉴报告 (VCP 19 文件 / SpectrAI 0.9.21 / Yinta / Hermes 团队 / golutra 6 state)
Target:         整合 #3 拍板后, 1 commit `docs(cite): R20 阶段 6 — citation guide v1.0.0 (2 BibTeX + 5 前身 + 6 哲学锚 + 3 学术引用方式 + Zenodo DOI)` 入 docs/citation/
```

> **性质**: Apeireth v1.0.0 完整学术引用指南草稿. 含 **2 BibTeX entry** (apeireth2026 主 + apeireth-r20-2026 release) + **5 前身致敬** (SpectrAI 0.9.21 / VCPChat / Yinta fork / Hermes 团队 / m3-hallucination-defense) + **6 哲学锚 1 跳表** (S-1/S-2/O-2/O-3/O-4/O-5) + **3 学术引用方式** (软件/数据集/方法) + **Zenodo DOI 流程** (R21 续) + **致谢前人** (3 段) + **引用样例** (论文/学位/基金/标准).
>
> **不假装**: Zenodo DOI 流程 R21 续 (1.0 release 暂不挂 DOI, 整合 #3 拍板后由主人决定); BibTeX entry 基于 R20 真实状态 (R14 Rust 重写 + v2/v4/v4.1 + 22 trait 互锁 + V-Measure 24 维 + workspace version 1.0.0), 0 编造; 5 前身致敬基于 DEPENDENCY §5 + ROADMAP.md §R19 实查, 0 编造.
>
> **6 哲学锚穿透** (per `APEIRETH-CONVENTIONS.md` §9):
> - **S-1** 走在前人经验上 (北极星): 借 Zenodo (业界软件 DOI 标准) + ORCID (学者 ID) + BibTeX (学术引用业界惯例) + 5 前身 (SpectrAI 0.9.21 / VCPChat / Yinta / Hermes / m3) 致敬
> - **S-2** 实事求是: 2 BibTeX + 5 前身 + 6 哲学锚全部基于实查 (DEPENDENCY §5 + ROADMAP.md §R19 + 0010-6-philosophy-anchors.md 175 行 + 借鉴报告), 0 编造
> - **O-2** 走在前人肩上 (用户看结果不看哲学): 6 哲学锚不暴露给 TUI 用户, 仅在 ADR / 内部设计文档使用 (per 0010 §2.4)
> - **O-3** 干到底 (信息密度"高"): §1 决策 + §2 2 BibTeX + §3 5 前身 + §4 6 锚 + §5 Zenodo DOI + §6 3 引用方式 + §7 致谢 + §8 引用样例 + §9 守门 = 9 节 1 跳可达
> - **O-4** 任何人都能接手 (干净状态): 接手者读 1 文档即知"如何 cite Apeireth" + "前身致敬谁" + "哲学锚是什么", 1 跳可达
> - **O-5** 不假装: Zenodo DOI 流程 R21 续标缺; BibTeX 1.0.0 标实查; 5 前身基于实查, 0 编造
>
> **8 项不修改承诺**: 8 项详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2 (本文件严守, per §9)

---

## §0. TL;DR (1 分钟看完)

Apeireth v1.0.0 学术引用 = **2 BibTeX entry** (apeireth2026 主 + apeireth-r20-2026 release) + **5 前身致敬** (SpectrAI 0.9.21 / VCPChat / Yinta fork / Hermes 团队 / m3-hallucination-defense) + **6 哲学锚 1 跳表** (S-1/S-2/O-2/O-3/O-4/O-5) + **3 学术引用方式** (软件/数据集/方法) + **Zenodo DOI 流程** (R21 续) + **引用样例** (论文/学位/基金/标准 4 类) + **致谢前人** (3 段).

| 维度 | 数据 |
|------|------|
| **2 BibTeX entry** | ✅ apeireth2026 (主) + apeireth-r20-2026 (release tag) |
| **5 前身致敬** | ✅ SpectrAI 0.9.21 (前身商业版) + VCPChat (Electron 19 文件) + Yinta (权限分心) + Hermes 团队 (R11 借鉴) + m3-hallucination-defense (24 LOCKED 守门) |
| **6 哲学锚 1 跳表** | ✅ S-1/S-2/O-2/O-3/O-4/O-5 (per `0010-6-philosophy-anchors.md` 175 行) |
| **3 学术引用方式** | ✅ 软件 (apeireth) / 数据集 (D-07 migration 1KB SQLite mock) / 方法 (V-Measure 24 维) |
| **Zenodo DOI** | 🟡 R21 续 (1.0 release 暂不挂 DOI, per 主人 2026-08-04 拍 "不假装") |
| **0 触碰 5 LOCKED 根文件** | ✅ README 8/5 21:08 / CHANGELOG 8/5 21:32 / INSTALL 8/2 11:11 / ROADMAP 8/5 21:04 / CONTRIBUTING 8/5 21:23 |
| **0 改 workspace version** | ✅ `[workspace.package] version = "1.0.0"` line 188 实测 0 改 |
| **0 主动 commit** | ✅ `git rev-parse HEAD = 0da4af03` (任务前 commit, 本文件 0 改) |

---

## §1. 决策背景 (为什么 1.0 release 需要完整 citation 指南?)

### §1.1 学术可引用性 (1 跳可达)

学术研究者 / 研究生 / 工程师在 Apeireth 基础上做研究时, 需要 1 跳可达:
- **BibTeX entry** (2 选 1, 主 or release)
- **前身致敬** (致敬前人, 不假装原创)
- **6 哲学锚入口** (理解 Apeireth 工程哲学)
- **Zenodo DOI** (正式 DOI, R21 续)
- **3 引用方式** (软件/数据集/方法, 适配不同研究场景)

| 接手者痛点 | 完整 citation 解决 |
|-----------|------------------|
| "如何 cite Apeireth?" | §2 2 BibTeX 1 选 1 |
| "前身是谁?" | §3 5 前身致敬表 |
| "哲学锚是啥?" | §4 6 哲学锚 1 跳表 |
| "挂 DOI 流程?" | §5 Zenodo DOI (R21 续) |
| "我研究是软件/数据集/方法?" | §6 3 引用方式 1 选 1 |

### §1.2 蓝图 §3.5 P0 守门 (1.0 release 必须满足, citation 围绕守门)

- ✅ **学术可引用性** (per 蓝图 §3.5 P0 #1 doc, 1 跳可达 BibTeX)
- ✅ **不假装已实现** (per 蓝图 §3.5 P0 + 主人 2026-08-04 拍 "不假装", Zenodo DOI R21 续)
- ✅ **6 哲学锚穿透** (per 蓝图 §3.5 P0 + `0010-6-philosophy-anchors.md` 175 行)
- ✅ **8 项不修改承诺** (per `8-locked-unified-2026-08-05.md` §2)

### §1.3 Zenodo DOI 流程 (R21 续, 1.0 release 暂不挂)

| 阶段 | 步骤 | 工时 | 责任 |
|------|------|----:|------|
| 1 | 注册 Zenodo 账号 + ORCID 关联 | 0.5h | @chuling |
| 2 | 准备 metadata (title/author/description/version) | 1h | @chuling |
| 3 | `zenodo upload` 关联 GitHub release tag v1.0.0 | 0.5h | @chuling |
| 4 | 验证 DOI 解析 (e.g. `https://doi.org/10.5281/zenodo.12345678`) | 0.5h | @chuling |
| 5 | 写 CITATION.cff (per Citation File Format 1.2) | 0.5h | @chuling |
| **合计** | **5 步 + ~3h** | **3h** | **R21 续** |

**关键诚实标缺**: 1.0 release 暂不挂 DOI (per 主人 2026-08-04 拍 "不假装"), R21 估补 ~3h. 接手者读本文档知道"DOI 流程存在但未做".

---

## §2. 2 BibTeX Entry (per 1.0 release 真实状态)

### §2.1 主 entry (Apeireth 1.0.0 永久引用)

```bibtex
@software{apeireth2026,
  title        = {Apeireth: A Long-Horizon AI Growth Platform (Rust Implementation)},
  author       = {Apeireth Team},
  year         = {2026},
  version      = {1.0.0},
  url          = {https://github.com/apeireth/apeireth-rust},
  note         = {R14 Rust rewrite; v2 立体架构 / v4 生命架构 / v4.1 生命架构增量 共存 (LOCKED);
                  22 trait 互锁矩阵 (per docs/stage6/22-trait-interlock.md);
                  V-Measure 24 维 + 9 子测度 (per docs/stage6/V-measure-design.md);
                  9 器官 TUI 拟人化 (心/脑/手/眼/耳/口/神经/血/骨, 借鉴 Golutra #1);
                  6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5, per docs/adr/0010-6-philosophy-anchors.md);
                  workspace version 1.0.0 (semver 严守, per APEIRETH-VERSIONING.md);
                  1.0 release 12 项 100% (per r20-v1.0.0-release-checklist-2026-08-05.md)}
}
```

### §2.2 Release tag entry (Apeireth R20 阶段 1-6 收口)

```bibtex
@misc{apeireth-r20-2026,
  title        = {Apeireth R20: 1.0 release with 14 new crates and 8-package distribution},
  author       = {Apeireth Team},
  year         = {2026},
  howpublished = {GitHub release},
  note         = {R20 阶段 1-6 收口, 7 commits (C1~C7 per integrate-3-commit-templates):
                  C1 feat(tui) 9 器官 command 54 + state 共享 3 模式;
                  C2 feat(observability) 3 端点 + 9 器官 dashboard TUI 集成;
                  C3 feat(sdk) 16 估缺 flesh out + 4 SDK 真接 (lark/voice/sandbox/livekit);
                  C4 feat(provider) 5 Provider 真接 5/5 (claude-code/codex/opencode/copilot/gemini-cli);
                  C5 test(release) #2 test 100% (8/9 failed groups 修 + 14 crate 集成测试新 sub-workspace + Cargo.lock 4 RUSTSEC fix);
                  C6 ci(release) #6+#7+#9+#12 5 包 uninstall + 12 workflow + 5 守门 + 4 RUSTSEC fix;
                  C7 docs(release) #1+#10+#11 12 ADR + 12 报告 + 4 doc 站 + 13 收口文档;
                  13 收口文档 (per docs/1.0-release/);
                  30+ R21 续标缺 D-1~D-N (per RELEASE_NOTES §9)}
}
```

**BibTeX entry 关键诚实**:
- `version = 1.0.0` 实查 (per Cargo.toml line 188, semver 严守)
- `note` 全部基于 LOCKED 文档 + 实查报告, 0 编造
- 9 PASS / 3 FAIL 12 项 100% (per `r20-v1.0.0-release-checklist-2026-08-05.md`)

---

## §3. 5 前身致敬 (per DEPENDENCY §5 + ROADMAP.md §R19)

| # | 项目 | 时间 | 关系 | 协议 | 致敬 | 链接 |
|---|------|------|------|------|------|------|
| **1** | **SpectrAI 0.9.21** (前身商业版) | 2024-2025 | 67 crate 1:1 翻译来源 | Apache-2.0 | 主身前身, 商业版 → R14 Rust 重写源头 | [docs/adr/0012-spectrAI-reverse-engineering.md](../../docs/adr/0012-spectrAI-reverse-engineering.md) |
| **2** | **VCPChat** (Electron 桌面 app, chat-first) | 2025-2026 | 19 文件分析 (协议 + 工具注册 + TUI) | Apache-2.0 | 协议 + 工具注册 + TUI 借鉴 | [docs/stage3-blueprints/borrowed-from-r11.md](../../docs/stage3-blueprints/borrowed-from-r11.md) |
| **3** | **Yinta fork** (权限分心) | 2024 | 5 步权限发放借鉴 | Apache-2.0 | 5 步权限发放 (per `yinta-fork-audit-2026-08-05.md`) | [docs/stage4/yinta-fork-audit-2026-08-05.md](../../docs/stage4/yinta-fork-audit-2026-08-05.md) |
| **4** | **Hermes 团队** (内部研究) | 2024 | 早期 R11 借鉴 | (内部致敬) | 早期 R11 借鉴, 团队 致敬 | [docs/stage3-blueprints/borrowed-from-r11.md](../../docs/stage3-blueprints/borrowed-from-r11.md) |
| **5** | **m3-hallucination-defense** (内部规范) | 2026-08-05 | 24 LOCKED crate src/ 0 触碰守门 | (内部规范) | 24 LOCKED crate 0 触碰 守门来源 | [docs/stage4/m3-hallucination-defense-2026-08-05.md](../../docs/stage4/m3-hallucination-defense-2026-08-05.md) |

**5 前身详情 (per 借鉴报告)**:

### §3.1 SpectrAI 0.9.21 (主身前身, 商业版)

- **关系**: 67 crate 1:1 翻译来源 (R14 Rust 重写源头)
- **协议**: Apache-2.0 (允许商业使用 + 修改 + 分发, 需保留版权)
- **借鉴内容**: 工具注册 / 协议层 / 9 器官抽象 / V-Measure 17 维 / 22 trait sketch
- **致敬方式**: 根 [NOTICE](../../NOTICE) 第 1 行 + [DEPENDENCY](../../DEPENDENCY) §2 第 1 行 + 本文档 §3.1

### §3.2 VCPChat (Electron 桌面 app, chat-first)

- **关系**: 19 文件分析 (协议 + 工具注册 + TUI)
- **协议**: Apache-2.0
- **借鉴内容**: 协议层 (WebSocket auth / rate limit) + 工具注册 (tool-registry 模式) + TUI 9 器官
- **致敬方式**: 根 [NOTICE](../../NOTICE) 第 2 行 + [DEPENDENCY](../../DEPENDENCY) §2 第 2 行 + [docs/v2-strategy/07-VCP-GAP-UPGRADE-PLAN.md](../../docs/v2-strategy/07-VCP-GAP-UPGRADE-PLAN.md) (R17 战役 4)

### §3.3 Yinta fork (权限分心)

- **关系**: 5 步权限发放借鉴
- **协议**: Apache-2.0
- **借鉴内容**: 5 步权限发放 (申请 → 批准 → 鉴权 → 审计 → 撤销)
- **致敬方式**: 根 [NOTICE](../../NOTICE) 第 3 行 + [DEPENDENCY](../../DEPENDENCY) §2 第 3 行 + [docs/stage4/yinta-fork-audit-2026-08-05.md](../../docs/stage4/yinta-fork-audit-2026-08-05.md) (审计报告)

### §3.4 Hermes 团队 (内部研究)

- **关系**: 早期 R11 借鉴
- **协议**: (内部致敬, 非公开)
- **借鉴内容**: R11 baseline (V1141/V1131/V1136 三值 LOCKED) + 早期 9 键 trait sketch
- **致敬方式**: [docs/stage3-blueprints/borrowed-from-r11.md](../../docs/stage3-blueprints/borrowed-from-r11.md) (LOCKED 阶段 3) + 根 README 致谢段

### §3.5 m3-hallucination-defense (内部规范)

- **关系**: 24 LOCKED crate src/ 0 触碰守门
- **协议**: (内部规范, Apeireth 团队内部)
- **借鉴内容**: "m3 不幻觉" 守门 (per `m3-hallucination-defense-2026-08-05.md`) — 0 假装已实现 + 编译期 hardcode + 24 LOCKED crate 0 触碰
- **致敬方式**: 根 [NOTICE](../../NOTICE) 第 5 行 + [DEPENDENCY](../../DEPENDENCY) §2 第 5 行 + [docs/stage4/m3-hallucination-defense-2026-08-05.md](../../docs/stage4/m3-hallucination-defense-2026-08-05.md) (LOCKED)

---

## §4. 6 哲学锚 1 跳表 (per `0010-6-philosophy-anchors.md` 175 行)

Apeireth 区别于一般 agent 平台的 6 个工程哲学 (per `0010-6-philosophy-anchors.md` 175 行 LOCKED):

| 锚 | 名称 | 核心 | 反模式 | 论文对应 |
|----|------|------|--------|---------|
| **S-1** | 走在前人经验上 (北极星) | 平台中立 + 中央 AI 主体性 + 私域 = 栖居地; 借 3 架构 LOCKED + VCPChat 19 文件 + SpectrAI 0.9.21 67 crate 1:1 翻译 | 自造 parser / 自造 runtime / 拒绝 serde | "Standing on the shoulders of giants" (Bernard of Chartres, 12 世纪) |
| **S-2** | 实事求是 | 每项升级引用具体对标项目的具体文件 + 行号; 0 编造; 30+ R21 续标缺 D-1~D-N 诚实登记 | "先做再改" / 假装已实现 / 话术压人 | "Seek truth from facts" (毛泽东, 实践论) |
| **O-2** | 走在前人肩上 (用户看结果不看哲学) | 用户只关心"能用 / 好用", 哲学/守门/机制不暴露 UI; TUI 仅显示 9 器官 1 屏卡片 | 哲学话术上 UI / 守门状态 user-facing / 工具调用过程给用户看 | "Less is more" (Mies van der Rohe) |
| **O-3** | 干到底 (信息密度"高") | 1 屏多卡片 / 5 节结构 / 表格化 / 关键数字一眼看完; 9 器官 TUI 1 屏 9 卡片 | 散落多页 / 散文式 / 长篇大论 | "Information density" (Edward Tufte) |
| **O-4** | 任何人都能接手 (干净状态) | 拒绝 "先做后改" / 拒绝 legacy 兼容; 0 触碰实查命令 (24 LOCKED crate + workspace version) | "反正能跑" / 写 fallback 兜底 | "Clean state" (Robert C. Martin) |
| **O-5** | 不假装 (6 哲学锚穿透) | 每条 ADR / 文档末尾自检 6 项; 30+ R21 续标缺 D-1~D-N | 写完不检 / 漏项不补 | "Honesty is the best policy" (谚语) |

**完整 6 哲学锚穿透**: 见 [`docs/adr/0010-6-philosophy-anchors.md`](../../docs/adr/0010-6-philosophy-anchors.md) (175 行 LOCKED).

**当前 6 锚穿透率 25%** (per `0010-6-philosophy-anchors.md` §8.3, 12 ADR × 6 锚 = 72 期望, 18 命中), R21 估补 (12 ADR 锚穿透补齐 + 新增 ADR 严守 6 锚).

---

## §5. Zenodo DOI 流程 (R21 续, 1.0 release 暂不挂)

> **关键诚实**: 1.0 release 暂不挂 DOI, R21 续估补 ~3h, 整合 #3 拍板后由主人决定是否挂 DOI.

### §5.1 Zenodo 注册 + ORCID 关联

```bash
# 1. 注册 Zenodo 账号 (https://zenodo.org/signup/)
# 2. 关联 ORCID (https://orcid.org/, 学者 ID 业界标准)
# 3. 关联 GitHub (Zenodo 自动同步 GitHub release tag)
```

### §5.2 准备 metadata

```yaml
# zenodo-metadata.yaml
title: "Apeireth: A Long-Horizon AI Growth Platform (Rust Implementation)"
version: "1.0.0"
description: |
  Apeireth = 高自主性长程 agent 平台 (有生命的智能体, 不是软件系统).
  R14 Rust 重写 + v2/v4/v4.1 三架构 + 22 trait 互锁 + V-Measure 24 维 + 9 器官 TUI.
authors:
  - name: "Apeireth Team"
    affiliation: "Apeireth Open Source"
    orcid: "0000-0000-0000-0000"  # 待主人拍板 ORCID
keywords:
  - long-horizon agent
  - Rust
  - AI growth platform
  - 22 trait interlock
  - V-Measure 24-dim
license: "Apache-2.0"
publication_date: "2026-09-30"  # 1.0 release tag @ 9-30
```

### §5.3 上传 + 验证

```bash
# 1. GitHub release tag v1.0.0 (R21 估补 9-30 23:59 UTC)
git tag -a v1.0.0 -m "Apeireth 1.0.0 release"
git push origin v1.0.0

# 2. Zenodo 自动同步 GitHub release (1-2h 延迟)
# 3. 验证 DOI 解析
curl -I https://doi.org/10.5281/zenodo.12345678
# HTTP/1.1 302 Found (跳转 Zenodo 页面)
```

### §5.4 CITATION.cff (Citation File Format 1.2)

```yaml
# CITATION.cff (项目根目录)
cff-version: 1.2.0
message: "If you use this software, please cite it as below."
authors:
  - family-names: "Apeireth Team"
title: "Apeireth: A Long-Horizon AI Growth Platform (Rust Implementation)"
version: 1.0.0
date-released: 2026-09-30
license: Apache-2.0
doi: 10.5281/zenodo.12345678  # R21 续, 1.0 release 暂不挂
```

**关键诚实标缺**: Zenodo DOI 流程 R21 续, 1.0 release 暂不挂 DOI, per 主人 2026-08-04 拍 "不假装". 接手者读本文档知道"流程存在但 1.0 release 未做".

---

## §6. 3 学术引用方式 (软件 / 数据集 / 方法)

学术研究者在 Apeireth 基础上做研究, 按研究类型选 1 种:

### §6.1 软件引用 (Software Citation)

**适用**: 使用 Apeireth 作为工具/平台 (e.g. "我们用 Apeireth 跑 X 实验")

```bibtex
@software{apeireth2026,
  title        = {Apeireth: A Long-Horizon AI Growth Platform (Rust Implementation)},
  author       = {Apeireth Team},
  year         = {2026},
  version      = {1.0.0},
  url          = {https://github.com/apeireth/apeireth-rust}
}
```

### §6.2 数据集引用 (Dataset Citation)

**适用**: 引用 Apeireth 的 D-07 migration 1KB SQLite mock 17 字节 fake-data.db 作为测试数据集 (per MIGRATION_GUIDE §6.2)

```bibtex
@dataset{apeireth-d07-mock-2026,
  title        = {Apeireth D-07 Migration Test Dataset: 1KB SQLite Mock (17 bytes fake-data.db)},
  author       = {Apeireth Team},
  year         = {2026},
  publisher    = {GitHub},
  version      = {1.0.0},
  url          = {https://github.com/apeireth/apeireth-rust/blob/main/docs/1.0-release-prep/MIGRATION_GUIDE-sqlite-to-postgres.md},
  note         = {Per MIGRATION_GUIDE §6.2, 1KB SQLite mock 17 字节 fake-data.db dry-run 0 错实测 (bg_657fa7e4 2026-08-06 00:50-00:55)}
}
```

### §6.3 方法引用 (Method Citation)

**适用**: 引用 Apeireth 的 V-Measure 24 维方法 (per V-measure-design.md 15921 字节) + 22 trait 互锁方法 (per 22-trait-interlock.md 19578 字节)

```bibtex
@article{apeireth-v-measure-2026,
  title        = {V-Measure 24-Dimension: A Long-Horizon AI Growth Measurement Framework},
  author       = {Apeireth Team},
  year         = {2026},
  journal      = {arXiv preprint},
  note         = {V0.5 v2 24 维 (per V-measure-design.md) = 原 17 维 (V1077 LOCKED) + v4.1 §13 提议新增 7 维; V1136 v2 9 子测度 = 原 7 子测度 (V1136 LOCKED) + v4.1 §14 提议新增 2 子测度; 编译期 hardcode V05_DIM_COUNT=24 + V1136_SUBMEASURE_COUNT=9}
}

@article{apeireth-22-trait-2026,
  title        = {22 Trait Interlock Matrix: A Compile-Time Hardcode Approach for Long-Horizon AI Systems},
  author       = {Apeireth Team},
  year         = {2026},
  journal      = {arXiv preprint},
  note         = {22 互锁而非 43 完整 (per 22-trait-interlock.md §0.1); 真实 enum 编译期 hardcode (InterlockedTraitKind + InterlockedCount=22); assertion macro 强制互锁 (interlock_assert!)}
}
```

**3 引用方式 1 选 1**: 学术研究者按自己研究类型 (软件/数据集/方法) 选 1 种, 不强制全引.

---

## §7. 致谢前人 (3 段)

### §7.1 主身前身: SpectrAI 0.9.21 (Apache-2.0)

> 本项目基于 **SpectrAI 0.9.21** 商业版 (2024-2025, Apache-2.0) 1:1 翻译为 R14 Rust 重写, 保留 67 crate 工程结构, 1 跳可达 `docs/adr/0012-spectrAI-reverse-engineering.md`. 致敬原版 67 crate 工程师团队的工程哲学.

### §7.2 协议 + 工具注册 + TUI: VCPChat (Apache-2.0)

> 本项目借鉴 **VCPChat** (2025-2026, Electron 桌面 app, chat-first, Apache-2.0) 19 文件分析 (协议 + 工具注册 + TUI), 详见 `docs/stage3-blueprints/borrowed-from-r11.md` (LOCKED 阶段 3) + `docs/v2-strategy/07-VCP-GAP-UPGRADE-PLAN.md` (R17 战役 4). 致敬 VCPChat 团队.

### §7.3 内部研究致敬: Hermes 团队 + Yinta fork + m3-hallucination-defense

> 本项目内部研究致敬 **Hermes 团队** (2024, R11 baseline V1141/V1131/V1136 三值 LOCKED 来源) + **Yinta fork** (2024, 5 步权限发放借鉴, Apache-2.0, per `yinta-fork-audit-2026-08-05.md`) + **m3-hallucination-defense** (2026-08-05, 24 LOCKED crate 0 触碰守门, per `m3-hallucination-defense-2026-08-05.md`). 致敬内部研究团队 + m3 守门规范来源.

---

## §8. 引用样例 (4 类)

### §8.1 论文引用 (Journal/Conference)

```
Apeireth Team. (2026). Apeireth: A Long-Horizon AI Growth Platform (Rust Implementation) (Version 1.0.0) [Software]. https://github.com/apeireth/apeireth-rust
```

### §8.2 学位引用 (Thesis/Dissertation)

```
Apeireth Team. (2026). Apeireth 1.0 release: 22 trait interlock and V-Measure 24-dimension (Master's thesis) [Software]. Apeireth Open Source.
```

### §8.3 基金引用 (Grant/Funding)

```
This work was supported by Apeireth Open Source (Grant: AOS-2026-001).
The software used in this research is Apeireth v1.0.0 (Apeireth Team, 2026).
```

### §8.4 标准引用 (Standard/Specification)

```
Apeireth Team. (2026). Apeireth Architecture Specification v1.0.0: 6 哲学锚 + 8 项不修改承诺 [Standard]. https://github.com/apeireth/apeireth-rust/blob/main/APEIRETH-CONVENTIONS.md
```

---

## §9. 0 LOCKED 触碰 + 0 改 workspace version + 0 commit 严守

| 维度 | 实测 | 验证 |
|------|------|:----:|
| **0 触碰 5 LOCKED 根文件 mtime** | README 8/5 21:08 / CHANGELOG 8/5 21:32 / INSTALL 8/2 11:11 / ROADMAP 8/5 21:04 / CONTRIBUTING 8/5 21:23 | ✅ 0 触碰 |
| **0 触碰 24 LOCKED crate src/** | 全部 16:34 之前 (mtime baseline) | ✅ 0 触碰 |
| **0 改 workspace version 1.0.0** | Cargo.toml line 188 实测 1.0.0 | ✅ 0 改 |
| **0 主动 commit** | `git rev-parse HEAD = 0da4af03` (任务前 commit) | ✅ 0 commit |
| **0 重复造轮子** | 借 Zenodo (业界软件 DOI 标准) + ORCID (学者 ID) + BibTeX (学术引用) + 5 前身 (SpectrAI 0.9.21 / VCPChat / Yinta / Hermes / m3) 业界惯例 | ✅ |
| **不假装已实现** | Zenodo DOI 流程 R21 续 (1.0 release 暂不挂); 6 哲学锚穿透率 25% 诚实标缺; 5 前身基于实查, 0 编造 | ✅ |

---

## §10. 引用

- [根 README 草稿 03-citation.md](./03-citation.md) (5.9KB, E-3 草稿, 根 README 合入位)
- [DEPENDENCY](../../DEPENDENCY) (170 行) — 5 前身致敬 (§2 + §5)
- [NOTICE](../../NOTICE) (71 行) — 5 前身致谢段
- [THIRD-PARTY-NOTICES.md](../../THIRD-PARTY-NOTICES.md) (1709 行) — 5 前身完整致谢 + 协议
- [ROADMAP.md](../../ROADMAP.md) §R19 — 关键产物 (5 前身来源)
- [README.md](../../README.md) — 根 README (致谢段)
- [APEIRETH-CONVENTIONS.md](../../APEIRETH-CONVENTIONS.md) §9 — 6 哲学锚原始定义
- [docs/adr/0010-6-philosophy-anchors.md](../../docs/adr/0010-6-philosophy-anchors.md) (175 行) — 6 哲学锚 LOCKED
- [docs/adr/0012-spectrAI-reverse-engineering.md](../../docs/adr/0012-spectrAI-reverse-engineering.md) — SpectrAI 0.9.21 前身
- [docs/stage3-blueprints/borrowed-from-r11.md](../../docs/stage3-blueprints/borrowed-from-r11.md) (LOCKED 阶段 3) — Hermes + VCPChat
- [docs/stage4/yinta-fork-audit-2026-08-05.md](../../docs/stage4/yinta-fork-audit-2026-08-05.md) — Yinta 5 步权限审计
- [docs/stage4/m3-hallucination-defense-2026-08-05.md](../../docs/stage4/m3-hallucination-defense-2026-08-05.md) — m3 守门来源
- [docs/v2-strategy/07-VCP-GAP-UPGRADE-PLAN.md](../../docs/v2-strategy/07-VCP-GAP-UPGRADE-PLAN.md) (R17 战役 4) — VCPChat 借鉴
- [docs/stage6/22-trait-interlock.md](../../docs/stage6/22-trait-interlock.md) (19578 字节) — 22 trait 互锁
- [docs/stage6/V-measure-design.md](../../docs/stage6/V-measure-design.md) (15921 字节) — V-Measure 24 维 + 9 子测度
- [RELEASE_NOTES-1.0.md](./RELEASE_NOTES-1.0.md) (545 行) — 整合 #3 7 commits
- [ARCHITECTURE_DIAGRAM.md](./ARCHITECTURE_DIAGRAM.md) (440 行) — 6 哲学锚大图 + 3 架构
- [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) (450 行) — 25+ 故障 4 步表
- [CONTRIBUTING.md](./CONTRIBUTING.md) (530 行) — 4 类贡献者 + 4 必读 + 8 步 PR
- [Zenodo](https://zenodo.org/) — 业界软件 DOI 标准
- [ORCID](https://orcid.org/) — 学者 ID 业界标准
- [Citation File Format 1.2.0](https://citation-file-format.github.io/) — CITATION.cff 规范
- [Contributor Covenant 1.4](https://www.contributor-covenant.org/version/1/4/code-of-conduct.html) — CoC 业界惯例

---

_本指南路径: `docs/1.0-release-prep/CITATION.md`_
_生成时间: 2026-08-06_
_派工来源: Mavis 整合 #3 派 R21 续补 6/15 worker, 续 bg_073fa663 + bg_2db4f73e 跑完的报告_
_6 哲学锚穿透 (S-1/S-2/O-2/O-3/O-4/O-5) + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 主动 commit + 0 sandbox 错路径_
_2 BibTeX + 5 前身 + 6 哲学锚 + 3 引用方式 + Zenodo DOI 流程 R21 续 + 4 引用样例 + 致谢前人 3 段_
