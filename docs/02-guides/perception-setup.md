# 多模态感知层配置指南 (Multimodal Perception Guide)

本文档介绍 Apeireth v2.0 感知层（`apeireth-perception`）的多模态后端装配、配置与安全准则。

---

## 1. 架构定位

Apeireth 感知层采用**单向分层**与**可插拔后端**设计：

- **Foundation 层**（`apeireth-plugin`）：定义 `VoiceBackend`、`VisionBackend`、`TactileBackend` 等纯 Trait 契约及 `PerceptionEvent` 统一事件格式。
- **Engine 层**（`apeireth-perception`）：实现具体的生产级通信与操作系统捕获后端。
- **Runtime 注入点**：感知后端以 `Arc<dyn VoiceBackend>` / `Arc<dyn VisionBackend>` trait 对象形式注入。但注意：当前 canonical runtime **默认不注册任何真实视觉后端**——`XcapVisionBackend` 仅支持显式 opt-in 构造（见 §3.1），`NoopVisionBackend` 为显式零假设占位（见 §3.2）。

---

## 2. 语音感知 (Voice Modality / STT)

### 2.1 WhisperHttpBackend

`WhisperHttpBackend` 提供对标准 OpenAI `/v1/audio/transcriptions` 及 MiniMax `/v1/audio/transcriptions` 接口的真实 HTTP 接入。

```rust
use std::sync::Arc;
use apeireth_perception::voice::{WhisperHttpBackend, WhisperHttpConfig};
use apeireth_plugin::credentials::StaticCredentials;

// 1. 凭据解析器 (生产环境走 OS Keyring 或环境变量)
let credentials = Arc::new(
    StaticCredentials::new()
        .with("provider.whisper.api_key", "sk-your-whisper-api-key-here-1234567890"),
);

// 2. 构造 OpenAI Whisper 后端
let openai_voice = WhisperHttpBackend::openai(credentials.clone());

// 3. 或构造 MiniMax 语音后端
let minimax_voice = WhisperHttpBackend::minimax(credentials);
```

### 2.2 安全与 0 装纪律
- **凭据零泄露 (O-1)**：API Key 通过 `CredentialResolver` 动态解析，日志与错误信息不暴露明文。
- **Fail-closed (O-5)**：缺少凭证时显式返回 `PerceptionBackendError::BackendUnavailable`，空音频拦截并返回 `PerceptionBackendError::Audio`，拒绝伪造假转写结果。

---

## 3. 视觉感知 (Vision Modality / Screen Capture)

### 3.1 XcapVisionBackend

`XcapVisionBackend` 封装操作系统显示器屏幕捕获，异步产出 `ScreenshotBytes`（PNG 格式及时间戳）。

```rust
use apeireth_perception::vision::{XcapVisionBackend, XcapVisionConfig};

// 主显示器默认配置
let vision = XcapVisionBackend::default_monitor();

// 自定义多显示器捕获
let secondary_vision = XcapVisionBackend::new(XcapVisionConfig {
    monitor_index: 1,
    format: "png".to_string(),
});
```

**状态标注 (0 装 PASS, 基线 candidate `8b7e3111`)**：

* **IMPLEMENTED**：真实 xcap 0.9.8 捕获后端，确定性显示器排序（主屏优先）、bounds 限制、真实 PNG/JPEG 编码、fail-closed 错误。
* **NOT PRODUCTION WIRED**：仅是 `VisionBackend` trait 的后端实现；canonical runtime 默认路径**不**调用、**不**注册。唯一用法是如上所示的显式 opt-in 构造。
* **NOT DEFAULT ENABLED**：任何地方都不默认开启；不构造即不捕获。
* **HARDWARE VALIDATED（仅 Windows）**：远端验证机交互会话中 ignored 测试 `real_xcap_hardware_capture_smoke` 通过（`\\.\DISPLAY1` 主屏 1680x1050，PNG 242067 字节，PNG magic 校验 + 真实桌面内容目检）；本地开发机交互会话 smoke 亦 PASS。无头 / session-0（SSH 服务上下文）环境按设计 fail-closed（`PerceptionBackendError::BackendUnavailable` invalid-handle `0x80070006` E_HANDLE）。**未做 macOS / Linux 硬件验证。**

### 3.2 NoopVisionBackend
在测试或纯文本交互环境下，使用 `NoopVisionBackend` 作为显式 0 装占位：

```rust
use apeireth_perception::vision::NoopVisionBackend;

let noop = NoopVisionBackend;
```

---

## 4. 验收与测试

运行感知模块全量测试（基线 candidate `8b7e3111` 远端 Windows 验证机实测：**39 passed / 0 failed，1 ignored**）：

```powershell
cargo test -p apeireth-perception --locked
cargo test -p apeireth-perception --test perception_integration --locked
cargo clippy -p apeireth-perception --all-targets --locked -- -D warnings
```

Xcap 真机冒烟测试默认 `#[ignore]`（需要交互式桌面会话，无头环境会 fail-closed 报 `BackendUnavailable`）。在有真实显示器的交互会话中可显式运行：

```powershell
cargo test -p apeireth-perception --locked -- --ignored real_xcap_hardware_capture_smoke
```
