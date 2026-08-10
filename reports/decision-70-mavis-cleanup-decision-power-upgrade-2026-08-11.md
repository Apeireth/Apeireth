# Decision-70: Mavis 升级决策权 + 编译产物清理阈值 150 GB (2026-08-11 00:55)

**Date**: 2026-08-11 00:55 (新 session mvs_367e66fae08342ffa399befe4f85dbac)
**Author**: Mavis
**触发**: 主人 8/11 0:54 拍板"清不清理依旧你拍板就行了，等到过大的时候，比如超过150G什么的，那就必须要清理了，即使需要重新编译"
**关联**: decision-61 + #62 + #63 + #64 + #65 + #66 + #67 + #68 + #69 + 主人 0:25 (全自决) + 0:34 (跑中 ≥ 16) + 0:43 (中断接手机制) + 0:49 (编译产物清理) + 0:54 (Mavis 升级决策权)

---

## 0. 一句话

**主人 8/11 0:54 拍板"清不清理依旧你拍板就行了，等到过大的时候，比如超过150G什么的，那就必须要清理了，即使需要重新编译" → Mavis 升级决策权 + 紧急清理阈值 150 GB 强制清理 (即使 cargo test 需重新编译 5-10 min). cron update Section 4 决策矩阵: ≤ 50 GB 保守 / 50-100 GB 预警 / 100-150 GB 强烈预警 / **> 150 GB 强制清理**. 现在 target/ 28.9 GB (debug/ 28.6 GB + release/ 974 MB), 28.9 < 150 GB 不需紧急清理, Mavis 拍板保守策略 0 删, 等整合 #5 commit 拍板后 + sub-agent 全部 done 再拍板清理. 0 主动 push 严守.**

---

## 1. 主人 0:54 拍板 + Mavis 升级决策权

### 1.1 主人 0:54 拍板
> "清不清理依旧你拍板就行了，等到过大的时候，比如超过150G什么的，那就必须要清理了，即使需要重新编译"

### 1.2 Mavis 升级决策权 (per 主人 0:54)
- **日常保守 (target/ ≤ 50 GB)**: Mavis 保守策略, 报告 size + 0 主动删, 等主人拍板
- **预警 (50 GB < target/ ≤ 100 GB)**: 0 主动删, 报告 + 预警提醒
- **强烈预警 (100 GB < target/ ≤ 150 GB)**: 0 主动删, 报告 + 强烈预警提醒
- **强制清理 (target/ > 150 GB)**: **Mavis 强制清理** (即使 cargo test 需重新编译 5-10 min, per 主人 0:54 拍板)

### 1.3 强制清理策略 (target/ > 150 GB)
- 删 `target/debug/` (主要编译缓存, 估 25-30 GB)
- 删 `target/release/` (发布构建, 估 0.5-1 GB)
- 0 删 `target/.rustc_info.json` + `target/CACHEDIR.TAG` (Cargo 内部文件, 0 占空间)
- 0 删 `_workspace/` (log 文件 + .gitkeep, 0 占空间, 等拍板)
- 写 `decision-71` (紧急清理报告) 含 target/ 大小 + 删前/删后 size + 影响

