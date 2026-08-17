# R173 "放最后" 模块接口完整性盘点

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R173 (interface audit, no code)
> **日期**: 2026-08-13
> **主人指示**: 语音识别 / 声纹 / 唤醒词 / 生图 = 只做好接口, 先不接入, 放到最后
> **borrow-id**: — (no new external borrow, audit-only)

---

## 0. 目的

按主人 2026-08-13 指示, 4 类能力"放最后"接入:
1. **语音识别** (STT)
2. **声纹** (voiceprint match)
3. **唤醒词** (wake word detection)
4. **生图** (image generation / processing)

R173 盘点这些模块的**接口现状**, 确认是否已就绪, 0 写新代码, 仅文档。

---

## 1. 语音识别 (STT)

### 1.1 接口位置: `apeireth_voice::real::VoiceRealImpl::speech_to_text`

文件: `crates/apeireth-voice/src/real.rs:846`

```rust
pub async fn speech_to_speech_to_text(
    &self,
    audio: &AudioBuffer,
    lang: Lang,
) -> VoiceResult<SttResult>
```

| 字段 | 值 |
|---|---|
| 入参 | `&AudioBuffer` (16kHz/16-bit PCM, 编译期 hardcode `VOICE_SAMPLE_RATE_HZ`) + `Lang` (5 enum) |
| 返 | `VoiceResult<SttResult>` (text + confidence + language) |
| HTTP | `POST {base_url}/v1/audio/transcriptions` (1:1 翻译商业版 v0.9.21 voice SDK) |
| Auth | `Authorization: Bearer <api_key>` |
| 真接率 | 1:1 翻译 wiremock 测过, 0 真接生产 API |

### 1.2 状态: ✅ 接口已就绪

- 5 Lang enum 编译期守门 (`SUPPORTED_LANGS.len() == 5`)
- wiremock 0.6 mock server 测 happy + error 双路径
- 401 自动重试 1 次 (per `ensure_api_key`)
- audio buffer 越界 K-1 强校验 (sample_rate + duration_ms)

### 1.3 真接时机

**留最后** — 等 STT 阶段做实接。

---

## 2. 声纹 (voiceprint match)

### 2.1 接口位置: `apeireth_voice::real::VoiceRealImpl::voiceprint_match`

文件: `crates/apeireth-voice/src/real.rs:993`

```rust
pub async fn voiceprint_match(
    &self,
    audio: &AudioBuffer,
    user_id: &str,
) -> VoiceResult<VoiceprintMatchResult>
```

| 字段 | 值 |
|---|---|
| 入参 | `&AudioBuffer` + `&str` (user_id) |
| 返 | `VoiceResult<VoiceprintMatchResult>` (similarity + verified bool) |
| HTTP | `POST {base_url}/v1/voiceprint/match` |
| Auth | `Authorization: Bearer <api_key>` |
| 真接率 | wiremock 测过, 远端未实现时返 `VoiceError::ServiceCallFailed` |

### 2.2 状态: ✅ 接口已就绪

