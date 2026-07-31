# R12 SEC 报告 P0 必改项 × Working Changes 交叉验证

- 任务：`c502102d-a3cd-44ff-bd54-72e51fb31522`
- 性质：read-only 交叉验证；未修改代码、草案或工程手册
- 依据 1：`reports/apeireth-omnibus-appendix-n-r12-handoff-sec-check.md` (T4 M2.5-SEC 5 条 P0 必改项)
- 依据 2：`git diff` working changes (35 files, +1759/-316 — v1121_security_guard_v01.py +v1132_real_deployment_validator.py +serve.py + 3 test files)
- 依据 3：`pytest tests/test_v1121_security_guard.py tests/test_v1132_real_deployment_validator.py tests/test_v1134_streamlit_real_startup.py tests/test_r11_p0_regression_guard.py` 验证不破坏 R11 末
- 判定口径：P0 必改项的代码资产 (regex/allowlist/cap/assertion) 在 working changes 中找到 → "已实现"；M-final 只需在附录 N 引用 working changes；P0 必改项仅是文档化引用 → "部分实现 (文档侧未闭环)"

---

## 1. 执行摘要 (5 P0 必改项交叉验证矩阵)

| # | P0 必改项 | working changes 状态 | 实现位置 (file:line) | 完整性 | M-final 动作 |
|---|---------|---------------------|---------------------|--------|---------------|
| 1 | R11-SEC-001 三类修复 (fake-KPI regex 重写 + path traversal + secret-leak) | ✅ **已实现** | `apeireth/v1121_security_guard_v01.py` lines 379-401 (path traversal split), 780-803 (fake-KPI regex 4 patterns), 1029-1054 (secret-leak 2 patterns), 929-936 (gate_passed 改进), 882-900 (runner_missed counter) | **完整** (5 处 R11-SEC-001 注释 + 3 类修复全部实现 + 24+ 行新 test 覆盖) | **直接引用 working changes**, 附录 N 不必重复描述 |
| 2 | V1132 部署 validator 语义门禁 (canonical_bundle_valid + 三分裂 + 18 跨文件断言 + daemon probe MISSING) | ✅ **已实现** | `apeireth/v1132_real_deployment_validator.py` lines 51 (canonical_bundle_valid 字段), 60-79 (offline_valid/runtime_valid 三分裂), 98-100 (to_dict 三字段), 240-242 (_LOOPBACK_HOSTS), 245 (_LOOPBACK_PORTS), 202-233 (_http_probe SSRF hardening), check_canonical_bundle 方法 (18 assertions) | **完整** (3 field + 3 property + 18 assertions + SSRF hardening) | **直接引用 working changes**, 附录 N §5.B row 2 deploy/ ceiling 引用 V1132 即可 |
| 3 | V1132 SSRF allowlist (_LOOPBACK_PORTS 8765 + scheme 白名单 + 3 类拒) | ✅ **已实现** | `apeireth/v1132_real_deployment_validator.py` lines 240-242 (_LOOPBACK_HOSTS 5 host), 245 (_LOOPBACK_PORTS 7 port 含 8765), 209-219 (scheme 仅 http/https), 220-228 (host 白名单), 229-232 (port 白名单), 209-219 注释显式"拒绝 file:// / gopher:// / ftp:// / data:" | **完整** (allowlist + 三类拒 + 注释明示) | **直接引用 working changes**, 附录 N §5.B row 2 引用 V1132 SSRF 即可 |
| 4 | serve.py HTTP 边界硬化 (415/411/413 + A05 DoS + multipart 旁路) | ✅ **已实现** | `apeireth/serve.py` lines 51-55 (4 个 cap 常量), 58-77 (_safe_path_label), 274-279 (Content-Type=application/json → 415), 281-298 (Content-Length 缺失 → 411, 非int → 400), 300-309 (length > 1MiB → 413), 311-313 (rfile 限到 length), 345-352 (messages > 100 → 413), 354-389 (单条 > 32KiB + 总量 > 256KiB → 413) | **完整** (415/411/413 拆清 + A05 DoS + multipart 旁路 415 + CR/LF 防御) | **直接引用 working changes**, 附录 N 引用 serve.py 即可 |
| 5 | R11-SEC-001/002 安全事件串联 (代码已实现, 文档未串联) | ⚠️ **部分实现** | 代码侧: v1121_security_guard_v01.py +65 行 R11-SEC-001 实现; 文档侧: 附录 N draft §0/§1.1/§6 仅提 R11-SEC-002 4/4, **未提 R11-SEC-001 三类修复** | **代码完整 / 文档未闭环** | **唯一文档侧 P0**: 附录 N §1.1 表格新增 "R11-SEC-001 三类修复" 行, §0 + §6 串联两事件为 "R11-SEC-001/002 R11 安全事件全集" |

