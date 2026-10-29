# A2 忽略测试全量审计与恢复

[Document-Meta]
Document: docs/04-internal/kernel-convergence-a2-ignored-tests-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: A2 完成；恢复安全离线合同，整体验收仍 Partial
Author: Codex

## 范围、口径和结论

从 clean `4de95fb9bdbc49453679d2f45ddf99dd39224eb6` 开始，仅在
`refactor/kc1-kernel-convergence` 施工。**恢复 7 项；Linux 原生仍忽略 44 项**。
恢复源码冻结于 `e92b22c60335afcc636b5303a8ff8bb870239f5a`，
审计全限定名称对照完善于 `0223004147bbaf5badc8d6e13f023062fef174a6`；文档另行提交。
没有删除或改名测试，没有新增 ignore，没有减负载或放宽断言。
[A1](kernel-convergence-a1-cold-start-2026-10-29.md) 的 Dev 冷启动缺口仍在，
[Final Acceptance v3.1](kernel-convergence-final-acceptance-2026-10-28.md) 继续 **Partial**；
本批不重跑 full gate_full，A4 才做完整门禁。

“51 ignored”是 S6 在 Linux 原生默认特性下实际打印的集合，不能直接当成源码全部声明数。
本批枚举 workspace 全部成员的 Rust 源码，起点共有 **53 个无条件 ignore 声明**，其中
**两个测试受 Windows cfg 限定**，不进入本机集合；另有 **9 个 cfg_attr(miri) 条件声明**，
在普通原生构建中没有忽略。恢复后无条件声明 **46 项 = 原生 44 + Windows 2**，
Miri 条件声明仍是 9 项。本文三张表分别完整列出这些范围，不将平台排除冒充恢复。

恢复判断要求原测试能在无服务密钥、无真实硬件的通用默认测试环境重复执行。
计时基准、大内存/穷举/长模糊负载、图片导出、主机钥匙库或外部服务验收按原合同保留专用任务，
不把“能在一台机器手跑”自动等同于适合常规并发测试。它们的退出条件逐项列在表中。
Owner 为模块维护归口角色；仓库未提供 CODEOWNERS 个人清单，不虚构已有人实名认领。
本次枚举、恢复与证据维护人为 Codex；剩余项目交由对应模块维护角色接续。

## 名称/路径集合对账与引入历史

基准源码为 S6 冻结的 `3f8048e8d8fca86eceba0e6e2eda89a16eed5ce1`，
实跑证据为 `/workspace/apeireth-codex/s6-gate-full.log`（原 `.exit` 保留）。
起点为 `4de95fb9`，不是从文案中的一个数量反推集合。
审计脚本把 S6 的每条 `test ... ignored` 映射到实际路径/函数，再与本轮的
`--ignored --list` 原生清单和源码声明逐项相等校验。

| 对比 | 新增忽略身份 | 移除忽略身份 | 改名/迁移 |
| --- | --- | --- | --- |
| S6 → A2 起点 | 无 | 无 | 无；名称和路径逐项一致 |
| A2 起点 → 恢复后 | 无 | 下列七项重新启用 | 无；原测试名与路径保留 |

- `crates/engine/organ/tests/causal_world_model.rs::real_llm_smoke`
- `crates/engine/organ/tests/curiosity.rs::curiosity_organ_real_llm_smoke`
- `crates/engine/organ/tests/emotion_memory.rs::emotion_organ_trait_shape_with_noop_llm_factory`
- `crates/engine/organ/tests/hypothesis.rs::hypothesis_organ_real_llm_smoke`
- `crates/engine/organ/tests/memory.rs::memory_organ_trait_shape_with_noop_llm_factory`
- `crates/engine/organ/tests/value_cases.rs::value_cases_organ_real_llm_smoke`
- `crates/engine/organ/tests/world_model.rs::world_model_organ_real_llm_smoke`

每项“引入”来自起点 ignore 属性行的 `git log -L` 最早引入变更；不是将函数诞生日期
或后来理由修改日期直接当成首次忽略日期。源码中的属性、邻近注释、完整 SHA/时间和
行追踪链保存在 `/tmp/apeireth-a2/after-audit.json`，结果另有外部证据副本。
图像家族表中的导入提交表示首次进入本仓库，导入前的原项目历史未推测。
仓库历史为非浅克隆；没有为本批改写历史或修改 main。
多数条目没有可解析的 issue 链接，以实际属性、注释与 Git 记录作为理由证据；
注释中的问题编号只作原样线索，不伪造当前仓库 issue URL。

