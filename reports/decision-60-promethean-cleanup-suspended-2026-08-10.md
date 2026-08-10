# Decision-60: promethean/ 全删挂起 (per 主人 22:06 "先放着, 回头我删")

**Date**: 2026-08-10 22:07
**Author**: Mavis (mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**触发**: 主人 22:06 拍板"我记起来了,这不是你的工作目录吗,实际上就是minimaxcode在占用,那就先放着,回头我删" → 撤销 22:04 授权 Mavis 删, 挂起等主人自执行
**关联**: decision-44 (33 核心待删 + Safety policy 阻挡) + decision-50 (39 个全 done) + decision-59 (promethean/ 全删方案 + 脚本) + decision-60 (本决策, 挂起)

---

## 0. 一句话

**主人 22:06 拍板"我记起来了,这不是你的工作目录吗,实际上就是minimaxcode在占用,那就先放着,回头我删" → 撤销 22:04 授权 Mavis 删, 挂起 promethean/ 全删方案. 原因: Mavis harness (minimaxcode) working dir 仍指向 `.openclaw\workspace\promethean\Apeireth-rust\`, 这就是 5 个 sub-agent (P0-3 + P2-3 + P4-1 + P8-2 + P7-3) 报告里都提到 "bash 工具死锁, working dir 不存在" 的根因. 整合 #4 commit 19:41 后主仓挪到 `Apeireth-rust/`, 但 harness working dir 还指向旧位置, 所以 22:02 删时 "另一进程使用此文件" 错. 主人决定挂起, 回头自执行 (关闭 minimaxcode 后再删). 0 主动 IM 打扰, 跑过夜 22:07.**

---

## 1. 根因分析 (per 主人 22:06 提示)

### 1.1 minimaxcode (Mavis harness) working dir
- **当前 working dir**: `.openclaw\workspace\promethean\Apeireth-rust\`
- **整合 #4 commit 后新位置**: `Apeireth-rust\`
- **历史**: 19:30 主人把 `.git` 从 `promethean/.git` 挪到 `Apeireth-rust/.git` (per 决策 #46), 19:41 主人自执行整合 #4 commit `abf12243` (per 决策 #48), 19:48 主人删了 `promethean/Apeireth-rust/` mv 残留 (per 决策 #50), 但 **Mavis harness 的 working dir 还指向旧位置**

### 1.2 5 个 sub-agent 死锁根因
- P0-3 (R125-16 retry): bash 100% 失败 + 路径 Mavis 派活时新位置 0 存在 (per 决策 #50 §1.3)
- P2-3 (B1 LOCKED verify retry): bash 工具死锁 (per 决策 #42 §1.1)
- P4-1 (整合 #5 pre-check verify): bash 工具死锁 (working dir `.openclaw\workspace\promethean\Apeireth-rust` 不存在, per P4-1 报告 §3)
- P7-3 (release notes): API error 500 daemon 抖动
- P8-2 (Library Stage 5.1 形式化证明): API error 500 daemon 抖动
- **P0-3 + P2-3 + P4-1 这 3 个** = bash 工具死锁, working dir 旧位置 (主人 22:06 拍板根因)
- **P7-3 + P8-2 这 2 个** = daemon 500 抖动 (无关 working dir)

### 1.3 sub-agent 应对
- P0-3 / P2-3 / P4-1 都用 read 工具读 `.git/HEAD` + `.git/refs/heads/master` + `.git/logs/HEAD` + `.git/COMMIT_EDITMSG` 4 维内部文件替代 bash (per P2-3 retry §6.3 + P4-1 §3)
- 0 装 PASS 严守 + 8 硬墙 0 越界 verify 100% 落实 (per 决策 #54 + #55 + #57)

---

## 2. 主人 22:06 拍板

**"我记起来了,这不是你的工作目录吗,实际上就是minimaxcode在占用,那就先放着,回头我删"**

### 2.1 撤销 22:04 授权
- 22:04 主人授权 Mavis 删 promethean/
- 22:05 + 22:06 Mavis 尝试 `cmd rmdir /s /q` 删 `promethean/Apeireth-rust/`, 中文错"另一进程使用此文件" (per 22:05 bash 输出)
- 22:06 主人拍板: minimaxcode (Mavis harness) 进程占用, **0 删**, 挂起等主人回头自执行

### 2.2 挂起 (0 主动 IM 打扰)
- promethean/ 全删方案 (per 决策 #59) + 脚本 v1 + v2 都 ready, 但 0 执行
- 主人回头执行步骤:
  1. 关闭 Mavis session (关闭 minimaxcode 进程)
  2. 跑 v1 脚本: `& 'Apeireth-rust\reports\promethean-full-cleanup-2026-08-10.ps1'`
  3. 跑后 verify 4 项 (Test-Path + borrowed-repos + apeireth-debug + new master HEAD = abf12243)
- 主人可以下次 session 时再做

---

## 3. 0 主动 commit + 0 主动 push 严守 (per 决策 #34 + 决策 #48 + 决策 #55 + 决策 #56 + 决策 #57 + 决策 #58)

- **0 主动 commit** (Mavis 整合 #4 commit abf12243 19:41 拍板 done, 整合 #5 commit 时机由 Mavis 拍板)
- **0 主动 push git push** (等主人 1.0 release 配 GitHub remote)
- **0 主动删** (Safety policy 阻挡, per 决策 #44 + #60 主人撤销 22:04 授权)
- **挂起 promethean/ 全删方案** (per 决策 #60 主人 22:06 拍板)

---

## 4. 主人起床后 8 步 (per P0-3 retry 报告 + 决策 #55 §8 + 决策 #57 §2.3 P12-1 准备)

1. 修 session working dir (`Apeireth-rust/`) — **新发现: 这是 working dir 死锁根因, 主人起床后必做**
2. cargo build --workspace
3. cargo test --workspace
4. cargo run --bin apeireth-tui
5. cargo run --bin apeireth-api
6. cargo audit + cargo deny
7. 验证 24 LOCKED 入口签名 0 改
8. 验证 8 硬墙 0 越界 + 0 装 PASS 严守 (✅ 11 + ⏳ 0 + ❌ 1)

**promethean/ 全删 + 关闭 minimaxcode** → 主人起床后必做 (在 8 步之前):
- 关 Mavis session (关闭 minimaxcode 进程)
- 跑 v1 脚本: `& 'Apeireth-rust\reports\promethean-full-cleanup-2026-08-10.ps1'`
- 跑后 verify 4 项
- 重新启动 Mavis session (working dir = `Apeireth-rust/`)
- 然后跑 cargo 8 步 verify

**整合 #5 commit 时机**: 主人起床后 8 步全 PASS + 0 装 PASS verify + 8 硬墙 0 越界 verify, 主人拍板 OR Mavis 自决.

---

## 5. 0 主动 IM 主人 (per gate-discipline)

- 仅 done notification 主动报告 (per 17:56 严守"仅报告 done 状态")
- 0 主动 plain reply on skip ticks (per gate-discipline)
- 0 主动 push / 0 主动 commit / 0 主动删 / 0 主动讨论后续
- 等 41 sub-agent done + 主人起床后 8 步全 PASS, 主动报告整合 #5 commit 时机
- **promethean/ 全删: 挂起, 等主人起床后关 minimaxcode + 自执行脚本**

---

## 6. 顶层 5 个老 cron 影响 (per 决策 #59 §8)

| Cron | 1 min tick / 5 min tick | 路径 | 影响 |
|---|---|---|---|
| dispatch-r125-r125-15-library-immediate | 1 min tick | mvs_ee7ca3badb session | 跑 ASI Python V1472/V1473/V1474 路线? 看 prompt 是否引用 promethean/apeireth/ |
| dispatch-r125-now-min-tick | 1 min tick | mvs_ee7ca3badb session | 同上 |
| watch-r121-1300 | 5 min tick | mvs_ee7ca3badb session | 监督 R121 1300 状态 |
| r123-1-deadline-1725 | 5 min tick | mvs_ee7ca3badb session | R123-1 done 17:26, 0 关联 |
| R120-finalize-1000 | 8 h | mvs_ee7ca3badb session | 跑 8 h tick |

**Mavis 建议**: 这些老 cron 在 mvs_ee7ca3badb session 跑(0 监督),如果它们引用 promethean/apeireth/ 路径,主人关 minimaxcode 后,这些老 cron 也自然挂掉 (session 关闭). **0 主动清**, 等主人下次 session 时自然处理.
