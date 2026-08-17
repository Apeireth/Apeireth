# TP33 发布工程门槛收尾报告（#46 Dockerfile 验证 + #47 compose 密码外部化）

**任务ID**: 1c03734b-a66a-4506-9096-774356b4c26a
**角色**: devops_engineer
**日期**: 2026-08-17
**纪律**: #8 实测或诚实标注；#8 禁止真实密码入 yml；0 装 PASS

---

## 一、本机 docker 环境核验

```bash
$ docker --version
docker: command not found
$ which docker
(no docker in PATH)
```

**结论**: 本机 **未装 docker**，按纪律 #8 必须 **诚实标"待实测"**，**不假装**绿色。下面给出基于 Dockerfile 内容的构建预期分析，并补一处实际修复（见 §三）。

---

## 二、#46 Dockerfile 修复验证

### 2.1 文件清单
| 文件 | 大小 | 状态 |
|---|---|---|
| `Dockerfile` | 10772 bytes | 已修复+补全（见 §三） |

### 2.2 静态审查结果

#### 3-stage 结构（per blueprint §3.2）
| Stage | FROM | 用途 | 行号 |
|---|---|---|---|
| 1 | `rust:1.80-slim-bookworm AS builder` | 编译 + dummy build 缓存依赖 | L13–125 |
| 2 | `debian:bookworm-slim AS runtime-deps` | 运行时动态库（libssl3/libsqlite3-0/libgit2-1.7） | L128–131 |
| 3 | `gcr.io/distroless/cc-debian12:nonroot AS final` | 仅复制二进制 + 动态库，~150MB 非 root 用户 | L134–152 |

✅ 3-stage 完整 / multi-arch 注释就位（L5–6 + L165–167 buildx 命令）/ distroless + nonroot（L147）/ 健康检查 + 健康端点 `--health-check`（L151–152）。

#### 依赖层缓存 + dummy build
- L20 `COPY Cargo.toml Cargo.lock ./` — workspace root 元数据
- L24–105 **82 → 83 条**逐成员 `COPY crates/apeireth-*/Cargo.toml ./crates/apeireth-*/Cargo.toml`（详 §三）
- L111–118 **dummy build 触发依赖编译**：
  ```bash
  for d in crates/*/; do
    mkdir -p "$d/src"
    [ ! -e "$d/src/lib.rs" ] && echo '' > "$d/src/lib.rs"
    [ ! -e "$d/src/main.rs" ] && echo 'fn main(){}' > "$d/src/main.rs"
  done
  cargo build --release --workspace --bin apeireth
  rm -rf crates
  ```
  对 `crates/*/` 全通配（含嵌套 `apeireth-memory/extensions/`），自动覆盖新增成员。

#### 真正构建
- L121 `COPY crates/ ./crates/`（覆盖占位 src）
- L124–125 `cargo build --release --workspace --bin apeireth && strip target/release/apeireth`

✅ 占位 src 自动建（lib.rs+main.rs），真正源码 COPY 覆盖同名 → workspace 全成员目标可解析 → strip 二进制减体积。

### 2.3 构建预期（基于 Dockerfile 内容推断，本机无 docker 实测）

**预期成功路径**：
```
[builder stage]
  Step 1/12 : FROM rust:1.80-slim-bookworm      ✓ ~1.2 GB 镜像
  Step 2/12 : apt-get install (pkg-config libssl-dev libsqlite3-dev libgit2-dev)  ✓ ~5 MB
  Step 3/12 : WORKDIR /build                     ✓
  Step 4/12 : COPY Cargo.toml Cargo.lock          ✓ ~50 KB
  Step 5/12 : COPY 83 × crates/apeireth-*/Cargo.toml  ✓ 缓存命中后无操作
  Step 6/12 : RUN dummy build (workspace 解析)   ⚠  首次 ~3-8 分钟（依赖图锁住）
  Step 7/12 : COPY crates/ ./crates/             ✓
  Step 8/12 : RUN cargo build --release --workspace --bin apeireth
                                              ⚠  首次 ~5-15 分钟（仅 apeireth-cli 重编）
  Step 9/12 : strip target/release/apeireth      ✓

[runtime-deps stage]
  Step 10/12 : apt-get install libssl3 libsqlite3-0 libgit2-1.7  ✓ ~50 MB

[final stage]
  Step 11/12 : COPY --from 动态库 + binary       ✓
  Step 12/12 : USER nonroot / EXPOSE 8080 9090 / HEALTHCHECK  ✓

最终镜像: gcr.io/distroless/cc-debian12:nonroot + apeireth binary
预估体积: ~150 MB（per blueprint §3.2）
```

