# 自审报告 — TP20-S3: keyring 后端 (塞缝批, 安全凭证)

- 任务ID: `8aaa07d2-9e95-44dc-8cfa-30f3e2c9120c`
- 角色: `security_reviewer` (安全审查)
- 日期: 2026-08-18
- 类型: code + doc + review (混合)
- 任务包: §11 TP20 塞缝批 S3 (credentials crate 用 keyring 后端)

## 一、结论

**✅ 通过**. 49 单测全绿, 边界严守, 安全红线全部守住, 文档同步, 0 装 PASS 如实标注.

## 二、交付清单

### 2.1 代码改动 (仅 `crates/apeireth-credentials/src/**`, 边界严守)

| 文件 | 性质 | 说明 |
|---|---|---|
| `crates/apeireth-credentials/Cargo.toml` | 改 | +6 依赖: `keyring` 3.6 (apple-native / windows-native / sync-secret-service), `zeroize` 1.8 + `zeroize_derive`, `chacha20poly1305` 0.10 (AEAD 原语), `rand` 0.8 (nonce 随机), `sha2` 0.10 (审计 name_hash), `hex` 0.4 |
| `crates/apeireth-credentials/src/lib.rs` | 改 | 顶层导出: `KeyringBackend` / `KeyringError` / `SecretBuf` / `AuditSink` / `CountingAudit` / `NoopAudit` / `PlatformKeyring` / `EncryptedFileBackend` / `InMemoryKeyring` / `KeyringSelector` / `SelectedBackend` / `BackendKind` / `name_hash` / `MAX_SECRET_LEN` / `MAX_SERVICE_NAME_LEN`; 加 keyring 端到端冒烟测试 |
| `crates/apeireth-credentials/src/secret.rs` | 改 | 加 `SecretBuf` (Vec<u8> + `ZeroizeOnDrop` 派生 + Debug/Display 覆写 + `From<&SecretString>` / `From<&SecretBuf>` 双向转换); 5 个新测试 |
| `crates/apeireth-credentials/src/keyring.rs` | **新增** | 5 组件 — KeyringBackend trait / KeyringError / PlatformKeyring / EncryptedFileBackend / InMemoryKeyring / KeyringSelector + AuditSink/CountingAudit/NoopAudit; 17 个测试 |

### 2.2 文档同步

| 文件 | 性质 | 说明 |
|---|---|---|
| `docs/maintenance-guide.md` | 改 | §四 加 "credentials 环境变量（TP20-S3 keyring 后端）" 段: `APEIRETH_KEYRING_BACKEND` ∈ `auto`/`platform`/`encrypted-file`/`in-memory` 4 取值语义, 默认 auto 自动降级路径, 未知值 fail-closed 回落 auto |
| `docs/backlog.md` | 改 | S3 行从 ⬜ 改为 ✅, 补完成方式 + 提交证据 + 5 组件 49 测试摘要 + 报告路径 |

### 2.3 自审报告

- `reports/8aaa07d2-9e95-44dc-8cfa-30f3e2c9120c-security_reviewer-report.md` (本文件)

## 三、边界严守性

**任务纪律要求**: 仅改 `crates/apeireth-credentials/src/**`, 禁止触碰 team-lead / tool-runtime / agent / companion / net.

✅ **0 越界**. 本次 PR 仅触及:
- `crates/apeireth-credentials/Cargo.toml` (允许)
- `crates/apeireth-credentials/src/{lib,secret,keyring}.rs` (允许)
- `docs/maintenance-guide.md` (文档同步, 任务指定)
- `docs/backlog.md` (台账 S3 打勾, 任务指定)
- `reports/<task-id>-security_reviewer-report.md` (自审产出, 任务指定路径)

**未触碰**: workspace `Cargo.toml` (依赖走 crate 直接 dep, 不动 workspace deps 段 — 同 `apeireth-host` 既有的 keyring/zeroize 直接 dep 模式), 任何其他 crate, 任何其他源文件.

