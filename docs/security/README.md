# docs/security/ — 1.0 release 签名与安全材料

> **性质**: 1.0 release 阶段 6 (per `docs/security/cosign-keys.md`) 安全材料汇编.
> 所有 release 工程师 / 集成者 / 用户**只读此目录**即可知道:
> - 公钥在哪 (`cosign.pub`, 本目录)
> - 密钥管理流程 (`cosign-keys.md`, 本目录)
> - 验证流程 (`scripts/release/cosign-verify.sh`)

## 目录文件

| 文件 | 性质 | 用途 |
|------|------|------|
| `cosign.pub` | **二进制公钥** (PEM, 178 B) | 1.0 release 8 包签名公钥 (用户侧 `cosign verify-blob --key` 验证) |
| `cosign-keys.md` | 文档 | 公钥文档 + 私钥管理流程 + 撤销流程 (per §2-§6) |
| `endpoint-inventory-2026-08-06.md` | 文档 | endpoint 资产清单 (per R20 阶段 6) |
| `p2-stub-discrepancy-audit-2026-08-06.md` | 文档 | P2 stub 差异审计 (per R20 阶段 6) |

## TP13 #33 (backlog #27) cosign.pub 落地

- **生成**: 2026-08-18, ECDSA P-256 (a.k.a. prime256v1 / secp256r1)
- **生成方式**: `python -c "from cryptography.hazmat.primitives.asymmetric import ec; ..."` (本地无 cosign CLI, 见 reports/8b9d492b-...-devops_engineer-report.md §3 #33)
- **算法**: ECDSA P-256 (NIST P-256, sigstore cosign 默认)
- **大小**: 178 字节 PEM (SPKI-wrap) / 91 字节 DER (SubjectPublicKeyInfo)
- **SHA256 (DER)**: `f4dec2d54bfe8a0e9010f112198e08fde18890f71f808eab41efea35f69c3208`
- **openssl 验证**: `openssl pkey -pubin -in docs/security/cosign.pub -text -noout` → 256 bit P-256 (Public-Key + NIST CURVE: P-256)

## 验证流程 (用户侧)

```bash
# 1. 拉取公钥 + 任意包签名
curl -fsSL https://raw.githubusercontent.com/apeireth/apeireth/main/docs/security/cosign.pub -o cosign.pub

# 2. 验证 8 包签名 (cosign CLI)
cosign verify-blob --key cosign.pub --signature apeireth-1.0.0-linux-x86_64.tar.gz.sig apeireth-1.0.0-linux-x86_64.tar.gz

# 3. openssl 旁路验证 (无 cosign CLI)
openssl pkey -pubin -in cosign.pub -text -noout
# Expected: 256 bit, P-256 curve, matches SHA256 above
```

## 0 装 PASS 标注

- cosign CLI 未在本机 (`which cosign` → not found)
- 用 Python `cryptography` 库 + OpenSSL 0.99+ 旁路生成的公钥格式与 cosign `generate-key-pair` 输出一致 (PEM SPKI ECDSA P-256)
- 真实 1.0 release 流程: 主人 / release engineer 跑 `cosign generate-key-pair` 替换本 placeholder, 私钥入库 GitHub Actions Secret
- 见 `cosign-keys.md` §2.1 fingerprint 表格 (release 时回填)
