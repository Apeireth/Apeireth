# Kani 形式化验证 — 本地运行指南

> 适用: `apeireth-formal` crate (V2 战区 5, `docs/v2-strategy/03 §4A`).
> 范围: 本文档只讲"如何在自己机器跑通 `cargo kani --harness double_onion_sample`".
> 架构与不变量设计见 crate 内 `src/lib.rs` doc 注释.

---

## 1. 什么是 Kani / cargo-kani

Kani 是 AWS 开源的 Rust 模型检查器 (基于 CBMC):
- **符号执行** + **有界模型检查**, 完备覆盖**所有**输入(非抽样的)
- 对**所有非确定性输入**自动探索(`kani::any()`)
- 给出反例轨迹(若不变量失败)
- 与 cargo 集成 (`cargo kani`)

代价: 慢 + 内存大. 单个简单 harness 通常 1–5 分钟, 复杂状态空间可能数小时.

---

## 2. 安装 (一次性)

### 2.1 系统要求

- Linux / macOS / **WSL2** (Windows 原生**不**支持, 必须 WSL2)
- Rust stable ≥ 1.80 (本 workspace 要求)
- ~5 GB 磁盘 (CBMC 依赖 + 缓存)
- ~4 GB 内存跑 harness

### 2.2 安装 Kani

```bash
# 1. Rust 工具链 (如未装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Kani (官方推荐: 用 cargo-binstall 或预编译 tarball)
cargo install --locked kani-verifier
cargo install --locked cargo-kani

# 3. 验证安装
cargo kani --version
```

> **常见坑**: 不要装到 nightly toolchain. Kani 自动下载自己的 nightly.
> 不要用 `cargo install kani` (老命令, 已废弃).

---

## 3. 跑通 sample 不变量

### 3.1 在本仓库根目录

```bash
cd apeireth-rust
cargo kani -p apeireth-formal --harness double_onion_sample
```

输出示例 (成功):

```
Checking harness invariants::double_onion_sample::double_onion_sample...
Complete - 1 successfully verified harnesses
SUMMARY: 1of1 harnesses verified
```

### 3.2 直接进入 crate

```bash
cd crates/apeireth-formal
cargo kani --harness double_onion_sample
```

### 3.3 可选 flag

```bash
# 看 Kani 内部 trace / CBMC args
cargo kani -p apeireth-formal --harness double_onion_sample --verbose

# 给 harness 限定 unwind bound (本 sample 不需要, 默认 1 即可)
cargo kani -p apeireth-formal --harness double_onion_sample --unwind 1
```

---

## 4. CI 状态

`.github/workflows/kani.yml` 自动跑上述命令.
Kani 在 GitHub Actions ubuntu-latest 上需要 ~3 分钟 cold cache / ~1 分钟 warm.

---

## 5. 写新不变量 (模板)

1. 在 `crates/apeireth-formal/src/invariants/` 下新建 `<your_invariant>.rs`
2. 模板 (~30 LOC, 见 `double_onion_sample.rs`):
   ```rust
   use crate::{..., PermissionLayerConfig};

   #[cfg_attr(kani, kani::proof)]
   pub fn <your_harness_name>() {
       let cfg = nondet_config();
       assert!(your_invariant_fn(cfg));
   }

   #[cfg(kani)] fn nondet_config() -> PermissionLayerConfig { kani::any() }
   #[cfg(not(kani))] fn nondet_config() -> PermissionLayerConfig {
       PermissionLayerConfig::new(0, true)
   }

   pub fn sanity_check() -> bool { ... }

   #[cfg(test)] mod tests { ... }
   ```
3. 在 `src/invariants/mod.rs` 注册: `pub mod <your_invariant>;` + 在 `run_all()` 调用
4. 跑本地: `cargo kani -p apeireth-formal --harness <your_harness_name>`

---

## 6. 已知陷阱

| 陷阱 | 说明 |
|---|---|
| **String / Vec / HashMap** 入参 | Kani 面对堆类型状态爆炸, 用 POD (u8 / u32 / bool / 固定 array) |
| **浮点** (`f32` / `f64`) | Kani 支持但成本高, 用整数 + 定点更好 |
| **递归 / 任意循环** | 必须配 `--unwind N`; 否则 CBMC 会跑死 |
| **`unsafe`** | 本 crate 已 `deny(unsafe_code)`; 其它 crate 也应避免在 harness 路径用 |
| **大 N (e.g. 100 步)** | Kani 单 harness 可能跑几小时. 拆小 harness |

---

## 7. 引用

- Kani 官方: <https://model-checking.github.io/kani/>
- CBMC 后端: <https://www.cprover.org/cbmc/>
- 论文: "Kani: A New Rust Verifier" (AWS, 2024)
- 项目文档: `docs/v2-strategy/03-EXTREME-PLAN.md §4A`