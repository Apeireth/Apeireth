# perception 归档清单（ADR-0005 R4b）

- **归档日期**：2026-10-20
- **裁决来源**：ADR-0005 §3 R4b（owner 2026-10-20 授权；「perception 整包 → 归档，主 workspace 移除该成员」）
- **源提交**：`e17f4acaf928cf6709e8913b286f0491bf36e8ca`（`main`，归档前 HEAD）
- **原路径**：`crates/engine/perception/`
- **归档路径**：`legacy/archived/perception/`
- **crate**：`apeireth-perception`（版本继承 workspace `2.0.0-rc.4`）
- **规模实测**（2026-10-20，`Get-ChildItem -Recurse` + `Measure-Object -Line`）：
  - 20 个文件 / 6,451 行（其中 19 个 `.rs` / **6,419 行**，`Cargo.toml` 32 行）
- **归档原因**：全仓 `Cargo.toml` 无任何 crate 依赖 `apeireth-perception`（仅根 members 自身声明）；
  零依赖者 = 不可达代码。归档后根 workspace members 31 → 30（图像家族仍在，R4a-2 另计）。

## 目录内容

| 路径 | 行数 | 说明 |
|---|---:|---|
| `src/lib.rs` | 74 | 模块声明 + 对 `apeireth_plugin::perception` trait 的 v1 兼容 re-export |
| `src/capture.rs` | 227 | 捕获元数据 |
| `src/memory_link.rs` | 580 | 感知→episodic 记忆链（默认关，`APEIRETH_ENABLE_PERCEPTION` opt-in） |
| `src/normalize.rs` | 500 | 观测归一化 + Attention 策略 |
| `src/observe.rs` | 314 | 观测队列 |
| `src/owner.rs` | 402 | `PerceptionOwner`（默认关、未接线） |
| `src/screen.rs` | 245 | 屏幕事件面 |
| `src/vision/` | 755 | 视觉后端（3 文件，含平台捕获后端 + fail-closed noop） |
| `src/voice/` | 3,159 | 语音面（8 文件：音频帧/会话/VAD/TTS/STT/流帧） |
| `tests/perception_integration.rs` | 163 | 集成测试 |
| `Cargo.toml` | 32 | 清单（workspace 继承） |

## 恢复步骤

1. `git mv legacy/archived/perception crates/engine/perception`；
2. 根 `Cargo.toml` 的 `members` 在 `"crates/engine/runtime",` 之后补回
   `"crates/engine/perception",`；
3. `cargo metadata --no-deps --format-version 1` 复核 members 30 → 31；
4. `cargo check -p apeireth-perception && cargo test -p apeireth-perception`（`Cargo.lock` 由 cargo 重新生成）；
5. 单批回滚等价于 `git revert <R4b-perception-commit>`（移动 + members + 文档标注同批）。

## 许可 / NOTICE

- `license.workspace = true` → 根 workspace 双协议 **Apache-2.0 OR MIT**；本 crate 无独立
  `LICENSE` / `NOTICE` / vendored 源码。
- 依赖 `xcap 0.9.8`、`image 0.25` 为 crates.io 常规依赖（非本仓 vendored 家族），
  第三方 attribution 仍由根 `THIRD-PARTY-NOTICES.md` 统一覆盖；归档不改许可义务。
- 恢复后如重新进入依赖图，需按 `docs/04-internal/license-port/` 口径重跑许可/NOTICE 核对。

## 未随本批迁移（如实登记）

- `docs/03-reference/crates.md`、`docs/01-architecture/architecture.md`、`docs/02-guides/perception-setup.md`
  等仍按归档前口径描述本 crate；R4b 机械范围只改 `founding-design-v2.md §10` 决策行，
  其余文档对齐留给架构线/后续文档批。
- `docs/03-reference/capabilities-matrix.md` 的 perception 行属效率线在途改动，本批 0 触碰，
  按四级口径补「已归档」由该线收口。
- `deny.toml` / `audit.toml` 中经 `xcap → zbus` 传递链登记的 advisory ignore 可能变为
  unmatched；依赖图缩表与 ignore 清理走 Wave 收口，不在本批路径白名单内。
