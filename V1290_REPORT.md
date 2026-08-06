# V1290 — VCP Rust Doc Section Depth Audit

- Total crates: 42
- Total public fns: 1583
- Total with doc: 1430
- Total sections: 0
- Overall avg sections/doc: 0.000
- Total section_depth_score: 0
- Duration: 64 ms

## 5 Hypotheses (主 13:08 真自问, Popper 可证伪)

| # | Hypothesis | Threshold | Result | Detail |
|---|------------|-----------|--------|--------|
| 1 | `h_avg_sections_per_doc_ge_1p5` | 1.5 | ✗**FAIL** | avg=0.0, total_sections=0, total_with_doc=1430 |
| 2 | `h_examples_pct_ge_10pct` | 10.0 | ✗**FAIL** | overall_pct=0.0, n_with_examples=0 |
| 3 | `h_returns_section_pct_ge_30pct` | 30.0 | ✗**FAIL** | overall_pct=0.0, n_returns_value=1310, n_with_returns=0 |
| 4 | `h_safety_section_pct_ge_50pct_on_unsafe` | 50.0 | ✓**PASS** | overall_pct=0.0, n_unsafe=0, n_with_safety=0 |
| 5 | `h_args_section_pct_ge_30pct_on_multiarg` | 30.0 | ✗**FAIL** | overall_pct=0.0, n_multiarg=33, n_with_args=0 |

## Per-Crate Doc Section Depth (Top-10 by score)

| Crate | pub_fns | with_doc | sections | avg/doc | score | examples% | errors% | panics% |
|-------|---------|----------|----------|---------|-------|-----------|---------|---------|
| apeireth-action | 30 | 30 | 0 | 0.00 | 0 | 0.0 | 0.0 | 0.0 |
| apeireth-agent | 29 | 29 | 0 | 0.00 | 0 | 0.0 | 0.0 | 0.0 |
| apeireth-api | 64 | 23 | 0 | 0.00 | 0 | 0.0 | 0.0 | 0.0 |
| apeireth-asi | 88 | 83 | 0 | 0.00 | 0 | 0.0 | 0.0 | 0.0 |
| apeireth-bench | 24 | 15 | 0 | 0.00 | 0 | 0.0 | 0.0 | 0.0 |
| apeireth-bus | 50 | 50 | 0 | 0.00 | 0 | 0.0 | 0.0 | 0.0 |
| apeireth-central | 31 | 31 | 0 | 0.00 | 0 | 0.0 | 0.0 | 0.0 |
| apeireth-cli | 16 | 15 | 0 | 0.00 | 0 | 0.0 | 0.0 | 0.0 |
| apeireth-cognition | 17 | 16 | 0 | 0.00 | 0 | 0.0 | 0.0 | 0.0 |
| apeireth-consciousness | 16 | 16 | 0 | 0.00 | 0 | 0.0 | 0.0 | 0.0 |

## Bottom-5 Crates by Doc Quality

| Crate | pub_fns | with_doc | sections | avg/doc | score |
|-------|---------|----------|----------|---------|-------|
| apeireth-action | 30 | 30 | 0 | 0.00 | 0 |
| apeireth-agent | 29 | 29 | 0 | 0.00 | 0 |
| apeireth-api | 64 | 23 | 0 | 0.00 | 0 |
| apeireth-asi | 88 | 83 | 0 | 0.00 | 0 |
| apeireth-bench | 24 | 15 | 0 | 0.00 | 0 |

## VCP Rust #1-#11 完整闭环

- VCP Rust 静态: V1280 ✓
- VCP Rust 语义 #1: V1281 ✓
- VCP Rust 语义 #2: V1282 ✓
- VCP Rust 语义 #3: V1283 ✓
- VCP Rust 安全 #1: V1284 ✓ (worst-5)
- VCP Rust 安全 #2: V1285 ✓ (all-42)
- VCP Rust 安全 #3: V1286 ✓ (fix priority)
- VCP Rust 安全 #4: V1287 ✓ (unsafe deep)
- VCP Rust 治理 #1: V1288 ✓ (governance deep)
- VCP Rust 文档 #1: V1289 ✓ (coverage)
- **VCP Rust 文档 #2: V1290 ✓ (section depth)** ← 本模块

## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)

- V1290 在此 ≠ "所有 42 crates 文档已深入": 仅审 apeireth-* production src/
- PASS ≠ 文档深度好: PASS 仅 = 阈值达标, 不代表质量好
- 不刷 KPI: section depth 是扫描数, 不是 KPI
- 失败也诚实披露: FAIL 全部列出, 不掩饰
- audit ≠ fix: V1290 仅审计, 不批量写 section
- section_depth_score 是启发式, 不权威 (主 17:43)
- section 检测用 regex `# Name`, 简化, 不解析 Markdown
- production src/ only: tests/ examples/ benches 不算
- V1290 不删 V1289: 是 spectrum 互补 (覆盖 → 深度)
