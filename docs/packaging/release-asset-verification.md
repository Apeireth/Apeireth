# 发布资产校验与签名（来源验证环节）

> **状态（0 装口径，2026-10-20）**：本环节已交付 = **SHA-256 清单校验脚本 + 本文流程**（Windows 实测）；
> **代码签名证书接入** 与 **可复现构建证明** = **排期待办（未交付）**，见 [§4](#4-已知边界与排期待办)。
> 适用面：Windows 发布物（本批实测平台）；其他平台的等价校验见 [§3.4](#34-其他平台与手工等价比对)。
> 台账与实测记录：[B8-release-signing-2026-10-19.md](../04-internal/backend-enhancement/B8-release-signing-2026-10-19.md)。

## 1. 本环节解决什么（边界如实）

| 能证明 | 不能证明 |
|---|---|
| 「下载到的文件」=「发布清单记载的文件」（SHA-256 逐字节比对） | 「二进制由某个源码提交构建而来」——需可复现构建证明（**排期待办**） |
| 平台签名状态（有效 / 未签名 / 校验失败，**如实外显**） | 「没签名 = 不安全」或「有签名 = 一定安全」的任何推断 |
| 清单缺项 / 哈希不一致时**明确失败**（缺项不装过） | 清单本身来自可信通道（由使用者负责，建议与安装包分通道获取） |

**0 装红线**：未签名就是「未签名」，脚本与本文都不写成「已签名/受保护」；哈希一致也不写成「来源已证明」。

## 2. 发布者侧（构建 → 清单 → 自检 → 发布）

1. **构建**：`packaging/` 各构建脚本（`zip/` `msi/` `desktop/` 等）产出安装包/压缩包，
   每个产物旁自带 `.sha256` sidecar（构建脚本既有行为）。
2. **汇总清单**：`scripts/generate-release-manifest.ps1` 扫描 `dist/` 生成
   `SHA256SUMS`（每行 `<哈希>  <文件名>`）与 `releaseon`（产物清单 JSON：版本 /
   提交号 / 产物分类 / 逐件哈希；文件名沿用生成脚本现名）。
3. **发布前自检（本批新增）**：对每个产物跑

   ```powershell
   pwsh -File scripts/verify-release-assets.ps1 -ArtifactPath dist\<产物> -ManifestPath dist\SHA256SUMS
   ```

   哈希全部一致（exit 0）才进入发布；任何 FAIL（不一致/缺项）即停，先查清再发。
4. **随包发布**：`SHA256SUMS`（与 `releaseon`）与安装包同页发布，供用户侧校验。
5. **签名步骤（排期待办）**：代码签名证书接入后，在本步对 exe/MSI 签名并把签名
   状态写入 `releaseon`。**当前发布物未签名，如实标注，不假装**。

## 3. 用户侧（下载 → 校验 → 再安装）

### 3.1 拿到两样东西

安装包本体 + 官方 `SHA256SUMS`（发布页同处提供）。建议清单与安装包**分通道**获取
（例如安装包走下载链接、清单另从发布页文本区复制），降低两者同时被替换的风险。

### 3.2 一键校验（推荐）

```powershell
pwsh -File scripts/verify-release-assets.ps1 -ArtifactPath <下载的安装包> -ManifestPath <下载的 SHA256SUMS>
```

输出逐件报告三样事实：**期望哈希 / 实算哈希 / 签名状态**。看结论行即可：

- `RESULT: PASS` — 哈希与官方清单一致；签名状态照实显示（当前发布物显示「未签名」属常态，如实记载）；
- `RESULT: FAIL` — 哈希不一致、清单缺项或门槛未过，**不要安装**；
- 想把「必须有有效签名」当硬门槛：加 `-RequireSignature`（签名非有效即失败）。

若下载物旁已带 `.sha256` sidecar，可省略 `-ManifestPath`（自动查找同目录 sidecar）。

### 3.3 手工等价比对（不跑脚本时）

```powershell
Get-FileHash -Path <下载的安装包> -Algorithm SHA256   # 与发布页 SHA256SUMS 中该文件行逐字比对
Get-AuthenticodeSignature -Path <下载的安装包>        # 签名状态如实看（NotSigned = 未签名）
```

### 3.4 其他平台与手工等价比对

Linux / macOS 用 `sha256sum -c SHA256SUMS`（或 `shasum -a 256`）比对同一份清单；
签名状态校验面为 Windows 专有，其他平台**不适用**（如实说明，不装有）。

## 4. 已知边界与排期待办

| 事项 | 现状（0 装口径） | 备注 |
|---|---|---|
| SHA-256 清单校验 | **已交付并实测**（2026-10-20，Windows；正反两路 7 用例见批次文档） | `scripts/verify-release-assets.ps1` |
| 签名状态如实报 | **已交付并实测**（未签名如实显示；`-RequireSignature` 门槛实测生效） | 平台校验面转述，不改文件 |
| 发布物代码签名 | **未签名（既成事实，如实）** | 审计口径：本机安装目录核心 exe 无 Authenticode 签名 |
| 代码签名证书接入 | **排期待办（未交付）** | owner 决策（证书采购/托管/时间戳服务），登记不实施 |
| 可复现构建证明 | **排期待办（未交付）** | 哈希只证明「文件=清单」；「二进制←源码提交」的证明链未建立 |
| 其他平台校验链 | **未实测** | 本批只在 Windows 实测；Linux/macOS 命令按 §3.4 等价，未跑过不称已验证 |

## 5. 关联材料

- 校验脚本：[scripts/verify-release-assets.ps1](../../scripts/verify-release-assets.ps1)
- 清单生成：[scripts/generate-release-manifest.ps1](../../scripts/generate-release-manifest.ps1)（`SHA256SUMS` + `releaseon`）
- 构建侧 `.sha256` sidecar：`packaging/zip/build.ps1`、`packaging/msi/build.ps1`、`packaging/desktop/build-desktop.ps1` 等（既有行为，本批未改）
- 安装生命周期：[windows-packaging-lifecycle.md](windows-packaging-lifecycle.md)
- 依赖供应链（依赖准入 / 漏洞公告 / 许可证）是另一条线：`deny.toml` / `audit.toml`（本批只读对账、口径不动）
- 1.0 时代产物签名脚本存于 `scripts/release/`（历史产物，非当前入口，见该目录 README）