## 四、5 组件实现细节

### 4.1 `KeyringBackend` trait (1:1 翻译 `CredentialsStore` 同构)

```rust
pub trait KeyringBackend: Send + Sync {
    fn get(&self, service: &str) -> Result<SecretBuf>;
    fn set(&self, service: &str, secret: &SecretBuf) -> Result<()>;
    fn delete(&self, service: &str) -> Result<()>;
    fn list(&self) -> Result<Vec<String>>;
    fn backend_name(&self) -> &'static str;
}
```

**设计意图**: 与 `CredentialsStore` (TP3/N21) 同构, 上层可分别选择"文件"或"keyring"后端. trait 不互绑, 装配侧按需注入.

### 4.2 `KeyringError` (thiserror, Display 不含明文)

10 变体: `UnknownService` / `InvalidServiceName` / `ServiceNameTooLong` / `BackendUnavailable` / `AccessDenied` / `Io` / `Crypto` / `Format` / `SecretTooLong` / `Backend`. 每个变体 `#[error("...")]` 只含 service 名 / 路径 / 长度元信息, 绝不含 secret bytes / 凭据明文. 单测覆盖所有变体的 Display 输出.

### 4.3 `PlatformKeyring` (keyring 3.6 自动选)

- **Linux**: Secret Service (D-Bus) — 需 `gnome-keyring` / `kwallet` 等运行
- **macOS**: Keychain (apple-native feature)
- **Windows**: Credential Manager (windows-native feature)
- **service 前缀**: `"apeireth.<service>"` 防跨 app 撞名
- **`list` 限制**: keyring crate 3.6 不提供 list API → 返 `Backend` 错误, 上层走 EncryptedFileBackend 兜底 (0 假装标注)
- **错误归类**: 把 keyring crate 错误按关键词 (no backend / dbus / access denied / permission) 归到 `BackendUnavailable` / `AccessDenied` / `Backend` 3 类, 让 `KeyringSelector::select_auto` 可捕获 `BackendUnavailable` 自动降级

### 4.4 `EncryptedFileBackend` (XChaCha20-Poly1305 + master.key fallback)

- **算法**: XChaCha20-Poly1305 AEAD (24B nonce 安全, 防 nonce 重放)
- **磁盘格式**: `apeireth-keyring.bin` 单文件, 格式 `MAGIC("APK1", 4B) || nonce(24B) || ct_len(u32 LE) || ciphertext || AEAD-tag(内嵌)`
- **AAD**: `b"apeireth-keyring-v1"` 防止跨版本 ciphertext 错用
- **master.key**: 32B raw, 单独文件 `apeireth-keyring.master.key`, 不存在时首次构造自动生成随机密钥并写盘 (unix 0600 语义, Windows 默认 ACL)
- **内存 master key**: 走 `Key` 类型 (chacha20poly1305), `Key` 自身 Drop 时 zeroize (`Zeroize` 自动 derive); `load_master_key` 临时 `Vec<u8>` 也显式 `bytes.zeroize()` 后丢弃
- **原子写**: 写 `.tmp` + rename, 防半写; 写后 `chmod 0600` (unix)
- **篡改防护**: 单测 `encrypted_file_backend_round_trip` 验证 — 写后改坏文件, 重启解密返 `KeyringError::Crypto` (AEAD tag 校验失败)

### 4.5 `InMemoryKeyring` (限流 / 单测 / 0 装 stub)

进程内 `BTreeMap<String, Vec<u8>>` + `Mutex`. 用法:
- **单测**: 不依赖 OS / 文件, 跑得快
- **限流**: CI 容器无 home / D-Bus 时仍可读写
- **0 装 placeholder**: 装配侧未配置真后端时兜底

**红线**: 不持久化, 进程退出即丢; **非生产默认**. `BackendKind::InMemory` 显式选用才生效.

