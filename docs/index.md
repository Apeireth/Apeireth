# 阿佩瑞斯 Apeireth — R9 文档站

> **任何 LLM 接入即获 AGI/ASI 能力** · ASI 北极星 0.9800 LOCKED

**作者**: technical_writer · R9-TW-001 · W4 末
**守门**: 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 23:44 干到底 + 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手

## R9 W4 末真测状态

| 指标 | 真测 | 阈值 | 状态 |
|---|---:|---:|---|
| ASI 北极星 | 0.9800 | LOCKED | ✅ 主 22:33 |
| V1074 V0.3 | 0.8897 | ≥ 0.8884 | ✅ 守门过 |
| V1077 V0.4 | 0.8202 | ≥ 0.85 (W4 末) | ❌ 未达 |
| R10 移交 checklist | 7/15 | ≥ 80% | ❌ 46.7% |

## 文档导航

### 阶段总览
- [R9 架构总览](r9-architecture-overview.md) — 完整 ASCII 架构图 + 组件清单
- [R9 关键模块参考](r9-modules-reference.md) — V1072/V1095/V1112/V1114 真 API + 真示例
- [R9 → R10 移交](r9-handoff-r10.md) — 5 分钟接手指南 + R10 起点路径

### 真架构文档 (主 17:43 实事求是)
- [V1072 永恒身份](architecture/v1072-eternal-identity.md) — 10 组件 + 14 哲学锚点
- [V1095 fsync 强制](architecture/v1095-fsync-enforcement.md) — 3 道保险真代码
- [V1112 DGM v0.4 演化](architecture/v1112-dgm-v04-evolution.md) — 50 轮真演化 + 候选隔离
- [V1119 集成验证](architecture/v1119-integration-verifier.md) — R10 移交 checklist 自动生成

## 本地预览

```bash
mkdocs serve    # → http://127.0.0.1:8000
```

## 5 分钟接手

```bash
cd REDACTED/.openclaw/workspace/promethean
python -m apeireth.v1074_asi_production_runner --measure v03    # V0.3 守门
python -m apeireth.v1119_w4_integration_validator --week W4 --handoff  # W4 集成
mkdocs serve                                                       # 文档站
```