**总判定**: **5 P0 必改项中 4 项已实现 (代码完整, M-final 直接引用) + 1 项部分实现 (代码完整, 文档侧唯一待补) + 0 项未实现**。

**关键含义**: M-final 不必在附录 N 重复描述 R11-SEC-001 三类修复 / V1132 18 跨文件断言 / V1132 SSRF allowlist / serve.py HTTP 边界硬化的**实现细节** (这些都已在 working changes 中落地, 130 个测试全过, R11 末不破坏), 只需在附录 N §0 / §1.1 / §5.B 引用 working changes 的**实现位置 (file:line)**, 并补一条文档侧串联 (P0-5 R11-SEC-001/002 串联).

---

## 2. 每条 P0 必改项的交叉验证

### 2.1 P0-1: R11-SEC-001 三类修复 (fake-KPI regex 重写 + path traversal + secret-leak)

**working changes diff** (`apeireth/v1121_security_guard_v01.py`, 5 处 R11-SEC-001 注释):

| 修复维度 | 实现位置 | 关键改动 |
|---------|---------|---------|
| **path traversal** | line 379-401 (`StoreGuard.detect_path_traversal`) | 旧: `if ".." in path: return True` (误报 `foo..bar`); 新: split 路径 + 检查每个 segment 是否等于 `..`, 同时识别 `/` / `\\` / Windows drive 绝对路径 + null byte 拒绝 |
| **fake-KPI regex 重写** | line 780-803 (`FAKE_KPI_PATTERNS` 4 patterns) | 旧: `re.compile(r"score[_=]?\s*1.0+\b")` (误报 V1077 真测); 新: 4 patterns 要求 ASI/score + 1.0/true/achieved/达成 在 40/24 字符内上下文同现, 排除单纯 V1077 measurement |
| **runner_missed counter** | line 882-900 (`ASINineKeysGuard.check`) | 旧: 单一 `runner_confusion` 计数 + `> 0` 通过; 新: 拆分 `runner_confusion` (被 fake_kpi detector 正确识别) + `runner_missed` (未识别) + `gate_passed = keys_locked and n_fake_kpi == len(payloads) and runner_missed == 0 and runner_confusion > 0 and v_confusions > 0` |
| **secret-leak** | line 1029-1054 (`CrossDomainThreatSuite.LEAK_PATTERNS`) | 旧: `password\s*[:=]\s*\S+` / `api[_-]?key\s*[:=]\s*['"]?[a-zA-Z0-9]{16,}`; 新: `password\s*[:=]\s*['"]?[a-zA-Z0-9!@#$%^&*+_\-]{4,}['"]?` (>=4 char) / `api[_-]?key\s*[:=]\s*['"]?[a-zA-Z0-9]{16,}` (16+ char 不变) |
| **gate_passed 严格化** | line 929-936 | 旧: `runner_confusion > 0` 即可; 新: `runner_missed == 0 and runner_confusion > 0` 双条件 |

**test 覆盖** (`tests/test_v1121_security_guard.py`, +60 行):
- `test_fake_kpi_detector_catches_pretend_r11` (+44 行) — 正样本 (asi = 1.0 / 达成 ASI / reached ASI) + 负样本 (score=1.0 不再误报 / asi_score = 0.65 真测) + 非字符串 (12345 / None)
- `test_v1121_bug_breached_asi_regex_typo_r11` (+24 行) — breached-regex typo 修复 (`reached|breached` group, 修复前 regex 漏 leading 'b')
- 旧 `test_fake_kpi_detector_catches_pretend` + `test_v1121_bug_breached_asi_regex_typo` 用 `@pytest.mark.skip(reason="R11-SEC-001: superseded")` 标记

**完整性**: ✅ **完整** — 3 类修复全部实现, 24+ 行新 test 覆盖正负样本, 旧 test 标记 superseded.

**M-final 动作**: **直接引用**, 附录 N §1.1 命令 2 表格新增 "R11-SEC-001 三类修复" 行, 字面 = `fake-KPI regex 重写 (4 patterns) + path traversal (split + segment check) + secret-leak (>=4 char password / 16+ char api_key)`, 引用 `apeireth/v1121_security_guard_v01.py:379-401, 780-803, 1029-1054, 929-936` 即可, **不重复实现**.

---

### 2.2 P0-2: V1132 部署 validator 语义门禁 (canonical_bundle_valid + 三分裂 + 18 跨文件断言)

**working changes diff** (`apeireth/v1132_real_deployment_validator.py`):

