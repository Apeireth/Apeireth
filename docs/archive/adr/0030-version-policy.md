# ADR 0030: workspace version 治理 (1.0 release 计划 vs 实际 1.2.0)

> **状态**: 🟢 Accepted (主人 2026-08-15 终极授权 + 自行拍板)
> **最后更新**: 2026-08-15
> **触发**: R174 审计 §Drift 3 发现 3 套 release plan 写过时 version (v1.0.0), 实际是 1.2.0

---

## 1. 背景

| 文档 | 日期 | 声称 version | 实际 version | 状态 |
|------|------|-------------|------------|------|
| `docs/1.0-release/checklist.md` | 2026-08-05 | v1.0.0 (2026-09-30 tag 计划) | 1.2.0 | \u274c \u8fc7\u671f |
| `docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md` | 2026-08-06 | v1.0.0 | 1.2.0 | \u274c \u8fc7\u671f |
| `docs/roadmap/v1.2-release-plan-2026-08-09.md` | 2026-08-09 | v1.2.0 | 1.2.0 | \u2705 \u6b63\u786e |
| `docs/roadmap/v1.0-released-r125-r127-2026-08-10.md` | 2026-08-10 | v1.0.0 (\u542b "已 release" 字样) | 1.2.0 | \u274c \u8fc7\u671f |

**当前 workspace version** (\u6743\u5a01): **1.2.0** (`Cargo.toml [workspace.package] version`)

## 2. \u51b3\u7b56

### 2.1 \u4e25\u5b88 1.2.0 \u4e0d\u52a8

- \u2705 `Cargo.toml [workspace.package] version = "1.2.0"` \u4e25\u5b88, \u672c ADR \u53ea\u6587\u6863\u6cbb\u7406, **\u4e0d\u52a8 Cargo.toml**
- \u26a0\ufe0f \u4e3b\u4eba 2026-08-14 \u62cd\u677f: \u201c\u5e94\u8be5\u4e3a 1.2.0, \u4e0d\u5e94\u8be5\u4e3a 1.0.0\u201d, 0 \u88ab\u6539\u52a8

### 2.2 \u7248\u672c\u8def\u7ebf (per 1.0 release \u62cd\u677f)

| \u9636\u6bb5 | version | \u72b6\u6001 | \u8d1f\u8d23\u4eba |
|----------|---------|----------|-------------|
| \u5f53\u524d (2026-08-15) | 1.2.0 | active | \u4e3b\u4eba + Codex |
| R178+ (2026-08+) | 1.2.x patch | bug fix | Codex |
| \u4e2d\u671f (2026-Q4) | 1.3.0 minor | \u65b0 organ + 桥 | \u4e3b\u4eba + Codex |
| \u957f\u671f (2027-Q1+) | 2.0.0 major | acp \u5347\u7ea7 + \u684c\u5ba0 + \u5546\u4e1a\u5316 | \u4e3b\u4eba |

### 2.3 \u6587\u6863\u8c03\u6574\u7b56\u7565

- \u26a0\ufe0f **\u4e0d\u5220\u9664** v1.0.0 \u8003\u53e4\u8def\u7ebf (\u5386\u53f2\u8bb0\u5f55, per O-2 \u8d70\u5728\u524d\u4eba\u80a9\u4e0a)
- \u26a0\ufe0f **\u4e0d\u6539** R11 / R20 / R22 \u539f\u59cb\u4efb\u4f55\u6587\u6863 (\u4ed6\u4eec\u53cd\u6620\u5386\u53f2)
- \u2705 **\u6dfb\u52a0** \u9876\u90e8\u627f\u8bfa\u5728 v1.0.0-* \u8001\u8003\u53e4\u6587\u6863: 
  ```text
  > **\u67b6\u6784\u9057\u8ff9**: \u672c\u8003\u53e4\u8def\u7ebf\u5199\u4e8e 2026-08-05, \u4ee5 v1.0.0 \u4e3a\u76ee\u6807. 
  > \u5b9e\u9645\u4e8e 2026-08-09 \u62cd\u677f\u4e3a v1.2.0 (per ADR-0030). \u6587\u6863\u4fdd\u7559\u4f5c\u4e3a\u5386\u53f2\u8bb0\u5f55.
  ```
