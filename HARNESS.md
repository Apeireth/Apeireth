# HARNESS.md — 薪火 / Promethean Harness 规范 v0.1

> **这不是文档,这是契约。**
> 任何接入薪火的 LLM,必须把自己的执行环境组织成下面 7 个组件。
> 每个组件的修改必须附 Change Manifest(见 §3),可被验证、被归因、被回滚。

---

## §0. 三句话定义

1. **Harness** = 包裹 LLM 的外系统,决定它"怎么观察、怎么行动、怎么记忆、怎么检查自己、怎么改进"
2. **薪火平台** = 一个让 Harness 自进化的开源框架(不动模型权重,只改 Harness 结构)
3. **超人工智能平台** = 任何 LLM 通过薪火的 Harness 自进化循环,在任意领域达到/超过人类专家水平

---

## §1. 7 个正交组件(HARNESS.md v1.0 标准)

```
<workspace>/
├── AGENTS.md                    # 1. System Rules       (宪法)
├── SOUL.md                      # 1. System Rules       (身份人格)
├── systemprompt.md              # 1. System Rules       (额外规则)
├── tool_descriptions/           # 2. Tool Descriptions  (产品说明书)
│   └── *.tool.yaml
├── tools/                       # 3. Tool Implementations(机器工人)
│   └── *.py / *.js
├── middleware/                  # 4. Middleware         (安检通道)
│   └── *.py
├── skills/                      # 5. Skills             (SOP 手册)
│   └── */SKILL.md
├── sub_agents/                  # 6. Sub-Agents         (外包团队)
│   └── */config.yaml
├── MEMORY.md                    # 7. Long-Term Memory   (个人笔记本)
└── experiences.md               # 7. Long-Term Memory   (经验教训)
```

### 最小可工作 Harness(必须满足)

只需要 2 个组件:
1. **System Rules** — 至少一个规则文件(AGENTS.md)
2. **Tool Descriptions + Implementations** — 至少一个工具

其他 5 个组件按需添加。AHE 论文证明:从最小化状态开始演化的效果最好。

---

## §2. 4 个差异化(薪火 vs VCP/OpenClaw/AHE/Claude Code)

### 2.1 本地优先(Local-First)
- **要求**:32G 笔记本 + RTX 5070 可一键跑
- **实现**:所有 LLM 调用走 API(Claude/DeepSeek/Qwen);本地只跑小模型(Qwen 7B-13B + LoRA)
- **不允许**:强依赖云服务(E2B / harbor);可本地 Docker 替代

### 2.2 安全优先(Safe-by-Default)
- **要求**:任何 Harness 修改必须经过 4 层安全门
- **实现**:
  - Layer 1 — **Process Gate**: git stash + diff size check(<200 行强制 review)
  - Layer 2 — **Sandbox Gate**: Landlock + seccomp + Docker rootless(no-network)
  - Layer 3 — **Evaluation Gate**: Harness Quality Benchmark + held-out regression gate
  - Layer 4 — **Human Gate**: 关键修改需 explicit human approval
- **不允许**:AHE 那种"全自动 evolve"(主人 07-19 明确指出 OpenClaw 69 CVE 是反例)

### 2.3 Benchmark 优先(Measurable-First)
- **要求**:每个 Harness 修改必须被 HQB(Harness Quality Benchmark)量化
- **实现**:Harness Quality Benchmark 4 维度 — SC 自洽性 / NR 抗噪性 / EV 可演化性 / CDT 跨域迁移
- **不允许**:只跑任务基准(任务表现 ≠ Harness 质量)

### 2.4 跨小模型(Cross-Small-Model)
- **要求**:同一 Harness 必须在 Qwen / Hermes / Llama / Gemma 上验证可迁移
- **实现**:冻结 Harness,跨模型 +3-5pp 视为合格
- **不允许**:绑死单一模型(像 AHE 绑 GPT-5.4)

---

## §3. Change Manifest Schema(每次 Harness 修改必须附)

### 3.1 完整 JSON Schema

```json
{
  "manifest_version": "1.0",
  "harness_spec_version": "0.1",
  "iteration": 3,
  "timestamp": "2026-07-20T10:30:00+08:00",
  "author": "evolve-agent-or-human",
  "trigger": "harness_quality_benchmark_drop",
  "changes": [
    {
      "change_id": "ch_001",
      "component": "tool_descriptions",
      "subtype": "update",
      "file_path": "tool_descriptions/search.tool.yaml",
      "summary": "一句话描述改了什么",
      "failure_evidence": "具体哪个 task / 哪个 trace 失败,trace ID 是多少",
      "root_cause": "为什么这个修改能修(根因,不是表面)",
      "targeted_fix": "具体改动的代码/diff",
      "predicted_impact": {
        "expected_fixes": ["task_id_1", "task_id_2"],
        "at_risk_regressions": ["task_id_3"],
        "rationale": "为什么这些 task 会修 / 这些会 regress"
      }
    }
  ],
  "safety_check": {
    "diff_lines": 47,
    "exceeds_review_threshold": false,
    "touches_protected_paths": ["MEMORY.md", ".env"],
    "requires_human_approval": false
  },
  "verification": {
    "status": "pending",
    "scheduled_at": "2026-07-21T10:30:00+08:00",
    "method": "harness_quality_benchmark",
    "expected_hqb_score_delta": "+2.3"
  }
}
```

### 3.2 三种验证结果

| Verdict | 条件 | 动作 |
|---|---|---|
| `keep` | HQB 总分提升 ≥ 0.5 | git commit + 更新 H_best |
| `partial` | HQB 总分 ±0.5 | 保留修改但标 partial,记录原因 |
| `revert` | HQB 总分下降 ≥ 0.5 | git revert,记录失败模式到 failure_taxonomy.md |

