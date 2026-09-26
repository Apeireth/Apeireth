---- MODULE QuotaSchedule ----
\* 配额非负 + 调度安全 (含 PIP) TLA+ 规格 —— 抽象自
\*   crates/foundation/orchestration/src/cognitive_quota_scheduler.rs
\*     - CognitiveQuota::consume_step: 无符号 saturating 记账 (各维度独立);
\*     - CognitiveQuotaScheduler::schedule_next: BTreeMap 优先级序出队
\*       (P0 最高 → P4 最低), 层内 VecDeque FIFO;
\*     - boost_priority_for_lock (PIP): 持锁者 effective 优先级被提升到
\*       请求者级并移到该层队首 (push_front), 防优先级反转。
\* 与 Kani 侧 harness_quota_scheduler.rs (3 任务有界证明) 互补:
\* 本模型在任意交错下全状态空间检查调度安全与记账守恒。
\*
\* 命题:
\*   InvQuotaNeverNegative — 各记账维度取值域为非负 (0..MAXQ) 且扣减单调不减;
\*   InvQuotaExhaustSticky — 耗尽状态粘滞 (不复活);
\*   InvHeadMostUrgent     — 出队者必为最高紧急度 (高优先级不被低优先级抢先);
\*   InvPipNoInversion     — PIP 提升后的持锁者不被同级请求者插队;
\*   InvBoostNeverUpgrades — PIP 只降序 (effective ≤ base)。

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS TASKS,   \* 任务 id 集 ({"a","b"})
          MAXQ,    \* 记账饱和点 (saturating_add 上界, 保持状态空间有限)
          STEPS    \* 单次扣减步长上界

VARIABLES queue,        \* 出队序列: 按 (effective 优先级, 层内次序) 排序
          base,         \* [TASKS -> PRIO] 基础优先级
          prio,         \* [TASKS -> PRIO] 有效优先级 (PIP 可提升)
          cons1, cons2, \* [TASKS -> 0..MAXQ] 两维消费记账 (saturating)
          held,         \* 资源持锁者 (PIP 记账)
          pip_h, pip_r, \* 挂起的 PIP 对 (持锁者, 请求者); "none" = 无
          pip_violation \* 违例墓志 (单调, 只升不降)

vars == <<queue, base, prio, cons1, cons2, held, pip_h, pip_r, pip_violation>>

PRIO == 0 .. 4

Range(s) == {s[i] : i \in DOMAIN s}

