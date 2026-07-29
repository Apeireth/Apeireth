# Cross-Small-Model CI Report (⚠️ 2/4 PASS)

![ci](https://img.shields.io/badge/cross-small-model-ci-2/4_pass_·_lift_-0.0125-yellow.svg)

## Summary
- 模型数: 4
- 通过数: 2
- available 数: 2
- 平均 subscore: 0.4344
- PASS 阈值: subscore >= 0.5

## HQB 4 维结果 (主 18:52 HARNESS §2.3)

| 模型 | family | available | SC | NR | EV | CDT | subscore | PASS | 推理次数 | 耗时 (s) |
|------|--------|-----------|-----|-----|-----|------|----------|------|----------|----------|
| text2vec-base-chinese | embedding | ✅ | 1.0000 | 1.0000 | 0.5000 | 0.9500 | 0.8625 | ✅ | 24 | 10.10 |
| fixture-7b-v1 | fixture | ✅ | 1.0000 | 1.0000 | 0.5000 | 1.0000 | 0.8750 | ✅ | 24 | 0.00 |
| real-qwen | qwen | — | 0.0000 | 0.0000 | 0.0000 | 0.0000 | 0.0000 | ❌ | 0 | 0.00 — err: env APEIRETH_QWEN35_PATH not set (主 17:58 不假装: 未提供 local_path → 跳过真模型) |
| real-llama | llama | — | 0.0000 | 0.0000 | 0.0000 | 0.0000 | 0.0000 | ❌ | 0 | 0.00 — err: env APEIRETH_LLAMA31_PATH not set (主 17:58 不假装: 未提供 local_path → 跳过真模型) |

## CDT 跨域迁移 (主 18:52)

| 模型 | code | math | reasoning | creative |
|------|------|------|-----------|----------|
| text2vec-base-chinese | 0.9500 | 0.9500 | 0.9500 | 0.9500 |
| fixture-7b-v1 | 1.0000 | 1.0000 | 1.0000 | 1.0000 |
| real-qwen | 0.0000 | 0.0000 | 0.0000 | 0.0000 |
| real-llama | 0.0000 | 0.0000 | 0.0000 | 0.0000 |

## 跨模型差异 (baseline = fixture-7b-v1)

baseline subscore = **0.8750** (SC=1.0000 NR=1.0000 EV=0.5000 CDT=1.0000)

| target | family | available | ΔSC | ΔNR | ΔEV | ΔCDT | Δsubscore | 备注 |
|--------|--------|-----------|-----|-----|-----|------|-----------|------|
| text2vec-base-chinese | embedding | ✅ | +0.0000 | +0.0000 | +0.0000 | -0.0500 | -0.0125 |  |
| real-qwen | qwen | ❌ | — | — | — | — | — | env APEIRETH_QWEN35_PATH not set (主 17:58 不假装: 未提供 local_path → 跳过真模型) |
| real-llama | llama | ❌ | — | — | — | — | — | env APEIRETH_LLAMA31_PATH not set (主 17:58 不假装: 未提供 local_path → 跳过真模型) |

### lift_summary
- n_targets: 3
- n_loaded: 1
- n_failed: 2
- mean_delta: -0.0125
- max_delta:  -0.0125
- min_delta:  -0.0125

## 哲学守门

- 主 17:58+20:46 不假装: adapter 加载失败 → is_available=False, 不混入 PASS
- 主 17:43 实事求是: subscore 来自 4 维真测, 不 hardcode
- 主 19:33 走在前人经验上: 借鉴 V36 HQB + V160 HQB 4 dims + V1085 HQB core + shields.io badge
- 主 13:31 大胆激进: 跨模型差异 + badge 自动生成 (W3 增强)

