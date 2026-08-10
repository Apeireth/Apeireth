# v2.0.0-alpha 镜像构建、供应链与 Registry Push Dry-run 验证报告

- **任务 ID**：`f9bba383-78d9-4f4e-962c-a8673e05fa7c`
- **角色**：DevOps Engineer
- **验证日期**：2026-08-05
- **验证方式**：本地 dry-run only；没有执行 registry login、push、tag 覆盖或远端 manifest 修改
- **工作目录**：`.openclaw\workspace\promethean\Apeireth-rust`
- **工作树 commit**：`e400e1492619cdb0b872b57fb539e8d99bf261c8`
- **验证时最新 integration ref**：`83ed6d22de181ae74a085b2ec1da178c1d8558d5`
- **完整命令证据**：`logs/docker-build-v2-alpha.log`
- **最终发布判定**：**NO-GO／不可推 registry，需修复并在容器工具齐备的受控 runner 重验**

> 本报告不把“工具不可用、扫描未运行”误写为“0 个漏洞”，也不把单架构 Dockerfile 的静态存在误写为 multi-arch manifest 已验证。验收要求中的“构建成功、amd64+arm64 完整、0 HIGH/CRITICAL”均需要真实执行证据；当前环境只能完成输入静态审计和失败路径 dry-run，因此必须作环境受限的 NO-GO 判定。

## 1. 执行摘要

本次任务要求验证 `docker/Dockerfile.v2-alpha` 构建、`linux/amd64` 与 `linux/arm64` manifest、Trivy 的 HIGH/CRITICAL 漏洞计数，并只做 registry push dry-run。验证开始时目标 Dockerfile在工作树和当时查询的 integration tree 中均未出现；并行任务随后在工作树创建了目标文件，因此静态审计可以继续，但最新 integration commit仍未包含该文件。验证宿主机没有 `docker`、`trivy`、`crane` 或 `skopeo` 可执行文件。

构建命令按要求记录为：

```text
docker build -f docker/Dockerfile.v2-alpha -t apeireth:v2.0.0-alpha .
```

由于 `docker` 不在 PATH，命令未进入 Docker daemon/buildkit，证据日志记录 `build_exit_code=127`。没有生成镜像 ID、配置 digest、层 digest、SBOM 或 provenance；因此也没有可供 Trivy 扫描的不可变目标。没有本地镜像或远端 registry digest，manifest inspect 无目标可查。整个过程中 `registry_credentials_used=false`、`registry_login_attempted=false`、`registry_push_attempted=false`，满足“不真推 registry”这一安全边界。

### 1.1 验收矩阵

| 验收项 | 要求 | 实测证据 | 结论 |
|---|---|---|---|
| Dockerfile 存在 | `docker/Dockerfile.v2-alpha` | 开始时缺失，执行前由并行任务写入工作树；integration `83ed6d22` 不含该文件 | ⚠️ 输入未合并 |
| 镜像构建 | exit 0 | Docker 不可用；日志 `build_exit_code=127` | ❌ 未通过 |
| amd64 manifest | 清单含 `linux/amd64` | 无镜像引用、无 Crane/Skopeo/Docker | ❌ 未验证 |
| arm64 manifest | 清单含 `linux/arm64` | 无镜像引用、无 Crane/Skopeo/Docker | ❌ 未验证 |
| HIGH CVE | 0 | Trivy 不可用且无镜像 digest | ❌ 未验证，不能声称 0 |
| CRITICAL CVE | 0 | Trivy 不可用且无镜像 digest | ❌ 未验证，不能声称 0 |
| Registry push | 禁止真推 | 无 login、无 push、无凭据使用 | ✅ 符合 dry-run 边界 |
| 是否可推 | 明确判断 | 构建、架构和 CVE 三个门禁未满足 | **NO-GO** |

## 2. 环境与输入基线

### 2.1 工具探测

验证前使用 PATH 探测确认：

