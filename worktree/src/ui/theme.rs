use ratatui::style::Color;

pub struct CyberTheme {
    pub primary: Color,      // Neon Cyan
    pub secondary: Color,    // Electric Purple
    pub accent: Color,       // Cyber Pink
    pub success: Color,      // Matrix Green
    pub error: Color,        // Blood Red
    pub warning: Color,      // Warning Orange
    pub text: Color,         // Ghost White
    pub subtle: Color,       // Dim Gray
    pub selection_bg: Color, // Indigo Pulse
    pub border: Color,       // Slate Border
    pub header_bg: Color,    // Header/Title Background
}

impl Default for CyberTheme {
    fn default() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Magenta,
            accent: Color::Rgb(236, 72, 153), // Keeping one RGB for "cyber" feel, but mostly ANSI
            success: Color::Green,
            error: Color::Red,
            warning: Color::Yellow,
            text: Color::White,
            subtle: Color::Indexed(242), // Dark grey
            selection_bg: Color::Indexed(236), // Deep grey/blue
            border: Color::Indexed(239),
            header_bg: Color::Indexed(17), // Deep blue
        }
    }
}

pub struct Icons;

impl Icons {
    pub const HUB: &'static str = "󰨝 "; // nf-md-hubspot
    pub const DETACHED: &'static str = " "; // nf-fa-warning
    pub const WORKTREE: &'static str = "󰘬 "; // nf-md-source_branch
    pub const CLEAN: &'static str = " "; // nf-fa-check_circle
    pub const DIRTY: &'static str = " "; // nf-fa-edit
}
