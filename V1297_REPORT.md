# V1297 — Cargo Feature Flag Audit Report

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 21:16 +08:00 2026-08-05)

**VCP 真源代码深读 #18** — Cargo.toml feature flag 维度审计

## 扫描统计
- workspace.dependencies count: **17**
- crates scanned: **56**
- crates with [features]: **6** (10.71%)
- total feature combinations (estimate): **66**
- duration: **24 ms**

## Workspace.dependencies with features

| dep | version | features | default_features |
|-----|---------|----------|------------------|
| tokio | 1.40 | full | default |
| serde | 1.0 | derive | default |
| reqwest | 0.12 | json, rustls-tls, stream | disabled |
| pyo3 | 0.22 | auto-initialize | default |
| rusqlite | 0.32 | bundled | default |
| chrono | 0.4 | serde | default |
| uuid | 1.10 | v4, serde | default |
| criterion | 0.5 | html_reports | default |

## Crates with [features] block

| crate | default | features | uses dep: | hardcoded version |
|-------|---------|----------|-----------|-------------------|
| apeireth-memory | (empty) | semantic | no | no |
| apeireth-central | (empty) | testing | no | no |
| apeireth-pybridge | (empty) | python-ext | yes | no |
| apeireth-bus | (empty) | full-bus | no | no |
| apeireth-web | ssr | ssr | yes | no |
| apeireth-graph | (empty) | supervisor-integration | yes | no |

## 假说验证 (6 个)

| ID | 描述 | 观察值 | 阈值 | 结果 |
|----|------|--------|------|------|
| h_workspace_deps_with_features | workspace.dependencies 显式 features >= 5 | 8 | 5 | ✓ PASS |
| h_crates_with_features_section | crates with [features] 占比 <= 25.0% | 10.714285714285714 | 25.0 | ✓ PASS |
| h_dep_prefix_usage | 用 dep: 前缀占比 >= 50.0% | 50.0 | 50.0 | ✓ PASS |
| h_default_empty_dominant | default = [] 占比 >= 50.0% | 83.33333333333334 | 50.0 | ✓ PASS |
| h_no_hardcoded_version_in_features | [features] 块内无 hardcoded 版本字符串 | 0 | 0 | ✓ PASS |
| h_pyo3_not_in_default | pyo3/python-ext 不在 default features (无 Python 也能 build) | 0 | 0 | ✓ PASS |

## V3 哲学守门
- Status: PASS

## 关键免责声明 (主 17:58 + 主 20:46)
- feature audit ≠ feature 安全
- PASS ≠ cargo build 成功
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变
- FAIL 也诚实披露
- 仅 regex 解析, 不解析 AST
- 纯 read-only, 不调 cargo
- features 是 API 暴露面, 不是漏洞
- `dep:` 前缀是 2021 edition 推荐, 不是强制
- audit ≠ fix, 仅审计不修改