```text
docker: UNAVAILABLE
trivy: UNAVAILABLE
crane: UNAVAILABLE
skopeo: UNAVAILABLE
```

这意味着以下动作都不能在本宿主机真实完成：Docker BuildKit 构建、Buildx 多平台输出、Docker manifest inspect、Crane manifest/raw inspect、Skopeo inspect，以及 Trivy image 扫描。本报告没有通过下载临时二进制或不受控脚本绕过环境约束，因为那会引入未经批准的供应链输入，并且仍缺少 Docker daemon 或远端待检镜像。

### 2.2 Git 状态与可重复性边界

验证工作树包含大量其他团队成员的并行未提交改动，因此没有切分支、reset、clean 或 checkout，以免破坏他人工作。任务通知称 V2 CI/CD 已 merged to integration，但验证时最新 integration ref 为 `83ed6d22de181ae74a085b2ec1da178c1d8558d5`，其 tree 中能找到 `.github/workflows/rust-ci.yml` 和 `deploy/docker-compose.protocols.yml`，找不到 `docker/Dockerfile.v2-alpha`。

目标 Dockerfile在首次探测后由并行任务写入当前工作树。构建证据日志同时记录：

```text
integration_dockerfile=MISSING
working_tree_dockerfile=present
docker=UNAVAILABLE
```

这不是矛盾，而是精确表示“集成基线未包含输入、并行工作树后来包含未合并输入”。正式 release runner 必须 checkout 一个明确 commit，并验证该 commit 的 tree 中包含 Dockerfile；禁止从脏工作树发布。

## 3. Dockerfile 静态审计

目标文件共 75 行，采用两阶段构建：

1. builder：`rust:1.80-bookworm`
2. runtime：`debian:bookworm-slim`
3. 默认 package 参数：`APEIRETH_CRATE=apeireth-cli`
4. 默认输出二进制参数：`APEIRETH_BIN=apeireth`
5. runtime 安装 `ca-certificates`、`tini`、`curl`
6. 入口：`/usr/bin/tini -- /usr/local/bin/apeireth`
7. 默认命令：`gateway serve --port 8080`

### 3.1 正向观察

- 多阶段构建避免把完整 Rust toolchain直接带入 runtime。
- `--no-install-recommends` 和 apt lists 清理可降低非必要包与层体积。
- `tini` 能正确转发信号并回收子进程，适合容器停止和滚动发布。
- runtime 只复制目标二进制，没有 `COPY . .` 把整个源码树带入最终层。
- Dockerfile提供 crate 与 binary 参数，允许同一模板服务不同 package。

### 3.2 发布阻断风险

1. **基础镜像未锁 digest**：`rust:1.80-bookworm` 与 `debian:bookworm-slim` 都是可变标签。同一个 Git commit在不同日期可能解析到不同 rootfs 与 CVE 集合，无法形成确定性 release artifact。
2. **无法证明 0 HIGH/CRITICAL**：漏洞属于解析后的镜像 digest，而不是标签字符串。没有构建产物和扫描数据库时间戳时，任何“已知 CVE 为 0”的人工声明都不可审计。
3. **没有 multi-arch 构建声明**：Dockerfile本身通常与架构无关，但 manifest list 必须由 Buildx 或 CI 工作流明确构建 `linux/amd64,linux/arm64` 并组合；当前文件不能证明清单存在。
4. **默认 root 运行**：文件注释也说明基础版未切换非 root 用户。这不直接属于本任务 CVE 数量，但属于发布安全门禁风险。
5. **无 SBOM/provenance**：没有 CycloneDX/SPDX SBOM、BuildKit provenance 或签名证据，难以追踪依赖与重建来源。
6. **构建上下文过宽**：`COPY crates ./crates` 会将整个 workspace 全部 crate送入构建上下文，构建时间、缓存失效面和潜在泄露面都较大。
7. **包名与二进制名可不一致**：改变 `APEIRETH_CRATE` 时如果未同时改变 `APEIRETH_BIN`，复制阶段会失败或复制错误程序。CI 应针对每种参数组合显式验证。
8. **工具链版本偏旧且不可变性不足**：Rust 1.80 标签未锁具体镜像 digest；是否满足所有新 crate MSRV、是否包含最新系统修复，都必须通过真实构建与扫描确认。

