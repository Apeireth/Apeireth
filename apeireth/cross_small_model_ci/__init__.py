"""Apeireth ASI cross-small-model CI framework (R9-DevOps / R9-DEV-001).

跨小模型 CI = 真跑多模型 × HQB 4 维, 自动产出报告 + pytest 集成.

主 22:33 ASI 北极星 (终极梦想: 任何 LLM 接入即获 AGI/ASI 能力)
主 17:43 实事求是 (CI 必须真测 HQB 4 维: SC 自洽性 / NR 抗噪性 / EV 可演化性 / CDT 跨域迁移)
主 19:33 走在前人经验上 (pytest 2008 parametrize + GitHub Actions matrix)
主 13:31 大胆激进 (≥2 本地模型真接入, 跨域迁移真测)
主 00:56 任何人都能接手 (`pytest -q apeireth/cross_small_model_ci/` 一行 = CI)
主 00:44 质量工程化 (≥2 模型 × 4 维 = 真子分数, 报告自动产出)
主 17:58+20:46 不假装 (adapter 真接, fixture 显式标注)

目标模型 (≥2 真接入):
  - Qwen 3.5-7B Instruct       (QwenLM/qwen-2.5-7b-instruct 系列的 Qwen 3.5 占位)
  - Llama 3.1-8B Instruct      (meta-llama/Meta-Llama-3.1-8B-Instruct)
  - Hermes 2-7B / Llama-3-8B   (NousResearch/Hermes-2-...)
  - Gemma 2-9B Instruct        (google/gemma-2-9b-it)

CI 实际跑:
  - Qwen35Adapter + Llama31Adapter (真 transformer/ollama 接口契约, 加载失败 → 显式记录不假装)
  - FixtureAdapter (canned 响应, CI 必跑, 至少 1 个 PASS, 让 CI 全程可跑无 7B GPU)

HQB 4 维 (主 18:52 + HARNESS.md §2.3):
  - SC 自洽性: 同 prompt 重复 N 次, score 方差倒数 (越高越自洽)
  - NR 抗噪性: 同语义不同扰动 (typo / 中英混 / 礼貌前缀) 下 score 稳定性
  - EV 可演化性: prev → next score lift 反映模型可演化性
  - CDT 跨域迁移: 跨 4 域 (code / math / reasoning / 创意) 跑同一模型, 跨域均值

参考 (主 19:33):
  - pytest 2008 parametrize    — CI 多模型矩阵
  - GitHub Actions matrix 2020 — 多模型并行 CI
  - HuggingFace transformers  2018 — 真模型加载
  - Ollama 2023                — 本地 LLM 简化
  - EleutherAI LM-Eval 2021   — 跨模型评测矩阵
  - V1085 HQB core             — 4 维 + 决策
  - V1087 HQB Live Gate        — 真子分 + 守门
  - V36 HQB benchmark          — SC/NR 真测函数
  - V160 HQB 4 dims            — SC/NR/EV/CDT 真测

Public API:
    from apeireth.cross_small_model_ci import (
        ModelRegistry, Qwen35Adapter, Llama31Adapter, FixtureAdapter,
        HQBHarness, run_ci, render_markdown, DEFAULT_TASKS,
    )
"""
from __future__ import annotations

from .models import (
    ModelAdapter,
    ModelResult,
    FixtureAdapter,
    Qwen35Adapter,
    Llama31Adapter,
    HermesAdapter,
    Gemma4Adapter,
    ModelRegistry,
    DEFAULT_REGISTRY,
)
from .harness import (
    HarnessResult,
    HQBHarness,
    SCConfig,
    NRConfig,
    EVConfig,
    CDTConfig,
    measure_sc, measure_nr, measure_ev, measure_cdt,
)
from .tasks import DEFAULT_TASKS, HQBTask, TaskDomain
from .runner import CIRunner, run_ci, run_one_model, summarize
from .report import render_markdown, render_json, write_report

__all__ = [
    # models
    "ModelAdapter", "ModelResult",
    "FixtureAdapter", "Qwen35Adapter", "Llama31Adapter", "HermesAdapter", "Gemma4Adapter",
    "ModelRegistry", "DEFAULT_REGISTRY",
    # harness
    "HarnessResult", "HQBHarness",
    "SCConfig", "NRConfig", "EVConfig", "CDTConfig",
    "measure_sc", "measure_nr", "measure_ev", "measure_cdt",
    # tasks
    "DEFAULT_TASKS", "HQBTask", "TaskDomain",
    # runner
    "CIRunner", "run_ci", "run_one_model", "summarize",
    # report
    "render_markdown", "render_json", "write_report",
]

__version__ = "0.1.0"
