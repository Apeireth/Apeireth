//! ⭐ **panic 兜底钩子** —— 台账 L1-G8 第二片。
//!
//! ## 解决什么
//!
//! wasm 里 `panic` 会**直接终止**执行：
//! - `panic=abort`（体积优化常见配置）⇒ 进程立刻 abort，页面停在**最后一帧**；
//! - `panic=unwind` ⇒ 若无人捕获，栈展开到宿主边界，同样白屏。
//!
//! 两种情况用户看到的都是**一片空白/冻结**，而且**没有任何错误信息**
//! ⇒ ⇒ 排查只能靠控制台，产品里等于没有。
//!
//! 本模块把 panic **变成可见事件**：装一个全局 hook ⇒ 记录消息 + 调**宿主提供的上报器**
//! ⇒ ⇒ 宿主在上报器里把消息写到界面（例如复用 `Renderer::set_error_boundary` 的兜底视图）
//! ⇒ ⇒ 「一崩全白」变成「一崩可见」。
//!
//! ## 边界（0 装纪律）
//!
//! - 本模块**不**决定兜底界面长什么样（视觉属 L3）⇒ 只提供消息与回调；
//! - 本模块**不**保证 panic 之后还能继续渲染（abort 模式下不能）⇒ 它保证的是
//!   **消息可达**，而不是「恢复运行」；
//! - 全局 hook 是**进程级**的 ⇒ 一个进程只能装一次；重复安装返回 `Err`。
//!
//! ## 用法（宿主）
//!
//! ```ignore
//! apeireth_ui_dom::panic_guard::install(Box::new(|message| {
//!     // 在浏览器里：把它写进 DOM（同步！），或用 `renderer.set_error_boundary(..)` 的兜底视图。
//!     web_sys::console::error_1(&message.into());
//! }));
//! ```

use std::sync::{Mutex, OnceLock};

type Reporter = Box<dyn Fn(&str) + Send + Sync>;

fn reporter_slot() -> &'static Mutex<Option<Reporter>> {
    static SLOT: OnceLock<Mutex<Option<Reporter>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

fn message_slot() -> &'static Mutex<Option<String>> {
    static SLOT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// 安装全局 panic hook（**进程级，只能装一次**）。
///
/// 返回 `Err` 的两种情况：
/// - 已经装过（`OnceLock` 已初始化）⇒ 重复安装会覆盖别人的 hook，语义混乱；
/// - hook 内部锁被毒化（说明上次 panic 发生在我们的锁内）。
pub fn install(reporter: Reporter) -> Result<(), &'static str> {
    {
        let mut slot = reporter_slot()
            .lock()
            .map_err(|_| "panic_guard: 上报器槽被毒化")?;
        if slot.is_some() {
            return Err("panic_guard: 已经安装过（进程级 hook 只能装一次）");
        }
        *slot = Some(reporter);
    }
    std::panic::set_hook(Box::new(move |info| {
        let message = format!("{info}");
        // ① 记录（即使上报器崩了也要留痕）
        if let Ok(mut slot) = message_slot().lock() {
            *slot = Some(message.clone());
        }
        // ② 上报（上报器自身 panic 不得再触发 hook 递归 ⇒ 捕获并忽略）
        if let Ok(slot) = reporter_slot().lock() {
            if let Some(reporter) = slot.as_ref() {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    reporter(&message);
                }));
            }
        }
    }));
    Ok(())
}

/// 最近一次 panic 的消息（不取走）。
pub fn last_panic() -> Option<String> {
    message_slot().lock().ok().and_then(|slot| slot.clone())
}

/// 取走最近一次 panic 的消息（宿主上报后用这个，避免重复上报）。
pub fn take_panic() -> Option<String> {
    message_slot().lock().ok().and_then(|mut slot| slot.take())
}

/// 是否已经安装过 hook。
pub fn is_installed() -> bool {
    reporter_slot()
        .lock()
        .map(|slot| slot.is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// 上报器收到的全部消息（**用容器**而不是计数器：同进程内可能有其它测试
    /// 触发 `#[should_panic]` ⇒ ⇒ 消息槽会被覆盖，用容器才能稳定断言）。
    static REPORTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    /// ⭐ **panic 被记录且被上报**（「一崩全白」→「一崩可见」）。
    ///
    /// ⚠️ 本测试安装**进程级** hook ⇒ 只允许安装一次，因此本模块的所有断言都在这一个
    /// `#[test]` 里完成（避免并行测试各自安装造成「已安装」错误）。
    /// ⚠️ 断言用「容器里包含我们的文本」而不是计数器/单个槽位：
    /// 同进程其它测试若触发 panic 会复用同一个 hook，用槽位断言会**间歇性失败**。
    #[test]
    fn panic_is_recorded_and_reported() {
        assert!(!is_installed(), "测试进程内不应有人先装过 hook");

        // ① 安装：上报器把消息收进容器（真实宿主会写 DOM / 日志）
        install(Box::new(|message| {
            if let Ok(mut slot) = REPORTED.lock() {
                slot.push(message.to_owned());
            }
        }))
        .expect("首次安装必须成功");

        // ② 重复安装必须被拒绝（进程级 hook 不能覆盖别人的）
        assert!(install(Box::new(|_| {})).is_err(), "重复安装必须报错");

        // ③ 触发一次 panic 并捕获（`catch_unwind` 让测试继续跑）。
        //   ⚠️ **不能**在这里 `take_hook()` —— 那会把刚装的 hook 摘掉，
        //   于是 panic 走默认 hook ⇒ 记录不到 ⇒ 测试会"通过"但什么也没验到。
        let _ = std::panic::catch_unwind(|| panic!("引擎崩了：测试注入"));

        // ④ 上报器必须收到含我们文本的消息
        let reported = REPORTED.lock().expect("上报容器可用").clone();
        assert!(
            reported
                .iter()
                .any(|item| item.contains("引擎崩了：测试注入")),
            "上报器必须收到 panic 消息，实际：{reported:?}"
        );

        // ⑤ 记录槽可用（同进程可能有别的 panic 覆盖 ⇒ 只断言"有记录"）
        assert!(last_panic().is_some(), "panic 必须留下记录");

        // ⑥ 取走后清空（避免重复上报）
        assert!(take_panic().is_some());
        assert!(last_panic().is_none(), "取走后应为 None");
    }
}
