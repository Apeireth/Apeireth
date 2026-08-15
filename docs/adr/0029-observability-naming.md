# ADR 0029: observability 命名治理

> **状态**: 🟢 Accepted (主人 2026-08-15 终极授权 + 自行拍板)
> **最后更新**: 2026-08-15
> **触发**: R174 审计 §Drift 2 发现 crate `apeireth-telemetry` vs 内部 `observability` mod vs 文档 37 处 `apeireth-observability/` 三套名并行

---

## 1. 背景

| 名 | 实际 / 引用 | 来源 |
|---|------------|------|
| `apeireth-telemetry` | **crate 名 (Cargo.toml)** | R22 重命名 (从 `apeireth-observability` 改) |
| `apeireth-telemetry::observability` | **内部 mod 路径** | R22 重命名时保留 mod 名 |
| `apeireth-observability` | **37 处文档引用** | R22 改名前的旧文档 |
| `apeireth_api::observability` | api crate 内的 observability mod | 1.0 release 估补 |

**现状**:
- 实际 crate 是 `apeireth-telemetry` v1.40
- 代码内访问用 `apeireth_telemetry::observability::*` (mod 路径)
- 文档大量引用 `apeireth-observability/` (\u226537 处)
- apeireth-api 端点 `GET /v1/observability/{metrics,health,status}` (URL path 仍用 observability)

**后果**:
- 新人接手 grep `apeireth-observability` 找不到 crate (因为是 `apeireth-telemetry`)
- 反之 grep `apeireth-telemetry` 找到 crate 但 mod 路径又是 `observability`
- 文档/代码/URL 三层名不一致

## 2. 决策

### 2.1 权威化三层命名

| 层 | 名 | 说明 |
|----|----|------|
| **crate** | `apeireth-telemetry` | \u2705 Cargo.toml `[package] name` (权威) |
| **mod** | `apeireth_telemetry::observability` | \u2705 Rust mod path (估补) |
| **URL path** | `/v1/observability/{metrics,health,status}` | \u2705 HTTP endpoint path (估补) |
| **env var** | `APEIRETH_TELEMETRY_*` | \u2705 \u73af\u5883\u53d8\u91cf\u524d\u7f00 (估补) |
| **log target** | `telemetry::*` | \u2705 tracing target (\u5728 1.0 release \u52a0) |

### 2.2 文档同步策略

- \u26a0\ufe0f **不删除** `apeireth-observability` \u4efb\u4f55\u5f15\u7528 (\u907f\u514d\u62a5\u9519)
- \u2705 **新增** `docs/observability-naming-cheatsheet.md`: \u4e00\u9875 cheat sheet, \u8d77\u59cb\u8868 \u300c\u4f60\u770b\u5230 X \u540d, \u5b9e\u9645\u5bf9\u5e94 Y \u300d
- \u2705 **保留** 所有 URL path `/v1/observability/*` (估补 HTTP \u8c03\u7528\u8005\u4e60\u60ef)
- \u2705 **保留** mod 路径 `pub mod observability` (估补 1.0 release 锁签名)

### 2.3 \u672a\u6765\u5347\u7ea7\u8def\u5f84 (\u4ec5\u53c2\u8003, \u4e0d\u5728\u672c ADR \u843d\u5730)

\u82e5 R20+ \u9700\u91cd\u547d\u540d, \u63a8\u8350: `apeireth-observability` (跟 mod/URL \u4e00\u81f4), \u4f46\u9700\u540c\u6b65:
1. Cargo.toml `[package] name`
2. 所有 crate `Cargo.toml` `dep = "apeireth-observability"`
3. 所有 `use apeireth_telemetry::*` \u2192 `use apeireth_observability::*`
4. 文档全 grep

\u672c ADR \u4e0d\u6267\u884c\u91cd\u547d\u540d (\u89c4\u6a21\u592a\u5927, R22 \u5df2 ROLLBACK \u4e00\u6b21).

## 3. \u540e\u679c

### 3.1 \u6b63\u9762

- \u2705 **cheat sheet \u4e00\u9875** \u89e3\u51b3 90% \u540d\u5b57\u51b2\u7a81 (\u4eba\u4e5f\u80fd\u770b\u61c2)
- \u2705 \u4ee3\u7801/\u6587\u6863/\u73af\u5883\u53d8\u91cf/\u65e5\u5fd7/\u7aef\u70b9 \u540d\u5b57\u6709\u6743\u5a01\u8868
- \u2705 \u4e0d\u78b0 24 LOCKED crate

### 3.2 \u8d1f\u9762

- \u26a0\ufe0f \u4ecd\u7136 3 \u5c42\u540d\u4e0d\u4e00\u81f4 (\u4f46\u6709\u6743\u5a01\u8868, \u4eba\u4e5f\u80fd\u770b\u61c2)
- \u26a0\ufe0f \u672a\u6765\u5347\u7ea7\u9700\u53cc\u8f68\u4e2d\u95f4\u5c42

## 4. \u5b9e\u65bd

### 4.1 \u672c ADR \u843d\u5730 (\u672c session)

1. \u2705 \u521b\u5efa `docs/observability-naming-cheatsheet.md`
2. \u2705 \u5728 `docs/audit/R174-comprehensive-audit.md` §Drift 2 \u52a0\u4e00\u884c: "per ADR-0029"
3. \u2705 \u4e0d\u78b0 24 LOCKED crate

### 4.2 \u672a\u6765\u53ef\u9009

- R22+ \u91cd\u547d\u540d (R22 \u5df2\u91cd\u547d\u540d\u4e00\u6b21, \u518d\u91cd\u8d77\u52a8\u5f15\u53d1\u5fc5\u8981)
- URL path `/v1/telemetry/*` (R20+ \u53ef\u8003\u8651, \u4f46\u4f1a BREAK 1.0 release)

## 5. \u53c2\u8003

- `crates/apeireth-telemetry/Cargo.toml` (\u6743\u5a01 crate \u540d)
- `crates/apeireth-telemetry/src/lib.rs` (mod 路径)
- `crates/apeireth-api/src/observability/` (URL path)
- `docs/audit/R174-comprehensive-audit.md` §Drift 2
- `docs/backend-capabilities.md` §2.1 (\u5f15\u7528\u7684 3 \u4e2a\u540d)

---

_\u4f5c\u8005: \u4e3b\u4eba\u62cd\u677f + Codex \u540e\u7aef\u5de5\u7a0b\u5e08_
_\u65e5\u671f: 2026-08-15_
_\u57fa\u7ebf: \u4e3b\u4eba\u7ec8\u6781\u6388\u6743 + \u9ad8\u6743\u9650 + \u81ea\u884c\u62cd\u677f_