- \u2705 **\u65b0\u589e** \u672c ADR \u4f5c\u4e3a\u552f\u4e00\u7248\u672c\u6743\u5a01\u8868

### 2.4 \u7248\u672c\u68c0\u67e5\u5b88\u95e8 (\u672a\u6765\u9636\u6bb5)

\u5728 `ci/version-check.yml` (\u672a\u521d\u59cb\u5316) \u52a0 1 \u70b9:
- \u8bfb `Cargo.toml [workspace.package] version`
- grep `docs/1.0-release/` `\u201cversion\u201d` \u4e0e `1.2.0` \u4e0d\u4e00\u81f4\u7684\u9879 (\u5141\u8bb8\u5728\u67b6\u6784\u9057\u8ff9\u9876\u90e8)
- grep `docs/roadmap/v1.0.0-*` \u4e0d\u9a8c (\u8003\u53e4)

## 3. \u540e\u679c

### 3.1 \u6b63\u9762

- \u2705 \u4eba\u4efb\u4f55\u4eba\u770b\u4e3b\u4ed3\u5e93: \u270b \u672c ADR \u4f5c\u4e3a\u552f\u4e00\u7248\u672c\u6743\u5a01
- \u2705 \u4e3b\u4eba 8 \u6708\u62cd\u677f\u201c1.2.0\u201d \u662f \u552f\u4e00\u7684 \u9762\u5411\u672a\u6765\u7684\u7248\u672c
- \u2705 \u67b6\u6784\u9057\u8ff9\u9876\u90e8 \u627f\u8bfa \u4fdd\u8bc1\u5386\u53f2\u53ef\u8ffd
- \u2705 \u4e0d\u78b0 Cargo.toml, \u4e0d\u78b0 24 LOCKED crate

### 3.2 \u8d1f\u9762

- \u26a0\ufe0f \u8001\u6587\u6863\u4ecd\u6709\u201cv1.0.0\u201d \u5b57\u6837, \u9700\u624b\u52a8\u8df3\u8fc7\u9996\u9875\u627f\u8bfa
- \u26a0\ufe0f \u672a\u521d\u59cb\u5316 CI \u7248\u672c\u68c0\u67e5

## 4. \u5b9e\u65bd (\u672c session)

1. \u2705 \u521b\u5efa\u672c ADR
2. \u2705 \u5728 `docs/1.0-release/checklist.md` \u9876\u90e8\u52a0\u627f\u8bfa (\u201c\u67b6\u6784\u9057\u8ff9, \u5b9e\u9645 v1.2.0, per ADR-0030\u201d)
3. \u2705 \u5728 `docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md` \u9876\u90e8\u52a0\u627f\u8bfa
4. \u2705 \u5728 `docs/roadmap/v1.0-released-r125-r127-2026-08-10.md` \u9876\u90e8\u52a0\u627f\u8bfa
5. \u2705 \u4e0d\u52a8 Cargo.toml

## 5. \u53c2\u8003

- `Cargo.toml` workspace.package.version (\u6743\u5a01 1.2.0)
- `docs/1.0-release/checklist.md`
- `docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md`
- `docs/roadmap/v1.2-release-plan-2026-08-09.md`
- `docs/roadmap/v1.0-released-r125-r127-2026-08-10.md`
- `docs/audit/R174-comprehensive-audit.md` §Drift 3

---

_\u4f5c\u8005: \u4e3b\u4eba\u62cd\u677f + Codex \u540e\u7aef\u5de5\u7a0b\u5e08_
_\u65e5\u671f: 2026-08-15_
_\u57fa\u7ebf: \u4e3b\u4eba\u7ec8\u6781\u6388\u6743 + \u9ad8\u6743\u9650 + \u81ea\u884c\u62cd\u677f_
