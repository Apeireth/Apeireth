# V1295 — Cargo.lock Lockfile Audit

**Workspace root**: `.openclaw\workspace\promethean\Apeireth-rust`
**Lockfile**: `.openclaw\workspace\promethean\Apeireth-rust\Cargo.lock`
**Duration**: 7 ms

## Summary

- **Lockfile version**: 3
- **Lockfile size**: 6067 lines / 142621 bytes
- **Total packages**: **567**
- **Internal (apeireth-*)**: **46** (8.11%)
- **External**: **521** (91.89%)
- **Distinct sources**: **1**
- **Checksum coverage**: **100.00%** (521/521)
- **With dependencies**: **391**
- **Yanked=true packages**: **0**
- **Multi-version crates**: **37** (7.21% of distinct names)

## Distinct Sources

- `registry+https://github.com/rust-lang/crates.io-index`

## Hypotheses (主 17:43 实事求是)

- ✅ **H1_checksum_full** — Checksum 覆盖率 ≥ 99% (几乎所有包都有 SHA256) → checksum 覆盖完整
  - detail: checksum_coverage=100.00% (521/521 with source) (expected >= 99.0%)
- ✅ **H2_internal_complete** — Internal (apeireth-*) 包 ≥ 40 (workspace 47 - 1 commented = 46 期望) → workspace members 全部 in lock
  - detail: n_internal=46 (expected >= 40)
- ✅ **H3_no_yanked** — 无 yanked=true 包 (Cargo.lock v3 + Rust 1.74+ 时才能查到) → 无 yanked package
  - detail: n_yanked_true=0 (yanked_field_seen_count=0, lockfile_v3=yes) (expected 0)
- ✅ **H4_lockfile_compact** — Lockfile ≤ 10000 行 (compact) → lockfile 紧凑
  - detail: lockfile_lines=6067 (expected <= 10000)
- ✅ **H5_multi_version_low** — Multi-version crate < 10% (semver major 一般允许多版本) → multi-version 受控
  - detail: multi_version_crates=37 (7.21% of distinct names, expected < 10.0%)
- ✅ **H6_source_diversity** — ≥ 1 distinct source (crates.io / git / path) → 有 source 注册
  - detail: n_distinct_sources=1 sources=['registry+https://github.com/rust-lang/crates.io-index'] (expected >= 1)
- ✅ **H7_no_workspace_drift** — 所有 workspace members 都在 lock (无 drift) → lock 与 workspace 一致
  - detail: workspace_members_missing_from_lock=0 out of 46 (expected 0)

## Top-10 Most-Referenced External Crates (in-degree)

| crate | version | referenced_by | n_ref |
|---|---|---|---:|
| serde | 1.0.229 | ahash, apeireth-action, apeireth-agent, apeireth-api, apeireth-asi ... (+78) | 83 |
| tokio | 1.53.1 | any_spawner, apeireth-action, apeireth-agent, apeireth-api, apeireth-asi ... (+56) | 61 |
| serde_json | 1.0.151 | apeireth-action, apeireth-agent, apeireth-api, apeireth-asi, apeireth-bench ... (+54) | 59 |
| thiserror | 1.0.69 | any_spawner, apeireth-action, apeireth-agent, apeireth-api, apeireth-asi ... (+53) | 58 |
| quote | 1.0.47 | async-stream-impl, async-trait, attribute-derive, attribute-derive-macro, axum-macros ... (+46) | 51 |
| proc-macro2 | 1.0.107 | async-stream-impl, async-trait, attribute-derive, attribute-derive-macro, axum-macros ... (+45) | 50 |
| syn | 2.0.119 | async-stream-impl, async-trait, attribute-derive, attribute-derive-macro, axum-macros ... (+41) | 46 |
| libc | 0.2.189 | android_system_properties, cpufeatures, cpufeatures, errno, filetime ... (+33) | 38 |
| tracing | 0.1.44 | apeireth-agent, apeireth-api, apeireth-http-client, apeireth-mcp, apeireth-mcp-relay-image ... (+23) | 28 |
| bytes | 1.12.1 | apeireth-api, apeireth-mcp, apeireth-pipeline, axum, axum-core ... (+23) | 28 |

