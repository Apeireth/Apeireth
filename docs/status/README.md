# docs/status — 文档事实体系的机器真源

> `Status: 🔧 P1 试点（2026-10-20 第三线落地；P1c 起 CI 阻塞）`
> 目的：让"状态/默认开关/证据"只有一个真源，且源码一变就能被 `--verify` 发现（STALE）。

## 文件

| 文件 | 性质 | 维护方式 |
|---|---|---|
| [`claims.yaml`](claims.yaml) | **唯一真源**：每条 claim = `id / level（四級口径）/ default / switch_env / sources / tests / verified_at / verify_cmd / owner / next_review` | 人工编辑 + `scripts/docs_truth.py --verify` 校验 |
| [`STATUS.md`](STATUS.md) | 生成物：状态仪表盘 | **禁止手改**；`python scripts/docs_truth.py --gen` |

## 用法

```powershell
python scripts/docs_truth.py --verify            # 核验（P0 report-only：结构/证据缺失才 exit 1）
python scripts/docs_truth.py --verify --block-stale  # STALE 即 exit 1（P1c changed-claim 阻塞口径）
python scripts/docs_truth.py --gen               # 生成 STATUS.md
python scripts/docs_truth.py --gen-badges        # README tests 徽章 ← test-baseline 单一真源
python scripts/docs_truth.py --gen-matrix-01     # matrix §0.1 结果/日期 ← 单一真源（默认 dry-run；有在途改动拒绝写）
python scripts/docs_truth.py --baseline-check --log <cargo-test.log>  # 全量日志汇总 ↔ test-baseline §1 核对
python scripts/docs_truth.py --matrix-check      # matrix 环境变量 / claims 开关互钉自查
python scripts/docs_truth.py --check             # 生成物 drift + README 徽章一致性
```

## 口径

- `level` 取四级口径（`IMPLEMENTED` / `PRODUCTION WIRED` / `DEFAULT ENABLED` / `HARDWARE VALIDATED`）；缺口用 `NOT IMPLEMENTED`。
- `STALE` = `git diff <verified_at> -- <sources>` 非空 → 源码动过而声明未复核；**改源码后的批必须更新 `verified_at` 并重跑 verify**。
- `FAIL` = 证据缺失（source/test 路径不存在、switch_env 在 `crates/` 找不到、字段缺失/重复 id）。
- 生成物 drift、生成区手改 → `--check` 失败。
- P1c 起 CI 阻塞：FAIL / STALE / 生成物（含 README 徽章）drift 都红；REVIEW_DUE 只提示。

## 新增一条 claim 的最短路径

1. 在 `claims.yaml` 的 `claims:` 下追加条目（字段见上）；
2. `python scripts/docs_truth.py --gen`；
3. `python scripts/docs_truth.py --verify --check`；
4. 在交付 commit 正文登记"新增 claim: `<id>`（来源/测试）"。

## 边界（0 装）

- 这里只放**有证据**的状态声明；没有证据的写 `notes` 或进 `claims-evidence-matrix`，不写进 level。
- `claims.yaml` 不是"新的人工真相"：`--verify` 必须回核源码/测试路径，核不到就 FAIL。
- 历史文档不追溯；叙事文档（vision/philosophy）不承载状态声明。
