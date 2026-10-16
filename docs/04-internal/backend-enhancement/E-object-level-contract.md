# E-P3 · 对象级判定契约（威胁模型 G3 处置）

> `Status: ✅ 已交付（E 分期 P3：对象级判定层 + 沙箱执行面接线 + 星港线接入契约）`
> 一事：受控子进程的文件操作判定从「命令词扫描」补强为**对象级判定** —— 执行前后
> 对象状态对比（事实判定，与 API 形态无关）+ 越界即帧 + `sandbox.object_violation`
> 审计 + `ObjectAccessPolicy` 挂载点（星港线深望智能体接此契约放行）。
> 纪律：[CONSTRUCTION-DISCIPLINE.md](CONSTRUCTION-DISCIPLINE.md)（进场第一课）；
> 上游：[E-threat-model.md](E-threat-model.md) G3 / [E-security-depth.md](E-security-depth.md) 想法 1；
> 战场：`crates/capabilities/tools/src/object_access.rs`（新建判定件）+ 旁侧判定面
> （`observed_gate.rs` 路径归一复用）+ 沙箱执行面最小接线（`shell.rs`，见 §6 报备）。

## 1. 缺口与处置（如实）

**G3 缺口**：词法/治理层已兜住「换 API 换权力」的大半（`write_intent.rs` 命令面族词
与静态调用面同族同权、`tests/delete_path_parity.rs` 锁对称），但 **API 形态不对称**
依旧存在于两处：

1. **仓外拦截层**（沙箱本体的进程/API 拦截层，本仓改不了）：按 cmdlet 形态拦、
   静态调用形态穿 —— 实测 `Remove-Item` 拒、`[System.IO.File]::Delete` 成；
2. **本仓词法面的明示漏放边界**（`write_intent.rs` 注释在案）：编码负载
   （`-EncodedCommand` 形态）、解释器内写入、解包类工具写盘、动态成员名/反射调用。

**处置**：本仓可控面立**对象级判定**——判定按「对象（路径对象域 × 四动作）」记账，
与 API 形态无关；受控执行**前后对象状态对比**得出「动了哪些对象」的事实，再逐对象
过策略，未授权即违规（失败即帧 + 审计）。词法放行但对象越界的形态（上列两类）由
这层按对象事实兜底。

**残余（如实，不美化）**：
- **仓外拦截层的形态不对称仍在**（本仓外，改不了）——但其「换 API 换权力」的收益
  被本仓面的判定对冲：同对象同动作不论哪条 API 路径落地，判定逐一相同（测试钉死）；
- **事实判定是事后判定**：违规对象变更已发生在盘上，判定给出失败帧 + 问责审计，
  回滚不在本层（可逆性由受控写工具/审批链负责）；
- **事实判定有域界**：快照只覆盖对象域内，域外对象变更不入差分（域外访问由请求时
  判定 + 沙箱物理围栏 + cwd 边界拒绝兜住）；
- **枚举零状态痕迹**：枚举不改对象状态、不入事实差分，只在请求时判定面裁决；
- **指纹口径**：对象指纹 =（种类, 长度, mtime）元数据三元组，不读文件内容（与读前
  观测门禁同口径）；同长同 mtime 粒度内的原地改写不可见；
- **同域并发窗口互相可见**：差分是窗口级事实（归属窗口级、非进程级）；并发安全面
  = 快照即值 + 判定器无共享可变态，不相交域并发零串扰、审计不丢行（测试钉死）；
- **审计面默认进程内**：`MemoryObjectAuditSink` 重启即重置（与读前观测表同一边界），
  落盘 sink 由宿主接线（挂载点已立，见 §3）。

## 2. 判定语义

### 2.1 对象词表（冻结）

| 词表 | 词 | 稳定词面 | 事实面（差分） | 请求面（check） |
|---|---|---|---|---|
| 动作 `ObjectAction` | 创建 / 修改 / 删除 / 枚举 | `create` / `modify` / `delete` / `enumerate` | 前三类入差分；枚举零状态痕迹 | 四类全走 |
| 域 `ObjectZone` | 域内 / 敏感 / 域外 | `workspace` / `sensitive` / `outside_domain` | 快照差分逐对象定域 | 请求即定域 |
| 种类 `ObjectKind` | 文件 / 目录 | — | 目录指纹只看种类（成员由子对象差分承担） | — |

域判定（`ObjectDomain::zone_of`）：域内且命中敏感面拒绝词表（`sensitive_path.rs`
族词表，只读消费）= 敏感域；对不上域根（含拼写不一致）= 域外（fail-closed，不猜）。
路径归一复用读前观测门禁同一把尺（`observed_gate::path_key`：统一分隔、折叠 `..`、
剥 verbatim 前缀；不展开 symlink、不折大小写）。

### 2.2 两个判定面