**关键风险点**（实测时盯住）：
1. **L111–118 dummy build** 必须看到 `Compiling apeireth-credentials v1.2.0`（补 COPY 后的第 83 成员），否则 workspace 解析失败。
2. **L117 `cargo build --release --workspace --bin apeireth`** 输出末尾应见 `Finished release [optimized] target(s) in ...`，无 `error[E0...]`。
3. **L124–125 真正 build** 应无 `error:` / `warning:` 数应为 0 或仅少数已知无害告警。

**实测前置**：需 Linux buildx 环境（本机 Windows 无 docker），推荐命令：
```bash
docker buildx build --platform linux/amd64 \
  -t apeireth/apeireth:1.0.0-validate . 2>&1 | tee /tmp/docker-build.log
# 期望: 12 steps 全部 ✓, 最终输出 "naming to docker.io/apeireth/apeireth:1.0.0-validate"
docker images apeireth/apeireth:1.0.0-validate --format "{{.Size}}"
# 期望: ~150 MB 量级
```

### 2.4 **【新发现 + 已修复】workspace 成员 vs Dockerfile COPY 不一致**

#### 发现
对比 `Cargo.toml [workspace] members` 与 `Dockerfile` 显式 COPY 列表，发现 **1 个成员漏 COPY**：

| 检查项 | 数 | 来源 |
|---|---|---|
| Cargo.toml workspace members | **83** | `grep -cE '^\s*"crates/apeireth-' Cargo.toml` |
| Dockerfile 显式 COPY 成员（验证前） | 82 | `grep -cE '^COPY crates/apeireth-' Dockerfile` |
| 差异 | **1** | — |

**遗漏成员**：`crates/apeireth-credentials`（TP3/N21 统一凭据存取层）

#### 根因追溯
```
commit 0bc9a8c5 (2026-08-17 01:19) fix(台账#46)
  当时 workspace = 82 成员, Dockerfile COPY = 82 → 82/82 ✓
后续 (0bc9a8c5 之后) 新增:
  "crates/apeireth-credentials",  # TP3/N21: 统一凭据存取层 (§10 装配主链第一环)
```
→ Cargo.toml 加了第 83 成员，但 **Dockerfile 没同步更新**。dummy build 阶段 `cargo --workspace` 解析会找不到 `apeireth-credentials/Cargo.toml` → **构建必失败**（这就是 #46 修复的核心阻塞场景）。

#### 修复（已落盘）
```diff
 COPY crates/apeireth-council/Cargo.toml ./crates/apeireth-council/Cargo.toml
+COPY crates/apeireth-credentials/Cargo.toml ./crates/apeireth-credentials/Cargo.toml  # TP33: 补 manifest COPY (0bc9a8c5 后新增成员, 缺则 cargo --workspace 解析失败)
 COPY crates/apeireth-cron/Cargo.toml ./crates/apeireth-cron/Cargo.toml
```

#### 修复后复核
```
$ grep -cE "^COPY crates/apeireth-" Dockerfile
83
$ comm -23 cargo_members.txt dockerfile_members.txt   # Cargo 有但 Docker 没有
(空)
$ comm -13 cargo_members.txt dockerfile_members.txt   # Docker 有但 Cargo 没有
(空)
✅ 83 = 83, 双向无 diff
```

**注意**：`apeireth-credentials` 是 `[lib]` only（无 `[[bin]]`），dummy build 的 `main.rs` 占位会被创建但 **不会被编译**（无 bin 目标）→ 对 dummy build 无副作用。

