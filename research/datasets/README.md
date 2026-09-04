# datasets/

实验数据集（原始 + 派生）。

- 大文件（>1 MB）不入 git；原始数据来源（URL / 生成脚本 / hash）必须记录在 `MANIFEST.md`。
- 每个数据集一个子目录：`<name>/raw/` + `<name>/processed/` + `<name>/README.md`（来源、license、规模、分割、污染检查）。
- 涉及用户数据的实验：脱敏 + 最小化 + 明确同意边界（per governance 8 类 PII 纪律）。
