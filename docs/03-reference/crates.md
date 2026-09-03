# Apeireth Crate Index

This index lists the 16 members of the root Cargo workspace. The independent
Tauri shell is documented with the frontend, and `legacy/` is excluded from
the product workspace.

## Foundation

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-core` | `crates/foundation/core` | 9 哲学锚、13 键判别词汇表、3 项不可变脊柱 (Self-Disable / L0 HA / 13 键 Verdict Cache)、Stable domain primitives, IDs, events, lifecycle, and clock vocabulary |
| `apeireth-protocol` | `crates/foundation/protocol` | Canonical normalized requests/results and provider protocol translation |
| `apeireth-plugin` | `crates/foundation/plugin` | Plugin lifecycle, capability descriptors, registries, and provider/tool contracts |
| `apeireth-governance` | `crates/foundation/governance` | Allow, deny, approval, OWASP ASI-01 工具描述投毒审计 (`tool_desc_audit`)、外部不可信内容信封包裹与逃逸中和 (`untrusted_mark`)、8 类 PII 检测与 `EnvSecret` 行解析 (`input_security`)、4 阶信任多尺度滑动窗口限流 (`rate_limit`) |
| `apeireth-credentials` | `crates/foundation/credentials` | `SecretString` / `SecretBuf` 内存零化 (Zeroize)、Fail-closed 凭据审批门控 (`CredentialGate`)、Keyring/Env 解析 |
| `apeireth-orchestration` | `crates/foundation/orchestration` | Multi-agent coordination, 7 Advisor 结构化辩论协议 (`council`)、Lumi_Nox 双 AI 发言权仲裁机 (`speech_arbiter`)、NemesisBot Prompt Cache 字节级稳定器 (`prompt_stabilizer`) 与环境自适应状态机 (`ambient_context`) |

## Engine

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-runtime` | `crates/engine/runtime` | 规范会话内核、多提供商路由、5 触发源二叉堆心跳调度器与 FlowLock 心流锁 (`heartbeat`)、DeepSeek Harness-R1 失败自进化修补 (`harness_patch`)、治理执行闭环与 Trace |
| `apeireth-provider` | `crates/engine/provider` | Anthropic, MiniMax, and OpenAI-compatible provider capabilities |
| `apeireth-storage` | `crates/engine/storage` | SQLite pool, migrations, storage configuration, and errors |
| `apeireth-memory` | `crates/engine/memory` | 五维时空记忆与 Browser 导出 (`five_dimensional`)、双时态事实图谱与残差特异性检索 (`bitemporal_graph`)、SHA-256 唯一事实链与 Merkle 仲裁 (`arbitration`)、6 阶段昼夜做梦引擎 (`dreaming`)、元思维递进思考链 (`meta_thinking`)、活维基知识编译与反熵 Lint (`wiki_fs`) |
| `apeireth-perception` | `crates/engine/perception` | 多模态感知体系: MiniMax LIVE 高保真 TTS 适配器与 3D PAD 情感语气调制 (`minimax_tts`)、Whisper 语音识别与 Xcap 屏幕捕获 |
| `apeireth-organ` | `crates/engine/organ` | 9 cognitive organs (W1..W3 world models, E4 curiosity, F1 emotion, F4 hypothesis, F6 values, E7 emergence, memory merger, persona tone synthesizer) |

## Capabilities

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-tools-canonical` | `crates/capabilities/tools` | 事务级多文件打补丁与自动回滚 (`apply_patch`)、前置路径/命令拦截与后置凭据出站绊线 (`guardrail`)、标准 JSON-RPC 2.0 MCP 客户端 (`mcp`)、ProcessExecutor 安全容器 (Windows Job Object / Linux cgroups) 与大文本溢出安全分页 (`spill`) |

## Adapters

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-gateway` | `crates/adapters/gateway` | 8 帧全双工 WebSocket 协议、SentenceDivider 标点流式分句器 (TTFAB < 300ms) 与毫秒级 Barge-in 语音插话打断控制器 (`duplex_gateway`, `barge_in`) |
| `apeireth-cli` | `crates/adapters/cli` | 统一开发者命令行与会话执行入口 |
| `apeireth-sdk` | `crates/adapters/sdk` | 跨平台安全客户端与 SDK 接口定义 |

## Independent frontend workspace

`frontend/companion-desktop/` contains the Svelte 5 UI and thin Tauri 2 shell.
It is deliberately outside the root Cargo workspace and is checked by
`.github/workflows/companion-desktop-ci.yml`.

## Excluded historical material

`legacy/donor/`, `legacy/archived/`, and `legacy/frozen/` contain historical
implementations and references. They are not product crates and current code
must not depend on them. The former nested `reconstruction_v2/` workspace was
removed from git after its useful ideas were captured in the root workspace;
an untracked local directory may remain on disk and is safe to delete.
`crates/_archived/` holds untracked local build leftovers and is not
repository content.