TypeOK ==
    /\ queue \in Seq(TASKS)
    /\ Len(queue) <= Cardinality(TASKS)
    /\ ( \A i, j \in DOMAIN queue : i # j => queue[i] # queue[j] )
    /\ base  \in [TASKS -> PRIO]
    /\ prio  \in [TASKS -> PRIO]
    /\ cons1 \in [TASKS -> 0 .. MAXQ]
    /\ cons2 \in [TASKS -> 0 .. MAXQ]
    /\ held  \in TASKS \cup {"none"}
    /\ pip_h \in TASKS \cup {"none"}
    /\ pip_r \in TASKS \cup {"none"}
    /\ pip_violation \in BOOLEAN

Init ==
    /\ queue = <<>>
    /\ base  \in [TASKS -> PRIO]
    /\ prio  = base
    /\ cons1 = [t \in TASKS |-> 0]
    /\ cons2 = [t \in TASKS |-> 0]
    /\ held  = "none"
    /\ pip_h = "none"
    /\ pip_r = "none"
    /\ pip_violation = FALSE

\* ---------------------------------------------------------------------------
\* 队列重排原语 (对应 VecDeque push_back/push_front 语义)
\*   InsertAfterEquals: 新任务追加到同级尾 (FIFO: 先提交者靠前)
\*   InsertFrontOfLevel: PIP 提升插入到同级队首 (push_front)
\* 排序前置: queue 已按 prio 升序 (InvQueueSorted 归纳保持)。
\* ---------------------------------------------------------------------------
InsertAfterEquals(q, t, p) ==
    LET idx == CHOOSE i \in 1 .. (Len(q) + 1) :
                   ( \A j \in 1 .. (i - 1) : prio[q[j]] <= p )
                /\ ( \A j \in i .. Len(q) : prio[q[j]] > p )
    IN  SubSeq(q, 1, idx - 1) \o <<t>> \o SubSeq(q, idx, Len(q))

InsertFrontOfLevel(q, t, p) ==
    LET idx == CHOOSE i \in 1 .. (Len(q) + 1) :
                   ( \A j \in 1 .. (i - 1) : prio[q[j]] < p )
                /\ ( \A j \in i .. Len(q) : prio[q[j]] >= p )
    IN  SubSeq(q, 1, idx - 1) \o <<t>> \o SubSeq(q, idx, Len(q))

RemoveFrom(q, t) == SelectSeq(q, LAMBDA x : x # t)

\* ---------------------------------------------------------------------------
\* 配额记账 (Rust: CognitiveQuota::consume_step 的 saturating 累加)
\* ---------------------------------------------------------------------------
SaturatingAdd(x, d) == IF x + d > MAXQ THEN MAXQ ELSE x + d

ConsumeDim1(t, d) == [cons1 EXCEPT ![t] = SaturatingAdd(cons1[t], d)]
ConsumeDim2(t, d) == [cons2 EXCEPT ![t] = SaturatingAdd(cons2[t], d)]

Exhausted1(t) == cons1[t] >= MAXQ
Exhausted2(t) == cons2[t] >= MAXQ

\* ---------------------------------------------------------------------------
\* Next 关系 (Rust: submit_task / schedule_next / register_lock /
\*            boost_priority_for_lock / release_lock / complete_task / consume_step)
\* ---------------------------------------------------------------------------
Submit(t) ==
    /\ t \notin Range(queue)
    /\ Len(queue) < Cardinality(TASKS)
    /\ queue' = InsertAfterEquals(queue, t, prio[t])
    /\ UNCHANGED <<base, prio, cons1, cons2, held, pip_h, pip_r, pip_violation>>

\* 出队 (Rust: schedule_next 取最高紧急度层队首)。
\* PIP 违例检查: 若挂起 PIP 对 (h, r) 且此次出队了 r 而 h 仍在队 → 反转违例。
Dispatch ==
    /\ queue # <<>>
    /\ LET t == Head(queue)
       IN  /\ pip_violation' =
                pip_violation
                \/ ( pip_h # "none" /\ pip_r # "none" /\ t = pip_r
                     /\ pip_h \in Range(Tail(queue)) )
           /\ queue' = Tail(queue)
           /\ IF t = pip_h \/ t = pip_r
              THEN /\ pip_h' = "none"
                   /\ pip_r' = "none"
              ELSE /\ UNCHANGED <<pip_h, pip_r>>
           /\ UNCHANGED <<base, prio, cons1, cons2, held>>

RegisterLock(t) ==
    /\ held' = t
    /\ UNCHANGED <<queue, base, prio, cons1, cons2, pip_h, pip_r, pip_violation>>

\* PIP 触发 (Rust: boost_priority_for_lock, 仅当请求者更紧急时生效):
\* 持锁者 effective 提升到请求者级并插到该层队首。
RequestLock(h, r) ==
    /\ held = h
    /\ r \in Range(queue)
    /\ r # h
    /\ prio[r] < prio[h]
    /\ prio' = [prio EXCEPT ![h] = prio[r]]
    /\ queue' = InsertFrontOfLevel(RemoveFrom(queue, h), h, prio[r])
    /\ pip_h' = h
    /\ pip_r' = r
    /\ UNCHANGED <<base, cons1, cons2, held, pip_violation>>

\* 释放锁并复位 effective 优先级 (Rust: release_lock)。
ReleaseLock(h) ==
    /\ held = h
    /\ prio' = [prio EXCEPT ![h] = base[h]]
    /\ queue' = IF h \in Range(queue)
                THEN InsertAfterEquals(RemoveFrom(queue, h), h, base[h])
                ELSE queue
    /\ pip_h' = "none"
    /\ pip_r' = "none"
    /\ UNCHANGED <<base, cons1, cons2, held, pip_violation>>

CompleteTask(t) ==
    /\ queue' = RemoveFrom(queue, t)
    /\ IF t = pip_h \/ t = pip_r
       THEN /\ pip_h' = "none"
            /\ pip_r' = "none"
       ELSE /\ UNCHANGED <<pip_h, pip_r>>
    /\ UNCHANGED <<base, prio, cons1, cons2, held, pip_violation>>

Consume(t) ==
    /\ \E d \in 0 .. STEPS :
        /\ cons1' = ConsumeDim1(t, d)
        /\ cons2' = ConsumeDim2(t, d)
    /\ UNCHANGED <<queue, base, prio, held, pip_h, pip_r, pip_violation>>

Stutter == UNCHANGED vars

Next ==
    \/ \E t \in TASKS : Submit(t)
    \/ Dispatch
    \/ \E t \in TASKS : RegisterLock(t)
    \/ \E h \in TASKS, r \in TASKS : RequestLock(h, r)
    \/ \E h \in TASKS : ReleaseLock(h)
    \/ \E t \in TASKS : CompleteTask(t)
    \/ \E t \in TASKS : Consume(t)
    \/ Stutter

Spec == Init /\ [][Next]_vars

\* ---------------------------------------------------------------------------
\* 配额记账不变量
\* ---------------------------------------------------------------------------

\* P1: 各维度扣减后非负 (值域 0..MAXQ) 且单调不减 (只增不减, 不回绕)。
InvQuotaNeverNegative ==
    /\ \A t \in TASKS, d \in 0 .. STEPS :
        /\ ConsumeDim1(t, d)[t] >= cons1[t]
        /\ ConsumeDim2(t, d)[t] >= cons2[t]

\* P2: 耗尽状态粘滞 —— 已耗尽维度经任意扣减仍耗尽。
InvQuotaExhaustSticky ==
    /\ \A t \in TASKS, d \in 0 .. STEPS :
        /\ (Exhausted1(t) => ConsumeDim1(t, d)[t] >= MAXQ)
        /\ (Exhausted2(t) => ConsumeDim2(t, d)[t] >= MAXQ)

\* ---------------------------------------------------------------------------
\* 调度安全不变量
\* ---------------------------------------------------------------------------

\* P3: 队列按 effective 优先级升序 (出队 = 队首 = 最高紧急度)。
InvQueueSorted ==
    \A i, j \in DOMAIN queue : i < j => prio[queue[i]] <= prio[queue[j]]

\* P4: 出队者必为最高紧急度 —— 高优先级不被低优先级抢先 (有界安全形)。
InvHeadMostUrgent ==
    queue = <<>> \/ \A i \in DOMAIN queue : prio[queue[1]] <= prio[queue[i]]

\* P5: PIP 防反转 —— 提升后的持锁者不被同级请求者插队 (违例墓志恒假)。
InvPipNoInversion == ~pip_violation

\* P6: PIP 只降序 —— 任意任务 effective 优先级不低于 (序值不大于) 其 base。
InvBoostNeverUpgrades ==
    \A t \in TASKS : prio[t] <= base[t]

====
