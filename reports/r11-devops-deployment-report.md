# R11 DevOps — 真部署可复现性审查与离线门禁落地

> 任务: 真部署真跑 (deploy/)
> 作者: R11 devops_engineer
> 范围: canonical `deploy/` 资产、validator 语义门禁、daemon/进程真探测
> 哲学 anchor: 主 22:33 终极授权 + 主 17:43 实事求是 + 主 17:58+主 20:46 不假装 + 主 19:33 走在前人经验上 + 主 23:44 干到底 + 主 00:56 任何人都能接手

## 1. 任务边界与原则

本轮只在 Omnicompass 真实任务里负责把 `deploy/` 的可复现性拉到“任一维护者 clone → 一行命令构建”级别，同时保证 validator 不再以“看起来像 YAML”冒充“容器在跑”。三道独立证据需要分别记录：

1. **静态 + 子进程验证 (offline)**: 子进程渲染 V1008/V1032，校验 canonical `deploy/Dockerfile + docker-compose.yml + k8s-asi.yaml + requirements.txt` 互相一致。
2. **Daemon 真探测**: docker / docker-compose / kubectl 在 PATH 是否可用。
3. **运行时真健康 (runtime)**: 8765 / 8875 端点 HTTP 200 + JSON 字段正确，并明确 docker 不在本地时这一项的失败原因。

三条证据任一缺失都会反映在报告里——不会用 offline_valid 冒充 runtime_valid。

## 2. canonical `deploy/` 资产修复

旧文件三处硬错使得"按图索骥构建"必然失败，已全部修正：

| 文件 | 旧问题 | 修复 |
|------|--------|------|
| `deploy/Dockerfile` | `FROM python:3.11-slim` 与项目 Python 3.13.14 不匹配；`COPY requirements.txt` 复制仓库根不存在的清单；运行用户仍是 root；镜像入口指向 `apeireth.v1075_asi_real_deployment_run` 但端口固定 8765 难调；HEALTHCHECK 引用 docker 子网 localhost | pin `python:3.13.14-slim-bookworm`；新增 `deploy/requirements.txt` 内部拷贝；`USER 10001:10001`；保留 8765；非 root + read-only-friendly |
| `deploy/docker-compose.yml` | `build: .` 把上下文设到 `deploy/`，Dockerfile 找不到顶层 `requirements.txt` 和 `apeireth/`；端口/环境变量/探针端口 / 8765 散乱 | `build: { context: '..', dockerfile: 'deploy/Dockerfile' }`；`init: true`；统一 `V1075_PORT=8765`；healthcheck 显式指向 8765 |
| `deploy/k8s-asi.yaml` | 无 resources requests；无 securityContext（默认 root + writable rootfs）；无 rolling update；selector 与 Service selector 对齐未校验；缺 readiness 失败阈值一致性 | 加 requests/limits、`runAsNonRoot: true` + `runAsUser: 10001` + `seccompProfile: RuntimeDefault` + `readOnlyRootFilesystem: true` + drop ALL；strategy `RollingUpdate` + `maxUnavailable: 0, maxSurge: 1` + `revisionHistoryLimit: 3`；显式 `imagePullPolicy: IfNotPresent` |
| `deploy/requirements.txt` | 文件不存在 | 新建；只 pin `fastapi==0.139.0` + `uvicorn==0.51.0`（与本机真实运行版本一致） |

旧 canonical 文件已被 V1075 生成器 `v1075_asi_real_deployment_run.py` 覆盖为旧版模板属于历史副作用；本轮只替换 `deploy/` 四个文件，不再触发 V1075 模板以避免回退。

## 3. V1132 validator 语义门禁

修改文件：`apeireth/v1132_real_deployment_validator.py` + `tests/test_v1132_real_deployment_validator.py`（保留其他成员 R11-SEC-001 SSRF 强化不变）。

新增字段：
- `canonical_bundle_valid`: 18 项跨文件语义断言（pinned base、构建上下文、镜像名、端口、探针 selector、非 root、rollingUpdate、healthcheck path/port 等）。
- `offline_valid`: 静态 + 子进程验证，**不声明容器在跑**。
- `runtime_valid`: offline_valid + daemon 可达 + canonical /health 真实返回。
- `passed`: 与历史兼容，仅当 `runtime_valid` 为真才为真。
- `_LOOPBACK_PORTS` 新增 8765，使 canonical 探针不被 SSRF 白名单拒掉。
- 健康探测收敛到 `http://127.0.0.1:8765/health` 一项；历史生成器端口只作为 offline 渲染证据，不再误报“runtime expected fail”。

执行顺序：`docker_daemon_probe` → `compose_files` → `v1008_render` → `v1032_render` → `canonical_bundle` → `consistency` → `health_probes`。