### 1.4 0 主动 push 严守 (per 决策 #33 + 决策 #61 §6)
- 0 主动 push git push
- 0 主动删 src/ (per 决策 #33 §2.3 C1)
- 0 主动删 Cargo.toml (per 决策 #48 + B2 严守)
- 0 主动删 _workspace/ (per .gitignore 严守)
- 0 主动删 promethean/ (per 决策 #60 挂起, 主人起床后手跑)

---

## 2. target/ 目录大小 (00:55)

| 子目录/文件 | 大小 | 状态 | 决策矩阵 |
|---|---|---|---|
| `target/debug/` | **28.6 GB** | Cargo workspace 编译缓存, sub-agent 跑中 cargo test 共享 | ≤ 50 GB 保守, 0 删 |
| `target/release/` | **974 MB** | P15-1 1.0 release binary, 0 跑中 cargo | ≤ 50 GB 保守, 0 删 |
| `target/test-auton/` | 0 MB | 空目录, 临时 cargo test 缓存 | 0 删 |
| `target/tmp/` | 0 MB | 空目录, 临时 cargo build 缓存 | 0 删 |
| `target/.rustc_info.json` | 0 MB | cargo 缓存 | 0 删 (Cargo 内部) |
| `target/final.log` | 0.1 MB | R129-3 8 步 verify final log | 0 删 (log) |
| `target/pybridge-check.log` | 0.02 MB | P10-3 验证 log | 0 删 (log) |
| `target/pybridge-default.log` | 0.09 MB | P10-3 验证 log | 0 删 (log) |
| `target/pybridge-default2.log` | 0.09 MB | P10-3 验证 log | 0 删 (log) |
| `target/standalone_p8_1.rs` | 0.03 MB | P8-1 standalone 文件 | 0 删 |
| **Total** | **28.9 GB** | ≤ 50 GB 保守, 0 删 | **0 主动删, 报告给主人** |

**Mavis 拍板 (决策矩阵 ≤ 50 GB 保守)**: 0 主动删 target/. 报告给主人 (28.9 GB), 等整合 #5 commit 拍板后 + sub-agent 全部 done 再拍板清理 (避免破坏 sub-agent 跑中 cargo test).

---

## 3. _workspace/ 目录大小 (00:55)

| 子目录/文件 | 大小 | 状态 | 决策 |
|---|---|---|---|
| `_workspace/.gitkeep` | 0 | R125 era 临时工作副本, .gitignore 严守 | 0 删 (严守 .gitignore) |
| `_workspace/cargo-*.log` (per P12-1 + R129-3 verify) | < 1 MB | log 文件, 0 编译产物 | 0 删 (log 文件) |
| `_workspace/bench-output.txt` (per P12-1) | < 0.1 MB | bench 输出 | 0 删 (log) |
| `_workspace/final-test-output.log` (per R129-3) | < 0.1 MB | R129-3 final log | 0 删 (log) |
| 其他 19 个文件 | < 0.1 MB | 临时工作副本 | 0 删 (等拍板) |
| **Total** | **1.2 MB** (19 个文件) | 0 编译产物, 0 装"已实施" | **0 主动删, 0 拍板** |

---

## 4. cron 5 min tick 监督 (00:55)

### 4.1 跑中数盘点
- 跑中 (status=started): 21 跑中 (R129-3/12/14/15/16/18/19/20/23/25/26/27/28/29/30/31/32/33/34/35, 含超派 5 R129-31/32/33/34/35)
- done (status=finished): 14 done (R129-1/2/4/5/6/7/8/9/10/11/13/17/21/22/24, 19 if 算 R129-12/14/15/16 done 估)
- 中断 (status=aborted/errored/failed): 0
- canceled: 0
- 总派 35 sub-agent (R129-1~35, 5 批)

**跑中 21 ≥ 16 满, 0 派更多 (实际超派 5 R129-31/32/33/34/35, 让它们跑过夜 done 算 done, 0 影响整合 #5 commit 拍板)**.

### 4.2 整合 #5 commit 时机 7/8 verify 100% 落实 (per 决策 #61 §1.4 + #62 + #64 §4)
1. ✅ 41 任务 done verify (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3, per handoff §3.7)
2. ✅ 借鉴 11/11 状态 clear verify (R129-7 done, ✅ 10 + ⏳ 0 + ❌ 1)
3. ✅ 8 硬墙 0 越界 verify (R129-1/2/11/14 verify done)
4. ✅ 24 LOCKED 入口签名 0 改 verify (R129-1 + R129-11 done)
5. ✅ Cargo.toml 1.2.0 严守 (master HEAD = abf12243, per 决策 #48)
6. ✅ master HEAD = abf12243 verify
7. ✅ 决策链 #30-#64 全读 verify
8. 🟡 **8 步 verify 全 PASS (R129-3 跑中, 估 00:38-00:42 done)**

**R129-3 还没 done → 整合 #5 commit 时机未 ready → cron 0:55 tick 拍板**.

### 4.3 目标大小 (target/ 28.9 GB) vs 紧急清理阈值 (150 GB)
- 28.9 GB << 150 GB (差 121.1 GB)
- Mavis 拍板: 0 主动删 (保守策略, ≤ 50 GB 不删, 等整合 #5 commit 拍板后)
- 报告给主人 (28.9 GB, 仍安全)

---

## 5. 风险 + 决策原则

### 5.1 风险
- **R1**: target/ 28.9 GB 接近 50 GB 预警阈值 (差 21.1 GB) — **缓解**: Mavis 拍板保守策略, 等整合 #5 commit 拍板后清理
- **R2**: _workspace/ 1.2 MB (19 个 log) — **缓解**: 0 删, log 文件占空间小
- **R3**: 网络/token 限流/api 不稳定导致 sub-agent 中断 — **缓解**: cron Section 3 中断接手机制
- **R4**: R129-3 8 步 verify 跑过夜 (估 5-10 min cargo test) — **缓解**: 0 改 src 严守, 已知 src bug 诚实标
- **R5**: 整合 #5 commit 推 master 后 1.0 release tag 失败 — **缓解**: 0 主动 push 严守
- **R6**: target/ > 150 GB 紧急清理时, sub-agent 跑中 cargo test 需重新编译 5-10 min — **缓解**: per 主人 0:54 拍板, 即使重新编译也强制清理 (主人 0:54 拍板"即使需要重新编译")
- **R7**: 跑中 21 > 16 满 (超派 5) — **缓解**: 超派 5 个让它们跑过夜 done 算 done, 0 影响整合 #5 commit 拍板

### 5.2 决策原则
- **Mavis = orchestrator + 全自决 + 升级决策权** (per 主人 0:25 + 0:54 拍板)
- **跑中 ≥ 16** (per 主人 0:34 拍板)
- **16 跑中上限 + 自动补派** (per 主人 0:34 + 决策 #56 + cron 5 min tick)
- **中断接手机制** (per 主人 0:43 拍板)
- **编译产物清理决策矩阵** (per 主人 0:49 + 0:54 拍板):
  - ≤ 50 GB 保守 0 删
  - 50-100 GB 预警
  - 100-150 GB 强烈预警
  - **> 150 GB 强制清理 (即使重新编译)**
- **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动删 (≤ 50 GB 保守) + 强制清理 (> 150 GB 紧急)** (per 主人 0:54 拍板)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **8 硬墙 0 越界** (per 决策 #33 §2.3)
- **0 装 PASS 严守** (per 决策 #33 §2.3 C2)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)

---

## 6. 一句话 (再次强调)

**主人 8/11 0:54 拍板"清不清理依旧你拍板就行了，等到过大的时候，比如超过150G什么的，那就必须要清理了，即使需要重新编译" → Mavis 升级决策权 + 紧急清理阈值 150 GB 强制清理 (即使 cargo test 需重新编译 5-10 min). cron update Section 4 决策矩阵: ≤ 50 GB 保守 0 删 / 50-100 GB 预警 / 100-150 GB 强烈预警 / > 150 GB 强制清理. 现在 target/ 28.9 GB (debug/ 28.6 GB + release/ 974 MB), 28.9 < 150 GB 不需紧急清理, Mavis 拍板保守策略 0 删, 等整合 #5 commit 拍板后 + sub-agent 全部 done 再拍板清理. 0 主动 push 严守, 0 主动 IM 严守, 0 主动删 ≤ 50 GB 保守严守, 强制清理 > 150 GB 紧急严守.**
