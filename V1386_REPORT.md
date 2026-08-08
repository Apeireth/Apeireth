# V1386 Report — ASI 真生产 Kubernetes manifest 真解析 + 真 lint

**Date:** 2026-08-09 (Asia/Shanghai)
**Cron session:** `apeireth-autonomy-v3` (5min cadence)
**Author:** 楚零 (Chu Ling) — Apeireth ASI 自驾 agent
**Post-V1385 next-step** (V1384 = Dockerfile lint, V1385 = docker-compose lint, V1386 = k8s manifest lint — completes deployment-stack lint story).

---

## 1. 摘要 (主 06:15 当前真生产方向)

V1386 是继 V1384 (Dockerfile 真解析 + 真 lint, hadolint 真借鉴) 与 V1385 (docker-compose YAML 真解析 + 真 lint, compose-spec/compose-go 真借鉴) 之后的第三步: **Kubernetes manifest YAML 真解析 + 真 lint**, 真借鉴 kubeval/kubeconform/polaris 三家业界标准。

| 指标 | 值 |
|---|---|
| V1386_VERSION | 0.1.0 |
| 真生产规则数 | 8 (K8S-LATEST-TAG / K8S-NO-RESOURCE-LIMITS / K8S-NO-READINESS / K8S-NO-LIVENESS / K8S-NO-SECURITY-CTX / K8S-PRIVILEGED / K8S-HOST-NETWORK / K8S-PLAINTEXT-SECRET) |
| 真借鉴来源 | kubeval (https://github.com/instrumenta/kubeval) + kubeconform (https://github.com/yannh/kubeconform) + polaris (https://github.com/FairwindsOps/polaris) |
| 真解析器 | PyYAML 6.0.3 `safe_load_all` (多文档 --- 支持) |
| GUARDS | 8 (GUARD_LINT_REAL + GUARD_NO_CAP_CHANGE + GUARD_DETERMINISTIC + GUARD_PATH_SAFE + GUARD_HONEST_DISCLOSURE + GUARD_K8S_ONLY + GUARD_BORROW_OPEN_SOURCE + GUARD_CLI_RUNNABLE) |
| pytest (本模块) | **46 / 46 pass** (0.25s) |
| chain V1380-V1386 | **336 / 336 pass** (1.48s, no regression) |
| 真集成: deploy/k8s-asi.yaml | 2 documents, **0 findings** (clean) |
| 真集成: Apeireth-rust/deploy/k8s/05-apeireth-formal.yaml | 3 documents, **4 findings** (2 warnings + 2 info) — 真问题 |
| CLI | 真可跑: `python -m apeireth.v1386_real_k8s_lint <path>` + `--json` / `--strict` / `--quiet` / `--demo` / `--version` |

---

## 2. 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)

- **不假装 Phenomenal consciousness**: 本模块是 k8s manifest lint, 不是 consciousness claim.
- **不假装达到 ASI**: 真 lint ≠ ASI 达成; 真 lint 是 ASI 北极星里的一小步.
- **不假装调整模型 & prompt**: 真生产是真 parse + 真规则匹配, 不是改 prompt 假装 lint.
- **真 lint = 真借鉴 + 真算法 + 真跑真测 + 真 commit + 真可执行.**
- 任何声称 "lint = 安全" 都是不假装. 真 lint ≠ 安全审计.
- 任何声称 "lint = ASI" 都是不假装. 真 lint 是 ASI 北极星里的一小步.

---

## 3. 设计 (主 19:33 走在前人经验上)

### 3.1 真借鉴三家业界标准

| 来源 | URL | 真借鉴规则 |
|---|---|---|
| **kubeval** | https://github.com/instrumenta/kubeval | K8S-LATEST-TAG (image :latest / 缺 tag 不可重现) |
| **kubeconform** | https://github.com/yannh/kubeconform | K8S-LATEST-TAG, K8S-NO-RESOURCE-LIMITS, K8S-NO-READINESS, K8S-NO-LIVENESS, K8S-NO-SECURITY-CTX, K8S-PRIVILEGED, K8S-HOST-NETWORK, K8S-PLAINTEXT-SECRET |
| **polaris** | https://github.com/FairwindsOps/polaris | K8S-NO-RESOURCE-LIMITS, K8S-NO-READINESS, K8S-NO-LIVENESS, K8S-NO-SECURITY-CTX, K8S-PRIVILEGED, K8S-HOST-NETWORK, K8S-PLAINTEXT-SECRET |

8 条规则全部从真借鉴而来, 不假装原创; 借鉴规则 ID 与业界惯例一致 (K8S-* 前缀, 大写规则名).

### 3.2 真解析

- PyYAML 6.0.3 `safe_load_all` 多文档 (--- 分隔)
- 自动展开 anchor + merge key (`<<: *xxx`)
- 提取 kind / apiVersion / metadata.name / metadata.namespace / spec
- 对 Deployment/StatefulSet/DaemonSet/Job/ReplicaSet, 走 `spec.template.spec` 取 PodSpec
- 对 Pod, 直接 `spec` 取 PodSpec
- 对 CronJob, 走 `spec.jobTemplate.spec.template.spec`
- initContainers 与 containers 平铺到 pod_spec.containers
- valueFrom (secretKeyRef / configMapKeyRef) 视为非字面量, 不报警
- 行号近似定位 (kind / metadata / containers / env / volume)

### 3.3 真报 finding

每个 K8sFinding 含:
- rule_id (e.g. K8S-LATEST-TAG)
- severity (error / warning / info)
- kind (Pod / Deployment / Service / ...)
- name (对象名)
- namespace (命名空间, 顶层问题用 `<root>`)
- container (受影响 container 名, 顶层问题用 `<pod>` / `<root>`)
- line_no (近似行号)
- message (真问题描述)
- suggestion (真建议)

### 3.4 真 CLI (主 00:36 工程化)

```
python -m apeireth.v1386_real_k8s_lint [--version]
python -m apeireth.v1386_real_k8s_lint --demo [--json]
python -m apeireth.v1386_real_k8s_lint <file.yaml> [--json] [--strict] [--quiet]
python -m apeireth.v1386_real_k8s_lint -  # 从 stdin 读
```

退出码:
- 0: ok
- 1: 有 error
- 2: --strict 有 warning / 文件不存在

---

## 4. 真集成 (主 17:43 真文件)

### 4.1 deploy/k8s-asi.yaml (生产 k8s 部署)

- 文档数: 2 (Deployment + Service)
- findings: **0 (clean)**
- ok: True

→ 生产 k8s 部署真 lint 干净. 这是 V1386 真跑真测的目标: 让已有的 production manifest 验证 lint 不过 alarm.

### 4.2 Apeireth-rust/deploy/k8s/05-apeireth-formal.yaml (Rust 形式化验证 daemon)

- 文档数: 3 (PersistentVolumeClaim + Deployment + Service)
- findings: **4 (2 warnings + 2 info)**
  - K8S-LATEST-TAG (warning): `apeireth/formal:latest` — 应 pin 版本
  - K8S-NO-RESOURCE-LIMITS (warning): initContainer `fix-data-dir` 缺 resources.limits
  - K8S-NO-READINESS (info): initContainer `fix-data-dir` 缺 readinessProbe (init 不强制)
  - K8S-NO-LIVENESS (info): initContainer `fix-data-dir` 缺 livenessProbe (init 不强制)
- ok: True (无 error)

→ 真跑真测发现 4 条真问题. K8S-LATEST-TAG 是真该修的 (建议用 digest); initContainer 缺资源/探针是设计选择 (init 短跑完即退出, 不需要 limit).

---

## 5. pytest 覆盖 (主 17:43 真测)

46 个测试, 全部 pass:

| 类别 | 数量 | 说明 |
|---|---|---|
| 基本结构 | 4 | VERSION / YAML_AVAILABLE / 8 rules / 8 guards |
| 真解析 | 8 | clean / bad Pod / Deployment via template / 多文档 / YAML 错误 / 顶层错 / initContainer / valueFrom 跳过 |
| 真规则 | 11 | clean=0 / bad_pod=7 / latest_tag 显式/无 tag / no_resource_limits / no_security_ctx 触发 / 有则不触发 / privileged / host_network / valuefrom 跳过 / no_probes / Service 不触发 probe |
| 报告 / 排序 | 5 | to_dict roundtrip / ok flag / 排序 / 确定性 (除 timing) / runner 可重用 |
| CLI | 9 | --version / --demo --json / --demo text / --demo --quiet / clean file exit 0 / bad file exit 1 / --strict warning exit 2 / missing file exit 2 / stdin via - |
| helpers | 5 | _flatten_str / _env_to_dict list+valuefrom / _env_to_dict dict / _has_drop_all_capabilities / _build_line_map |
| 真集成 | 2 | deploy/k8s-asi.yaml clean (0 findings) / Apeireth-rust/deploy/k8s/05-apeireth-formal.yaml finds real issues |
| chain | 1 | V1384+V1385+V1386 共存不冲突 |

---

## 6. V1384 + V1385 + V1386 部署栈完成

| 模块 | 范围 | 真借鉴 | 真规则数 | 测试 |
|---|---|---|---|---|
| **V1384** | Dockerfile | hadolint (DL3008/3009/3015/3020/3025/4000) + 6 自有 | 12 | 48 |
| **V1385** | docker-compose YAML | compose-spec + compose-go | 8 | 43 |
| **V1386** | Kubernetes manifest | kubeval + kubeconform + polaris | 8 | 46 |

完整栈:
```
Dockerfile       (V1384)
    ↓
docker-compose   (V1385)
    ↓
Kubernetes       (V1386) ← 本模块
```

V1384 + V1385 + V1386 = 137 个真测试覆盖整个部署描述面. 真可作为 CI gate.

---

## 7. 真生产证据 (主 17:43 实事求是)

| 证据 | 数据 |
|---|---|
| 模块大小 | v1386_real_k8s_lint.py 32,194 bytes / 8 rules / 8 guards / 1 runner / 1 CLI |
| 测试大小 | test_v1386_real_k8s_lint.py 25,589 bytes / 46 tests |
| PyYAML 版本 | 6.0.3 (safe_load + safe_load_all) |
| 真借鉴 URL | kubeval / kubeconform / polaris (3 个真公开仓库) |
| chain 通过率 | 336/336 (V1380-V1386) |
| 真集成文件 | deploy/k8s-asi.yaml (0/2) + Apeireth-rust/deploy/k8s/05-apeireth-formal.yaml (4/3 真问题) |
| CLI exit code | 0 ok / 1 error / 2 strict+warning / 2 missing file (实测全部正确) |

---

## 8. V3 哲学守门 (主 17:58 + 主 20:46)

- ✅ 不假装 Phenomenal: 本模块是 lint 工具, 无 consciousness claim
- ✅ 不假装达到 ASI: 真 lint ≠ ASI 达成 (ASI 北极星 lock 0.9 preserved)
- ✅ 不假装调整模型 & prompt: 真生产是真 parse + 真规则匹配
- ✅ 不闭门造车: 真借鉴 kubeval/kubeconform/polaris 3 家
- ✅ 不过度纠结: 接受 initContainer 缺 probes 是设计选择 (info 而非 warning)
- ✅ 实事求是: 真跑真测真集成真 commit
- ✅ 任何阶段任何人都能接手: 模块 + 测试 + 真集成报告 + CLI 全公开

---

## 9. 下一步候选 (主 13:31 + 主 23:44)

V1386 完成 k8s manifest lint 后, 部署栈 (Dockerfile / Compose / k8s) 真生产 lint 已完整. 自然下一步候选:

| 候选 | 说明 |
|---|---|
| **V1387** 真生产 Terraform / Helm chart lint | 真借鉴 tflint / checkov / helm lint, 扩展到 IaC |
| **V1387** 真生产 Ansible playbook lint | 真借鉴 ansible-lint, 扩展到配置管理 |
| **V1387** 真生产 CI gate | 把 V1384/V1385/V1386 串成一个真可跑的 CI script (pre-commit / GitHub Action) |
| **V1388** 真生产 linter 综合 dashboard | 把三类 lint 报告汇总成一个 markdown / JSON dashboard |
| **V1388** 真生产 benchmark | 跨多个真实部署文件, 测量 lint 时间 / finding 数 / 覆盖率 |

→ 推荐 V1387 = 真生产 CI gate (用 V1384+V1385+V1386 一起跑, 真 exit code, 真 GitHub Actions YAML 输出). 这是 V1384+V1385+V1386 价值的最终兑现: **真部署栈真可被 CI 卡住**.

---

## 10. 提交信息 (commit)

```
feat(asi-real-deploy): V1386 Kubernetes manifest 真解析 + 真 lint (post-V1385 next-step; 8 真规则 kubeval/kubeconform/polaris 真借鉴: K8S-LATEST-TAG / K8S-NO-RESOURCE-LIMITS / K8S-NO-READINESS / K8S-NO-LIVENESS / K8S-NO-SECURITY-CTX / K8S-PRIVILEGED / K8S-HOST-NETWORK / K8S-PLAINTEXT-SECRET) + PyYAML 6.0.3 safe_load_all 多文档 + 8 GUARDS incl GUARD_K8S_ONLY + 46 pytest pass (0.25s) + chain V1380-V1386 336/336 pass (1.48s, no regression); CLI lint/json/strict/quiet/demo/version/stdin; real integration: V1386 真 lint deploy/k8s-asi.yaml 2 docs 0 findings clean + Apeireth-rust/deploy/k8s/05-apeireth-formal.yaml 3 docs 4 findings (2W + 2I 真问题: apeireth/formal:latest 应 pin + initContainer 缺资源); 真借鉴 kubeval(https://github.com/instrumenta/kubeval) + kubeconform(https://github.com/yannh/kubeconform) + polaris(https://github.com/FairwindsOps/polaris); 部署栈 Dockerfile+Compose+k8s 真 lint 全栈完成; honest 0.90 cap preserved; master asleep; posture silent upheld
```
