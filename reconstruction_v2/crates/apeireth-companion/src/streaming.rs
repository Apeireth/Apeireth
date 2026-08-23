use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    ReasoningDelta(String),
    ContentDelta(String),
    ToolCallDelta { name: String, args: String },
    Finished,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StreamingMode {
    Content,
    InsideReasoningXml,  // <think> ... </think>
    InsideReasoningHtml, // <!-- ... -->
    InsideToolCall,      // <tool> ... </tool>
}

pub struct StreamingStateMachine {
    mode: StreamingMode,
    raw_buffer: String,
    emitted_events: Vec<StreamEvent>,
}

impl Default for StreamingStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingStateMachine {
    pub fn new() -> Self {
        Self {
            mode: StreamingMode::Content,
            raw_buffer: String::new(),
            emitted_events: Vec::new(),
        }
    }

    pub fn feed_chunk(&mut self, chunk: &str) -> Vec<StreamEvent> {
        self.raw_buffer.push_str(chunk);
        let mut events = Vec::new();

        loop {
            match self.mode {
                StreamingMode::Content => {
                    if let Some(pos) = self.raw_buffer.find("<think>") {
                        let text_before = &self.raw_buffer[..pos];
                        if !text_before.is_empty() {
                            events.push(StreamEvent::ContentDelta(text_before.to_string()));
                        }
                        self.raw_buffer.drain(..pos + 7);
                        self.mode = StreamingMode::InsideReasoningXml;
                    } else if let Some(pos) = self.raw_buffer.find("<!--") {
                        let text_before = &self.raw_buffer[..pos];
                        if !text_before.is_empty() {
                            events.push(StreamEvent::ContentDelta(text_before.to_string()));
                        }
                        self.raw_buffer.drain(..pos + 4);
                        self.mode = StreamingMode::InsideReasoningHtml;
                    } else if let Some(pos) = self.raw_buffer.find("<tool>") {
                        let text_before = &self.raw_buffer[..pos];
                        if !text_before.is_empty() {
                            events.push(StreamEvent::ContentDelta(text_before.to_string()));
                        }
                        self.raw_buffer.drain(..pos + 6);
                        self.mode = StreamingMode::InsideToolCall;
                    } else {
                        // Flush safe prefix if no partial tag pending
                        if !self.raw_buffer.ends_with('<') && !self.raw_buffer.ends_with("<!") && !self.raw_buffer.ends_with("<!--") {
                            let content = std::mem::take(&mut self.raw_buffer);
                            events.push(StreamEvent::ContentDelta(content));
                        }
                        break;
                    }
                }
                StreamingMode::InsideReasoningXml => {
                    if let Some(pos) = self.raw_buffer.find("</think>") {
                        let reasoning = &self.raw_buffer[..pos];
                        if !reasoning.is_empty() {
                            events.push(StreamEvent::ReasoningDelta(reasoning.to_string()));
                        }
                        self.raw_buffer.drain(..pos + 8);
                        self.mode = StreamingMode::Content;
                    } else {
                        if !self.raw_buffer.ends_with('<') && !self.raw_buffer.ends_with("</") && !self.raw_buffer.ends_with("</think") {
                            let reasoning = std::mem::take(&mut self.raw_buffer);
                            events.push(StreamEvent::ReasoningDelta(reasoning));
                        }
                        break;
                    }
                }
                StreamingMode::InsideReasoningHtml => {
                    if let Some(pos) = self.raw_buffer.find("-->") {
                        let reasoning = &self.raw_buffer[..pos];
                        if !reasoning.is_empty() {
                            events.push(StreamEvent::ReasoningDelta(reasoning.to_string()));
                        }
                        self.raw_buffer.drain(..pos + 3);
                        self.mode = StreamingMode::Content;
                    } else {
                        if !self.raw_buffer.ends_with('-') && !self.raw_buffer.ends_with("--") {
                            let reasoning = std::mem::take(&mut self.raw_buffer);
                            events.push(StreamEvent::ReasoningDelta(reasoning));
                        }
                        break;
                    }
                }
                StreamingMode::InsideToolCall => {
                    if let Some(pos) = self.raw_buffer.find("</tool>") {
                        let tool_body = &self.raw_buffer[..pos];
                        events.push(StreamEvent::ToolCallDelta {
                            name: "dynamic_tool".into(),
                            args: tool_body.to_string(),
                        });
                        self.raw_buffer.drain(..pos + 7);
                        self.mode = StreamingMode::Content;
                    } else {
                        break;
                    }
                }
            }
        }

        self.emitted_events.extend(events.clone());
        events
    }

    pub fn finish(&mut self) -> Vec<StreamEvent> {
        let mut remaining = Vec::new();
        if !self.raw_buffer.is_empty() {
            match self.mode {
                StreamingMode::InsideReasoningXml | StreamingMode::InsideReasoningHtml => {
                    remaining.push(StreamEvent::ReasoningDelta(std::mem::take(&mut self.raw_buffer)));
                }
                StreamingMode::Content | StreamingMode::InsideToolCall => {
                    remaining.push(StreamEvent::ContentDelta(std::mem::take(&mut self.raw_buffer)));
                }
            }
        }
        remaining.push(StreamEvent::Finished);
        remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_think_tags() {
        let mut sm = StreamingStateMachine::new();
        let e1 = sm.feed_chunk("Hello <think>analyzing request");
        assert_eq!(e1.len(), 2);
        assert_eq!(e1[0], StreamEvent::ContentDelta("Hello ".into()));
        assert_eq!(e1[1], StreamEvent::ReasoningDelta("analyzing request".into()));

        let e2 = sm.feed_chunk(" deeply</think> here is response");
        assert_eq!(e2.len(), 2);
        assert_eq!(e2[0], StreamEvent::ReasoningDelta(" deeply".into()));
        assert_eq!(e2[1], StreamEvent::ContentDelta(" here is response".into()));
    }

    #[test]
    fn test_streaming_minimax_html_comment_cot() {
        let mut sm = StreamingStateMachine::new();
        let e1 = sm.feed_chunk("<!-- MiniMax internal thinking -->Final answer from model");
        assert_eq!(e1.len(), 2);
        assert_eq!(e1[0], StreamEvent::ReasoningDelta(" MiniMax internal thinking ".into()));
        assert_eq!(e1[1], StreamEvent::ContentDelta("Final answer from model".into()));
    }
}