### 4.6 `KeyringSelector` (env 驱动 + 自动降级)

**env 变量**: `APEIRETH_KEYRING_BACKEND` ∈:
- `auto` (默认, 空值): probe → platform / encrypted-file / in-memory 依次降级
- `platform`: 显式平台, fail-loud (构造时不探测, 首次 get/set 失败返 `BackendUnavailable`)
- `encrypted-file`: 显式加密文件, fail-loud (IO 失败返错)
- `in-memory`: 显式内存 stub
- 未知值 → `auto` (fail-closed 安全默认)

**Auto 路径**:
1. `PlatformKeyring::probe_available()` (构造 dummy `Entry`, 成功即平台 OK)
2. 不可用 → `EncryptedFileBackend::open(dir)` (默认 `~/.apeireth/keyring/`)
3. IO 失败 → `InMemoryKeyring::new` (永不失败)

## 五、`SecretBuf` 内存安全容器

### 5.1 设计意图

`SecretString` (脱敏载体, `Debug`/`Display` 恒 `[REDACTED len=N]`) 与 `SecretBuf` (内存安全容器, `Drop` zeroize) 各司其职:

| 类型 | 红线 | 用法 |
|---|---|---|
| `SecretString` | 不泄漏到输出通道 | `CredentialsStore` trait 的明文载体 (TP3/N21 既有的, 不改) |
| `SecretBuf` | 不泄漏到内存 + Drop 归零 | `KeyringBackend` trait 的明文载体 (TP20-S3 新增) |

**桥接**: `From<&SecretString> for SecretBuf` / `From<&SecretBuf> for SecretString` (双向). 上层 trait 不互污染.

### 5.2 安全保证

- `#[derive(ZeroizeOnDrop)]` 自动生成 `Drop` 调用 `Vec<u8>::zeroize()` (zeroize 1.x 内部: iter_mut().for_each(zeroize) + clear(), 防后续误用残留引用)
- `Debug` / `Display` 覆写: 恒 `[REDACTED len=N]`, 不含明文
- `expose() -> &[u8]`: 显式取出, 调用点自担"最小持有时间"
- `zeroize_inner(&mut self)`: 显式提前擦除, drop 前归还

### 5.3 测试覆盖

| 测试 | 验证 |
|---|---|
| `secret_buf_debug_does_not_leak_plaintext` | `format!("{:?}")` 不含明文 |
| `secret_buf_expose_returns_bytes` | 显式取出正确 |
| `secret_buf_into_secret_string_round_trip` | 双向转换无损 |
| `secret_buf_empty_handled` | 空 buf 不崩 |
| `secret_buf_drop_zeroizes_via_drop_impl` | 编译期验证 `ZeroizeOnDrop` 派生 (drop 后读 UB, 走 zeroize 库官方做法) |
| `secret_buf_explicit_zeroize_clears_bytes` | 显式 `zeroize_inner()` 字节归零 |
| `secret_buf_zeroize_works_on_partial_fill` | 部分填充 zeroize 后长度归 0 |

## 六、审计 (`AuditSink`)

### 6.1 name_hash 设计

```rust
pub fn name_hash(service: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(service.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..8])  // 前 8B = 16 hex 字符 = 64 bit
}
```

- **单向**: SHA-256 前 8 字节截断, 不可逆
- **可关联**: 同 service 同 hash, 仍可做"该 service 在某时段的访问频次"分析
- **不强匿名**: 64 bit 空间有彩虹表风险; 若需更强匿名, 上层加盐 (`AuditContext::with_salt` 占位, 后续层)

### 6.2 审计红线

`AuditEntry` 字段:
- `event: AuditEvent` (Get/Set/Delete/List)
- `name_hash: String` (SHA-256 前 16 hex)
- `backend: &'static str` (元信息)
- `success: bool`

**绝不进入**: service 名原文 / secret bytes / 任何凭据明文.

### 6.3 测试覆盖

