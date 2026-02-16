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
}

impl Default for CyberTheme {
    fn default() -> Self {
        Self {
            primary: Color::Rgb(34, 211, 238),      // #22d3ee (Vibrant Cyan)
            secondary: Color::Rgb(129, 140, 248),   // #818cf8 (Indigo Glow)
            accent: Color::Rgb(244, 114, 182),      // #f472b6 (Soft Pink)
            success: Color::Rgb(52, 211, 153),      // #34d399 (Emerald)
            error: Color::Rgb(248, 113, 113),       // #f87171 (Coral Red)
            warning: Color::Rgb(251, 191, 36),      // #fbbf24 (Amber)
            text: Color::Rgb(241, 245, 249),        // #f1f5f9 (Slate 100)
            subtle: Color::Rgb(100, 116, 139),      // #64748b (Slate 500)
            selection_bg: Color::Rgb(30, 41, 59),   // #1e293b (Deep Slate)
            border: Color::Rgb(51, 65, 85),         // #334155 (Border Slate)
        }
    }
}