## Multi-Version Crates (ABI drift 风险)

| crate | versions | n_distinct_major |
|---|---|---:|
| bitflags | 1.3.2, 2.13.1 | 2 |
| http | 0.2.12, 1.5.0 | 2 |
| indexmap | 1.9.3, 2.14.0 | 2 |
| mio | 0.8.11, 1.2.2 | 2 |
| r-efi | 5.3.0, 6.0.0 | 2 |
| rustix | 0.38.44, 1.1.4 | 2 |
| serde_spanned | 0.6.9, 1.1.1 | 2 |
| syn | 2.0.119, 3.0.3 | 2 |
| thiserror | 1.0.69, 2.0.19 | 2 |
| thiserror-impl | 1.0.69, 2.0.19 | 2 |
| toml | 0.8.23, 1.1.4+spec-1.1.0 | 2 |
| toml_datetime | 0.6.11, 1.1.1+spec-1.1.0 | 2 |
| winnow | 0.7.15, 1.0.4 | 2 |
| convert_case | 0.6.0, 0.7.1 | 1 |
| cpufeatures | 0.2.17, 0.3.0 | 1 |
| getrandom | 0.2.17, 0.3.4, 0.4.3 | 1 |
| gloo-net | 0.5.0, 0.6.0 | 1 |
| hashbrown | 0.12.3, 0.14.5, 0.15.5, 0.17.1 | 1 |
| itertools | 0.10.5, 0.13.0, 0.14.0 | 1 |
| linux-raw-sys | 0.12.1, 0.4.15 | 1 |
| rand | 0.10.2, 0.8.7, 0.9.5 | 1 |
| rand_chacha | 0.3.1, 0.9.0 | 1 |
| rand_core | 0.10.1, 0.6.4, 0.9.5 | 1 |
| socket2 | 0.5.10, 0.6.5 | 1 |
| tower | 0.4.13, 0.5.3 | 1 |
| tower-http | 0.5.2, 0.6.11 | 1 |
| tungstenite | 0.21.0, 0.25.0 | 1 |
| unicode-width | 0.1.14, 0.2.0 | 1 |
| windows-sys | 0.45.0, 0.48.0, 0.52.0, 0.59.0, 0.61.2 | 1 |
| windows-targets | 0.42.2, 0.48.5, 0.52.6 | 1 |
| windows_aarch64_gnullvm | 0.42.2, 0.48.5, 0.52.6 | 1 |
| windows_aarch64_msvc | 0.42.2, 0.48.5, 0.52.6 | 1 |
| windows_i686_gnu | 0.42.2, 0.48.5, 0.52.6 | 1 |
| windows_i686_msvc | 0.42.2, 0.48.5, 0.52.6 | 1 |
| windows_x86_64_gnu | 0.42.2, 0.48.5, 0.52.6 | 1 |
| windows_x86_64_gnullvm | 0.42.2, 0.48.5, 0.52.6 | 1 |
| windows_x86_64_msvc | 0.42.2, 0.48.5, 0.52.6 | 1 |

## Workspace Member Lockfile Presence

✅ All workspace members present in Cargo.lock

### All workspace members:

