# docs/ci/ — CI 流水线

```
[Document-Meta]
Document:       docs/ci/README.md
Version:        R119-4a
R-Cycle:        R119 (文档体系推倒重建)
Last-Modified:  2026-08-10
Status:         🟢 索引层
```

> **性质**: CI / CD 流水线设计文档。1 文件, R20 阶段 6 1.0 release 时期留下。

---

## 索引

| 文档 | 性质 | 字节 |
|---|---|---|
| [`1.0-release-pipeline.md`](1.0-release-pipeline.md) | 1.0 release CI pipeline 设计 (cosign 签 + verify + 多平台 matrix) | 6,729 |

---

## 相关入口

- CI workflow 实装: [`.github/workflows/`](../../.github/workflows/) (rust-ci.yml / coverage.yml / nightly.yml / benchmark.yml / protocol-e2e.yml / eval-live.yml / cargo-audit.yml)
- 1.0 release 12 项 #5 ci 状态: [`docs/1.0-release/checklist.md`](../1.0-release/checklist.md)
- 1.0 release 报告: [`docs/release/1.0.0/CHANGELOG.md`](../release/1.0.0/CHANGELOG.md)
