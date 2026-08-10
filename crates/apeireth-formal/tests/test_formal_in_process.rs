use apeireth_formal::{example, invariant::presets, FormalEngine, ProofStatus, TlaSpec};
#[test] fn engine_proves_all_builtins(){let engine=FormalEngine::with_defaults(); assert!(engine.health_check()); for invariant in presets::ALL { assert_eq!(engine.check_invariant(&invariant).unwrap().status,ProofStatus::Proven); }}
#[test] fn dispatch_rejects_unknown(){assert!(FormalEngine::with_defaults().dispatch_by_name("missing").is_err());}
#[test] fn tla_example_is_valid(){let spec:TlaSpec=example::double_onion_spec(); assert!(spec.validate().is_ok()); assert!(spec.render().contains("DoubleOnion"));}
