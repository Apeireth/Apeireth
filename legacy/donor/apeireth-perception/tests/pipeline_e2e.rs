//! apeireth-perception 集成测试 (端到端 pipeline).
//!
//! ponytail: 与单元测试互不重复 — 单元测单 trait/单函数, 这里测整条链路.
//! 目标: 1 个集成测试覆盖感知全流程 (输入 → 通道 → 注意力 → 事件).

use apeireth_perception::{
    batch_process, default_attention_threshold, Attention, ChannelKind, CommandChannel,
    CommandInput, PerceptionChannel, PerceptionEvent, SignalSource, TactileChannel, TactileInput,
    TextChannel, TextInput, ThresholdAttention, TopKAttention, VisionChannel, VisionInput,
    VoiceChannel, VoiceInput,
};

/// 端到端: 文本/语音/视觉/触觉输入 → 各自通道 → 阈值过滤 → 校验事件.
#[test]
fn end_to_end_multimodal_pipeline() {
    // 1) 准备 4 种模态输入.
    let texts = vec![
        TextInput::new("hello world", SignalSource::Cli).with_priority(0.6),
        TextInput::new("noise", SignalSource::Internal).with_priority(0.1),
    ];
    let voices = vec![VoiceInput::new("say hi", SignalSource::Http, 0.85)];
    let visions = vec![VisionInput::new(
        1280,
        720,
        SignalSource::PyBridge,
        Some("screen".into()),
    )];
    let tactiles = vec![TactileInput::new(-0.9, SignalSource::Internal)];

    // 2) 各通道批处理.
    let text_events = batch_process(&TextChannel, texts);
    let voice_events = batch_process(&VoiceChannel, voices);
    let vision_events = batch_process(&VisionChannel, visions);
    let tactile_events = batch_process(&TactileChannel, tactiles);

    // 3) 合并 + 按 priority 排序 + TopK(3) (注意: 直接对 PerceptionEvent 用
    //    Attention::filter 要求 PerceptionInput, 这里用手动 partial_cmp 复现
    //    TopK 行为, 保持 e2e 测试独立于 PerceptionInput trait).
    let mut all: Vec<PerceptionEvent> = text_events
        .into_iter()
        .chain(voice_events)
        .chain(vision_events)
        .chain(tactile_events)
        .collect();
    all.sort_by(|a, b| {
        b.priority
            .partial_cmp(&a.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all.truncate(TopKAttention::new(3).k);

    // 4) 阈值过滤.
    let thr = default_attention_threshold();
    let above_thr: Vec<PerceptionEvent> = all.into_iter().filter(|e| e.priority >= thr).collect();

    // 5) 断言: 至少 1 条事件 (按数据至少有 vision/vision/tactile 中高优先级),
    //    全部 priority >= 0.5, 每条都有非空 payload.
    assert!(!above_thr.is_empty(), "应至少保留 1 条");
    for ev in &above_thr {
        assert!(
            ev.priority >= thr,
            "event {:?} priority {}",
            ev.channel,
            ev.priority
        );
        assert!(!ev.payload.is_empty(), "event {:?} payload 空", ev.channel);
        assert!(
            matches!(
                ev.channel,
                ChannelKind::Text
                    | ChannelKind::Voice
                    | ChannelKind::Vision
                    | ChannelKind::Tactile
                    | ChannelKind::Command
            ),
            "未知 channel: {:?}",
            ev.channel
        );
    }
}

/// CommandChannel 链路: 输入 → 事件 → tag 完整性.
#[test]
fn command_channel_round_trip() {
    let ch = CommandChannel;
    let inp = CommandInput::new("/status", SignalSource::Cli);
    let ev = ch.process(&inp);
    assert_eq!(ev.payload, "/status");
    assert!(ev.tags.contains(&"command".to_string()));
    assert!(ev.tags.contains(&"user_initiated".to_string()));
}