| 面 | 入口 | 输入 | 产出 | 覆盖 |
|---|---|---|---|---|
| **请求时判定** | `ObjectAccessJudge::check` | `ObjectAccessRequest{subject, object, action, zone}` | `ObjectVerdict::Allow / Deny{reason}` | 声明面动作：枚举、预声明对象操作（星港线主面） |
| **事实判定** | `ObjectAccessJudge::guard` / `open`+`verify` | 执行前后对象快照差分 `diff_object_states` | `ObjectGuardReport{changes, violations, probe_error, complete}` | 受控子进程落地的一切对象变更，**与 API 形态无关** |

事实判定流程：开窗（执行前快照）→ 受控执行 → 执行后快照 → 事实差分（动了哪些
对象 = 事实）→ 逐对象过策略 → 未授权 = 违规（`ObjectViolation{object, action,
process, zone, reason}`）。

### 2.3 越界即帧 + 审计

- **失败即帧**：`ObjectGuardReport::failure()` 走**闭合失败词表**出帧
  （`pipeline.post_block`，source=`object_access_judge`），理由串以
  `object violation:` 起头、逐条列对象/动作/进程/域/理由 —— 失败走正式通道报真因，
  不静默降级、无半态错误；
- **审计**：每条违规落一行 `sandbox.object_violation`（`object=… action=…
  process=… zone=… reason=…`），经 `ObjectAuditSink` 挂载点落账；合规执行零帧
  零审计行；
- **可见墙**：合规执行的结果载荷带 `object_audit{changes, violations, coverage}`
  计数（`coverage=complete|partial` 如实标注快照覆盖）；
- **fail-closed**：判定窗开不出（域根不可读）→ 受控执行**不启动**，
  `ObjectGuardError::Unobservable` 报真因；执行后快照失败 → 按 fail-closed 判失败。

## 3. 挂载点（能力接口）

| 接口 | 语义 | 默认实现 | 挂载处 |
|---|---|---|---|
| `ObjectAccessPolicy` | 对象级裁决（谁/对象/动作/域 → Allow/Deny+理由），纯判定无 IO | `WorkspaceObjectPolicy`（域内放行、敏感/域外一律拒） | `ObjectAccessJudge::new` / `ObjectPolicyChain` |
| `ObjectStateProbe` | 执行前后域快照 | `FsObjectStateProbe`（元数据三元组，条目上限 200k，触顶如实 `complete=false`） | 同上 |
| `ObjectAuditSink` | 审计行落账（`事件名 空格 细节`，与网关审计行同形） | `MemoryObjectAuditSink`（进程内，重启重置） | 同上 |
| `ObjectPolicyChain` | 策略叠加：顺序判定，第一个拒绝即拒绝，全放行才放行 | — | 附加策略叠加在默认策略之上 |
| shell 执行面 | 受控子进程的对象事实兜底 | `TrustedShellConfig::object_judgment = None` → 默认判定器 | `TrustedShellConfig::with_object_judgment` |

## 4. 给星港线（深望智能体）的接入契约

深望（浏览/语义树的智能体操控）接此契约放行，三步：

1. **实现对象策略**（只按对象与动作立规则，不看 API 形态）：

```rust
struct DeepWatchPolicy { /* 智能体的语义树对象域规则 */ }

impl ObjectAccessPolicy for DeepWatchPolicy {
    fn name(&self) -> &str { "deepwatch_object_policy" }
    fn decide(&self, request: &ObjectAccessRequest<'_>) -> ObjectVerdict {
        // request: subject（谁）/ object（哪个对象）/ action（四动作）/ zone（域）
        match (request.zone, request.action) {
            (ObjectZone::InsideWorkspace, ObjectAction::Enumerate) => ObjectVerdict::Allow,
            (ObjectZone::InsideWorkspace, _) if self.is_semantic_tree_object(request.object) => {
                ObjectVerdict::Allow
            }
            _ => ObjectVerdict::deny("deepwatch: 对象不在语义树操控范围"),
        }
    }
}
```

2. **装进判定器并挂载**（策略链叠加在默认策略之上，不替换纵深）：

```rust
let judge = Arc::new(ObjectAccessJudge::new(
    Arc::new(ObjectPolicyChain::new(vec![
        Arc::new(WorkspaceObjectPolicy),
        Arc::new(DeepWatchPolicy::default()),
    ])),
    Arc::new(FsObjectStateProbe::new()),
    Arc::new(MemoryObjectAuditSink::new()), // 生产换落盘 sink（实现 ObjectAuditSink）
));
let tool = ShellTool::new(TrustedShellConfig::new(root).with_object_judgment(Arc::clone(&judge)));
```

3. **自持工具面消费判定**：声明面动作先 `judge.check(&domain.request(...))` 再动手；
   自带受控执行的用 `judge.guard(subject, domain, || …)` 包住执行本体，拿
   `ObjectGuardReport`（changes/violations）自定帧或直接用
   `report.failure()` 出标准帧；审计行经 `judge.audit_rows()`（进程内）或自挂
   sink 落盘。

