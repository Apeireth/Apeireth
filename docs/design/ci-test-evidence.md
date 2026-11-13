# CI 测试重试与报告证据

[Document-Meta]
Document: docs/design/ci-test-evidence.md
Class: SPEC
Version: 1.2
Last-Modified: 2026-11-13
Status: 🟢 活跃
Author: lead

## 目标与范围

测试重试成功后仍应能追溯用例名称、失败输出和实际重试记录。
沿用现有 nextest CI profile、三个操作系统矩阵和 artifact 上传流程，保留重试次数、超时和判定断言。
此次缺口在 [PR #122](https://github.com/Apeireth/Apeireth/pull/122) 的最终检查中被确认：
Windows/macOS 汇总分别报告一个 flaky 用例，但逐例日志缺失，JUnit 上传步骤提示文件不存在。
历史失败身份和具体重试次数无法恢复，不能据此宣称零重试或已修复这些用例。

## 批次与验收

| 批次 | 交付 | 验收 |
|---|---|---|
| E1 | 启用 CI JUnit 与重试/最终 flaky 明细 | CI 重试次数、超时、断言、忽略集合保持原值；日志与 XML 能保留重试证据 |
| E2 | 独立报告契约探针 | 临时无依赖 Rust 包主动首错、随后成功；旧配置缺报告时真红，新配置包含用例和失败详情时真绿 |
| E3 | 上传证据绑定提交与执行轮次 | 缓存恢复后先清理预期旧报告；三平台真实生成 JUnit；缺文件明确失败；artifact 名绑定 SHA 与 attempt |
| E4 | 对完整重型回归声明调度资源预算 | TIFF 扫描与嵌套 Rayon compose 回归各自占用当前测试线程预算；保留所有样本、断言、超时和重试；独立探针在 2/4 个线程下验证调度反证，最终实际用例核对首轮结果及执行区间 |

探针独立于工作区测试，不计入产品通过用例数，不作为产品稳定性或能力分数。
探针所有文件位于自己创建并验证的临时目录，结束后清理；不读取用户记忆或业务数据。
使用本仓现有 Cargo/nextest 工具，不引入产品依赖或外部实现。

工作区在缓存恢复后、构建前调用 `--prepare-workspace-report`，只删除预期的
`target/nextest/ci/junit.xml` 单文件，拒绝路径逃逸、符号链接及 Windows 重解析点。
因此构建或测试未生成本轮报告时，上传步骤会明确失败，不能把缓存中的旧 XML 标成本轮证据。
清理步骤拒绝异常路径时，上传步骤也不执行；清理成功后，即使产品测试失败，仍尝试保留本轮报告。
artifact 名使用实际 checkout 的 SHA 与 workflow attempt；PR 工作流的 SHA 可以是测试合并提交。

## 进度与验收

E1/E2 软件验证通过（IMPLEMENTED）：保留原重试与超时配置，记录重试身份和首次失败输出，探针独立于产品工作区。
E3 上传接线由现有三平台工作流承载；合并条件是对应提交的三份实际 XML 可下载、可解析且名称绑定 SHA/attempt。
2026-11-12，[PR #123](https://github.com/Apeireth/Apeireth/pull/123) 已完成该条件并合并，验证面为 PRODUCTION WIRED。
实际 checkout 为 `51baedd47cfec840ce41f24330037a73d88712f1`，run `43155215071` attempt 1；
三份 XML 均解析成功、与日志逐项对应，探针未混入产品用例。
本轮具名 flaky 的后续修复见[生产装配回归稳定性](production-test-reliability.md)，不覆盖历史未知身份。
既有重试成功只说明该轮最终通过，不证明第一次执行成功，也不证明根因已清除。

2026-11-12 本地实测：Windows、Rust 1.97.1、cargo-nextest 0.9.151；
`python scripts/test_nextest_reporting.py` 在旧配置下，测试已重试成功但因 JUnit 缺失退出 1；
新配置退出 0。再次仅摘除 JUnit 接线，同一探针回到缺报告失败；恢复后再次通过。
探针同时验证恰好一次失败、随后成功，及 XML 的用例身份、失败输出和控制标记。
预放旧 XML 后禁用清理，探针退出 1；恢复清理后通过。报告路径为目录时拒绝删除，目录内文件保留。
本机创建符号链接缺少 Windows 特权，该分支未实测，不记作通过。

```text
FAIL: nextest report probe: CI JUnit report was not created by the controlled retry run
PASS: isolated nextest report probe retained retry identity and failed output in JUnit and logs.
```

旧 PR 中不可追溯的历史用例仍在债栏 D36 留账；本批修复报告链路，不把未知根因标为已清。

### E4：完整故障扫描的调度资源预算（2026-11-13）

[PR #125](https://github.com/Apeireth/Apeireth/pull/125) 的检查点
`90259f832cb50c45c72a0cd6d17aedb7356ce442` 最终 CI 检查成功，但 macOS 的实际 JUnit
仍保留 `apeireth-image-codecs::tiff_pages` 中
`zoo_decodes_and_never_panics_when_cut_or_corrupted` 的一次重试：首轮在 360.009 秒以
`test timeout` 结束，随后以 142.234 秒通过。旧检查点 `fcb3b30fe` 的同一用例首轮耗时
325.070 秒。该轮原 XML 与首次失败输出保留，最终成功不写成零重试或根因已清除。

该用例使用固定构造文件，逐字节截断、三种字节翻转及固定种子的变异序列，
原始测试没有依赖墙钟或外部服务。其解码路径会启动内部并行工作，原 CI 配置却仅为每个
测试保留一个调度槽。旧 XML 的时间区间核对显示，该首轮与其他 7793 个成功用例有墙钟重叠，
其中还包含耗时的内部并行图像测试。这是并发资源争用的线索；没有 CPU 遥测或独立负载重现，
不能把这些区间直接解释成完整根因证明。

首批 E4 仅为该完整测试的确切名称设置 CI 资源权重 `threads-required = "num-test-threads"`。
它按本次 nextest 实际线程预算保留调度槽，使这个单例执行时不与同一测试运行中的其他用例重叠；
其余用例仍按原配置并行。使用实际线程预算，而非宿主 CPU 数，避免命令行或环境覆盖线程数后
仍留下可并发的调度槽。原完整扫描的样本、循环、断言、360 秒终止边界、重试次数和 JUnit
记录方式保持，不通过减少检查量或放宽判定来消除超时。产品 Rust 源码不在此调整范围。

现有 `scripts/test_nextest_reporting.py` 的独立报告探针继续验证首次失败证据。
新增的独立临时调度夹具用同名重型测试和普通同行测试记录实际进入、离开事件，分别用 2/4 个
测试线程执行；这些探针不进入工作区 JUnit，不计入产品测试数。
2026-11-13 本地在 Windows、Rust 1.97.1、nextest 0.9.151 下实跑：新配置通过；
仅摘除资源声明后，8 个临时测试自身仍全部通过，但探针因 2 个线程下实际重叠退出 1；
逐字节恢复原配置后，2/4 个线程下均无重型重叠且普通同行仍并行，探针退出 0。
对应 `tiff-resource-probe-red.log`、`tiff-resource-probe-green-restored.log` 和
`tiff-resource-scheduling-reverse.json`，不把这些临时用例加入产品测试总数。

```text
FAIL: nextest report probe: resource probe heavy case overlapped a peer with 2 workers
PASS: isolated nextest resource probe reserved all 2/4-worker slots for its heavy case while ordinary peers ran concurrently.
```

验收须同时看到摘除该资源声明后探针因实际重叠失败、恢复后无重叠通过，以及最终 CI 中原完整
TIFF 用例的首轮结果、重试明细和相对其他用例的执行区间。本节记录提交前的诊断和调度调整快照；
探针红绿与最终用例区间按 [PR #125 的检查](https://github.com/Apeireth/Apeireth/pull/125/checks)
及绑定 SHA 的证据裁决，不从旧轮成功重试预报新结论或合并状态。

该资源声明只约束同一次 nextest 运行的调度，不承诺独占托管 runner 的全部系统资源，
也不证明任意负载下的性能上界。具体语义依照工具的[重型测试资源配置](https://nexte.st/docs/configuration/threads-required/)。

### E4 后续：完整 compose 回归与心跳观测（提交前快照）

检查点 `d69757618e17517eb686d20307677a559c962759` 的完整 TIFF 用例在 Ubuntu、Windows、
macOS 均首轮通过，分别耗时 131.154、116.753、170.722 秒。该检查点仍有独立阻断：macOS 的
`tests::effect_maps_with_nested_rayon_work_do_not_deadlock` 三次均失败；
`shutdown_reaps_the_heartbeat_thread_without_leaking` 首次因停机前 poll 计数 `1 -> 1` 失败，
重试通过。mdns、process 必需真实场景门禁的 SKIP 也继续阻断该检查点的合并裁决，不能记作 CI 通过。

心跳停机回归复用现有 10 秒有界观测，等全部四个线程的实际 `QuietPoll` 计数继续增长且至少完成
两拍，再验证停机。运行态、首次回收、四线程 join 和幂等断言保留；停机后原 120 毫秒观察窗
同时核对全部四个状态计数与实际 poll 计数不再增长。定向 9 个测试通过；仅断开实际 poll
观察者接线时，存活断言真实失败（退出 101），逐字节恢复后通过（退出 0）。生产心跳实现和
poll 间隔未改，固定短 sleep 缺少调度同步的缺口已由显式观测覆盖。

compose 的临时进度诊断保留原样本、100 轮及像素断言、120 秒 watchdog。固定单核的安静条件下，
101/101 次渲染完成，用例以 101.79 秒通过；同一 CPU 加入四个有限负载线程后，118.5 秒时仍推进到
25 次完整渲染，随后触发原 watchdog。这复现了持续推进但耗时越界的资源争用场景；没有复现
macOS 的 CPU 遥测，不能据此把该 CI 失败的完整根因标为已证实。临时进度接线已移除，原测试
逐字节恢复，所有负载、轮数、断言与 watchdog 保留。

后续配置仅为完整名称 `tests::effect_maps_with_nested_rayon_work_do_not_deadlock` 增加
`threads-required = "num-test-threads"`，TIFF 原声明保持。现有调度探针对这两个真实选择器分别创建
独立临时夹具，各用 2/4 个线程执行；JUnit 按完整名称核验，事件文件使用安全的叶名称，继续同时
检查活动标记、执行时间及普通同行并发。报告与双用例调度探针已定向退出 0，探针结果不计入产品
通过数。仅摘除 compose 资源声明、保留 TIFF 声明后，探针因实际重叠退出 1；逐字节恢复后退出 0。
原 compose 源码恢复后再跑原定向用例，首轮 21.99 秒通过；本轮全工作区 all-targets Clippy 零警告、
Windows 逐 crate fmt 覆盖 35 个 crate 且零差异。对应调度反证输出为：

```text
FAIL: nextest report probe: resource probe tests::effect_maps_with_nested_rayon_work_do_not_deadlock overlapped a peer with 2 workers
PASS: isolated nextest resource probes reserved all 2/4-worker slots for both heavy cases while ordinary peers ran concurrently.
```

最终实际用例的首次结果、重试和区间，统一按
[PR #125 的检查](https://github.com/Apeireth/Apeireth/pull/125/checks)及绑定最终 head SHA 的证据裁决；
本节的提交前结果不替代最终完整门禁，也不改写 d697 的失败与 SKIP 历史。

配置语义依照工具的[配置说明](https://nexte.st/docs/configuration/reference/)
和[JUnit 输出接口](https://nexte.st/docs/machine-readable/junit/)。
