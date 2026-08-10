//! supervisor_demo — boot PID 1 with the default plan and exercise the actor mailbox.
//!
//! Run with: `cargo run --example supervisor_demo -p apeireth-supervisor --offline`

use apeireth_supervisor::actor::{spawn_actor, CounterActor};
use apeireth_supervisor::strategy::RestartStrategy;
use apeireth_supervisor::supervisor::default_plan;
use apeireth_supervisor::PidOneSupervisor;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // 1. Boot PID 1
    let pid_one = PidOneSupervisor::new();
    println!(
        "[PidOneSupervisor] booting {} sub-supervisors ({} child specs total)",
        pid_one.sub_supervisors.len(),
        pid_one.total_children()
    );

    // 2. Show each subtree
    for (kind, specs) in &pid_one.sub_supervisors {
        println!(
            "[{:>8}] {:>2} children ({:?})",
            kind.as_str(),
            specs.len(),
            kind.default_strategy()
        );
        for spec in specs.iter().take(2) {
            println!(
                "    - {:<24} restart={:?} max={} window={:?}",
                spec.id, spec.restart, spec.max_restarts, spec.restart_window
            );
        }
        if specs.len() > 2 {
            println!("    - ... ({} more)", specs.len() - 2);
        }
    }

    // 3. Exercise the actor mailbox
    let counter = CounterActor::new();
    let (tx, handle, state) = spawn_actor(counter, 32);
    println!("[actor] Counter spawned, mailbox=32");

    for v in [5, 10, 15] {
        tx.send(v).await.expect("mailbox open");
        println!("[actor] Counter handle({})", v);
    }
    drop(tx);
    handle.await.expect("actor task");
    println!(
        "[actor] Counter mailbox closed → state = {:?}",
        *state.lock().unwrap()
    );

    // 4. Replace plan (hot-swap simulation)
    let mut pid_one = pid_one;
    let new_version = pid_one.replace_plan(default_plan());
    println!(
        "[PidOneSupervisor] plan replaced, new version = {}",
        new_version
    );

    // 5. Strategy summary
    let _ = RestartStrategy::default(); // suppress unused warning
    println!("[PidOneSupervisor] all sub-supervisors ready, entering idle loop");
}
