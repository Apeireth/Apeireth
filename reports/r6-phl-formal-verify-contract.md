# R6-PHL-03｜FormalVerify 契约/选型

## 决策
`formal_verify`验证**修改路径满足规约**；不同于同型重生(PHL-01)与运行时修改边界(PHL-02)。本轮仅Protocol/dataclass，无prover/新依赖。

|方法|ASI哲学兼容|Python工程|曲线|本地借鉴|V3/V1074/V1081守门|
|---|---|---|---|---|---|
|TLA+|高：安全/活性/回滚|中：文件+CLI|中|R13 q4状态循环|高：反例/时序|
|Lean 4|高：显式公理|中低：桥接|高|无|高：proof artifact|
|Rocq/Coq|高：内核/抽取|低|高|无|高|
|Isabelle/HOL|高：成熟HOL|低|高|无|高|
|Dafny|中高：契约|中高|中|无|中高：可执行规约|

**推荐：TLA+→Lean 4。** 修改流程是 `snapshot→propose→gate→apply→verify→keep/revert` 状态机，TLA+先证门序、安全与最终回滚；CompilerIR稳定后Lean证明纯转换。**替代**：快速Python相邻原型选Dafny；Rocq用于抽取，Isabelle用于HOL团队。首要对象不是函数而是并发时序。

## 真借鉴/密度
1. `research-v7-round-13.json` q4：formal verification agent loops/runtime safety；R8–R38仅此直接主题，证实低密度缺口。
2. Tokio `tokio/src/io/read_buf.rs:116-123`：unsafe调用者必须维持 initialized/filled 不变量；借“显式责任边界”，非机器证明。
3. SQLx `README.md:68-73`：compile-time checked queries；借“失败前移”，非依赖类型证明。
4. AgentMemory `src/agent_memory/integrity.py`：HMAC+原子写+fail-closed；借证据完整性，**完整性≠规约正确**。
5. VCPToolBox `AdminPanel-Vue/src/features/tool-list/types.ts`：工具schema入口；借规约载体，**schema≠proof**。

## 契约与守门
`FormalVerifyProtocol`五方法：`spec()`规约描述；`prove(claim)`证明尝试；`verify(code)`规约检查；`counterexample()`最近反例；`invariants()`修改路径不变量。数据：`VerificationSpec(name,invariants,theorem_provers)`、`VerificationResult(success,proof_id,counterexample)`。

主17:58“三不”：spec为真≠proof为真；反例只证伪特定声明≠所有bug；prover依赖逻辑/公理/编码≠真理。`guard_formal_verify()`以三条注记作V3 evidence；V1074四个“不等于”保持PASS，未改V1074/V1081。

## 烟测/边界
|烟测|结果|
|---|---|
|Protocol存在/5方法|PASS/PASS|
|runtime Protocol/dataclass|PASS/PASS|
|无真实现|PASS|
|philosophy guard|PASS|
|apeireth命名空间|PASS|
|选型docstring|PASS|

`pytest tests/test_r6_formal_verify_contract.py -q`：**8 passed**。模块112行；测试70行。不接PHL-01/02，不写真证明。

## R7+
R7：TLA+最小Harness状态机与3不变量（apply前snapshot；四门有序；失败终达revert）+ TLC反例artifact。R8：Python adapter只传spec/result，不信任布尔值。R9：CompilerIR稳定后Lean 4证明round-trip；Dafny仅作团队/CI无法承载TLA+时替代。任何proof须记录prover/version/axioms/spec hash，交V3+V1074+V1081守门。