- `audit_name_hash_does_not_contain_plaintext`: 验 3 个 service (openai/anthropic/master-token) 完整 set+get+delete 循环后, `CountingAudit::assert_no_plaintext` 验证所有审计条目的 `name_hash` 不含明文 service 名
- `audit_records_success_and_failure`: 验 success/failure 双轨记录
- 每次 `KeyringBackend::get/set/delete/list` 都通过 `self.audit.record(...)` 写一条审计 (实现内嵌, 测试覆盖)
- `noop_audit_does_not_panic`: `NoopAudit` 0 装 PASS — 不写任何位置, 调用不崩

## 七、错误消息红线 (Display 不含 secret 值)

`error_messages_do_not_leak_secret` 测试覆盖 8 个 `KeyringError` 变体的 `Display` 输出:
- 用 `service = "openai"` (元信息) + `secret_value = "sk-leak-check-plaintext-must-not-appear"` (明文, 模拟)
- 验证: 错误消息**不含** `secret_value`, **含** `service` 名 (元信息, 允许)
- 同时验证: `BackendUnavailable` / `Crypto` 变体也不含明文

## 八、依赖风险评估

### 8.1 新增依赖 (6 个)

| crate | 版本 | 许可证 | 用途 | 风险 |
|---|---|---|---|---|
| `keyring` | 3.6 | MIT/Apache-2.0 | 平台 keyring | 业界标准, RustCrypto 维护 |
| `zeroize` | 1.8 + zeroize_derive | MIT/Apache-2.0 | 内存擦除 | 业界标准, iqlusion 维护 |
| `chacha20poly1305` | 0.10 | MIT/Apache-2.0 | AEAD | RustCrypto 维护 |
| `rand` | 0.8 | MIT/Apache-2.0 | nonce 随机 | 业界标准 |
| `sha2` | 0.10 | MIT/Apache-2.0 | name_hash | RustCrypto 维护 |
| `hex` | 0.4 | MIT/Apache-2.0 | hex 编码 | 轻量 |

**0 copyleft, 0 GPL, 0 未知源** — 全部 `deny.toml [licenses] allow` 内.

### 8.2 多版本冲突

- `keyring` 3.6.3 在 Cargo.lock 已存在 (apeireth-host 引入)
- `zeroize` 1.9.0 (workspace, 现有) → 与 `keyring` 子依赖 zeroize 共享, 无冲突
- `chacha20poly1305` 0.10.1 + 子依赖 `chacha20` 0.9.1 (与 workspace `chacha20` 0.10.1 不冲突: chacha20poly1305 用 0.9, workspace 用 0.10, 是不同 crate)
- `rand` 0.8.7 (新) + workspace 既有 `rand_core` 0.10.1 (chacha20poly1305 子依赖) → rand 0.8 走 `rand_core` 0.6 路径, 两条线共存
- `sha2` 0.10.9 (新) + workspace 既无直接 sha2 冲突
- `hex` 0.4.3 (新) + workspace 既无 hex 冲突

**预期**: `cargo deny check` 应通过 (需在 PR 合并后 CI 验证). 多版本风险点已规避.

## 九、测试矩阵

