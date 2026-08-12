# apeireth-tool-image-process

**R141** — 图像处理工具

## 职责

图像后处理: 裁剪 / 缩放 / 滤镜 / 格式转换 / EXIF 提取 / OCR.

## 核心能力

- 6 子模块 (resize / crop / filter / convert / exif / ocr)
- 异步处理 (大图不阻塞)
- 与 `apeireth-tool-image-gen` 串联

## 0 假装

✅ 20 单元测试 | ⚠️ EXIF / OCR 需 feature-gated kamadak-exif / tesseract-rs
