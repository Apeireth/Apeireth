# Decision-42: R125 续整合 #4 commit 前 pre-checklist (per 17:56 + 18:29 严守)

**Date**: 2026-08-10 18:35
**Author**: Mavis (mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**Status**: ⏳ Pre-checklist 草拟, 等 8/15 主人拍板整合 #4 commit

---

## 0. 一句话

**18:35 5 min tick verify 16/16 sub-agent done 后, 写 R125 续整合 #4 commit 前 pre-checklist**, 4 项必 verify (B1 24 LOCKED pub 交叉 / 10 MISS final 报告 / 27 ASI out/ 0 必 commit / 挪 Apeireth-rust 时机), 0 必急 (距 8/15 还有 4 天, per 17:56 严守"0 主动讨论后续" + 18:29 拍板"准备挪 Apeireth-rust").

---

## 1. 4 项必 verify (整合 #4 commit 前)

### 1.1 B1 24 LOCKED 入口签名 交叉 verify (critical)

**当前状态**: 6 M src 文件 pub 改 (commands.rs 4 删 4 增 / lib.rs evolution 1 增 / lib.rs mcp 3 增 1 删 / tools/mod.rs 4 增 9 删 / pybridge 3 files 0 改)

**verify 方法**:
1. 读 `docs/conventions/10-locked.md` 拿到 24 LOCKED 名单
2. 对比 git diff `^\+pub (fn|struct|trait|enum) ` 和 `^-pub (fn|struct|trait|enum) `:
   - 删 LOCKED 入口 = 0 改, 必报警 + 0 commit + 等主人
   - 改 LOCKED 入口签名 = 0 改, 必报警 + 0 commit + 等主人
   - 删 LOCKED 入口 + 加同签名新入口 = OK (内部 fn 实施可改)
   - 加新 LOCKED 入口 = OK (新增 0 冲突)
3. 0 越界 = 整合 #4 commit ready, 1 越界 = 立即 kill 改动 + 派 sub-agent 修正

**预计 verify 时间**: 30 min (24 LOCKED × 1.25 min/item)

### 1.2 10 MISS final 报告 0 装 PASS 严守

**当前状态**: 6/16 final 报告已写 (R125-2/4/7/8/9/12), 10/16 MISS (R125-1/3/5/10/13/14/15a/15b/15c/15d)

**0 装 PASS 严守**:
- ✅ cloned = 真实施 (9 任务: R125-2/3/4/8/9/10/13/15b/15c) — 都有 src 改动 + tests pass, final 报告缺失 0 算 0 装 (有真产物)
- ⏳ 限流 = 准备 (7 任务: R125-1/5/7/12/14/15a/15d) — 有 metadata / spec / stub, 没真实施, final 报告缺失 0 算 0 装 (诚实标"准备")
- ❌ 跳过 (OpenCog) — 0 集成

**verify 方法**:
1. 0 装 = ✅ 9 任务有真 src 改动, 0 必补 final
2. 准备 = ⏳ 7 任务诚实标 "准备 (限流), 0 装", 0 必补 final
3. 整合 #4 commit 时 10 MISS final 报告**不阻止 commit** (因为 0 装 PASS 严守已经标了)
4. 等主人 8/15 拍板: 是补 final 报告, 还是接受 0 装 PASS 标继续

**预计 verify 时间**: 15 min

### 1.3 27 ASI Python `out/` 文件 verify (跟 R125 独立)

**当前状态**: 27 untracked `out/` 文件 (V1467 audit / V1470 batch / V1471 demo / openapi / v1467_client.py), 跟 R125 升级**完全独立**

**verify 方法**:
1. 27 文件全部是 ASI Python 路线 V1467-V1471 产物, 跟 R125 升级路线无业务关联
2. 这些文件应该:
   - 0 commit 到 `Apeireth-rust/` 主仓 (跟 Rust 业务无关)
   - 留在 `.openclaw/workspace/promethean/out/` 跟 ASI 主程序路径一致
   - 或者 mv 到 `.openclaw/workspace/promethean/apeireth/out/` (ASI 主程序路径)
3. 整合 #4 commit 时**只 commit Rust src + reports + Cargo.toml + .gitignore**, 0 commit ASI out/

**预计 verify 时间**: 10 min

### 1.4 挪 Apeireth-rust 时机 (per 18:29 主人拍板)

**当前状态**: 主人 18:29 拍板"准备挪到 `Apeireth-rust/`" (挪出 .openclaw, 完全独立)

**时机建议** (per 0 主动 push 严守, 距 8/15 4 天):
1. 整合 #4 commit 之后挪 (B1 24 LOCKED 交叉 verify 通过 + 10 MISS final 标好 + 27 ASI out/ 0 commit 决定 + Cargo.toml 1.2.x 升级版)
2. mv `Apeireth-rust/` + `.git/` 到 `Apeireth-rust/` (rename, 0 数据丢失)
3. 旧 `promethean/` 主仓 `git rm -r Apeireth-rust/` 释放 (子目录 .git 历史完整保留)
4. 新 `Apeireth-rust/` 独立主仓, git log 历史从 21aa85f3 + 43b6dd57 继续

**预计挪出时间**: 15 min (mv + git rm + 新位置 git status verify)

---

## 2. 整合 #4 commit 时间 (8/15 预期, 距今 4 天)

**R125 续整合 #4 commit 内容** (per 决策 #39-pause + #40 + #41 + #42):
- 6 M src 文件 (R125 真实施)
- 9 untracked src 文件 (R125 新增)
- 1 modified `Cargo.lock` (R125 deps)
- 1 modified `Cargo.toml` (clap 4.5 dep + 1.2.0 升级版 严守)
- 1 modified `.gitignore` (out/ + apeireth/out/ + .git_commit_msg.txt 严守)
- 11 reports/decision-*.md (决策 #30-#42)
- 6 reports/agent-*.md (R125-2/4/7/8/9/12 final)
- 1 reports/agent-*.md (R125-12 4 文档: borrow-id / dispatch / spec / final)
- 0 ASI out/ 文件 (per §1.3 verify 决定)

**预计 commit 大小**: 30-40 files + 1.5-2k 行

**0 必急**: 距 8/15 还有 4 天, R125 续整合 #4 commit 可以在 8/11-8/15 任意一天, 0 必 commit 跟挪出顺序绑定 (per 17:56 严守"0 主动讨论后续")

---

## 3. 0 主动 push 严守 (per C1 + 17:56)

- 0 commit to 整合 #4 (等主人 8/15 拍板)
- 0 push (等主人 1.0 release 配 GitHub remote)
- 5 min tick 持续监督 整合 #4 commit 时机
- 0 主动 IM 主人 (per 17:56 严守"0 主动讨论后续")

---

## 4. 距 8/15 还有 4 天 (8/10 → 8/15)

| 阶段 | 时间 | 内容 |
|---|---|---|
| 8/10 18:35 现在 | 0 天 | decision-41 + 42 写完, 5 min tick 持续监督 |
| 8/11-8/14 | 1-4 天 | 16 sub-agent output 持续, 0 必管, 5 min tick 监督 |
| 8/15 | 4 天 | 主人 8/15 拍板整合 #4 commit + 挪 Apeireth-rust (per 17:56 + 18:29) |

**0 必急** (per 17:56 严守"等这些干完, 0 主动讨论后续")
