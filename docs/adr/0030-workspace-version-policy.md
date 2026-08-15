
# ADR 0030: workspace version 治理 + 跳过 v1.0.0/v1.1.0 直发 v1.2.x

> **状态**: 🟢 Accepted (主人 2026-08-14 终极授权 + 自行拍板)
> **commit 锚**: 本 ADR + `docs/audit/R174-comprehensive-audit.md` §2 Drift 3
> **最后更新**: 2026-08-14 23:05
> **触发**: 全面审计发现 workspace version 三套计划过期

---

## 1. 背景

R20 阶段 6 (8/5) 声称 v1.0.0 tag 计划 2026-09-30 + R21 (8/9) v1.1.0 估补 + R23 (8/13) v1.2.0 release plan. 但:
- 1.0 release checklist (8/5) 写 v1.0.0
- 1.1 release plan (8/9) 写 v1.1.0
- 实际 Cargo.toml `[workspace.package] version = "1.2.0"` (8/14 实查)
- v1.0-released-r125-r127-2026-08-10.md 含 "已 release" 字样但实际未 release

后果:
- 三套 release 计划全过期, 团队无法判断当前目标 version
- 1.0 release 12 项 checklist 已 PASS 但无 release tag
- v1.2.0 在生产但 release notes / GitHub tag 缺

## 2. 决策 (Decision)

### 2.1 workspace version 当前权威

`Cargo.toml [workspace.package] version = "1.2.0"` 是唯一权威 source.

### 2.2 历史 release 计划处理

- ❌ **v1.0.0 (8/5 计划)**: 跳过, 不补 release tag. 历史 12 项 checklist PASS 归档到 `docs/1.0-release/` (R20-Rev-A 状态保留)
- ❌ **v1.1.0 (8/9 计划)**: 跳过, 不补 release tag. 历史 1.1 release plan 归档到 `docs/1.1-release/`
- ✅ **v1.2.0 (8/14 实际)**: 当前权威, 1.2 release plan (`docs/roadmap/v1.2-release-plan-2026-08-09.md`) 是 roadmap, 实查 version 跟它对得上
- 🎯 **下次 version bump**: v1.3.x 估补 (R-3 month roadmap), 走 ADR 留痕

### 2.3 workspace version 治理规则

1. **修改权**: 仅主人 1 句话 + 1 ADR 留痕
2. **修改流程**:
   - 主人 1 句话授权
   - 1 子代理写新 ADR 拍板 (新 version 政策)
   - `Cargo.toml` workspace.package.version 同步更新
   - 全 workspace `git grep "v1.X.0"` 替换新 version
   - 1 R 周期 commit + 0 主动 push
3. **不漂移**: workspace.version 改动 = 0 触碰 24 LOCKED crate + 0 触碰 R11 baseline 三值

### 2.4 文档对齐策略

| 文档 | 当前 | 改后 | 备注 |
|------|------|------|------|
| `docs/1.0-release/checklist.md` | v1.0.0 (2026-09-30 tag 计划) | v1.2.0 (历史归档, 当前权威 v1.2.0) | 加历史声明 |
| `docs/1.0-release/README.md` | 同上 | 同上 | 同上 |
| `docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md` | v1.0.0 | v1.0.0 跳过, 历史归档 | 文件名前加 `_archived_` |
| `docs/roadmap/v1.0-released-r125-r127-2026-08-10.md` | "v1.0.0 已 release" | "v1.2.0 历史归档, v1.0.0 跳过" | 同上 |
| `docs/roadmap/v1.1-release-plan-2026-08-09.md` (在 1.1-release/) | v1.1.0 | 同上 | 同上 |
| `docs/roadmap/v1.2-release-plan-2026-08-09.md` | v1.2.0 (对得上) | 0 改 | 已经是当前权威 |

## 3. 后果

### 3.1 正面

- ✅ workspace version 治理规则明确
- ✅ 跳过 v1.0.0/v1.1.0 历史归档, 0 误判
- ✅ 下次 version bump 走 ADR 留痕
- ✅ 团队接手 1 眼看清当前权威

### 3.2 负面

- ⚠️ 文档 grep 替换 ~10 处 (低风险)
- ⚠️ 历史归档文件名前缀需统一 (`_archived_`)
- ⚠️ v1.0 release 12 项 checklist 历史 PASS 但无 release tag, 团队可能误以为已 release

## 4. 不漂移

- 0 改 workspace version (本 ADR 1.2.0 严守, 仅文档对齐)
- 0 触碰 24 LOCKED crate
- 0 触碰 R11 baseline 3 值

## 5. 6 哲学锚穿透

- ✅ **S-1**: 借鉴 semver (semantic versioning) 业界标准
- ✅ **S-2**: 基于 `Cargo.toml [workspace.package] version = "1.2.0"` 实查, 0 编造
- ✅ **O-2**: 不上 UI, 纯文档对齐
- ✅ **O-3**: §2.4 表格 1 眼看清文档对齐策略
- ✅ **O-4**: §2.3 治理规则让接手者 1 眼明白
- ✅ **O-5**: §3.2 标历史归档副作用

## 6. 8 项不修改承诺

- ✅ 不假装: §2.2 跳过 v1.0.0/v1.1.0 诚实标
- ✅ 编译期 hardcode: workspace.version 编译期常量
- ✅ 不改 LOCKED: 0 触碰
- ✅ 不改 workspace version: 1.2.0 严守
- ✅ 6 哲学锚穿透: §5 自检
- ✅ 不依赖 NewAPI
- ✅ 不重复造轮子: semver 业界
- ✅ 诚实标缺: §3.2

---

_作者: 楚零_
_日期: 2026-08-14_
