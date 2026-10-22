# 模板 · 开工自检清单（Preflight Checklist）

```text
[Document-Meta]
Document:        docs/04-internal/engineering-experience/templates/preflight-checklist.md
Version:         1.0（可复制形态）
Last-Modified:   2026-10-20
Status:          🟢 活跃
口径真源:        00-construction-handbook.md §1/§3；CONSTRUCTION-DISCIPLINE.md；AGENTS.md §5
```

> 复制本文件到批工作区，逐项打勾；任何一项不过，先解决再开工。

## A. 环境确认

- [ ] `git log --oneline -5` —— HEAD 与任务口径一致（不一致先对账，不假设）
- [ ] `git status` —— 认清哪些脏文件是**他线在途**（不碰不提交），哪些是本的
- [ ] 推送通道：`git -c http.proxy=http://127.0.0.1:7900 -c https.proxy=http://127.0.0.1:7900 -c http.version=HTTP/1.1 fetch origin` 试通（代理瞬断属常态，重试循环）
- [ ] author env 已设：`GIT_AUTHOR_NAME` / `GIT_AUTHOR_EMAIL` / `GIT_COMMITTER_NAME` / `GIT_COMMITTER_EMAIL`
- [ ] hooks 已启用：`git config core.hooksPath .githooks`（一次性）

## B. 声明与排队

- [ ] **write_scope 已声明**：本批动哪些文件/目录（宪章 §二.2 战场不重叠）
- [ ] 同文件区有他线在途 → 已排队或已报备接缝（接缝 = 合同：形状 + 词表 + 谁供谁消费 + 测试钉死）
- [ ] 认领登记：00-backlog 对应行状态 = 🔧 在飞（含本批）

## C. 基线（改任何 src/doc 前先跑，真账自己跑过）

- [ ] `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` → 落盘 log → 0 failed
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` → 0 警告
- [ ] `cargo fmt --check` → 0 diff
- [ ] 数字对照 [test-baseline.md](../../../03-reference/test-baseline.md)：不一致如实记「开工基线实测值 + 日期」，不改真源
- [ ] 全量出红 → 先定性再定罪（隔离复跑；并行扰动证据；真 flake 登记不定罪不掩盖）

## D. 红线自查（一票否决）

- [ ] LOCKED 5 项 0 触碰（9 锚本体 / 13 键 / 3 不可变脊柱 / workspace.version / R11 baseline）
- [ ] 不绕过 P0 governance 3 hook（Permission / CredentialDisclosure / PromptInjection）
- [ ] 本批产出将过 `scripts/check-neutral-terms.ps1`（命中先改表述，不先改豁免表）
- [ ] 取材类任务：读取预算已立 / 外部素材净化工序已备（渐进落盘三律，宪章 §六.1）
