# 眼 (Eye) 器官 API

> **性质**: 9 器官之一 (per 整合 #3 C-1 借 Golutra #1)
> **对应 crate**: `apeireth-perception` (24 LOCKED 之一)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **TUI 短单字**: 眼 / **i18n 解剖名词**: 双眼

---

## 0. 概览

| 维度 | 值 |
|------|----|
| **器官名** | eye (眼 / 双眼) |
| **6 command** | scan / watch / focus / blur / zoom / stop |
| **关键 dep** | tokio 1.40 / serde 1.0 / apeireth-image-prompt / image 0.25 |
| **状态** | ✅ 24 LOCKED 之一 |
| **i18n 状态** | G-1 续补 (per 整合 #3 G-2) |

---

## 1. 6 command

| command | 用途 | i18n key (中文) |
|---------|------|----------------|
| `scan` | 扫描 (image OCR / 物体识别) | 扫描 |
| `watch` | 监视 (持续观察, 触发器) | 监视 |
| `focus` | 聚焦 (特定区域) | 聚焦 |
| `blur` | 模糊 (隐私过滤) | 模糊 |
| `zoom` | 缩放 (细节放大) | 缩放 |
| `stop` | 停止 (watch) | 停止 |

---

## 2. API 调用

```rust
use apeireth_perception::organ::eye::{Eye, ScanResult};

let eye = Eye::new();
let result = eye.scan(
    &image_bytes,
    ScanOptions {
        model: ScanModel::OcrChinese,
        detail: Detail::High,
    },
).await?;
// ScanResult { texts: ["Apeireth", "1.0", "release"], objects: [], faces: [] }
```

---

## 3. 5 扫描模式

| 模式 | 功能 | 1:1 翻译源 |
|------|------|------------|
| **OcrChinese** | 中文 OCR | Tesseract 5 + chi_sim |
| **OcrEnglish** | 英文 OCR | Tesseract 5 + eng |
| **ObjectDetection** | 物体识别 (per YOLO v8) | YOLO 0.1 |
| **FaceDetection** | 人脸检测 (per 隐私) | OpenCV Haar |
| **SceneRecognition** | 场景识别 (per 5 类) | ResNet50 |

---

## 4. 4 边缘 case (per `v1-tools-drive.md` § 4 隐私)

| 边缘 | 处理 |
|------|------|
| 人脸检测到 | 自动 blur (per `Eye::blur()`) |
| 敏感文本 (密码 / 密钥) | 自动 redact |
| 多次连续 watch | 节流 (1s 一次) |
| 失败 / 网络断 | 重试 3 次, 0 强依赖 |

---

## 5. TUI 9 器官 集成 (per 整合 #3 C-1)

```rust
// crates/apeireth-tui/src/organ/command/eye.rs
impl Command for EyeCommand {
    fn name(&self) -> &str { "eye" }  // i18n 改 async t() per G-2
    fn run(&self, args: &[String]) -> CommandResult { /* scan / watch / etc */ }
}
```

---

## 6. 相关

- 实现: `crates/apeireth-perception/`
- 1:1 翻译源: v0.9.21 SpectrAI eye organ
- 决策: 整合 #3 C-1 + G-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
