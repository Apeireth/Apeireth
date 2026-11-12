# 生产装配回归的存储与阶段隔离

[Document-Meta]
Document: docs/design/production-test-reliability.md
Class: SPEC
Version: 1.0
Last-Modified: 2026-11-12
Status: 🟢 活跃
Author: lead

## 目标与范围

[PR #123](https://github.com/Apeireth/Apeireth/pull/123) 的真实 JUnit 定位了两类重试：
Windows/macOS 的网关装配测试打开共享会话库遇锁；Windows 的消息管线测试在短请求截止后仍等待处理器进入通知。
本工作线修正测试夹具与有效阶段假设，保留生产入口和业务契约验证，不新增 CI 重试、忽略或全局串行策略。
这些是本轮可追溯的失败，不能据此认定 PR #122 历史未知重试的身份。

## 批次与验收

| 批次 | 交付 | 验收 |
|---|---|---|
| F1 | 并行派发两条生产装配测试每例独立存储 | 三条路径均隔离，保留生产入口和调度断言；自有临时 CWD 默认会话库被排他锁定时旧例红、新例绿；同进程并发也通过 |
| F2 | 消息准入、取消与超时的阶段分别验证 | 不把短请求截止等同于已进入处理器；背压、取消、超时、无副作用及不确定执行状态仍有可判定断言；不靠提高等待护栏通过 |
| F3 | 当前来源回归与远端确认 | 受影响 target、格式与检查通过；最终提交三平台测试及实际 XML 可复核 |

## 进度与验收

F1/F2 软件修复完成（IMPLEMENTED），F3 本地定向验证通过；远端按最终提交验收。
当前失败来源为 `8cd3233e712c32e8b9e8d0ef54fc01590cafd683`，实际 checkout 为
`51baedd47cfec840ce41f24330037a73d88712f1`，workflow run `43155215071` attempt 1。
原报告保留失败输出；成功重试不证明首次执行成功。

F1 两条测试保留原生产入口和全部调度断言，每例使用独立临时目录，同时隔离会话库、认知库和数据根。
局部异步环境锁只保护该测试二进制的环境变量；不改变 nextest 全局并发设置。
环境守卫保存并恢复原始环境值，运行时先释放，再恢复环境并清理临时目录。

2026-11-12，Windows、Rust 1.97.1：在自建临时 CWD 的默认会话库持有 SQLite 排他锁，
旧二进制在原装配断言以 `database is locked` 退出 101；新二进制在相同锁条件下单例与双例并发均通过。
锁和数据库均在新建的临时目录，不使用仓库或用户既有会话库。

F2 把准入/取消与超时分开：前者明确等待处理器进入，同时监视请求提前结束，保留背压、取消、不确定执行和无副作用断言。
后者仍以真实 `100ms` 截止结束请求，不要求超时前一定已进入处理器；以独立内存存储中的 typed
`ModuleMessageStarted` 严格核对 `may_have_executed`，并验证无副作用及已创建上下文的取消传播。
预先取消的请求还明确验证未进入处理器、执行状态确定为未开始。现有 `10s` 诊断护栏未提高。
这修正了有效阶段假设；既有 XML 未记录两次失败时请求提前结束的具体结果，不把该诊断写成独立重现了当时的调度时序。

`cargo test -p apeireth-runtime-assembly --test module_message_pipeline --locked -j 2` 本地退出 0。
`desktop_message_policy`、`mesh_execution`、`mesh_production_host` 三个相关 target 同口径退出 0。
两处受影响 test target 的 `cargo clippy ... --locked -j 2 -- -D warnings`、
`pwsh -NoProfile -File scripts/fmt-check.ps1` 和体量预算检查均退出 0。
未修改生产代码、依赖、CI 重试或预算/测试基线。

历史测试名 `bounded_admission_cancellation_and_timeout_report_uncertain_execution` 本批拆为
`bounded_admission_cancellation_reports_uncertain_execution_after_handler_entry` 与
`message_timeout_reports_the_recorded_execution_phase_without_requiring_handler_entry`。
既有带日期验收记录保留原名，当前断言以这两条测试为准。

相邻 CLI 夹具中，`local_read_tools_knob`、`self_status_e2e`、`trait_cli`、`production_knobs`
仍有未隔离默认数据根的写面；本批不把整个 CLI 测试面标为全面隔离，邻近范围登记在 D37。