## 4. 构建验证证据

### 4.1 请求命令

```bash
docker build --progress=plain \
  -f docker/Dockerfile.v2-alpha \
  -t apeireth:v2.0.0-alpha .
```

### 4.2 日志摘录

```text
# v2.0.0-alpha image build validation log
timestamp_utc=2026-08-05T00:48:18Z
requested_command=docker build -f docker/Dockerfile.v2-alpha -t apeireth:v2.0.0-alpha .
working_tree_commit=e400e1492619cdb0b872b57fb539e8d99bf261c8
integration_commit=83ed6d22de181ae74a085b2ec1da178c1d8558d5
integration_dockerfile=MISSING
working_tree_dockerfile=present
docker=UNAVAILABLE
ERROR: docker build not executed because docker executable is not installed or not on PATH.
build_exit_code=127
registry_login_attempted=false
registry_push_attempted=false
```

exit 127 表示命令不可用，不是 Dockerfile编译失败，也绝不能算构建成功。缺少 BuildKit输出意味着以下数据均为 `NOT_AVAILABLE`：镜像 ID、runtime digest、config digest、各层 digest、镜像大小、构建耗时、cache hit率、二进制启动验证和 health endpoint结果。

### 4.3 合格 runner 的重验命令

```bash
set -euo pipefail
mkdir -p logs

docker version
docker buildx version

docker buildx build \
  --builder apeireth-release \
  --platform linux/amd64,linux/arm64 \
  --file docker/Dockerfile.v2-alpha \
  --tag registry.example.invalid/apeireth:v2.0.0-alpha \
  --provenance=mode=max \
  --sbom=true \
  --output=type=oci,dest=logs/apeireth-v2.0.0-alpha.oci.tar \
  . 2>&1 | tee logs/docker-build-v2-alpha.log
```

使用 OCI archive 输出可以在不推 registry 的前提下完成真正的多平台 dry-run。`registry.example.invalid` 仅作为无效示例，不能替换成真实 registry 后执行 push。若 Docker driver不支持 multi-platform OCI output，应改用 `docker-container` Buildx builder，而不是先推远端再检查。

## 5. Multi-arch manifest 验证

### 5.1 当前证据

```text
required_platforms=linux/amd64,linux/arm64
crane=UNAVAILABLE
skopeo=UNAVAILABLE
docker=UNAVAILABLE
image_reference=UNAVAILABLE
manifest_inspect_executed=false
amd64_evidence=NOT_VERIFIED
arm64_evidence=NOT_VERIFIED
```

当前没有 manifest JSON，因此不能给出 mediaType、schemaVersion、platform数组或每个平台的 digest。静态 Dockerfile没有 `FROM --platform=...` 并不代表多架构成功；最终结果取决于基础镜像是否提供两种平台、Rust依赖是否能在两种架构编译，以及 Buildx是否输出 manifest index。

### 5.2 必须保存的验收证据

合格证据至少应包含一个 OCI image index，且同时存在：

```json
{
  "platform": { "os": "linux", "architecture": "amd64" },
  "digest": "sha256:<amd64-manifest-digest>"
}
```

以及：

```json
{
  "platform": { "os": "linux", "architecture": "arm64" },
  "digest": "sha256:<arm64-manifest-digest>"
}
```

推荐在 OCI archive 上用 `skopeo inspect --raw oci-archive:...` 或 `crane manifest` 对明确引用执行解析，并把原始 JSON 保存到 `logs/v2-alpha-manifest.json`。若最终使用远端 dry-run staging registry，必须以 digest而不是可变 tag检查，避免检查与发布之间发生 TOCTOU。