---

## §4. Harness 自进化主循环(Phase 1+ 都要遵循)

```python
for iteration in range(1, max_iterations + 1):
    # Phase 0: 快照 + git tag
    git_tag(f"iter_{iteration}_before")
    snapshot_workspace()
    
    # Phase 1: EVAL — 跑基准
    if skip_eval:
        job_dir = find_latest_job_dir()
    else:
        job_dir = run_benchmark(harness=harness, dataset=benchmark)
    
    # Phase 2: STATS + HQB 评分
    stats = compute_stats(job_dir)
    hqb_score = compute_hqb_score(harness, stats)  # 4 维度
    
    # Phase 2.5: Agent Debugger — 蒸馏失败为根因
    failure_report = distill_failures(job_dir, prev_failure_report)
    
    # Phase 3: EVOLVE — 用 LLM 改 Harness
    change_manifest = evolve_agent.propose_change(
        harness=harness,
        failure_report=failure_report,
        hqb_score=hqb_score
    )
    
    # Safety Gate 4 层
    if not safety_check(change_manifest):  # Layer 1-4
        revert(change_manifest)
        continue
    
    apply_change(change_manifest)
    
    # Phase 4: VERIFY — 跑下一次 EVAL 验证
    next_stats = run_benchmark(harness=updated_harness, dataset=benchmark)
    next_hqb = compute_hqb_score(updated_harness, next_stats)
    
    # Phase 5: COMMIT or ROLLBACK
    if next_hqb.total > hqb_score.total + 0.5:
        git_commit(change_manifest, verdict="keep")
        H_best = updated_harness
    elif abs(next_hqb.total - hqb_score.total) <= 0.5:
        git_commit(change_manifest, verdict="partial")
    else:
        git_revert(change_manifest, verdict="revert")
        record_to_failure_taxonomy()
```

---

## §5. 安全门 4 层(对应 §2.2)

### Layer 1 — Process Gate
```yaml
checks:
  - diff_size: <= 200 行强制人工 review
  - protected_paths: [MEMORY.md, .env, tools/sandbox/*, harness/self_modify.py]
  - git_stash_before: true
  - git_tag_after: f"iter_{N}_after"
```

### Layer 2 — Sandbox Gate(Landlock + seccomp + Docker)
```python
# promethean/safety/sandbox.py
import landlock
from promethean.safety.seccomp_strict import SECCOMP_STRICT_PROFILE

def run_evolve_in_sandbox(cmd: list[str]):
    # Landlock 限制 FS 访问:只能读写 workspace/ 和 experiments/
    landlock.restrict_fs(["workspace/", "experiments/"])
    # seccomp 限制 syscall:禁用网络/调试/重启
    seccomp.apply(SECCOMP_STRICT_PROFILE)
    # 禁用网络(netns 隔离)
    unshare(CLONE_NEWNET)
    return subprocess.run(cmd)
```

### Layer 3 — Evaluation Gate(HQB)
- 任何 Harness 修改提交前必须跑 HQB 4 维度
- 任一维度下降 ≥ 1 分 = 拒绝

### Layer 4 — Human Gate
- 触发条件:
  - diff > 200 行
  - 触及 protected paths
  - HQB 总分连续 2 次下降
  - 涉及 weights/RL/LoRA 修改(dual-lever 模式)
- 触发后:暂停自动循环,推送给主人审批

---

## §6. 失败模式分类学(Failure Taxonomy)

每次 `revert` 必须记录到 `failure_taxonomy.md`,7 类失败模式:

1. **Regression** — 总分下降 ≥ 0.5
2. **Mode Collapse** — 所有输出变得同质
3. **Reward Hacking** — 钻 HQB 评分漏洞
4. **Goal Misgeneralization** — 修了一个 task 但坏了 5 个
5. **Backdoor** — 故意留下隐藏行为
6. **Sandbox Escape** — 沙箱被绕过(最高危)
7. **Irreversible Drift** — 累积小修改导致方向漂移

---

## §7. 版本演进

| 版本 | 状态 | 关键差异 |
|---|---|---|
| v0.1 | ✅ 当前 | 本文件 — 7 组件 + 4 差异化 + Manifest schema + 主循环骨架 |
| v0.2 | 待 | 加上 Phase 0 sandbox 实现 + HQB v1 (SC + EV 两维度) |
| v1.0 | 待 | HQB 4 维度 + 跨小模型验证 + 12 周路线图完成 |
| v2.0 | 待 | Dual-lever 模式 + Workflow Design 派融合 |
| v3.0 | 待 | L2 Meta²(meta-procedure 本身可改,极危险) |

---

## §8. 参考文献

基于以下真证据(不堆词):
1. **AHE HARNESS.md v1.0** (Fudan, arxiv 2604.25850) — 7 组件 + Manifest schema
2. **Lilian Weng 2026-07-04 Harness Engineering** — 5 阶段 ASI 路径
3. **ACE** (Stanford/SAP/Berkeley, arxiv 2510.04618) — Generator/Reflector/Curator
4. **MCE** (Haoran Ye et al. 2026, arxiv 2601.21557) — 双层 skill optimization
5. **DGM** (Sakana AI, arxiv 2505.22954) — archive + open-ended exploration
6. **Self-Harness** (Zhang et al. 2026-06, arxiv 2606.09498) — 模型自改三阶段
7. **SIA** (Hebbar et al. 2026-05, arxiv 2605.27276) — 双重杠杆(harness + weights)

---

_本规范由楚零 2026-07-20 起草,基于 AHE HARNESS.md v1.0 + 主人 07-19 反馈 + 笔记 01-16 综合。_
_下次修订:promethean v0.2 实现后,根据跑通情况调整。_