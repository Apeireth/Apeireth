# E-3 草稿 — 根 README "## 📚 引用" 节

```
[Document-Meta]
Document:       docs/1.0-release-prep/03-citation.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 12 项 #1 doc E-3 续补
Last-Modified:  2026-08-06
Status:         🟢 草稿 (根 README.md LOCKED, 等 Mavis 整合 #3 拍板)
Author:         Mavis (Mavis@local)
Source:         续 reports/1.0-release-doc-30-2026-08-06.md §1.2 E-3
Target:         学术可引用性 (BibTeX 入口 + 前身致敬 + 6 哲学锚)
```

> **性质**: 根 README.md **缺"引用"节** 草稿 (per 续补报告 §1.2 E-3: 0 学术引用入口, 接手者无法 cite 本项目).
>
> **本节草稿目标**: 让学术研究者 **1 跳** 看到 BibTeX 引用 + 前身 (SpectrAI 0.9.21) 致敬 + 6 哲学锚入口.
>
> **不假装**: BibTeX entry 基于 R20 真实状态 (R14 Rust 重写 + v2/v4/v4.1 + 22 trait 互锁 + V-Measure 24 维 + workspace version 1.0.0), 0 编造.

---

## §0. 草稿内容 (建议合入根 README 借鉴节后)

> **合入位**: 根 README 借鉴节 (line 263 后新增) 后, **新增** 1 个 H2 节 "## 📚 引用".

```markdown
## 📚 引用 (Citation)

如果在本项目基础上做学术研究, 请引用:

### BibTeX

\`\`\`bibtex
@software{apeireth2026,
  title        = {Apeireth: A Long-Horizon AI Growth Platform (Rust Implementation)},
  author       = {Apeireth Team},
  year         = {2026},
  version      = {1.0.0},
  url          = {https://github.com/apeireth/apeireth-rust},
  note         = {R14 Rust rewrite; v2/v4/v4.1 three-architecture; 22 trait interlock matrix; V-Measure 24 dimensions}
}

@misc{apeireth-r20-2026,
  title        = {Apeireth R20: 1.0 release with 14 new crates and 8-package distribution},
  author       = {Apeireth Team},
  year         = {2026},
  howpublished = {GitHub release},
  note         = {R20 阶段 1-6 收口, 11 commits + 13 收口文档 + 12/12 checklist PASS}
}
\`\`\`

### 前身 (前人肩上)

| 项目 | 时间 | 关系 | 协议 |
|------|------|------|------|
| **SpectrAI 0.9.21** (前身商业版) | 2024-2025 | 67 crate 1:1 翻译来源 | Apache-2.0 |
| **VCPChat** (Electron 桌面 app, chat-first) | 2025-2026 | 19 文件分析 (协议 + 工具注册 + TUI) | Apache-2.0 |
| **Yinta fork** (权限分心) | 2024 | 5 步权限发放借鉴 | Apache-2.0 |
| **Hermes 团队** | 2024 | 早期 R11 借鉴 | (内部致敬) |
| **m3-hallucination-defense** (内部规范) | 2026-08-05 | 24 LOCKED crate src/ 0 触碰守门 | (内部规范) |

详见 [`docs/stage3-blueprints/borrowed-from-r11.md`](./docs/stage3-blueprints/borrowed-from-r11.md) (LOCKED 阶段 3) + [`docs/v2-strategy/07-VCP-GAP-UPGRADE-PLAN.md`](./docs/v2-strategy/07-VCP-GAP-UPGRADE-PLAN.md) (R17 战役 4).

### 核心哲学 (6 哲学锚)

Apeireth 区别于一般 agent 平台的 6 个工程哲学 (per `docs/adr/0010-6-philosophy-anchors.md` 175 行 LOCKED):

| 锚 | 名称 | 核心 |
|---|------|------|
| **S-1** | 北极星导向 | 平台中立 + 中央 AI 主体性 + 私域 = 栖居地 |
| **S-2** | 实事求是 | 每项升级引用具体对标项目的具体文件 + 行号 |
| **O-2** | 走在前人肩上 | 30 项目源码逐项对比, 0 重造轮子 |
| **O-3** | 干到底 | 短期 / 中期 / 长期 3 阶段 + 每项 DoD |
| **O-4** | 任何人都能接手 | 表格化 + 借鉴 ID + 时间盒 + 责任人 |
| **O-5** | 不假装 | 严守 5 项不假装 (实查 / 失败标 FAIL / 不假装已实现) |

完整 6 哲学锚穿透: 见 [`docs/adr/0010-6-philosophy-anchors.md`](./docs/adr/0010-6-philosophy-anchors.md) (175 行 LOCKED).
```

