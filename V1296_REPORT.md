# V1296 — Cargo.toml Edition / MSRV / Metadata Hygiene Audit Report

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 20:25 +08:00 2026-08-05)

**VCP 真源代码深读 #17** — Cargo.toml 元数据卫生审计

## Workspace.package 元数据
- edition: `2021`
- rust-version: `1.80`
- license: `Apache-2.0`
- authors count: 1
- repository: `https://github.com/apeireth/apeireth-rust`

## 扫描统计
- Crates scanned: **56**

### Edition inheritance 分布
- `hardcoded`: 8
- `workspace`: 48

### Publish 分布
- `false`: 1
- `missing`: 55

## 假说验证 (5 个)

| ID | 描述 | 观察值 | 阈值 | 结果 |
|----|------|--------|------|------|
| h_workspace_package_fields | workspace.package fields >= 4 (edition/rust-version/license/authors/repository) | 5 | 4 | ✓ PASS |
| h_edition_inheritance | edition.workspace inheritance >= 90.0% | 85.71428571428571 | 90.0 | ✗ FAIL |
| h_rust_version_inheritance | rust-version.workspace inheritance >= 90.0% | 85.71428571428571 | 90.0 | ✗ FAIL |
| h_license_inheritance | license.workspace inheritance >= 90.0% | 85.71428571428571 | 90.0 | ✗ FAIL |
| h_description_coverage | description field coverage >= 90.0% | 100.0 | 90.0 | ✓ PASS |

## V3 哲学守门
- Status: PASS

## 关键免责声明 (主 17:58 + 主 20:46)
- metadata audit ≠ metadata 安全
- PASS ≠ cargo build 成功
- 不假装 ASI V1 = 不刷 KPI = ASI NS 92.91% LOCKED 不变
- FAIL 也诚实披露
- 仅 regex 解析, 不解析 AST
- 纯 read-only, 不调 cargo