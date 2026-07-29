# Cross-Small-Model CI Report (✅ ALL PASS)

## Summary
- 模型数: 1
- 通过数: 1
- available 数: 1
- 平均 subscore: 0.8750
- PASS 阈值: subscore >= 0.5

## HQB 4 维结果 (主 18:52 HARNESS §2.3)

| 模型 | family | available | SC | NR | EV | CDT | subscore | PASS | 推理次数 | 耗时 (s) |
|------|--------|-----------|-----|-----|-----|------|----------|------|----------|----------|
| fixture-7b-v1 | fixture | ✅ | 1.0000 | 1.0000 | 0.5000 | 1.0000 | 0.8750 | ✅ | 24 | 0.00 |

## CDT 跨域迁移 (主 18:52)

| 模型 | code | math | reasoning | creative |
|------|------|------|-----------|----------|
| fixture-7b-v1 | 1.0000 | 1.0000 | 1.0000 | 1.0000 |

## 哲学守门

- 主 17:58+20:46 不假装: adapter 加载失败 → is_available=False, 不混入 PASS
- 主 17:43 实事求是: subscore 来自 4 维真测, 不 hardcode
- 主 19:33 走在前人经验上: 借鉴 V36 HQB + V160 HQB 4 dims + V1085 HQB core

