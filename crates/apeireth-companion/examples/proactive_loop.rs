//! 主动涌现循环 demo — 机制自转的样子.
//!
//! 真: Bond(关系) + EmergenceLoop(机制) + ConsoleDelivery(送达).
//! 诚实: 交互是「演示的」, 不是接真用户/真通道; 生产用 ProactiveDriver + lark/通知.

use apeireth_companion::emergence::{Boundaries, ConsoleDelivery, Delivery, EmergenceLoop, Feedback};
use apeireth_companion::{Bond, BondStage};
use chrono::{TimeZone, Utc};

fn at(day: u32, h: u32, m: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, day, h, m, 0).single().unwrap()
}

#[tokio::main]
async fn main() {
    let mut bond = Bond::new();
    bond.evolve(BondStage::Trusted, 0.6);

    let b = Boundaries {
        max_initiatives_per_day: 1,
        ..Default::default()
    };
    let mut l = EmergenceLoop::new(bond, b);
    let delivery = ConsoleDelivery;

    println!("--- 第 15 天 08:40, 尚无观察 ---");
    assert!(l.tick(at(15, 8, 40), None).is_none());
    println!("(保持安静: 还没观察到你的作息)\n");

    for d in 9..=15 {
        l.observe_interaction(at(d, 8, 40));
    }
    println!("--- 已观察 7 天 08:40 作息, 第 16 天 08:40 心跳 ---\n");

    let init = l.tick(at(16, 8, 40), Some("你上周在改 council 的 bug".to_string()));
    match init {
        Some(i) => {
            let _ = delivery.deliver(&i).await;
        }
        None => println!("(未主动)"),
    }

    println!();
    let s = l.apply_feedback(Feedback::Responded, at(16, 8, 45));
    println!("你回了 → 自评 {:.2}, 关系深度 {:.2}\n", s.value, l.depth());

    println!("--- 同一天 09:00 再心跳 ---");
    let again = l.tick(at(16, 9, 0), None);
    println!("再次主动? {}", if again.is_some() { "是" } else { "否 (频率门禁)" });
}
