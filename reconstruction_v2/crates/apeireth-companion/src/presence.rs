pub enum PresenceEvent {
    EmotionUpdated,
    InitiativeTriggered,
    DreamStarted,
    MemoryRecalled,
}

pub struct PresenceEmitter {
    listeners: Vec<Box<dyn Fn(PresenceEvent) + Send + Sync>>,
}

impl PresenceEmitter {
    pub fn new() -> Self { Self { listeners: vec![] } }
    
    pub fn subscribe<F>(&mut self, f: F)
    where F: Fn(PresenceEvent) + Send + Sync + 'static {
        self.listeners.push(Box::new(f));
    }
    
    pub fn emit(&self, event: PresenceEvent) {
        for listener in &self.listeners {
            listener(event.clone());
        }
    }
}

impl Clone for PresenceEvent {
    fn clone(&self) -> Self {
        match self {
            Self::EmotionUpdated => Self::EmotionUpdated,
            Self::InitiativeTriggered => Self::InitiativeTriggered,
            Self::DreamStarted => Self::DreamStarted,
            Self::MemoryRecalled => Self::MemoryRecalled,
        }
    }
}
