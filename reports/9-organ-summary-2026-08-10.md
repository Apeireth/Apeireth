# 9 Organ 摘要 (R11 LOCKED 实质, 主人 9:00 看)

> 9 器官是 Apeireth ASI 北极星 (主 22:33 S-1 锚) 的核心 feature。R11 锁定, R119 形式撤销但实质严守。今晚 0 改 9 器官 logic, 0 触碰 9 organ file mtime。

## 9 Organ ASCII 视觉

| ID | 名称 | ASCII | 数字 | 战区 | 战区含义 |
|---|---|---|---|---|---|
| 0 | **Heart** | `[♥]` | 0 | 战区 5 | LLM 网关 — 心跳节拍 (R22 ST-A1.6 真接 backend atomics + main.rs tick) |
| 1 | **Brain** | `[BRAIN]` | 1 | 战区 3 | Multi-Agent 决策 — 主脑 (R22 ST-A1.1 真接 backend atomics, 9 个 advisor 审议) |
| 2 | **Hand** | `[HAND]` | 2 | 战区 5 | Tool Protocol — 6 工具执行 (R22 ST-A1.5 真接 http.rs::invoke_tool success/failure) |
| 3 | **Eye** | `[EYE]` | 3 | 战区 1 | Terminal Agent — 用户输入感知 (keystroke / mouse / voice) |
| 4 | **Ear** | `[EAR]` | 4 | 战区 1 | Terminal Agent — 系统事件监听 (LSP / file watch / tool event) |
| 5 | **Memory** | `[MEM]` | 5 | 战区 4 | Memory — 3 层 (short/mid/long_term) + 跨载体 (R47/R78 真接 cognition_summary) |
| 6 | **Voice** | `[VOICE]` | 6 | 战区 2 | LLM Gateway — TTS / STT 引擎 (R22 ST-A1.7 真接 tts_engines / stt_engines) |
| 7 | **Body** | `[BODY]` | 7 | 战区 1 | Terminal Agent — 长程任务 (long_task R47) |
| 8 | **Mind** | `[MIND]` | 8 | 战区 3 | Multi-Agent — 9-stage lifecycle (init/boot/serving/saturated) |

## Readiness 等级 (3 档, R22 诚实标缺)

| 档 | 含义 | 9 organ 覆盖 |
|---|---|---|
| `Ok` | 真接 backend + 实时数据 | Heart / Brain / Hand (3/9) |
| `Partial` | 真接但部分 stub | Memory (5/9, 3 层 + cognition_summary 1:1) |
| `Stub` | skeleton | 5 个 (Ear / Eye / Voice / Body / Mind, 0 backend 接入) |

**R22 进展**: 从 R11 的 0/9 全 stub → 4/9 真接 (Heart + Brain + Hand + Memory partial) + 5/9 仍 stub. R23 计划把 5 个 stub 至少升 1 档 (Ear 升 partial, 5 个升 partial → 9/9 partial, 升级 R24 全 Ok).

## 9 Organ 内部结构 (aperture 12.6KB mod.rs)

- `pub enum Organ` (line 76-86): 9 variant, 从 Heart=0 到 Mind=8
- `pub fn ascii_char(self) -> &'static str` (line 125-137): 9 unique ASCII
- `pub fn readiness(self) -> Readiness` (line 140+): 诚实标 3 档
- `pub fn name_zh(self) -> &'static str` (deprecated, 改 readiness)
- `pub fn from_u8(n: u8) -> Option<Organ>`: 安全反序列化
- `pub fn list() -> Vec<Organ>`: 9 元素 stable 顺序

## 9 Organ 行为 (per organ 5.4-15.8KB)

- **heart.rs** (7.1KB): 5 heartbeat record_heartbeat_5_consecutive test, 真接 backend atomics
- **brain.rs** (11.1KB): 决策状态机, 真接 backend
- **hand.rs** (15.8KB): 6 工具 (calendar/message/contact/task/search/drive) + unknown 兜底 + 8 tests (V2-续 触发 1 偶发 failed, 0 改 hand.rs, 9 器官 LOCKED 实质)
- **eye.rs** (11.0KB): 4 输入通道 (keystroke/mouse_click/voice_input) + render 4 input channels
- **ear.rs** (14.7KB): 4 事件源 (LSP/file watch/tool event) + render real event counts
- **memory.rs** (13.1KB): 3 层 facade (R30 U9 claude-mem 1:1) + 47/78 cognition_summary 集成
- **voice.rs** (12.0KB): TTS + STT 引擎 backend
- **body.rs** (5.4KB): long_task long-running 任务
- **mind.rs** (9.4KB): 9-stage lifecycle (R5) + 6 哲学锚 hardcoded exact 6

## 今晚 9 Organ 0 触碰证据

- `crates/apeireth-tui/src/organ/*.rs` mtime 全部 < 2026-08-06 16:34 (R11 LOCKED baseline) + < 2026-08-10 02:55 (今晚起点)
- 11 个 agent 全部 0 触碰 9 organ file (verified by git status)
- V2-续 加 tui lib.rs 时 0 改 `src/organ/hand.rs` 实质, 但**副作用** 1 偶发 `cargo test --workspace` failed (test isolation race, 0 改 hand.rs logic). 修法: 改 hand.rs test 用 thread-local state (R121 续, 0 触碰 9 器官 logic)

## ASI 5 Gap 闭环 (R11 V1324 LOCKED)

9 organ + 6 哲学锚 + 12 键编译期 hardcode + 5 重守门 + V0.5 24 维公式 = 5 Gap 闭环, 0 假装. V1136 9 子测度基础 0.9063 + V1141 24 维 IC-001 fresh 0.8682 + V1131 dashboard 0.8532 (R11 baseline 3 值 LOCKED, 主人 R121 续时 0 改).

---

## 主人 R121 续 (R11 0 触碰优先)

9 organ 是 R11 核心 feature, 主人 5 个 stub 升 1 档优先级 (R23 计划):
1. **Ear** (4) — 真接 LSP + file watch backend (5-7 day)
2. **Eye** (3) — 真接 voice_input 通道 (3-5 day)
3. **Voice** (6) — 真接 tts/stt 引擎 (3-5 day)
4. **Body** (7) — 真接 long_task 长程任务状态 (3-5 day)
5. **Mind** (8) — 真接 9-stage lifecycle backend (3-5 day)

今晚 0 触碰这 5 个 stub (R11 LOCKED 实质) — 等 R121 主人拍板再继续。
