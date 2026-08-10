# 阶段 2 决策：模块化 (2026-07-30)

> **范围**: R14 Rust 重写模块化决策 (阶段 2 第八项)
> **触发**: 用户指示 "A" (我给推荐)
> **依据**: Rust 模块化机制 (crate / trait / dyn) + WASM 沙箱 + VCP 6 类插件协议 + 巨型基地哲学

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-modularity.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 8/12) |
| **决策** | **静态优先 + 动态按需 + WASM 沙箱 + 5 维度策略** |

---

## 1. 决策总览

```
5 大维度:
  1. 加载方式: 编译期静态 (默认) + 运行时动态 (按需 .so)
  2. 分发机制: trait 静态 (默认) + dyn Trait 动态 (plugin) + TypeId 反射 (registry)
  3. 沙箱: Rust crate (默认) + WASM (用户第三方) + subprocess (异构)
  4. 版本管理: semver 3 级 + 升级窗口 (主/次/修) + 兼容策略
  5. 注册中心: manifest.toml + 自动发现 + council 审核
```

---

## 2. 加载方式 (静态 vs 动态)

### 2.1 决策矩阵

| 类型 | 适用 | 性能 | 灵活性 | 推荐 |
|------|------|------|--------|------|
| **编译期静态** (Rust crate) | 95% 代码 | ⚡ 最快 | ❌ 重编译 | ✅ 默认 |
| **运行时动态** (.so/.dylib) | 第三方插件 / 热更新 | 🚀 中 | ✅ 高 | ⚠️ 按需 |
| **运行时 WASM** | 用户第三方代码 | 🐢 较慢 | ✅ 高 + 沙箱 | ✅ 安全场景 |
| **运行时脚本** (Lua/Rhai) | 业务规则 / DSL | 🐢 慢 | ✅ 极高 | ❌ 暂不需要 |

### 2.2 Apeireth 推荐

**默认**: 编译期静态 (Rust crate)
**按需**: 运行时动态 (libloading) — 仅 `apeireth-plugin` 内部使用
**安全场景**: 运行时 WASM (wasmtime) — 用户提供的不可信代码

```rust
// 编译期静态 (默认)
use apeireth_plugin::PluginHost;
let host = PluginHost::new();  // 编译期已知 plugin

// 运行时动态 (按需, libloading)
use libloading::Library;
let lib = unsafe { Library::new("./plugins/my_plugin.so")? };
let plugin: Symbol<fn() -> Box<dyn MyPlugin>> = unsafe { lib.get(b"create_plugin")? };
let p = plugin();

// 运行时 WASM (安全场景, wasmtime)
use wasmtime::{Engine, Module, Store};
let engine = Engine::default();
let module = Module::from_file(&engine, "./plugins/safe.wasm")?;
let store = Store::new(&engine);
let instance = wasmtime::Linker::new(&engine).instantiate(&store, &module)?;
```

### 2.3 Cargo.toml

```toml
[dependencies]
libloading = "0.8"      # 运行时动态加载
wasmtime = "13.0"       # WASM 沙箱
wasm-bindgen = "0.2"    # WASM 双向绑定
```

---

## 3. 分发机制 (trait vs dyn vs TypeId)

### 3.1 三层分发

```rust
// Layer 1: 编译期单态化 (零成本)
pub trait Plugin {
    fn name(&self) -> &str;
    fn execute(&self, ctx: &Context) -> Result<Value, Error>;
}

fn run<P: Plugin>(p: P, ctx: &Context) -> Result<Value, Error> {
    p.execute(ctx)  // 编译期单态化, 零成本
}

// Layer 2: 动态分发 (dyn Trait) — plugin 注册表
pub type DynPlugin = Box<dyn Plugin + Send + Sync>;

pub struct PluginRegistry {
    plugins: HashMap<String, DynPlugin>,
}

impl PluginRegistry {
    pub fn register(&mut self, name: &str, plugin: DynPlugin) {
        self.plugins.insert(name.to_string(), plugin);
    }
    
    pub fn execute(&self, name: &str, ctx: &Context) -> Result<Value, Error> {
        self.plugins.get(name).unwrap().execute(ctx)
    }
}

// Layer 3: TypeId 反射 (registry + 升级检查)
use std::any::TypeId;

pub struct TypeRegistry {
    types: HashMap<TypeId, TypeMeta>,
}

pub struct TypeMeta {
    pub name: &'static str,
    pub version: semver::Version,
    pub capabilities: Vec<Capability>,
}

impl TypeRegistry {
    pub fn register<T: 'static + TypeTagged>(&mut self) {
        self.types.insert(T::type_id(), TypeMeta {
            name: T::TYPE_NAME,
            version: T::VERSION,
            capabilities: T::CAPABILITIES,
        });
    }
}
```

