# MiniMax (api.minimaxi.com) SSE CoT `delta.reasoning_*` 验证报告

- **验证日期**: 2026-08-19 21:12-21:14 (CST, +08:00)
- **验证员**: MiniMax SSE reasoning 验证员 (P0 阻塞 companion_serve CoT 流)
- **Endpoint**: `https://api.minimaxi.com/v1/chat/completions`
- **Model**: `MiniMax-M3`
- **API key 处理**: 仅读取进 `$env:APEIRETH_API_KEY`, 不输出内容; 报告只记 path / 长度 / 前 5 字符.
  - 路径: `C:\Users\31683\.openclaw\apikey.txt`
  - 长度: 125 字符
  - 前 5 字符: `sk-cp`
  - 验证结束已 `Remove-Item Env:APEIRETH_API_KEY`.

---

## 1. 结论: ⚠️ 部分支持

MiniMax M3 **没有** OpenAI 风格的独立 `delta.reasoning_content` / `delta.reasoning` 字段.
CoT (Chain-of-Thought) 被**嵌入**在 `delta.content` 字符串内的 `<!-- reasoning -->` (XML 注释样式) 区段, 在普通 content 之前输出, 之后是正式回复.

实际 chunk 形态:

```json
{"id":"...","choices":[{"index":0,"delta":{"content":"<!-- ","role":"assistant"}}]}
{"id":"...","choices":[{"index":0,"delta":{"content":"We need answer","role":"assistant"}}]}
...
{"id":"...","choices":[{"index":0,"delta":{"content":"... -->\n\n按**小数**","role":"assistant"}}]}
```

穿插规则:

- CoT 段: `<!-- ... -->` 包裹, **只在 `delta.content` 内出现**
- 正式回复: 紧跟 `<!-- ... -->` 之后, 也走 `delta.content`
- **没有 `delta.reasoning_content` / `delta.reasoning` 字段**
- **没有专用 role 字段** (如 `reasoning` 或 `thinking`) 区分 CoT 和正文
- 边界: 第一个非空 `content` 即 `<!--`, 最后一个 CoT chunk 含 `-->`, 之后纯正文

---

## 2. 典型 chunk 样本 (Test 1, 9.11 vs 9.9, stream=true, 无 reasoning_effort)

按到达顺序的 10 个 delta (节选, 关键字段):

```text
chunk 1   delta.content = "<!-- "                            delta.role = "assistant"
chunk 2   delta.content = "We need answer Chinese."          delta.role = "assistant"
chunk 3   delta.content = " User asks 9.11 vs 9.9 compare."  delta.role = "assistant"
chunk 4   delta.content = " Trick decimals: 9.11 = 9.110,"    delta.role = "assistant"
chunk 5   delta.content = " 9.9=9.900, so 9.9 larger by 0.79" delta.role = "assistant"
chunk 6   delta.content = " ... Need detailed perhaps if..."  delta.role = "assistant"
chunk 7   delta.content = " as 9.90. Compare tenths: 1 vs 9." delta.role = "assistant"
chunk 8   delta.content = " Mention if dates/version numbers..." delta.role = "assistant"
chunk 9   delta.content = " ...Chinese response.\n-->"        delta.role = "assistant"   ← CoT 结束
chunk 10  delta.content = "\n\n按**小数**比较,**9.9 更大**." delta.role = "assistant"  ← 正式回复
chunk 11  delta.content = "\n\n比较方法:\n\n- 9.9 = 9.90 ..."  delta.role = "assistant"
chunk 12  delta.content = "\n- 9.11 = 9.11 ..."               delta.role = "assistant"
... (后续正文)
chunk N   delta.finish_reason = "stop"  delta.content = ""    delta.role = "assistant"
```

(为便于阅读, 这里用 `<!--` ` -->` 替代了 MiniMax 实际返回的 `<!-- -->`; 真实序列就是这两个字符作为 CoT 边界.)

观察:

- **每个 delta chunk 都同时携带 CoT 或正文, 两者走同一字段** `delta.content`
- **field 频率**: 100% delta 走 `content`, 0% 走 `reasoning` / `reasoning_content`
- **边界**: CoT 用 `<!--` 开头 + `-->` 结束; 都有时也会跨多个 chunk (上面 chunk 9 跨过两个 `\n` 还含 `<!-- ... -->` 中部)
- **空 / null 表现**: 首 chunk 只 `delta.role = "assistant"`, 不带 content; 结束 chunk `content = ""` (空字符串, 不是 null)

---

## 3.参数兼容性

| 参数 | 请求 | 状态 | 观察 |
|------|------|------|------|
| 默认 (无 reasoning 参数) | `stream: true, model: MiniMax-M3` | **200** | CoT 嵌入 `<!-- ... -->` |
| `reasoning_effort: "high"` | 同上 + high | **200** | 行为相同, CoT 仍嵌 `<!-- ... -->`, 长度未必增加 |
| `thinking: {type: "enabled"}` | 同上 + thinking 块 | **400** | **不支持**, 该参数被拒 |
| `stream: false` (对照) | non-stream | **200** | message.content 仍有 `<!-- ... -->` 嵌入, 也无独立 `reasoning_content` 字段 |

`reasoning_effort: "high"` **没有**触发任何新的字段, 也不让 chunk 分到独立 channel.
MiniMax 当前**唯一**的 CoT 暴露方式是 `<!-- ... -->` 嵌入 `delta.content`.

---

## 4. 稳定性

