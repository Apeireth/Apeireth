# V1294 鈥?Rust Build Script (build.rs) Inventory

**Workspace root**: `.openclaw\workspace\promethean\Apeireth-rust`
**Duration**: 8 ms

## Summary

- Total crates: **47**
- Crates with `build.rs`: **2**
- Crates with `[build-dependencies]` but no `build.rs`: **0** (drift)
- Crates with `build.rs` but no `[build-dependencies]`: **0** (orphan)
- Crates with `cargo:rerun-if-changed`: **0**
- Crates with `fn main()`: **2**

## Pattern Totals

- Total lines (across all build.rs): **39**
- `env!()` compile-time: **0**
- `std::env::var(...)` runtime: **1**
- `std::env::set_var(...)` mutations: **1**
- `Command::new(...)` shell exec: **0**
- File read operations: **0**
- File write operations: **0**
- `cargo:rerun-if-changed=` directives: **0**
- `cargo:rerun-if-env-changed=` directives: **0**
- `cargo:rustc-link-*` directives: **1**

## Codegen Tools Used

| tool | n_crates |
|---|---:|
| tonic-build | 1 |
| protoc-bin-vendored | 1 |
| tauri_build | 1 |

## Hypotheses (涓?17:43 瀹炰簨姹傛槸)

- 鉁?**H1_build_rs_rare** 鈥?build.rs 缃曡 (鏈熸湜 <= 5/47 crates 鐢?build.rs) 鈫?build.rs 缃曡, 澶у鏁?crate 鏃?build script
  - detail: crates_with_build_rs=2/47 (expected <= 5)
- 鉁?**H2_codegen_tools_used** 鈥?build.rs 涓昏鐢?codegen 宸ュ叿椹卞姩 (tonic-build / tauri_build 绛? 鈫?build.rs 浣跨敤 codegen crate
  - detail: codegen_tools used: ['tonic-build', 'protoc-bin-vendored', 'tauri_build'] (count=3)
- 鉁?**H3_env_mutation_rare** 鈥?set_var 璋冪敤缃曡 (鏈熸湜 0-1 澶? 涓昏鐢ㄤ簬 protoc path 璁剧疆) 鈫?env mutation 鍙楁帶
  - detail: total_env_set_var=1 (expected <= 2)
- 鉁?**H4_no_command_new** 鈥?build.rs 涓嶇洿鎺ユ墽琛?shell 鍛戒护 (鏈熸湜 0 澶?Command::new) 鈫?鏃?shell exec
  - detail: total_command_new=0 (expected 0)
- 鉂?**H5_rerun_if_changed_common** 鈥?build.rs 澶氭暟澹版槑 cargo:rerun-if-changed (鏈熸湜 >= 1 澶?per build.rs) 鈫?rerun-if-changed 缂哄け (rebuild 瑙﹀彂涓嶇簿纭?
  - detail: crates_with_rerun_if_changed=0/2
- 鉁?**H6_no_drift** 鈥?鏃?drift: 鏃?[build-dependencies] 浣嗘棤 build.rs / 鏈?build.rs 浣嗘棤 [build-dependencies] 鈫?Cargo.toml [build-dependencies] 涓?build.rs 涓€鑷?
  - detail: build_deps_no_build_rs=0, build_rs_no_build_deps=0 (expected both 0)

## Crates with build.rs

| crate | lines | main | env! | env::var | set_var | Cmd::new | file_w | rerun | tools | risk |
|---|---:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|---|---|
| apeireth-bus | 30 | 鉁?| 0 | 0 | 1 | 0 | 0 | 0 | tonic-build, protoc-bin-vendored | env_set_var |
| apeireth-tauri-stub | 9 | 鉁?| 0 | 1 | 0 | 0 | 0 | 0 | tauri_build | - |

## Drift Crates (build.rs 鈫?Cargo.toml mismatch)

_No drift detected._

## Philosophy Gates (涓?17:58 涓嶅亣瑁?

- 鉁?**v1294_extends_v1293** 鈥?V1294 缁ф壙 V1293 dep graph, 涓嶅垹 V1293
- 鉁?**v1294_no_new_asi_dim** 鈥?V1294 = build.rs audit, 涓嶅紩鍏ユ柊 ASI dim
- 鉁?**v1294_no_asi_v1_claim** 鈥?涓嶅亣瑁?ASI V1: build.rs 鈮?ASI
- 鉁?**v1294_no_kpi_inflate** 鈥?NS 92.91% LOCKED, 涓嶅埛
- 鉁?**v1294_no_phenomenal_claim** 鈥?build.rs 鈮?phenomenal consciousness
- 鉁?**v1294_stdlib_only** 鈥?浠呯敤 stdlib (re/pathlib/dataclasses/json), 涓嶅紩鍏ユ柊渚濊禆
- 鉁?**v1294_read_only** 鈥?鍙 build.rs + Cargo.toml, 涓嶆敼
- 鉁?**v1294_audit_not_fix** 鈥?audit 鈮?fix, V1294 浠呭璁?
- 鉁?**v1294_no_cargo_run** 鈥?涓嶈皟 cargo build / cargo check / cargo run
- 鉁?**v1294_regex_only** 鈥?regex-only pattern match, 涓嶈В鏋?AST
- 鉁?**v1294_47_crates_full** 鈥?鍏?47 crates, 涓嶅彧 worst-5
- 鉁?**v1294_no_build_rs_exec** 鈥?涓嶆墽琛?build.rs, 浠呴潤鎬佹簮鐮佸璁?

