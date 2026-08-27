# Apeireth Verification Ladder (L0–L5)

> **现状 (2026-08-27)**：本文是 v1 时代（master 线/86-crate）或 reconstruct_v2 过程中的历史快照，正文保留原样。当前基线：默认分支 `main`、13-crate 工作区（`crates/foundation|engine|capabilities|adapters`，见根 `ARCHITECTURE.md` 与 `docs/01-architecture/architecture.md`）、tag `v2.0.0-alpha.1` @ `d6910cf7`；旧 86-crate 代码整体在 `legacy/`（workspace exclude）；v2 下一步见根 `ROADMAP.md` §4。补充：本阶梯与当前 CI 部分脱节（缺失 clippy 三档/miri/audit/deny/rustdoc/13 键守门），以 `.github/workflows/*.yml` 为准。

> Runtime Decoupling: defines the merge-blocking vs environment-dependent
> verification rungs. L4 must NOT block ordinary PRs on a real provider secret.

## Rungs

| Rung | What | Credential | Merge-blocking? |
|---|---|---|---|
| **L0** | compile / static (`cargo check --workspace`, `cargo fmt --check`, `svelte-check`) | none | ✅ yes |
| **L1** | unit tests (`cargo test --lib`, per-crate) | none | ✅ yes |
| **L2** | integration tests (`cargo test --test *`, in-process, no socket) | none | ✅ yes |
| **L3** | local runtime HTTP smoke — real TCP socket, no provider credential | **none** | ✅ yes |
| **L4** | provider live smoke — real inference/SSE against a real model | real API key | ❌ no (env-dependent) |
| **L5** | desktop E2E / visual acceptance (Playwright+Edge, 1280×720 + 1920×1080) | runtime + desktop | ❌ no (env-dependent) |

## L0–L3 = merge-blocking (CI-runnable, no secrets)

- **L0**: `cargo check --workspace`, `cargo fmt --all -- --check`, `pnpm check` (svelte-check), `pnpm build` (vite).
- **L1**: `cargo test --lib` per crate（当前 13-crate 工作区；旧 `apeireth-companion --lib 694+` 为 v1 历史数，代码在 legacy/）。
- **L2**: integration tests（in-process, no socket）——如 `canonical_agent_loop`（17 条：成功环/Deny/RequireApproval/ProviderFailure/ToolFailure）、`canonical_approval_lifecycle`、storage migration tests。
- **L3**: no-key HTTP smoke——当前等价物：`canonical_entry_e2e`（gateway 全链路，测试替身 provider，无真实 key）+ M3A controlled fetch validation（三 OS，无 key）。v1 时代的 `no_key_runtime_smoke`（companion_serve）随 86-crate 工作区移入 `legacy/`。

### CI integration

L0–L3 由以下 workflow 自动运行（`.github/workflows/`）：
- `rust.yml`：`cargo nextest run --workspace --profile ci --locked`（3 OS）+ 13 键测试契约守门
- `rust-lint.yml`：clippy 3 档 + fmt（单独）
- `miri.yml` / `rustdoc.yml` / `cargo-audit.yml` / `cargo-deny.yml` / `coverage.yml`
- `m2b-xv-isolation.yml` / `m2c-xv-shell-validation.yml` / `m3a-canonical-fetch.yml`：三 OS 隔离/Shell/Fetch 验证
- `companion-desktop-ci.yml`：前端（Tauri `cargo check` + `pnpm svelte-check`）

## L4–L5 = environment-dependent (release validation, NOT PR-blocking)

- **L4**: requires a real provider API key (`apikey-ultra.txt` or `APEIRETH_API_KEY`).
  Validates real inference, SSE streaming, model discovery against a live model.
  Run manually before release; never required to pass an ordinary PR.
- **L5**: requires desktop runtime + a display. Playwright+Edge visual smoke
  (Chat / Sessions / Activity / Tools / Memory / Settings / RuntimeModal) at
  1280×720 and 1920×1080. Run manually before release.

## Why L4 is not a PR blocker

Provider availability is orthogonal to core runtime compliance. Requiring a
real model key on every PR would couple provider reachability into the merge
gate — a provider hiccup would block unrelated PRs. The Runtime Decoupling
design (Core Runtime vs Provider Runtime) makes this separation clean:
`/health` reports core health independently of provider, and capability
`available` reflects provider state without making the runtime dead.
