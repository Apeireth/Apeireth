# V1291 — VCP Rust Build Artifact Profile

- Deps root: `.openclaw\workspace\promethean\Apeireth-rust\target\debug\deps`
- Total crates scanned: 42
- Crates with artifacts: 41
- Crates with test binary: 0
- Crates with example binary: 0
- Total artifacts: 6008
- Total size: 31150.318 MB
- Duration: 18377 ms

## 5 Hypotheses (主 13:08 真自问, Popper 可证伪)

| # | Hypothesis | Threshold | Result | Detail |
|---|------------|-----------|--------|--------|
| 1 | `h_crate_build_coverage_ge_80pct` | 80.0 | ✓**PASS** | crates_with_artifacts=41, total_crates=42, coverage_pct=97.61904761904762 |
| 2 | `h_artifacts_per_crate_ge_3` | 3 | ✓**PASS** | crates_pass=41, total_crates=42 |
| 3 | `h_median_artifact_size_lt_5mb` | 5000 | ✓**PASS** | median_kb=178.1162109375, n_artifacts=6008 |
| 4 | `h_example_binary_count_ge_1` | 1 | ✗**FAIL** | crates_with_example_binary=0 |
| 5 | `h_total_size_lt_50gb` | 50.0 | ✓**PASS** | total_gb=30.420232449658215, total_mb=31150.318 |

## Top-10 Crates by Total Artifact Size

| Crate | rlib | rmeta | exe | d | pdb | total_MB | max_MB | has_test |
|-------|------|-------|-----|---|-----|----------|--------|----------|
| apeireth-api | 52 | 85 | 17 | 110 | 17 | 5422.041 | 114.48 | False |
| apeireth-tui | 0 | 11 | 18 | 29 | 18 | 2057.252 | 163.262 | False |
| apeireth-cli | 27 | 32 | 16 | 52 | 16 | 1792.877 | 85.121 | False |
| apeireth-memory | 53 | 78 | 16 | 96 | 16 | 1522.124 | 85.285 | False |
| apeireth-sovereignty | 41 | 53 | 16 | 70 | 16 | 1516.717 | 88.074 | False |
| apeireth-tools | 37 | 68 | 16 | 84 | 16 | 1476.01 | 73.371 | False |
| apeireth-extension | 28 | 34 | 14 | 48 | 14 | 1361.929 | 97.879 | False |
| apeireth-council | 43 | 55 | 15 | 71 | 15 | 1236.352 | 84.863 | False |
| apeireth-tool-runtime | 16 | 25 | 10 | 35 | 10 | 1108.927 | 101.191 | False |
| apeireth-upgrade | 26 | 29 | 15 | 44 | 15 | 1045.281 | 86.371 | False |

## Bottom-5 Crates by Total Artifact Size

| Crate | rlib | rmeta | exe | d | pdb | total_MB |
|-------|------|-------|-----|---|-----|----------|
| apeireth-relation | 0 | 0 | 0 | 0 | 0 | 0.0 |
| apeireth-tauri-stub | 1 | 1 | 0 | 1 | 0 | 0.01 |
| apeireth-formal | 1 | 5 | 1 | 6 | 1 | 4.038 |
| apeireth-sdk | 3 | 14 | 5 | 19 | 5 | 34.329 |
| apeireth-consciousness | 29 | 48 | 11 | 59 | 11 | 68.108 |

## Crates Without Build Artifacts

| Crate | deps_dir_exists |
|-------|-----------------|
| apeireth-relation | True |

## VCP Rust #1-#12 完整闭环

- VCP Rust 静态: V1280 ✓ (源代码)
- VCP Rust 语义 #1-#3: V1281-V1283 ✓ (源代码)
- VCP Rust 安全 #1-#4: V1284-V1287 ✓ (源代码)
- VCP Rust 治理 #1: V1288 ✓ (源代码)
- VCP Rust 文档 #1-#2: V1289-V1290 ✓ (源代码)
- **VCP Rust 构建 #1: V1291 ✓ (target/debug/deps/* artifacts)** ← 本模块

## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)

- V1291 在此 ≠ "42 crates 已编译干净": 仅扫描 target/debug/deps/
- PASS ≠ 编译健康: PASS 仅 = 阈值达标
- 不刷 KPI: artifact count/size 是扫描数, 不是 KPI
- 失败也诚实披露: FAIL 全部列出, 不掩饰
- audit ≠ fix: V1291 仅审计, 不 cargo clean / 不 cargo build
- release profile 不扫: 仅 debug (主 13:08 真自问)
- test/example 检测简化: 找 *test*.exe / *example*.exe
- artifact 命名依赖 cargo 标准, 自定义 build script 可能不匹配
- V1291 不删 V1280-V1290: 是 spectrum 互补 (源代码 → 构建产物)
