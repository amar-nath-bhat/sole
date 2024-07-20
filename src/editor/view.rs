use std::cmp::min;

use self::line::Line;

use super::{
    editorcommand::{Direction, EditorCommand},
    terminal::{Position, Size, Terminal},
    DocumentStatus, NAME, VERSION,
};
mod buffer;
use buffer::Buffer;

mod line;

pub struct View {
    buffer: Buffer,
    needs_redraw: bool,
    size: Size,
    margin_bottom: usize,
    text_location: Location,
    scroll_offset: Position,
}

#[derive(Clone, Copy, Default)]
pub struct Location {
    pub grapheme_index: usize,
    pub line_index: usize,
}

impl View {
    pub fn new(margin_bottom: usize) -> Self {
        let terminal_size = Terminal::size().unwrap_or_default();
        Self {
            buffer: Buffer::default(),
            needs_redraw: true,
            size: Size {
                width: terminal_size.width,
                height: terminal_size.height.saturating_sub(margin_bottom),
            },
            margin_bottom,
            text_location: Location::default(),
            scroll_offset: Position::default(),
        }
    }

    pub fn get_status(&self) -> DocumentStatus {
        DocumentStatus {
            total_lines: self.buffer.height(),
            current_line_index: self.text_location.line_index,
            file_name: format!("{}", self.buffer.file_info),
            is_modified: self.buffer.dirty,
        }
    }