### 2.5 验收结论
| 检查项 | 结果 |
|---|---|
| 3-stage 结构 | ✅ 完整 |
| 依赖层缓存（COPY Cargo.toml + dummy build） | ✅ 完整 |
| 82 → 83 逐成员 manifest COPY | ✅ **修复后** 83/83 |
| dummy build 占位 src 全通配 | ✅ `crates/*/` 覆盖 |
| 真正 build + strip | ✅ L124–125 |
| distroless + nonroot + HEALTHCHECK | ✅ |
| **实测构建** | ⬜ **待实测（本机无 docker，按纪律 #8 诚实标注）** |

---

## 三、#47 compose 密码外部化验证

### 3.1 文件清单
| 文件 | 大小 | 状态 |
|---|---|---|
| `docker-compose.yml` | 3949 bytes | 已修复（commit caf6fce5） |
| `.env.example` | 1217 bytes | **新建**（占位） |

**确认**：`ls docker-compose*.yml` 只 1 个文件（无 dev/override/test 变体），无 `.env` 实体（仅 `.env.example` 占位）。

### 3.2 硬编码密码 grep 验证（任务指定正则）

#### 任务正则（含注释）
```bash
$ grep -rn -E "(POSTGRES_PASSWORD|MYSQL_ROOT_PASSWORD|REDIS_PASSWORD|DB_PASSWORD|MONGO_PASSWORD|API_KEY|SECRET_KEY).*[:-][^${]" docker-compose*.yml
# 3 行命中 — 全部为 ${VAR} 插值 (line 25/27/59), 无任何硬编码值:
#   25:      APEIRETH_API_KEY: ${APEIRETH_API_KEY:-}
#   27:      APEIRETH_DB_URL: postgresql://apeireth:${POSTGRES_PASSWORD:?POSTGRES_PASSWORD 未设置...}@postgres:5432/apeireth
#   59:      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:?POSTGRES_PASSWORD 未设置...}
```
✅ 任务正则意图是"排除 `${VAR}` 形式, 仅抓硬编码"，但字面 regex `[:-][^${]` 在 `${VAR}` 后空格处会匹配（`[^${]` 排除 `$` 但空格不属 `${`），所以 3 行命中 — 这 3 行**都是合法的 `${VAR}` 插值**（1× `${VAR:-}` 默认值，2× `${VAR:?...}` 必填），符合任务"仅允许 `${VAR:?...}` 形式"的精神。

#### 排除注释 + 排除 ${VAR} 后的精确硬编码验证
```bash
$ grep -nE "(POSTGRES_PASSWORD|MYSQL_ROOT_PASSWORD|REDIS_PASSWORD|DB_PASSWORD|MONGO_PASSWORD|API_KEY|SECRET_KEY)" docker-compose.yml \
    | grep -vE '^[[:space:]]*#' \
    | grep -vE '\$\{[A-Z_]'
# 仅 3 行注释提及 (line 57, 113, 114), 全部以 # 开头, 无任何赋值:
#   57:      # 台账 #47: 强制外部注入, 无默认值 — POSTGRES_PASSWORD 缺省时...
#  113:# 启动前置 (台账 #47): 必须提供 POSTGRES_PASSWORD (无默认值, 缺省直接报错):
#  114:#   export POSTGRES_PASSWORD='<强密码>'   # 或写入 compose 同目录 .env 文件
```
✅ **0 真实硬编码**（这 3 行是文档注释，不是 yml 字段赋值）。

### 3.3 `${VAR}` 引用清单（任务要求）

| 行 | 服务 | 变量 | 形式 | 说明 |
|---|---|---|---|---|
| 22 | apeireth | `APEIRETH_LLM_BACKEND` | `${VAR:-default}` | 默认 `scripted` |
| 23 | apeireth | `APEIRETH_LLM_API_URL` | `${VAR:-default}` | 默认 `https://api.minimaxi.com` |
| 24 | apeireth | `APEIRETH_LLM_MODEL` | `${VAR:-default}` | 默认 `MiniMax-M3` |
| 25 | apeireth | `APEIRETH_API_KEY` | `${VAR:-default}` | 默认空字符串 |
| 27 | apeireth | `POSTGRES_PASSWORD` | `${VAR:?<err>}` | **必填**，DB URL 内联 |
| 59 | postgres | `POSTGRES_PASSWORD` | `${VAR:?<err>}` | **必填**，PG 服务认证 |

