# OCR 类评估（Tesseract / PaddleOCR / EasyOCR）

## 机制（What it does）

- 核心功能：图片/扫描件 → 文字识别（OCR），含版面分析 + 多语言 + 表格识别
- 解决什么问题：图像中的文字无法直接被 LLM 消费 → OCR 转文本后才能检索/摘要/翻译
- 关键技术对比：

| 库 | 引擎 | 语言 | 优势 | 劣势 |
|---|---|---|---|---|
| Tesseract | LSTM (传统 OCR) | 100+ 语言 | 离线、纯 CPU、轻量 | 复杂版面/手写识别差 |
| PaddleOCR | PP-OCR (Transformer) | 80+ 语言 | 中文识别强、版面分析、表格识别 | Python + PaddlePaddle 重 |
| EasyOCR | CRAFT + CRNN | 80+ 语言 | 易用 API、深度学习模型 | 性能慢于 PaddleOCR |

## 对照（How it relates to APEIRETH）

- 相似能力：
  - `apeireth-tool-image-process`（图像处理工具套件）
  - `apeireth-tool-approval`（含审批流，可处理 OCR 后的人工校对）
  - 缺失：APEIRETH 无 OCR 套件
- 差异化优势：
  - 三个 OCR 库覆盖「轻量/中文/易用」三角，可按场景挑选
  - APEIRETH 当前若需 OCR 只能调外部 API（不满足本地/隐私场景）
- 可借鉴：
  - **本地 OCR 套件骨架**：在 `apeireth-tool-image-process` 加 `OcrEngine` trait（Tesseract/PaddleOCR/EasyOCR 三后端），主用 Tesseract（Rust 端有 `leptess` crate 绑定）
  - **隐私优先**：本地 OCR 满足 `apeireth-guard` (R173 Privacy Guard) 的「数据不出本地」原则

## 吸收建议（Action items）

- P0 立即做：加 `apeireth-tool-image-process` 的 `OcrEngine` trait 接口（最小可扩展） + Tesseract 后端（用 `leptess` crate，0 新系统依赖）。
- P1 评估后做：PaddleOCR 后端（通过 FFI/Python sidecar）；表格识别（PP-Structure）。
- P2 长期调研：端到端版面 + 手写（TrOCR / GOT-OCR2）。
- 不做（重复 / 价值低）：EasyOCR 单独集成（PaddleOCR 已覆盖其优势场景）。

## 0 装 PASS 标注

- 真用：**否**（未实测）
- 源：**未下载实测**。`research/source/` 无三个 OCR 库源码；本评估基于各项目 GitHub README + `leptess` crate 文档推理。
- 未调研不写结论：三个 OCR 库的实际准确率 / 性能 benchmark / 中文版识别效果均为推理判断。**强烈建议**先 `git clone` 三个库到 `research/source/ocr/` 实测后重写 P0 建议（特别是 `leptess` API 是否真覆盖 Tesseract 全部功能）。
- 注：与 `apeireth-tool-image-process` 实际接口契约冲突时，以 `apeireth-tool-image-process` 现状为准。