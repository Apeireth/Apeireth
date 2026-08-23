use tokio::sync::mpsc;

pub struct SessionHandle {
    #[allow(dead_code)]
    base_url: String,
}

impl SessionHandle {
    pub(crate) fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn send_message(&self, _msg: &str) -> Result<(), crate::Error> {
        Ok(())
    }

    pub fn stream_events(&self) -> EventStream {
        let (_tx, rx) = mpsc::unbounded_channel();
        EventStream { rx }
    }
}

pub struct EventStream {
    rx: mpsc::UnboundedReceiver<String>,
}

impl EventStream {
    pub async fn next_event(&mut self) -> Option<String> {
        self.rx.recv().await
    }
}