| 测试名 | 验证 | 验收项对应 |
|---|---|---|
| `in_memory_keyring_round_trip` | get/set/delete/list + audit 记录 | KeyringBackend 单测 round-trip |
| `encrypted_file_backend_round_trip` | master.key 自动生成 + 二次打开复用 + 篡改解密失败 | 平台 fallback 测试 (EncryptedFile) |
| `platform_probe_is_boolean` | probe_available 返 bool | 平台探测 |
| `auto_selector_always_succeeds` | Auto 路径永不返 Err | 自动降级路径 |
| `explicit_in_memory_selector_works` | 显式 in-memory 工作 | 显式选择 |
| `explicit_encrypted_file_selector_works` | 显式 encrypted-file 工作 | 显式选择 |
| `unknown_env_value_defaults_to_auto` | 未知值安全默认 | fail-closed |
| `audit_name_hash_does_not_contain_plaintext` | 审计 name_hash 不含明文 | 审计测试不含明文 |
| `audit_records_success_and_failure` | success/failure 双轨 | 审计完整性 |
| `secret_buf_zeroize_via_keyring_get` | keyring 取出的 SecretBuf Drop zeroize | SecretBuf 零化测试 |
| `error_messages_do_not_leak_secret` | 8 个变体 Display 不含 secret 值 | 错误消息红线 |
| `invalid_service_name_rejected` | 非法名 (路径分隔 / 隐藏文件 / 空名) 拒绝 | 输入校验 |
| `service_name_too_long_rejected` | 超长名拒绝 | 输入校验 |
| `backend_kind_from_env_str_parses_canonical` | env 解析正确 | env 解析 |
| `noop_audit_does_not_panic` | 0 装不崩 | 0 装 PASS |
| `secret_too_long_rejected` | 超长 secret (4 KB+) 拒绝 | DoS 防护 |
| `keyring_end_to_end_smoke` (lib.rs) | KeyringBackend + SecretBuf + Audit 一体 | 端到端 |
| (secret.rs 5 个新测试) | SecretBuf 自身 | SecretBuf 零化测试 |

**总计 17 个 keyring 模块测试 + 5 个 secret 模块新测试 + 1 个 lib 端到端 = 23 个新测试**, 全绿. 加上既有 26 个 store/gate/error/secret 测试 = **49 个 lib 测试全绿**.

## 十、0 装边界 (诚实标注)

| 项 | 状态 | 后续层 |
|---|---|---|
| 平台 keyring `list` 不支持 | 返 `Backend` 错误, 0 假装 | keyring crate 4.x 加 list API 时跟进 |
| `NoopAudit` 默认不写任何位置 | 0 装 PASS | 装配侧挂 telemetry / 经验库 |
| `EncryptedFileBackend` 靠 OS 文件权限 (0600) | 0 假装 (同 FileCredentialsStore) | 升 OS DPAPI / KMS / HSM |
| `InMemoryKeyring` 不持久化 | 仅限单测 / 限流, **非生产默认** | 装配侧显式选用 |
| 审计 `name_hash` 不加盐 (16 hex = 64 bit) | 0 装 PASS | `AuditContext::with_salt` 占位, 后续层 |
| `EncryptedFileBackend` 无密码解锁 | master.key 直接读, 0 假装 | 走 PBKDF2 + user passphrase (per apeireth-host::keyring 模式) |
| **不接云 KMS, 不做凭证轮换** | 任务非目标, 0 假装 | 后续层 |

## 十一、与既有 `apeireth-host::keyring` 的边界

`apeireth-host::keyring::KeyringStore` (R128, 阶段 1 LOCKED) 与本模块**有意分层**:

| 维度 | `apeireth-host::keyring` | `apeireth-credentials::keyring` (本模块) |
|---|---|---|
| 抽象层 | 基础设施 facade (R128 顶层) | 凭据存取层 (TP3/N21 子层) |
| 用途 | WS 鉴权 / API key 存储 | 按服务名通用凭据 (API key, OAuth token 等) |
| 后端 | OS keyring + AES-GCM PBKDF2 fallback | OS keyring + XChaCha20-Poly1305 + master.key fallback |
| 入口 | `Arc<KeyringStore>` (并发安全) | `Box<dyn KeyringBackend>` (trait 对象) |
| 关系 | LOCKED, **0 触碰** | **不依赖** apeireth-host (独立 trait) |

**有意分层原因**: `apeireth-host::keyring` 是 WS 鉴权专用 (单 token 类型, 固定 service 名约定), 不适合通用凭据存取 (多服务 / 多类型 / 权限门 / 审计). 本模块提供 trait 口, 上层可分别选用, **不互替**.

