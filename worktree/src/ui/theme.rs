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
            primary: Color::Rgb(6, 182, 212), // #06b6d4 (Cyan 500) - Pops more
            secondary: Color::Rgb(139, 92, 246), // #8b5cf6 (Violet 500) - Electric
            accent: Color::Rgb(236, 72, 153), // #ec4899 (Pink 500) - Cyber Pink
            success: Color::Rgb(16, 185, 129), // #10b981 (Emerald 500) - Vivid Green
            error: Color::Rgb(239, 68, 68),   // #ef4444 (Red 500) - Warning Red
            warning: Color::Rgb(245, 158, 11), // #f59e0b (Amber 500) - Bright Warning
            text: Color::Rgb(226, 232, 240),  // #e2e8f0 (Slate 200) - High Contrast Text
            subtle: Color::Rgb(71, 85, 105),  // #475569 (Slate 600) - Readable Subtle
            selection_bg: Color::Rgb(30, 41, 59), // #1e293b (Slate 800) - Deep Contrast
            border: Color::Rgb(51, 65, 85),   // #334155 (Slate 700) - Visible Border
            header_bg: Color::Rgb(23, 37, 84), // #172554 (Blue 950) - Header Distinction
        }
    }
}

pub struct Icons;

impl Icons {
    pub const BARE: &'static str = "󰋜 "; // nf-md-home
    pub const DETACHED: &'static str = " "; // nf-fa-warning
    pub const WORKTREE: &'static str = "󰘬 "; // nf-md-source_branch
    pub const CLEAN: &'static str = " "; // nf-fa-check_circle
    pub const DIRTY: &'static str = " "; // nf-fa-edit
}
