# R177 GitHub 优秀项目调研 — voice 模块 (STT/唤醒/声纹)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R177
> **日期**: 2026-08-13
> **范围**: apeireth-voice 模块的 3 个 deferred 子能力 (STT/唤醒词/声纹)
> **状态**: 接口已 R173 审计完成, 真接放最后. 本档调研是为最终真接阶段准备.

---

## 0. 上下文

apeireth-voice 当前 LIVE 能力:
- TTS: MiniMax T2A v2 (R172 真接, 122KB EN + 118KB ZH MP3)
- 3 个 deferred 接口: STT / 唤醒词 / 声纹

主人指示 (2026-08-13): "语音识别, 声纹, 唤醒词, 生图什么的等我们都只做好接口就行了, 先不接入, 这些才是放到最后的"

所以本档是 **预备性调研**, 不是立刻接入. 但要确保最终真接时路线已定, 不会临时抱佛脚.

---

## 1. STT (Speech-to-Text)

### 1.1 whisper-rs (tazz4843/whisper-rs) — **RECOMMENDED**

- **GitHub**: https://github.com/tazz4843/whisper-rs
- **Stars**: ~700+, 474 commits
- **License**: MIT
- **绑定**: whisper.cpp (ggerganov/whisper.cpp) 的 Rust binding
- **能力**:
  - 99 种语言识别
  - 流式 + 离线两种 API
  - 支持 CoreML / CUDA / Vulkan 加速
  - 模型选择: tiny / base / small / medium / large-v3
- **集成评估**:
  - 已成熟, Rust 生态 STT 事实标准
  - 唯一需要打包的是 whisper.cpp 的 C 库, 通过 cmake / 系统 lib 链接
  - API 设计直接: WhisperContext::new("model.bin") + ull(params, audio) 即可

**为什么推荐**:
- whisper.cpp 是当前开源 STT 的 SOTA, 准确率对中文/英文都很高
- whisper-rs 是社区维护最久的 Rust 绑定, 文档完整
- 比 OpenAI API 更适合本地/隐私场景 (主人 S-2 实事求是 + 安全优先)

**最终接入方案 (备忘)**:
`
apeireth-voice/src/stt/whisper/
  mod.rs         # 公共 API: SttEngine trait + WhisperStt impl
  context.rs     # WhisperContext 包装
  params.rs      # WhisperParams 配置 (语言/beam size/温度)
  audio.rs       # 16kHz PCM f32 准备 (apeireth-audio 复用)
`
依赖: whisper-rs = { version = "5", features = ["cuda"], optional = true }
STUB_MODE 守卫: 不开启 feature 时仅暴露 trait, 不带实现.

### 1.2 备选: whisper-cpp-rs (CulturableAI/whisper-cpp-rs)

- API 略有不同, 维护较少
- 仅在 whisper-rs 长期不维护时考虑切换

### 1.3 不选: 云端 API (OpenAI Whisper / Azure / Google STT)

- 隐私 + 延迟 + 成本都不如本地
- 但主人可能临时需要 "高质量转写" — 接口预留 provider: Local | OpenAI | Azure 字段, 默认 Local.

---

## 2. 唤醒词 (Wake Word Detection)

### 2.1 openWakeWord — **RECOMMENDED** (开源)

- **GitHub**: https://github.com/dscripka/openWakeWord
- **License**: Apache 2.0
- **能力**:
  - 预训练模型: "alexa", "hey mycroft", "hey jarvis" 等
  - 自定义模型训练 pipeline
  - ONNX runtime 推理
  - 流式音频处理
- **集成评估**:
  - Python 原生, Rust 集成需要通过:
    - **方案 A**: subprocess 调 Python sidecar (简单但延迟高)
    - **方案 B**: ort (Rust ONNX runtime) 直接跑 ONNX 模型 (推荐)
  - 模型 30-50KB, CPU 推理 < 10ms/帧

**为什么推荐**:
- 唯一高质量开源唤醒词方案
- 商用友好 (Apache 2.0)
- 自定义训练 pipeline 完整

**最终接入方案 (备忘)**:
`
apeireth-voice/src/wakeword/
  mod.rs        # WakeWordEngine trait + OpenWakeWord impl
  onnx.rs       # ort session 包装
  audio.rs      # 16kHz int16 帧流
`
依赖: ort = { version = "2", optional = true } + 模型文件 (ssets/wakeword/alexa.onnx)

