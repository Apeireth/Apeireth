# apeireth-credentials

> Apeireth 统一凭据存取层 (TP3/N21): 按服务名读写凭据 (CredentialsStore trait + 文件形态后端) + 权限洋葱衔接 (高危凭据走审批门 trait 口) + 脱敏红线 (明文不入日志/错误). TP20-S3 塞缝批: 加 KeyringBackend trait + 平台 keyring 后端 + EncryptedFileBackend fallback (chacha20poly1305) + SecretBuf zeroize + 审计 (name_hash 不含明文)

apeireth-credentials 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (6 src 文件 / 50 测试)

- `src/lib.rs` — 入口 re-export + CredentialsStore trait 定义 + 3 测试
- `src/store.rs` — EncryptedFileBackend fallback (chacha20poly1305) + 9 测试
- `src/keyring.rs` — KeyringBackend trait + 平台 keyring 后端 (Linux/macOS/Windows 自动选) + 16 测试
- `src/secret.rs` — SecretBuf (zeroize Drop) + 13 测试
- `src/gate.rs` — 审批门 trait (高危凭据路由) + 6 测试
- `src/error.rs` — CredentialsError + 3 测试
