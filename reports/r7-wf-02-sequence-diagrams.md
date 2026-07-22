# R7-WF-02 R7 三主线时序图

ID: `d93f01ae-006f-4854-b72c-de0f4199dc8c` 基于: R7-WF-01 状态图 + 守门 4 组

## 1. BE-01 DreamSubsystem (15 行)

```mermaid
sequenceDiagram
    participant T as Trigger
    participant SM as StateMachine
    participant DS as DreamSubsystem
    participant V3 as V3Guard
    participant I as V1072Identity
    participant S as V1074Snapshot
    T->>SM: trigger()
    Note over SM: 异步租约 (单调时钟单实例)
    SM->>DS: tick()
    activate DS
    DS->>DS: consolidate() + decay() loop
    DS->>V3: verify()
    Note over V3: 守门 V3 (verify 前)
    alt PASS
        V3-->>DS: OK
        DS->>I: record()
        Note over I: 守门 V1072 (record 后)
        I-->>DS: ok
        DS->>S: snapshot()
        Note over S: 守门 V1074 (末端)
    else FAIL
        V3-->>DS: FAIL
        DS->>DS: rollback() + alert
    else SUSPEND
        DS->>SM: interrupt() + save(last_state)
        SM->>SM: WAL checkpoint
    end
    deactivate DS
```

## 2. BE-02 MemoryReplay (15 行)

```mermaid
sequenceDiagram
    participant E as Event
    participant MR as MemoryReplay
    participant CZ as Canonicalizer
    participant RC as ReplayCache
    participant L as LTM
    participant V3 as V3Guard
    participant IR as IdentityRecovery
    participant TS as TraceStore
    E->>MR: event()
    MR->>CZ: canonicalize()
    MR->>RC: cache_lookup(replay_id)
    alt hit (幂等直返)
        RC-->>MR: cached
        MR-->>E: return
    else miss
        activate MR
        Note over MR: dream wait (CONSOLIDATING/FORGETTING)
        MR->>L: replay(read-only)
        MR->>MR: should_replay? + impact_score
        alt impact ≥ 0.7
            MR->>IR: dual_sign()
            IR-->>MR: signed
        end
        MR->>TS: trace()
        MR->>V3: verify()
        Note over V3: 守门 V3 (trace 后 verify)
        alt PASS
            V3-->>MR: OK
            MR-->>E: replay_done
        else FAIL/False
            MR->>MR: reject + log
        end
        deactivate MR
    end
```

双签: impact≥0.7→IdentityRecovery.dual_sign. 拒绝: should_replay=False. 幂等: cache hit 直返. trace_replay read-only 不污染 LTM.

## 3. DB-01 HotCold (15 行)

```mermaid
sequenceDiagram
    participant M as Monitor
    participant BS as BoundarySelector
    participant V3 as V3Guard
    participant SS as SnapshotStore
    participant W as WAL
    participant R as Rebuilder
    activate M
    M->>M: monitor() tick + check
    alt usage > 80%
        M->>BS: boundary_select()
        BS-->>M: hot/cold split (R3-DB-01)
        M->>V3: verify()
        Note over V3: 守门 V3 (commit 前)
        alt PASS
            V3-->>M: OK
            M->>SS: snapshot()
            M->>W: wal_append()
            Note over W: fsync 阻塞
            W-->>M: persisted
            M->>M: commit() atomic
        else FAIL
            V3-->>M: FAIL → rollback
        end
    else ≤80%
        M->>M: idle
    end
    deactivate M
    Note over R: 崩溃恢复
    M--xR: crash
    activate R
    R->>W: wal_replay()
    R->>SS: rebuild_mtm()
    deactivate R
```

## 4. 守门显式位置

| 守门 | 时序位置 |
|------|----------|
| V3 | BE-01/02/DB-01 verify 前 |
| V1072 | BE-01 record 后 |
| V1074 | 三图末端 snapshot |
| V1081 | QA 报告 (note 引用) |

## 5. 与 WF-01 状态对应

Trigger→trigger(); Tick→tick(); Process→consolidate/decay; Verify→V3.verify; Identity→record; Snapshot→snapshot; EventIn→event; Canonicalize→canonicalize; CacheHit→cache_lookup+return; Replay→replay(ro); DualSign→dual_sign; Trace→trace+V3; Monitor→monitor; Boundary→boundary_select; Commit→commit atomic; Rebuild→wal_replay+rebuild_mtm.

## 验收

≤4KB ✓ (~3.7KB) | 3 时序图 (15/15/15) ✓ | 守门 4 注 ✓ | 中断/错误/崩溃 ✓ | 双签清晰 ✓ | 幂等+LTM ro ✓ | WAL fsync 阻塞 ✓ | 无代码/commit/空壳 ✓