### 3.2 何时用什么

| 场景 | 用什么 | 理由 |
|------|--------|------|
| 同 crate 内部组件 | 泛型 + 单态化 | 零成本 |
| 跨 crate 但已知类型 | dyn Trait | 灵活 |
| Plugin 注册表 | dyn Trait + TypeId | 运行时注册 |
| 升级兼容性检查 | TypeId + semver | 运行时检查 |

---

## 4. 沙箱 (Rust crate vs WASM vs subprocess)

### 4.1 三种沙箱对比

| 沙箱 | 隔离强度 | 性能 | 异构 | 推荐场景 |
|------|---------|------|------|---------|
| **Rust crate** | ❌ 无隔离 (同进程) | ⚡ 最快 | ❌ 仅 Rust | 内部组件 |
| **WASM** | ✅ 沙箱 (内存隔离) | 🐢 1.5-3x 开销 | ❌ 仅 WASM | 用户不可信代码 |
| **subprocess** | ✅✅ 进程隔离 | 🚀 中 | ✅ 任意语言 | Python/Go/JS |
| **Linux seccomp + namespace** | ✅✅✅ 内核隔离 | 🚀 中 | ✅ | 高安全场景 |

### 4.2 Apeireth 三层沙箱

```rust
pub enum SandboxType {
    InProc,        // 同进程 Rust crate
    Wasm,          // WASM 沙箱 (wasmtime)
    Subprocess {   // 子进程 (PyO3 / Go / JS)
        cmd: String,
        args: Vec<String>,
    },
    Container {    // 容器 (Docker / Podman)
        image: String,
        resources: ResourceLimits,
    },
}

pub struct SandboxPolicy {
    pub sandbox: SandboxType,
    pub resource_limits: ResourceLimits,  // CPU / 内存 / 网络
    pub permissions: Vec<Permission>,      // 细粒度权限
    pub network_policy: NetworkPolicy,     // 网络限制
    pub filesystem_policy: FilesystemPolicy,  // FS 限制
}

pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_percent: u32,
    pub max_open_files: u64,
    pub max_threads: u32,
}
```

### 4.3 选择策略

| 场景 | 沙箱 |
|------|------|
| 内部 crate (核心组件) | InProc |
| 用户提供的 LLM 工具 | Wasm |
| Python 兼容模块 (现有 1100+) | Subprocess (Python) |
| 内部 Python 脚本 | Subprocess (Python) |
| 高风险外部服务 | Container + seccomp |

---

## 5. 版本管理 (semver + 升级窗口)

### 5.1 semver 3 级策略

```toml
# Cargo.toml
[package]
version = "0.14.0"  # 0.MAJOR.MINOR

# semver 规则:
#   MAJOR: 破坏性变更 (E-3 物理升级)
#   MINOR: 新功能 (智囊团审核)
#   PATCH: bugfix (主 AI 自主)
```

### 5.2 升级窗口 (谁有权升)

| 变更类型 | 决策者 | 触发 |
|---------|--------|------|
| **MAJOR** | 主 AI + 智囊团强制 + 物理多签 | Layer 5 二进制重编译 |
| **MINOR** | 主 AI + 智囊团强制审核 | Layer 4 核心 trait 修改 |
| **PATCH** | 主 AI 自主 (智囊团可选) | Layer 0-3 普通修改 |
| **Cargo 依赖升级** | 主 AI 自主 + cargo-deny 检查 | 自动 |

