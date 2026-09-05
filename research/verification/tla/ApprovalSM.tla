---- MODULE ApprovalSM ----
\* Phase 5 (RA-5) 审批状态机 TLA+ 规格 —— 1:1 对应
\* crates/engine/runtime/src/canonical/research_approval_sm.rs
\* 不变量与 Rust 侧 inv_a/inv_b/inv_c 完全同构 (修正版语义, 见文件内注释)。
\* 崩溃模型: durable 前缀回退 + Dispatched∧¬result_appended → Interrupted。

EXTENDS Integers, TLC

CONSTANTS REC,       \* 审批 id 集合 (模型检查用单元素 {"a"})
          EXPIRES_AT \* 过期时间点 (时钟常数, 保持状态空间有限)

VARIABLES status, executed, appended, durable, approved, active, now

vars == <<status, executed, appended, durable, approved, active, now>>

States == {"pending", "claimed", "dispatched", "consumed",
           "rejected", "expired", "interrupted"}

\* ---------------------------------------------------------------------------
\* 类型不变量 (Rust: ResearchApprovalRecord / ResearchApprovalMachine 字段)
\* ---------------------------------------------------------------------------
TypeOK ==
    /\ status   \in [REC -> States]
    /\ executed \in [REC -> BOOLEAN]
    /\ appended \in [REC -> BOOLEAN]
    /\ durable  \in [REC -> BOOLEAN]
    /\ approved \in [REC -> BOOLEAN]
    /\ active   \in REC \cup {"none"}
    /\ now    \in 0 .. (EXPIRES_AT + 1)

Init ==
    /\ status   = [r \in REC |-> "pending"]
    /\ executed = [r \in REC |-> FALSE]
    /\ appended = [r \in REC |-> FALSE]
    /\ durable  = [r \in REC |-> FALSE]
    /\ approved = [r \in REC |-> FALSE]
    /\ active   = "none"
    /\ now    = 0

\* ---------------------------------------------------------------------------
\* Next 关系 (Rust: ResearchApprovalMachine::next, 逐条转移 + 非法转移拒绝)
\* ---------------------------------------------------------------------------

\* Pending -> Claimed (P2 claim-before-effect 落账)
Approve(r) ==
    /\ status[r] = "pending"
    /\ status'   = [status EXCEPT ![r] = "claimed"]
    /\ durable'  = [durable EXCEPT ![r] = TRUE]
    /\ approved' = [approved EXCEPT ![r] = TRUE]
    /\ active'   = r
    /\ UNCHANGED <<executed, appended, now>>

\* Pending -> Rejected (Reject / Cancel)
Reject(r) ==
    /\ status[r] = "pending"
    /\ status' = [status EXCEPT ![r] = "rejected"]
    /\ UNCHANGED <<executed, appended, durable, approved, active, now>>

\* Pending -> Expired (P6 落账; 未到点 no-op 保持 Pending)
Expire(r) ==
    /\ status[r] = "pending"
    /\ now > EXPIRES_AT
    /\ status'  = [status EXCEPT ![r] = "expired"]
    /\ durable' = [durable EXCEPT ![r] = TRUE]
    /\ active'  = "none"
    /\ UNCHANGED <<executed, appended, approved, now>>

\* Claimed -> Dispatched (P3 Dispatched 先持久化再发请求; executed 单调翻转)
BeginDispatch(r) ==
    /\ status[r] = "claimed"
    /\ status'   = [status EXCEPT ![r] = "dispatched"]
    /\ executed' = [executed EXCEPT ![r] = TRUE]
    /\ durable'  = [durable EXCEPT ![r] = TRUE]
    /\ UNCHANGED <<appended, approved, active, now>>

\* Dispatched -> Consumed (P4 落账)
Complete(r) ==
    /\ status[r] = "dispatched"
    /\ status'   = [status EXCEPT ![r] = "consumed"]
    /\ appended' = [appended EXCEPT ![r] = TRUE]
    /\ durable'  = [durable EXCEPT ![r] = TRUE]
    /\ active'   = "none"
    /\ UNCHANGED <<executed, approved, now>>

\* Claimed/Dispatched -> Interrupted (P5 落账; 允许 executed=FALSE 的显式 fail-closed)
Interrupt(r) ==
    /\ status[r] \in {"claimed", "dispatched"}
    /\ status'  = [status EXCEPT ![r] = "interrupted"]
    /\ durable' = [durable EXCEPT ![r] = TRUE]
    /\ active'  = "none"
    /\ UNCHANGED <<executed, appended, approved, now>>

