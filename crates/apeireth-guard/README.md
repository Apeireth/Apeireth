# apeireth-guard

> Apeireth Privacy Guard (VCP 模式 3/8 — 隐私卫士): PII 检测 + 脱敏 + 审计。

## 3 模块

| 模块 | 职责 | 公共 API |
|---|---|---|
| `pii` | PII 检测 | `detect_pii` / `PiiKind` / `PiiMatch` |
| `redactor` | 脱敏 | `redact_text` / `redact_one` / `RedactionStrategy` |
| `audit` | 审计 | `AuditLog` / `PrivacyEvent` / `PrivacyAction` / `hash_value_sha256` |

## 借鉴

VCP PrivacyGuard (字段级: 5 类 PII + 4 类脱敏策略 + 审计日志)。

## 设计约束 (不漂移)

- 0 改 VCP 任何内部代码
- 0 副作用: 检测/脱敏是纯函数; 审计是内存 ring buffer
- `#![deny(unsafe_code)]`

## 状态

Apeireth workspace 成员 (81 members, 0 orphan)。

**No-fake**: PII 检测 + 脱敏 + 审计 最小骨架 (R173 阶段 5 落地, 最小骨架而非完整实现)。
**Run-no-fear**: `cargo check --workspace` 0 errors。

## 入口

- `Cargo.toml`: 见 [dependencies](Cargo.toml)
- `src/lib.rs`: 顶部 doc comment 是模块级总览

## 参见

- [Apeireth conventions](../../docs/conventions/README.md)
- [Apeireth 文档归位映射](../../docs/document-relocation-map.md)