新增测试：
- `test_validator_canonical_bundle_is_semantically_consistent`：要求 `canonical_bundle_valid` 与 `offline_valid` 同时为真。
- `test_validator_offline_success_does_not_claim_runtime_success`：在 daemon 不可达或 health probe 失败时，要求 `runtime_valid=False`，`passed=False`。

## 4. 真探测与运行时证据

### 4.1 daemon 探测（reports/r11-devops-daemon-probe.txt）

```
docker_path=MISSING
docker_info_rc=127
docker_compose_path=MISSING
kubectl_path=MISSING
kubectl_client_rc=127
```

含义：本机既无 docker CLI 也无 docker-compose / kubectl；容器层验证不能在此机执行。

### 4.2 validator 离线门禁（reports/r11-devops-validator-output.md）

```
docker_daemon_available: False
compose_files_parsed: 2
services_seen: 14
k8s_manifests_ok: 3
dockerfile_valid: 2
subprocess_runs_ok / failed: 2 / 0
health_probes_ok / failed: 0 / 1
canonical_bundle_valid: True
offline_valid: True
runtime_valid: False
passed: False
```

`canonical_bundle` 18/18 通过；其余历史生成器与 r8 compose 仍可解析（保留为兼容证据）。

### 4.3 V1075 进程 fallback（reports/r11-v1075-process/deployment-report.md）

```
State: STOPPED
Mode: process
Port: 8875
PID: 20408
Health: status=200 latency=1150.4ms attempt=1
Body: {"status":"ok","service":"apeireth-asi","version":"0.1.0",...}
```

起停链路 1.17s，5/6 真实阶段全过；该结果只证明应用入口可执行，不证明 Docker/K8s 上线。

## 5. 交付 SHA-256

```
fd0e12e8... deploy/Dockerfile
2a19afa4... deploy/docker-compose.yml
d31d39b8... deploy/k8s-asi.yaml
9838fd3e... deploy/requirements.txt
8b1ba179... reports/r11-devops-validator-output.md
0a1b2fc7... reports/r11-devops-daemon-probe.txt
23d32492... reports/r11-v1075-process/deployment-report.md
```

完整清单见 `reports/r11-devops-sha256.txt`。

## 6. 发布窗口 / 回滚 / 监控告警

### 6.1 发布窗口

1. PR 合入 master 之前必须通过本报告 §3 的 canonical 18 项语义断言。
2. 在具备 docker 的节点上跑：
   ```bash
   python -m apeireth.v1132_real_deployment_validator
   docker compose -f deploy/docker-compose.yml up --build -d
   curl -fsS http://127.0.0.1:8765/health
   ```
3. 监控三项必须为真：`offline_valid=True`、`runtime_valid=True`、`probe[canonical-v1075]` passed。

### 6.2 回滚

- **代码层**: `git revert <commit>` + 重新跑 §3 validator（canonical 18 项）。
- **容器层**: `docker compose -f deploy/docker-compose.yml down` 或 K8s `kubectl rollout undo deployment/apeireth-asi`；保留 `revisionHistoryLimit: 3` 以便回滚三版。
- **服务层**: systemd `systemctl revert apeireth-asi` 或 supervisor `supervisorctl reread && supervisorctl update`。

### 6.3 监控告警

| 指标 | 阈值 | 触发动作 |
|------|------|---------|
| 8765 /health 5xx 或超时 | ≥3 连续失败 | 触发 deployment alert；自动 `kubectl rollout undo` |
| 启动延迟 (P95) | > 3s | 容量告警；CPU/memory requests 上调 |
| 启动探针失败 | 12 × 5s 失败 | 滚动告警；锁定 Pod 进入 CrashLoop |
| OOMKilled | ≥1 / 24h | 上调 memory limits 至 1Gi；归档根因 |

## 7. 与历史缺陷的对账

- §9 G “k8s manifest 完整 (V1008 衔接)”：本轮 canonical `k8s-asi.yaml` 已具 revisionHistoryLimit、RollingUpdate、resources、securityContext、readiness/liveness/startup probes，且 selector 与 pod labels / Service selector 显式一致。
- V1132 旧“0/4 health probes 真通过”由 daemon 不可用导致：现明确收敛到 canonical 单端点，并由 `runtime_valid` 与 `passed` 严格门禁，避免离线冒充运行。
- SSRF 安全门：`_LOOPBACK_PORTS` 加入 8765 后，canonical 探针可执行，外部 host/port 仍被拒绝。

## 8. 未启动事项（透明）

- Docker/K8s 上线验证不在本机进行；需在具 daemon 的节点上重跑 §4.2 命令。
- 9 选 1 的 k8s 节点类型（kind / minikube / 自建）由 DevOps 决策；本轮提供 manifest 模板与离线门禁作为唯一保证。
- `prometheus` + `grafana` 接入仍在 R10 后台，未触动。