---

## §1. 草稿要点 (Mavis 整合 #3 拍板用)

| # | 要点 | 依据 |
|---:|------|------|
| 1 | **2 个 BibTeX entry**: apeireth2026 (主) + apeireth-r20-2026 (release tag) | per 续补报告 §5.1 模板 + R20 真实状态 |
| 2 | **5 前身致敬表**: SpectrAI 0.9.21 + VCPChat + Yinta + Hermes + m3 | per DEPENDENCY §5 + ROADMAP.md §R19 关键产物 |
| 3 | **6 哲学锚 1 跳表**: 名称 + 核心, 引导跳 ADR 0010 | per `docs/adr/0010-6-philosophy-anchors.md` 175 行 |
| 4 | **不假装 entry**: BibTeX year 2026 (项目起始), version 1.0.0 (实查) | 0 编造, 全部基于实查 R20 状态 |
| 5 | **学术可引用性**: 接手者读根 README 即可 cite, 无需跳 5+ 文件 | O-4 任何人都能接手 |

---

## §2. 守门表

| 守门 | 本草稿 | 验证 |
|------|--------|:----:|
| **0 触碰根 README.md** (LOCKED) | 草稿在本文件, 不动根 README | ✅ |
| **0 触碰根 CHANGELOG.md** (LOCKED) | 草稿不动 CHANGELOG, BibTeX 在 README | ✅ |
| **0 触碰 docs/adr/0010-6-philosophy-anchors.md** (LOCKED) | 草稿仅引用, 6 哲学锚表来自实查 | ✅ |
| **0 改 workspace version** | 草稿不动 Cargo.toml (BibTeX 1.0.0 引用实查) | ✅ |
| **6 哲学锚穿透** (S-1/S-2/O-2/O-3/O-4/O-5) | S-2 实事求是 (BibTeX 全部基于实查) + O-4 接手可达 (1 跳 cite) | ✅ |
| **8 项不修改承诺** | 不假装已实现 (BibTeX 1.0.0 实查) + 编译期 hardcode (semver 严守) | ✅ |
| **诚实标缺** | BibTeX 标 version 1.0.0 (实查 Cargo.toml) + R20 阶段 1-6 标 11 commits (实查 1.0-release/README.md §5) | ✅ |

---

## §3. R21 续合入动作

1. 主解除根 README.md LOCKED
2. R21 sub-agent 在根 README 借鉴节后**新增** 1 个 H2 "## 📚 引用" (per §0 草稿)
3. 估 commit: `docs: R21 续 — 根 README 加"引用"节 (per #1 doc 续补 E-3, 2 BibTeX + 5 前身 + 6 哲学锚)`
4. 工时估: 0.5h (新增 H2 + 复刻 §0 草稿)

---

_本草稿路径: `docs/1.0-release-prep/03-citation.md`_
_生成时间: 2026-08-06_
_续: `reports/1.0-release-doc-30-2026-08-06.md` §1.2 E-3 (根 README 缺"引用"节, 估补 1h → 草稿 0.5h, 合入 0.5h)_
_6 哲学锚穿透 + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 主动 commit_
