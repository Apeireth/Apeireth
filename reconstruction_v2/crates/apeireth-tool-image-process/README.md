# apeireth-tool-image-process

> Apeireth R141: image processing tool (multimodal router, OCR placeholder, image hash, EXIF)

apeireth-tool-image-process 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (10 src 文件 / 26 测试 + 2 Kani proof)

- `src/lib.rs` — 入口 re-export (ToolBridge 装配)
- `src/router.rs` — multimodal router + ProcessOp 枚举 + 4 测试
- `src/hash.rs` — image hash (perceptual + cryptographic) + 4 测试
- `src/exif.rs` — EXIF reader (基础 tag 抽取) + 2 测试
- `src/ocr.rs` — OCR placeholder (O-5 不假装, stub 返 NotImplemented 守门) + 1 测试
- `src/mcp.rs` — MCP server (4 工具: ImageHash/ImageExif/ImageOcr/ImageThumbnail) + 4 测试
- `src/register.rs` — ToolBridge catalog 接入 + 1 测试
- `src/compat.rs` — 兼容层 adapter + 3 测试
- `src/enhanced.rs` — enhanced 路径 + 2 测试
- `src/organ_kani_proofs.rs` — image-process organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
