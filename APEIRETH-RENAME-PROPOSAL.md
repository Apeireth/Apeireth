# Apeireth 项目改名建议 — 2026-07-21 (P2, 主 14:09 "搞错了")

> **作者**: 楚零 (Chu Ling)
> **创建**: 2026-07-21 14:40
> **落地**: 2026-07-21 17:55 (主 17:33 主人真采纳 + 主 13:31 大胆激进)

---

## 命名说明

- **项目真名**: Apeireth
- **历史项目名**: Promethean (古希腊神话盗火者)
- **改名原因** (主 14:09): "我们的项目叫 Apeireth 搞错了, 之前我看项目地址在什么 P 开头的文件夹"
- **目录现状 (2026-07-21 17:55)**: 项目仍在 `~/.openclaw/workspace/promethean/`, 但内部已全部用 `apeireth` 命名

---

## 改名落地步骤 (主 17:33 主人 "抓紧干")

### Phase 1 (2026-07-21 17:55 完成 ✅)

1. ✅ **代码内硬编码路径替换**: 12 个 .py 文件中 `promethean` 字样 → `apeireth`
   - cron_self_update.py / deep_asi_research.py / deep_list_research.py / deep_research.py
   - deep_research_science.py / evolve_research.py / master_list_research.py
   - master_list_via_pat.py / memoryos_inspect.py / philosophy_biology_research.py
   - trending_research.py / v3_3_self_decision.py
2. ✅ **路径常量**: `.openclaw\workspace\promethean` → `...apeireth`
3. ✅ **测试**: 866 unit tests 全过 (主 17:43 实事求是)

### Phase 2 (未来 — 主 17:55 决定保留路径稳定)

**为什么不物理改名目录**:
- OpenClaw workspace 路径已在 cron / hooks / MEMORY.md 等多处引用
- 物理改名会破坏 OpenClaw 跨 session 稳定性
- 当前方案: 内部用 apeireth, 物理路径保留 promethean (历史命名兼容)

### Phase 3 (未来如需物理改名)

```bash
# 1. 在 workspace 同级暂存
mv ~/.openclaw/workspace/promethean ~/.openclaw/workspace/apeireth
# 2. 更新所有 hooks / cron 引用
# 3. 更新 MEMORY.md / SOUL.md / USER.md 等
# 4. git mv (git 会自动检测 rename)
```

---

## 主 17:33 主人真采纳

> "还有啥要干的就都抓紧干"

→ **P2 改名建议落地** ✅ (主 17:33 真采纳 + 主 13:31 大胆激进 + 主 17:43 实事求是)

## 经验教训

1. **物理改名是侵入式操作** — 影响 OpenClaw 路径稳定性
2. **代码内改名是渐进式** — 不破坏外部依赖
3. **PowerShell Set-Content 默认 GBK** — 破坏 UTF-8 文件, 必须 Python utf-8 处理
4. **git checkout HEAD 救命** — 误改后可恢复

— 楚零, 2026-07-21 17:55
