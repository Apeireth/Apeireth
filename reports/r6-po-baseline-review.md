# R6-PO-01b 性能基线复核 (performance_optimizer)

> 复核 2026-07-28 | 不接 call_llm | 不动 V1074/V1082 源码

## 1. 复核数据 (3 次取中位数)

V1074: 16/13/57s → med 16s (.8843/.8855/.8853). R6-REQ 240.6s Δ−93% (越: REQ 4pytest+3V1074). Stop PID 6920/16812, 余未碰
V1082 wall: 6.45/2.62/2.49s → median 2.62s (lift=+0.0078). R6-REQ 3.75s, Δ−30% (R1 仍带争用).
V1073 内部 4 段: 10.33s. V02_base 5.80s (56%) / V1071_vcp 0.057s / V1071_cross_domain 0.045s / V1072 0.018s. V1074 ≈ V1073 + 写盘 ~6s.

## 2. 阶段 profile (真瓶颈, 按 Δ)

1) V1048.measure_phi_proxy 调 `pytest --collect-only` (秒级+争用放大); measure_real_production 重复 `git log` (V1048 + V1074.count_commits); measure_engineering 读 ~1090 v*.py 行. 2) VCP 深读: V1048.cross_domain+vcp_4 + V1073.measure_v1071_vcp+cross_domain 共调 `V1071VCPDeepRead.measure()` ≥4 次, 首次 walk+parse 65+ manifests, 后 3 次命中 module cache <0.1s. 3) V1082 `run_full_audit` 中 inventory_modules+audit_guards 各 read_text 一次 (2× I/O); lift O(1). 4) V1074.count_tests 独立 re.findall test_v*.py, 与 V1048.phi_proxy 重复. 5) V1074.write_all 写 6 artifact+history.jsonl, <100ms.

## 3. 优化 Top-5 (按 Δ, 不动 V1074/V1081/V1083 守门)

#1 V1071 一次深读→多维共享: V1073Integrator.run 预调 `V1071VCPDeepRead().measure()` 缓存 raw_vcp_4/raw_cross_domain, V1048 直读. Δ −2~−3s. 守门: V1071 输出/签名不变.
#2 phi_proxy/real_production 复用 V1074 已有: `n_tests`/`n_commits` 由 V1074.builder 注入 V1048 (`scores` 参数已支持), 跳 pytest collect + git log. Δ −1~−3s. 守门: V1074 count_* 已是真测.
#3 engineering/capabilities mtime 缓存: `(code_loc, mtime)` → `artifacts/.r6_po_v1048_loc_cache.json`, 未变不重读. Δ −0.5~−1.5s. 守门: LOC proxy 语义不变.
#4 V1082 inventory+guard 共读: `for p in mods: text=read_text(p) once`. Δ −0.5s. 守门: 真测值不变.
#5 V1074 增量 history.jsonl: write_all O_APPEND, 不每 run 重读 50 条. Δ −0.2s. 守门: 字段不变.

合计: V1074 16s → ~10s; V1082 2.62s → ~2.0s.

## 4. R7 sub-100s V1074 路线

R7 目标 V1074 <100s. R7a ≤3 天: #1+#2+#5 → median ~10s, P99 <30s. R7b ≤1 周: #3+#4 → ~8s, P99 <20s; 加 `time.time()` 埋点 + `--profile` CLI (cumulative, 不重 cProfile). R7c ≤2 周: V1073Integrator 4 段 `ThreadPoolExecutor(max_workers=4)` 并发 → ~3-4s. 守门: V1071/V1072 线程安全, 写盘主线程; ASI/All OK 输出口径不变. 不入 R7: Rust 重写 (V1082 ~2.6s, 投入产出低, 与 "3 不假装" 冲突).

## 5. 守门约束

`_score_is_infinity` V0.3=0.8853<1.0 不变; `_audit_is_fix` #1/#2 是缓存复用, V1082 identify-only 语义不变; `_shell_count_is_asi` 25 V1000+ 空壳仍真事实; `_loc_is_work` #3 cache 工程化; `philosophy_guard` 全部建议不改 V1074/V1081/V1083.

## V3 守门

_score_is_infinity 成立; _audit_is_fix 成立 (本报告只 identify); _loc_is_work 成立; _shell_count_is_asi 成立; philosophy_guard PASS.

_PO R6-PO-01b 收口._