\* Claimed -> Interrupted (G3 重开落账语义)
RecoverClaimed(r) ==
    /\ status[r] = "claimed"
    /\ status'   = [status EXCEPT ![r] = "interrupted"]
    /\ durable'  = [durable EXCEPT ![r] = TRUE]
    /\ active'   = "none"
    /\ UNCHANGED <<executed, appended, approved, now>>

\* 时钟前进 (保证 Expire 可发生; 有界保证状态空间有限)。
\* 注: TLC 枚举后继状态要求 primed 变量为可赋值形式 (v' = e 或 v' \in S),
\* 故用 now' \in (now+1)..(EXPIRES_AT+1) 等价表达 now' > now /\ now' <= EXPIRES_AT+1。
Tick ==
    /\ now' \in (now + 1) .. (EXPIRES_AT + 1)
    /\ UNCHANGED <<status, executed, appended, durable, approved, active>>

\* ---------------------------------------------------------------------------
\* 崩溃模型 (Rust: ResearchApprovalMachine::simulate_crash, RA-5 §5)
\*   - 非 durable 记录丢失: 视为从未发生 (回退 Pending, executed/appended/approved 复位)
\*   - durable 且 Dispatched ∧ ¬appended -> 强制 Interrupted (InvC)
\*   - durable 的 Claimed -> 恢复为 active
\* ---------------------------------------------------------------------------
Crash ==
    LET newStatus(r) ==
            IF ~durable[r] THEN "pending"
            ELSE IF status[r] = "dispatched" /\ ~appended[r] THEN "interrupted"
            ELSE status[r]
        newExecuted(r) == IF ~durable[r] THEN FALSE ELSE executed[r]
        newAppended(r) == IF ~durable[r] THEN FALSE ELSE appended[r]
        newApproved(r) == IF ~durable[r] THEN FALSE ELSE approved[r]
    IN  /\ status'   = [r \in REC |-> newStatus(r)]
        /\ executed' = [r \in REC |-> newExecuted(r)]
        /\ appended' = [r \in REC |-> newAppended(r)]
        /\ approved' = [r \in REC |-> newApproved(r)]
        /\ durable'  = durable
        /\ active'   = (IF \E r \in REC : durable[r] /\ status[r] = "claimed"
                        THEN CHOOSE r \in REC : durable[r] /\ status[r] = "claimed"
                        ELSE "none")
        /\ UNCHANGED <<now>>

Next ==
    \/ \E r \in REC : Approve(r)
    \/ \E r \in REC : Reject(r)
    \/ \E r \in REC : Expire(r)
    \/ \E r \in REC : BeginDispatch(r)
    \/ \E r \in REC : Complete(r)
    \/ \E r \in REC : Interrupt(r)
    \/ \E r \in REC : RecoverClaimed(r)
    \/ Crash
    \/ Tick

Spec == Init /\ [][Next]_vars

\* ---------------------------------------------------------------------------
\* 三不变量 (Rust: inv_a / inv_b / inv_c, 修正版语义 —— 与实现 1:1)
\* ---------------------------------------------------------------------------

\* InvA — 无双副作用:
\*   executed=TRUE 时不得处于 Pending/Claimed/Rejected/Expired;
\*   Dispatched/Consumed 必有 executed=TRUE。
InvA ==
    \A r \in REC :
        ( executed[r] => status[r] \notin {"pending", "claimed", "rejected", "expired"} )
        /\ ( status[r] \in {"dispatched", "consumed"} => executed[r] )

\* InvB — 批准意图不丢 (liveness 语义的 safety 面):
\*   已持久化且已批准且未派发未落账的意图, 只允许 Claimed/Dispatched/Interrupted,
\*   绝不回 Pending/Rejected/Expired。
InvB ==
    \A r \in REC :
        ( durable[r] /\ approved[r] /\ ~executed[r] /\ ~appended[r] )
            => status[r] \in {"claimed", "dispatched", "interrupted"}

\* InvC — 效果不确定强制 fail-closed:
\*   已派发未落账 (executed ∧ ¬appended) 只能 Dispatched 或 Interrupted。
InvC ==
    \A r \in REC :
        ( executed[r] /\ ~appended[r] )
            => status[r] \in {"dispatched", "interrupted"}

\* 终态锁 (Rust: is_final 无出边) —— 结构上由 Next 保证, 附带检查:
\*   终态下没有任何转移动作被使能。
TerminalLock ==
    \A r \in REC :
        status[r] \in {"consumed", "rejected", "expired", "interrupted"}
        => ~(ENABLED (Approve(r) \/ Reject(r) \/ Expire(r) \/ BeginDispatch(r)
                      \/ Complete(r) \/ Interrupt(r) \/ RecoverClaimed(r)))

====
