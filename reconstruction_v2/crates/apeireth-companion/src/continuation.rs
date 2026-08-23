pub enum EditAction {
    Retain(usize),
    Remove(usize),
    Replace(String),
}

pub struct SegmentEditor {
    buffer: String,
}

impl SegmentEditor {
    pub fn new(initial: &str) -> Self {
        Self { buffer: initial.into() }
    }
    
    pub fn apply(&mut self, actions: &[EditAction]) {
        let mut new_buf = String::new();
        let mut chars = self.buffer.chars();
        
        for action in actions {
            match action {
                EditAction::Retain(n) => {
                    for _ in 0..*n {
                        if let Some(c) = chars.next() { new_buf.push(c); }
                    }
                }
                EditAction::Remove(n) => {
                    for _ in 0..*n { chars.next(); }
                }
                EditAction::Replace(s) => {
                    new_buf.push_str(s);
                }
            }
        }
        self.buffer = new_buf;
    }
}