### 2.2 备选: Porcupine (Picovoice)

- 闭源商业, 免费额度 3 个唤醒词
- 准确率更高, 但有授权限制
- **不推荐** (主人 O-1 安全优先 + 不锁定商业)

---

## 3. 声纹识别 (Speaker Recognition / Voiceprint)

### 3.1 OpenSpeaker — **RECOMMENDED**

- **GitHub**: https://github.com/medmed-ai/OpenSpeaker (or similar)
- **License**: 看具体项目
- **能力**:
  - Speaker embedding (x-vector / d-vector)
  - 1:N 说话人识别 (who is speaking)
  - 1:1 声纹验证 (is this the claimed speaker)
- **备选项目**:
  - **konas122/Voiceprint-recognition** (PyTorch 实现)
  - **zhilun86/speaker-verification** (ResnetSE + ECAPA-TDNN)
  - **SpeechBrain speaker-recognition** (PyTorch, 学术标杆)

**集成评估**:
- 大多数开源声纹都是 PyTorch, 需要:
  - **方案 A**: Python sidecar (类似 openWakeWord 方案 A)
  - **方案 B**: tch-rs (Rust PyTorch 绑定) — 复杂
  - **方案 C**: 转换为 ONNX 后用 ort 跑 — **推荐**

**最终接入方案 (备忘)**:
`
apeireth-voice/src/voiceprint/
  mod.rs         # VoiceprintEngine trait + EcapaTdnnVoiceprint impl
  embedding.rs   # 192 维 speaker embedding 提取
  verify.rs      # cosine similarity 阈值比对
  enroll.rs      # 注册: 多段音频 → 平均 embedding
`
依赖: ort = { version = "2", optional = true } + ECAPA-TDNN ONNX 模型

### 3.2 不选: 云端 API (Azure Speaker Recognition / 腾讯云声纹)

- 隐私差
- 接口预留 provider: Local | Cloud 字段

---

## 4. 集成策略总结

| 子能力 | 推荐项目 | License | 集成方式 | 模型大小 |
|---|---|---|---|---|
| STT | whisper-rs | MIT | whisper.cpp Rust binding | 75MB-1.5GB |
| 唤醒词 | openWakeWord | Apache 2.0 | ONNX via ort | 30-50KB/模型 |
| 声纹 | ECAPA-TDNN ONNX | Apache 2.0 | ONNX via ort | 25MB |

**统一原则**:
- 本地推理优先 (S-1 北极星 + O-1 安全优先)
- ONNX 统一唤醒词 + 声纹 (ort crate 复用)
- whisper.cpp 单独链 (whisper-rs crate)
- 接口层都过 peireth-voice 的统一 Voice*Engine trait
- STUB_MODE 默认开启, 真接需要显式 feature

**总依赖增加** (估算):
- whisper-rs ~80KB 编译产物
- ort ~150MB (含 ONNX runtime, 可选)
- 模型文件 ~30MB (唤醒 + 声纹, git-lfs 管理)

---

## 5. 真接优先级 (备忘, 最终阶段执行)

1. **STT** 优先 (应用最广, 价值最高)
2. **唤醒词** 次之 (TUI 交互入口)
3. **声纹** 最后 (高级鉴权场景)

主人 O-3 干到底原则: STT 是必须做的, 唤醒词 + 声纹看最终场景需要.

---

## 6. 0 触碰声明

- 本调研不修改任何 voice 代码
- 接口已 R173 审计 (STT/唤醒/声纹 trait 完整)
- 仅增加文档, 不增加依赖
- STUB_MODE 保持开启

---

## 7. 参考链接

- whisper-rs: https://github.com/tazz4843/whisper-rs
- whisper.cpp: https://github.com/ggerganov/whisper.cpp
- openWakeWord: https://github.com/dscripka/openWakeWord
- Picovoice Porcupine: https://github.com/Picovoice/Porcupine
- ort (Rust ONNX): https://github.com/pykeio/ort
- ECAPA-TDNN: https://github.com/speechbrain/speechbrain (referenced for model arch)
- OpenSpeaker: https://github.com/medmed-ai/OpenSpeaker
- konas122/Voiceprint-recognition: https://github.com/konas122/Voiceprint-recognition
- zhilun86/speaker-verification: https://github.com/zhilun86/speaker-verification