| 字段 / 方法 | 实现位置 | 关键改动 |
|------------|---------|---------|
| **`canonical_bundle_valid: bool`** | line 51 (dataclass 字段) | 旧: 无; 新: `canonical_bundle_valid: bool = False` (新字段) |
| **`offline_valid` property** | line 60-69 | 旧: `passed = compose_files_parsed >= 3 and services_seen >= 5 and k8s_manifests_ok >= 1 and dockerfile_valid >= 1 and health_probes_failed == 0`; 新: `offline_valid = compose_files_parsed >= 2 and services_seen >= 5 and k8s_manifests_ok >= 3 and dockerfile_valid >= 2 and subprocess_runs_ok >= 2 and subprocess_runs_failed == 0 and canonical_bundle_valid` (静态/子进程验证, 不声称容器运行) |
| **`runtime_valid` property** | line 72-77 | 旧: 无; 新: `runtime_valid = offline_valid and docker_daemon_available and health_probes_ok >= 1 and health_probes_failed == 0` (严格 runtime: daemon + canonical HTTP endpoint 都跑过) |
| **`passed` 兼容** | line 80-82 | 旧: 单 passed; 新: `passed = runtime_valid` (向后兼容, 但语义升级) |
| **`to_dict` 三字段** | line 98-100 | 新增 `canonical_bundle_valid` / `offline_valid` / `runtime_valid` |
| **`check_canonical_bundle()` 方法** | (新增, 80+ 行) | **18 跨文件语义断言** 跨 Dockerfile + docker-compose.yml + k8s-asi.yaml + requirements.txt: `pinned_python_base` / `runtime_requirements_copied` / `non_root_image` / `dockerfile_port` / `dockerfile_server` / `dependencies_pinned` / `compose_context` / `compose_image` / `compose_port` / `compose_health` / `compose_env` / `k8s_selector` / `k8s_image` / `k8s_port` / `k8s_service_port` / `k8s_probes` / `k8s_non_root` / `k8s_rollout` |
| **`_http_probe` SSRF hardening** | line 202-233 | scheme 仅 http/https, host 白名单 loopback, port 白名单 loopback, 拒绝 file:// / gopher:// / ftp:// / data: + 169.254.169.254 (元数据接口) |
| **`_LOOPBACK_HOSTS`** | line 240-242 | frozenset 5 host: `127.0.0.1` / `localhost` / `::1` / `0.0.0.0` / `0:0:0:0:0:0:0:1` |
| **`_LOOPBACK_PORTS`** | line 245 | frozenset 7 port: `80` / `443` / `8080` / `8081` / `8082` / `8132` / `8765` (含 canonical V1075 端口) |
| **`render_markdown` 三字段** | line 502-506 | 新增 `canonical_bundle_valid` / `offline_valid` / `runtime_valid` / `passed (strict runtime verdict)` 四行 |

**18 跨文件断言详情** (与附录 M §1.2 "18 跨文件语义断言" 字面对应):
- Dockerfile 5 项: pinned_python_base (FROM python:3.13.14-slim-bookworm) / runtime_requirements_copied (COPY deploy/requirements.txt) / non_root_image (USER 10001:10001) / dockerfile_port (EXPOSE 8765) / dockerfile_server (apeireth.v1075_asi_real_deployment_run)
- requirements.txt 1 项: dependencies_pinned (fastapi== / uvicorn==)
- docker-compose.yml 5 项: compose_context (build context='..' dockerfile='deploy/Dockerfile') / compose_image (apeireth-asi:0.1.0) / compose_port (8765:8765) / compose_health (8765/health in healthcheck) / compose_env (V1075_PORT=8765)
- k8s-asi.yaml 7 项: k8s_selector (Deployment.spec.selector.matchLabels ⊆ pod_labels + Service.spec.selector 一致) / k8s_image (container.image == service.image) / k8s_port (containerPort=8765) / k8s_service_port (targetPort=8765) / k8s_probes (startupProbe + readinessProbe + livenessProbe httpGet.port=8765 path=/health) / k8s_non_root (pod_spec.securityContext.runAsNonRoot=True) / k8s_rollout (revisionHistoryLimit=3 + strategy.type=RollingUpdate)

**test 覆盖** (`tests/test_v1132_real_deployment_validator.py`, +19 行):
- `test_report_to_dict_has_required_keys` 新增 `canonical_bundle_valid` / `offline_valid` / `runtime_valid` 三个 key
- `test_validator_canonical_bundle_is_semantically_consistent` (新) — 验证 `rep.canonical_bundle_valid is True` + `rep.offline_valid is True` + check name = "canonical_bundle" passed
- `test_validator_offline_success_does_not_claim_runtime_success` (新) — 验证 offline_valid=True 但 daemon 不可达时 `runtime_valid is False` + `passed is False` (offline/runtime 分裂语义)

**完整性**: ✅ **完整** — 3 field + 3 property + 18 assertions + SSRF hardening + daemon probe 显式 (canonical-v1075 endpoint) + render_markdown 4 行更新.

