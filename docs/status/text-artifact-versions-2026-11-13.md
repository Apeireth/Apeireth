# 文本成果与版本：C2施工进度

[Document-Meta]
Document: docs/status/text-artifact-versions-2026-11-13.md
Class: STATUS
Version: 4
Last-Modified: 2026-11-18
Status: 本地工程验证完成；用户C2视觉验收通过；2026-11-18 进入主干合并评审，未发布
Author: desktop-collaboration-line

## 目标与基线

[完整契约与批次](../design/text-artifact-versions.md)。基线b6b98ab07795a1cb72747027418ccc34b4ecfe4a；新分支feat/c2-artifact-versions-20261113。只本地施工，不推送/合并/发布。

功能源码检查点`9cc6ab3a9a375b4b0d3f3b7c842a0a621278b81c`；工程验证与文档收口提交`8b0aa464992badbe42f4ff98db725b64c12b748a`，本地`checkpoint/c2-artifact-engineering-20261113`保持指向该提交。候选程序在b6基点dirty源码构建，SHA256为`af6135d30245b3cb21790a576383d85545a07a17db7614e059e858ca69b1cf9a`，实际使用该程序完成修复后精确导出及工具共核检查，不倒填构建HEAD。

## 当前进度与还差什么

- 共同schema1已实现：权威SQLite Artifact/Revision/Receipt与seal、事务CAS、持久幂等、历史恢复、精确导出，受治理工具与HTTP显式共享同Arc。
- 受影响protocol/assembly/gateway/cli的lib检查通过；strict架构守卫无违规。四项粗粒度crate业务预算按ADR0025有限接受，gateway继承超额与本批净增分别保留红灯证据，不抬单文件架构基线。
- 实际独立受控canonical链已验证：agent读能力、原审批后写稿、HTTP人工提交第二版、agent再读到相同人类新版本；两条出口共用权威库，工具写保留可信来源。
- 私有桌面schema6文本工作面已有功能检查点；真实原生窗口完成用户编辑/迟到ACK/丢回执、同数据重启恢复、明确版本AI续做候选、拒绝与过期保护、冲突/rebase、历史恢复及精确导出。旧只读Document不改。
- 导出实测先红：服务返回正文但缺ETag，原生端拒绝不可核验下载。已补响应头和实际red→green测试；保留原失败reservation/JSON，同数据换程序后仅GET/下载续验，零模型或写动作重放；正常重开检查通过。
- CAS与人工审批底线均实际删除关键守卫做反向验伪，红后还原精确源码再绿。未以静态断言冒充行为证据。
- 本轮全workspace all-targets、doc tests、strict Clippy、Windows逐crate格式检查均退出0；claims、生成STATUS/wiring、环境登记与strict接线交叉检查通过。原b6两条生产引用登记遗漏已实跑复现，按真实CI/命令注册入口补来源，不改守卫或等级。用户于2026-11-13通过本轮C2视觉验收，尚无推送/合并/发布。

首批明确single_operator，不宣称多用户/非default域成果协作。现有session_users不是持久owner依据；不在未装配域回落默认库。人保存与agent审批是独立语义，同一权威内核。

## 2026-11-18 主干合并复核

本线以`git cherry-pick -n 9cc6ab3a9`落到当前主干`c6c5a20d7`，不是整支合并旧基线分支：只带入成果/版本内核的29个文件，不带入旧基线上的会话历史与回合结果分叉内容。冲突按当前主干结构解决：保留主干本地native handler，成果路由经`artifacts::routes_with_panels()`与顶层`protect_response`接入；遗留PanelData不制造版本库，真实服务由组合根注入。`canonical_entry.rs`保持2291行，等于架构基线。

本批复核门禁：四包`cargo check --all-targets`退出0；四包`--lib`回归通过（gateway首轮两例`cache-control`断言失败，根因是保护层放在认证层内侧，移到最外层后通过）；Windows逐crate格式检查35个crate零diff；四包clippy`-D warnings`零警告；strict架构守卫零违规；未接线审计退出0；crate预算按ADR0025重建为34个crate·605507行；claims真源、生成STATUS/wiring、文档索引与strict接线交叉检查在文档提交中另行复核。全量workspace、live E2E与外部模型质量仍未在本批重跑或声明。合并评审与发布状态以主干记录为准；私有前端不在本仓推送范围。

## 用户视觉验收记录（2026-11-13）

用户查看实际运行的C2工作面后明确确认：

> 看着没什么问题，这轮过，保存一下验收进度。接下来你建议先做什么？

本次查看使用独立可写验收副本，通过本机9891入口直连后端，原工程验证资料保持封存。该副本没有配置外部模型；本次通过记录界面视觉验收，不扩大此前工程验证的技术范围，不新增外部模型质量、多用户或跨平台验证结论。

本次仅保存验收进度，功能源码与上述工程检查点均未改动；[本仓验证账](text-artifact-versions-2026-11-13-verification.json)保留原样。没有重跑Cargo测试、Clippy或格式检查，也没有推送、合并或发布。

## 检查与证据

施工期间由根代理串行运行Cargo、记录退出码和原始日志；子代理不并行争用共享target。[本仓验证账](text-artifact-versions-2026-11-13-verification.json)保留实际命令、工具链、日志摘要/SHA、红绿反证及继承登记问题。原始日志在本机TEMP `apeireth-c2-*`。全量采用独立target、test/dev debug=0、incremental=0、jobs=2以控制本机资源；未关闭debug assertions、未启用ignored/live/hardware测试，也未把忽略项当成已验证。当前Windows软件测试不代表Linux或硬件/真实外部模型验收。未改历史全局test-baseline计数或徽章。

主原生链三次、独立工具链六次本机受控provider调用分别记账，外部模型调用零；不复用既有临时key，不作外部模型质量或硬件验证声明。主夹具与工具夹具owned子进程均退出；截图没有保存。用户默认资料升级另由系统入口实窗核对，不能把TEMP profile当成默认库。
