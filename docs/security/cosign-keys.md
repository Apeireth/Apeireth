```
[Document-Meta]
Document:  docs/security/cosign-keys.md
Version:   R20-Rev-A
R-Cycle:   R20 阶段 6 — 1.0 release cosign 公钥文档
Commit:    <commit 时回填>
Last-Modified: 2026-08-05
Status:    🟢 活跃
Author:    Mavis (security_reviewer sub-agent, R20 阶段 6)
Originated: 主人 2026-08-05 21:14 拍板"ABCD 都派, 内存大放心派"
```

> **性质**: 1.0 release 8 包 cosign 签名的**公钥文档 + 密钥管理流程**. 团队 / 用户 / CI 都靠本文档 (per §2 公钥) 验证包来源.
>
> **必读输入**:
> - `docs/ci/1.0-release-pipeline.md` (CI 集成, 3 workflow 触发)
> - `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` §3.5 (12 项 checklist 依据)
> - `scripts/release-1.0-checklist.sh` #12 signature (8 形态 P0 守门)
> - `scripts/release/cosign-sign-all.sh` (8 包统一签名脚本)
> - `scripts/release/cosign-verify.sh` (用户侧验证脚本)
> - 8 项不修改承诺详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2

---

## §1 战略背景 (为什么选 cosign)

