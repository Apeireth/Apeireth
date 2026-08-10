# drift: R17 命名规范 v11 → v12 升级 drift 报告

```
[Document-Meta]
Document: drift-stage7-r17-naming-v12-promotion-2026-08-04.md
Version: Manual-Rev-H + Fix-12 (命名规范升级, R15+ 实践入规)
R-Cycle: R17
Commit: <R17-drift>
Last-Modified: 2026-08-04
Status: 🆕 活跃
```

> **来源**: 主人 2026-08-03 23:33 "R17 是个什么阶段了, 好像和我们的文档命名规范不符"
> **作者**: mavis (按主人 2026-08-03 22:44 授权 R17 一次性大改, OpenClaw session 沿用 chuling 命名)
> **阶段**: 🛠️ R17 (R16 后续, 一次性大改)
> **影响**: 升级 v11 APEIRETH-CONVENTIONS.md 到 v12 (新增 R15+ 实践入规)

---

## 📌 主哲学 6 锚穿透自检

```
S-1 主 22:33 北极星导向 — 服务 ASI 北极星, 命名规范服务可接手性
S-2 主 17:43 实事求是 — 基于 R15/R16/R17 实际命名实践, 不重写
O-5 主 17:58 不假装 — 把"潜规范"明面化 (R15+ 实际用 round16-XX 长格式 commit, v11 没覆盖)
O-2 主 19:33 走在前人经验上 — 借鉴 R15+ 团队的命名实践
O-3 主 23:44 干到底 — 落到 v12 增量规范
O-4 主 00:56 任何人都能接手 — 统一规范, 新人能查 R-N 命名/报告命名/commit 命名
```

---

## 🚨 漂移检测

### 漂移点 #1: Commit message 规范 (§6) — R15+ 偏离 v11

**v11 §6 规定**: `<scope>: <subject>` (≤72 字符), 例子 `R14: apeireth-cli session 启动`

**R15+ 实际**: 完整多行格式
- `chuling round15-04 (leader): 最终退出报告` (R15)
- `chuling round16-12 (chuling): 后端系统性验收 - 73/73 PASS` (R16)
- `chuling via mavis: round17-01 (chuling via mavis): R17 重构启动 - 砍掉 NewAPI 依赖, ...` (R17 之前 commit, R17-01 实际)

**偏离原因**: R15+ 团队需要**长格式 commit 写"做了什么 + 验证 + 待办"**, 短格式放不下这么多信息。

### 漂移点 #2: 报告命名规范 (§5) — R16 偏离 v11

**v11 §5 规定**: `r<N>-finalize-<date>` / `r<N>-baseline-verification-<date>` / `r<N>-sec-cross-validation-<date>` (3 种类型)

**R16 实际**: 自创"周报"风格
- `r16-week1-hello-llm-2026-08-03.md` (R16-01)
- `r16-week2-gateway-platform-2026-08-03.md` (R16-04~05)
- `r16-week3-council-llm-2026-08-03.md` (R16-06)
- `r16-week4-real-llm-pass-through-2026-08-03.md` (R16 真 LLM 验证)
- `r16-backend-verification-2026-08-03.md` (R16-12)

**偏离原因**: R16 工作流**按周推进**(Week 1-4 + finalize), 周报形式更清晰。

### 漂移点 #3: R-N 命名空间 (§1) — R15+ 全小写

**v11 §1 规定**: `R-N` 大写 (R11, R12, R13, R14)

**R15+ 实际**: 全小写 `round16-XX` (git commit message), 报告用 `r16-week1-...` 小写 r

**偏离原因**: R15+ 团队沿用 git 社区的 lowercase commit 规范, 跟 v11 §1 大写不一致。

---

## ✅ 不破坏承诺 (v12 升级不动以下 LOCKED)

- ✅ v1-v10 修正链文件名保留 (v12 增量, 不重写)
- ✅ 阶段 1+2+3+4+5 LOCKED 内容不动
- ✅ Cargo.toml `version = "0.14.0"` 不变
- ✅ R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) LOCKED
- ✅ Document-Meta 格式不变 (v12 升 version 字段)
- ✅ v10 版本号系统不变
- ✅ 12 子规范系统结构不变 (v12 在 §1 + §5 + §6 加 R15+ 实践描述, 其他 § 不动)
- ✅ 7 项不修改承诺 不变
- ✅ R-Measure baseline 三值 不变
- ✅ 架构图 P1-P5 不变

