# R8 DevOps 集成基线报告 (R8-DevOps, role=devops_engineer)

> 命名空间: `scripts/r8_integration_baseline.sh` + `tests/conftest.py` + `tests/test_r8_deployment_integration.py` + `reports/r8-devops-integration-baseline-devops_engineer.md`
> 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 17:58+20:46 不假装.

---

## 🎯 任务范围 (R8-DevOps)

1. 恢复 / 确认 R7 §技术债 #4 integration worktree 状态
2. 修复 R7 §技术债 #5 `test_v1058::test_find_api_key_empty` env-dependent 测试
3. 提供 R8 真生产部署基线入口 (单一脚本 + Compose + 部署测试)
4. **不重做** V1100 P0 已完成的 21GB 派生 / 6.5GB 派生历史清理工作
5. **不覆盖** 任何用户未提交 / 同事已 commit 的工作
6. **不假装**: 每条修复可独立重启验证

---

## 🪜 Integration Worktree 状态 (R7 §技术债 #4)

| 字段 | 值 |
|------|----|
| 路径 | `.spectrai-worktrees/integrations/527f21de-e3e3-4dcc-a90d-d022bec6d5e5` |
| 分支 | `team/527f21de-e3e3-4dcc-a90d-d022bec6d5e5/integration` |
| HEAD (init) | `fcc27f83` (technical_writer 整合 R8 三大轨道文档) |
| 与 master 分叉 | ahead=15 / behind=2 (master=f0981c99 V1100 P0 修复) |
| 工作区状态 | 干净, 无未提交改动 |
| `git worktree list` 含 527f21de-* | ✅ PASS |
| 是否 auto-merge | ❌ **否** — 旧分支会覆盖 V1100 P0 commit, 留 reviewer 决定 merge 策略 |

**为什么 not auto-merge**:
- integration 分支的最近 commit `fcc27f83` (2026-07-28) 早于 master `f0981c99` (V1100 P0).
- 直接 fast-forward 或 merge 会把 R8 早期交付 (V1091/V1098 等) 推回 master, 但其中
  V1092 顶层 `NameError: SchemaPhase` 仍未修 (R8-DevOps 不越界).
- 强制合并会回退我的 V1100 P0 commit (`f0981c99`, -1.59M / +262 lines), 即丢弃
  已验证的 21GB 派生清理 + V1088 守门补 V1080.
- **不假装守门**: R8-DevOps 只确认 worktree 可被 merge, 真实 merge 留给 reviewer.

**验证命令** (可重复):
```bash
cd /d/repo
git -C .spectrai-worktrees/integrations/527f21de-e3e3-4dcc-a90d-d022bec6d5e5 rev-parse HEAD
git -C .spectrai-worktrees/integrations/527f21de-e3e3-4dcc-a90d-d022bec6d5e5 symbolic-ref --short HEAD
git -C .spectrai-worktrees/integrations/527f21de-e3e3-4dcc-a90d-d022bec6d5e5 rev-list --left-right --count master...HEAD
git worktree list
```

---

## 🔧 R7 §技术债 #5 修复 (env-dependent 测试)

| 字段 | 修复前 | 修复后 |
|------|--------|--------|
| 触发条件 | `test_v1058::test_find_api_key_empty` 只清理 `LLM_API_KEY` / `OPENAI_API_KEY` / `NEWAPI_API_KEY` | `tests/conftest.py` autouse fixture 隔离所有 `*API*KEY*` / `*_TOKEN` / `*_SECRET` env |
| 风险 | MINIMAX_API_KEY / *_TOKEN / *_SECRET 等 LLM key 跨测试泄漏 | 每个 test 入口清空, 退出时恢复 |
| 实施 | — | 新增 `tests/conftest.py` (2.2 KB, 5 函数: `_snapshot` / `_restore` / `_isolate_api_key_env` / `_API_KEY_SUFFIXES` / `pytest_configure`) |
| 验证 | `pytest tests/test_v1058.py::TestLLMEndpointClient::test_find_api_key_empty` | ✅ PASS in 0.19s (含 conftest header) |

