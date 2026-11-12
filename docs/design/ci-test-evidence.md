# CI 测试重试与报告证据

[Document-Meta]
Document: docs/design/ci-test-evidence.md
Class: SPEC
Version: 1.0
Last-Modified: 2026-11-12
Status: 🟢 活跃
Author: lead

## 目标与范围

测试重试成功后仍应能追溯用例名称、失败输出和实际重试记录。
沿用现有 nextest CI profile、三个操作系统矩阵和 artifact 上传流程，不改变测试判定政策。
此次缺口在 [PR #122](https://github.com/Apeireth/Apeireth/pull/122) 的最终检查中被确认：
Windows/macOS 汇总分别报告一个 flaky 用例，但逐例日志缺失，JUnit 上传步骤提示文件不存在。
历史失败身份和具体重试次数无法恢复，不能据此宣称零重试或已修复这些用例。

## 批次与验收

| 批次 | 交付 | 验收 |
|---|---|---|
| E1 | 启用 CI JUnit 与重试/最终 flaky 明细 | CI 重试次数、超时、断言、忽略集合保持原值；日志与 XML 能保留重试证据 |
| E2 | 独立报告契约探针 | 临时无依赖 Rust 包主动首错、随后成功；旧配置缺报告时真红，新配置包含用例和失败详情时真绿 |
| E3 | 上传证据绑定提交与执行轮次 | 缓存恢复后先清理预期旧报告；三平台真实生成 JUnit；缺文件明确失败；artifact 名绑定 SHA 与 attempt |

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

配置语义依照工具的[配置说明](https://nexte.st/docs/configuration/reference/)
和[JUnit 输出接口](https://nexte.st/docs/machine-readable/junit/)。
