# R23 续补计划 (2026-08-06, 主人 8/6 12:50 拍板)

> **触发**: 主人 8/6 12:50 拍 "没做的加到后续计划里面去, codex 干完了, 你看看".
> **依据**: 本座 8/6 12:30-12:50 跑 32 项审计 + 主人 8/6 12:25 "三项接受, 按你建议来" 拍板.
> **3 件偏差** (per 整合 #3+#4+#5 报告 vs 实测):
>   - OAuth device_code (1/3 模式缺, 真实现 2/3)
>   - 6 module (cron/skills/acp/config/test/eval) 0 落, 0 近似
>   - Memory 3 Provider (in_memory/file/mongodb) 0 落, 0 提及 (mongodb)

---

## 1. 3 件偏差 + 估时 + 优先级

### A. OAuth device_code (1/3 缺)

- **现状**: apeireth-oauth 8 .rs 文件, 真实现 authorization_code + PKCE (264 命中), device_code 0 落
- **缺啥**: device_code grant_type + 设备码轮询 (poll) + user_code 一次性码 + verification_uri 引导
- **估时**: 3 天 (1 dev × 3 days)
- **优先级**: P3 (1.0 release 0 需 device_code, 2.0+ 估补)
- **依赖**: 0 (独立, 借 Golutra device_code flow)
- **范围**: 估 200-300 行 + 5-8 tests

### B. 6 module 0 落

| Module | 0 落 | 估时 | 优先级 | 替代 |
|--------|:----:|:---:|:---:|------|
| `apeireth-cron` | 0 | 1 周 | P2 | system cron |
| `apeireth-skills` | 0 | 1 周 | P2 | apeireth-plugin 部分含 |
| `apeireth-acp` | 0 | 1 周 | P3 | MCP 协议 |
| `apeireth-config` | 0 | 3 天 | P2 | Cargo.toml workspace config |
| `apeireth-test` | 0 | 0 (0 需) | P4 | tests/ 目录已够 |
| `apeireth-eval` | 0 | 1 周 | P2 | apeireth-bench 部分含 |
| **小计** | | **5 周** | | |

- **优先级总评**: P2 (4 件, 估 4 周) + P3 (1 件, 估 1 周) + P4 (1 件, 0 估时)
- **依赖**: cron / skills / config / eval 4 件独立, acp 依赖 claude-code ACP 协议, test 0 需

### C. Memory 3 Provider 0 落

| Provider | 0 落 | 估时 | 优先级 | 状态 |
|----------|:----:|:---:|:---:|------|
| `in_memory` | 0 提及 0 落 | 2-3 天 | P1 | HashMap 包装 |
| `file` | 0 提及 0 落 | 3-5 天 | P1 | 本地文件存储 |
| `mongodb` | 0 提及 0 落 | 1 周 | P3 | mongodb client 包装 |
| **小计** | | **2 周** | | |

- **优先级总评**: P1 (2 件, 估 1 周) + P3 (1 件, 估 1 周)
- **依赖**: in_memory / file 独立, mongodb 依赖 mongodb crate (Cargo.toml 新增)

---

## 2. R23 续补总估时

| 偏差 | 估时 | 优先级 |
|------|:---:|:---:|
| A. OAuth device_code | 3 天 | P3 |
| B. 6 module 0 落 | 5 周 | P2 (4 周) + P3 (1 周) + P4 (0) |
| C. Memory 3 Provider | 2 周 | P1 (1 周) + P3 (1 周) |
| **总计** | **8 周 (估)** | |

**P1 (高优先)**: in_memory + file = **1 周** (估 5-8 天)
**P2 (中优先)**: cron + skills + config + eval + 6 module 替代 = **4 周**
**P3 (低优先)**: device_code + acp + mongodb = **3 周**
**P4 (跳过)**: test = 0

---

## 3. R23 续补 7 commit 模式 (跟整合 #3+#4+#5 1:1)

per 整合 #3+#4+#5 模式, R23 续补估 7 commit 收尾:

| # | 主题 | 估时 | 范围 |
|---|------|:---:|------|
| **C15** | R23 P1 Memory 2 Provider (in_memory + file) | 1 周 | Memory 7 → 7 真实现 |
| **C16** | R23 P2 cron + skills (估 1:1 借) | 1 周 | 30 v1 目标 14/30 → 16/30 |
| **C17** | R23 P2 config + eval | 1 周 | 30 v1 目标 16/30 → 18/30 |
| **C18** | R23 P3 device_code (估 1:1 借 Golutra) | 3 天 | OAuth 2/3 → 3/3 |
| **C19** | R23 P3 acp (估 1:1 借 claude-code) | 1 周 | 30 v1 目标 18/30 → 19/30 |
| **C20** | R23 P3 mongodb (估 1:1 借 mongodb crate) | 1 周 | Memory 7/7 → 真 7/7 |
| **C21** | R23 收尾 + 1.1 release RC 验证报告 | 1 天 | R23 落地 6 commit + final-alignment 报告 |
| **总计** | | **6 周 + 1 天** | |

**注**: 7 commit 估时 6 周 + 1 天, 不是 8 周, 因为并行 (P2 4 件估 4 周, 但可以重叠).

---

## 4. 4 阻塞项 vs 9 件 R22 已落

R22 (整合 #5) 16 commit 落 9 件:
- ST-A1 (9 器官 TUI 5/9 真接 backend, 4/9 placeholder)
- ST-A2 (维度 1 生命层 反思期+认知梦+涌现+6历史流+主体连续性 全真实现)
- ST-A3 (12 维度 R-Measure M7-M12 深度增强)
- ST-A4 (FormalEngine 8 文件重建)
- ST-A5 (5/5 Kani 不变量)

**4 阻塞项 (R23 必做)**:
1. **B 步骤未做**: 删 cosign.key 私钥 + 推 git tag v1.0.0 (per 主人 8/6 12:50 "codex 干完了, 你看看")
2. **5 R-Measure bench 实际未产 measurement**: criterion main 与 libtest 共存导致空输出, 估 1 天修 harness
3. **9 器官 TUI 4/9 仍 placeholder**: brain / eye (mouse/voice) / ear (tool) / body (sysinfo), 估 2 周补
4. **R22 实测 5 项透明缺口** (per 1a172e96 报告):
   - formal 4 backend (Z3/CVC5/Coq/Lean4) 0 在本机调用
   - .github/workflows/*.yml 真 CI 0 跑
   - docs/security/cosign.pub 0 存在 (Mavis 估 publish-pubkey 推)
   - 51 历史文档 0 批量重写
   - workspace 既存 warnings

---

## 5. 4 阻塞项 守门状态 (per 8 项承诺)

| 阻塞项 | 守门 | 8 项承诺 | 估时 |
|-------|:----:|:----:|:----:|
| cosign.key 私钥 | 0 删 (Mavis 报告 "策略拦截") | #2 私钥不漏 | 5 分钟 (Recycle Bin) |
| git tag v1.0.0 | 0 推 (Mavis 报告 "未授权不擅推") | #6 0 擅推 | 5 步 30 秒 |
| 5 R-Measure bench | criterion + libtest 共存, 0 measurement | #1 不假装已实现 | 1 天修 harness |
| 9 器官 TUI 4/9 placeholder | brain/eye/ear/body 仍 stub | #5 诚实标缺 | 2 周补 |

---

## 6. R23 续补依赖图

```
R22 (整合 #5, 16 commit, 8/6 11:30 已落) ✅
   ↓
主人 8/6 12:25 拍 "三项接受"
   ↓
R23 续补 8 周 (per 本计划)
   ├─ P1 (1 周): C15 Memory in_memory + file
   ├─ P2 (4 周): C16 cron + skills, C17 config + eval
   └─ P3 (3 周): C18 device_code, C19 acp, C20 mongodb
   ↓
C21 R23 收尾 1 天
   ↓
1.1 release 候选
```

---

## 7. 4 阻塞项立即处理 (per 主人 "codex 干完了, 你看看")

**主人 1 句话定**:

- **A. 主人立刻手动**: 删 cosign.key 私钥 (5 分钟) + 推 git tag v1.0.0 (5 步 30 秒) — 0 阻塞
- **B. 主人让 Mavis 处理**: 4 阻塞项 (cosign/key 删除 + tag 推 + bench 修 + TUI 4 器官补) — Mavis 已熟悉架构
- **C. 0 立即, 主人先内测 1.0 release** (per 主人 8/6 08:10 "没内测通过就不打 1.0"), 内测后主人 1 句话开 R23

**本座 0 动, 等主人 1 句话**.

---

## 8. 来源

- 本座 8/6 12:30-12:50 跑 32 项审计 (per reports/apeireth-vision-alignment-readout-2026-08-06.md v1+v2)
- Mavis 8/6 17:58-18:00 写 2 commit (d4f0bfc7 + 1a172e96) + reports/apeireth-final-alignment-2026-08-06.md 75 行 9-item 自审
- 主人 8/6 12:50 拍 "没做的加到后续计划, codex 干完了, 你看看"
- 主人 8/6 12:25 拍 "三项接受, 按你建议来"
- 主人 8/5 21:35 拍 "0 主动 commit, 留整合 #3 拍板"
- 主人 8/6 08:10 拍 "没内测通过就不打 1.0"

---

_报告落盘: 2026-08-06 12:55, Mavis 收尾阶段 1 屏审计 + R23 续补计划_
