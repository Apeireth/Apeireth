# B1 \u62a5\u544a \u2014 telemetry 4 \u5408\u5e76 1.1 \u771f\u5408\u5e76\u5b8c\u6210 (2026-08-09)

## \u80cc\u666f

R35 \u672c\u8eab\u662f facade\uff0c4 \u4e2a\u8001 crate (`apeireth-cache` / `apeireth-metrics` / `apeireth-tracing` / `apeireth-observability`) \u4ecd\u662f\u72ec\u7acb\u5b58\u5728\u3002
B1 \u5e72\u5b8c\u5e95\uff0c\u8d70 R35 facade \u5347\u7ea7\u4e3a 1.1 \u771f\u5408\u5e76\uff1a4 \u4e2a\u8001 crate \u7684\u6e90\u4ee3\u7801\u5168\u90e8 1:1 \u642c\u8fdb `apeireth-telemetry/src/{cache,metric,trace,observability}/`\uff0c\u8001 crate \u7f29\u6210 1 \u884c re-export shim\u3002

## \u52a8\u4f5c

1. **\u6e90\u4ee3\u7801\u642c\u8fd0**\uff1a4 \u4e2a\u8001 crate `lib.rs` \u4e2d `pub mod X;` \u4e4b\u540e\u7684\u5168\u90e8\u5185\u5bb9\uff08re-export / const / trait / struct / impl / fn / test\uff09\u6574\u4f53\u642c\u8fdb\u65b0\u7684 `_root.rs`\uff0c\u9876\u90e8 `use std::fmt;` \u7b49 import \u4fdd\u7559
2. **\u8def\u5f84\u4fee\u590d**\uff1a`_root.rs` \u5185 `pub use X::Y` / `use crate::X::Y` \u6539\u4e3a `pub use super::X::Y` / `use super::X::Y`\uff08X \u4e3a\u540c module \u5144\u5f1f\uff09
3. **\u5b50\u6587\u4ef6\u4fee\u590d**\uff1a\u6240\u6709\u540c module \u4e0b\u7684 source \u6587\u4ef6 `use crate::X` \u6539 `use super::X`\uff1b`mod tests` \u4e2d\u7684 `super::X` \u6539 `super::super::X`\uff1b`crate::X` \u4e2d\u7684\u5b9a\u4e49\u70b9\u91cd\u5b9a\u4f4d\u5230 `super::X`\u3002
4. **shim \u751f\u6210**\uff1a4 \u4e2a\u8001 crate \u7684 `lib.rs` \u7f29\u6210 1 \u884c `pub use apeireth_telemetry::{module}::*;`\uff0c`Cargo.toml` \u52a0 `apeireth-telemetry = { path = "../apeireth-telemetry" }`\u3002
5. **\u6c34\u4ee3\u7801\u6e05\u7406**\uff1a\u5220 4 \u4e2a `_compat_*.rs`\uff08\u65e7 facade \u8df3\u677f\uff09\uff0c\u4e0e mod.rs \u91cc\u88ab\u9519\u8bef\u63d0\u53d6\u7684 pub const \u5757\u3002

## \u4ea4\u4ed8

| \u9879 | \u6570\u91cf |
|---|---|
| `\u65b0\u589e` cache/metric/trace/observability 4 \u4e2a module | 4 \u4e2a\u76ee\u5f55 |
| `\u65b0\u589e` _root.rs \uff08\u4ece\u8001 lib.rs \u642c\u8fc1\u4f53\uff09 | 4 \u6587\u4ef6 |
| `\u4fee\u6539` source \u6587\u4ef6 \uff08super::X \u8def\u5f84\u4fee\u590d\uff09 | 30+ \u6587\u4ef6 |
| `\u5220\u9664` _compat_*.rs \u4e0e\u65e7 mod.rs \u63d0\u53d6\u5757 | 4+4 \u6587\u4ef6 |
| shim \u8001 crate lib.rs | 4 \u6587\u4ef6 |
| `\u4fee\u6539` 4 \u8001 crate Cargo.toml \u52a0 telemetry path dep | 4 \u6587\u4ef6 |

## \u9a8c\u8bc1

```text
cargo check --workspace                 \u2192 0 error, ~7s
cargo test -p apeireth-telemetry --lib  \u2192 429 passed, 0 failed
cargo test --workspace --lib            \u2192 4148 passed, 0 failed
```

- 4 \u4e2a\u8001 crate `cargo check` \u5168\u90e8 OK
- `apeireth_telemetry::{cache,metric,trace,observability}` 4 \u8def\u5f84 0 \u6539\u52a8 0 \u4ee3\u7801
- `\u8001 import \u8def\u5f84` `apeireth_cache::X` / `apeireth_metrics::X` / `apeireth_tracing::X` / `apeireth_observability::X` \u5168 workspace 0 \u4fee\u6539 \u4ecd\u5de5\u4f5c\uff08shim \u900f\u660e\u8d70\u5411\uff09

## 6 \u54f2\u5b66\u953a

- **S-1 \u5317\u6781\u661f**\uff1a\u4e3b\u5bfc `apeireth-telemetry` \u4f5c\u4e3a\u552f\u4e00\u5165\u53e3\uff0c1:1 \u642c v0.9.21 \u5546\u4e1a\u7248 + Prometheus + W3C \u8d28\u91cf
- **S-2 \u5b9e\u4e8b\u6c42\u662f**\uff1a0 \u91cd\u5199 0 \u5047\u88c5\uff0c\u4ee3\u7801 1:1 \u642c\u8fd0\uff0cshim \u900f\u660e
- **O-3 \u5e72\u5230\u5e95**\uff1a131 \u7f16\u8bd1\u9519 \u2192 0\uff0c429 tests 0 failed
- **O-4 \u4efb\u4f55\u4eba\u90fd\u80fd\u63a5\u624b**\uff1a4 \u4e2a module \u90fd\u770b\u4e0d\u8d77\u6765\u8ddf\u8001 crate \u4e00\u6837\uff0c\u8001 crate \u8def\u5f84 0 \u6539
- **O-5 \u4e0d\u5047\u88c5**\uff1ashim `pub use` 0 \u4ee3\u7801\uff0c\u80fd\u770b\u51fa 1.1 \u4e3b\u4f53\u5728 telemetry

## \u540e\u7eed

- B2\uff1a`apeireth-pipeline::tool_loop` \u771f\u63a5 TUI / Web / \u684c\u9762
- B3\uff1aMCP 3 \u4e2a ResourceServer \u771f\u63a5\uff08File/Organ/Convention\uff09
- B4\uff1aCouncilMember \u8de8 5 provider \u534f\u5546 demo
- B5-B9\uff1aCI yaml / Memory Provider / OAuth device_code / Graph cognition / workspace 1.1