### 5.3 建议检查命令

```bash
skopeo inspect --raw \
  oci-archive:logs/apeireth-v2.0.0-alpha.oci.tar \
  > logs/v2-alpha-manifest.json

jq -e '[.manifests[].platform | select(.os=="linux") | .architecture]
       | (index("amd64") != null and index("arm64") != null)' \
  logs/v2-alpha-manifest.json
```

只有 `jq -e` 返回 0，且两个子 manifest都能解包并启动，才能判定 multi-arch门禁通过。

## 6. CVE 扫描与清单

### 6.1 当前扫描结论

| 扫描目标 | Digest | Scanner | DB 时间 | HIGH | CRITICAL | 状态 |
|---|---|---|---|---:|---:|---|
| `apeireth:v2.0.0-alpha` runtime | 无 | Trivy 不可用 | 无 | 未验证 | 未验证 | 阻断 |
| `linux/amd64` manifest | 无 | 未执行 | 无 | 未验证 | 未验证 | 阻断 |
| `linux/arm64` manifest | 无 | 未执行 | 无 | 未验证 | 未验证 | 阻断 |
| `rust:1.80-bookworm` builder | 未解析 | 未执行 | 无 | 未验证 | 未验证 | 构建阶段风险未知 |
| `debian:bookworm-slim` runtime | 未解析 | 未执行 | 无 | 未验证 | 未验证 | runtime风险未知 |

**CVE 清单：当前没有可验证 CVE 条目，也没有可验证的 0 计数。** 原因是目标 Dockerfile使用未锁定 tag，未产生镜像 digest，Trivy不可用。人工 grep一个通用 Debian 或 Rust历史 CVE 列表无法证明当前镜像安全：包版本、架构、Debian revision、修复状态和扫描 DB 时间都会影响结论。为避免误导，本报告把 HIGH/CRITICAL 记为 `NOT_VERIFIED`，而不是 0。

### 6.2 必须执行的 Trivy 门禁

```bash
trivy image \
  --input logs/apeireth-v2.0.0-alpha.oci.tar \
  --scanners vuln \
  --severity HIGH,CRITICAL \
  --ignore-unfixed=false \
  --exit-code 1 \
  --format json \
  --output logs/trivy-v2-alpha.json
```

然后分别统计 HIGH 与 CRITICAL：

```bash
jq '[.Results[]?.Vulnerabilities[]? | select(.Severity=="HIGH")] | length' \
  logs/trivy-v2-alpha.json
jq '[.Results[]?.Vulnerabilities[]? | select(.Severity=="CRITICAL")] | length' \
  logs/trivy-v2-alpha.json
```

验收要求是两项都为 0，且要记录 Trivy版本、漏洞数据库更新时间、镜像 digest和扫描参数。若存在 unfixed HIGH/CRITICAL，不能仅凭 `--ignore-unfixed` 隐藏；需要风险接受记录、补偿控制与到期日，否则仍为 NO-GO。

### 6.3 基础镜像整改建议

- 将两个 `FROM` 固定为经审批 digest，例如 `image:tag@sha256:...`。
- 每周自动重建并扫描，以获取 Debian security更新；固定 digest不等于永不更新。
- 发布报告同时记录 builder 与 runtime digest，但门禁重点针对实际交付的 runtime层。
- 生成 SBOM并将其与镜像 digest绑定；Trivy JSON、SBOM、provenance均作为 release artifact保留。
- 在 amd64、arm64两个子 manifest上分别扫描，不能只扫描 manifest list的默认平台。

## 7. Registry Push Dry-run

没有调用 `docker login`、`docker push`、`crane push`、`skopeo copy ... docker://` 或 Buildx `--push`。没有读取、打印或使用 registry token。dry-run在本地构建前置条件处停止：

```text
registry_credentials_used=false
registry_login_attempted=false
registry_push_attempted=false
dry_run_result=BLOCKED_BEFORE_PUSH
```

