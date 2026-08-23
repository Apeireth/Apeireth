use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FoldBlock {
    pub id: String,
    pub summary: String,
    pub raw_content: String,
    pub semantic_score: f64,
    pub is_folded: bool,
}

impl FoldBlock {
    pub fn new(id: String, summary: String, raw_content: String, semantic_score: f64) -> Self {
        Self {
            id,
            summary,
            raw_content,
            semantic_score,
            is_folded: true, // Default to folded
        }
    }

    pub fn unfold_if_above_threshold(&mut self, threshold: f64) {
        if self.semantic_score >= threshold {
            self.is_folded = false;
        }
    }

    pub fn get_content(&self) -> &str {
        if self.is_folded {
            &self.summary
        } else {
            &self.raw_content
        }
    }
}

pub struct ContextFolder {
    blocks: Vec<FoldBlock>,
}

impl ContextFolder {
    pub fn new(blocks: Vec<FoldBlock>) -> Self {
        Self { blocks }
    }

    pub fn fold_all(&mut self) {
        for block in &mut self.blocks {
            block.is_folded = true;
        }
    }

    pub fn apply_semantic_unfolding(&mut self, threshold: f64) {
        for block in &mut self.blocks {
            block.unfold_if_above_threshold(threshold);
        }
    }

    pub fn render_context(&self) -> String {
        self.blocks
            .iter()
            .map(|b| b.get_content())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_folding_and_unfolding() {
        let block1 = FoldBlock::new("b1".into(), "Summary 1".into(), "Detailed Content 1".into(), 0.85);
        let block2 = FoldBlock::new("b2".into(), "Summary 2".into(), "Detailed Content 2".into(), 0.30);

        let mut folder = ContextFolder::new(vec![block1, block2]);
        assert_eq!(folder.render_context(), "Summary 1\nSummary 2");

        folder.apply_semantic_unfolding(0.70);
        assert_eq!(folder.render_context(), "Detailed Content 1\nSummary 2");
    }
}

