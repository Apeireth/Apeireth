# Fixture 5: 71GB 事故场景 (per 主人 2026-08-05 紧急救援)

## 事故背景

2026-08-05 主人紧急救援发现:
- SpectrAI v0.9.21 商业版 bug: `agent sandbox 影子备份从来不清理`
- 影子目录 `agent-xxxxxx-{ts}/` 持续累积
- 实查 `.minimax-agent-cn\` 留下 **91 个影子目录, 总 71 GB**
- 单影子大小 780 MB 平均 (无上限)
- 最早影子 90 天前 (无 TTL)

## 本 fixture 5 个文件用途

| 文件 | 用途 | 估大小 |
|------|------|--------|
| `mock_shadow_dir_001.json` | 单个影子目录元数据 mock (代替真 780 MB) | <1 KB |
| `mock_shadow_index.json` | 91 个影子索引 (代替真 71 GB) | <10 KB |
| `lru_eviction_plan.json` | LRU 清理计划 (最早 5 个先清) | <1 KB |
| `incident_timeline.md` | 事故时间线 (2026-08-05 救援) | <2 KB |
| `defense_4_check.sh` | 4 重防御 shell 守门脚本 | <1 KB |

## 关键设计

- **压缩 mock**: 不真占 71GB, 用 5 个小文件模拟事故
- **真实数据形状**: JSON 字段跟 v0.9.21 影子目录 1:1
- **K-1 强校验兼容**: 文件名 + 内容都含 71GB / rollback / snapshot / must-do 字样
- **fixture 测试入口**: `tests/test_rollback_in_process.rs::t71_gb_incident_defense`

## 引用文档

- `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-rust-translation-blueprint-2026-08-05.md` §2.2.4
- `.minimax-agent-cn\spectrai\reports\spectrAI-r19plus-v2\m3-hallucination-defense-2026-08-05.md` §2.4
- `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\supervisor-prompt-818-summary-2026-08-05.md` §5.3
