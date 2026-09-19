# 工程日志 2026-10-06 (Engineering Log)

> **给谁看**: 协作者 / 接手人。**口径**: 这一天发生在 `main` 上的全部工程动作,
> 以 commit 为索引。每项修复都来自**真机实测** (主人逐条试出来的 bug), 不是
> 推测。测试口径以 `live-verification-ledger.md` 为准。
> **基线**: 起点 `b86a89d5` (协作者 56 提交合并后), 终点见文末。

## 0. 协作者批整合 (早盘)

- `origin/main` 领先本地 56 提交 (记忆 v2.2 生产收口 + 行为链安全 Guard 新 crate
  `apeireth-guard` 接生产治理管线 + 认知 vnext 生产接线, PR #15)。
- 快进整合后全量验证: `cargo check --workspace --all-targets` + `cargo test
  --workspace` + 桌面 src-tauri + svelte-check + 7/7 全绿; 新包装机 e2e 17/17。
- 合并点: 无冲突 (协作者基于我方推送续作)。

## 1. 真机修复批次 (主人在桌面应用上逐个试出的)

| # | 现象 (真机) | 根因 | 修复 | commit |
|---|---|---|---|---|
| 1 | 认知深度"深度"档: judge 一要求重试, 第 9 次模块调用 `BudgetExceeded` 炸毁整个回合, 归码 internal | judge(1)+council(7)=8 恰好顶满旧预算 8 | 预算 8→24 + **BudgetExceeded 优雅降级** (该模块本 pass 跳过, 候选回复照常提交, 会话事件留痕 `module_budget_degraded`) | `c9174eba` |
| 2 | 审批解析卡死: 通过后"一直没有后续" | resolve fetch 无超时 + 晚到失败 `.catch(()=>{})` 静默吞 + 无二次拉取 | resolve 5min 超时 + 晚到失败留提示 + 10s/30s 定时重拉 inbox 自救 + pending 文案同步 | `20a52098` |
| 3 | judge retry 预算耗尽 → Stop 枪毙审批续跑回合 (HTTP 500) | 预算耗尽语义错误 (应接受候选) | 改为 best-effort `continue_()` (Retry 裁决仍如实记录) | `58a9ef2d` |
| 4 | `judge returned invalid JSON: EOF at line 1 column 0` 杀回合 | DeepSeek 对纯 JSON 指令偶发空回复 | 空响应/坏 JSON → 降级放行候选 + stderr 留痕 (0 装不伪造评分) | `6cbb0759` |
| 5 | "已提交, 工具正在执行" 挂"建议 请重试" | 状态提示走错误横幅 | **notice 与 error 分离**: 状态提示走独立蓝色横幅 (等待审批/执行中), 错误横幅只挂真错误 | `ee32ddb0` |
| 6 | 审批通过后弹的其实是切换预设前的旧审批, 用户误以为"完全放行无效" | 预设变更不追溯 + inbox 自愈反复弹出 | 切策略后主动检查并提示"有 N 个未决审批不追溯" | `683a9d9d` |
| 7 | `date` 输出乱码 | 中文 Windows cmd 输出 GBK, 硬解 UTF-8 | UTF-8 失败回退 GBK 解码 (encoding_rs) | `d20df3bb` |
| 8 | judge Stop ("AI Judge rejected the candidate") 挂"重启 Companion" | 模块停靠归码 internal | 新错误码 **`review_rejected`** (方案: 重新生成/调低认知深度) | `09ceb8d2` |
| 9 | "turn did not converge within 8 rounds" 挂"重启 Companion" | 轮数上限归码 internal | 新错误码 **`turn_not_converged`** (方案: 拆小问题/重新生成) | `05a5d118` |
| 10 | 全套边界测试发现敏感名单漏网: `GeminiApiKey.txt`/`Users31683.git-credentials`/`apeireth_gateway.db` 可被只读工具看见 | 屏蔽名单只认精确名/前缀 | `contains("apikey")` + `.git-credentials` 后缀 + 产品 DB 精确名 | `b41be6c1` |
| 11 | 引号被吃: `powershell -Command "..."` 只回显不执行 | cmd `/S` 引号规则 × Rust argv 转义叠加 | 命令改走 **`raw_arg` 直传** (`/D /S /C "<cmd>"` 原样进命令行), 双引号全保真 + 真实执行回归测试 | `b41be6c1` |

**元层原则 (本日确立, 写进代码注释)**: judge/council 等评审机制只能降级
(预算耗尽/空响应/坏 JSON → best-effort 放行候选), 绝不枪毙主任务; 枪毙仅保留给
评审的**显式安全裁决** (Stop), 且该裁决必须用诚实错误码呈现, 不伪装成系统错误。

## 2. 新功能

| 功能 | 内容 | commit |
|---|---|---|
| 会话级工作区 | 新会话自动弹工作区选择器; 切换会话自动重根侧车 (~1s); 分支继承父会话工作区; 工作区目录同时锚定侧车 CWD (工具根) 与 store 路径 | `d4944286` / `284441c8` |
| 错误码目录扩展 | `review_rejected` + `turn_not_converged` (原 8 码 → 10 码, 前端 ERROR_SOLUTIONS 同步) | `09ceb8d2` / `05a5d118` |
| 认知深度档位实测标注 | 设置页标注实测延迟 (轻量 ≈1.1s / 平衡 ≈2.6s / 深度 ≈13s 每轮) | `136dcc01` |
| 杂项清理 | app-data 中 9/11 调试残留探针文件 (payload.json/p.py/probe.py/q.py) 删除 | — |

## 3. 安全发现与文档

- **shell 非沙箱** (真机边界测试实证): 以主人本机账号全权运行, 可读全盘/写任意/
  对外联网。filesystem/search/fetch 各有策略墙, shell 是唯一无遮一环。
  → 设计文档: `docs/01-architecture/shell-sandbox-lite-design-2026-10-06.md`
  (轻量档 = AppContainer 工作区限定 + 断网 + 输出凭据触发器 + 守门扩展; **待主人拍板实施**)。
- 主人工作区曾=用户主目录, 凭据文件暴露给只读工具视野 → 名单补漏 (见 §1#10) +
  建议主人把工作区改为具体项目目录并轮换相关 key。

## 4. 遗留 / 待办 (诚实口径)

1. **Shell 沙箱轻量档**: 设计已出, 待拍板实施 (§3)。
2. **会话转录重载 API**: resolve 请求中途断连时, 已完成的答案 UI 无法从后端取回
   (无 transcript 端点)。列入后续波次。
3. **信任分级 (trust tier) / 审批频率 / 免审批黑名单**: v1 有、v2 缺, RC-13 排期
   (见 `capability-knob-audit-2026-09-12.md` §4)。
4. **认知深度"深度"档提速**: 议会 7 顾问可降 3 (可选, 需拍板)。

## 5. 全链路验证

- 装机 e2e **17/17** (含真聊天, 多次重打包重装验证)。
- 全套边界测试 (主人在应用内让模型实测): filesystem/search/fetch 的墙全部实锤,
  shell 无沙箱实锤, 敏感名单补漏后复测通过。
- 每项修复的自动化测试见各 commit; 台账新增条目 #26-#28。