### 7.1 理想 dry-run输出

真正的无推送 dry-run应产生本地 OCI archive，并验证以下事项：

1. tag格式为 `v2.0.0-alpha`，不会覆盖稳定 tag或 `latest`；
2. repository名称符合 registry策略；
3. OCI index同时含 amd64、arm64；
4. 两个平台的 digest明确且不同；
5. Trivy HIGH=0、CRITICAL=0；
6. SBOM和 provenance可读取；
7. 容器以预期入口启动，`/health`成功；
8. 发布命令预览不含实际 `--push`；
9. 回滚目标 digest已经记录。

## 8. 发布窗口、回滚与监控门禁

即使后续技术门禁全绿，上线前也必须明确以下运维条件。

### 8.1 建议发布窗口

- 选择工作日低流量窗口，预留至少 60 分钟观察期和 30 分钟回滚缓冲。
- 发布负责人、安全审批人和当班响应人必须在线。
- 窗口开始前冻结 registry tag变更，禁止多人并发推相同 tag。
- alpha镜像只能推到预发布 repository，不得直接替换 production stable/`latest`。

### 8.2 回滚策略

- 部署配置必须引用不可变 digest，不能只引用 `v2.0.0-alpha` tag。
- 推送前记录上一已知良好镜像 digest与配置版本。
- 若启动失败、健康检查连续失败、错误率或延迟越阈值，立即把部署引用切回旧 digest。
- 回滚不删除新镜像，保留现场用于根因分析；tag重新指向需审计。
- 数据迁移不属于镜像回滚能力，若 alpha包含不可逆 schema变更，必须另有数据库回滚方案。

### 8.3 最低监控告警

- 容器启动失败、CrashLoop、OOMKilled和重启次数。
- `/health` 可用性、HTTP 5xx率、P95/P99延迟。
- CPU、内存、文件描述符、磁盘与网络错误。
- 关键依赖连接失败和超时。
- 镜像 digest漂移、未知 tag部署和签名验证失败。
- 发布后 15、30、60 分钟分别复核，任何阻断告警触发自动或人工回滚。

## 9. 最终判定

### 9.1 判定：NO-GO／当前不可推 registry

阻断理由：

1. Docker构建没有成功，实际 exit code为 127；
2. 最新 integration commit未包含目标 Dockerfile，无法从干净、明确基线重现；
3. 没有 amd64+arm64 manifest原始 JSON或 digest证据；
4. 没有 Trivy扫描结果，HIGH/CRITICAL均为 `NOT_VERIFIED`，不能声称 0；
5. 基础镜像未锁 digest；
6. 没有真实镜像 ID、SBOM、provenance或启动健康检查结果。

### 9.2 转为 GO 的必要条件

- 将 Dockerfile合并到明确 release candidate commit；
- 在 Docker/Buildx可用的隔离 runner上构建成功，exit 0；
- OCI manifest显示 `linux/amd64` 与 `linux/arm64`；
- 分别扫描两平台镜像，Trivy HIGH=0且CRITICAL=0；
- 固定基础镜像 digest并保存 SBOM/provenance；
- 完成容器启动和 `/health` smoke test；
- 明确发布窗口、旧 digest回滚目标与监控告警；
- 经安全/发布审批后才允许真实 push。

## 10. 证据索引

| 文件 | 用途 |
|---|---|
| `docker/Dockerfile.v2-alpha` | 并行任务写入的待验证构建输入；验证时尚未位于 integration ref |
| `logs/docker-build-v2-alpha.log` | 工具探测、构建 exit code、manifest/scan未验证状态、未 push证明 |
| `reports/v2-alpha-image-validation-2026-08-05.md` | 本报告；发布门禁、风险和重验步骤 |

本报告的结论刻意保守：**环境受限可以完成验证任务的记录与判定，但不能让未执行的安全门禁自动变绿。当前不可推 registry。**
