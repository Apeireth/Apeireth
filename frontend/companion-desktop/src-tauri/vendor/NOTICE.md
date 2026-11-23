# 本地依赖补丁

这些源码来自原锁文件指定的 crates.io 发布包。保留包身份、版权、许可证和原始清单。下表的 SHA-256 对应未修改的发布压缩包，便于独立核对。发版前复核本地差异和上游公告，待兼容的上游版本包含相应修复后移除本地补丁。

| 包 | 发布压缩包 SHA-256 | 本地变更 |
| --- | --- | --- |
| [`glib 0.18.5`](https://crates.io/crates/glib/0.18.5) | `233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5` | [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) 的[上游补丁](https://github.com/gtk-rs/gtk-rs-core/pull/1343)修正字符串迭代器的可变输出指针。GTK 3 的兼容 API 保持原版本。 |
| [`glib-macros 0.18.5`](https://crates.io/crates/glib-macros/0.18.5) | `0bb0228f477c0900c880fd78c8759b95c7636dbd7842707f49e132378aa2acdc` | 通过既有 `syn::Error` 生成诊断，移除 [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) 所列宏依赖。 |
| [`gtk3-macros 0.18.2`](https://crates.io/crates/gtk3-macros/0.18.2) | `52ff3c5b21f14f0736fed6dcfc0bfb4225ebf5725f3c0209edeec181e4d73e9d` | 同样用 `syn::Error` 返回模板宏诊断。 |
| [`tauri-plugin-single-instance 2.5.5`](https://crates.io/crates/tauri-plugin-single-instance/2.5.5) | `726b6da0dd5bd1344f84f4108a622c07d01923baacb5c803a605189672c5352f` | Linux 使用 zbus 6 及其公开同步入口，避免 [RUSTSEC-2026-0370](https://rustsec.org/advisories/RUSTSEC-2026-0370.html)、[0371](https://rustsec.org/advisories/RUSTSEC-2026-0371.html)、[0372](https://rustsec.org/advisories/RUSTSEC-2026-0372.html) 中停止维护的包。保留原 D-Bus 名称、对象路径、调用参数和清理时序。 |
| [`tauri-utils 2.10.4`](https://crates.io/crates/tauri-utils/2.10.4) | `28d1d33d4453c5df0bf1672dbc7ada69d8ed74f56a4316d964a4aa73d4ecd3ad` | `serde_with` 仅启用 `macros`。两个配置模块继续使用同一个序列化宏，锁文件不再解析未使用的日期适配器。 |

`tauri-plugin-single-instance` 的平台分派集中在 `src/platform_impl/mod.rs`，由单一入口按目标操作系统选择对应实现。

`cargo audit --deny warnings` 继续检查整个锁文件。RustSec 用注册表来源识别发布包，上表的本地源码靠独立的上游压缩包比对和行为回归验证。

在 `src-tauri` 中执行 Linux 回归。

```sh
cargo test --locked --test gtk3_variant_iterator
dbus-run-session -- cargo test --locked --test single_instance_dbus -- --ignored
```
