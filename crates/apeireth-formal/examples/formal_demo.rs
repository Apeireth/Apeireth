//! Runnable FormalEngine demonstration.
fn main() { let engine = apeireth_formal::FormalEngine::with_defaults(); let result = engine.dispatch_by_name("double_onion").expect("built-in proof"); println!("{}: {:?}", result.invariant, result.status); }