S6 日志与本批原生清单的 SHA256 同样写入 JSON，可用以下命令复核（2026-10-29，
Linux x86_64，rustc/cargo 1.97.1；完整环境见 `a2-environment.log`）：

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --workspace --tests --locked -j 2 -- --ignored --list
python scripts/audit_ignored_tests.py --baseline-ref 3f8048e8d8fca86eceba0e6e2eda89a16eed5ce1 --before-ref 4de95fb9bdbc49453679d2f45ddf99dd39224eb6 --baseline-log /workspace/apeireth-codex/s6-gate-full.log --before-list /workspace/apeireth-codex/a2-native-list-before.log --current-list /workspace/apeireth-codex/a2-native-list-after.log --history --output /tmp/apeireth-a2/after-audit.json
```

前后清单分别在编辑前和编辑后落盘；`--list` 只枚举，不执行被忽略的真实服务或硬件测试，
也不是 full gate_full。除源码路径/函数身份外，脚本还单列实际全限定原生名称的新增/移除，
同一文件内模块名变化不会被函数末段或相同数量掩盖。模块改名 fixture 已验证这一点，
见 `a2-audit-name-drift-fixture.log`；该 fixture 不混入实际集合。
脚本支持本仓库当前单行 ignore/Miri 条件语法，原生映射按本批 Linux 口径；
若观察到无法唯一映射的名称、重复身份或源码/可执行集合不一致，会报错，不静默漏行。

## 原生集合完整审计表（S6 的全部 51 项）

本表保留恢复前的全量身份，所以含七个现已启用的条目；“本批判断”明确区分现状。
属性原文与上下文在 JSON，表内按实际依赖使用中性摘要。原函数名含历史 live/smoke 字样时
仍保留该名字；七个恢复项的通过只证明原离线合同，不证明真实模型服务已实测。

| ID | 测试名与路径 | 忽略引入（本仓库） | 原因（属性/注释及实际依赖） | 保留忽略的风险 | 本批判断 | Owner / 退出条件 |
| --- | --- | --- | --- | --- | --- | --- |
| N01 | `discovers_rea_tools`<br>[crates/capabilities/tools/tests/rea_bridge_probe.rs](../../crates/capabilities/tools/tests/rea_bridge_probe.rs) | `6fad7948ce07`<br>2026-10-22 | 属性要求本机安装外部工具服务；代码实际建立 MCP 连接 | 真实服务工具目录、命名空间和显式 grant 对接变化无默认实测 | 保留；当前不适合无条件默认运行 | **tools/MCP 维护**；保留集成任务；安装固定兼容版本并配置 REA_MCP_CMD，真实发现和原授权断言通过 |
| N02 | `shell_reach_reads_outside_workspace`<br>[crates/capabilities/tools/tests/shell_reach_probe.rs](../../crates/capabilities/tools/tests/shell_reach_probe.rs) | `c0323db64675`<br>2026-09-23 | 属性 live reach probe；依赖主机系统文件和可执行 shell | 实际主机可达范围缺少复验；系统文件本身的放行不能证明完整隔离状态 | 保留；当前不适合无条件默认运行 | **tools/process 维护**；保留主机验收；在支持平台运行原系统文件探针，并与用户空间隔离对偶证据一起审阅 |
| N03 | `shell_sandbox_probe_walls_user_space`<br>[crates/capabilities/tools/tests/shell_reach_probe.rs](../../crates/capabilities/tools/tests/shell_reach_probe.rs) | `6dba80bffceb`<br>2026-09-23 | 属性 live sandbox probe；函数实际硬编码系统命令与隔离能力 | 实际用户空间边界或工作区内权限回归未在 Linux 默认验证；当前代码要求 Windows 环境 | 保留；当前不适合无条件默认运行 | **tools/process 维护**；保留 Windows 隔离任务；系统具备所需隔离能力，原界外拒绝与界内成功断言同时通过 |
| N04 | `artistic_fx::tests::bench_gallery`<br>[crates/engine/image/algo/src/artistic_fx.rs](../../crates/engine/image/algo/src/artistic_fx.rs) | `a034a4af70b8`<br>2026-10-18 | 裸 ignore；注释要求大图 Release 计时 | 滤镜组合的大图吞吐没有常规趋势证据 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N05 | `artistic_fx::tests::gallery_sheet`<br>[crates/engine/image/algo/src/artistic_fx.rs](../../crates/engine/image/algo/src/artistic_fx.rs) | `a034a4af70b8`<br>2026-10-18 | 裸 ignore；需 GALLERY_RAW/W/H 并写图 | 视觉回归依赖人工输入与查看；无输入时直接返回不构成验证 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；固定图像、输出目录和视觉验收人，按原效果逐项审阅；不能把无输入空跑算恢复 |
| N06 | `blur::tests::surface_timing`<br>[crates/engine/image/algo/src/blur.rs](../../crates/engine/image/algo/src/blur.rs) | `a034a4af70b8`<br>2026-10-18 | 裸 ignore；surface 计时负载 | Surface 模糊路径吞吐退化不能由本项默认发现 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N07 | `blur::motion_bench::motion_upstream_performance`<br>[crates/engine/image/algo/src/blur/motion_bench.rs](../../crates/engine/image/algo/src/blur/motion_bench.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求内核与完整分派 Release 校准 | 运动模糊内核比较及分派成本可能漂移 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N08 | `inpaint::proximity_integral::tests::proximity_integral_release_comparison`<br>[crates/engine/image/algo/src/inpaint/proximity_integral/tests.rs](../../crates/engine/image/algo/src/inpaint/proximity_integral/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求大图 Release 对比 | 邻近积分填补的大图性能与候选差异缺少默认对照 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N09 | `matting::morphology::tests::octagonal_morphology_release_comparison`<br>[crates/engine/image/algo/src/matting/morphology/tests.rs](../../crates/engine/image/algo/src/matting/morphology/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求图块和大掩码 Release 对比 | 形态学大掩码吞吐或等价表现可能漂移 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N10 | `matting::morphology::tests::tiled_refinement_release_comparison`<br>[crates/engine/image/algo/src/matting/morphology/tests.rs](../../crates/engine/image/algo/src/matting/morphology/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求完整分块细化 CPU 路径 Release 对比 | 完整分块细化链的代价变化未自动量化 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N11 | `matting::morphology::tests::dense_matte_release_comparison`<br>[crates/engine/image/algo/src/matting/morphology/tests.rs](../../crates/engine/image/algo/src/matting/morphology/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求密集分数掩码 Release 对比 | 密集软掩码场景的性能变化不由本项默认检测 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N12 | `quantize::tests::indexed_diffusion_24mp_release_comparison`<br>[crates/engine/image/algo/src/quantize.rs](../../crates/engine/image/algo/src/quantize.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求大图抖动 Release 计时 | 索引抖动大图吞吐和路径差异缺少默认趋势 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N13 | `segment::maxflow::push_relabel::tests::solver_workload_release_comparison`<br>[crates/engine/image/algo/src/segment/maxflow/push_relabel/tests.rs](../../crates/engine/image/algo/src/segment/maxflow/push_relabel/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求稀疏/密集终端图 Release 对比 | 图割求解器在极端图上的工作量可能退化 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N14 | `segment::tests::dump_quick_scenes`<br>[crates/engine/image/algo/src/segment/tests.rs](../../crates/engine/image/algo/src/segment/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 裸 ignore；需 QS_DUMP 并导出场景 | 仅生成诊断素材；无输出目录时空跑，有目录时产生文件 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；隔离输出目录后导出并由视觉任务消费，明确自动比较合同后再评估默认化 |
| N15 | `segment::tests::select_subject_timing`<br>[crates/engine/image/algo/src/segment/tests.rs](../../crates/engine/image/algo/src/segment/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性 timing only；整幅主体选择计时 | 大图主体选择延迟变化没有默认时间证据 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N16 | `segment::tests::select_subject_dump_pictures`<br>[crates/engine/image/algo/src/segment/tests.rs](../../crates/engine/image/algo/src/segment/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性 writes pictures；需 SUBJECT_DUMP | 主体选择图片导出没有自动视觉判据，缺配置会直接返回 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；提供隔离输出目录和视觉对照判据，保存并检查原场景与选区图 |
| N17 | `segment::workflow_tests::quick_selection_workflow_release_comparison`<br>[crates/engine/image/algo/src/segment/workflow_tests.rs](../../crates/engine/image/algo/src/segment/workflow_tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求真实选择笔划大图 Release 对比 | 完整笔划链的质量/性能对照只在手动任务执行 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N18 | `segment::workflow_tests::quick_selection_surface_release_comparison`<br>[crates/engine/image/algo/src/segment/workflow_tests.rs](../../crates/engine/image/algo/src/segment/workflow_tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求经过栅格 Surface 的大图 Release 对比 | 编辑栅格入口的选择链性能可能偏离内核 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N19 | `selection::distance::tests::distance_transform_release_comparison`<br>[crates/engine/image/algo/src/selection/distance/tests.rs](../../crates/engine/image/algo/src/selection/distance/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求大掩码 Release 对比 | 距离变换在大掩码上的成本可能回退 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N20 | `selection_blur::tests::bench_24mp`<br>[crates/engine/image/algo/src/selection_blur.rs](../../crates/engine/image/algo/src/selection_blur.rs) | `a034a4af70b8`<br>2026-10-18 | 裸 ignore；注释为大图选择模糊基准 | 选择模糊的大图吞吐没有默认基准信号 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N21 | `blur2::shape_tests::direct_36mp_image_with_small_output_is_not_rejected`<br>[crates/engine/image/algo/src/shape_blur_tests.rs](../../crates/engine/image/algo/src/shape_blur_tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性注明大图直接内核内存回归，约 1.7 GiB 为源码估计 | 大图小输出被错误拒绝或高内存路径退化，不受本项默认保护 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留专用资源测试；独占大内存任务执行原断言并实测峰值内存，建立资源调度预算后才适合默认化 |
| N22 | `sharpen::fft_bench::unsharp_fft_performance`<br>[crates/engine/image/algo/src/sharpen/fft_bench.rs](../../crates/engine/image/algo/src/sharpen/fft_bench.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求自适应大半径完整操作单跑 | 大半径锐化的质量/性能取舍可能漂移 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N23 | `sharpen::fft_bench::unsharp_fft_cancel_latency`<br>[crates/engine/image/algo/src/sharpen/fft_bench.rs](../../crates/engine/image/algo/src/sharpen/fft_bench.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求宽半径取消延迟在 Release 单跑 | 大图取消响应退化不能由本项默认发现 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N24 | `sharpen::tests::unsharp_performance`<br>[crates/engine/image/algo/src/sharpen/tests.rs](../../crates/engine/image/algo/src/sharpen/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求完整操作 baseline/candidate Release 对比 | 完整锐化链的基线/候选差异缺少定期证据 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N25 | `sharpen::tests::unsharp_baseline_profile`<br>[crates/engine/image/algo/src/sharpen/tests.rs](../../crates/engine/image/algo/src/sharpen/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求不变滤镜阶段 Release 单跑剖析 | 锐化内部阶段的成本分布没有默认记录 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N26 | `stylize2::tests::swept_blocks_exhaustive_matches_the_pixel_search`<br>[crates/engine/image/algo/src/stylize2/tests.rs](../../crates/engine/image/algo/src/stylize2/tests.rs) | `e14a812589ca`<br>2026-10-23 | 属性与注释：组合穷举超出常规单测时限；已有默认抽样档 | 未穷举组合仍可能藏有等价性问题；当前只有抽样档默认覆盖 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留穷举档；专用长时任务完成原组合集并满足超时预算，再评估常规门禁调度 |
| N27 | `gallery::tests::iris_profile_baseline_stages`<br>[crates/engine/image/algo/src/tests_iris.rs](../../crates/engine/image/algo/src/tests_iris.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求独立 Release 剖析 | 光圈效果阶段成本可能发生未量化的迁移 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N28 | `tests_mosaic::mosaic_24mp_release_comparison`<br>[crates/engine/image/algo/src/tests_mosaic.rs](../../crates/engine/image/algo/src/tests_mosaic.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求大图 Release 性能对比 | 马赛克大图吞吐或候选路径对照缺少默认证据 | 保留；当前不适合无条件默认运行 | **image-algo 维护**；保留手动；在独占 Release 性能任务按原负载运行，记录环境、正确性与耗时；确认默认资源预算后再评估解除忽略 |
| N29 | `bench_srgb8_to_cmyk8`<br>[crates/engine/image/cms/tests/bench.rs](../../crates/engine/image/cms/tests/bench.rs) | `a034a4af70b8`<br>2026-10-18 | 裸 ignore；注释要求 Release 色彩转换吞吐测量 | 大画布色彩转换构建/吞吐没有本项默认量化；本项本身不作语义断言 | 保留；当前不适合无条件默认运行 | **image-cms 维护**；保留性能任务；按原整缓冲与分块负载记录转换性能，确认独占资源预算 |
| N30 | `rotating_24_mp_is_fast`<br>[crates/engine/image/codecs/tests/orientation.rs](../../crates/engine/image/codecs/tests/orientation.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求 Release；注释 #285，保留原耗时断言 | 大图方向旋转性能预算可能回退；共享 Debug 负载会造成不稳定失败 | 保留；当前不适合无条件默认运行 | **image-codecs 维护**；保留性能验收；独占 Release runner 按原方向与原时间阈值通过，不放宽阈值 |
| N31 | `decode_30mp`<br>[crates/engine/image/codecs/tests/tiff_bench.rs](../../crates/engine/image/codecs/tests/tiff_bench.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求 Release benchmark；构造多布局大图 | 大图解码与压缩/位深/分页布局的吞吐缺少默认趋势 | 保留；当前不适合无条件默认运行 | **image-codecs 维护**；保留独占 Release benchmark；按原全部布局运行，保存正确性和吞吐对照，确认内存预算 |
| N32 | `zoo_long_random_mutations`<br>[crates/engine/image/codecs/tests/tiff_pages.rs](../../crates/engine/image/codecs/tests/tiff_pages.rs) | `a034a4af70b8`<br>2026-10-18 | 属性 long fuzz run；默认长随机变异档，可用 TIFF_FUZZ_ITERS 配置 | 额外变异序列中的解析 panic/资源问题不由长档默认筛查；已有短畸形输入档 | 保留；当前不适合无条件默认运行 | **image-codecs 维护**；保留长模糊任务；按原默认长序列执行并确认最坏解码资源和超时预算，不能减迭代数换默认通过 |
| N33 | `masks::tests::benchmark_50mp_mask_blur`<br>[crates/engine/image/compose/src/masks.rs](../../crates/engine/image/compose/src/masks.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求超大掩码串并行手测 | 掩码模糊大尺寸串并行性能变化缺少本项默认对照 | 保留；当前不适合无条件默认运行 | **image-compose 维护**；保留独占性能任务；原负载串并行运行、比对输出并记录内存与耗时 |
| N34 | `tests::modern_lut_timing`<br>[crates/engine/image/compose/src/tests.rs](../../crates/engine/image/compose/src/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求 Release timing；重复构建 LUT 的纯计时工具 | LUT 构建耗时不被本项默认量化；无语义断言不能当功能覆盖 | 保留；当前不适合无条件默认运行 | **image-compose 维护**；保留 Release 剖析工具；记录原重复工作量的耗时；自动性能预算需单独定义和稳定环境 |
| N35 | `tests::cmyk_ink_render_timing`<br>[crates/engine/image/compose/src/tests.rs](../../crates/engine/image/compose/src/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 属性要求大图 ink/RGB 合成路径 Release 计时 | 墨色与通用合成路径的大图成本未被本项默认量化 | 保留；当前不适合无条件默认运行 | **image-compose 维护**；保留性能任务；原文档与调整层负载在隔离资源下测量、记录两条路径 |
| N36 | `tests::perf_6016`<br>[crates/engine/image/vector/src/tests.rs](../../crates/engine/image/vector/src/tests.rs) | `a034a4af70b8`<br>2026-10-18 | 裸 ignore；注释要求大画布复杂路径 Release 测量 | 复杂填充/描边的大画布性能和资源需求缺少定期记录 | 保留；当前不适合无条件默认运行 | **image-vector 维护**；保留性能任务；按原复杂路径和原画布单跑，记录填充/描边性能及内存预算 |
| N37 | `backend::sqlite::tests::performance_1000_episodes_under_1s`<br>[crates/engine/memory/src/backend/sqlite.rs](../../crates/engine/memory/src/backend/sqlite.rs) | `1c79a7245938`<br>2026-09-10 | 属性明确 wall-clock benchmark；保留原写入耗时断言 | 批量写入性能回归只由专用验收发现，普通共享任务的墙钟波动会造成误报 | 保留；当前不适合无条件默认运行 | **memory 维护**；保留独占性能验收；原写入数量、原断言在规定环境下通过并持续记录，不修改时间阈值 |
| N38 | `real_llm_smoke`<br>[crates/engine/organ/tests/causal_world_model.rs](../../crates/engine/organ/tests/causal_world_model.rs) | `01e3aa36331c`<br>2026-08-28 | 原属性称需密钥/端点，实际注入 Noop；检查 W2 返回形状与原错误分支 | 不恢复会持续遗漏离线工厂/输出合同的默认回归；原标签还会误报 live 覆盖 | 可恢复，已恢复并实跑 | **organ 维护**；已解除忽略；普通定向测试执行原断言通过。保留历史函数名；真实调用仍由独立 live 测试承担 |
| N39 | `curiosity_organ_real_llm_smoke`<br>[crates/engine/organ/tests/curiosity.rs](../../crates/engine/organ/tests/curiosity.rs) | `e3e96b5edeeb`<br>2026-08-28 | 原属性称需密钥/端点，实际 Noop + 确定性 Curiosity process | 不恢复会持续遗漏离线工厂/输出合同的默认回归；原标签还会误报 live 覆盖 | 可恢复，已恢复并实跑 | **organ 维护**；已解除忽略；普通定向测试执行原断言通过。保留历史函数名；真实调用仍由独立 live 测试承担 |
| N40 | `emotion_organ_trait_shape_with_noop_llm_factory`<br>[crates/engine/organ/tests/emotion_memory.rs](../../crates/engine/organ/tests/emotion_memory.rs) | `1d6ec260863a`<br>2026-08-28 | 原属性仅称手动 trait shape；实际 Noop 类型合同与确定性情绪 process | 不恢复会持续遗漏离线工厂/输出合同的默认回归；原标签还会误报 live 覆盖 | 可恢复，已恢复并实跑 | **organ 维护**；已解除忽略；普通定向测试执行原断言通过。保留历史函数名；真实调用仍由独立 live 测试承担 |
| N41 | `hypothesis_organ_real_llm_smoke`<br>[crates/engine/organ/tests/hypothesis.rs](../../crates/engine/organ/tests/hypothesis.rs) | `d94ff693dbb6`<br>2026-08-28 | 原属性称需密钥，实际 Noop + 确定性 Hypothesis process | 不恢复会持续遗漏离线工厂/输出合同的默认回归；原标签还会误报 live 覆盖 | 可恢复，已恢复并实跑 | **organ 维护**；已解除忽略；普通定向测试执行原断言通过。保留历史函数名；真实调用仍由独立 live 测试承担 |
| N42 | `memory_organ_trait_shape_with_noop_llm_factory`<br>[crates/engine/organ/tests/memory.rs](../../crates/engine/organ/tests/memory.rs) | `01e3aa36331c`<br>2026-08-28 | 原属性仅称手动 trait shape；实际确定性合并与无工厂合同 | 不恢复会持续遗漏离线工厂/输出合同的默认回归；原标签还会误报 live 覆盖 | 可恢复，已恢复并实跑 | **organ 维护**；已解除忽略；普通定向测试执行原断言通过。保留历史函数名；真实调用仍由独立 live 测试承担 |
| N43 | `w1_world_model_simulate_live`<br>[crates/engine/organ/tests/organ_live_llm.rs](../../crates/engine/organ/tests/organ_live_llm.rs) | `d956b9c495cc`<br>2026-09-09 | 属性明确密钥/模型配置；实际工厂调用外部模型 | 器官真实服务推演与 provider 适配回归可能只在部署时暴露 | 保留；当前不适合无条件默认运行 | **organ/provider 维护**；保留 live 任务；受控测试凭据、端点和固定模型映射就绪，原真实内容与推演断言通过并留证据 |
| N44 | `w2_causal_simulate_live`<br>[crates/engine/organ/tests/organ_live_llm.rs](../../crates/engine/organ/tests/organ_live_llm.rs) | `d956b9c495cc`<br>2026-09-09 | 属性明确密钥/模型配置；实际工厂调用外部模型 | 器官真实服务推演与 provider 适配回归可能只在部署时暴露 | 保留；当前不适合无条件默认运行 | **organ/provider 维护**；保留 live 任务；受控测试凭据、端点和固定模型映射就绪，原真实内容与推演断言通过并留证据 |
| N45 | `value_cases_organ_real_llm_smoke`<br>[crates/engine/organ/tests/value_cases.rs](../../crates/engine/organ/tests/value_cases.rs) | `1d6ec260863a`<br>2026-08-28 | 原属性称需密钥，实际 Noop + 确定性 Value process | 不恢复会持续遗漏离线工厂/输出合同的默认回归；原标签还会误报 live 覆盖 | 可恢复，已恢复并实跑 | **organ 维护**；已解除忽略；普通定向测试执行原断言通过。保留历史函数名；真实调用仍由独立 live 测试承担 |
| N46 | `world_model_organ_real_llm_smoke`<br>[crates/engine/organ/tests/world_model.rs](../../crates/engine/organ/tests/world_model.rs) | `01e3aa36331c`<br>2026-08-28 | 原属性称需密钥/端点，实际只断言标识与 Noop 工厂存在性 | 不恢复会持续遗漏离线工厂/输出合同的默认回归；原标签还会误报 live 覆盖 | 可恢复，已恢复并实跑 | **organ 维护**；已解除忽略；普通定向测试执行原断言通过。保留历史函数名；真实调用仍由独立 live 测试承担 |
| N47 | `real_llm_call_smoke`<br>[crates/engine/provider/tests/minimax_llm_factory.rs](../../crates/engine/provider/tests/minimax_llm_factory.rs) | `123d2d41cc27`<br>2026-08-28 | 属性要求服务密钥；实际发送模型请求 | 真实 provider 鉴权、协议适配与内容返回缺少默认联网验证 | 保留；当前不适合无条件默认运行 | **provider 维护**；保留 live 任务；按源码指定的受控凭据/端点/模型配置运行原调用断言，不能以 mock 或空返回替代 |
| N48 | `deepseek_live_e2e_smoke`<br>[crates/engine/provider/tests/openai_compatible_live.rs](../../crates/engine/provider/tests/openai_compatible_live.rs) | `bddd7fe2aeea`<br>2026-09-09 | 属性要求服务密钥；实际发送模型请求 | 真实 provider 鉴权、协议适配与内容返回缺少默认联网验证 | 保留；当前不适合无条件默认运行 | **provider 维护**；保留 live 任务；按源码指定的受控凭据/端点/模型配置运行原调用断言，不能以 mock 或空返回替代 |
| N49 | `deepseek_live_factory_smoke`<br>[crates/engine/provider/tests/openai_compatible_live.rs](../../crates/engine/provider/tests/openai_compatible_live.rs) | `bddd7fe2aeea`<br>2026-09-09 | 属性要求服务密钥；实际发送模型请求 | 真实 provider 鉴权、协议适配与内容返回缺少默认联网验证 | 保留；当前不适合无条件默认运行 | **provider 维护**；保留 live 任务；按源码指定的受控凭据/端点/模型配置运行原调用断言，不能以 mock 或空返回替代 |
| N50 | `service_token::tests::service_token_face_live_probe`<br>[crates/foundation/credentials/src/service_token.rs](../../crates/foundation/credentials/src/service_token.rs) | `34a425c80a06`<br>2026-10-11 | 裸 ignore；注释为 OS 钥匙库只读探针，EXPECTED 才作强校验 | 跨工具凭据目标不一致可能漏检；空条目打印成功不证明真实凭据读取 | 保留；当前不适合无条件默认运行 | **credentials 维护**；保留专用主机探针；受控钥匙库中准备测试条目并设置 SERVICE_TOKEN_PROBE_EXPECTED，原逐字节断言通过 |
| N51 | `council_7_advisor_live_decide`<br>[crates/foundation/orchestration/tests/council_live.rs](../../crates/foundation/orchestration/tests/council_live.rs) | `d6bde1e291f1`<br>2026-09-09 | 属性要求服务密钥/模型配置；先真实探针再多评审 | 真实通道、桥接或多评审故障缺少默认 live 信号；代码防止降级假绿 | 保留；当前不适合无条件默认运行 | **orchestration/provider 维护**；保留 live 任务；原真实内容探针与评审路径都通过，失败不能由降级 verdict 掩盖 |

## 非本机编译的 Windows 声明（2 项）

这两项在 S6 Linux 清单中不存在；仍保留源码和原 ignore，未虚称完成真机验证。

| ID | 测试名与路径 | 忽略引入（本仓库） | 原因（属性/注释及实际依赖） | 保留忽略的风险 | 本批判断 | Owner / 退出条件 |
| --- | --- | --- | --- | --- | --- | --- |
| W01 | `real_machine_sapi_pronunciation`<br>[crates/capabilities/tools/tests/voice_speak.rs](../../crates/capabilities/tools/tests/voice_speak.rs) | `cb20ddc10e2c`<br>2026-10-17 | Windows cfg + ignore；需系统语音引擎和播放设备 | 真实系统合成、播放完成与语音账本未由 Linux 门禁验证 | 保留 Windows 真机手动验收 | **tools/voice 维护**；保留 Windows 真机验收；引擎/嗓音/播放设备就绪，原 spoken、时长与账本断言通过并确认可听播放 |
| W02 | `real_machine_interrupt_stops_the_previous_utterance`<br>[crates/capabilities/tools/tests/voice_speak.rs](../../crates/capabilities/tools/tests/voice_speak.rs) | `cb20ddc10e2c`<br>2026-10-17 | Windows cfg + ignore；需真实语音播放与打断 | 真实播放重叠、打断状态或账本遗漏没有本平台验证 | 保留 Windows 真机手动验收 | **tools/voice 维护**；保留 Windows 真机验收；原新话完成、旧话中断与双记录断言通过，确认无僵尸播放 |

## Miri 条件声明（9 项）

这些测试在本机普通原生口径本来就不忽略，未记入 51 或 44；本批未运行 Miri，
不将解释器不支持的宿主 I/O/进程能力写成已覆盖。

| ID | 测试名与路径 | 忽略引入（本仓库） | 原因（属性/注释及实际依赖） | 保留忽略的风险 | 本批判断 | Owner / 退出条件 |
| --- | --- | --- | --- | --- | --- | --- |
| M01 | `layered_configuration_overlays_per_model_and_env_wins`<br>[crates/foundation/core/src/model_profile.rs](../../crates/foundation/core/src/model_profile.rs) | `05369bdf3062`<br>2026-10-15 | cfg_attr(miri)；解释器不支持目录 fsync | 该实际文件持久化路径不被解释器覆盖；原生测试仍启用 | 保留 Miri 条件；本机原生默认启用 | **core/storage 维护**；保留条件忽略；解释器支持对应宿主操作或有等价受控 I/O 接缝后，再在 Miri 跑原断言确认 |
| M02 | `durable_tier_syncs_before_replace_atomic_tier_does_not`<br>[crates/foundation/core/src/storage_atomic.rs](../../crates/foundation/core/src/storage_atomic.rs) | `df1b98421348`<br>2026-10-14 | cfg_attr(miri)；解释器不支持目录 fsync | 该实际文件持久化路径不被解释器覆盖；原生测试仍启用 | 保留 Miri 条件；本机原生默认启用 | **core/storage 维护**；保留条件忽略；解释器支持对应宿主操作或有等价受控 I/O 接缝后，再在 Miri 跑原断言确认 |
| M03 | `durable_tier_applies_exact_unix_mode`<br>[crates/foundation/core/src/storage_atomic.rs](../../crates/foundation/core/src/storage_atomic.rs) | `df1b98421348`<br>2026-10-14 | cfg_attr(miri)；解释器不支持目录 fsync | 该实际文件持久化路径不被解释器覆盖；原生测试仍启用 | 保留 Miri 条件；本机原生默认启用 | **core/storage 维护**；保留条件忽略；解释器支持对应宿主操作或有等价受控 I/O 接缝后，再在 Miri 跑原断言确认 |
| M04 | `file_lock_excludes_concurrent_holders`<br>[crates/foundation/core/src/storage_atomic.rs](../../crates/foundation/core/src/storage_atomic.rs) | `09c2fcf36ca8`<br>2026-10-15 | cfg_attr(miri)；解释器不支持进程存活探测 | 该真实锁/进程路径不被解释器覆盖；原生测试仍启用 | 保留 Miri 条件；本机原生默认启用 | **core/storage 维护**；保留条件忽略；解释器支持对应宿主操作或有等价受控 I/O 接缝后，再在 Miri 跑原断言确认 |
| M05 | `file_lock_takes_over_marker_of_dead_holder`<br>[crates/foundation/core/src/storage_atomic.rs](../../crates/foundation/core/src/storage_atomic.rs) | `09c2fcf36ca8`<br>2026-10-15 | cfg_attr(miri)；解释器不支持进程存活探测 | 该真实锁/进程路径不被解释器覆盖；原生测试仍启用 | 保留 Miri 条件；本机原生默认启用 | **core/storage 维护**；保留条件忽略；解释器支持对应宿主操作或有等价受控 I/O 接缝后，再在 Miri 跑原断言确认 |
| M06 | `file_lock_takes_over_corrupt_marker_record`<br>[crates/foundation/core/src/storage_atomic.rs](../../crates/foundation/core/src/storage_atomic.rs) | `09c2fcf36ca8`<br>2026-10-15 | cfg_attr(miri)；解释器不支持进程存活探测 | 该真实锁/进程路径不被解释器覆盖；原生测试仍启用 | 保留 Miri 条件；本机原生默认启用 | **core/storage 维护**；保留条件忽略；解释器支持对应宿主操作或有等价受控 I/O 接缝后，再在 Miri 跑原断言确认 |
| M07 | `file_lock_never_takes_over_unreadable_marker`<br>[crates/foundation/core/src/storage_atomic.rs](../../crates/foundation/core/src/storage_atomic.rs) | `09c2fcf36ca8`<br>2026-10-15 | cfg_attr(miri)；解释器不支持进程存活探测 | 该真实锁/进程路径不被解释器覆盖；原生测试仍启用 | 保留 Miri 条件；本机原生默认启用 | **core/storage 维护**；保留条件忽略；解释器支持对应宿主操作或有等价受控 I/O 接缝后，再在 Miri 跑原断言确认 |
| M08 | `file_lock_serializes_concurrent_writers`<br>[crates/foundation/core/src/storage_atomic.rs](../../crates/foundation/core/src/storage_atomic.rs) | `09c2fcf36ca8`<br>2026-10-15 | cfg_attr(miri)；解释器不支持进程存活探测 | 该真实锁/进程路径不被解释器覆盖；原生测试仍启用 | 保留 Miri 条件；本机原生默认启用 | **core/storage 维护**；保留条件忽略；解释器支持对应宿主操作或有等价受控 I/O 接缝后，再在 Miri 跑原断言确认 |
| M09 | `file_lock_released_when_closure_panics`<br>[crates/foundation/core/src/storage_atomic.rs](../../crates/foundation/core/src/storage_atomic.rs) | `09c2fcf36ca8`<br>2026-10-15 | cfg_attr(miri)；解释器不支持进程存活探测 | 该真实锁/进程路径不被解释器覆盖；原生测试仍启用 | 保留 Miri 条件；本机原生默认启用 | **core/storage 维护**；保留条件忽略；解释器支持对应宿主操作或有等价受控 I/O 接缝后，再在 Miri 跑原断言确认 |

## 恢复结果与验证

修改限于七个 organ 集成测试文件的 ignore 属性和相关失真注释，以及审计脚本。
保留所有历史函数名、测试输入、可执行语句、分支和断言。
`a2-preservation.log` 对七个文件分别比对：剔除注释、空白与原 ignore 属性后，源码完全相同；
同时检查每个恢复测试的原名称在普通执行日志中实际为 `ok`。
没有把 `--ignored --list` 的枚举成功当成测试执行通过。

原七个属性中的“需密钥/真模型”或“手动 trait shape”与实际 Noop 合同不匹配，
本批移除这些过时限制并纠正说明。独立的真实 organ/provider/council live 测试保持忽略，
不以 Noop 测试替代真实服务验收。

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test -p apeireth-organ --test causal_world_model --test curiosity --test emotion_memory --test hypothesis --test memory --test value_cases --test world_model --locked -j 2
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo clippy -p apeireth-organ --all-targets --locked -j 2 -- -D warnings
cargo fmt --check
python scripts/arch_guard.py --strict --log /tmp/apeireth-a2/a2-arch-native.log
python -m py_compile scripts/audit_ignored_tests.py
```