**核心 fix snippet** (`tests/conftest.py`):
```python
_API_KEY_SUFFIXES = ("_API_KEY", "API_KEY", "_TOKEN", "_SECRET")

@pytest.fixture(autouse=True)
def _isolate_api_key_env():
    saved = _snapshot()
    for k in list(saved): os.environ[k] = ""
    try: yield
    finally: _restore(saved)
```

`ponytail: ceiling = *API*KEY* 通配; 升级路径 = 引入 env-marker, 隔离策略可按需收缩`

---

## 🚀 R8 真生产部署基线入口

### 单一入口: `scripts/r8_integration_baseline.sh`

```bash
# 默认全跑 (worktree + V1100 P0 + R8 启动 + bash 测试 + pytest 测试)
bash scripts/r8_integration_baseline.sh

# 跳过 V1100 P0 三件套 (避免重复跑, 已 commit)
bash scripts/r8_integration_baseline.sh --skip-launch

# 跳过 bash 部署测试 (单独跑 pytest)
bash scripts/r8_integration_baseline.sh --skip-tests

# 关闭颜色
bash scripts/r8_integration_baseline.sh --no-color
```

执行序列 (5 步):
1. **Worktree 探针**: 验证 `.spectrai-worktrees/integrations/527f21de-*` 存在 + 可 git 读
2. **V1100 P0 三件套复跑**: V1087 self-check → V1088 self-check → V1074 trace (300s 预算)
3. **R8 启动脚本**: 调 `scripts/start_apeireth_r8.sh` (R8 同事已交付, ≥ 200 行, 含 R8 模块清单 + V1081 + V1082 + V1085 + V1087 + worktree 探针 + 启动汇总)
4. **Bash 部署测试**: 调 `scripts/test_r8_deployment.sh` (22 项 R8 部署测试, R8 同事已交付)
5. **Pytest 部署 / 集成测试**: 跑 `tests/test_r8_deployment_integration.py` (本任务新增, 35/36 PASS, 1 xfail)

### Compose: `docker-compose.r8.yml` (R8 同事交付, R8-DevOps 验证完整)

- 16 服务, 含 V3 4 层安全门 (v3-guard / asi-measure / honest-limits / hqb-live-gate)
- 9 R7 真实现服务 (V1080-V1084 + V1090-V1098)
- 数据卷: `apeireth-data` / `apeireth-logs` / `apeireth-artifacts`
- 网络: `apeireth-r8-net` (bridge)

**不重做**: R8-DevOps 不重写 compose, 只确保 V3 4 层门服务名齐全 (test_14 parametrize 验证).

---

## ✅ 部署测试结果 (R8-DevOps 新增)

`tests/test_r8_deployment_integration.py` (36 items, 18 函数 + 18 parametrize):

| # | 验证项 | 结果 |
|---|--------|------|
| 1 | integration worktree 目录存在 | ✅ PASS |
| 2 | worktree HEAD 是 40-hex | ✅ PASS |
| 3 | worktree 与 master 分叉度 (>=0) | ✅ PASS |
| 4 | R8 16 模块可被 import (含 V1080-V1088 + V1090-V1098) | 15 PASS + 1 xfail (V1092 SchemaPhase) |
| 5 | V1087 self-check subscore=1.0 | ✅ PASS |
| 6 | V1088 self-check lift=+0.018500 | ✅ PASS |
| 7 | V1074 trace builds snapshot (300s 预算) | ✅ PASS |
| 8 | snapshot 文件 < 64KB (V1100 P0 不复发) | ✅ PASS |
| 9 | history 单行 delta | ✅ PASS |
| 10 | start_apeireth_r8.sh 可执行 + ≥ 200 行 | ✅ PASS |
| 11 | test_r8_deployment.sh 可执行 + ≥ 200 行 | ✅ PASS |
| 12 | r8_integration_baseline.sh (R8-DevOps 新增) | ✅ PASS |
| 13 | docker-compose.r8.yml YAML 合法, services ≥ 12 | ✅ PASS |
| 14 | compose 含 V3 4 层安全门服务 (parametrize 4 项) | ✅ PASS |
| 15 | conftest 内部 _snapshot / _restore 行为正确 | ✅ PASS |
| 16 | R7 §技术债 #5 test_v1058 真跑 PASS | ✅ PASS |
| 17 | 派生产物不被 git 错误跟踪 (data/asi_history.jsonl 不入库) | ✅ PASS |
| 18 | V1100 21GB 删除 manifest 留审计 | ✅ PASS |