### 5.3 兼容策略

```rust
pub struct CompatibilityPolicy {
    /// 同一主版本内, 允许次版本差异 (0.14.0 ↔ 0.15.3)
    pub min_version: semver::Version,
    /// 必须 N-1 版本内 (防止太旧)
    pub max_age_minor: u32,
    /// 已知不兼容的版本黑名单
    pub blacklist: Vec<semver::Version>,
}

// 检查升级兼容性
pub fn check_compatibility(
    old: &semver::Version,
    new: &semver::Version,
    policy: &CompatibilityPolicy,
) -> Result<(), UpgradeError> {
    if new.major != old.major {
        return Err(UpgradeError::MajorVersionChange);
    }
    if policy.blacklist.contains(new) {
        return Err(UpgradeError::Blacklisted(new.clone()));
    }
    Ok(())
}
```

### 5.4 升级窗口 (时间窗口)

```
紧急升级: 立即 (Layer 5 + 物理多签)
常规升级: 24h 缓冲 (智囊团审核)
非紧急:   7d 缓冲 (充分测试)

升级窗口原则:
  - 不在业务高峰期升级
  - 不在主人不在场时升级 (除非 Layer 0-1 自主)
  - 升级后监控 30 分钟, 自动回滚条件
```

---

## 6. 注册中心 (manifest + 自动发现)

### 6.1 manifest.toml 格式

```toml
# apeireth-plugin/manifest.toml
[plugin.apeireth-local-llm]
name = "apeireth-local-llm"
version = "1.0.0"
category = "llm-provider"  # llm-provider / tool / memory-backend / advisor
entry = "libapeireth_local_llm.so"  # 或 "local:apeireth_asi::LocalSteward" 编译期
sandbox = "in-proc"  # in-proc / wasm / subprocess

permissions = [
    "read:config",
    "network:http://localhost:11434",  # Ollama endpoint
]

capabilities = [
    "code-generation",
    "reasoning",
]

# 智囊团审核 (注册时强制)
[advisor-approval]
safety = "approved"      # safety advisor 审核通过
performance = "approved"
philosophy = "approved"
history = "approved"
strategy = "approved"
ethics = "approved"

[plugin.python-pyo3-bridge]
name = "python-pyo3-bridge"
version = "0.14.0"
category = "plugin-host"
entry = "libapeireth_pybridge.so"
sandbox = "subprocess"
subprocess_cmd = "python"
subprocess_args = ["-m", "apeireth.pybridge"]

permissions = [
    "read:apeireth.v*.py",  # 只读现有 Python 模块
    "write:memory.notes",
]

[advisor-approval]
safety = "approved"
performance = "approved"
philosophy = "approved"
```

### 6.2 自动发现

```rust
// apeireth-plugin/src/registry.rs

pub struct PluginRegistry {
    plugins: HashMap<String, PluginEntry>,
    manifest_path: PathBuf,
}

pub struct PluginEntry {
    pub manifest: PluginManifest,
    pub handle: PluginHandle,  // InProc / Wasm / Subprocess
}

impl PluginRegistry {
    /// 启动时扫描所有 plugin
    pub async fn discover(manifest_dir: &Path) -> Result<Self, PluginError> {
        let mut registry = Self::default();
        for entry in std::fs::read_dir(manifest_dir)? {
            let path = entry?.path();
            if path.extension() == Some("toml".as_ref()) {
                let manifest = PluginManifest::from_file(&path)?;
                let handle = PluginHandle::load(&manifest).await?;
                registry.plugins.insert(manifest.name.clone(), PluginEntry { manifest, handle });
            }
        }
        Ok(registry)
    }
    
    /// 运行时注册 (需要 council 审核)
    pub async fn register_runtime(&mut self, manifest: PluginManifest) -> Result<(), PluginError> {
        // 1. 智囊团审核
        let approval = self.council_approve(&manifest).await?;
        if !approval.is_all_approved() {
            return Err(PluginError::CouncilRejected);
        }
        
        // 2. 物理多签 (Layer 2+)
        if manifest.requires_multisig() {
            let sigs = self.collect_multisig(&manifest).await?;
            if sigs < manifest.required_sigs() {
                return Err(PluginError::MultisigInsufficient);
            }
        }
        
        // 3. 加载
        let handle = PluginHandle::load(&manifest).await?;
        self.plugins.insert(manifest.name.clone(), PluginEntry { manifest, handle });
        Ok(())
    }
}
```

