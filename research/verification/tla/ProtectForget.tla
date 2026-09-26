---- MODULE ProtectForget ----
\* 记忆 protect/forget 守恒 TLA+ 规格 —— 抽象自
\*   crates/engine/memory/src/memory_governance.rs
\*     (forget_episode_impl: protected → Protected 拒绝 / 二次 forget →
\*      AlreadyForgotten 拒绝 / 否则软删 status='forgotten', CAS revision)
\*   crates/engine/memory/src/retention.rs
\*     (sweep_session: protected 与已遗忘条目跳过, 其余按年龄/衰减判据遗忘)
\* 与 Kani 侧 harness_memory_conservation.rs (bitemporal_graph 纯模型投影) 互补:
\* 本模型直接刻画 SQL 转移规则本身, 全状态空间检查守恒命题。
\*
\* 命题:
\*   InvProtectedNeverForgotten — 被 protect 的 active 条目在任何 forget
\*                                路径下状态无变化 (条目仍存在);
\*   InvSweepKeepsProtected     — retention 路径同口径 (任意 drop 判定下跳过);
\*   InvForgetIdempotent        — forget 幂等: 二次 forget 状态无变化 (f∘f = f);
\*   InvNoResurrection          — forgotten 粘滞, 无复活路径。

EXTENDS Integers, TLC

CONSTANTS REC,      \* 记忆条目 id 集 (模型检查用 {"a","b"} 或 {"a"})
          MAX_REV   \* 修订号上界 (CAS 到顶后转移停用 —— 状态空间有限化边界)

VARIABLES status,       \* [REC -> {"active","forgotten"}]
          protected,    \* [REC -> BOOLEAN]
          rev,          \* [REC -> 0..MAX_REV]
          was_forgotten \* [REC -> BOOLEAN] 单调墓志 (粘滞性检查用)

vars == <<status, protected, rev, was_forgotten>>

cur == [status |-> status, protected |-> protected, rev |-> rev]

TypeOK ==
    /\ status       \in [REC -> {"active", "forgotten"}]
    /\ protected    \in [REC -> BOOLEAN]
    /\ rev          \in [REC -> 0 .. MAX_REV]
    /\ was_forgotten \in [REC -> BOOLEAN]

Init ==
    /\ status        \in [REC -> {"active", "forgotten"}]
    /\ protected     \in [REC -> BOOLEAN]
    /\ rev           = [r \in REC |-> 0]
    /\ was_forgotten = [r \in REC |-> status[r] = "forgotten"]

\* ---------------------------------------------------------------------------
\* 转移规则的纯函数形态 (Rust: forget_episode_impl 的前检查 + 软删)
\*   active ∧ ¬protected ∧ rev < MAX_REV → forgotten, rev+1
\*   否则 (Protected / AlreadyForgotten / CAS 到顶) 拒绝, 状态无变化
\* ---------------------------------------------------------------------------
ForgetOp(st, r) ==
    IF st.status[r] = "active" /\ ~st.protected[r] /\ st.rev[r] < MAX_REV
    THEN [status    |-> [st.status EXCEPT ![r] = "forgotten"],
          protected |-> st.protected,
          rev       |-> [st.rev EXCEPT ![r] = st.rev[r] + 1]]
    ELSE [status    |-> st.status,
          protected |-> st.protected,
          rev       |-> st.rev]

\* retention 路径单条目结算 (Rust: sweep_session 的年龄/衰减判据抽象为
\* 任意布尔 drop; protected/已遗忘跳过是其守恒面, 与 forget 同一纯核)。
SweepOp(st, r, drop) ==
    IF drop /\ st.status[r] = "active" /\ ~st.protected[r]
    THEN ForgetOp(st, r)
    ELSE [status    |-> st.status,
          protected |-> st.protected,
          rev       |-> st.rev]

\* ---------------------------------------------------------------------------
\* Next 关系 (Rust: protect_episode / unprotect_episode / forget_episode /
\*            retention::sweep_session 逐条路径)
\* ---------------------------------------------------------------------------
Protect(r) ==
    /\ rev[r] < MAX_REV
    /\ protected' = [protected EXCEPT ![r] = TRUE]
    /\ rev'       = [rev EXCEPT ![r] = rev[r] + 1]
    /\ UNCHANGED <<status, was_forgotten>>

Unprotect(r) ==
    /\ rev[r] < MAX_REV
    /\ protected' = [protected EXCEPT ![r] = FALSE]
    /\ rev'       = [rev EXCEPT ![r] = rev[r] + 1]
    /\ UNCHANGED <<status, was_forgotten>>

Forget(r) ==
    LET st == ForgetOp(cur, r)
    IN  /\ status'        = st.status
        /\ protected'     = st.protected
        /\ rev'           = st.rev
        /\ was_forgotten' = [was_forgotten EXCEPT
                                ![r] = was_forgotten[r] \/ st.status[r] = "forgotten"]

Sweep(r) ==
    \E drop \in BOOLEAN :
        LET st == SweepOp(cur, r, drop)
        IN  /\ status'        = st.status
            /\ protected'     = st.protected
            /\ rev'           = st.rev
            /\ was_forgotten' = [was_forgotten EXCEPT
                                    ![r] = was_forgotten[r] \/ st.status[r] = "forgotten"]

Stutter == UNCHANGED vars

Next ==
    \/ \E r \in REC : Protect(r)
    \/ \E r \in REC : Unprotect(r)
    \/ \E r \in REC : Forget(r)
    \/ \E r \in REC : Sweep(r)
    \/ Stutter

Spec == Init /\ [][Next]_vars

\* ---------------------------------------------------------------------------
\* 守恒不变量
\* ---------------------------------------------------------------------------

\* P1: 被 protect 的 active 条目在任何 forget 路径后仍存在 (转移无变化)。
InvProtectedNeverForgotten ==
    \A r \in REC :
        (protected[r] /\ status[r] = "active") => ForgetOp(cur, r) = cur

\* P2: retention 路径同口径 —— 任意 drop 判定下 protected active 条目无变化。
InvSweepKeepsProtected ==
    \A r \in REC :
        (protected[r] /\ status[r] = "active")
            => \A drop \in BOOLEAN : SweepOp(cur, r, drop) = cur

\* P3: forget 幂等 —— 对任意可达状态与任意条目, 二次 forget 状态无变化。
InvForgetIdempotent ==
    \A r \in REC : ForgetOp(ForgetOp(cur, r), r) = ForgetOp(cur, r)

\* P4: forgotten 粘滞 —— 一旦遗忘永不复活 (墓志单调蕴含当前态)。
InvNoResurrection ==
    \A r \in REC : was_forgotten[r] => status[r] = "forgotten"

====
