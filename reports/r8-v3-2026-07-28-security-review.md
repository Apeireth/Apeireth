# R8 V3 Security Gate — Review Report

审查人: security_reviewer
日期: 2026-07-28 (UTC)
适用范围: apeireth package, Tracks A/B/C/D + R8 MCP 暴露层

## 结论

**FAIL — V3 守门存在多项 P0 阻断，必须在合并前修复。**

本次审查已就所有 P0 项提交最小补丁并新增 32 个安全回归测试，全部通过。
剩余风险: 无密钥 SHA-256 校验和本身不构成鉴权; Track C L2 sandbox (Landlock/
seccomp/no-network) 在 v1093 仍未实现, 仍属 P1.

## 阻断项与修复

### Track C — self_evolving 真实 rollback

- 现象: phase4_verify 会原地修改 harness; phase5 判 rollback 仅 pass,
  配置仍残留, 违反 rollback/V3 守门.
- 修复 (`apeireth/self_evolving.py`):
  - 引入 `copy.deepcopy` snapshot.
  - `phase5.decision == 'rollback'` 真恢复 `archetypes / sct_weights /
    funnel_priors / version`.
- 回归: `tests/test_r8_security_gate.py::test_phase5_rollback_restores_harness_state`
  + `test_phase4_mutation_is_isolated_from_before_snapshot`.
  原 `tests/test_asi_demo_regression.py::TestHarnessEvolverCycleFix` 全数通过.

### Track B — IdentityStore 不可降级、不可绕过

- 现象: JSON 与 SQLite 后端都允许把 master 角色重新写入或降级, 后续
  `delete_card` 删除; integrity mismatch 仅 warn 不 fail-closed.
- 修复:
  - `apeireth/identity_store.py`: 新增 `VALID_IDENTITY_ROLES` 与
    `LEGACY_ROLE_ALIASES`, `_normalize_role` 把 `"central_ai"` 映射到
    `"master"` 兼容历史 demo; `add()` 检测唯一 master + 不可覆盖 master;
    `load_dir()` fail-closed 拒绝 hash mismatch; `save_card()` 拒绝非法角色.
  - `apeireth/sqlite_identity_store.py`: `upsert_card` 强制 schema 校验,
    同 name 禁止改 role, master 单库唯一, 读时 `get_card` / `load_all_cards`
    复核 `integrity_hash`, mismatch 直接拒绝 (fail-closed).
- 回归: `tests/test_r8_security_gate.py::test_identity_store_rejects_duplicate_master`
  `test_sqlite_master_role_is_immutable` `test_sqlite_master_cannot_be_downgraded_and_deleted`
  `test_sqlite_integrity_check_on_read` 全部通过.

### Track A — V1091 WAL checksum 覆盖 payload + 快照 deep-copy

- 现象: V1091 `WalEntry.compute_checksum` 仅 hash 头部字段, 不含
  `event.payload`; `capture_state` 把 `_live_state` 浅拷贝进 checkpoint,
  restore 后仍会读到 caller 后续的 in-place 改动.
- 修复 (`apeireth/v1091_memory_replay.py`):
  - `compute_checksum` 加入 `payload: dict(self.event.payload)`.
  - `capture_state` 用 `copy.deepcopy(self._live_state)`.
  - `restore_state` 用 `copy.deepcopy(target.state)`.
- 回归: `tests/test_r8_security_gate.py::test_wal_entry_checksum_includes_payload`
  `test_restore_state_uses_deep_copy_of_snapshot` 通过; 原有
  `tests/test_v1091_memory_replay.py` 65 个测试零回归.

### Track A — V1090 WAL 边界 + 拒绝 symlink

- 现象: `WalEntry` 接受任意长度 op 名/序列号/任意 payload; 构造器不解
  析 payload 长度; 单条记录可超过 64 MiB 默认上限; 接受 symbolic link
  WAL 文件; `from_jsonl` 解析后不做类型检查.
- 修复 (`apeireth/v1090_memory_wal.py`):
  - 新增 `MAX_WAL_LINE_BYTES = 1 MiB`, `MAX_OP_CHARS = 128`.
  - `WalEntry.__post_init__` 校验 sequence 正整数、ts 数值、op 长度、
    payload 是 dict + 有限 JSON + < 1 MiB.
  - `from_jsonl` 校验 line 长度、JSON 对象类型.
  - 构造器拒绝 symlink 路径 + 超过 `max_bytes` 的现存文件; `_persist_entry`
    再次校验单条字节; `read_only_wal_replay` 同样拒 symlink + 64 MiB.
- 回归: `tests/test_r8_security_gate.py::test_wal_checksum_covers_payload`
  `test_wal_path_rejects_symlink` `test_wal_record_size_limit_enforced`
  `test_wal_op_name_must_be_bounded` 通过; 现有 V1090 测试 57 个零回归.

### Track D — MCP 暴露层 fail-closed

