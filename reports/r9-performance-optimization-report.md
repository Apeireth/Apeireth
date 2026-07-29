# R9 Performance Optimization Report — Required-Name Index

This specification used two report names. The single detailed fact source is:

- [`r9-performance-optimizer-report.md`](r9-performance-optimizer-report.md)
- focused raw timing: [`v1074_perf_before_after.md`](v1074_perf_before_after.md)

Acceptance summary:

- implementation commit: `638e9624d0ed3db9d39860b476a132d7f39018b1`;
- five real optimizers implemented and independently switchable;
- independent-process V1074 median: **3.252062 s → 1.018498 s**;
- reduction: **68.6815%**, speedup **3.1930×**;
- gate: `<2.5 s` and `≥20%` both PASS;
- tests: **139 passed** (61 new V1118 + 78 V1074 regression);
- all measured semantic-equivalence guards pass.
