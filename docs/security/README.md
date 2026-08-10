# docs/security/ — 安全

```
[Document-Meta]
Document:       docs/security/README.md
Version:        R119-4a
R-Cycle:        R119 (文档体系推倒重建)
Last-Modified:  2026-08-10
Status:         🟢 索引层
```

> **性质**: 安全相关文档。3 份: cosign 密钥管理 + 端点清单 + 1.0 release P2 stub 偏差审计。

---

## 索引

| 文档 | 性质 | 字节 |
|---|---|---|
| [`cosign-keys.md`](cosign-keys.md) | cosign (sigstore) 签名密钥管理 (1.0 release 5 项 #6 sec 配套) | 10,590 |
| [`endpoint-inventory-2026-08-06.md`](endpoint-inventory-2026-08-06.md) | 端点清单 (4 协议 + 4 端口 + 鉴权) | 2,393 |
| [`p2-stub-discrepancy-audit-2026-08-06.md`](p2-stub-discrepancy-audit-2026-08-06.md) | 1.0 release P2 stub 偏差审计 | 3,505 |

---

## 何时看

- **cosign 签 / verify** → `cosign-keys.md` (release artifact 签名)
- **网络端点配** → `endpoint-inventory-2026-08-06.md`
- **1.0 release #6 sec 100% 验证** → [`docs/1.0-release/security-audit.md`](../1.0-release/security-audit.md)
- **rustsec / cargo-audit CI** → `.github/workflows/cargo-audit.yml` (R57-R62 followup-2 加入)

---

## 相关入口

- 1.0 release #6 sec checklist: [`docs/1.0-release/security-audit.md`](../1.0-release/security-audit.md) (12 项 #6 sec 100% 收口)
- 1.0 release #7 sec cosign CI: [`reports/1.0-release-security-cosign-ci-2026-08-06.md`](../../reports/1.0-release-security-cosign-ci-2026-08-06.md)
- 1.0 release #7 sec rustsec: [`reports/1.0-release-security-rustsec-r21-2026-08-06.md`](../../reports/1.0-release-security-rustsec-r21-2026-08-06.md)
- 1.0 release #7 sec 100% 收口: [`reports/1.0-release-security-100-2026-08-06.md`](../../reports/1.0-release-security-100-2026-08-06.md)
