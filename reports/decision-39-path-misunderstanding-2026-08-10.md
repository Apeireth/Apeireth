# Decision #39 — 路径误解根因 verify + 4 补充建议 (18:15-18:18)

**Date**: 2026-08-10 18:18
**Author**: Mavis (新 root session, mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**触发**: 主人 18:15 "我在团队成员的消息中发现他们对: '发现严重路径问题! 我之前写到 Apeireth\xa1\xaaRust-0.9 (错的目录), 真项目是 Apeireth─Rust-0.9 (em-dash)'"
**关联**: decision-35 (16 sub-agent 真派) + decision-36 (P2 现状) + decision-37 (R125-8 done) + decision-38 (主人 17:56 "0 新派成员" 严守)

---

## 0. 一句话

**桌面错目录 `ApeirethＲust-0.9` (全角 R U+FF32, UTF-8 bytes `EF BC B2`, 主人以为 em-dash 实际是全角 R) 是废弃目录, 主人可删; 真项目 = `Apeireth-rust` (hyphen `-`) 在 `.openclaw/workspace/promethean/` 父目录, git HEAD = 43b6dd57 (17:43 V1469 ASI round 131, 上层 cron 自动派, 跟 R125 升级路线独立); R125-12 sub-agent 仍 running 38 min 0 output yet, 0 装 PASS 严守, 让它自己纠正路径 0 重派 (per 17:56 "0 新派成员" 严守)**.

---

## 1. 路径误解根因 verify (18:15-18:18)

### 1.1 桌面错目录 raw bytes verify

| 目录 | raw bytes | 实际字符 | 类型 |
|------|----------|---------|------|
| `Desktop\Apeireth\uffffust-0.9` | `41 70 65 69 72 65 74 68 EF BC B2 75 73 74 2D 30 2E 39` | **`ApeirethＲust-0.9`** (全角 R U+FF32) | 错的废弃目录, 主人准备删 |
| `Desktop\Apeireth` | `41 70 65 69 72 65 74 68` | `Apeireth` (无后缀) | 桌面另一个目录 |
| `Desktop\apeireth-desktop-preview` | `61 70 65 69 72 65 74 68 2D 64 65 73 6B 74 6F 70 2D 70 72 65 76 69 65 77` | `apeireth-desktop-preview` | 桌面 preview 目录 |
| `Desktop\Apeireth\uffff\uffff\uffff\uffff\uffff.txt` | (乱码) | (乱码) | 旧主目录文件 |
| `Desktop\Apeireth\uffff\uffff\uffff\uffff\uffff\uffff\uffff\uffff\uffff\uffff.txt` | (乱码) | (乱码) | 旧主目录文件 |

**`Apeireth\xa1\xaaRust-0.9` (NBSP+ordinal, 主人以为 sub-agent 写到的错目录) 0 在磁盘上** — sub-agent 误解, 实际它没写到任何目录 (0 output yet)

### 1.2 真项目 = `Apeireth-rust` (hyphen `-`)

| 路径 | 状态 |
|------|------|
| **我主仓 = `.openclaw\workspace\promethean\Apeireth-rust\`** (hyphen `-`) | ✅ **真项目** (17:30:34 commit 21aa85f3 + 17:43 V1469 commit 43b6dd57) |
| **git 主仓根 = `.openclaw/workspace/promethean/`** (父目录) | ✅ |
| **git HEAD = `43b6dd57` V1469 ASI round 131** | ✅ |
| **promethean/ 下 6 Apeireth 目录** (apeireth/apeireth-legacy/Apeireth-protocol/Apeireth-rust/Apeireth-tui/rust-substrate) | ✅ `Apeireth-rust` (hyphen) 是真项目 |
| **git worktree 列表** | 主仓 master + 8 worktree (含 .spectrai-worktrees) |

### 1.3 R125-12 sub-agent 误解路径

| 状态 | 值 |
|------|---|
| task_id | `bg_1b294685-358a-43a5-9c7c-620b90e1e9f2` |
| 派活时间 | 17:32 (per decision-35) |
| 当前时间 | 18:18 (running 46 min) |
| task status | **running** |
| final 报告 | 0 写 |
| borrow-id-index | 0 写 |
| host-call-replay-spec | 0 写 |
| 写到错目录文件 | 0 写 (0 output yet) |
| 误解路径 | `Apeireth\xa1\xaaRust-0.9` (NBSP, 0 在磁盘) + `Apeireth─Rust-0.9` (em-dash, 0 在磁盘) → 实际桌面 `ApeirethＲust-0.9` (全角 R) 是错的废弃目录 |

**R125-12 误解 + 0 output yet 状态**: sub-agent 收到 "真项目是 em-dash" 的信号, 实际上 em-dash 0 在磁盘, 全角 R 是错的废弃目录. R125-12 也许在 thinking 阶段自己纠正, 0 装 PASS 严守 (0 写文件, 0 假装"已写到错目录").

### 1.4 V1469 commit 43b6dd57 (17:43) — 上层 cron 自动派 ASI 路线

| 指标 | 值 |
|------|---|
| commit hash | `43b6dd5793c3f5c2f32f83b5f2b759ec6d06e329` |
| commit 时间 | 2026-08-10 17:43:43 +0800 (我 17:30:34 commit 21aa85f3 后 13 min) |
| author | chuling <chuling@apeireth.local> |
| Co-Authored-By | Mavis (cron tick 17:33 round-131) |
| 路线 | **ASI round 131** (跟 R125 升级路线独立, 上层 cron 1 min tick `dispatch-r125-now-min-tick` 自动派) |
| 文件 | 5 files +1992 lines |
| tests | 60 pass + popper 6/6 PASS + 6/6 endpoints hit via V1468-generated client + real subprocess V1467 server in process A + real subprocess V1469 client driver in process B |
| guards | 14 V1469 guards (V1468_REUSED/V1467_REUSED/TWO_PROCESSES/PORT_REUSED/HTTP_LIVE/CLIENT_GENERATED/CLIENT_DRIVER_RUNS/RESULT_FILE_PARSED/ALL_ENDPOINTS_HIT/BOUNDED_WALLCLOCK/SUBPROCESS_CLEANED/LINEAGE_CITED/RUNS_ON_WINDOWS/DETERMINISTIC) |
| 7 守门 | DRIVER_NOT_CI/LOAD_TEST/FUZZER/ASI/PHENOMENAL/HUMAN_LEVEL/ORCHESTRATOR |
| 0 越界 8 硬墙 | ✅ 0 改 24 LOCKED + 0 改 workspace.version 1.2.0 + 0 改 R11 baseline 0.8682/0.8532/0.9063 |
| 0 主动 push | ✅ 严守 |
| 借鉴 ID | 5 borrowed (V1468+V1467+V1466+V1437+stdlib subprocess/tempfile/socket/json/urllib/time) |

**V1469 0 越界 verify**: V1469 commit 在我 17:30 拍 21aa85f3 后 13 min 自动派, 5 files 全 NEW (没改我拍板的 257 files), 60 tests pass, 跟 R125 升级路线独立 (ASI 路线 round 131, 不是 R125 P0/P1/P2/P3 路线).

---

## 2. 4 补充建议 (主人可拍板)

### 2.1 建议 1: 桌面 `ApeirethＲust-0.9` (全角 R) 删

- **现状**: `Desktop\Apeireth\uffffust-0.9` (实际 `ApeirethＲust-0.9` 全角 R) 是错的废弃目录, 8/10 17:52:01 修改, 内部有 `crates/apeireth-http-client` (R125-3 sub-agent bg_26a6b507 17:47 写到这, 误解路径)
- **建议**: ✅ 删, 主人拍板
- **不删风险**: R125-3 sub-agent (hyper 池) 写到这目录, 删了丢失 (R125-3 0 output yet, 也许 0 写实际 src 实施, 但可能有 spec/index 文件)
- **建议执行**: 删前先 verify R125-3 写了什么 (`Test-Path Desktop\Apeireth\uffffust-0.9\crates\apeireth-http-client\`), 0 写实际 src 实施 → 直接删; 有 src 实施 → 复制到主仓 `Apeireth-rust/crates/apeireth-api/src/http_client.rs` 然后删

### 2.2 建议 2: R125-12 sub-agent 0 重派 (per 17:56 严守)

- **现状**: R125-12 bg_1b294685 仍 running 46 min 0 output yet, 误解路径
- **建议**: ❌ 0 重派 (per 主人 17:56 "0 新派成员" 严守)
- **理由**: 
  1. 主人 17:56 拍板"0 新派成员, 等这些干完"
  2. R125-12 0 output yet, 也许正在 thinking 阶段自己纠正路径
  3. 0 装 PASS 严守, 即使纠正后跑完, 0 写实际 src 实施 (跑过夜明早 8/20 截止, 0 必 R125 续 mavis 整合 daemon 处理)
  4. 重派浪费 17:32-18:18 已跑的 46 min token
- **监控**: 5 min tick cron self `watch-r125-supervisor-17-22` 持续监督, R125-12 跑过夜明早 8/20 截止

### 2.3 建议 3: V1469 commit 43b6dd57 (17:43) 0 越界 verify

- **现状**: V1469 ASI round 131, 5 files +1992 lines, 上层 cron 1 min tick 自动派
- **建议**: ✅ 0 撤回 (0 越界 8 硬墙 + 0 主动 push 严守)
- **理由**:
  1. V1469 跟 R125 升级路线独立 (ASI 路线 round 131)
  2. 上层派活 daemon 复活后 (per decision-30) 继续派 ASI 路线 (V1467→V1468→V1469 链)
  3. 60 tests pass + 14 guards + 7 守门 verify 0 越界
  4. 0 改 24 LOCKED + 0 改 workspace.version 1.2.0 + 0 改 R11 baseline 数字
  5. 5 files 全 NEW, 跟我 17:30 拍 21aa85f3 的 257 files 无 overlap
- **新发现**: 顶层 cron 仍在跑 ASI 路线, 0 必管, 但 5 min tick 监督 + 0 越界 verify

### 2.4 建议 4: 0 主动讨论后续 (per 17:56 严守)

- **现状**: 16 sub-agent 跑中 (3 done R125-8/10/15c + 4 done R125-15b/1/9/5 实际 7 done + 9 running), 跑过夜明早 8/11-8/22 陆续 done
- **建议**: ❌ 0 主动提议 R125-15e/f + R125-16~21 Library 6 阶段 + R126 续 + R127 1.0 release
- **理由**:
  1. 主人 17:56 拍板"0 新派成员, 等这些干完, 0 自主讨论后续"
  2. 16 sub-agent 跑过夜明早 8/11-8/22 陆续 done, 主人主动回来讨论后续
  3. 0 主动提议 = 0 装"我已经知道后面该干什么" (per 主人 17:56 "等这些干完我们讨论下后续干什么")

---

## 3. 决策链 (接 #38)

- **#22 (16:35)**: 主人 16:31 最高权限 + 24 LOCKED 自主确认 + 9 项实质 locked 升级
- **#30 (17:15)**: 新 Mavis 接入 + 派活 daemon 复活
- **#31 (17:17)**: 17:30 拍板 dry-run + 138 src 改动诚实标
- **#32 (17:18)**: R125 派活大主管启动 (17:23 task_stop, 0 实施 错)
- **#33 (17:23)**: 主人 17:22 升级授权 + 8 硬墙重置 + 0 装解除
- **#34 (17:30)**: 17:30 整合 #3 commit 21aa85f3 拍板 done (257 files +61969/-520)
- **#35 (17:32)**: 主人 17:31 "16 成员人数要多" + supervisor 模式废弃 + Mavis 真派 16 sub-agent
- **#36 (17:44)**: 主人 17:44 提醒 P2 + 借鉴源码 7/11 ✅ cloned
- **#37 (17:49)**: R125-8 P1 头一个完成 (5 阶段 78.3KB, 0 装 PASS 严守)
- **#38 (17:56)**: 主人 17:56 "0 新派成员, 等这些干完, 0 自主讨论后续"
- **#39 (18:18, 本决策)**: **路径误解根因 verify (桌面 `ApeirethＲust-0.9` 全角 R 是错的废弃目录, 真项目 = `Apeireth-rust` hyphen) + R125-12 仍 running 46 min 0 output yet 0 装 PASS 严守 + V1469 commit 43b6dd57 (17:43) ASI round 131 0 越界 8 硬墙 + 4 补充建议 (桌面删 + R125-12 0 重派 + V1469 0 撤回 + 0 主动讨论后续)**

---

## 4. 一句话 (TL;DR)

**路径误解根因 verify 完毕: 桌面 `ApeirethＲust-0.9` (全角 R U+FF32, 主人以为 em-dash 实际是全角 R) 是错的废弃目录, 主人可删 (删前 verify R125-3 bg_26a6b507 17:47 写的文件); 真项目 = `Apeireth-rust` (hyphen `-`) 在 `.openclaw/workspace/promethean/`, git HEAD = 43b6dd57 (17:43 V1469 ASI round 131, 5 files +1992 lines, 0 越界 8 硬墙); R125-12 仍 running 46 min 0 output yet 0 装 PASS 严守 0 重派 (per 17:56 "0 新派成员"); 4 补充建议 (桌面删 + R125-12 0 重派 + V1469 0 撤回 + 0 主动讨论后续)**.
