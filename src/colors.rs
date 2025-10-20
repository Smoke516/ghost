use ratatui::style::Color;

/// Tokyo Night color palette
pub struct TokyoNight;

impl TokyoNight {
    // Background colors
    pub const BG: Color = Color::Rgb(26, 27, 38);           // #1a1b26
    pub const BG_HIGHLIGHT: Color = Color::Rgb(41, 46, 66);  // #292e42
    pub const BG_POPUP: Color = Color::Rgb(30, 32, 48);     // #1e2030
    
    // Terminal colors
    pub const BORDER: Color = Color::Rgb(39, 41, 53);            // #27293a
    pub const BORDER_HIGHLIGHT: Color = Color::Rgb(51, 65, 85);   // #334155
    
    // Text colors
    pub const FG: Color = Color::Rgb(169, 177, 214);       // #a9b1d6
    pub const COMMENT: Color = Color::Rgb(86, 95, 137);    // #565f89
    
    // Accent colors for hacker theme
    pub const GREEN: Color = Color::Rgb(158, 206, 106);    // #9ece6a - success/online
    pub const RED: Color = Color::Rgb(247, 118, 142);      // #f7768e - error/offline
    pub const BLUE: Color = Color::Rgb(122, 162, 247);     // #7aa2f7 - info/connecting
    pub const ORANGE: Color = Color::Rgb(255, 158, 100);   // #ff9e64 - warning
    pub const PURPLE: Color = Color::Rgb(187, 154, 247);   // #bb9af7 - special
    pub const CYAN: Color = Color::Rgb(125, 207, 255);     // #7dcfff - highlight
    pub const YELLOW: Color = Color::Rgb(224, 175, 104);   // #e0af68 - attention
    
    // Theme green for highlights
    pub const THEME_GREEN: Color = Color::Rgb(158, 206, 106);   // #9ece6a
    
    // Status indicator colors
    pub const STATUS_ONLINE: Color = Self::GREEN;
    pub const STATUS_OFFLINE: Color = Self::RED;
    pub const STATUS_CONNECTING: Color = Self::BLUE;
    pub const STATUS_WARNING: Color = Self::ORANGE;
    pub const STATUS_UNKNOWN: Color = Self::COMMENT;
}

