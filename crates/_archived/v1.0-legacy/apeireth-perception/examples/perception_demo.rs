//! apeireth-perception demo — 演示一次完整感知周期.
//!
//! 运行: `cargo run -p apeireth-perception --example perception_demo`
//!
//! 流程:
//!   1. 构造一批多模态输入 (文本 / 语音 / 视觉 / 触觉 / 命令)
//!   2. 用对应通道逐个处理 → PerceptionEvent
//!   3. Top-K 注意力过滤 → 保留最值得 cognition 关注的 K 条
//!   4. 阈值注意力过滤 → 二次过滤
//!   5. 汇总输出

use apeireth_perception::{
    batch_process, default_attention_threshold, default_top_k, pipeline, top_k_filter,
    validate_event, Attention, CommandChannel, CommandInput, PerceptionChannel, PerceptionEvent,
    PerceptionInput, SignalSource, TactileChannel, TactileInput, TextChannel, TextInput,
    TopKAttention, VisionChannel, VisionInput, VoiceChannel, VoiceInput,
};

fn main() {
    println!("=== apeireth-perception demo ===\n");

    // ============================================
    // 场景 1: 文本通道 + 阈值注意力
    // ============================================
    println!("[场景 1] 文本通道 (3 条) → 阈值注意力 (默认 0.5)");
    let ch = TextChannel;
    let inputs = vec![
        TextInput::new("hello", SignalSource::Cli).with_priority(0.3),
        TextInput::new("urgent: please review PR #42", SignalSource::Http).with_priority(0.9),
        TextInput::new("background log line", SignalSource::Internal).with_priority(0.2),
    ];
    let events = pipeline(&ch, inputs, default_attention_threshold());
    print_events("text", &events);

    // ============================================
    // 场景 2: Top-K 注意力 — 5 输入, 保留 Top 2
    // ============================================
    println!("[场景 2] Top-K 注意力 (5 输入 → Top 2)");
    let texts = vec![
        TextInput::new("t1", SignalSource::Cli).with_priority(0.1),
        TextInput::new("t2", SignalSource::Cli).with_priority(0.95),
        TextInput::new("t3", SignalSource::Cli).with_priority(0.4),
        TextInput::new("t4", SignalSource::Cli).with_priority(0.8),
        TextInput::new("t5", SignalSource::Cli).with_priority(0.5),
    ];
    let att = TopKAttention::new(2);
    let kept = att.filter(texts);
    println!("  保留 {} 条 (期望 2):", kept.len());
    for t in &kept {
        println!("    - pri={:.2} text=\"{}\"", t.priority, t.content);
    }
    println!();

    // ============================================
    // 场景 3: 多模态通道对比
    // ============================================
    println!("[场景 3] 5 通道 × 1 输入 → 5 PerceptionEvent");

    let text_ev = TextChannel.process(&TextInput::new("user typed a message", SignalSource::Cli));
    let voice_ev = VoiceChannel.process(&VoiceInput::new("用户说了句话", SignalSource::Http, 0.6));
    let vision_ev = VisionChannel.process(&VisionInput::new(
        1920,
        1080,
        SignalSource::PyBridge,
        Some("screen OCR text".into()),
    ));
    let tactile_ev = TactileChannel.process(&TactileInput::new(-0.5, SignalSource::Internal));
    let command_ev = CommandChannel.process(&CommandInput::new("/reflect", SignalSource::Mcp));

    let events = vec![text_ev, voice_ev, vision_ev, tactile_ev, command_ev];
    for ev in &events {
        match validate_event(ev) {
            Ok(_) => println!(
                "  ✅ {:?} from {}: pri={:.2} payload={}",
                ev.channel,
                ev.source.label(),
                ev.priority,
                truncate(&ev.payload, 50)
            ),
            Err(e) => println!("  ❌ {:?}: {}", ev.channel, e),
        }
    }
    println!();

    // ============================================
    // 场景 4: 批量处理 + Top-K 全局过滤
    // ============================================
    println!(
        "[场景 4] 6 文本输入 → batch_process → Top-K({})",
        default_top_k()
    );
    let ch = TextChannel;
    let inputs: Vec<TextInput> = (0..6)
        .map(|i| {
            TextInput::new(format!("msg-{}", i), SignalSource::Cli)
                .with_priority(f64::from(i) / 6.0)
        })
        .collect();
    let events = batch_process(&ch, inputs.clone());
    let top_inputs = top_k_filter(inputs, 3);
    println!("  处理后事件: {} 条", events.len());
    println!("  Top-3 原始输入 (按 priority 降序):");
    for t in &top_inputs {
        println!("    - pri={:.3} text=\"{}\"", t.priority, t.content);
    }
    println!();

    println!("=== demo done ===");
}

fn print_events(label: &str, events: &[PerceptionEvent]) {
    println!("  [{}] 通过 {} 条:", label, events.len());
    for ev in events {
        println!(
            "    - kind={:?} src={} pri={:.2} payload=\"{}\"",
            ev.channel,
            ev.source.label(),
            ev.priority,
            truncate(&ev.payload, 40)
        );
    }
    println!();
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

// 引用 trait 占位以避免 dead_code 警告 (演示用).
#[allow(dead_code)]
fn _ensure_traits_used<C: PerceptionChannel, I: PerceptionInput>() {}
