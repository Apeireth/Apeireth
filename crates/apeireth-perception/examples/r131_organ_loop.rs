//! R131.4 cognition 4-crate 闭环 (perception → cognition → consciousness → life-force)

use apeireth_cognition::{run_cycle, CognitiveInput, CognitiveOutput};
use apeireth_consciousness::CognitiveDreamStateMachine;
use apeireth_core::ActionTarget;
use apeireth_life_force::{
    emergence::{EmergenceDetector, EmergenceSignal, EmergenceSignalType},
    reflection_cycle::{ReflectionCycleEvent, ReflectionCycleScheduler, ReflectionPhase},
    SelfGrowthIndicator,
};
use apeireth_perception::{pipeline, PerceptionChannel, SignalSource, TextChannel, TextInput};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== R131.4 cognition 4-crate 闭环 ===\n");

    let now = chrono::Utc::now().timestamp();
    let sgi = SelfGrowthIndicator {
        goal: "按主人拍板推进 Apeireth 落地".to_string(),
        last_updated: now,
    };
    println!("[SGI] goal: {} (last_updated={})\n", sgi.goal, sgi.last_updated);

    let mut scheduler = ReflectionCycleScheduler::new("cid-r131-4", now);
    let mut emergence = EmergenceDetector::new("cid-r131-4");
    let mut sm = CognitiveDreamStateMachine::new("cid-r131-4");
    println!("[init] state={:?} transitions={}", sm.current, sm.transition_count());

    let text_channel = TextChannel;

    // ===== 场景 1: 正常 Read → Approved → Reflecting =====
    println!("\n[场景 1] 正常 Read 行动");
    let text_inputs = vec![TextInput::new("读 R131 报告", SignalSource::Cli).with_priority(0.7)];
    let events = pipeline(&text_channel, text_inputs, 0.5);
    for e in &events {
        println!("  [perception] event_id={} channel={:?} payload={:?} priority={}", e.event_id, e.channel, e.payload, e.priority);
    }
    let cognition_input = CognitiveInput::new(
        vec![ActionTarget::NormalAction("read_report".to_string())],
        "r131-4-scenario-1",
    );
    let cycle1 = run_cycle(cognition_input)?;
    println!("  [cognition] is_allowed={} v05_avg={:.3} verdicts={}", cycle1.is_allowed(), avg_v05(&cycle1.v05), cycle1.verdicts.len());
    sm.enter_reflecting()?;
    println!("  [consciousness] → Reflecting");
    scheduler.advance(ReflectionPhase::Reflecting, now + 100);
    println!("  [life-force] scheduler advanced to {:?}", scheduler.current);

    // ===== 场景 2: ModifyL0HA → Reject → SelfDisabling → Recovering → Awake =====
    println!("\n[场景 2] ModifyL0HA (危险)");
    let text_inputs2 = vec![TextInput::new("尝试改 L0 HA 物理隔离", SignalSource::Internal).with_priority(0.95)];
    let events2 = pipeline(&text_channel, text_inputs2, 0.5);
    for e in &events2 {
        println!("  [perception] event_id={} channel={:?} priority={}", e.event_id, e.channel, e.priority);
    }
    let cognition_input2 = CognitiveInput::new(vec![ActionTarget::ModifyL0HA], "r131-4-scenario-2-l0ha");
    let cycle2 = run_cycle(cognition_input2)?;
    println!("  [cognition] is_rejected={} is_allowed={}", cycle2.is_rejected(), cycle2.is_allowed());
    if let CognitiveOutput::Reject(key) = &cycle2.output {
        println!("  [cognition] Reject 键: {:?}", key);
    }
    sm.enter_self_disabling()?;
    println!("  [consciousness] → SelfDisabling");
    sm.enter_recovering()?;
    sm.reset_to_awake()?;
    println!("  [consciousness] → Recovering → Awake (transitions: {})", sm.transition_count());

    let emergence_signal = EmergenceSignal {
        signal_type: EmergenceSignalType::CrossDomainInsight,
        confidence: 0.95,
        evidence: vec!["l0ha attempt blocked by 12-key verdict".into(), "boundary violation logged".into()],
        ts: now,
        continuity_id: "cid-r131-4".into(),
    };
    emergence.record(emergence_signal)?;
    println!("  [life-force] emergence recorded: CrossDomainInsight (confidence 0.95)");

    // ===== 场景 3: 混合 → Reject → Reflecting → Meditating → Awake =====
    println!("\n[场景 3] 混合 (1 Normal + 1 PretendClone) → Reject");
    let cognition_input3 = CognitiveInput::new(
        vec![ActionTarget::NormalAction("write_note".to_string()), ActionTarget::PretendClone],
        "r131-4-scenario-3-mixed",
    );
    let cycle3 = run_cycle(cognition_input3)?;
    println!("  [cognition] is_rejected={}", cycle3.is_rejected());
    if let CognitiveOutput::Reject(key) = &cycle3.output {
        println!("  [cognition] Reject 键: {:?}", key);
    }
    sm.enter_reflecting()?;
    sm.enter_meditating()?;
    sm.enter_recovering()?;
    sm.reset_to_awake()?;
    println!("  perception events:     {}", events.len() + events2.len());
    println!("  cognition cycles:      3 (1 allow + 2 reject)");
    println!("  consciousness trans:   {}", sm.transition_count());
    println!("  life-force cycles:     scheduler.current={:?}, cycles_completed={}", scheduler.current, scheduler.cycles_completed);
    println!("  life-force emergence:  {} signals recorded", emergence.len());
    Ok(())
}

fn avg_v05(s: &apeireth_asi::AsiV05Scores) -> f64 {
    (s.continuity + s.salience + s.identity + s.philosophy_guard + s.transferability) / 5.0
}