**契约承诺**：① 判定输入只有对象/动作/域/主体，API 形态永不进裁决；② 词表冻结
（四动作/三域/稳定词面），改动=治理语义变更，须同步本文件与测试矩阵；③ 违规必出
帧 + `sandbox.object_violation` 审计行，合规零变化；④ 挂载点全可替换（策略/探针/
审计），默认件可直接用。

## 5. 测试证据（16 测试全绿）

`crates/capabilities/tools/tests/object_access_judgment.rs`（13）+
`src/object_access.rs` 内（3）：

| # | 测试 | 钉住的行为 |
|---|---|---|
| 1 | `object_decision_matrix_covers_four_actions_across_zones` | 判定矩阵：四动作 ×（域内合规/敏感文件/敏感嵌套/域外）16 格，合规放行、越界一律拒 |
| 2 | `state_diff_covers_create_modify_delete_and_keeps_untouched_objects` | 事实矩阵：创建/修改/删除逐一对账，未动对象零入账 |
| 3 | `window_comparison_attributes_only_in_window_object_changes` | 执行前后状态对比正确性：只认窗口内事实，修改≠创建 |
| 4 | `concurrent_object_judgments_stay_independent_and_keep_every_audit_row` | 并发安全：共享判定器 × 不相交域，零串扰、审计一行不丢 |
| 5 | `same_object_same_verdict_across_api_forms` | 形态无关：档案 API / 句柄 API / 子进程命令面同对象同判定 |
| 6 | `lexical_pass_but_object_violation_is_fenced_with_frame_and_audit` | 词法放行但对象越界兜底（编码负载删 `.env`）：词法+守门放行实证 → 失败即帧 + 审计行 |
| 7 | `compliant_lexical_pass_keeps_the_result_and_the_object_audit_wall` | 对照：同形态合规变更零帧零审计行 + `object_audit` 可见墙 |
| 8 | `violation_audit_rows_name_object_action_and_process` | 审计行：对象/动作/进程/域/理由逐项在行 + 帧走 `pipeline.post_block` |
| 9 | `fake_policy_mount_decides_and_chain_keeps_first_deny_wins` | 挂载点：fake policy 请求面+事实面都被征询；策略链第一个拒绝即拒绝 |
| 10 | `shell_mounted_object_policy_replaces_the_default_face` | 生产面挂载点：同命令默认放行、挂载禁改策略拦下 |
| 11 | `enumerate_is_judged_on_the_request_face_and_leaves_no_fact_trace` | 枚举：请求面同受矩阵、事实面零痕迹（如实边界） |
| 12 | `unobservable_domain_fails_closed_without_running_the_payload` | fail-closed：判定窗开不出 = 执行不启动 |
| 13 | `domain_boundary_misspellings_fail_closed_to_outside` | 域界词法口径：拼写对不上域根按域外拒 |
| 14-16 | `object_access::tests::*` | 四动作词表冻结 / 纯差分事实表 / 域三分判定 |

零回归：既有 shell/写闸/执行流水线/进程边界套件 + 组装根 shell 审批 e2e 实跑全绿
（`delete_path_parity` / `shell_execution` / `exec_pipeline` / `process_executor` /
`architecture_invariants` / `canonical_shell_approval_e2e` / `file_write_registration` 等），
全量 `cargo test --workspace --locked -j 2` 的套件数口径随交付回报实测值；在飞各线
战场零触碰（见 §6）。

## 6. 跨范围最小改动报备（宪章 §二.4）

| 文件 | 改动 | 理由 |
|---|---|---|
| `crates/capabilities/tools/src/observed_gate.rs` | `fn path_key` → `pub(crate) fn path_key`（零语义变化） | 对象判定件复用同一路径归一尺，不复制第二份词法归一 |
| `crates/capabilities/tools/src/shell.rs` | 沙箱执行面包一个对象判定窗 + `TrustedShellConfig::with_object_judgment` 挂载点 + 结果 `object_audit` 计数 | 生产接线（IMPLEMENTED ≠ PRODUCTION WIRED 的整改要求）：受控子进程的对象事实兜底必须在真实执行面上跑 |

其余战场零触碰：治理审批规则面 / `write_intent.rs` / `guardrail.rs` /
`permission_preset`（G2 线，只读消费其族词表）、`frontend/**`、`CHANGELOG.md`、
`.github/**`、`docs/design/**`、`research/verification/**`、
`crates/engine/memory/{reflexion,river_topology}.rs`。

## 7. 威胁模型 G3 处置注

见 [E-threat-model.md](E-threat-model.md) §5 G3 处置注（本文件即其落地契约）。
一句话：**本仓面已闭环**（对象级判定 = 形态无关的事实判定 + 请求判定 + 越界即帧
+ 审计 + 挂载点），仓外拦截层的形态不对称如实留残余（本仓改不了，由对象级判定
对冲其权力差）。