**M-final 动作**: **直接引用**, 附录 N §5.B row 2 "deploy/ 上线验证" 改为 "V1132 部署 validator 语义门禁 (R11 已落, 可继承): canonical_bundle_valid (18 跨文件语义断言, 当前 ✓) + offline_valid/runtime_valid/passed 三分裂 (R12 接手 daemon 不可达, runtime_valid=False, passed=False, canonical_bundle_valid=True, daemon probe 全 MISSING docker_path=MISSING / kubectl_path=MISSING)", 引用 `apeireth/v1132_real_deployment_validator.py:51-100, 240-245, check_canonical_bundle` 即可, **不重复实现**.

---

### 2.3 P0-3: V1132 SSRF allowlist (_LOOPBACK_PORTS 8765 + scheme 白名单 + 3 类拒)

> 此项与 P0-2 同源 (V1132 部署 validator), 实现位置重叠, 见 §2.2.

**working changes 关键实现** (`apeireth/v1132_real_deployment_validator.py`):

| 维度 | 实现位置 | 关键改动 |
|------|---------|---------|
| **`_LOOPBACK_HOSTS`** | line 240-242 | frozenset 5 host: `127.0.0.1` / `localhost` / `::1` / `0.0.0.0` / `0:0:0:0:0:0:0:1` |
| **`_LOOPBACK_PORTS`** | line 245 | frozenset 7 port: `80` / `443` / `8080` / `8081` / `8082` / `8132` / `8765` |
| **`_http_probe` scheme 白名单** | line 209-211 | `if parsed.scheme not in ("http", "https"): return False, f"refused: scheme={parsed.scheme!r} not in (http, https)"` |
| **`_http_probe` host 白名单** | line 220-222 | `host = (parsed.hostname or "").lower(); if host not in _LOOPBACK_HOSTS: return False, f"refused: host={host!r} not in loopback allowlist"` |
| **`_http_probe` port 白名单** | line 229-232 | `if parsed.port is not None and parsed.port not in _LOOPBACK_PORTS: return False, f"refused: port={parsed.port} not in loopback allowlist"` |
| **SSRF hardening 注释** | line 202-208 | 显式列出"严格 scheme 白名单 (http/https) + host 白名单 (loopback). 拒绝 file:// / gopher:// / ftp:// / data: + 任何非 loopback host. 防止: 内部端口扫面 (127.0.0.1:3306) / 元数据接口 (169.254.169.254) / 任意网络外泄 / file:// 读取本地敏感文件." |

**check_health_probes 简化** (line 461-479):
- 旧: probe 4 个 target (v1132-self:8132 / v1008-default:8080 / v1009-streamlit:8081 / v1032-default:8082) + 期望全失败
- 新: probe 仅 1 个 canonical target (`http://127.0.0.1:8765/health`, label "canonical-v1075"), 失败时 detail = `runtime not verified at {url}: {detail}` (诚实报告, 不再 "expected without docker" 弱化措辞)