    pub fn handle_command(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::Resize(size) => self.resize(size),
            EditorCommand::Move(direction) => self.move_text_location(direction),
            EditorCommand::Quit => {}
            EditorCommand::Insert(character) => self.insert_char(character),
            EditorCommand::Delete => self.delete(),
            EditorCommand::Backspace => self.delete_backward(),
            EditorCommand::Enter => self.insert_newline(),
            EditorCommand::Save => self.save(),
        }
    }

    pub fn load(&mut self, file_name: &str) {
        if let Ok(buffer) = Buffer::load(file_name) {
            self.buffer = buffer;
            self.needs_redraw = true;
        }
    }

    fn insert_char(&mut self, character: char) {
        let old_len = self
            .buffer
            .lines
            .get(self.text_location.line_index)
            .map_or(0, Line::grapheme_count);
        self.buffer.insert_char(character, self.text_location);
        let new_len = self
            .buffer
            .lines
            .get(self.text_location.line_index)
            .map_or(0, Line::grapheme_count);
        let grapheme_delta = new_len.saturating_sub(old_len);
        if grapheme_delta > 0 {
            //move right for an added grapheme (should be the regular case)
            self.move_text_location(Direction::Right);
        }
        self.needs_redraw = true;
    }

    fn delete(&mut self) {
        self.buffer.delete(self.text_location);
        self.needs_redraw = true;
    }

    fn insert_newline(&mut self) {
        self.buffer.insert_newline(self.text_location);
        self.move_text_location(Direction::Right);
        self.needs_redraw = true;
    }
    fn delete_backward(&mut self) {
        if self.text_location.line_index != 0 || self.text_location.grapheme_index != 0 {
            self.move_text_location(Direction::Left);
            self.delete();
        }
    }

    fn save(&mut self) {
        let _ = self.buffer.save();
    }

    fn resize(&mut self, to: Size) {
        self.size = Size {
            width: to.width,
            height: to.height.saturating_sub(self.margin_bottom),
        };
        self.scroll_text_location_into_view();
        self.needs_redraw = true;
    }

    pub fn render(&mut self) {
        if !self.needs_redraw || self.size.height == 0 {
            return;
        }

        let Size { height, width } = self.size;

        if height == 0 || width == 0 {
            return;
        }

        #[allow(clippy::integer_division)]
        let vertical_center = height / 3;
        let top = self.scroll_offset.row;

        for row in 0..height {
            if let Some(line) = self.buffer.lines.get(row.saturating_add(top)) {
                let left = self.scroll_offset.col;
                let right = self.scroll_offset.col.saturating_add(width);
                Self::render_line(row, &line.get_visible_graphemes(left..right));
            } else if row == vertical_center && self.buffer.is_empty() {
                Self::render_line(row, &Self::build_welcome_message(width));
            } else {
                Self::render_line(row, "~");
            }
        }
        self.needs_redraw = false;
    }

    fn render_line(at: usize, line_text: &str) {
        let result = Terminal::print_row(at, line_text);
        debug_assert!(result.is_ok(), "Failed to render line");
    }

    pub fn build_welcome_message(width: usize) -> String {
        if width == 0 {
            return String::new();
        }
        let welcome_msg = format!("Welcome to {} -- Version {}!", NAME, VERSION);
        let len = welcome_msg.len();

        let remaining_width = width.saturating_sub(1);
        // hide the welcome message if it doesn't fit entirely.
        if remaining_width < len {
            return "~".to_string();
        }
        format!("{:<1}{:^remaining_width$}", "~", welcome_msg)
    }

    fn scroll_vertically(&mut self, to: usize) {
        let Size { height, .. } = self.size;
        let offset_changed = if to < self.scroll_offset.row {
            self.scroll_offset.row = to;
            true
        } else if to >= self.scroll_offset.row.saturating_add(height) {
            self.scroll_offset.row = to.saturating_sub(height).saturating_add(1);
            true
        } else {
            false
        };
        if offset_changed {
            self.needs_redraw = true;
        }
    }

    fn scroll_horizontally(&mut self, to: usize) {
        let Size { width, .. } = self.size;
        let offset_changed = if to < self.scroll_offset.col {
            self.scroll_offset.col = to;
            true
        } else if to >= self.scroll_offset.col.saturating_add(width) {
            self.scroll_offset.col = to.saturating_sub(width).saturating_add(1);
            true
        } else {
            false
        };
        if offset_changed {
            self.needs_redraw = true;
        }
    }

    fn scroll_text_location_into_view(&mut self) {
        let Position { col, row } = self.text_location_to_position();
        self.scroll_vertically(row);
        self.scroll_horizontally(col);
    }

    pub fn caret_position(&self) -> Position {
        self.text_location_to_position()
            .saturating_sub(self.scroll_offset)
    }

    fn text_location_to_position(&self) -> Position {
        let row = self.text_location.line_index;
        let col = self.buffer.lines.get(row).map_or(0, |line| {
            line.width_until(self.text_location.grapheme_index)
        });
        Position { col, row }
    }

    fn move_text_location(&mut self, direction: Direction) {
        let Size { height, .. } = self.size;

        match direction {
            Direction::Up => self.move_up(1),
            Direction::Down => self.move_down(1),
            Direction::Left => self.move_left(),
            Direction::Right => self.move_right(),
            Direction::PageUp => self.move_up(height.saturating_sub(1)),
            Direction::PageDown => self.move_down(height.saturating_sub(1)),
            Direction::Home => self.move_to_start_of_line(),
            Direction::End => self.move_to_end_of_line(),
        }
        self.scroll_text_location_into_view();
    }
    fn move_up(&mut self, step: usize) {
        self.text_location.line_index = self.text_location.line_index.saturating_sub(step);
        self.snap_to_valid_grapheme();
    }
    fn move_down(&mut self, step: usize) {
        self.text_location.line_index = self.text_location.line_index.saturating_add(step);
        self.snap_to_valid_grapheme();
        self.snap_to_valid_line();
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn move_right(&mut self) {
        let line_width = self
            .buffer
            .lines
            .get(self.text_location.line_index)
            .map_or(0, Line::grapheme_count);
        if self.text_location.grapheme_index < line_width {
            self.text_location.grapheme_index += 1;
        } else {
            self.move_to_start_of_line();
            self.move_down(1);
        }
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn move_left(&mut self) {
        if self.text_location.grapheme_index > 0 {
            self.text_location.grapheme_index -= 1;
        } else if self.text_location.line_index > 0 {
            self.move_up(1);
            self.move_to_end_of_line();
        }
    }
    fn move_to_start_of_line(&mut self) {
        self.text_location.grapheme_index = 0;
    }
    fn move_to_end_of_line(&mut self) {
        self.text_location.grapheme_index = self
            .buffer
            .lines
            .get(self.text_location.line_index)
            .map_or(0, Line::grapheme_count);
    }

    fn snap_to_valid_grapheme(&mut self) {
        self.text_location.grapheme_index = self
            .buffer
            .lines
            .get(self.text_location.line_index)
            .map_or(0, |line| {
                min(line.grapheme_count(), self.text_location.grapheme_index)
            });
    }

    fn snap_to_valid_line(&mut self) {
        self.text_location.line_index = min(self.text_location.line_index, self.buffer.height());
    }
}
