//! 绑定注册表的**按根目录单例** (B3b-1 批)。
//!
//! 回合闸 (运行时) 与合同面 (HTTP) 必须读**同一份**联系人/工作区事实:
//! 这两个文件后备注册表在 `open` 时把档读进内存, 之后只改内存 —— 两个实例
//! 各读各的, 闸就看不见合同面刚写下的事实 (B3b-1 裁决 R8)。
//!
//! 所以这里按**根目录**缓存实例: 同一个根 ⇒ 同一个实例 (谁先开谁拿着);
//! 换根 (测试用临时目录) ⇒ 另一个实例, 互不干扰。
//!
//! 开不出 (磁盘不可写等) 不静默降级: 失败也进缓存 ⇒ 闸与合同面看到**同一个**
//! 失败 —— 闸对已绑定会话 fail-closed 拒绝, 合同面装配期让启动失败。
//!
//! 根目录口径: 联系人落 `<root>/agents/contacts/<id>/contact.json`,
//! 工作区落 `<root>/workspaces/` (B3b-1 裁决 R14)。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use apeireth_core::agent_contact::{ContactRegistry, FileContactRegistry};
use apeireth_runtime::canonical::workspace::{FileWorkspaceRegistry, WorkspaceRegistry};
use apeireth_runtime::canonical::RuntimeBuilder;

/// 一个根目录上的注册表槽位: 开着了 (拿到实例) 或开不出 (记下原因)。
enum Slot<T: ?Sized> {
    Ready(Arc<T>),
    Failed(String),
}

/// 联系人注册表缓存 (按根目录)。
static CONTACTS: OnceLock<Mutex<BTreeMap<PathBuf, Slot<dyn ContactRegistry>>>> = OnceLock::new();
/// 工作区注册表缓存 (按根目录)。
static WORKSPACES: OnceLock<Mutex<BTreeMap<PathBuf, Slot<dyn WorkspaceRegistry>>>> =
    OnceLock::new();

/// 取某个根目录上的联系人注册表 (同根 ⇒ 同实例)。
pub(crate) fn contacts(root: &Path) -> Result<Arc<dyn ContactRegistry>, String> {
    let cache = CONTACTS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut guard = cache.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(slot) = guard.get(root) {
        return match slot {
            Slot::Ready(registry) => Ok(Arc::clone(registry)),
            Slot::Failed(reason) => Err(reason.clone()),
        };
    }
    match FileContactRegistry::open(root) {
        Ok(registry) => {
            let registry: Arc<dyn ContactRegistry> = Arc::new(registry);
            guard.insert(root.to_path_buf(), Slot::Ready(Arc::clone(&registry)));
            Ok(registry)
        }
        Err(error) => {
            let reason = format!("打开联系人注册表失败: {error}");
            guard.insert(root.to_path_buf(), Slot::Failed(reason.clone()));
            Err(reason)
        }
    }
}

/// 取某个根目录上的工作区注册表 (同根 ⇒ 同实例)。
pub(crate) fn workspaces(root: &Path) -> Result<Arc<dyn WorkspaceRegistry>, String> {
    let cache = WORKSPACES.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut guard = cache.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(slot) = guard.get(root) {
        return match slot {
            Slot::Ready(registry) => Ok(Arc::clone(registry)),
            Slot::Failed(reason) => Err(reason.clone()),
        };
    }
    match FileWorkspaceRegistry::open(root) {
        Ok(registry) => {
            let registry: Arc<dyn WorkspaceRegistry> = Arc::new(registry);
            guard.insert(root.to_path_buf(), Slot::Ready(Arc::clone(&registry)));
            Ok(registry)
        }
        Err(error) => {
            let reason = format!("打开工作区注册表失败: {error}");
            guard.insert(root.to_path_buf(), Slot::Failed(reason.clone()));
            Err(reason)
        }
    }
}

/// 把注册表装进运行时 (回合闸用)。
///
/// 与合同面共用同一实例 (同一个根 ⇒ 同一个缓存槽 ⇒ 闸读的就是合同面刚写的
/// 事实)。开不出 ⇒ 不装: 闸对未绑定会话照常放行, 对已绑定会话 fail-closed
/// 拒绝 (B3b-1 裁决 R9) —— 合同面装配期仍会让启动失败, 两处都不假装成功。
pub(crate) fn inject(builder: RuntimeBuilder) -> RuntimeBuilder {
    let root = crate::default_panel_data_dir();
    builder
        .with_contact_registry(contacts(&root).ok())
        .with_workspace_registry(workspaces(&root).ok())
}