| member | in_lock | lock_version |
|---|:-:|---|
| apeireth-core | ✓ | 1.0.0 |
| apeireth-memory | ✓ | 1.0.0 |
| apeireth-asi | ✓ | 1.0.0 |
| apeireth-tools | ✓ | 1.0.0 |
| apeireth-cli | ✓ | 1.0.0 |
| apeireth-bench | ✓ | 1.0.0 |
| apeireth-cognition | ✓ | 1.0.0 |
| apeireth-action | ✓ | 1.0.0 |
| apeireth-life-force | ✓ | 1.0.0 |
| apeireth-constraint | ✓ | 1.0.0 |
| apeireth-central | ✓ | 1.0.0 |
| apeireth-value | ✓ | 1.0.0 |
| apeireth-consciousness | ✓ | 1.0.0 |
| apeireth-relation | ✓ | 1.0.0 |
| apeireth-motivation | ✓ | 1.0.0 |
| apeireth-perception | ✓ | 1.0.0 |
| apeireth-upgrade | ✓ | 1.0.0 |
| apeireth-onion | ✓ | 1.0.0 |
| apeireth-council | ✓ | 1.0.0 |
| apeireth-sovereignty | ✓ | 1.0.0 |
| apeireth-supervisor | ✓ | 1.0.0 |
| apeireth-pybridge | ✓ | 1.0.0 |
| apeireth-verify | ✓ | 1.0.0 |
| apeireth-extension | ✓ | 1.0.0 |
| apeireth-evolution | ✓ | 1.0.0 |
| apeireth-bus | ✓ | 1.0.0 |
| apeireth-api | ✓ | 1.0.0 |
| apeireth-web | ✓ | 1.0.0 |
| apeireth-tui | ✓ | 1.0.0 |
| apeireth-protocol | ✓ | 1.0.0 |
| apeireth-http-client | ✓ | 1.0.0 |
| apeireth-pipeline | ✓ | 1.0.0 |
| apeireth-tool-registry | ✓ | 1.0.0 |
| apeireth-tool-runtime | ✓ | 1.0.0 |
| apeireth-tool-approval | ✓ | 1.0.0 |
| apeireth-agent | ✓ | 1.0.0 |
| apeireth-mcp | ✓ | 1.0.0 |
| apeireth-graph | ✓ | 1.0.0 |
| apeireth-formal | ✓ | 1.0.0 |
| apeireth-vector | ✓ | 1.0.0 |
| apeireth-sdk | ✓ | 1.0.0 |
| apeireth-workflow | ✓ | 1.0.0 |
| apeireth-team-lead | ✓ | 1.0.0 |
| apeireth-mcp-relay-image | ✓ | 1.0.0 |
| apeireth-mcp-ssh | ✓ | 1.0.0 |
| apeireth-mcp-winrm | ✓ | 1.0.0 |

## Internal Packages (apeireth-*)

| package | version | source | checksum | deps |
|---|---|---|:-:|---:|
| apeireth-action | 1.0.0 | (path/workspace) | ✗ | 7 |
| apeireth-agent | 1.0.0 | (path/workspace) | ✗ | 14 |
| apeireth-api | 1.0.0 | (path/workspace) | ✗ | 26 |
| apeireth-asi | 1.0.0 | (path/workspace) | ✗ | 10 |
| apeireth-bench | 1.0.0 | (path/workspace) | ✗ | 9 |
| apeireth-bus | 1.0.0 | (path/workspace) | ✗ | 22 |
| apeireth-central | 1.0.0 | (path/workspace) | ✗ | 1 |
| apeireth-cli | 1.0.0 | (path/workspace) | ✗ | 12 |
| apeireth-cognition | 1.0.0 | (path/workspace) | ✗ | 9 |
| apeireth-consciousness | 1.0.0 | (path/workspace) | ✗ | 8 |
| apeireth-constraint | 1.0.0 | (path/workspace) | ✗ | 6 |
| apeireth-core | 1.0.0 | (path/workspace) | ✗ | 7 |
| apeireth-council | 1.0.0 | (path/workspace) | ✗ | 7 |
| apeireth-evolution | 1.0.0 | (path/workspace) | ✗ | 6 |
| apeireth-extension | 1.0.0 | (path/workspace) | ✗ | 10 |
| apeireth-formal | 1.0.0 | (path/workspace) | ✗ | 0 |
| apeireth-graph | 1.0.0 | (path/workspace) | ✗ | 6 |
| apeireth-http-client | 1.0.0 | (path/workspace) | ✗ | 12 |
| apeireth-life-force | 1.0.0 | (path/workspace) | ✗ | 5 |
| apeireth-mcp | 1.0.0 | (path/workspace) | ✗ | 12 |
| apeireth-mcp-relay-image | 1.0.0 | (path/workspace) | ✗ | 14 |
| apeireth-mcp-ssh | 1.0.0 | (path/workspace) | ✗ | 11 |
| apeireth-mcp-winrm | 1.0.0 | (path/workspace) | ✗ | 13 |
| apeireth-memory | 1.0.0 | (path/workspace) | ✗ | 13 |
| apeireth-motivation | 1.0.0 | (path/workspace) | ✗ | 8 |
| apeireth-onion | 1.0.0 | (path/workspace) | ✗ | 4 |
| apeireth-perception | 1.0.0 | (path/workspace) | ✗ | 8 |
| apeireth-pipeline | 1.0.0 | (path/workspace) | ✗ | 12 |
| apeireth-protocol | 1.0.0 | (path/workspace) | ✗ | 5 |
| apeireth-pybridge | 1.0.0 | (path/workspace) | ✗ | 9 |
| apeireth-relation | 1.0.0 | (path/workspace) | ✗ | 8 |
| apeireth-sdk | 1.0.0 | (path/workspace) | ✗ | 3 |
| apeireth-sovereignty | 1.0.0 | (path/workspace) | ✗ | 9 |
| apeireth-supervisor | 1.0.0 | (path/workspace) | ✗ | 4 |
| apeireth-team-lead | 1.0.0 | (path/workspace) | ✗ | 17 |
| apeireth-tool-approval | 1.0.0 | (path/workspace) | ✗ | 11 |
| apeireth-tool-registry | 1.0.0 | (path/workspace) | ✗ | 9 |
| apeireth-tool-runtime | 1.0.0 | (path/workspace) | ✗ | 14 |
| apeireth-tools | 1.0.0 | (path/workspace) | ✗ | 16 |
| apeireth-tui | 1.0.0 | (path/workspace) | ✗ | 26 |
| apeireth-upgrade | 1.0.0 | (path/workspace) | ✗ | 11 |
| apeireth-value | 1.0.0 | (path/workspace) | ✗ | 9 |
| apeireth-vector | 1.0.0 | (path/workspace) | ✗ | 7 |
| apeireth-verify | 1.0.0 | (path/workspace) | ✗ | 7 |
| apeireth-web | 1.0.0 | (path/workspace) | ✗ | 28 |
| apeireth-workflow | 1.0.0 | (path/workspace) | ✗ | 13 |