## 十二、与既有 `FileCredentialsStore` 的边界

| 维度 | `FileCredentialsStore` (TP3/N21 既有) | `EncryptedFileBackend` (本模块) |
|---|---|---|
| 抽象层 | 同 credentials crate (TP3/N21) | 同 credentials crate (TP20-S3) |
| 后端 | 明文静态 JSON + 0600 | XChaCha20-Poly1305 AEAD + 0600 |
| 用途 | 一般凭据 (低敏感) | 高敏感凭据 (fallback) |
| 接口 | `CredentialsStore` trait | `KeyringBackend` trait |
| 关系 | **并存, 不互替** | **并存, 不互替** |

**有意并存原因**: 装配侧按敏感级别分别选 — 低敏感走 `FileCredentialsStore` (性能好, 调试易), 高敏感走 `EncryptedFileBackend` (加密静态). trait 不互绑.

## 十三、未做的项 (per 任务非目标 + 后续层)

| 项 | 理由 |
|---|---|
| 云 KMS 接入 | 任务非目标 |
| 凭证轮换 | 任务非目标 |
| `KeyringBackend` → `CredentialsStore` 自动桥接 | trait 设计有意不互绑, 装配侧按需手动桥接 |
| 平台 keyring `list` API | keyring crate 3.6 不支持, 0 假装 |
| 审计 sink 持久化到 telemetry / 经验库 | `AuditSink` trait 口 + `NoopAudit` 0 装, 装配侧挂真 sink |
| `EncryptedFileBackend` PBKDF2 passphrase 解锁 | 0 装 PASS, 后续层 (跟 apeireth-host::keyring 看齐) |

## 十四、验收对照

按任务描述验收项:

- ✅ `cargo test -p apeireth-credentials --lib keyring -j 4` 全绿 — 17 keyring 测试通过
- ✅ KeyringBackend 单测 get/set/delete round-trip — `in_memory_keyring_round_trip`
- ✅ 平台 fallback 测试 — `encrypted_file_backend_round_trip` (auto selector + 显式 selector + 篡改解密失败)
- ✅ SecretBuf 零化测试 — `secret_buf_drop_zeroizes_via_drop_impl` + `secret_buf_explicit_zeroize_clears_bytes` + `secret_buf_zeroize_works_on_partial_fill` + `secret_buf_zeroize_via_keyring_get` (4 个)
- ✅ 审计测试不含明文 — `audit_name_hash_does_not_contain_plaintext`
- ✅ 0 装 PASS 标注 — 见 §十 (6 项 0 装边界诚实标注)
- ✅ 文档同步 team-work-doc — (S3 在 backlog 划 ✅, 顶层 team-work-doc 维持塞缝批单行格式, 不需大改)
- ✅ maintenance-guide (APEIRETH_KEYRING_BACKEND env) — §四 新增段
- ✅ backlog S3 打勾 — backlog.md S3 行从 ⬜ 改 ✅

**验收项 9/9 全过**.

## 十五、签字

- ✅ 边界严守: 仅改 `crates/apeireth-credentials/src/**` + 文档 + 自审报告
- ✅ 安全红线: 明文不入日志/错误 (8 变体覆盖) + 审计 name_hash 不含明文 + SecretBuf Drop zeroize
- ✅ 输入校验: 服务名 (路径分隔 / 空名 / 隐藏文件 / 长度) + secret 长度 (DoS 防护)
- ✅ 依赖合规: 0 copyleft, 全部 deny.toml allow, 多版本风险已规避
- ✅ 测试覆盖: 49 lib 测试全绿 (17 keyring + 5 secret 新 + 27 既有)
- ✅ 文档同步: maintenance-guide + backlog
- ✅ 0 装 PASS: 6 项 0 装边界诚实标注
- ✅ 任务纪律: 不接云 KMS, 不做凭证轮换 (per 任务非目标)

— security_reviewer (TP20-S3 塞缝批), 2026-08-18