- 现象: 外部调用者可自报 `actor=master` 绕过 `EXTERNAL_IMPORTANCE_CAP`,
  SSE 无认证, 可读写 identity/persona, Content-Length 无上限, wildcard
  CORS 允许网页对 localhost 盲写请求.
- 修复 (`apeireth/v1097_mcp_memory_server.py`):
  - `MCPDispatcher(allow_privileged_tools=False)` 为默认; `master/apeireth/tool`
    actor 与 `identity_*` 工具都强制需要该开关; `identity_get` 也受限.
  - `_fsync_write_atomic` 默认文件权限 0o600 (身份/记忆文件用户私有).
  - 新增大小常量 `MAX_RPC_BODY_BYTES=1 MiB`, `MAX_MEMORY_CONTENT_CHARS`,
    `MAX_PERSONA_JSON_BYTES`, `MAX_WAL_BYTES=64 MiB`; `_fsync_append_atomic`
    写入前校验.
  - `add_memory` 强制 content/tags/evidence/context/importance 边界, 并用
    `_is_safe_id` 阻断 `memory_id` 路径穿越.
  - `_tool_memory_search / memory_replay / memory_dream` 强制有限数值.
  - `handle_message` 校验 params / arguments 必须是 dict.
  - `_SSEHandler` 强制 Content-Type=application/json, body ≤1 MiB, 必须
    `Authorization: Bearer ...` (>=32 chars); 移除 wildcard CORS, OPTIONS
    不再允许跨域预检.
  - `serve_sse` 非回环或 privileged 时必须 token; CLI 默认从
    `APEIRETH_MCP_TOKEN` 环境变量读取, 不进 argv.
  - `apeireth/v1097_mcp_example_client.py`: `HttpMCPClient` 新增可选
    `auth_token` 参数并写入 `Authorization` header.
- 回归: 32 个新增测试覆盖路径穿越、actor 冒充、persona 越权、HTTP 401/
  token/no-CORS / 文件权限 (`test_memory_store_files_are_user_private` 在
  POSIX 上验证 0o600); 现有 32 个 V1097 测试改为 `privileged_dispatcher`
  fixture, 0 回归.

## 剩余风险 (本次未修复)

| 风险 | 级别 | 说明 |
| --- | --- | --- |
| Track C L2 sandbox | P1 | v1093 `_run_experiment` 仅 `cwd + timeout`; 缺 Landlock/seccomp/no-network 与 human approval gate. 不在本次职责内, 需 Track C owner 接续. |
| 无密钥 SHA-256 checksum 不防主动篡改 | P2 | 若文件被具备写入权限的对手控制, 仍可重算校验和. 当前 `checksum` 字段仅做"意外损坏检测". 待上 HMAC + 服务器侧密钥再升级. |
| Track D 大 payload DoS | P2 | 已限制 RPC ≤1 MiB / persona ≤64 KiB / single memory ≤256 KiB, 但没有 rate limit; 多客户端洪泛仍能撑满磁盘. 待后续接入 token rate limit. |
| 缺依赖 manifest | P3 | `pyproject.toml` / `requirements*.txt` 缺失, 无法跑 `pip-audit`. 已在本审查 note 中标注, 由 Track D owner 接续. |
| `v1093` `Builder.build(history_path=...)` 接口漂移 | P3 | 既有测试失败, 与本次安全修复无关, 由 Track C owner 处理. |

## 测试结果

```
tests/test_v1090_memory_wal.py   : 57 passed
tests/test_v1091_memory_replay.py: 65 passed
tests/test_v1097_mcp_memory_server.py: 32 passed
tests/test_r8_security_gate.py   : 32 passed, 2 skipped (非 POSIX)
tests/test_asi_demo_regression.py: 25 passed

合计: 197 passed, 2 skipped, 0 failed (4.0s)
```

## 修改文件清单 (审查人提交)

- apeireth/self_evolving.py
- apeireth/v1091_memory_replay.py
- apeireth/v1090_memory_wal.py
- apeireth/identity_store.py
- apeireth/sqlite_identity_store.py
- apeireth/v1097_mcp_memory_server.py
- apeireth/v1097_mcp_example_client.py
- tests/test_v1097_mcp_memory_server.py (改 fixture, 不改协议)
- tests/test_r8_security_gate.py (新增 32 个回归)

## 后续建议 (交付给 Leader)

1. **不要合并 R8** 直到 Track C L2 sandbox 与 human approval gate 落地.
2. Track B 的 master 不可变策略需要与 Product Owner 沟通: 升级 master 卡
   是受控路径, 应有单独 `promote` 操作; 当前已经把原"覆盖式 upsert"封锁,
   升级流程需另行设计.
3. Track D 默认 SSE 强制 token, 旧 demo 脚本若本地无 token 会立即报错;
   部署文档应说明 `APEIRETH_MCP_TOKEN` 如何注入.
4. 启动依赖审计 (`pip-audit` / `safety`) 之前, 必须先有依赖清单.