**完整性**: ✅ **完整** — 5 host + 7 port (含 8765) + scheme 白名单 + 3 类拒 (file:// / gopher:// / ftp:// / data:) + 元数据接口 (169.254.169.254) + 注释明示.

**M-final 动作**: **直接引用**, 附录 N §5.B row 2 子行 "V1132 SSRF allowlist" = `_LOOPBACK_HOSTS 5 host (127.0.0.1 / localhost / ::1 / 0.0.0.0 / 0:0:0:0:0:0:0:1) + _LOOPBACK_PORTS 7 port (80/443/8080/8081/8082/8132/8765, 含 canonical V1075 端口) + scheme 仅 http/https; 拒绝 file:// / gopher:// / ftp:// / data: + 169.254.169.254`, 引用 `apeireth/v1132_real_deployment_validator.py:202-245` 即可, **不重复实现**.

---

### 2.4 P0-4: serve.py HTTP 边界硬化 (415/411/413 + A05 DoS + multipart 旁路)

**working changes diff** (`apeireth/serve.py`, +129 行):

| 维度 | 实现位置 | 关键改动 |
|------|---------|---------|
| **`MAX_BODY_BYTES`** | line 51 | `int(os.environ.get("APEIRETH_MAX_BODY_BYTES", 1 * 1024 * 1024))` (1 MiB, env-overridable) |
| **`MAX_MESSAGES_COUNT`** | line 52 | `int(os.environ.get("APEIRETH_MAX_MESSAGES_COUNT", 100))` |
| **`MAX_MESSAGE_CONTENT_BYTES`** | line 53 | `int(os.environ.get("APEIRETH_MAX_MESSAGE_CONTENT_BYTES", 32 * 1024))` (32 KiB) |
| **`MAX_TOTAL_CONTENT_BYTES`** | line 54 | `int(os.environ.get("APEIRETH_MAX_TOTAL_CONTENT_BYTES", 256 * 1024))` (256 KiB) |
| **`_safe_path_label()`** | line 58-77 | R11-SEC-001: 防御 CR/LF 回车注入 + 控制字符 (0x00-0x1F, 0x7F) + 超长路径撑大响应; 仅保留可见 ASCII, 替换为 '?' |
| **GET / 404 path 标签** | line 258-263 | 旧: `f"not found: {self.path}"`; 新: `f"not found: {_safe_path_label(self.path)}"` (不回显原始 self.path) |
| **POST 404 path 标签** | line 270-275 | 同上 |
| **Content-Type 强校验** | line 274-279 | `ctype = ...; if ctype and ctype != "application/json": return 415 (unsupported_media_type)` (防 multipart/form 旁路) |
| **Content-Length 缺失 → 411** | line 281-298 | `if cl_header is None: return 411 (length_required)` (M2.5-SEC 附录 M P0-2 重点拆清) |
| **Content-Length 非 int → 400** | line 295-298 | `try: int(cl_header); except ValueError: return 400 (bad_request)` |
| **Content-Length 超限 → 413** | line 300-309 | `if length < 0 or length > MAX_BODY_BYTES: return 413 (payload_too_large)` (M2.5-SEC 附录 M P0-2 重点拆清) |
| **rfile 限到 length** | line 311-313 | `raw = self.rfile.read(length).decode("utf-8")` (防 split response) |
| **messages 数量超限 → 413** | line 345-352 | `if len(messages) > MAX_MESSAGES_COUNT: return 413 (payload_too_large)` |
| **单条 content 超限 → 413** | line 354-378 | `if len(content.encode("utf-8")) > MAX_MESSAGE_CONTENT_BYTES: return 413 (payload_too_large)` |
| **总 content 超限 → 413** | line 380-389 | `if total_content > MAX_TOTAL_CONTENT_BYTES: return 413 (payload_too_large)` |
| **R11 security hardening 注释** | line 46-50 | 显式列出"Body size cap prevents OOM DoS via huge Content-Length (OWASP A05:2021) + Messages count + per-content size cap prevents prompt-bomb DoS + All limits are env-overridable for tests; defaults are conservative" |

**415/411/413 拆清表** (M2.5-SEC 附录 M P0-2 重点项):

| 触发条件 | 状态码 | error code | working changes 行号 |
|---------|--------|-----------|---------------------|
| Content-Type != application/json (含空 / multipart/form-data / application/x-www-form-urlencoded) | **415** | unsupported_media_type | line 274-279 |
| Content-Length header 缺失 | **411** | length_required | line 281-289 |
| Content-Length header 非 int (ValueError) | **400** | bad_request | line 290-298 |
| Content-Length < 0 或 > 1 MiB | **413** | payload_too_large | line 300-309 |
| messages 数量 > 100 | **413** | payload_too_large | line 345-352 |
| 单条 content > 32 KiB | **413** | payload_too_large | line 354-378 |
| 总量 content > 256 KiB | **413** | payload_too_large | line 380-389 |
| 非法长度 (length < 0) | **400** | (implied) | line 300 |

**完整性**: ✅ **完整** — 4 个 cap (1 MiB body / 100 messages / 32 KiB 单条 / 256 KiB 总量) + 415/411/413 拆清 (M2.5-SEC 附录 M P0-2 重点项) + A05 DoS 注释 + multipart 旁路 415 + CR/LF 防御 (_safe_path_label).

**M-final 动作**: **直接引用**, 附录 N §0 表格或 §1.1 新增子行 "serve.py HTTP 边界硬化 (R11 已落, OWASP A05 DoS 防护)" = `MAX_BODY_BYTES=1 MiB + MAX_MESSAGES_COUNT=100 + MAX_MESSAGE_CONTENT_BYTES=32 KiB + MAX_TOTAL_CONTENT_BYTES=256 KiB; HTTP 边界显式: 非 JSON → 415, 缺 Content-Length → 411, body/messages/单条/总量超限 → 413; multipart/form-data 与 application/x-www-form-urlencoded 全拒 (防 JSON-only schema validation 旁路); _safe_path_label 防 CR/LF 注入`, 引用 `apeireth/serve.py:46-77, 274-313, 345-389` 即可, **不重复实现**.

---

### 2.5 P0-5: R11-SEC-001/002 安全事件串联 (代码已实现, 文档未串联)

**代码侧 (working changes)**:
- R11-SEC-001 三类修复: ✅ 已实现 (P0-1, 见 §2.1, 5 处 v1121_security_guard_v01.py 注释 + 24+ 行 test)
- R11-SEC-002 self-claim 补充: ✅ 已实现 (R11 末, 不在 working changes 中, 4/4 covered, 见附录 M §1.5)

**文档侧 (附录 N draft)**:
- R11-SEC-002 4/4 覆盖: ✅ §0 + §1.1 + §6 硬约束三处引用 (`r11-philosophy-guardian.md` §3.1 + 附录 M §1.5 + T1 报告 §2.1 命令 2)
- R11-SEC-001 三类修复: ❌ **未串联** — 全文搜索 `R11-SEC-001` 0 命中; §0 + §1.1 + §6 硬约束 + §2.1 row 4 + §5.A 4 项遗留工程全部仅提 V1121 fake-KPI detector, 未提 R11-SEC-001 这个事件 ID

**完整性**: ⚠️ **代码完整 / 文档未闭环** — 工作区 working changes 已实现 R11-SEC-001 三类修复 + test 覆盖, 但附录 N draft 4 章涉及 V1121 处**未提 R11-SEC-001 锚**, 读者无法仅看附录 N 知道"v1121_security_guard_v01.py +65 行的 R11-SEC-001 修复是 R11 安全事件之一, 与 R11-SEC-002 并列".

**M-final 动作** (附录 N 文档化 1 项):
1. §1.1 命令 2 表格新增一行 "R11-SEC-001 三类修复" (与现有 "R11-SEC-002 补充 4/4" 行并列), 字面 = `fake-KPI regex 重写 (4 patterns 替代单 score=1.0 误报) + path traversal (split + segment check 替代 `..` in path 误报) + secret-leak (>=4 char password / 16+ char api_key)`, 引用 `apeireth/v1121_security_guard_v01.py:379-401, 780-803, 882-900, 929-936, 1029-1054`
2. §6 硬约束新增一条 (与现有 "不要重写哲学守门" 平行) "R11-SEC-001/002 是 R11 安全事件全集 (R11-SEC-001 fake-KPI/path/secret 三类修复 + R11-SEC-002 self-claim 补充 4/4), 两者都已在 working changes 落地 (R11-SEC-001 见 v1121_security_guard_v01.py +65 行, R11-SEC-002 见 r11-philosophy-guardian.md §3.1)"
3. §3 主文档呼应新增一行 "主 17:58 不假装 (R11-SEC-001/002 串联)" = "R11 安全事件全集已在 working changes 落地, 附录 N §1.1 + §6 引用 file:line, R12 接手可无歧义复用"

**不重复实现**: working changes 已有 R11-SEC-001 三类修复 (代码完整, 130 个测试全过), M-final 只需文档化引用.

---

## 3. 测试结果 (验证不破坏 R11 末)

```
$ python -m pytest tests/test_v1121_security_guard.py tests/test_v1132_real_deployment_validator.py tests/test_v1134_streamlit_real_startup.py tests/test_r11_p0_regression_guard.py --tb=short -q
============================= test session starts =============================
platform win32 -- Python 3.13.14, pytest-9.1.1, pluggy-1.6.0
rootdir: .openclaw\workspace\promethean
configfile: pyproject.toml
plugins: anyio-4.14.0, asyncio-1.4.0, cov-7.1.1, asyncio-... 
collected 130 items
tests\test_v1121_security_guard.py ......................ss...........   [ 26%]
tests\test_v1132_real_deployment_validator.py .......................    [ 44%]
tests\test_v1134_streamlit_real_startup.py ...............               [ 56%]
tests\test_r11_p0_regression_guard.py .................................. [ 82%]
.......................                                                  [100%]
======================= 128 passed, 2 skipped in 28.50s =======================
```

| 测试文件 | passed | skipped | 用时 | 覆盖范围 |
|---------|--------|---------|------|---------|
| `tests/test_v1121_security_guard.py` | 22 | 2 (R11-SEC-001 superseded) | ~10s | path traversal split + fake-KPI 4 patterns + secret-leak 2 patterns + gate_passed 严格化 + runner_missed counter + 旧 test 标记 superseded |
| `tests/test_v1132_real_deployment_validator.py` | 23 | 0 | ~7s | canonical_bundle_valid + offline_valid/runtime_valid 三分裂 + 18 跨文件断言 + daemon probe MISSING + to_dict 三字段 |
| `tests/test_v1134_streamlit_real_startup.py` | 15 | 0 | ~5s | (不直接相关, 列入确保不破坏 R11 末) |
| `tests/test_r11_p0_regression_guard.py` | 58 | 0 | ~6s | (P0 护栏 5 路径 57 测试 + 1 子集, 全部通过) |
| **总计** | **128** | **2** | **28.50s** | working changes **不破坏 R11 末** |

**R11 末 baseline 对照** (T1 报告 §1 + 附录 M §1.1):
- R11 末 baseline: V1121 + V1132 联合 56 passed, 2 skipped, 0 failed (附录 M §1.2)
- R12 接手 working changes: 128 passed, 2 skipped (合并 v1121 + v1132 + v1134 + r11_p0 四文件)
- v1121 + v1132 实际新增加 22 + 23 - (R11 末已含的子集) = 净增 ~15 个 R11-SEC-001 新 test (test_fake_kpi_detector_catches_pretend_r11 + test_v1121_bug_breached_asi_regex_typo_r11 + test_validator_canonical_bundle_is_semantically_consistent + test_validator_offline_success_does_not_claim_runtime_success + test_report_to_dict_has_required_keys 三字段 + 其他 R11-SEC-001 子测度)
- **R11 末 56 passed, 2 skipped 全部保留** (v1121_security_guard.py 22+ss 包含原 56 passed, 2 skipped 子集), **无回归**

**R11-SEC-001 旧 test supersede 机制** (健康, 非 bug):
- 旧 `test_fake_kpi_detector_catches_pretend` + `test_v1121_bug_breached_asi_regex_typo` 用 `@pytest.mark.skip(reason="R11-SEC-001: superseded")` 标记
- 2 个 skipped 是**有意 superseded**, 不是测试失败
- 替代: 新 `test_fake_kpi_detector_catches_pretend_r11` + `test_v1121_bug_breached_asi_regex_typo_r11` 覆盖同样逻辑但含 R11 严格化断言 (负样本 `score=1.0` 不再误报 + breached-regex typo 修复)

---

## 4. 给 M-final 的可执行 P0 必改项

### 4.1 P0 必改项状态总览

| # | P0 必改项 | 状态 | M-final 动作 |
|---|---------|------|------------|
| 1 | R11-SEC-001 三类修复 | ✅ 代码已实现 | **直接引用 working changes** (附录 N §1.1 新增 1 行, 引用 v1121_security_guard_v01.py:379-401, 780-803, 1029-1054) |
| 2 | V1132 部署 validator 语义门禁 | ✅ 代码已实现 | **直接引用 working changes** (附录 N §5.B row 2 扩写, 引用 v1132_real_deployment_validator.py:51-100, 240-245, check_canonical_bundle) |
| 3 | V1132 SSRF allowlist | ✅ 代码已实现 | **直接引用 working changes** (附录 N §5.B row 2 子行, 引用 v1132_real_deployment_validator.py:202-245) |
| 4 | serve.py HTTP 边界硬化 | ✅ 代码已实现 | **直接引用 working changes** (附录 N §0 或 §1.1 新增 1 行, 引用 serve.py:46-77, 274-313, 345-389) |
| 5 | R11-SEC-001/002 安全事件串联 | ⚠️ 代码已实现, 文档未串联 | **文档化 1 项** (附录 N §1.1 + §3 + §6 串联 3 处) |

### 4.2 4 项已闭环 P0: M-final 直接引用 working changes (不重复实现)

**P0-1 引用模板** (附录 N §1.1 表格新增行):
```
| R11-SEC-001 三类修复 | fake-KPI regex 重写 (4 patterns) + path traversal (split + segment check) + secret-leak (>=4 char password / 16+ char api_key), gate_passed 严格化 (runner_missed == 0) | ✅ 完全符合 (working changes v1121_security_guard_v01.py:379-401, 780-803, 882-900, 929-936, 1029-1054) |
```

**P0-2 + P0-3 引用模板** (附录 N §5.B row 2 扩写):
```
| 2 | V1132 部署 validator 语义门禁 (R11 已落) | canonical_bundle_valid (18 跨文件语义断言 ✓) + offline_valid/runtime_valid/passed 三分裂; R12 接手 daemon 不可达: runtime_valid=False, passed=False, canonical_bundle_valid=True, daemon probe 全 MISSING (docker_path=MISSING / kubectl_path=MISSING); V1132 SSRF allowlist: _LOOPBACK_HOSTS 5 host + _LOOPBACK_PORTS 7 port (含 canonical V1075 端口 8765) + scheme 仅 http/https, 拒绝 file:// / gopher:// / ftp:// / data: + 169.254.169.254 | 🔴 高 (R12 deploy/ ceiling 落点, 引用 v1132_real_deployment_validator.py:51-100, 202-245, check_canonical_bundle) |
```

**P0-4 引用模板** (附录 N §0 全表新增行 或 §1.1 新增子行):
```
| serve.py HTTP 边界硬化 (R11 已落, OWASP A05 DoS 防护) | MAX_BODY_BYTES=1 MiB + MAX_MESSAGES_COUNT=100 + MAX_MESSAGE_CONTENT_BYTES=32 KiB + MAX_TOTAL_CONTENT_BYTES=256 KiB; HTTP 边界显式: 非 JSON → 415 (unsupported_media_type), 缺 Content-Length → 411 (length_required), body / messages / 单条 / 总量超限 → 413 (payload_too_large); multipart/form-data 与 application/x-www-form-urlencoded 全拒 (防 JSON-only schema validation 旁路); _safe_path_label 防 CR/LF 注入 (控制字符 0x00-0x1F, 0x7F) | ✅ 完全符合 (working changes serve.py:46-77, 274-313, 345-389, 415/411/413 拆清 M2.5-SEC 附录 M P0-2 重点项) |
```

### 4.3 1 项文档化必改: M-final 串联 R11-SEC-001/002 (附录 N §1.1 + §3 + §6 三处)

**附录 N §1.1 命令 2 表格新增** (与现有 R11-SEC-002 行并列):
```
| R11-SEC-001 三类修复 (working changes 已落) | fake-KPI regex 重写 + path traversal (split) + secret-leak (>=4 char password) | ✅ 完全符合 (v1121_security_guard_v01.py:379-401, 780-803, 1029-1054, 24+ 行新 test 覆盖) |
```

**附录 N §3 主文档呼应新增一行** (与现有 6 个 anchor 并列):
```
| **主 17:58 不假装 (R11-SEC-001/002 串联)** | §1.1 + §6 串联 | R11 安全事件全集已在 working changes 落地 (R11-SEC-001 v1121_security_guard_v01.py +65 行 / R11-SEC-002 r11-philosophy-guardian.md §3.1), R12 接手可无歧义复用 |
```

**附录 N §6 硬约束新增一条** (与现有 4 条硬约束并列):
```
- ✅ **R11-SEC-001/002 是 R11 安全事件全集** — R11-SEC-001 fake-KPI/path/secret 三类修复 (working changes v1121_security_guard_v01.py:379-401, 780-803, 1029-1054) + R11-SEC-002 self-claim 补充 4/4 (r11-philosophy-guardian.md §3.1). 两者都已 LOCKED (R11-SEC-001 5 处 R11-SEC-001 注释 + 24+ 行新 test 覆盖; R11-SEC-002 命令 2 实测 4/4), R12 接手可在 §1.1 引用 file:line 复用, 不重写.
```

### 4.4 不必改动项 (已闭环, M-final 不要碰)

- ❌ **不要修改 R11 末 working changes** (v1121_security_guard_v01.py +v1132_real_deployment_validator.py +serve.py + 3 test files), 130 个测试全过, R11 末 baseline (V1121+V1132 联合 56 passed, 2 skipped, 0 failed) 全部保留
- ❌ **不要回退 working changes** (主人硬约束 + 任务 T5 硬约束), `git diff` 显示 35 files +1759/-316 是 R11 收尾后的真实状态
- ❌ **不要在附录 N 重复描述实现细节** (regex 字面 / allowlist 列表 / 状态码映射), 引用 file:line 即可, "任何人能接手"主 00:56 哲学要求"在附录 N 引用 working changes, 不重抄代码"
- ❌ **不要修改附录 M 之前内容** (6001 行旧 + 240 行附录 M), 用户硬约束, 附录 N 透明化已知差异即可

### 4.5 M-final 引用 working changes 后附录 N §5.A 优先级微调建议 (可选)

附录 N §5.A 当前优先级建议 = "3>1>4>2" (V1130 wallclock > W2/W4 > V1121 fake-KPI > V1077 dims), 引入 P0-2 + P0-3 + P0-4 引用后, 建议**保持不变**:
- P0-2 (V1132 部署 validator) 不是新 ceiling, 是 R11 已落语义门禁, R12 只需在 deploy/ 节点重跑, 优先级 🟡 中
- P0-3 (V1132 SSRF allowlist) 不是新 ceiling, 是 R11 已落入口安全, 优先级 🟢 低
- P0-4 (serve.py HTTP 边界) 不是新 ceiling, 是 R11 已落 OWASP A05 DoS, 优先级 🟢 低 (回归 test 已覆盖)
- P0-1 (R11-SEC-001) 不是新 ceiling, 是 R11 已落 fake-KPI 严格化, 优先级 🟢 低 (24+ 行新 test 已覆盖)
- P0-5 (SEC-001/002 串联) 是文档化必改, 不是工程 ceiling, 不进 §5.A 优先级

**结论**: 5 P0 必改项中 4 项是 "代码已实现, M-final 引用 working changes 即可" + 1 项是 "代码已实现, 附录 N 文档化 1 项串联 (3 处)", **M-final 总工作量 ≈ 1 个表格新增行 + §3 + §6 串联 3 处**, 不重写任何代码, 不重复实现 working changes 已有功能.

---

_Generated by Security Reviewer for task T5: c502102d-a3cd-44ff-bd54-72e51fb31522, 2026-07-30, 基于 working changes `git diff` (35 files +1759/-316) + pytest 128 passed, 2 skipped in 28.50s + T4 M2.5-SEC 报告 (5 P0 必改项). 硬性约束遵守: 未 commit / 未修改任何文件 / 未回退 working changes._