- `VoiceprintMatchResult` 结构完整 (similarity 0.0-1.0, verified bool)
- 真实本地模型 (ECAPA-TDNN) **未做** — 接口走 HTTP 远端 (诚实标缺 #2 in real.rs 顶部)

### 2.3 真接时机

**留最后** — 接口先, 真接 + 本地模型放最后。

---

## 3. 唤醒词 (wake word detection)

### 3.1 接口位置: `apeireth_voice::real::VoiceRealImpl::detect_wake_word`

文件: `crates/apeireth-voice/src/real.rs:968`

```rust
pub async fn detect_wake_word(
    &self,
    _audio: &AudioBuffer,
) -> VoiceResult<WakeWord>
```

| 字段 | 值 |
|---|---|
| 入参 | `&AudioBuffer` (Porcupine 512 frames) |
| 返 | `VoiceResult<WakeWord>` (keyword + confidence + model) |
| HTTP | **0 网络调用** — STUB hardcode |
| STUB | `WakeWord { keyword: "apeireth", confidence: 0.95, model: "stub-default" }` |
| 5 keyword enum | `Apeireth / Computer / Jarvis / HeyApeireth / Custom` |

### 3.2 STUB facade: `apeireth_voice::VoiceSdk::wake_word_detect`

文件: `crates/apeireth-voice/src/lib.rs`

```rust
#[async_trait]
pub async fn wake_word_detect(&self, frame: &AudioFrame) -> VoiceResult<WakeWord>
// STUB_MODE = true 时返 Err(VoiceError::NotImplemented("apeireth_voice_wake_word_detect"))
```

### 3.3 状态: ✅ 接口已就绪 (STUB 双层)

- 9-tool whitelist (`apeireth_voice_wake_word_detect` etc.) 编译期守门
- 5 WakeWordType enum 编译期守门 (`SUPPORTED_WAKE_WORDS.len() == 5`)
- Porcupine 真接 **未做** — `Cargo.toml` 注释 `⏳ R21+ 续` per 0 引外部 dep + 8 项承诺 #7

### 3.4 真接时机

**留最后** — 加 `porcupine = "0.4"` 到 Cargo.toml 时再接。

---

## 4. 生图 (image generation)

### 4.1 接口位置: `apeireth_tool_image_gen`

文件: `crates/apeireth-tool-image-gen/src/{provider,generators,params,result,mcp}.rs`

```rust
// 4.1.1 主 trait (provider.rs)
#[async_trait]
pub trait ImageGenProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    async fn generate(&self, params: &ImageGenParams) -> Result<ImageGenResult>;
    fn max_batch(&self) -> u32 { 1 }
}

// 4.1.2 13 provider enum
pub enum ProviderKind {
    OpenAiDallE3, OpenAiDallE2, StabilitySdxl, StabilitySd3,
    MiniMaxImage, MockProvider, // + 7 placeholder
    // 共 13, 编译期守门
}

// 4.1.3 MCP 工具
pub struct ImageGenMcp;
impl ImageGenMcp {
    pub fn image_generate(&self, ...) -> ... { }
    pub fn list_providers(&self) -> Vec<ProviderKind> { }
}
```

| 字段 | 值 |
|---|---|
| Trait | `ImageGenProvider` (uniform interface, async generate) |
| Provider 数量 | 13 (编译期 hardcode `PROVIDER_COUNT == 13`) |
| 真接率 | 4/13 = mock + 3 API stubs (OpenAI DALL-E / Stability AI / MiniMax-Image), 0 真 HTTP |
| MCP 工具 | `image_generate` + `list_providers` (per VCP 13-provider compat) |

### 4.2 状态: ✅ 接口已就绪

- ImageGenProvider trait 完整 (uniform interface 13 provider 通用)
- 5 params (prompt / size / count / quality / style) + 3 ImageSize + 2 ImageQuality + 2 ImageStyle
- ImageGenResult (base64 + URL + metadata) 完整
- ProviderRegistry 7 deliverable 模块全在 (`R141_DELIVERABLES == 7`)
- **诚实标缺**: "Providers are honest stubs that validate inputs and return a placeholder image. Real API calls require API keys (NOT hardcoded; env vars or `apeireth-config`)."

### 4.3 真接时机

**留最后** — 主人 2026-08-13 指示"先不接入, 放到最后"。

---

## 5. 图处理 (image processing) — 同上

### 5.1 接口位置: `apeireth_tool_image_process`

文件: `crates/apeireth-tool-image-process/src/{hash,exif,ocr,router,mcp}.rs`

```rust
// 5.1.1 hash (perceptual)
pub fn perceptual_hash(bytes: &[u8]) -> Result<ImageHash>;

// 5.1.2 EXIF
pub struct ExifData { /* ... */ }
pub fn extract_exif(bytes: &[u8]) -> Result<Option<ExifData>>;

// 5.1.3 OCR (honest stub)
pub fn ocr_extract(bytes: &[u8], lang: &str) -> Result<OcrResult>;

// 5.1.4 router
pub struct ImageRouter;
impl ImageRouter {
    pub fn process(&self, op: ProcessOp, bytes: &[u8]) -> Result<Vec<u8>>;
}

// 5.1.5 MCP
pub struct ImageProcessMcp;
impl ImageProcessMcp {
    pub fn image_hash(&self, bytes: &[u8]) -> ... { }
    pub fn image_exif(&self, bytes: &[u8]) -> ... { }
    pub fn image_ocr(&self, bytes: &[u8], lang: &str) -> ... { }
}
```

| 字段 | 值 |
|---|---|
| Hash | perceptual hash (real impl, 用于去重) |
| EXIF | honest stub (extract_exif returns metadata placeholders) |
| OCR | honest stub (no vision dependency, deferred) |
| Router | multimodal 路由 (hash/exif/ocr/resize/format) |
| MCP | 3 工具 (image_hash / image_exif / image_ocr) |

### 5.2 状态: ✅ 接口已就绪

- 7 deliverable 模块全在 (`R141_IMAGE_PROC_DELIVERABLES == 7`)
- `image_hash` 是**真实现** (perceptual hash 算法)
- `image_exif` / `image_ocr` 是 honest stub (per 顶部诚实标缺)

### 5.3 真接时机

**留最后** — EXIF 真解析 + OCR 真模型放最后。

---

## 6. 总评

| 模块 | 接口位置 | 接口状态 | 真接时机 |
|---|---|---|---|
| STT | `VoiceRealImpl::speech_to_text` | ✅ 就绪 | 最后 |
| 声纹 | `VoiceRealImpl::voiceprint_match` | ✅ 就绪 | 最后 |
| 唤醒词 | `VoiceRealImpl::detect_wake_word` + STUB facade | ✅ 就绪 (STUB 双层) | 最后 |
| 生图 | `ImageGenProvider` trait + 13 provider + MCP | ✅ 就绪 | 最后 |
| 图处理 | `ImageRouter` + perceptual hash + EXIF/OCR stub + MCP | ✅ 就绪 | 最后 |

**结论**: 5 类"放最后"模块的接口**全部已就绪**, 0 新代码需要写, 仅等主人最终阶段统一真接。

---

## 7. 0 触碰声明

- 3 不可变脊柱 (Self-Disable / L0 HA / 13-key verdict cache): 0 触碰
- workspace.version 1.2.0: 0 改
- 24 LOCKED crate 入口签名: 0 改
- R172 MiniMaxLive TTS 真接路径: 0 触碰 (R173 仅盘点)
- 4 块语音接口 (real.rs): 0 改
- 生图/图处理 trait: 0 改

---

## 8. R173 后续

| R | 任务 | 优先级 |
|---|---|---|
| R174 | apeireth-tool-fetch HTTP 真接 (Tier 1.5 缺项) | 高 |
| R175 | TUI 接入真后端 (评估是否够格) | 高 |
| R176 | Hyperlight 设计实施 (R170+1~5) | 中 |
| R177 | SurrealDB 后端实施 (R171+1~6) | 中 |
| R178+ | GitHub 调研每个模块的优秀项目 | 持续 |
| 最后 | STT / 声纹 / 唤醒词 / 生图 / 图处理 真接 | 最后 |

**R173 仅文档审计**, 0 代码改动。
