# 后端增强待办（想法级索引）

> `Status: 📋 待办（2026-10-02）`
> 纪律：本页**只列想法**（一行一事）；细节一律折进同目录小文件（每文件一主题）。
> 开工时认领想法 → 拆批派工 → 细节文件随批更新；想法完成就在本页标 ✅ 并把细节文件归档。

## 想法清单

| # | 想法 | 一句话 | 细节文件 |
|---|---|---|---|
| A | **配额真接口** | 四维配额从"只有类型"变"真执行"——烧钱闸数值化 | [A-quota-real-interface.md](A-quota-real-interface.md) |
| B | **性能极致** | 找写得不好的开刀：冷启动红行、热点、并行 | [B-performance.md](B-performance.md) |
| C | **记忆深化** | consolidation/reflexion/自调校/图谱向量上生产 | [C-memory-deepening.md](C-memory-deepening.md) |
| D | **干活能力** | worktree_apply 受控写 + git 写分级（commit 级走审批） | [D-doing-work.md](D-doing-work.md) |
| E | **安全纵深** | 沙箱对象级判定、审批族归一、多用户权限 | [E-security-depth.md](E-security-depth.md) |
| F | **服务端形态** | headless daemon、多客户端复用、远程访问（T2 中枢） | [F-server-form.md](F-server-form.md) |
| G | **观测与可靠** | 指标数据源、崩溃恢复、备份 | [G-observability.md](G-observability.md) |
| H | **模型面** | 多模型路由/fallback、成本分级 | [H-model-plane.md](H-model-plane.md) |

## 架构定案（2026-10-02 owner 拍板）

**星群为骨、共享为翼**：每户伙伴实例独立（隐私硬边界、性格独立养成），中枢选择性共享**匿名化经验**（教训/技能，不含记忆原文）——既守「它只记得你」的魂，又吃群体智慧。详见 [F-server-form.md](F-server-form.md)。

## 施工纪律

**[CONSTRUCTION-DISCIPLINE.md](CONSTRUCTION-DISCIPLINE.md) 为进场第一课**（多 AI 共同施工宪章：零债/叠加不覆盖/口径诚实/门禁/验证）。

## 排序建议（owner 可改）

施工波序（全都要，分波交付）：**W1 看见与管住**（A 配额+G 指标+E 威胁模型）→ **W2 站住 7×24**（F daemon/多端/远程）→ **W3 多户多端**（E 多用户纵深）→ **W4 能干活**（D）→ **W5 更聪明更省**（C+H）。B 性能与 G 备份为横切纪律随波验收。

**零债开场白**：W1 前先清既有债（src-tauri 告警/登记欠账）。

## 与既有账目的关系

- 性能红行现状见 `reports/benchmark-reproduction.md`；开关权力现状见 `docs/02-guides/capability-switch-power-audit.md`；
- 验证面硬化（Kani 证明收口）持续进行，与 A/E 的安全语义互为弹药；
- 驾驶舱 P2 遥测、对话遥测条的"未接线"槽位=G 的数据源需求清单。
