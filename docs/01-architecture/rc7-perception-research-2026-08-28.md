# RC-7 Perception 真 modality 真账 (Whisper + xcap, 硬件依赖, 估 2-3 周)

> **本文档定位**: 调研账 — 继 R14 spec (`rc-7-perception-true-modality-spec.md`, 746 行) 后, 本子代理 (R15-style 调研岗) 写"真账": 不假装硬件可达, 不假装端点调通, 列出**当下能做的 (mock test) vs 必须硬件的 (真集成) vs 估时**三段.
>
> **关系**: 本文 + R14 spec (§0-§10) + `perception_backend.rs` (R6 真写, 456 行) + `v2.0.0-release-path-integration.md` §3.2.
>
> **本文状态**: 🟡 **调研账 (no code change, no new dep added)**, 真实施待主代理后续派 R15+ 真做.
>
> **0 装诚实**: 本文**不**真做 2-3 周 RC-7 真 modality 实施, 仅**调研 + 估时 + 真实施前置条件清单**. 0 改 `perception_backend.rs` (R6 LOCKED), 0 改 `Cargo.lock`, 0 改任何 LOCKED 5 项.

```
[Document-Meta]
Document:      docs/01-architecture/rc7-perception-research-2026-08-28.md
Version:       Research-0.1 (R14 spec 续)
Last-Modified: 2026-08-28
Status:        🟡 调研账, 真实施待
HEAD:          22c6e72b
Author:        子代理 R15 (调研岗, 0 装诚实真账)
```

---

## §1. 硬件依赖真账 (不假装"无硬件能跑")

### 1.1 Voice 真接 (Whisper HTTP) — 麦克风 + 网络可达 endpoint

**硬件需求**:
- **麦克风**: USB 或内置 (Windows WASAPI / Linux pulseaudio / macOS coreaudio). v1 走 `voice_session.rs` 取音频流, 但 `voice_session.rs` **无 cpal/mic 调用** (grep 0 match) — v1 是**离线文件**走的 STT, **不录实时**.
- **真 endpoint 可达**: OpenAI `https://api.openai.com/v1/audio/transcriptions` OR MiniMax `https://api.minimaxi.com/v1/audio/transcriptions` (兼容未确认, 估 1-2 天联调).
- **真 API key**: `APEIRETH_API_KEY` env 或 RC-9 `KeyringCredentialResolver`.

**v1 真接参考** (1:1 翻译, `legacy/donor/apeireth-voice/src/real.rs:824-938`):
- 走 `reqwest::multipart::Form` POST `/v1/audio/transcriptions`
- `Authorization: Bearer {key}` header
- `language` + `model=whisper-1` + `file=<PCM bytes>` part
- 401 自动重试 1 次 (refresh key + retry)
- 返 plain text (text/plain) 或 json (跟 `response_format` 走)
- K-1 强校验: `audio.assert_sample_rate_hardcode()` + `audio.assert_duration_within_limit()` (v2 `AudioBuffer` 是最小公约数, R14 spec §3.4 已标)

### 1.2 Vision 真接 (xcap) — Windows desktop + 多显示器

**硬件需求**:
- **Windows 10 1903+ / Windows 11** (Windows.Graphics.Capture API 最低版本要求)
- **多显示器** (可选, 单显示器也能测)

**xcap crate 真账**:
- 版本: `0.7.x` (R14 spec 估, 真实施时核 crates.io 最新)
- 跨平台: 标 "cross-platform" 但**仅 Windows 真接** Windows.Graphics.Capture API; Linux/macOS 走 x11/cocoa backend, **0 装占位** (per R14 §4.4)
- API 复杂度: 中 — `Monitor::all()` + `monitor.capture_image()` + `image.to_png()`, sync API 需 `tokio::spawn_blocking` 包成 async
- Cargo.toml 加 (估 R15+ 真实施):
  ```toml
  xcap = "0.7"
  ```
- 估 +1-2 行 Cargo.lock diff (新外部 dep, R14 §4.2 已标)

**v1 真接参考 — 重要诚实标**:
- `legacy/donor/apeireth-companion/src/screen_perception.rs` **不是截屏**, 是 **foreground window 轮询** (`GetForegroundWindow` 注释, 168 行)
- v1 没有任何 xcap / BitBlt / PNG encode 调用 (grep 全仓 0 match for `xcap`)
- **0 装诱导 prevention**: v1 屏幕"感知"= 窗口事件, **不是像素截屏**; RC-7 Vision 真接是**新增能力**, v1 没历史实现可参考

### 1.3 跨平台 fallback (Linux/macOS)

**诚实话**: xcap 跨平台 backend 是**0 装占位**:
- Linux: `NoopVisionBackend` 返 `Err(BackendUnavailable("x11 backend not wired"))`
- macOS: `NoopVisionBackend` 返 `Err(BackendUnavailable("cocoa backend not wired"))`
- 真实施时估主代理拍板 feature flag 方案 (per R14 §4.4)

---