**总计**: 6 处 `${VAR}` 引用，其中 **2 处 `${VAR:?...}` 强制必填**（POSTGRES_PASSWORD 在 DB URL + PG 服务各 1 处，缺一即 `variable is not set` 报错）。

### 3.4 `.env.example` 占位（新建）

**纪律 #8 检查**：禁止真实密码入 yml ✅（`.env` 已在 `.gitignore` line 18 排除 → 用户填 `.env` 不会被 commit）。

**新建文件**: `redacted/Apeireth-rust/.env.example`（1217 bytes）

包含：
- `POSTGRES_PASSWORD=__REPLACE_WITH_STRONG_PASSWORD__`（唯一必填占位，含使用说明）
- `APEIRETH_LLM_BACKEND/API_URL/MODEL` 三个有默认值变量（列出来供生产部署覆盖）
- `APEIRETH_API_KEY=`（空占位，明确"留空=scripted 桩，真用 LLM 必须填"）
- 用法说明 + 密码生成推荐 `openssl rand -base64 32 | tr -d '=+/' | head -c 32`

### 3.5 验收结论
| 检查项 | 结果 |
|---|---|
| 硬编码密码 grep 0 残留 | ✅ |
| `${VAR:?...}` 强制必填 | ✅ 2 处（POSTGRES_PASSWORD × 2） |
| `${VAR:-default}` 可选覆盖 | ✅ 4 处 |
| `.env.example` 占位 | ✅ **新建**（无真实密码） |
| `.env` 在 `.gitignore` | ✅ line 18 已排除 |
| 启动注释前置说明（line 113–118） | ✅ 含 export/.env 两种注入方式 |

---

## 四、变更清单（本任务产出）

```
M  Dockerfile                    +1 行 (apeireth-credentials manifest COPY)
A  .env.example                  +1217 bytes (无真实密码占位)
```

未触动：`docker-compose.yml`（caf6fce5 已完整修复，本次仅验证）。

---

## 五、台账勾选（建议同步）

| backlog # | 项 | 本次发现/产出 | 建议 |
|---|---|---|---|
| **#46** | Dockerfile COPY crates 互覆盖修复 | 验证发现 **1 成员漏 COPY（apeireth-credentials）已修复**，83/83 一致；本机无 docker 未实测 | 保留 `✅ 提交 0bc9a8c5` 历史外，**追加本任务 commit**（补 COPY）；实测待有 docker 环境 |
| **#47** | compose POSTGRES_PASSWORD 外部化 | grep 0 残留验证；${VAR} 引用清单确认；.env.example 新建 | 保持 `✅ 提交 caf6fce5`，可补充 `.env.example` 一笔（可选） |

---

## 六、纪律核对

| 纪律 | 执行 |
|---|---|
| #8 实测或诚实标注 | ✅ Dockerfile 验证标"本机无 docker，待实测" + 给出基于内容的构建预期 + 找到 1 个真实 bug 并修复 |
| #8 禁止真实密码入 yml | ✅ 仅 `.env.example` 占位（`__REPLACE_WITH_STRONG_PASSWORD__`），`.env` 在 `.gitignore` 排除 |
| 0 装 PASS 禁止"模拟绿" | ✅ 无 docker 就不写构建成功日志，只给预期路径 + 风险点 |
| 台账完成即划 ✅ | ✅ 已在 §五 给建议（建议 Leader 同步 backlog.md） |

---

## 七、待办（移交）

| 项 | 责任人 | 阻塞 |
|---|---|---|
| 实际 docker buildx 测试 Dockerfile | 有 docker 的环境（CI runner / Linux 工作站） | 当前 Windows 开发机 |
| 把本任务 commit 同步到 backlog.md #46 条目 | technical_writer 或 leader | 报告已交付 |
| 可选：`.env.example` 在 README/INSTALL.md 加引用 | technical_writer | 无阻塞 |