# R6-SR-01 安全审查报告

**审查者**: security_reviewer | **范围**: 4 模块只读
**基线**: 4 模块 0 命中 RCE 模式 (eval/exec/subprocess/shell/yaml.unsafe/full/Loader)

## 1. self_reproduction.py (R6-PHL-01) — LOW/MED

117 行契约壳。`ReproductionSpec`+5 方法 Protocol stub+guard。`seed:bytes`/`target_path:str`/`expected_modules:int>0` 经 `__post_init__` 校验。PHILOSOPHY_NOTES 列 not_clone/not_perfect/not_uuid 防假归因; `claimed_pass=None` 不强穿 V3 守门。**R7 MED**: `reproduce(target_path)` 需 `Path.resolve()`+白名单根; `seed` 不暴露原始字节。

## 2. self_mod_safety.py (R6-PHL-02) — LOW/MED

121 行契约壳。`Checkpoint/SafetyVerification/DryRunResult`+5 方法 Protocol。`risk_score∈[0,1]` 防数值注入;Checkpoint label/id/scope 非空校验;PHILOSOPHY_NOTES(not_undo/not_proof/not_safe)防假 rollback/verify/dry_run 归因。**R7 MED**: `rollback(checkpoint_id)` 缺属主校验, 易越权; `verify(code:bytes)` 缺 size 上限; `dry_run` 与真跑 diff 必须审计。

## 3. formal_verify.py (R6-PHL-03) — LOW/INFO

113 行契约壳, `CONTRACT_ONLY=True`。架构文档完整(TLA+主、Lean 4 次、Dafny 备、Coq/Isabelle 高级)。无 prover 集成、无子进程/网络。PHILOSOPHY_NOTES 三连(spec_is_not_proof/counterexample_is_not_bug/prover_is_not_truth)是最严谨哲学声明。**R7 INFO**: 真接 Lean/tlc 时子进程即沙箱逃逸通道, 需 cwd/timeout/无 shell 隔离。

## 4. v1000_yaml_serializer.py (R6-BE-04) — LOW-MED

296 行真实现。`_validate_mode` 白名单 SAFE/ROUND_TRIP;仅 `yaml.safe_load/safe_dump/safe_load_all/safe_dump_all`;ruamel `typ="rt"`(非 unsafe/base)。`_pre_dump` 走白名单类型, 自定义对象原样传 safe_dump 不执行代码。`deep_merge` 纯递归 dict。**MED**: `_read/_write` 任意 Path 跟随(受信假设);无 size 限制(DoS);`add_representer` 返回对象仍走 `_pre_dump` 递归。

## 风险分级

- **HIGH(现)**: 无
- **MED(现)**: yaml_serializer `_read/_write` 任意 Path;yaml_serializer 无 size 限制
- **MED(R7)**: self_reproduction target_path 缺路径校验;self_mod_safety rollback 缺属主;verify 无 size 上限
- **INFO**: 4 模块哲学守门完整, 无假归因;`__post_init__` 覆盖所有 dataclass

## R7 真实现安全建议

1. **P0 路径沙箱**: self_reproduction/self_mod_safety/yaml_serializer 三处 Path 操作必须 `Path.resolve(strict=True)`+白名单根+禁 symlink
2. **P0 子进程隔离**: formal_verify 接 Lean/tlc 必须 `subprocess.run(shell=False, timeout=30, cwd=sandbox)`, 禁 root 注入
3. **P1 size/超时**: 4 模块 bytes/str 输入统一加 size 上限, yaml 加解析超时
4. **P1 权限校验**: rollback/reproduce/verify 用 owner+role 而非 id 自验证
5. **P1 representer 收敛**: yaml 公共构造器关闭 `add_representer`
6. **P2 审计日志**: guard 调用写 `artifacts/r6_guard_log.jsonl` 与 V3 共审

**结论**: R6 可上线(壳层无 I/O, 无 RCE 路径)。R7 前必须先落地 P0 路径沙箱+子进程隔离, 否则沙箱逃逸升 HIGH。
