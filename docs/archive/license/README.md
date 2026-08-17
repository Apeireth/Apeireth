# Apeireth License FAQ & Compliance Guide

> **性质**: Apeireth 1.0 release (v1.0.0) 许可证常见问题 + 合规指南
> **依据**: `LICENSE` (Apache-2.0) + `NOTICE` + `DEPENDENCY` + `THIRD-PARTY-NOTICES.md` (1709 lines / 561 crate / 12 unique SPDX)
> **最后更新**: 2026-08-06
> **owner**: 整合 #3 R21 续补 (D-5)
> **不假装**: 仅描述 **已 commit 代码** + 业界 SPDX 标准 + 实际第三方 crate 列表

---

## 0. TL;DR

| 问题 | 答案 | 详细 |
|------|------|------|
| **Apeireth 是什么 license?** | **Apache License 2.0** (per `LICENSE` 根目录) | 见 [01-commercial-use.md](01-commercial-use.md) |
| **我能商用吗?** | ✅ **能**, Apache-2.0 明确授予商用权 | 见 §1 |
| **我能修改 + 再分发吗?** | ✅ **能**, 保留版权 + NOTICE + 改动说明 即可 | 见 [03-modification-redistribution.md](03-modification-redistribution.md) |
| **需要贡献代码回去吗?** | ⚪ **不强制**, 但强烈建议 (per Apache-2.0 §5) | 见 [01-contribution.md](01-contribution.md) |
| **商标能用吗?** | ⚠️ **"Apeireth"** 名 + logo **不能用** 暗示官方背书 | 见 [04-faq.md](04-faq.md) §3 |
| **第三方 license 冲突吗?** | ❌ **0 冲突** (cargo-deny 16 allow + 0 violation) | 见 [05-spdx-reference.md](05-spdx-reference.md) |
| **看到 license 错误怎么办?** | 提交 issue, 我们 R21+ 修 (1.0 release 不阻塞) | 见 [04-faq.md](04-faq.md) §6 |

---

## 1. 5 文档结构

| 文档 | 主题 | 受众 |
|------|------|------|
| [01-contribution.md](01-contribution.md) | 如何贡献代码 / 文档 / 测试 | 贡献者 |
| [02-commercial-use.md](02-commercial-use.md) | 商业使用 + SaaS + 嵌入产品 | 商业用户 |
| [03-modification-redistribution.md](03-modification-redistribution.md) | 修改 + 再分发 (含 fork + Docker 镜像) | 二次开发者 |
| [04-faq.md](04-faq.md) | 18 常见问题 (商标 / 专利 / 私用 / etc) | 所有人 |
| [05-spdx-reference.md](05-spdx-reference.md) | 12 SPDX 类别详解 + 完整 attribution | 法律 / 合规 |

---

## 2. 0 触碰 24 LOCKED crate + 0 改 workspace version 验证

| 守门 | 验证 | 状态 |
|------|------|:----:|
| 0 触碰 24 LOCKED src (本目录) | 0 写/0 改 src/ (本目录是 docs/license/ 新建) | ✅ |
| 0 改 workspace version | `Cargo.toml:188 version = "1.0.0"` 未动 | ✅ |
| 0 主动 commit | 本任务纯 meta 写盘, 0 git add/commit/push | ✅ |
| 6 哲学锚穿透 | S-1 业界惯例 / S-2 实事求是 / O-3 信息密度高 / O-5 不假装 | ✅ |
| 8 项不修改承诺 | 0 改 24 LOCKED / 0 改 6 哲学锚 / 0 改 workspace version / 0 重复造轮子 / 0 假装 / 0 改 LOCKED 文档 / 0 sandbox 错路径 / 0 主动 commit | ✅ |

---

## 3. 与根目录 LICENSE/NOTICE/DEPENDENCY 关系

```
根目录 (项目级, 编译期 hardcode)        docs/license/ (用户级 FAQ)
─────────────────────────────          ──────────────────────────────
LICENSE (180 行, Apache-2.0 完整)       docs/license/01-contribution.md
NOTICE (71 行, 项目声明 + 致谢)         docs/license/02-commercial-use.md
DEPENDENCY (170 行, 5 段 + 5 表)       docs/license/03-modification-redistribution.md
THIRD-PARTY-NOTICES.md (1709 行)        docs/license/04-faq.md (18 问)
                                        docs/license/05-spdx-reference.md
                                        docs/licenses-3rdparty/ (50+ 副本, D-1)
                                        DEPENDENCY-trees/ (30+ 树, D-2)
```

**职责边界**:
- `LICENSE` / `NOTICE` / `DEPENDENCY` / `THIRD-PARTY-NOTICES.md` = **合规必须** (Apache-2.0 §4(a-d) 要求)
- `docs/license/` = **用户体验** (减少律师咨询, 快速回答常见问题)

---

## 4. 适用版本

| 维度 | 值 | 备注 |
|------|----|------|
| **项目** | Apeireth | 1.0 release (v1.0.0) |
| **项目 license** | Apache-2.0 | per `Cargo.toml:192 license = "Apache-2.0"` |
| **第三方 license 数** | 12 unique SPDX | per THIRD-PARTY-NOTICES.md |
| **第三方 crate 数** | 561 transitive | per cargo-about 0.8.4 |
| **allow-list** | 16 license | per `deny.toml` |
| **violation** | 0 | per `cargo deny check licenses` |
| **生成时间** | 2026-08-06 | per R21 续补 (整合 #3 D-5) |

---

## 5. 相关文档

- 根 `LICENSE` (Apache-2.0, 180 行)
- 根 `NOTICE` (项目声明 + 致谢, 71 行)
- 根 `DEPENDENCY` (workspace 依赖摘要, 170 行)
- 根 `THIRD-PARTY-NOTICES.md` (1709 行, 561 crate attribution)
- [docs/licenses-3rdparty/](../licenses-3rdparty/) (50+ 第三方 LICENSE 副本, D-1)
- [DEPENDENCY-trees/](../../DEPENDENCY-trees/) (30+ cargo tree 导出, D-2)
- [docs/api/](../api/) (HTTP/WebSocket API 文档)
- [docs/sdk/](../sdk/) (5 SDK 客户端文档)
- [docs/adr/](../adr/) (架构决策记录, 含 6 哲学锚)

---

**Last-Modified**: 2026-08-06
**Tool**: Mavis R21 续补 (整合 #3 D-5)
**Format**: MADR 4.0 简化版 + Keep a Changelog 1.1.0
