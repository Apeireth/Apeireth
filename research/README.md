# Apeireth 研究工作区（research/）

> 建立：Phase 0（2026-09-04）· 依据 `_research_mem/ra/00-master-plan.md` §5
> 基线 commit：`ede73515cab5c4b2bc5dd4fc03ada7e97de35fc5`

## 定位

- 实验与产品代码分离（铁律 4）：本目录承载实验配置、baseline、指标定义与研究日志，**不进入主 workspace 默认构建**（无 Cargo.toml，cargo 忽略）。
- 纪律（铁律 1–3）：新机制一律 `Research* / Shadow* / Experimental*` 前缀、默认关闭；每个实验有学术账本（问题/假设/状态/引用/baseline/局限）；报告不得使用未经实验验证的"已证明/SOTA/超越业界"表述。

## 目录

| 目录 | 用途 |
|---|---|
| `datasets/` | 实验数据集（原始 + 派生），大文件不入 git |
| `baselines/` | 冻结基线记录（commit / 测试数 / 指标快照） |
| `metrics/` | 统一指标层定义与计算约定 |
| `runners/` | 实验运行器(已实现:`apeireth-research-runner`,独立 cargo 项目,合成基准 + 效用-成本曲线 + bootstrap CI) |
| `configs/` | 实验配置（seed、参数矩阵、config hash 约定） |
| `reports/` | 实验报告（每实验一份：假设/设置/结果/局限/失败判据） |
| `logs/` | JSONL 研究日志（schema 见 `logs/README.md`） |

## JSONL 研究日志

所有实验事件写 `logs/<experiment>-<confighash>.jsonl`，每行一条事件，schema 见 `logs/README.md`。核心字段：`ts, seed, config_hash, experiment, event, payload`。

## 四道合入闸门（任何实验结论进入产品前）

1. 等价性门：旧 API / 旧默认路径行为一致；
2. 性能门：P50/P95 / 内存 / token 不劣化；
3. 证据门：≥2 benchmark 优于确定性 baseline 且报告置信区间；
4. 术语门：无未经验证的"已证明 / SOTA / 超越业界"表述。
