use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

#[derive(Default)]
pub struct UiState {
    input: String,
    cursor: usize,
    completion_index: usize,
}

impl UiState {
    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn completion_index(&self) -> usize {
        self.completion_index
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.completion_index = 0;
    }

    pub fn insert_char(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.completion_index = 0;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.input[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.input.remove(prev);
        self.cursor = prev;
        self.completion_index = 0;
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        self.input.remove(self.cursor);
        self.completion_index = 0;
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.input[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.input.len() {
            let ch = self.input[self.cursor..].chars().next().unwrap();
            self.cursor += ch.len_utf8();
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.input.len();
    }

    pub fn sync_completion_index(&mut self, len: usize) {
        self.completion_index = normalize_index(self.completion_index, len);
    }

    pub fn select_next_completion(&mut self, len: usize) {
        if len == 0 {
            self.completion_index = 0;
            return;
        }
        self.completion_index = (self.completion_index + 1) % len;
    }

    pub fn select_prev_completion(&mut self, len: usize) {
        if len == 0 {
            self.completion_index = 0;
            return;
        }
        self.completion_index = if self.completion_index == 0 {
            len - 1
        } else {
            self.completion_index - 1
        };
    }

    pub fn apply_completion(&mut self, suggestion: &str) {
        let cursor = self.cursor.min(self.input.len());
        let prefix = &self.input[..cursor];
        let start = self.input[..cursor]
            .rfind(char::is_whitespace)
            .map(|index| index + 1)
            .unwrap_or(0);

        if suggestion.starts_with(prefix)
            || (suggestion.contains(' ') && !self.input[..cursor].contains(char::is_whitespace))
        {
            self.input.replace_range(0..cursor, suggestion);
            self.cursor = suggestion.len();
        } else {
            self.input.replace_range(start..cursor, suggestion);
            self.cursor = start + suggestion.len();
        }

        if !self.input.ends_with(' ') {
            self.input.push(' ');
            self.cursor += 1;
        }
        self.completion_index = 0;
    }
}

pub struct UiModel {
    pub summary_lines: Vec<String>,
    pub prompt_lines: Vec<String>,
    pub log_lines: Vec<String>,
    pub history_lines: Vec<String>,
    pub input: String,
    pub cursor: usize,
    pub selected_suggestion: usize,
    pub suggestions: Vec<String>,
    pub completion_on: bool,
}

pub fn render(frame: &mut Frame<'_>, model: &UiModel) {
    let area = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(12),
            Constraint::Length(6),
        ])
        .split(area);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(vertical[0]);
    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(vertical[1]);

    render_lines(
        frame,
        top[0],
        "Summary",
        &model.summary_lines,
        Style::default().fg(Color::Cyan),
    );
    render_lines(
        frame,
        top[1],
        "Prompt",
        &model.prompt_lines,
        Style::default().fg(Color::Yellow),
    );
    render_lines(
        frame,
        middle[0],
        "Log",
        &model.log_lines,
        Style::default().fg(Color::Green),
    );
    render_lines(
        frame,
        middle[1],
        "History",
        &model.history_lines,
        Style::default().fg(Color::Magenta),
    );
    render_input(frame, vertical[2], model);
}

fn render_lines(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    lines: &[String],
    border_style: Style,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);
    let inner = block.inner(area);
    let text = Text::from(
        lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect::<Vec<_>>(),
    );
    let scroll = tail_scroll(lines.len(), inner.height as usize);
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph.scroll((scroll, 0)), area);
}

fn render_input(frame: &mut Frame<'_>, area: Rect, model: &UiModel) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Input")
        .border_style(Style::default().fg(Color::Blue));

    let mut lines = vec![Line::from(format!("> {}", model.input))];
    if model.completion_on {
        if model.suggestions.is_empty() {
            lines.push(Line::from("suggestions: none"));
        } else {
            lines.push(Line::from("suggestions:"));
            for (index, suggestion) in model.suggestions.iter().take(3).enumerate() {
                let style = if index == model.selected_suggestion {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                lines.push(Line::styled(
                    format!(
                        "{} {}",
                        if index == model.selected_suggestion {
                            ">"
                        } else {
                            " "
                        },
                        suggestion
                    ),
                    style,
                ));
            }
        }
    } else {
        lines.push(Line::from("completion: off"));
    }

    let inner = block.inner(area);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);

    let cursor_x = inner.x + 2 + model.cursor as u16;
    let cursor_y = inner.y;
    if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn tail_scroll(total_lines: usize, visible_lines: usize) -> u16 {
    total_lines.saturating_sub(visible_lines) as u16
}

fn normalize_index(index: usize, len: usize) -> usize {
    if len == 0 { 0 } else { index.min(len - 1) }
}

#[cfg(test)]
mod tests {
    use super::UiState;

    #[test]
    fn backspace_handles_multibyte_char() {
        let mut s = UiState::default();
        // 'あ' is 3 UTF-8 bytes
        s.insert_char('あ');
        assert_eq!(s.cursor(), 3);
        s.backspace();
        assert_eq!(s.input(), "");
        assert_eq!(s.cursor(), 0);
    }

    #[test]
    fn move_left_right_handles_multibyte_chars() {
        let mut s = UiState::default();
        s.insert_char('あ');
        s.insert_char('い');
        // cursor == 6 (2 × 3 bytes)
        s.move_left();
        assert_eq!(s.cursor(), 3);
        s.move_left();
        assert_eq!(s.cursor(), 0);
        s.move_right();
        assert_eq!(s.cursor(), 3);
    }
}