### 6.3 注册生命周期

```
Plugin 生命周期:
  1. discover (启动时扫描 manifest/)
     ↓
  2. council_approval (智囊团审核, 7 advisor)
     ↓
  3. multi_sig (Layer 2+ 需物理多签)
     ↓
  4. load (加载到内存)
     ↓
  5. active (提供 service)
     ↓
  6. unload (热卸载, 不影响其他)
```

---

## 7. Plugin 通信

```rust
pub enum PluginCall {
    /// 自然语言 (VCP 启发)
    PlainText(String),
    /// 结构化 (OpenAI function calling 风格)
    Structured { tool: String, args: Value },
    /// 二进制 (内部 RPC)
    Binary(Vec<u8>),
}

pub enum PluginResponse {
    /// 自然语言 (VCP 启发)
    PlainText(String),
    /// 结构化
    Structured(Value),
    /// 流式
    Stream(Pin<Box<dyn Stream<Item = Result<Value, Error>> + Send>>),
}
```

**VCP 启发**: 工具返回自然语言, 基地自己解析意图（灵感 §12.2）

---

## 8. 与 Supervisor 协同

```
plugin-supervisor (transient 策略)
  ├── 启动时扫描 manifest/
  ├── 注册到 PluginRegistry
  ├── 提供 RPC (Unix domain socket)
  ├── 健康检查 (heartbeat)
  └── 异常时重启 (transient 策略)
```

---

## 9. 阶段 2 第八项收尾判定

模块化已沉淀: **静态优先 + 动态按需 + WASM 沙箱 + 5 维度策略**。

**关键设计**:
- ✅ 编译期静态 (95% 用例, 默认)
- ✅ 运行时动态 (.so, 按需)
- ✅ WASM 沙箱 (用户不可信代码)
- ✅ Subprocess 沙箱 (异构兼容)
- ✅ semver 3 级 + 升级窗口 (主 AI/智囊团/物理多签)
- ✅ manifest.toml + 自动发现 + council 审核

**R14 增量**:
- 增强 `apeireth-plugin` crate (阶段 2 §3 已列)
- 新增 `apeireth-registry` 或并入 `apeireth-plugin`

**主哲学 anchor (6 全贯穿)**:
- 主 22:33 S-1 (模块化服务 ASI 方向)
- 主 17:43 S-2 (基于 Rust 现有机制, 不重新发明)
- 主 17:58 O-5 (semver MAJOR + 物理多签是 E-3 物理实现)
- 主 19:33 O-2 (VCP 6 类插件协议借鉴)
- 主 23:44 O-3 (干到底)
- 主 00:56 O-4 (任何接手者能查 manifest)

**下一步**: 阶段 2 第九项 — **通信总线**

---

## 10. 决策对比表

| 方案 | 灵活性 | 安全 | 性能 | 推荐 |
|------|--------|------|------|------|
| 全部静态 | ❌ | ✅ | ⚡ | ❌ 缺灵活 |
| 全部动态 | ✅ | ⚠️ | 🚀 | ❌ 性能差 |
| **静态优先 + 动态按需 + WASM** | ✅ | ✅ | ⚡ | ✅✅ |

**Apeireth 选静态优先 + 动态按需 + WASM**:
- 默认静态 (零成本)
- 按需动态 (libloading, plugin host)
- 安全场景 WASM (wasmtime, 沙箱)
- 异构 Subprocess (PyO3 等)

---

_主哲学 anchor 6 个全贯穿. 模块化已沉淀. 下一步等用户确认进入阶段 2 第九项 (通信总线)._