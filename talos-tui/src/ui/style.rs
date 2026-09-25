use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Padding};

// Default foreground/background follow the terminal theme. Selection uses a
// deliberate contrasting pair; NO_COLOR retains markers, bold and reverse.
pub const PRIMARY: Style = Style::new();
pub const SECONDARY: Style = Style::new().fg(Color::Reset);
pub const BORDER: Style = Style::new().fg(Color::DarkGray);
pub const FOCUS: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub const SUCCESS: Style = Style::new().fg(Color::Green);
pub const WARNING: Style = Style::new().fg(Color::Yellow);
pub const ERROR: Style = Style::new().fg(Color::Red);

pub fn selection(focused: bool) -> Style {
    let style = Style::new().fg(Color::White).bg(Color::DarkGray);
    if focused {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

pub fn pane(title: impl Into<String>, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::TOP)
        .border_style(if focused { FOCUS } else { BORDER })
        .title(format!(
            "{} {} ",
            if focused { ">" } else { " " },
            title.into()
        ))
        .padding(Padding::horizontal(1))
}

pub fn status(text: &str) -> Style {
    if text.starts_with("error:") || text.starts_with("rejected:") || text.contains("' rejected:") {
        ERROR
    } else if text.starts_with("pending ")
        || text.starts_with("loading ")
        || text.starts_with("setting ")
    {
        WARNING
    } else {
        SUCCESS
    }
}