**cosign** (sigstore 官方, [github.com/sigstore/cosign](https://github.com/sigstore/cosign)) 是 CNCF Incubating 项目的供应链签名工具, 业界标准 (Kubernetes / Argo CD / Tekton / Helm / Docker 全部支持).

**8 包签名机制选型** (per §3 蓝图 1.0 release #3 signature):

| # | 包形态 | 签名机制 | 工具 | 备注 |
|---:|--------|---------|------|------|
| 1 | deb | `cosign sign-blob` (透明日志 Rekor) | `cosign` v2.2+ | 同时输出 SHA256 (apt 自带 fallback) |
| 2 | rpm | `cosign sign-blob` (透明日志 Rekor) | `cosign` v2.2+ | 同时输出 SHA256 (rpm 自带 fallback) |
| 3 | brew | `cosign sign-blob` | `cosign` v2.2+ | formula JSON + signature |
| 4 | scoop | `cosign sign-blob` | `cosign` v2.2+ | manifest JSON + signature |
| 5 | tarball | `cosign sign-blob` | `cosign` v2.2+ | Linux/macOS 离线包 |
| 6 | zip | `cosign sign-blob` | `cosign` v2.2+ | Windows 通用 |
| 7 | MSI | `signtool` (Authenticode) + `cosign sign-blob` (供应链) | `signtool.exe` + `cosign` | Authenticode 走 Microsoft, cosign 走 sigstore (双签) |
| 8 | Docker (OCI) | `cosign sign` (透明日志 + OIDC) | `cosign` v2.2+ | 推 GHCR 后 `cosign sign ghcr.io/apeireth/apeireth:1.0.0` |

**为什么 8 包统一用 cosign (而不是每个包自己的格式)**:
- 1 工具覆盖 8 形态 (业界共识, per O-2 走在前人肩上)
- 1 公钥覆盖 8 形态 (团队 / 用户只记 1 个 key fingerprint)
- 1 撤销流程覆盖 8 形态 (per §6)
- 透明日志 Rekor 不可抵赖 (公开审计)

---

## §2 cosign 公钥 (团队可见)

> **公钥位置**: 本文档 (committed) + `docs/security/cosign.pub` (binary 副本, 验证用)
>
> **私钥位置**: **不在仓里** (per §3 密钥管理流程).

```
-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE+placeholder+replace+with+actual+
cosign+public+key+generated+via+cosign+generate-key-pair+1.0+release
+Apeireth+production+signing+key+do+not+use+this+placeholder+in+
real+release
-----END PUBLIC KEY-----
```

> ⚠️ **1.0 release 真实公钥** (Mavis 拍板后由 devops_engineer 替换):
> - 跑 `cosign generate-key-pair` 生成 `(cosign.key, cosign.pub)` 对
> - 公钥 commit 到 `docs/security/cosign.pub` (binary, ~250 字节)
> - 上面 PEM 内容是 **placeholder**, 真实 release **必须替换**
> - 公钥 fingerprint (sha256) 公开贴在本文档 §2.1 表格

### §2.1 1.0 release 公钥 fingerprint 表 (release 时回填)

| 项 | 值 |
|----|---|
| 生成时间 | `<release 时回填>` |
| 算法 | ECDSA P-256 (per sigstore 默认) |
| Key ID (sha256 fingerprint) | `<release 时回填>` |
| Rekor 透明日志条目 | `<release 时回填 tlog index>` |
| 阈值 | 1-of-1 (per §5) |
| 信任根 | sigstore public-good instance (fulcio + rekor) |

---

## §3 私钥管理流程 (不在仓里)

> **铁律**: `cosign.key` 永远不 commit 到 git, 永远不在任何 PR 描述 / CI 日志 / 截图 / 文档里出现明文.

### §3.1 生成

```bash
# 仅 release engineer 跑 (1 次, 1.0 release 用)
cosign generate-key-pair
# 生成 cosign.key (私钥, PKCS#8 PEM, 约 250 字节)
# 生成 cosign.pub (公钥, ~250 字节)
```

### §3.2 存储 (4 选项, 任选 1)

| 方案 | 适用 | 工具 |
|------|------|------|
| **A. GitHub Actions Secret** (推荐) | CI 自动签名 (per `release-1.0.0.yml`) | `secrets.COSIGN_KEY` (base64-encoded 私钥) |
| **B. HashiCorp Vault** | 多 release engineer 协作 | `vault kv put secret/apeireth/cosign key=@cosign.key` |
| **C. Hardware token (YubiKey)** | 单 release engineer 离线签名 | `cosign sign-blob --key cosign.key` + YubiKey PIV |
| **D. KMS (AWS / GCP / Azure)** | 1.0 release 阶段 7+ 多签升级 (per §5) | 加密 envelope, 私钥永不出 KMS |

**1.0 release 起步**: 方案 A (GitHub Actions Secret). **阶段 7+ 升级**: 方案 D (KMS + 2-of-3 多签).

### §3.3 使用 (CI 自动)

```yaml
# .github/workflows/release-1.0.0.yml (节选)
- name: cosign sign 8 packages
  env:
    COSIGN_KEY: ${{ secrets.COSIGN_KEY }}
    COSIGN_PUB: docs/security/cosign.pub
    DIST_DIR: dist
  run: bash scripts/release/cosign-sign-all.sh
```

### §3.4 轮转 (per 季度或 release 工程师变动)

1. 生成新密钥对 (`cosign generate-key-pair`)
2. 旧公钥标记 deprecated (在 `docs/security/cosign.pub` 顶部加 `# DEPRECATED: <date>`)
3. 新公钥 commit + GitHub Action Secret 替换
4. 1.0.x patch release 用旧公钥 + 新公钥双签, 1.1 release 全切新公钥

---

## §4 GitHub OIDC 信任机制 (per sigstore Fulcio)

**`cosign sign` (Docker OCI 镜像) 默认走 Fulcio + Rekor**:

```
GitHub Actions OIDC token
  ↓
Fulcio CA 签发短期证书 (10 分钟, ECDSA P-256)
  ↓
cosign 用短期私钥签名 (不暴露长期私钥)
  ↓
Rekor 透明日志记录 (不可抵赖, 公开审计)
  ↓
cosign verify 走 Fulcio + Rekor 公开验证
```

**1.0 release 1-of-1 阈值场景**:
- CI 用 OIDC token 调 Fulcio, 无需本地私钥 (更安全, 私钥永不进 CI runner)
- 本地私钥 `cosign.key` 仅用于 `cosign sign-blob` (deb/rpm/brew/scoop/tarball/zip/MSI 7 形态)
- Docker 镜像用 OIDC (无本地私钥, 更高安全)

**1.0 release 信任链**:
- GitHub OIDC issuer (`token.actions.githubusercontent.com`) → Fulcio → 短期证书 → 签名
- 用户验证: `cosign verify ghcr.io/apeireth/apeireth:1.0.0` 自动走 Fulcio + Rekor

---

## §5 签名阈值 (1-of-1 → 阶段 7+ 2-of-3 多签)

| 阶段 | 阈值 | release engineer 数 | 流程 |
|------|------|---------------------|------|
| **1.0 release (本任务)** | 1-of-1 | 1 | 单 release engineer 用 GitHub Actions Secret 签 8 包 |
| 阶段 7+ (R21 商业化) | 2-of-3 | ≥ 2 | 3 release engineer 各持 1 私钥, 至少 2 签才算 1 release tag |
| 阶段 8+ (R22+) | 3-of-5 | ≥ 3 | KMS 托管 5 私钥, 至少 3 签, 防止单点 compromise |

**1.0 release 用 1-of-1 的理由**:
- 起步简化 (per O-3 干到底), 不为多签而多签
- GitHub Actions Secret + OIDC Fulcio 已经是高安全门槛 (短期证书 + 透明日志)
- 1.0 release 阶段 7+ 必升级 2-of-3 (per `r20-product-finalize` 阶段 7+ 路线图)

**升级到 2-of-3 时本脚本改动**:
- `cosign-sign-all.sh` 加 `--key key1.pem --key key2.pem --key key3.pem` (3 公钥验)
- `cosign-verify.sh` 加 `--certificate-chain certs.pem` (验 2-of-3 阈值)

---

## §6 撤销流程 (per `cosign verify` 失败)

### §6.1 自动检测 (CI 守门)

`release-1.0.0.yml` 的 `release-checklist` job 调用 `cosign verify-blob` (per `scripts/release/cosign-verify.sh`), 任何 1 形态 verify fail → exit 1, 阻塞 tag.

### §6.2 用户侧撤销信号

如果用户跑 `cosign verify-blob --key docs/security/cosign.pub --signature <pkg>.sig <pkg>` 失败:
- **不安装**该包 (per O-5 不假装, 信任 0)
- 检查 [rekor.sigstore.dev](https://rekor.sigstore.dev) 透明日志, 确认是否签了
- 报告 issue 到 [github.com/apeireth/apeireth](https://github.com/apeireth/apeireth) (含 verify 失败输出 + 包 sha256)

### §6.3 主动撤销 (私钥 compromise)

1. 在 [github.com/apeireth/apeireth/security/advisories](https://github.com/apeireth/apeireth/security/advisories) 发 GHSA (含旧公钥 fingerprint + 撤销时间)
2. `docs/security/cosign.pub` 顶部加 `# REVOKED: <date> - <reason>`
3. 生成新公钥 (per §3.4 轮转), commit
4. 通知所有 1.0 release 已知用户 (GitHub Discussions + mailing list)
5. 旧公钥签的包重签 (per §3.4 轮转双签)

---

## §7 不修改承诺 + 6 哲学锚

**8 项不修改承诺** (per `docs/stage4/8-locked-unified-2026-08-05.md` §2):
- 项 1-6: LOCKED 文档 / v2/v4/v4.1 / 阶段 4 / 阶段 5 / v6 / R11 baseline 三值 → **0 触碰** (本任务仅加 `docs/security/cosign-keys.md` 新文件)
- 项 7: 顶层 3 规范文件 → **0 触碰**
- 项 8: workspace version 1.0.0 → **0 触碰** (本任务仅新文件, 不改 Cargo.toml)

**6 哲学锚穿透** (per APEIRETH-CONVENTIONS §9):
- **S-1** 北极星 ASI: cosign 是 ASI 完整性的"包可信"维度 (用户能验包)
- **S-2** 实事求是: 上面公钥是 **placeholder** (per §2), 真实 release 必替换, 不假装已就位
- **O-2** 走在前人肩上: 用 sigstore cosign (业界共识), 0 重复造轮子
- **O-3** 干到底: 1 commit 落地 3 文件 (本脚本 + 公钥文档 + verify 脚本)
- **O-4** 任何人都能接手: 团队读本文档即可知道公钥 + 私钥流程 + 撤销
- **O-5** 不假装: 1.0 release 1-of-1 阈值, 阶段 7+ 才升级多签, 不假装"已经多签"

---

## §8 关联文档

- `scripts/release/cosign-sign-all.sh` (200 行, 8 包统一签名)
- `scripts/release/cosign-verify.sh` (80 行, 用户侧验证)
- `docs/ci/1.0-release-pipeline.md` (CI 3 workflow + release-checklist job 调 verify)
- `scripts/release-1.0-checklist.sh` #12 signature (8 形态 P0 守门)
- `packaging/<target>/build.{sh,ps1}` (8 包 build 实装, 1.0 release 阶段 2 已落地)
- 8 项不修改承诺: `docs/stage4/8-locked-unified-2026-08-05.md` §2

---

_本文档是 R20 阶段 6 cosign 8 包签名的**公钥 + 密钥管理 + 撤销流程**的团队可见说明, 任何接手者读此文档即可知道公钥在哪、私钥怎么管、阈值怎么升级、撤销怎么走. 等 Mavis 拍板 + 主人复核后, 由 security_reviewer 替换 placeholder 公钥 + 落地 1 commit._