---

## 🎯 建议: v11 → v12 增量升级

### 升级内容 (3 处, 全部增量)

**§1 命名空间 (R-N) 加 v12 增补**:
- v11: `R11` / `R12` / `R13` / `R14` (大写)
- v12 增: `R15+` 实践 `round<N>-<NN>` (小写 + round 全拼 + 0 补齐), 沿用 R15/R16 实际

**§5 报告命名 (r-N) 加 v12 增补**:
- v11: `r<N>-finalize-<date>` / `r<N>-baseline-verification-<date>` / `r<N>-sec-cross-validation-<date>` (3 种)
- v12 增: `r<N>-week<X>-<topic>-<date>.md` (R16 实践, 周报风格)
- v12 增: `r<N>-backend-verification-<date>.md` (R16-12 实践, 兼容性保留)
- v12 增: `r<N>-real-key-final-<date>.md` (R16-13 实践, 真 API key 验证)

**§6 commit 规范 加 v12 增补**:
- v11: `<scope>: <subject>` ≤72 字符 (短格式)
- v12 增: `Round-RR (chuling): <一句话标题>` 长格式 (R15+ 实践)
  - 格式: `<scope-Round> (<author>): <subject>`
  - 例子: `round16-12 (chuling): 后端系统性验收 - 73/73 PASS`
- v12 增: `<scope>: <subject>` 短格式保留 (R14 之前用, 仍有效)

### 升级后, 命名规范使用规则

| 类型 | 场景 | 格式 |
|------|------|------|
| 短 commit (R14 及之前) | 简单 commit | `<scope>: <subject>` |
| 长 commit (R15+) | 多行 commit (R15+ 实践) | `round<N>-<NN> (<author>): <subject>` + body |
| 周报 (R16+) | 每周一份 | `r<N>-week<X>-<topic>-<date>.md` |
| 收尾 (R-N 必备) | R 收尾时 | `r<N>-finalize-<date>.md` |
| baseline (R-N 必备) | R 后端系统性验收 | `r<N>-backend-verification-<date>.md` |
| drift 报告 | 规范漂移 | `drift-stage<stage>-<topic>-<date>.md` |
| R-N 命名空间 | R 编号 | `R11` / `R12` / ... / `R17` (大写) + `round17-XX` (commit, 小写) |

---

## 📋 落地清单

| # | 项目 | 状态 |
|---|------|------|
| 1 | 写本 drift 报告 | ✅ (本文件) |
| 2 | 升级 APEIRETH-CONVENTIONS.md 到 v12 (改 §1 / §5 / §6) | ⏳ |
| 3 | R17 业务报告 (按 v12 新规范写) | ⏳ |
| 4 | R17 后续 commit 用 v12 短格式 `<scope>: <subject>` | ⏳ |
| 5 | 文档顶部 README.md 加 v12 索引 | ⏳ (R18+) |

---

## 💡 主哲学 6 锚穿透自检 (再确认)

```
S-1 22:33 北极星导向: ✅ 命名规范服务可接手性, 任何新人查 R17 都能定位
S-2 17:43 实事求是: ✅ 基于 R15+ 实际实践, 不重写 v11
O-5 17:58 不假装: ✅ 把"潜规范"明面化 (v11 没覆盖 R15+)
O-2 19:33 走在前人经验上: ✅ 借鉴 R15+ 团队的命名实践
O-3 23:44 干到底: ✅ 落到 v12 增量规范
O-4 00:56 任何人都能接手: ✅ R-N 命名/报告命名/commit 命名统一
```

---

**作者**: mavis (按主人 2026-08-03 23:33 询问命名规范)
**R17 累计**: 7 笔 commit (R17-01 ~ R17-07), 命名沿用 R15/R16 风格
**R17 漂移**: 1 (本报告)
