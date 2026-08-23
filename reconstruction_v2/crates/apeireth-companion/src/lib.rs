pub mod emergence;
pub mod emotion;
pub mod world_model;
pub mod curiosity;
pub mod intent_brier;
pub mod streaming;
pub mod presence;
pub mod prompt_assembler;
pub mod continuation;
pub mod observer_capture;
pub mod dream;
pub mod epistemic;

pub use dream::{DreamEngine, DreamReport, EntityTriplet, DreamRehearsalResult};
pub use epistemic::{EpistemicHealer, FailureIncident};