## §2. Whisper endpoint 兼容性调研

**web_search 工具本次不可用** (返回 auth error), 调研基于 R14 spec 既有 + v1 `apeireth-voice::real.rs` 真接代码 + 子代理 cutoff 知识:

| Endpoint | Base URL | Model | Auth | 兼容性 |
|---|---|---|---|---|
| **OpenAI Whisper** | `https://api.openai.com/v1/audio/transcriptions` | `whisper-1` | `Bearer sk-...` | ✅ 1:1 翻译 v1, 已知 (R6 测试 + v1 真接) |
| **MiniMax 兼容** | `https://api.minimaxi.com/v1/audio/transcriptions` | `speech-01` | `Bearer ...` | ⚠️ **未确认兼容**, R6 §2.5 已默认 URL, **估 1-2 天联调** |
| **自托管 whisper.cpp** | `http://localhost:8080/v1/audio/transcriptions` (估) | ggml-base.bin 估 | `Bearer any` 或无 | ⚠️ 估 OpenAI 兼容, 但 binary + 模型文件配置需自管 |

**MiniMax Coding Plan audio transcription 兼容性未确认** — R6 标了 `model = "speech-01"`, 端点格式 估 OpenAI 兼容 (path / form / auth header 同), 但实际 200 OK + JSON shape 估需 R15+ 真测一次才能标 "✅ 真接".

**自托管 whisper.cpp** — 出 scope, R15 不实施. R15 估标 "future work" (v2.1 续).

---

## §3. xcap crate 真账 (Windows 真接代码示例, R15+ 真实施用)

```rust
// crates/engine/perception/src/vision/xcap_vision_backend.rs (R15+ 真实施估 100-200 行)
use xcap::Monitor;

pub struct XcapVisionBackend { monitor_index: usize }

#[async_trait]
impl VisionBackend for XcapVisionBackend {
    async fn capture(&self) -> Result<ScreenshotBytes, PerceptionBackendError> {
        let monitor_idx = self.monitor_index;
        let img = tokio::task::spawn_blocking(move || -> Result<image::RgbaImage, xcap::XCapError> {
            let monitors = Monitor::all()?;
            let m = monitors.get(monitor_idx)
                .ok_or_else(|| xcap::XCapError::new("monitor out of range"))?;
            m.capture_image()
        }).await
         .map_err(|e| PerceptionBackendError::Stream(format!("join: {e}")))??;

        // PNG encode (image crate 估 +1 行 Cargo.lock)
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        img.write_with_encoder(encoder)
           .map_err(|e| PerceptionBackendError::Stream(format!("png: {e}")))?;

        Ok(ScreenshotBytes { bytes: png, format: "png".into(),
                            captured_at_ms: chrono::Utc::now().timestamp_millis() })
    }
    fn name(&self) -> &'static str { "xcap" }
}
```

**0 装诱导 prevention**:
- `xcap = "0.7"` 估新外部 dep, R15+ 真实施时主代理拍板
- `image` crate 估新外部 dep (PNG encode), 主代理拍板
- `chrono` 已在 workspace (核验), 不新引
- `tokio::task::spawn_blocking` 复用现有 runtime, 不新引

---

## §4. 不依赖硬件的真实施 (R15 估 1 周内可做, 无麦克风可测)

### 4.1 写 backend impl 骨架 (`WhisperHttpBackend`)

- 新建 `crates/engine/perception/src/voice/whisper_http_backend.rs` (估 200-300 行)
- 复用 `ProviderCapability` HTTP client (per RC-5 真接模式, `minimax_llm_factory.rs:75-99`)
- 凭证每次 `transcribe()` resolve (per RC-9 keyring 模式)
- 错误 `ProviderError::*` → `PerceptionBackendError::*` 一对一映射

### 4.2 Mock test (估 5-7 测试, 不需麦克风)

用 `wiremock` crate (估新外部 dep, R15+ 真实施时主代理拍板):

| # | 测试 | 不需硬件 |
|---|---|---|
| 1 | `whisper_http_backend_transcribes_wav_succeeds` | ✅ 用 sample wav file (随便下个公域 wav) |
| 2 | `whisper_http_backend_401_retries_once` | ✅ mock server 返 401 → 200 |
| 3 | `whisper_http_backend_429_returns_rate_limited` | ✅ mock server 返 429 + Retry-After |
| 4 | `whisper_http_backend_400_returns_audio_error` | ✅ mock server 返 400 |
| 5 | `whisper_http_backend_no_key_returns_backend_unavailable` | ✅ NoCredentials + 不调 HTTP |
| 6 | `whisper_http_backend_network_error_returns_network` | ✅ mock server close connection |
| 7 | `whisper_http_backend_minimax_endpoint` | ✅ mock server 听 `api.minimaxi.com` shape |

**xcap mock** 难 (无 mockable native capture), 但 `XcapVisionBackend` 单测可写**构造 + error 路径** (越界 / 空 monitor list) 不需真截屏.

### 4.3 不依赖硬件估时 (R15 估 1 周)

