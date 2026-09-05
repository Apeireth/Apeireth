# TLA+/TLC 机器验证 —— Phase 5 审批状态机 (RA-5)

## 是什么

`ApprovalSM.tla` 是 `crates/engine/runtime/src/canonical/research_approval_sm.rs`
的 **1:1 TLA+ 规格**: 七状态审批机 + 崩溃前缀模型 + 三不变量 (InvA/InvB/InvC)
+ 终态锁, 由 TLC 模型检查器**穷举全部可达状态**验证。

与 Rust 侧的映射 (一一对应):

| TLA+ | Rust | 语义 |
|---|---|---|
| `Approve(r)` | `(Pending, Approve)` | Pending→Claimed, durable=TRUE, approved=TRUE, active=r (P2 落账) |
| `Reject(r)` | `(Pending, Reject/Cancel)` | Pending→Rejected |
| `Expire(r)` | `(Pending, Expire{now})` | 时钟越过 EXPIRES_AT → Expired, durable=TRUE (P6 落账) |
| `BeginDispatch(r)` | `(Claimed, BeginDispatch)` | Claimed→Dispatched, executed=TRUE (单调), durable=TRUE (P3) |
| `Complete(r)` | `(Dispatched, Complete)` | Dispatched→Consumed, appended=TRUE, durable=TRUE (P4) |
| `Interrupt(r)` | `(Claimed/Dispatched, Interrupt)` | →Interrupted, durable=TRUE (P5 落账) |
| `RecoverClaimed(r)` | `(Claimed, RecoverClaimed)` | →Interrupted (G3 重开落账语义) |
| `Crash` | `simulate_crash()` | 非 durable 回退 Pending; durable Dispatched∧¬appended→Interrupted; durable Claimed→active |
| `InvA/InvB/InvC` | `inv_a/inv_b/inv_c` | 三条不变量, 语义逐字同构 (含修正版口径) |
| `TerminalLock` | `is_final()` + Next 拒绝 | 终态无出边 |

## 运行方法

```powershell
# 单记录模型 (最小状态空间)
java -cp tla2tools-1.7.1.jar tlc2.TLC -config ApprovalSM.cfg ApprovalSM.tla
# 三记录模型 (并发交错 + active 恢复语义)
java -cp tla2tools-1.7.1.jar tlc2.TLC -config ApprovalSM3.cfg ApprovalSM.tla
```

工具链: TLC 2.16 (tla2tools-1.7.1.jar, MIT License, 本机 JDK 17; jar 不入库)。
jar 下载: https://github.com/tlaplus/tlaplus/releases/download/v1.7.1/tla2tools.jar

## 验证结果 (2026-09-05, 本机)

| 模型 | 生成状态 | 去重状态 | 深度 | 结果 |
|---|---|---|---|---|
| REC={"a"} | 143 | 36 | 5 | **全部不变量通过, No error** |
| REC={"a","b","c"} | 20051 | 3164 | 11 | **全部不变量通过, No error** |

指纹碰撞漏检概率 (TLC 报告): 2.1E-16 (单记录) / 2.9E-12 (三记录)。

结论: 在崩溃交错 (任意时刻 crash) 与任意事件序下, 三不变量与终态锁
**在全部可达状态上成立** —— 与 Rust 侧模型级故障注入 (6 持久化点 × 100 轮,
0 违例) 互相印证。Kani 机器证明走 GitHub Actions (`../../.github/workflows/kani.yml`,
本机 Windows 无 Kani 支持)。

## 诚实标注 (0 装口径)

1. 模型是**纯状态机抽象**: durable 前缀布尔, 未建模真实 fsync/部分写语义;
   时钟离散有界 (0..EXPIRES_AT+1)。
2. TLC 要求 primed 变量为可赋值形式, `Tick` 用
   `now' \in (now+1)..(EXPIRES_AT+1)` 等价表达 `now' > now /\ now' <= EXPIRES_AT+1`
   (语义无损, 仅满足枚举要求, 见规格内注释)。
3. 单/三记录模型检查了 per-record 不变量与 active 恢复; 无限记录数模型的
   归纳泛化未做 (Kani 的 `#[kani::proof]` harness 覆盖了符号级路径, 互补)。
