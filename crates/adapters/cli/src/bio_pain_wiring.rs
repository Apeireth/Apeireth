//! W3.3/W3.6: 痛觉/再生领域观察者的装配出口。
//!
//! 领域账本归 `apeireth-organ::nociception`；runtime 只认识通用完成观察口。
//! 本模块只做两件事：按开关构造观察者、把自述 probe 的读取口绑定到同一个
//! `Arc` 实例。开关关 = 不构造 observer，读取口返回同口径空快照。

use std::sync::Arc;

use apeireth_organ::nociception::{BioPainSnapshot, NociceptionObserver};
use apeireth_runtime_assembly::canonical::production::BioProbeSlots;

/// 按开关构造观察者；关闭 = `None` = 内核零接线。
pub(crate) fn observer_from_switch(enabled: bool) -> Option<Arc<NociceptionObserver>> {
    enabled.then(|| Arc::new(NociceptionObserver::new()))
}

/// 把痛觉/再生读取口绑定到装配时装入 runtime 的同一观察者实例。
///
/// 未开启时仍提供 `enabled=false` 的空快照（旧口径），不假装已接线，也不读
/// 任何运行时状态。
pub(crate) fn bind_reader(slots: &BioProbeSlots, observer: Option<&Arc<NociceptionObserver>>) {
    match observer {
        Some(observer) => {
            let observer = Arc::clone(observer);
            slots.fill_pain_reader(Arc::new(move || Ok(observer.snapshot())));
        }
        None => {
            slots.fill_pain_reader(Arc::new(|| Ok(BioPainSnapshot::default())));
        }
    }
}
