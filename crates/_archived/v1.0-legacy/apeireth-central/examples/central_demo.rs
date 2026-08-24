use apeireth_central::{ApeirethCentral, CentralAI};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut central = ApeirethCentral::new();
    let receipt = central.start_supervisor()?;
    println!(
        "Apeireth Central started: pid={}, stage={:?}, linked={}/{}",
        receipt.pid,
        receipt.stage,
        central.linked_component_count(),
        central.components().len()
    );
    Ok(())
}
