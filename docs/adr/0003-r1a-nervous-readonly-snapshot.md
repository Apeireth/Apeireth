# 0003. R1a 神经系统·只读聚合（spec）

- **Status**: Accepted（owner 授权代裁 2026-10-20「按你的建议来吧」；实施归生物线，排期由该线确认）
- **Date**: 2026-10-20
- **Owners**: 架构线（spec/验收）· 生物线（实施）
- **Supersedes**: —
- **契约影响**: 无新增依赖边 / 无新增 crate / 无新增开关 / 默认值 0 变化；仅 `tool.self_status` 追加一个只读节（21 节 → 22 节，旧节语义逐字节不变）
- **例外关联**: —
- **预算净额**: 源码 +1 模块 +1 集成测试（估 600-900 行），结构净额 0

## 1. 背景

14 条生物机制跨 4 个 crate、6+ 接线点、10+ 个 env 开关；`self_status` 里只有承载力 1 条 probe，其余 13 条 `wired:false`；控制面（开关）、观测面（自述）、审计面三套口径互不引用。R1b 要做订阅迁移，但迁移前必须先把"现状"变成一份可核对的只读账本，否则迁移无从验收。

## 2. 决策

**新增一个只读聚合模块，不做任何行为改动**：

- 位置：`crates/engine/runtime-assembly/src/canonical/nervous_snapshot.rs`；
- 入口：由 `self_status_source` 以既有 probe 模式注入 `tool.self_status` 的新节 `capabilities.nervous_system`；
- 数据来源：**只读**现有开关解析（与生产同一解析函数，不新增解析路径）、现有接线点常量、现有观测口；不构造任何机制实例、不改任何开关语义；
- 输出（schema v1）：

```json
"nervous_system": {
  "version": 1,
  "summary": { "total": 14, "production_wired": 11, "half": 1, "unwired": 2, "observed": 1 },
  "mechanisms": [
    {
      "id": "SG-13.neuromodulation",
      "level": "PRODUCTION WIRED",
      "enabled_env": "APEIRETH_ENABLE_BIO_TONE",
      "enabled": false,
      "wiring": "production-assembly",
      "signal_sources": [
        { "id": "tone_feedback", "state": "not_wired", "reason": "真实反馈事件源未接" }
      ],
      "last_observation": null,
      "gaps": ["真实反馈事件源未接"]
    }
  ]
}
```

- **0 装口径**：无数据 = `null` + 原因；未接线 = `wired:false` + 原因；`enabled` 只反映开关实况，不推断可用性；`observed` 只计有真实 probe 的机制（当前 = 1）。

## 3. 实施边界（生物线）

- 允许改：`runtime-assembly/src/canonical/nervous_snapshot.rs`（新）、`self_status_source.rs`（注入）、`tests/nervous_snapshot.rs`（新）、`docs/03-reference/capabilities-matrix.md`（自述合同节数 21→22 同步）。
- 不许改：`organ/**` 机制逻辑、默认开关值、生产装配顺序、其它 21 节语义。
- 测试：①节存在且 `version=1`；②`mechanisms` 恰 14 条且 id 与契约一致；③未接线条目 `wired:false` 且 reason 非空；④无数据字段为 `null` 不伪造 0；⑤既有 21 节快照逐字节不变（golden）；⑥关全部 bio 开关时快照与开一半时 `enabled` 只随开关变化、行为无差。

## 4. 迁移 / 回滚 / 验收

- 单批交付（生物线），无需分阶段；`arch_guard --strict` 0 违规。
- 回滚：删模块 + 删注入 + 恢复 21 节合同文档（5 分钟）。
- 验收：`tool.self_status` 新节可读、14 条全列、6 条测试绿、旧 21 节 golden 不变、0 默认值变化。

## 5. O-6 三阶审查

- **总体最优**：先做只读聚合再做订阅迁移——迁移期需要一份不变量基线，否则"迁移后行为没变"无法证明。拒的替代：①直接 R1b（无基线，回归不可判）②不做聚合、只改文档（文档不承载运行时真值）。
- **系统最优**：放在 runtime-assembly（唯一装配根）而非 organ（机制层不知道谁开它）或 tools（自述的消费端）。拒的替代：①`self_status` 内直接扫 env（观测端不该解析配置）②新 crate（为只读快照建 crate，结构净增）。
- **架构最优**：复用既有 probe 注入模式，不新增 trait/注册表；输出是纯数据，未来 R1b 可直接把它改成订阅状态源。拒的替代：①在 organ 加全局单例（破坏无状态机制）②把快照塞进 metrics 端点（观测面混用）。

## 6. 后续

R1b（订阅迁移）依赖 R2（扩展 ABI）+ R3（配置 schema）；本 ADR 交付后，`nervous_system.observed` 是 R1b 的验收指标（1/14 → 逐波上升）。