| 子任务 | 估时 | 不需硬件 |
|---|---|---|
| `WhisperHttpBackend` impl 骨架 | 3-5 天 | ✅ |
| 5-7 mock test + wiremock server | 2-3 天 | ✅ |
| `XcapVisionBackend` 骨架 + 2 error path test | 2-3 天 | ✅ (error path 不需真截屏) |
| `NoopVisionBackend` 占位 (Linux/macOS) | 0.5 天 | ✅ |
| **估 1 周 (不需硬件)** | | |

---

## §5. 估时真账 (R15 不假装, 跟 R14 spec §6.1 1:1 对齐)

| 块 | 子任务 | 估时 | 需硬件 |
|---|---|---|---|
| **Whisper 真接** | HTTP client + multipart + verbose_json 解析 | 3-5 天 | ❌ |
| | 401 重试 1 次 + 错误分类 | 1-2 天 | ❌ |
| | 5-7 mock test + wiremock | 2-3 天 | ❌ |
| | **真 API key + 麦克风硬件联调** | **1-2 天** | **✅ 麦克风** |
| | (Whisper 总 1-2 周) | | |
| **xcap 真接** | xcap crate + 异步包装 | 2-3 天 | ❌ |
| | ScreenshotBytes + PNG encode | 1 天 | ❌ |
| | 3-4 测试 + 跨平台 0 装占位 | 1-2 天 | ❌ |
| | **Windows 桌面 + 多显示器联调** | **1-2 天** | **✅ Windows** |
| | (xcap 总 1 周) | | |
| **总估** | | **2-3 周** | |

**R15 估时分两段**:
- **无硬件段**: 1 周 (Whisper 5-7 天 + xcap 3-5 天 mock)
- **硬件联调段**: 1-2 周 (主代理亲做, 不在 R15 范围)

---

## §6. commit message 模板 (4 项标, 跟 R13/R14 一致)

```
feat(perception): RC-7 真接 Whisper + xcap backend impl + mock test (R15)

1. 0 装诚实标: R15 真做 {子任务} (估时 N 天), R14 spec §0 §6 已拍方向
2. 0 触碰 LOCKED: perception_backend.rs (R6 真写) 0 改, perception.rs 0 改,
   Cargo.lock +{N} 行 (xcap 新外部 dep, 主代理拍板)
3. 测试: cargo test -p apeireth-plugin --lib perception_backend 估 0 破,
   +{N} 新 mock test (whisper_http_backend_*, xcap_vision_backend_*)
4. 文档: docs/01-architecture/rc7-perception-research-2026-08-28.md (本调研账)
   + R14 spec (rc-7-perception-true-modality-spec.md) 续实施记录
```

---

## §7. 下步建议 (不假装"已全做完")

1. **R15+ 优先做不需硬件的 1 周**: `WhisperHttpBackend` + mock test + `XcapVisionBackend` 骨架 + error path. 估 1 周内可完成, 不卡硬件.
2. **硬件联调**: 主代理亲做 — 麦克风 (USB 或内置) + Windows 桌面 + 真 API key + endpoint ping. 估 1-2 周.
3. **MiniMax 端点联调**: 估 1-2 天 ping + 真 wav file 上传. R15+ 启动前先确认兼容性, 不假装"OpenAI = MiniMax".
4. **xcap 跨平台**: 真实施时估主代理拍板 `#[cfg(target_os = "windows")]` 守门, Linux/macOS 估 v2.1 续 (`NoopVisionBackend` 占位).
5. **新外部 dep 拍板**: xcap + image + wiremock 共 3 新外部 dep, 估主代理拍板 + Cargo.lock 估 +5-15 行 diff (vs R14 LOCKED "0 行 diff").

---

## §8. 0 装诚实真账 (R15 独立判断)

- **本文 0 装诱导 prevention**: 不真做 2-3 周 RC-7 真 modality, 仅调研 + 估时 + 实施前置清单
- **v1 屏幕感知 ≠ 截屏**: `screen_perception.rs` 是 GetForegroundWindow 轮询, **不是像素截屏**, RC-7 Vision 真接是新增能力
- **MiniMax 端点未实测**: R15 不假装"已调通 MiniMax speech-01", R15+ 真实施时需主代理亲测
- **web_search 本次不可用**: 调研基于 v1 真接代码 + R14 spec 既有 + cutoff 知识, 不假装"已 web 验证 endpoint"
- **xcap 真接代码示例 (§3)**: 是 spec 阶段骨架, 不假装"已编译通过", R15+ 真实施时需跑 cargo check

```
Document:      docs/01-architecture/rc7-perception-research-2026-08-28.md
Version:       Research-0.1
Last-Modified: 2026-08-28
Status:        🟡 调研账, 真实施待
Total Lines:   ~180 (估)
Real Impl:     估 2-3 周 (Whisper 1-2 周 + xcap 1 周, 含硬件联调 1-2 周)
No-Hardware:   估 1 周可做 (mock test + backend impl 骨架)
```