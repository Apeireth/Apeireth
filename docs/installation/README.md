# docs/installation/ — 多平台安装

```
[Document-Meta]
Document:       docs/installation/README.md
Version:        R119-4a
R-Cycle:        R119 (文档体系推倒重建)
Last-Modified:  2026-08-10
Status:         🟢 索引层
```

> **性质**: 三平台 6 格式安装文档 + 包对比。R20 阶段 6 1.0 release 时期留下。

---

## 索引

| 平台 | 包格式 | 文档 |
|---|---|---|
| **Windows** | Scoop | [`windows-scoop-install.md`](windows-scoop-install.md) |
| **macOS** | Homebrew | [`macos-brew-install.md`](macos-brew-install.md) |
| **Linux (Debian/Ubuntu)** | .deb | [`deb-install.md`](deb-install.md) |
| **Linux (Fedora/RHEL)** | .rpm | [`rpm-install.md`](rpm-install.md) |
| **Linux (通用)** | .tar.gz | [`linux-tarball-install.md`](linux-tarball-install.md) |
| **跨平台对比** | 5 格式 | [`package-comparison.md`](package-comparison.md) |

---

## 何时看

- **装 apeireth 客户端** → 选上面对应平台/格式
- **选包格式** (个人开发 vs 团队部署) → `package-comparison.md`
- **5 包 100% PASS 验证** (1.0 release 阶段) → [`docs/1.0-release/install-status.md`](../1.0-release/install-status.md)
- **CI 真测 5 包** → [`reports/1.0-release-install-5pkg-k1-check-2026-08-05.md`](../../reports/r20-1.0-install-5pkg-k1-check-2026-08-05.md)

---

## 顶层入口

- 顶层 `INSTALL.md` 链这里 (1.0 release 收口后)
- R20 install 验证: [`reports/r20-1.0-install-5pkg-k1-check-2026-08-05.md`](../../reports/r20-1.0-install-5pkg-k1-check-2026-08-05.md)
