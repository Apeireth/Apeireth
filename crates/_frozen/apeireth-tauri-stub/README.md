# `apeireth-tauri-stub` — DEPRECATED Tauri 2 参考实现 ⚠️

> **状态**: ⚠️ **DEPRECATED** (V2 Day 1 Step 1.3, `docs/v2-strategy/05-EXECUTION-NOW.md`)
>
> **本文档作用**: 给 R19 战役(及未来 worker)留一份 Tauri 2 desktop 集成参考样例。

---

## 为什么不进产品

- 本 crate 原名 `apeireth-desktop`，R17 战役 2-3 创建,目的只在让 workspace 可 build 时保留 Tauri 2 deps 占位。
- R17 砍 Tauri 前端战役已经过去,R19 战役计划用真前端(技术栈未定)。
- 现作为 **参考实现**保留:**不在产品里**,不进 CI artifact,不影响下游依赖。

---

## 给 R19 worker 的注意点

1. `src/lib.rs` 只有 1 个常量 `R19_DESKTOP_STUB = true` —— R19 worker 替换此 stub 时,应同时把 `apeireth-tauri-stub` 改名回 `apeireth-desktop`(或重新规划命名)。
2. `src/main.rs` 是 26KB 真 Tauri 代码,**可当作集成示例**。
   - 调 `apeireth_central::LEGAL_TRANSITIONS`、`apeireth_cognition::run_cycle`、`apeireth_memory::SqliteMemoryStore` 等下游 crate。
   - 但要遵守 R11 LOCKED 边界:本文件只调下游 crate 的公开 API,不修改 R11 LOCKED enum / 转换矩阵 / 8 项不修改承诺。
3. `tauri.conf.json` + `gen/schemas/` + `icons/` 是 Tauri 2 标准结构,**不要删**,R19 worker 直接复用。

---

## 升级路径 (R19 worker 接管时)

```bash
# 1. 把名字改回来
git mv crates/apeireth-tauri-stub crates/apeireth-desktop

# 2. 改 Cargo.toml: package.name = "apeireth-desktop"(去掉 DEPRECATED marker)
# 3. 改 workspace Cargo.toml: 把 "crates/apeireth-tauri-stub" 改回 "crates/apeireth-desktop"
# 4. 跑 cargo check --workspace + cargo build -p apeireth-desktop
```

---

**Latest check**: `cargo check -p apeireth-tauri-stub` 0 error (V2 Day 1)。