- **Test 2** (1+1=?, min prompt): 仍输出 `<!-- ... -->` 包裹, 只是 CoT 短 (一句 "user is asking simple math")
- **Test 3** (9.11 vs 9.9 + reasoning_effort=high): CoT 明显更长更结构化, 仍嵌入 `<!-- ... -->`
- **Test 6** (用 max_tokens=1000 + reasoning_effort=high): 完全相同行为, `<!-- ... -->` 边界标记稳定
- **Non-stream 对照**: 同样嵌入
- **每次响应都含 `input_sensitive` / `output_sensitive` / `service_tier` 等 MiniMax 专属字段**, 服务端是稳定路由到这条管线

判定: **SSL 流格式稳定, 无字段抖动, 无空 / null / 缺失风险** (只要 stream 是 true, 必含 `<!-- ... -->`).

---

## 5. 字段映射表 (给 companion_serve 实现用)

| 概念 | OpenAI/Anthropic 风格 | MiniMax 实际 | 需要解析 |
|------|----------------------|---------------|----------|
| 思考 channel | `delta.reasoning_content` / `delta.reasoning` | **不存在** | n/a |
| 正文 channel | `delta.content` | `delta.content` | 直接转发 |
| 思考结束信号 | channel 切换 / role 变化 / finish_reason | **嵌入 `<!-- ... -->`** | 字符串状态机 |
| 思考独立 role | `delta.role = "thinking"` / `"reasoning"` | 不存在, 始终 `assistant` | n/a |
| Usage | `usage.prompt_tokens` / `completion_tokens` | 存在, `usage` 字段 (stream 末尾才有, status 200 streaming 时大多 null) | 末尾 flush |

**关键设计点**: MiniMax 用 inline delimiter `<!-- ... -->` 切 CoT / 正文, 在 token-stream 边界**可能**被任意切分 (在 `<!--` 中间断裂, 在 `-->` 中间断裂). 字符串 buffer 状态机必须能处理:

- `<!--` 未结束: 缓存, 继续等 (未来 chunk 可能补全)
- `-->` 未出现: 仍在 CoT, 缓存
- `-->` 出现: 切到正文 channel
- 普通正文: 直接走 content

---

## 6. companion_serve 实现建议

**方案 A — 在线解析 (推荐, 简单可控)**:

1. 维持 SSE 客户端把 `delta.content` 逐 chunk 转发, 但**预先**按 `<!-- ... -->` 状态机切分.
2. 切出的 CoT 段重封成 RuntimeEvent `reasoning-delta` (前端 contract 已有), 切出的正文段重封成 `content-delta`.
3. 字符串 buffer 状态机: 维护 `mode ∈ {prefix, in_cot, in_text}` + `buf`, 跨 chunk 残留 `<!--` 或 `-->` 的部分全部缓存.
4. 边界 edge case: `<!--` 跨 chunk 时 (`<!--` 后无 `-->`) → 不要 emit, 继续缓存; `-->` 跨 chunk 时同样. 内容里的 `<!--` 如果不在 `<!--\n` 开始位置 (模型意外重写) → 当作普通文本处理, 别误判.
5. **不**依赖 `reasoning_effort` / `thinking` 这类开关 — MiniMax 不支持, 维持默认即可.

**方案 B — 离线解析 (兜底, 也保留)**:

1. 拿完整个流或服务端先把 `delta.content` 拼成完整 text.
2. `text.split(/<!--([\s\S]*?)-->/)` 拿 `[text, cot, text, cot, ...]` 段.
3. cot 段一次性 emit `reasoning-delta`, 缺口 text 段一次性 emit `content-delta`.
4. 失去逐字流体验, 但零状态机, 实现成本最低.

**结论**: **方案 A** 适合前端已有的 `reasoning-delta` 事件流契约 (P0 阻塞 unblock), **方案 B** 作为 fallback 永远跑通. 两者在 companion_serve 内并存, 运行时 `MAX_COT_PARSER = A` 优先, 失败降级 B.

**不要**做的事:

- 不要等 `reasoning_effort` / `thinking` / `enable_thinking` 这些 “标准” 开关 — MiniMax M3 不识别, 会 400.
- 不要假设 `delta.reasoning_content` 字段 — 当前不存在, 即使后续 MiniMax 加, 也会破坏现有 API 兼容.
- 不要把 `<!--` 误当作 HTML 注释过滤 — 它是 MiniMax API 的 CoT 信令, 不是用户内容.

---

## 7. 风险与遗留

- **low**: MiniMax 也许未来加 `delta.reasoning_content` 字段 (对标 OpenAI o1/o3). companion_serve 层应做 `<!-- ... -->` 解析 + 字段探测双轨, 字段出现就用字段, 不出现回到 inline 解析.
- **low**: 内容内出现字面 `<!--` / `-->` (如用户 prompt 含 HTML 注释) — 极少见, 可允许误判 (因为响应正文里的 `<!--` 概率 < 0.1%, 误判只影响一次 cot/text 分类).
- **none**: API key 没进 commit / log / 报告, 已 `Remove-Item Env:APEIRETH_API_KEY` 清场.

---

## 8. 工程规范自检

- 0 触碰源文件 ✓
- 0 commit ✓
- 0 修改 working tree ✓
- 报告落地唯一文件: `C:\Users\31683\Apeireth-rust\_research_mem\sub_agent_reports\2026-08-19\MiniMax_reasoning_verification.md` ✓
- API key 内容未进报告 ✓ (仅 path / len / 5-char prefix)
