# Phase 5 审批状态机 —— 形式验证总览 (RA-5)

三路互证的证据链, 全部指向同一条不变量 (InvA 无双副作用 / InvB 批准意图不丢 /
InvC 效果不确定强制 fail-closed):

| 路 | 方法 | 状态 | 强度 |
|---|---|---|---|
| ① 模型级故障注入 | `research_approval_sm.rs` fault injection (6 持久化点 × 100 轮随机事件/崩溃交错) | ✅ 0 违例 (Phase 5 交付时) | 随机采样, 概率保证 |
| ② TLA+/TLC 穷举 | `tla/ApprovalSM.tla` + TLC 2.16 (JDK 17) | ✅ 2026-09-05: 单记录 36 状态 / 三记录 3164 状态全通过 | **全可达状态** 枚举, 指纹碰撞 2.9E-12 |
| ③ Kani 机器证明 | `kani/` mirror crate (零复制 `#[path]` 包含 canonical) + GitHub Actions | ✅ 2026-09-05: harness 1 (终态锁) / 2 (executed 单调) 已证; harness 3 见 run 记录 | 符号执行, 有界展开 (unwind 32) |

## 关键工程事实 (2026-09-05)

- **Kani 平台限制**: 本机 Windows 无 Kani (仅 Linux/macOS) → CI ubuntu runner 跑;
  TLC 本机 JDK 17 跑通。
- **rust-version 墙**: workspace 要求 rustc 1.97, Kani 0.67 (crates.io 最新) 自带 nightly 1.93
  → 用 `kani/` mirror crate (`#[path]` 直接包含 canonical 源文件, 零复制漂移) 绕开。
- **SipHash 符号展开爆炸**: Kani 把 String 字节缓冲当符号长度, HashMap 哈希循环无限展开
  (CI 日志实测 1900+ 迭代 × 2s) → 3 个 harness 加 `#[kani::unwind(32)]` (仅 cfg(kani) 生效,
  生产零影响)。真实执行用具体短键, 32 覆盖全部真实执行; 属**有界模型检查**口径, 与②互补。
- **TLC 枚举要求**: primed 变量须可赋值形式 (`v' \in S`), 规格内 Tick 已等价改写并注释。

## 复现

```powershell
# ② 本机 TLC (JDK 17 + tla2tools-1.7.1.jar, MIT, 自取见 tla/README.md)
java -cp tla2tools-1.7.1.jar tlc2.TLC -config ApprovalSM3.cfg ApprovalSM.tla
# ③ Kani CI: .github/workflows/kani.yml (workflow_dispatch 可手动触发)
```

## 诚实边界

- 三条路都建立在**同一份状态机语义**上 (TLA+ 规格与 Rust 实现 1:1 对照见 `tla/README.md`
  映射表); 跨路一致性 = 语义编码没写错 + 不变量确实成立。
- 崩溃模型是 durable 前缀布尔抽象, 未建模真实 fsync/部分写 (见 canonical 文件学术账本)。
- Kani 为有界展开口径 (unwind 32), TLC 为有限模型 (REC ≤ 3, 时钟有界);
  无限模型归纳泛化未做, 如实标注。
