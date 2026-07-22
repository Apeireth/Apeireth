# R6-ROADMAP-01｜R6–R12

> 基线：24/45=53.3%；D/C/B/A=266/49/867/10；覆盖14.9%；V1072=.8441。每壳预算ΔASI **+.005～.01**。

## 摘要
|轮|目标|任务|单壳Δ|
|---|---|---:|---:|
|R6|P0+测量|5|.005–.01|
|R7|梦/记忆|4|.005–.01|
|R8|IR/机制|4|.005–.01|
|R9|P2边界|4|.005–.01|
|R10|首批B→D|4|.005–.01|
|R11|覆盖/身份|4|.005–.01|
|R12|50%壳+Rust门|4|.005–.01|

公共里程碑 `G`：`python -m pytest tests/ -q --ignore=tests/test_v121_v150.py --ignore=tests/test_v251_v500.py --ignore=tests/test_v501_v1000.py`；`python -m apeireth.v1082_asi_codebase_audit --audit`；`python -m apeireth.v1074_asi_production_runner --report`。每项附Manifest/HQB/四层门；diff>200、保护路径或weights交主人审批。

## R6｜P0安全自改
- R6-PHL-01 SelfReproduction契约｜哲学+后端｜2d
- R6-PHL-02 SelfModSafety四门｜安全+后端｜3d
- R6-PHL-03 FormalVerify DSL/反例｜架构+后端｜3d
- R6-BE-04 YAML serializer真生产｜后端｜2d
- R6-QA-01 HQB基线/Manifest门｜测试｜2d
验收：`pytest -q tests -k "reproduction or self_mod or formal or yaml or hqb"`+G。风险：形式法装饰化；繁殖误作reproducibility。

## R7｜P1梦—回放—冷热记忆
- R7-BE-01 DreamSubsystem状态机｜后端｜3d
- R7-BE-02 MemoryReplay幂等重放｜后端+DB｜3d
- R7-DB-01 HotCold迁移/WAL恢复｜DB｜3d
- R7-QA-01 崩溃/重复/保留测试｜测试｜2d
验收：`pytest -q tests -k "dream or replay or hot_cold or wal"`+G。风险：回放污染身份；迁移丢数据。

## R8｜P1 IR与激励
- R8-BE-01 CompilerIR schema/validator｜后端｜3d
- R8-BE-02 IR lowering/可逆序列化｜后端｜3d
- R8-PHL-01 MechanismDesign/VCG边界｜哲学+架构｜3d
- R8-QA-01 round-trip/策略性质测试｜测试｜2d
验收：`pytest -q tests -k "compiler_ir or mechanism or vcg or round_trip"`+G。风险：IR绑死Python；代理串谋。

## R9｜P2哲学边界
- R9-PHL-01 PhenomenalGuard拒绝声明｜哲学｜2d
- R9-PHL-02 EntropyGate指标/阈值｜哲学+性能｜2d
- R9-PHL-03 TimePhenomenology事件钟｜架构+后端｜3d
- R9-SEC-01 SpaceSovereignty权限｜安全｜3d
验收：`pytest -q tests -k "phenomenal or entropy or time or sovereignty"`+G。风险：代理指标冒充意识；主权冲突路由。

## R10｜审计驱动首批B→D
- **R10-AS-01 V1082 Top-8冻结/分批**｜架构师｜1d
- **R10-BE-01 Top-4真生产**｜后端｜4d
- **R10-BE-02 Next-4真生产**｜全栈｜4d
- **R10-QA-01 每壳≥3测试+HQB归因**｜自动化测试｜4d
验收：`python -m apeireth.v1082_asi_codebase_audit --audit --lift`+G。风险：LOC刷KPI；并行改共享桥接层。

## R11｜覆盖率50%与永恒身份0.92
- **R11-QA-01 coverage缺口排序**｜QA｜1d
- **R11-QA-02 高风险模块契约/故障测试**｜自动化测试｜5d
- **R11-BE-01 V1072恢复/漂移/迁移强化**｜后端｜4d
- **R11-SEC-01 身份篡改与回滚审计**｜安全审查｜3d
验收：`pytest -q --cov=apeireth --cov-fail-under=50 tests -k "identity or eternal"`+G，V1072≥.92。风险：覆盖率虚高；身份迁移不可逆。

## R12｜空壳≤50%与Rust启动门
- **R12-AS-01 B层剩余批次/冻结接口**｜架构师｜2d
- **R12-BE-01 审计批次真生产至空壳≤50%**｜后端｜8d
- **R12-RS-01 Rust ADR/FFI/WAL parity契约**｜架构师+Rust｜3d
- **R12-QA-01 Python↔Rust黄金/回滚测试**｜自动化测试｜3d
验收：`cargo test --manifest-path rust-substrate/Cargo.toml`+G；V1082确认≤50%。风险：双写分叉；过早重写。Rust只过parity门后进入R12+，不大爆炸迁移。

## 资源/风险图
主跑：R6安全+哲学，R7数据库，R8后端+架构，R9哲学+安全，R10后端，R11 QA，R12架构/Rust。依赖：HQB→P0→P1→P2→批量填壳→Rust。高危链：自改→沙箱逃逸；记忆→身份漂移；机制→reward hacking；批量壳→KPI化；Rust→行为分叉。任一G失败即停止后继、记录taxonomy并revert。