各命令真实退出码均以 `/workspace/apeireth-codex/a2-*.exit` 为准；对应日志为
`a2-restored-tests-final.log`、`a2-clippy.log`、`a2-fmt.log`、`a2-arch.log`、
`a2-python-compile-final.log`，以及枚举/历史复核用的 `a2-audit-final-verified.log`。
普通定向测试中七个恢复名称全部实际执行通过；没有运行仍忽略的外部服务、主机或大负载项目。
前置编辑脚本曾误匹配注释中的标记而在写源码前退出；早期确认日志仍显示原忽略状态，
保留为 `a2-before-restore-confirm.log`，不作恢复证据。最终只认 `a2-restored-tests-final.log`。

不引入外部依赖，不触碰 LOCKED、workspace.version、Cargo.lock 或测试基线数字真源；
本文的数量只表示本批审计集合和恢复差集，不替代 A4 全量测试统计，也不抄改 README 徽章。
源代码/审计脚本与文档分提交，按仓库提交 wrapper 执行 Codex 身份和 strict arch 检查；
最终完整 SHA、快进推送回执和后续 A3 归口见外部 STATE.md 与 A2 交付回执。

本机仓库钩子文件没有可执行位，Git 未自动执行它们；每次提交前另用 bash 显式运行
pre-commit 与 commit-msg，日志为 `a2-*-precommit.log` / `a2-*-commitmsg.log`。
提交 wrapper 的 secret、neutral 与 ArchStrict 检查照常执行，没有修改钩子配置或使用绕过开关。
