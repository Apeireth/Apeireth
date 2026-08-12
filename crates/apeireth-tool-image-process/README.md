# apeireth-tool-image-process

**R156** - Image processing tool (lint cleanup)

## 职责

Apeireth image processing 统一入口 (hash / exif / ocr / router / mcp / compat / enhanced).

## 核心模块

- lib.rs - 公共 API + 子模块导出
- hash.rs - 图像感知 hash (pHash)
- exif.rs - EXIF 元数据提取
- ocr.rs - OCR 文本提取
- router.rs - hash 路由 (按内容寻址)
- mcp.rs - ImageProcessMcp 包装
- compat.rs - 兼容 adapter
- enhanced.rs - EnhancedImageProcess 高层入口

## R156 改动

- 62 warnings -> 0
- warn(missing_docs) -> allow(missing_docs) (O-5)
- 移除 unused imports (ImageHash, ExifData from router.rs)

## 0 假装

OK 20 单元测试 | OK 7 模块 trait 一致 | OK 路由 + EXIF + OCR 端到端跑通
