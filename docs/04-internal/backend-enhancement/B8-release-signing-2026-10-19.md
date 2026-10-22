# B8 发布签名与来源证明（安全隐患 #8「安装包来源验证不足」）— 批次登记

> `Status: ✅ 本批范围已交付（2026-10-19 开工，2026-10-20 跨日收尾实测）`
> 战场：`scripts/` + `packaging/` + `docs/`（战场外 0 触碰）。**未 commit**，等主代理审查统一提交。
> 0 装口径：验不了的一律写「未验证/排期」，未签名不装签名。

## 1. 隐患与修法

外部审计口径（主代理转述）：本机安装目录核心 exe 无 Authenticode 签名；发布页有
SHA-256 校验材料但未验证过本机安装包；只能确认版本号与源码标签一致，**不能证明
二进制来自该源码提交**。属供应链与发布流程问题。本批按「补校验环节 + 排期登记」修：

1. **发布资产校验脚本**（新增，见 §2）：哈希比对 PASS/FAIL + 签名状态如实报；
2. **发布流程文档**（新增，见 §2）：发布者侧（构建→清单→自检→发布）+ 用户侧（下载→校验→安装）；
3. **排期待办**（登记不实施，见 §4）：代码签名证书接入、可复现构建证明。

## 2. 改动清单

| 文件 | 动作 | 内容 |
|---|---|---|
| `scripts/verify-release-assets.ps1` | 新增 | 发布资产校验器：SHA-256 哈希比对（`SHA256SUMS` 清单 / `.sha256` sidecar / `-ExpectedSha256` 直给三种口径）+ `Get-AuthenticodeSignature` 状态如实报（未签名显示「未签名」）+ `-RequireSignature` 附加门槛（签名非有效即失败）+ 缺项/不一致明确 FAIL（缺项不装过）。UTF-8 BOM（对齐 `check-neutral-terms.ps1` 用法注记的 5.1 口径） |
| `docs/packaging/release-asset-verification.md` | 新增 | 「发布资产校验与签名」流程：边界如实表（能证明/不能证明）、发布者侧 5 步、用户侧校验命令、其他平台等价口径、已知边界与排期表、关联材料 |
| `packaging/README.md` | 一行挂链 | 文末追加 verification 文档链接（叠加不覆盖，其余未动） |
| `docs/04-internal/backend-enhancement/B8-release-signing-2026-10-19.md` | 新增 | 本批次登记（含排期待办；`00-backlog.md` 按令不碰） |

**0 引新外部依赖、0 改 Rust、0 改既有打包/清单脚本**（`packaging/*/build*.ps1` 与
`generate-release-manifest.ps1` 的 `.sha256`/`SHA256SUMS` 产出行为是既有能力，只读对账）。

## 3. 实测门禁（2026-10-20，Windows，pwsh 7）

测试输入：真二进制 `target/release/apeireth.exe`（复制为 `apeireth-2.0.0-rc.4-windows-x86_64.exe`，
实算 SHA-256 `402b4726…beef4e4`）+ 构造清单（正/反/缺项/无清单四种）。

| # | 用例 | 期望 | 实测 |
|---|---|---|---|
| 1 | 正确清单（SHA256SUMS） | PASS exit 0 | ✅ PASS exit 0，哈希逐字一致 |
| 2 | 哈希不一致（全零期望） | FAIL exit 1 | ✅ FAIL exit 1，「不要安装」 |
| 3 | 清单缺项 | FAIL exit 1 | ✅ FAIL exit 1，「缺项不装过」 |
| 4 | 同目录 `.sha256` sidecar 自动查找 | PASS exit 0 | ✅ PASS exit 0 |
| 5 | `-ExpectedSha256` 直给 | PASS exit 0 | ✅ PASS exit 0 |
| 6 | `-RequireSignature`（真二进制未签名） | FAIL exit 1 | ✅ FAIL exit 1，签名如实报「未签名」 |
| 7 | 参数不合法（短哈希） | FAIL exit 1 | ✅ FAIL exit 1，拒绝不猜 |

签名状态面（用例 1–6 全部如实显示）：`未签名（如实报告：无任何 Authenticode 签名）`
——与审计口径一致（核心 exe 无签名），未假装。PASS 结果行固定带边界句：
「哈希一致 ≠ 二进制来源可证（可复现构建证明 = 排期待办）」。

- `scripts/check-neutral-terms.ps1`：0 命中（新增行全中性；`Authenticode`/`SHA-256` 为标准名）。
- Rust 三件套（test/clippy/fmt）：**不适用**——本批 0 触碰 `.rs`/`Cargo.toml`。
- 前端 `pnpm check` / `pnpm test`：**不适用**——本批 0 触碰 `frontend/`。

## 4. 排期待办清单（登记不实施，owner 决策）

| # | 事项 | 现状 | 为什么本批不做 |
|---|---|---|---|
| P1 | 代码签名证书接入（对 exe/MSI 签名 + 签名状态入 `releaseon`） | 未交付 | 证书采购/托管/时间戳服务是 owner 决策与外部支出，登记不实施 |
| P2 | 可复现构建证明（二进制←源码提交的证明链） | 未交付 | 需确定性构建链与构建环境固化方案，独立批立项 |
| P3 | 其他平台（Linux/macOS）校验链实测 | 未实测 | 本批 Windows 单平台；文档已给等价命令，未跑过不称已验证 |
| P4 | 清单与安装包分通道发布的通道方案 | 仅文档建议 | 发布渠道由 owner 拍板 |

## 5. 供应链挂账对账（只读，口径不动）

- `deny.toml`（依赖许可证允许表 / advisories ignore）与 `audit.toml`（advisory ignore，
  三处同步纪律）覆盖**依赖源码供应链**（准入/漏洞公告/许可证），与本批的**发布物
  来源证明**是相邻但不同的两条面；本批只引用不改（对齐口径如上）。
- `scripts/release/`（1.0 时代产物签名脚本等）按其 README 标注保留为历史产物，非当前入口。
- 依赖侧无新增：0 引新外部依赖 = deny/audit 口径无涉。

## 6. 纪律自查

- **LOCKED 5 项 0 触碰**：9 哲学锚本体 / 13 键 / 3 不可变脊柱 / `workspace.version` / R11 baseline——本批未涉。
- **0 装 PASS**：未签名如实「未签名」；不可复现构建/无证书一律「排期/未验证」；无假旋钮
  （脚本对缺清单/缺项直接 FAIL，不给「大概通过」）。
- **中性表述**：零第三方产品名（`Authenticode`/`SHA-256`/MSI 为标准名；既有文件名引用走真实路径口径）。
- **叠加不覆盖**：`packaging/README.md` 仅追加一行挂链；既有脚本 0 改。
- **不 commit**：交主代理审查后统一提交。