## Philosophy Gates (主 17:58 不假装)

- ✅ **v1295_extends_v1294** — V1295 继承 V1294 build.rs, 不删 V1294
- ✅ **v1295_no_new_asi_dim** — V1295 = Cargo.lock audit, 不引入新 ASI dim
- ✅ **v1295_no_asi_v1_claim** — 不假装 ASI V1: Cargo.lock ≠ ASI
- ✅ **v1295_no_kpi_inflate** — NS 92.91% LOCKED, 不刷
- ✅ **v1295_no_phenomenal_claim** — Cargo.lock ≠ phenomenal consciousness
- ✅ **v1295_stdlib_only** — 仅用 stdlib (re/dataclasses/json/pathlib), 不引入新依赖
- ✅ **v1295_read_only** — 只读 Cargo.lock, 不改
- ✅ **v1295_audit_not_fix** — audit ≠ fix, V1295 仅审计
- ✅ **v1295_no_cargo_run** — 不调 cargo build / cargo check / cargo update
- ✅ **v1295_regex_only** — regex-only pattern match, 不解析完整 TOML AST
- ✅ **v1295_offline** — 不联网, 不 fetch crates.io / rustsec advisory db
- ✅ **v1295_no_yanked_check_online** — 无法在线查 advisory db, 不假装 'no yanked = safe'

## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装达到 ASI)

- V1295 在此 ≠ 'Cargo.lock 安全': 仅 lockfile 静态解析, 不调 cargo build
- PASS ≠ cargo build 成功: PASS 仅 = 阈值达标
- 不刷 KPI: 计数是真统计, 不是 KPI
- 失败也诚实披露: FAIL 全部列出, 不掩饰
- audit ≠ fix: V1295 仅审计, 不 cargo update / 不 cargo build
- 不依赖网络: offline 跑, 无法查 rustsec advisory db (主 19:33)
- 不假装 yanked = safe: 无法在线查 advisory db = honest disclosure
- regex-only TOML parse: 可能漏 multi-line 字段或含特殊字符的字符串
- 不假装 parse 完整 TOML: 用 regex 简化, 多行字符串可能截断