# scripts/guard_ml/ — Guard ML 评估数据与脚本

> Status: 在库 9 件（脚本 + 场景夹具 + 配对/影子夹具）；4 大可再生导出件已外置（F-02 执行批，2026-10-18）。
> 迁移清单见 [../../docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.tsv](../../docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.tsv)。

## 在库文件（勿迁）

| 文件 | 为什么必留 |
|---|---|
| `scenarios.jsonl` | `crates/engine/guard/tests/security_scenarios.rs` `include_str!` **编译期依赖**（硬红线） |
| `fixture_artifact.json` | 同上，`include_str!` 编译期依赖 |
| `counterfactual_pairs.json` / `negation_pairs.json` | 导出器与评估脚本的输入夹具（`train_eval.py` 记录其 sha256 入证据清单） |
| `shadow_fixture.jsonl` | `analyze_shadow.py` 影子自检夹具 |
| `export_v3.py` / `train_eval.py` / `train_shadow.py` / `analyze_shadow.py` | 评估管线脚本（CI `guard-v3.3.yml` 直接调用） |

## 已外置：4 大可再生导出件（由 export_guard_scenarios 再生）

`action_samples.jsonl`（9.6MB）、`generated_features.jsonl`（7.9MB）、`trace_samples.jsonl`（3.5MB）、
`pair_samples.jsonl`（0.8MB），共 4 件 / 22.0MB，已打包迁
`release-assets/guard-ml-exports-archive-2026-10-18.tar`（F-02 执行批，git 历史不动）。

**再生入口**：

```powershell
cargo run -p apeireth-guard --example export_guard_scenarios --locked
```

- CI（`.github/workflows/guard-v3.3.yml`）每次先跑该导出、再做存在性/隐私断言并上传 artifact，
  库内不存放不影响 CI（校验步断言的是**导出产物**，与库内是否有件无关）。
- 本地跑 `train_eval.py` / `train_shadow.py` 前请先跑同一导出命令补齐上述 4 件。
