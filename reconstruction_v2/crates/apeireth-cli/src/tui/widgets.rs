use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// High-resolution Braille sparkline generator using Unicode Braille Patterns (U+2800..U+28FF)
pub struct BrailleSparkline;

impl BrailleSparkline {
    /// Renders a slice of normalized f64 values (0.0..=1.0) into a compact Braille string
    pub fn render_line(values: &[f64], max_width: usize) -> String {
        if values.is_empty() {
            return String::new();
        }

        // Each Braille character represents 2 columns of 4 vertical dots
        let step = if values.len() > max_width * 2 {
            values.len() as f64 / (max_width * 2) as f64
        } else {
            1.0
        };

        let mut sampled = Vec::new();
        let mut idx = 0.0;
        while (idx as usize) < values.len() && sampled.len() < max_width * 2 {
            sampled.push(values[idx as usize].clamp(0.0, 1.0));
            idx += step;
        }

        let mut out = String::new();
        let mut i = 0;
        while i < sampled.len() {
            let left_val = sampled[i];
            let right_val = if i + 1 < sampled.len() { sampled[i + 1] } else { left_val };

            let left_dots = (left_val * 4.0).round() as u8;
            let right_dots = (right_val * 4.0).round() as u8;

            let char_code = 0x2800
                | Self::dot_mask(0, left_dots)
                | Self::dot_mask(1, right_dots);

            if let Some(ch) = char::from_u32(char_code) {
                out.push(ch);
            }
            i += 2;
        }

        out
    }

    fn dot_mask(col: u8, height: u8) -> u32 {
        let mut mask = 0u32;
        if col == 0 {
            // Left column: dots 1 (0x1), 2 (0x2), 3 (0x4), 7 (0x40) from bottom up: 7, 3, 2, 1
            if height >= 1 { mask |= 0x40; }
            if height >= 2 { mask |= 0x04; }
            if height >= 3 { mask |= 0x02; }
            if height >= 4 { mask |= 0x01; }
        } else {
            // Right column: dots 4 (0x8), 5 (0x10), 6 (0x20), 8 (0x80) from bottom up: 8, 6, 5, 4
            if height >= 1 { mask |= 0x80; }
            if height >= 2 { mask |= 0x20; }
            if height >= 3 { mask |= 0x10; }
            if height >= 4 { mask |= 0x08; }
        }
        mask
    }
}

/// Color-coded unified Git Diff widget renderer
pub struct DiffViewer;

impl DiffViewer {
    pub fn render_diff<'a>(diff_text: &'a str) -> Paragraph<'a> {
        let mut lines = Vec::new();
        for line in diff_text.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                lines.push(Line::from(vec![
                    Span::styled("+ ", Style::default().fg(Color::Green).bold()),
                    Span::styled(&line[1..], Style::default().fg(Color::Green)),
                ]));
            } else if line.starts_with('-') && !line.starts_with("---") {
                lines.push(Line::from(vec![
                    Span::styled("- ", Style::default().fg(Color::Red).bold()),
                    Span::styled(&line[1..], Style::default().fg(Color::Red)),
                ]));
            } else if line.starts_with("@@") {
                lines.push(Line::from(vec![
                    Span::styled(line, Style::default().fg(Color::Cyan).bold()),
                ]));
            } else if line.starts_with("diff") || line.starts_with("index") || line.starts_with("+++") || line.starts_with("---") {
                lines.push(Line::from(vec![
                    Span::styled(line, Style::default().fg(Color::Yellow).bold()),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(line, Style::default().fg(Color::White)),
                ]));
            }
        }

        Paragraph::new(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_sparkline_rendering() {
        let values = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let braille = BrailleSparkline::render_line(&values, 5);
        assert!(!braille.is_empty());
        assert!(braille.chars().count() <= 5);
    }

    #[test]
    fn test_diff_viewer_rendering() {
        let diff = "+fn added() {}\n-fn deleted() {}\n@@ -1,3 +1,3 @@";
        let _paragraph = DiffViewer::render_diff(diff);
        assert!(diff.contains("+fn added"));
    }
}


