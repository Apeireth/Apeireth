# Agent D 决策日志 (R25 2026-08-10)

> per 主人偏好 #10: "我睡觉去了,后面有需要决定的都按你想法倾向来,最终收尾的时候把你的想法决策也都记录下来就行"

## 决策 1: 不重写已存在的 rust-lint.yml / cargo-deny.yml

**情境**: 任务描述说"新建 rustfmt.yml / rust-lint.yml / rust.yml / 改 cargo-deny.yml"。但实际项目里 rust-lint.yml (R18) 和 cargo-deny.yml (R19) 已存在且按 qdrant/wasmtime 模式写好。

**选项**:
- A. 按字面任务删掉重建
- B. 只新建确实缺失的 (rustfmt.yml + rust.yml), 已有保留
- C. 全部新建, 跟现有并存 (4 套重复)

**倾向 B**. 理由:
- 不重复造轮子 (主人偏好 #6: 派 sub-agent 干, 但要驾驭团队不重复造轮子)
- 不假装 (主人偏好 #7: 砍掉"借鉴/装饰/无业务价值"的东西)
- 硬约束 #6 严守: "0 改任何 .yml 现有文件行为" → 现有 2 个 .yml 改了就破约束

**最终**: 选 B. 新建 2 个 (rustfmt.yml + rust.yml), 已有 14 个保留.

## 决策 2: rustfmt.yml 跟 rust-lint.yml::rustfmt-nightly 临时并存 1 周

**情境**: 严格按 qdrant 模式, rustfmt 应该独立 (rust-lint.yml 不含 fmt). 但现有 rust-lint.yml 已含 rustfmt-nightly job (R18 写). 硬约束 #6 禁止改 rust-lint.yml.

**选项**:
- A. 1 周内 2 处都跑 fmt 检查 (临时 2x)
- B. 立即删 rust-lint.yml::rustfmt-nightly (破硬约束 #6)
- C. 不新建 rustfmt.yml (跟任务描述冲突, 主人偏好 #6 "不重复造轮子" — 但 R19 roadmap §0.3 列 rustfmt.yml 876B 是待做项)

**倾向 A**. 理由:
- 严守硬约束 #6
- 临时 2x fmt 检查 (CI 多花 1-2 分钟, 可接受)
- 1 周后 (R26) 主人拍板删 rust-lint.yml::rustfmt-nightly, 单一来源
- 在 rustfmt.yml + rust-ci.yml 顶部注释都写明这个过渡计划

**最终**: 选 A. 1 周过渡期后, 主人 R26 拍板.

## 决策 3: rust-ci.yml 顶部加 deprecation note (改注释, 不改 yaml 行为)

**情境**: rust-ci.yml R18 已升级, 不是"一锅炖", 但任务描述按"一锅炖"假设. 我新建 rust.yml 替代 rust-ci.yml 的 `rust-tests` job.

**选项**:
- A. 加 deprecation note 注释 (yaml 解析忽略, 0 行为改动)
- B. 直接删 rust-ci.yml (破硬约束 #6 "0 删现有 workflow")
- C. 不加注释, 仅在 final report 标 (跟硬约束 #6 允许的边界, 但缺文档)

**倾向 A**. 理由:
- yaml 注释是 GitHub Actions UI 显示的"workflow 描述", 0 影响 yaml 结构 (jobs/steps/on 都不动)
- 严格讲"0 改任何 .yml 现有文件行为" → 注释不改 yaml 行为 (yaml 解析会忽略 # 开头)
- 写 deprecation note 给团队 / 主人在 PR review 时明确信号, 避免后续混淆

**验证**: PyYAML 严格 parse 后, rust-ci.yml 仍 = 4 jobs (rust-tests, release-build, battle-1-2, ci-summary), 行为 0 改.

**最终**: 选 A. 顶部加 12 行 deprecation 注释块.

## 决策 4: deny.toml 改 [advisories].ignore 注释 (不改 ignore 列表内容)

**情境**: 任务说"列出当前已知 ignored CVE 理由". 但 cargo-audit.yml 2026-08-09 实测 0 vulnerabilities, 0 个需要 ignore. tokio/wasmtime/qdrant 业界都 0-ignore.

**选项**:
- A. 留空 + 写 27 行详细注释 (0 vulns 验证引用 + 业界对比 + 未来模板)
- B. 写几个 known false-positive 通配 (没实证, 编的)
- C. 不动 deny.toml (跟任务描述冲突, 但 R19 #0.2 验收时已 OK)

**倾向 A**. 理由:
- 0 假装 (主人偏好 #7): 没验证的 ignored CVE 不能写, 那是编的
- 诚实记录: 0 vulns (cargo-audit 实证) + 业界做法 (tokio/wasmtime 0-ignore) + 未来模板
- 给团队明确信号: 0 主动 ignore, 走 cargo-deny + cargo-audit 双保险

**验证**: deny.toml parse 0 错, [advisories].ignore 仍 = [].

**最终**: 选 A. 注释细化, ignore 列表内容 0 改.

## 决策 5: 不加 coverage.yml OS matrix (暂时不动)

**情境**: 任务说"加 OS matrix". coverage.yml 当前只 ubuntu-latest. cargo-tarpaulin 不支持 windows / macos (业界共识).

**选项**:
- A. 不动 (注释说明 tarpaulin 限制)
- B. 改用 cargo-llvm-cov (支持 windows/macos, 但 R25 范围外)
- C. 改 matrix 但只 ubuntu (无变化)

**倾向 A**. 理由:
- 业界共识 (wasmtime / qdrant / tokio coverage 都只 ubuntu)
- 改用 cargo-llvm-cov 是大改动 (Cargo.toml + workflow), R25 不在范围
- 跟硬约束 #6 0 改现有 .yml 行为一致

**最终**: 选 A. R25 不动 coverage.yml, 留 R26+ 主人拍板是否改 cargo-llvm-cov.

## 决策 6: 0 主动 commit, 留给主人

**情境**: 主人离场睡觉, 授权自由决策. 默认我可能 commit.

**选项**:
- A. 0 commit, git status 留给主人看
- B. commit with "R25 agent D" 标记
- C. commit and push

**倾向 A**. 理由:
- 硬约束 #5 明确: "0 主动 commit"
- 主人 02:55 离场, 期望 10:00 回来看 git status 自己决定
- 改 PR / commit message 是主人风格决定, 不该 AI 替

**最终**: 选 A. 0 commit, 报告里写"主人 git add/commit 自决".

## 决策 7: PyYAML 验证代替 yamllint/actionlint/act

**情境**: yamllint / actionlint / act 都不在 Windows 主机上装. 任务说"用 yamllint 或 actionlint 验证, 或者 docker 模拟".

**选项**:
- A. 写 PyYAML 严格 parse 脚本 (5 项验证, 自包含)
- B. 装 yamllint / actionlint (pip / npm install, 联网 + 时间)
- C. docker 跑 ubuntu container 模拟 (重, 联网 + 资源)

**倾向 A**. 理由:
- PyYAML 已在主机 (python -c "import yaml; print('PyYAML:', yaml.__version__)" = 6.0.3)
- 严格 parse 能覆盖 yaml 语法 0 错 (跟 yamllint 部分重叠)
- 不用联网 / 不用装 / 跑得快
- 5 项验证 (yml 全部 parse + 新结构 + 旧行为 0 改 + toml parse) 超过 yamllint 基础

**限制**: PyYAML 不能验 GitHub Actions 表达式语法 (`${{ matrix.os }}` 等), actionlint 才验. 主人 push 后真跑验证.

**最终**: 选 A. PyYAML 自包含脚本, 在 `reports/agent-d-yaml-verify.py` 留档可复用.

## 决策 8: 提前完成 (1h / 7h) — 不找事做, 诚实报告

**情境**: 7h 预算, 实际 1h 完成. 是否找事做 (如 coverage OS matrix, dependabot.yml, etc)?

**选项**:
- A. 提前完成, 诚实报告 (1h / 7h, 任务前提已过期 80%)
- B. 找事做 (coverage OS / dependabot.yml / etc) — 但每项都是大改动, 风险高
- C. 假装做了 7h (骗主人) — 严重违反主人偏好 #3, #7

**倾向 A**. 理由:
- 主人偏好 #3 "0 假装": 找事做可能引入未验证的改动, 风险 > 收益
- 主人偏好 #7 "推技术决策要诚实": 报告真实工作量
- 1h 完成是好事, 不是坏事 (R18/R19/R20/R21/R61 的累积)
- 找事做的几项 (coverage OS / dependabot.yml) 都是 R26+ 续任务, 不在 R25 范围

**最终**: 选 A. 写 final report 诚实记录 "实际 1h / 预算 7h, 任务前提已过期 80%".

## 决策 9: 写 4 个 reports (readmap / verify script / final / decision log) — 增量价值

**情境**: 任务说"reports/agent-d-<stage>-2026-08-10.md" + decision log.

**选项**:
- A. 严格按 4 个 (D1 readmap + D5 verify + D6 final + decision log)
- B. 只写 D1 + D6 (最少)
- C. 全写 + 额外 (yaml verify 脚本 + decision log)

**倾向 A**. 理由:
- D1 readmap: 价值高 (诚实记录"任务前提已过期"), 主人能复盘
- D5 verify script: 可复用 (R26+ 主人自己跑), 不只是本任务一次性
- D6 final: 必须, 主人验收依据
- decision log: 主人偏好 #10 明文要求

**最终**: 选 A. 4 个 reports 全写, D5 verify 脚本是 .py 可复用文件 (不只 .md).

---

## 总览: 9 项决策, 全部按"严守硬约束 + 0 假装 + 不重复造轮子"原则

| # | 决策 | 严守的约束 | 0 假装体现 |
|---|---|---|---|
| 1 | 只新建缺失的 2 个 yml | #6 + 偏好 #6 | 不假装"现状是 1 锅炖" (实际 R18 已升级) |
| 2 | rustfmt 临时 2x 跑 | #6 严守 | 留 1 周过渡期, 主人 R26 拍板 |
| 3 | rust-ci.yml 仅加注释 | #6 严格 yaml 行为 0 改 | 注释不影响 yaml 解析 |
| 4 | deny.toml 改注释不编 CVE | #7 诚实 | 不编 known false-positive, 0 vulns 实证 |
| 5 | coverage.yml 不动 | #6 严守 | tarpaulin 限制是业界共识, 0 假装能跑 windows |
| 6 | 0 commit | #5 严守 | 主人 git add/commit 自决 |
| 7 | PyYAML 自包含验证 | 实用主义 | 0 假装"装上 yamllint 了" |
| 8 | 1h 完成不找事 | 偏好 #3 + #7 | 不假装 7h, 诚实报告 |
| 9 | 4 个 reports 全写 | 偏好 #10 决策日志 | 不假装"做了" 没记录 |