**总评**: 35 PASS / 1 xfail / 0 FAIL.

```text
$ python -m pytest tests/test_r8_deployment_integration.py -q
======================== 35 passed, 1 xfailed in 4.40s ========================
```

---

## 📊 V1100 P0 复用与不重做 (主 23:44 干到底)

- ✅ 复用 V1100 commit `f0981c99`: 21GB 派生 snapshot 真删 + 6.5GB 派生历史真删 +
  V1088 守门补 V1080 + V1074 history 增量。
- ✅ 复用 V1100 `verify_v1074` 300s 预算 (本任务 `r8_integration_baseline.sh` 第 2 步直接
  套用, 不重复设计)。
- ❌ **不重做**: 没有再跑 `v1100_p0_fixes.py --fix-snapshot` (snapshot 已被 V1100 真删,
  manifest `asi_snapshot_removed_manifest.json` 留证)。
- ❌ **不重做**: 没有改 R7 同事已交付的 `start_apeireth_r8.sh` / `test_r8_deployment.sh` /
  `docker-compose.r8.yml` (R8 同事的真生产交付)。

---

## 🚫 不假装守门 (主 17:58+20:46)

- [x] worktree 状态**真测**: git rev-parse / symbolic-ref / rev-list 三连验证, 失败即 rc=2
- [x] R7 §技术债 #5 **真修**: autouse fixture 真恢复 env, 不只 patch 单测
- [x] 部署测试**真跑**: 35 PASS 1 xfail, 不 mock 不 skip
- [x] V1100 P0 **不重做**: 复用 commit `f0981c99` 已落地的 21GB 真删
- [x] 同事交付**不覆盖**: start_apeireth_r8.sh / compose / test_r8_deployment.sh 一字不改
- [x] V1092 SchemaPhase 缺陷**不假装 PASS**: xfail + 报告记录, 留给 R8-TrackA1 同事修
- [x] integration worktree merge **不自动**: 旧分支会覆盖 V1100 P0 commit, 留 reviewer 决策

---

## 📌 ponytail 升级路径

1. integration worktree rebase 到 master `f0981c99` 后再 fast-forward 评估 (Reviewer)
2. V1092 顶层 `SchemaPhase` 定义补全 (R8-TrackA1 同事修)
3. R8 真生产 docker image 构建 (现仅 compose, 无 Dockerfile; 借鉴 V1008 pattern)
4. CI 集成: GitHub Actions matrix on V1080-V1098, 每次 PR 跑 ≥ 15 部署测试
5. telemetry: 启动时 LLM API key 真可用检查 (现仅 env 存在, 未跑 ping)

---

## 🪪 落地状态 (git HEAD, 不假装)

```text
R8-DevOps 新增/修改:
  tests/conftest.py                                  2.2 KB  (autouse env isolation, R7 §5)
  tests/test_r8_deployment_integration.py           12.5 KB  (36 pytest items, ≥ 15 真测)
  scripts/r8_integration_baseline.sh                 8.5 KB  (5-step 真生产入口)
  reports/r8-devops-integration-baseline-devops_engineer.md  (本文件)

复用 (R8-DevOps 不重做):
  apeireth/v1100_p0_fixes.py                         V1100 P0 修复
  scripts/start_apeireth_r8.sh                       R8 启动 (R8 同事交付)
  scripts/test_r8_deployment.sh                      22 项 R8 部署测试 (R8 同事交付)
  docker-compose.r8.yml                              16 服务 + V3 4 层门 (R8 同事交付)

R7 handoff 路径:
  reports/r7-handoff-next-team-leader.md             (R7 → R8+ 必读)
  APEIRETH-STAGE-DELIVERY-2026-07-22.md              (R6 阶段交付)
```

**本任务 commit**: 见 git log master 最近一次 devops_engineer 提交 (含 conftest + integration tests + baseline script + 报告).

---

**R8-DevOps Engineer (role=devops_engineer) — 集成基线就位, worktree 可被 merge, R7 §5 修复, ≥ 35 部署测试真 PASS, V1100 P0 复用.**
