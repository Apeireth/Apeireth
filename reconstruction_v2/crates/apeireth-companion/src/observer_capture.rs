pub struct ToolObservation {
    pub tool_name: String,
    pub input: String,
    pub output: String,
}

pub struct ExperienceQueue {
    observations: Vec<ToolObservation>,
}

impl ExperienceQueue {
    pub fn new() -> Self { Self { observations: vec![] } }
    
    pub fn capture(&mut self, obs: ToolObservation) {
        self.observations.push(obs);
    }
}
