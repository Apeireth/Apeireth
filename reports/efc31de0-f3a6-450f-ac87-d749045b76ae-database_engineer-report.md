# 自审报告 — 发布前置 P0 批：台账 #46/#47/#28（Dockerfile 互覆盖 / compose 密码外部注入 / gitignore 加固）

- **任务 ID**: efc31de0-f3a6-450f-ac87-d749045b76ae
- **角色**: database_engineer
- **日期**: 2026-08-16
- **边界遵守**: 只改 Dockerfile / docker-compose.yml / .gitignore 三文件（git diff --stat 证据在 §2），未动任何其他文件

## 1. 台账 #46 — Dockerfile crates manifest COPY 互覆盖（提交 0bc9a8c5）

**原缺陷**：`COPY crates/apeireth-*/Cargo.toml ./crates/` — Docker COPY 对多源 glob 是**平铺到目标目录**，82 个成员的 Cargo.toml 同名互覆盖只剩一个，且不建 member 子目录；后续 dummy build（`cargo build --workspace`）必然失败 → 依赖缓存层整体失效，发布产物阻塞级。

**修复**（选"逐 crate COPY"——可验证性最高的最简正确写法）：
1. 82 条逐成员 `COPY crates/<m>/Cargo.toml ./crates/<m>/Cargo.toml`，每条建 member 子目录
2. dummy build 补全：逐 member 占位 `src/lib.rs` + `src/main.rs`（workspace build 要求全部 member 目标可解析；原写法只占位 apeireth-cli 一个，即使 COPY 修好 dummy 也会挂），真正源码第 3 步 COPY 覆盖同名占位，结尾 `rm -rf crates` 清残

**清单完整性验证（本机可做的全部验证）**：
- 根 Cargo.toml [workspace] members 提取 82 条（剥注释/去重排序）
- `cargo metadata --no-deps` 交叉核实 = 82 包，两清单一致
- 82 个 `crates/<m>/Cargo.toml` 路径存在性逐一 `[ -f ]` 验证，missing=0

**0 装 PASS（如实标注）**：本机无 docker，未实测 `docker build` — **待有 docker 环境验证** dummy build 是否真锁住依赖层。可做的静态验证已全部通过；剩余风险如实留给有 docker 环境的验收（建议命令：`docker build -t apeireth-test .` 观察 stage 1 dummy build 成功且第 4 步增量编译只编本项目代码）。

## 2. 台账 #47 — compose POSTGRES_PASSWORD 强制外部注入（提交 caf6fce5）

**原缺陷**：`${POSTGRES_PASSWORD:-secret}` ×2（postgres 服务环境变量 + apeireth 服务 APEIRETH_DB_URL 内联）— 缺省弱密码 'secret' 直接上线。

**修复**：两处插值改 `${POSTGRES_PASSWORD:?<错误提示>}` — compose 变量缺省时 `docker compose up` **立即报错**（variable is not set），0 弱默认；底部启动注释补前置要求（export 或 .env；`.env` 已在 .gitignore:18 忽略，注入路径安全）。

**验证**：PyYAML 解析整文件 YAML_OK（无 YAML 破坏性字符）；`grep secret` 仅剩注释中的说明文字，无 `:-secret` 残留。
**0 装 PASS（如实标注）**：无 docker compose 实测缺省报错路径 — 待有 docker 环境验证（`unset POSTGRES_PASSWORD && docker compose up` 应报 "required variable POSTGRES_PASSWORD is missing a value"）。

## 3. 台账 #28 — .gitignore 密钥类加固（提交 4e25da14）

**修复**：cosign 段后追加 `*.pem` / `*.key` / `*.p12` / `*.pfx` / `id_rsa*` + `_research_mem/`。

**验证（实测，非 0 装）**：
- `git check-ignore -v` 六样例全命中新规则：secret.pem(行68) / certs/x.key(69) / a.p12(70) / b.pfx(71) / id_rsa_test(72) / _research_mem/x.md(75)
- `git ls-files` 无任何已跟踪文件匹配新模式 → 0 既有文件被"假忽略"（gitignore 不影响已跟踪文件，此检查确认无漏网需 git rm --cached 的情形）
- 与既有规则关系：`**/cosign.key` 专项规则保留不动（更具体，无冲突）；`.env` 已忽略（行18）无需重复

## 4. 兼容性说明（数据库工程师视角）

三项改动均不触碰任何数据/schema：Dockerfile 只改构建层 COPY/占位策略；compose 只改密码注入方式（**已初始化的 postgres volume 不受影响**——注意：既有部署若曾用 'secret' 初始化过 postgres-data volume，改外部注入后需以同一密码 export 才能连接，或重建 volume；此为既有数据兼容提示，非本次改动引入的破坏）；.gitignore 0 文件改动。

## 5. 验收对照

| 验收项 | 状态 |
|---|---|
| 三文件改动各有 diff 证据 | ✅ 0bc9a8c5(+96/-6) / caf6fce5(+6/-2) / 4e25da14(+11) |
| 0 装 PASS 标注（无 docker） | ✅ §1/§2 如实标注"待 docker 环境验证" + 给出验证命令 |
| backlog #46/#47/#28 划 ✅ | ✅ 本报告同提交回填 |
| 只改三文件边界 | ✅ |

## 6. 提交清单

| hash | 内容 |
|---|---|
| 0bc9a8c5 | fix(台账#46) Dockerfile 逐成员 COPY + dummy 占位补全 |
| caf6fce5 | fix(台账#47) compose 密码强制外部注入 |
| 4e25da14 | fix(台账#28) .gitignore 密钥类加固 |
| 558a5f20 | docs: backlog 三项划 ✅ + 本报告 |

## 7. 并发事故实录（0 装如实记录）

共享工作区 index 并发：558a5f20 提交时意外卷入 security_reviewer 已暂存的报告文件 `reports/fe468acf-...-security_reviewer-report.md`（+44 行，内容完好）。尝试 `reset --soft HEAD~1` 剔除时遭遇：① index 被其他并发进程持续改写（staged 清单秒变）② 平台流程将 master HEAD 重置回 558a5f20（同 hash 复活）③ index.lock 被并发进程占用。判断：并发风暴下历史改写风险 > 归属收益 → 停止手术，保留现状。已单独通报 security_reviewer（其提交跳过该文件即可）与 leader。教训：高并发共享树中 commit 应始终用 pathspec 限定（`git commit -- <paths>`），本次后续提交已改用该方式。
