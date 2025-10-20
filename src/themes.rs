use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// Available theme variants
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ThemeVariant {
    TokyoNightDark,
    TokyoNightLight,
    DraculaDark,
    GruvboxDark,
    GruvboxLight,
    NordDark,
    SolarizedDark,
    SolarizedLight,
    MonokaiDark,
    CatppuccinDark,
    OneDark,
    Ayu,
}

impl Default for ThemeVariant {
    fn default() -> Self {
        ThemeVariant::TokyoNightDark
    }
}

impl ThemeVariant {
    pub fn all() -> Vec<ThemeVariant> {
        vec![
            ThemeVariant::TokyoNightDark,
            ThemeVariant::TokyoNightLight,
            ThemeVariant::DraculaDark,
            ThemeVariant::GruvboxDark,
            ThemeVariant::GruvboxLight,
            ThemeVariant::NordDark,
            ThemeVariant::SolarizedDark,
            ThemeVariant::SolarizedLight,
            ThemeVariant::MonokaiDark,
            ThemeVariant::CatppuccinDark,
            ThemeVariant::OneDark,
            ThemeVariant::Ayu,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ThemeVariant::TokyoNightDark => "Tokyo Night (Dark)",
            ThemeVariant::TokyoNightLight => "Tokyo Night (Light)",
            ThemeVariant::DraculaDark => "Dracula",
            ThemeVariant::GruvboxDark => "Gruvbox (Dark)",
            ThemeVariant::GruvboxLight => "Gruvbox (Light)",
            ThemeVariant::NordDark => "Nord",
            ThemeVariant::SolarizedDark => "Solarized (Dark)",
            ThemeVariant::SolarizedLight => "Solarized (Light)",
            ThemeVariant::MonokaiDark => "Monokai",
            ThemeVariant::CatppuccinDark => "Catppuccin",
            ThemeVariant::OneDark => "One Dark",
            ThemeVariant::Ayu => "Ayu",
        }
    }

    pub fn is_dark(&self) -> bool {
        match self {
            ThemeVariant::TokyoNightLight | ThemeVariant::GruvboxLight | ThemeVariant::SolarizedLight => false,
            _ => true,
        }
    }
}

/// Comprehensive theme structure with all colors needed for the app
#[derive(Debug, Clone)]
pub struct Theme {
    // Base colors
    pub bg: Color,
    pub bg_dark: Color,
    pub bg_highlight: Color,
    pub bg_popup: Color,
    pub fg: Color,
    pub fg_dark: Color,
    pub comment: Color,
    
    // UI element colors
    pub border: Color,
    pub border_highlight: Color,
    pub cursor: Color,
    
    // Accent colors
    pub theme_primary: Color,
    pub theme_secondary: Color,
    
    // Status colors
    pub status_online: Color,
    pub status_offline: Color,
    pub status_connecting: Color,
    pub status_warning: Color,
    pub status_unknown: Color,
    
    // Semantic colors
    pub red: Color,
    pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    pub cyan: Color,
    pub blue: Color,
    pub purple: Color,
    pub pink: Color,
    
    // Special colors
    pub terminal_black: Color,
    pub selection: Color,
    pub match_highlight: Color,
}

impl Theme {
    pub fn from_variant(variant: ThemeVariant) -> Self {
        match variant {
            ThemeVariant::TokyoNightDark => Self::tokyo_night_dark(),
            ThemeVariant::TokyoNightLight => Self::tokyo_night_light(),
            ThemeVariant::DraculaDark => Self::dracula_dark(),
            ThemeVariant::GruvboxDark => Self::gruvbox_dark(),
            ThemeVariant::GruvboxLight => Self::gruvbox_light(),
            ThemeVariant::NordDark => Self::nord_dark(),
            ThemeVariant::SolarizedDark => Self::solarized_dark(),
            ThemeVariant::SolarizedLight => Self::solarized_light(),
            ThemeVariant::MonokaiDark => Self::monokai_dark(),
            ThemeVariant::CatppuccinDark => Self::catppuccin_dark(),
            ThemeVariant::OneDark => Self::one_dark(),
            ThemeVariant::Ayu => Self::ayu(),
        }
    }

    fn tokyo_night_dark() -> Self {
        Self {
            bg: Color::Rgb(26, 27, 38),
            bg_dark: Color::Rgb(22, 23, 32),
            bg_highlight: Color::Rgb(54, 54, 68),
            bg_popup: Color::Rgb(30, 31, 44),
            fg: Color::Rgb(169, 177, 214),
            fg_dark: Color::Rgb(130, 137, 172),
            comment: Color::Rgb(86, 95, 137),
            
            border: Color::Rgb(54, 54, 68),
            border_highlight: Color::Rgb(125, 207, 255),
            cursor: Color::Rgb(125, 207, 255),
            
            theme_primary: Color::Rgb(125, 207, 255),
            theme_secondary: Color::Rgb(158, 206, 106),
            
            status_online: Color::Rgb(158, 206, 106),
            status_offline: Color::Rgb(247, 118, 142),
            status_connecting: Color::Rgb(255, 158, 100),
            status_warning: Color::Rgb(224, 175, 104),
            status_unknown: Color::Rgb(86, 95, 137),
            
            red: Color::Rgb(247, 118, 142),
            orange: Color::Rgb(255, 158, 100),
            yellow: Color::Rgb(224, 175, 104),
            green: Color::Rgb(158, 206, 106),
            cyan: Color::Rgb(125, 207, 255),
            blue: Color::Rgb(125, 207, 255),
            purple: Color::Rgb(187, 154, 247),
            pink: Color::Rgb(247, 118, 142),
            
            terminal_black: Color::Rgb(22, 23, 32),
            selection: Color::Rgb(54, 54, 68),
            match_highlight: Color::Rgb(255, 158, 100),
        }
    }

    fn tokyo_night_light() -> Self {
        Self {
            bg: Color::Rgb(213, 214, 219),
            bg_dark: Color::Rgb(200, 201, 206),
            bg_highlight: Color::Rgb(180, 181, 186),
            bg_popup: Color::Rgb(230, 231, 236),
            fg: Color::Rgb(52, 59, 88),
            fg_dark: Color::Rgb(77, 84, 113),
            comment: Color::Rgb(143, 147, 161),
            
            border: Color::Rgb(180, 181, 186),
            border_highlight: Color::Rgb(52, 84, 138),
            cursor: Color::Rgb(52, 84, 138),
            
            theme_primary: Color::Rgb(52, 84, 138),
            theme_secondary: Color::Rgb(72, 94, 48),
            
            status_online: Color::Rgb(72, 94, 48),
            status_offline: Color::Rgb(150, 65, 89),
            status_connecting: Color::Rgb(166, 77, 17),
            status_warning: Color::Rgb(140, 102, 15),
            status_unknown: Color::Rgb(143, 147, 161),
            
            red: Color::Rgb(150, 65, 89),
            orange: Color::Rgb(166, 77, 17),
            yellow: Color::Rgb(140, 102, 15),
            green: Color::Rgb(72, 94, 48),
            cyan: Color::Rgb(15, 75, 110),
            blue: Color::Rgb(52, 84, 138),
            purple: Color::Rgb(90, 74, 120),
            pink: Color::Rgb(150, 65, 89),
            
            terminal_black: Color::Rgb(200, 201, 206),
            selection: Color::Rgb(180, 181, 186),
            match_highlight: Color::Rgb(166, 77, 17),
        }
    }

    fn dracula_dark() -> Self {
        Self {
            bg: Color::Rgb(40, 42, 54),
            bg_dark: Color::Rgb(33, 35, 45),
            bg_highlight: Color::Rgb(68, 71, 90),
            bg_popup: Color::Rgb(44, 47, 58),
            fg: Color::Rgb(248, 248, 242),
            fg_dark: Color::Rgb(189, 147, 249),
            comment: Color::Rgb(98, 114, 164),
            
            border: Color::Rgb(68, 71, 90),
            border_highlight: Color::Rgb(139, 233, 253),
            cursor: Color::Rgb(248, 248, 242),
            
            theme_primary: Color::Rgb(189, 147, 249),
            theme_secondary: Color::Rgb(80, 250, 123),
            
            status_online: Color::Rgb(80, 250, 123),
            status_offline: Color::Rgb(255, 85, 85),
            status_connecting: Color::Rgb(255, 184, 108),
            status_warning: Color::Rgb(241, 250, 140),
            status_unknown: Color::Rgb(98, 114, 164),
            
            red: Color::Rgb(255, 85, 85),
            orange: Color::Rgb(255, 184, 108),
            yellow: Color::Rgb(241, 250, 140),
            green: Color::Rgb(80, 250, 123),
            cyan: Color::Rgb(139, 233, 253),
            blue: Color::Rgb(98, 114, 164),
            purple: Color::Rgb(189, 147, 249),
            pink: Color::Rgb(255, 121, 198),
            
            terminal_black: Color::Rgb(33, 35, 45),
            selection: Color::Rgb(68, 71, 90),
            match_highlight: Color::Rgb(255, 184, 108),
        }
    }

    fn gruvbox_dark() -> Self {
        Self {
            bg: Color::Rgb(40, 40, 40),
            bg_dark: Color::Rgb(29, 32, 33),
            bg_highlight: Color::Rgb(60, 56, 54),
            bg_popup: Color::Rgb(50, 48, 47),
            fg: Color::Rgb(235, 219, 178),
            fg_dark: Color::Rgb(213, 196, 161),
            comment: Color::Rgb(146, 131, 116),
            
            border: Color::Rgb(80, 73, 69),
            border_highlight: Color::Rgb(131, 165, 152),
            cursor: Color::Rgb(235, 219, 178),
            
            theme_primary: Color::Rgb(184, 187, 38),
            theme_secondary: Color::Rgb(142, 192, 124),
            
            status_online: Color::Rgb(142, 192, 124),
            status_offline: Color::Rgb(251, 73, 52),
            status_connecting: Color::Rgb(254, 128, 25),
            status_warning: Color::Rgb(250, 189, 47),
            status_unknown: Color::Rgb(146, 131, 116),
            
            red: Color::Rgb(251, 73, 52),
            orange: Color::Rgb(254, 128, 25),
            yellow: Color::Rgb(250, 189, 47),
            green: Color::Rgb(142, 192, 124),
            cyan: Color::Rgb(131, 165, 152),
            blue: Color::Rgb(131, 165, 152),
            purple: Color::Rgb(211, 134, 155),
            pink: Color::Rgb(211, 134, 155),
            
            terminal_black: Color::Rgb(29, 32, 33),
            selection: Color::Rgb(60, 56, 54),
            match_highlight: Color::Rgb(254, 128, 25),
        }
    }

    fn gruvbox_light() -> Self {
        Self {
            bg: Color::Rgb(251, 241, 199),
            bg_dark: Color::Rgb(242, 229, 188),
            bg_highlight: Color::Rgb(235, 219, 178),
            bg_popup: Color::Rgb(249, 245, 215),
            fg: Color::Rgb(60, 56, 54),
            fg_dark: Color::Rgb(80, 73, 69),
            comment: Color::Rgb(146, 131, 116),
            
            border: Color::Rgb(189, 174, 147),
            border_highlight: Color::Rgb(121, 116, 14),
            cursor: Color::Rgb(60, 56, 54),
            
            theme_primary: Color::Rgb(121, 116, 14),
            theme_secondary: Color::Rgb(102, 92, 84),
            
            status_online: Color::Rgb(121, 116, 14),
            status_offline: Color::Rgb(157, 0, 6),
            status_connecting: Color::Rgb(175, 58, 3),
            status_warning: Color::Rgb(181, 118, 20),
            status_unknown: Color::Rgb(146, 131, 116),
            
            red: Color::Rgb(157, 0, 6),
            orange: Color::Rgb(175, 58, 3),
            yellow: Color::Rgb(181, 118, 20),
            green: Color::Rgb(121, 116, 14),
            cyan: Color::Rgb(66, 123, 88),
            blue: Color::Rgb(7, 102, 120),
            purple: Color::Rgb(143, 63, 113),
            pink: Color::Rgb(143, 63, 113),
            
            terminal_black: Color::Rgb(242, 229, 188),
            selection: Color::Rgb(235, 219, 178),
            match_highlight: Color::Rgb(175, 58, 3),
        }
    }

    fn nord_dark() -> Self {
        Self {
            bg: Color::Rgb(46, 52, 64),
            bg_dark: Color::Rgb(35, 39, 49),
            bg_highlight: Color::Rgb(59, 66, 82),
            bg_popup: Color::Rgb(51, 57, 70),
            fg: Color::Rgb(216, 222, 233),
            fg_dark: Color::Rgb(229, 233, 240),
            comment: Color::Rgb(76, 86, 106),
            
            border: Color::Rgb(67, 76, 94),
            border_highlight: Color::Rgb(136, 192, 208),
            cursor: Color::Rgb(216, 222, 233),
            
            theme_primary: Color::Rgb(94, 129, 172),
            theme_secondary: Color::Rgb(163, 190, 140),
            
            status_online: Color::Rgb(163, 190, 140),
            status_offline: Color::Rgb(191, 97, 106),
            status_connecting: Color::Rgb(235, 203, 139),
            status_warning: Color::Rgb(235, 203, 139),
            status_unknown: Color::Rgb(76, 86, 106),
            
            red: Color::Rgb(191, 97, 106),
            orange: Color::Rgb(208, 135, 112),
            yellow: Color::Rgb(235, 203, 139),
            green: Color::Rgb(163, 190, 140),
            cyan: Color::Rgb(136, 192, 208),
            blue: Color::Rgb(94, 129, 172),
            purple: Color::Rgb(180, 142, 173),
            pink: Color::Rgb(180, 142, 173),
            
            terminal_black: Color::Rgb(35, 39, 49),
            selection: Color::Rgb(59, 66, 82),
            match_highlight: Color::Rgb(235, 203, 139),
        }
    }

    fn solarized_dark() -> Self {
        Self {
            bg: Color::Rgb(0, 43, 54),
            bg_dark: Color::Rgb(0, 30, 38),
            bg_highlight: Color::Rgb(7, 54, 66),
            bg_popup: Color::Rgb(0, 36, 46),
            fg: Color::Rgb(131, 148, 150),
            fg_dark: Color::Rgb(147, 161, 161),
            comment: Color::Rgb(88, 110, 117),
            
            border: Color::Rgb(7, 54, 66),
            border_highlight: Color::Rgb(42, 161, 152),
            cursor: Color::Rgb(131, 148, 150),
            
            theme_primary: Color::Rgb(38, 139, 210),
            theme_secondary: Color::Rgb(133, 153, 0),
            
            status_online: Color::Rgb(133, 153, 0),
            status_offline: Color::Rgb(220, 50, 47),
            status_connecting: Color::Rgb(203, 75, 22),
            status_warning: Color::Rgb(181, 137, 0),
            status_unknown: Color::Rgb(88, 110, 117),
            
            red: Color::Rgb(220, 50, 47),
            orange: Color::Rgb(203, 75, 22),
            yellow: Color::Rgb(181, 137, 0),
            green: Color::Rgb(133, 153, 0),
            cyan: Color::Rgb(42, 161, 152),
            blue: Color::Rgb(38, 139, 210),
            purple: Color::Rgb(108, 113, 196),
            pink: Color::Rgb(211, 54, 130),
            
            terminal_black: Color::Rgb(0, 30, 38),
            selection: Color::Rgb(7, 54, 66),
            match_highlight: Color::Rgb(203, 75, 22),
        }
    }

    fn solarized_light() -> Self {
        Self {
            bg: Color::Rgb(253, 246, 227),
            bg_dark: Color::Rgb(238, 232, 213),
            bg_highlight: Color::Rgb(238, 232, 213),
            bg_popup: Color::Rgb(248, 241, 222),
            fg: Color::Rgb(101, 123, 131),
            fg_dark: Color::Rgb(88, 110, 117),
            comment: Color::Rgb(147, 161, 161),
            
            border: Color::Rgb(238, 232, 213),
            border_highlight: Color::Rgb(38, 139, 210),
            cursor: Color::Rgb(101, 123, 131),
            
            theme_primary: Color::Rgb(38, 139, 210),
            theme_secondary: Color::Rgb(133, 153, 0),
            
            status_online: Color::Rgb(133, 153, 0),
            status_offline: Color::Rgb(220, 50, 47),
            status_connecting: Color::Rgb(203, 75, 22),
            status_warning: Color::Rgb(181, 137, 0),
            status_unknown: Color::Rgb(147, 161, 161),
            
            red: Color::Rgb(220, 50, 47),
            orange: Color::Rgb(203, 75, 22),
            yellow: Color::Rgb(181, 137, 0),
            green: Color::Rgb(133, 153, 0),
            cyan: Color::Rgb(42, 161, 152),
            blue: Color::Rgb(38, 139, 210),
            purple: Color::Rgb(108, 113, 196),
            pink: Color::Rgb(211, 54, 130),
            
            terminal_black: Color::Rgb(238, 232, 213),
            selection: Color::Rgb(238, 232, 213),
            match_highlight: Color::Rgb(203, 75, 22),
        }
    }

    fn monokai_dark() -> Self {
        Self {
            bg: Color::Rgb(39, 40, 34),
            bg_dark: Color::Rgb(30, 31, 25),
            bg_highlight: Color::Rgb(73, 72, 62),
            bg_popup: Color::Rgb(45, 46, 40),
            fg: Color::Rgb(248, 248, 242),
            fg_dark: Color::Rgb(117, 113, 94),
            comment: Color::Rgb(117, 113, 94),
            
            border: Color::Rgb(73, 72, 62),
            border_highlight: Color::Rgb(102, 217, 239),
            cursor: Color::Rgb(248, 248, 242),
            
            theme_primary: Color::Rgb(249, 38, 114),
            theme_secondary: Color::Rgb(166, 226, 46),
            
            status_online: Color::Rgb(166, 226, 46),
            status_offline: Color::Rgb(249, 38, 114),
            status_connecting: Color::Rgb(253, 151, 31),
            status_warning: Color::Rgb(230, 219, 116),
            status_unknown: Color::Rgb(117, 113, 94),
            
            red: Color::Rgb(249, 38, 114),
            orange: Color::Rgb(253, 151, 31),
            yellow: Color::Rgb(230, 219, 116),
            green: Color::Rgb(166, 226, 46),
            cyan: Color::Rgb(102, 217, 239),
            blue: Color::Rgb(102, 217, 239),
            purple: Color::Rgb(174, 129, 255),
            pink: Color::Rgb(249, 38, 114),
            
            terminal_black: Color::Rgb(30, 31, 25),
            selection: Color::Rgb(73, 72, 62),
            match_highlight: Color::Rgb(253, 151, 31),
        }
    }

    fn catppuccin_dark() -> Self {
        Self {
            bg: Color::Rgb(30, 30, 46),
            bg_dark: Color::Rgb(24, 24, 37),
            bg_highlight: Color::Rgb(49, 50, 68),
            bg_popup: Color::Rgb(35, 38, 52),
            fg: Color::Rgb(205, 214, 244),
            fg_dark: Color::Rgb(166, 173, 200),
            comment: Color::Rgb(108, 112, 134),
            
            border: Color::Rgb(49, 50, 68),
            border_highlight: Color::Rgb(137, 180, 250),
            cursor: Color::Rgb(245, 224, 220),
            
            theme_primary: Color::Rgb(203, 166, 247),
            theme_secondary: Color::Rgb(166, 227, 161),
            
            status_online: Color::Rgb(166, 227, 161),
            status_offline: Color::Rgb(243, 139, 168),
            status_connecting: Color::Rgb(250, 179, 135),
            status_warning: Color::Rgb(249, 226, 175),
            status_unknown: Color::Rgb(108, 112, 134),
            
            red: Color::Rgb(243, 139, 168),
            orange: Color::Rgb(250, 179, 135),
            yellow: Color::Rgb(249, 226, 175),
            green: Color::Rgb(166, 227, 161),
            cyan: Color::Rgb(148, 226, 213),
            blue: Color::Rgb(137, 180, 250),
            purple: Color::Rgb(203, 166, 247),
            pink: Color::Rgb(245, 194, 231),
            
            terminal_black: Color::Rgb(24, 24, 37),
            selection: Color::Rgb(49, 50, 68),
            match_highlight: Color::Rgb(250, 179, 135),
        }
    }

    fn one_dark() -> Self {
        Self {
            bg: Color::Rgb(40, 44, 52),
            bg_dark: Color::Rgb(33, 37, 43),
            bg_highlight: Color::Rgb(55, 59, 69),
            bg_popup: Color::Rgb(46, 50, 58),
            fg: Color::Rgb(171, 178, 191),
            fg_dark: Color::Rgb(130, 137, 151),
            comment: Color::Rgb(92, 99, 112),
            
            border: Color::Rgb(55, 59, 69),
            border_highlight: Color::Rgb(97, 175, 239),
            cursor: Color::Rgb(171, 178, 191),
            
            theme_primary: Color::Rgb(97, 175, 239),
            theme_secondary: Color::Rgb(152, 195, 121),
            
            status_online: Color::Rgb(152, 195, 121),
            status_offline: Color::Rgb(224, 108, 117),
            status_connecting: Color::Rgb(209, 154, 102),
            status_warning: Color::Rgb(229, 192, 123),
            status_unknown: Color::Rgb(92, 99, 112),
            
            red: Color::Rgb(224, 108, 117),
            orange: Color::Rgb(209, 154, 102),
            yellow: Color::Rgb(229, 192, 123),
            green: Color::Rgb(152, 195, 121),
            cyan: Color::Rgb(86, 182, 194),
            blue: Color::Rgb(97, 175, 239),
            purple: Color::Rgb(198, 120, 221),
            pink: Color::Rgb(198, 120, 221),
            
            terminal_black: Color::Rgb(33, 37, 43),
            selection: Color::Rgb(55, 59, 69),
            match_highlight: Color::Rgb(209, 154, 102),
        }
    }

    fn ayu() -> Self {
        Self {
            bg: Color::Rgb(15, 20, 25),
            bg_dark: Color::Rgb(10, 14, 17),
            bg_highlight: Color::Rgb(25, 30, 37),
            bg_popup: Color::Rgb(19, 24, 30),
            fg: Color::Rgb(230, 237, 243),
            fg_dark: Color::Rgb(191, 197, 202),
            comment: Color::Rgb(92, 103, 115),
            
            border: Color::Rgb(25, 30, 37),
            border_highlight: Color::Rgb(57, 186, 230),
            cursor: Color::Rgb(230, 237, 243),
            
            theme_primary: Color::Rgb(255, 180, 84),
            theme_secondary: Color::Rgb(191, 199, 75),
            
            status_online: Color::Rgb(191, 199, 75),
            status_offline: Color::Rgb(242, 151, 24),
            status_connecting: Color::Rgb(255, 180, 84),
            status_warning: Color::Rgb(230, 194, 122),
            status_unknown: Color::Rgb(92, 103, 115),
            
            red: Color::Rgb(242, 151, 24),
            orange: Color::Rgb(255, 180, 84),
            yellow: Color::Rgb(230, 194, 122),
            green: Color::Rgb(191, 199, 75),
            cyan: Color::Rgb(57, 186, 230),
            blue: Color::Rgb(57, 186, 230),
            purple: Color::Rgb(242, 151, 24),
            pink: Color::Rgb(230, 194, 122),
            
            terminal_black: Color::Rgb(10, 14, 17),
            selection: Color::Rgb(25, 30, 37),
            match_highlight: Color::Rgb(255, 180, 84),
        }
    }
}

/// Theme manager for the application
#[derive(Debug, Clone)]
pub struct ThemeManager {
    current_theme: Theme,
    current_variant: ThemeVariant,
}

impl Default for ThemeManager {
    fn default() -> Self {
        let variant = ThemeVariant::default();
        Self {
            current_theme: Theme::from_variant(variant),
            current_variant: variant,
        }
    }
}

impl ThemeManager {
    pub fn new(variant: ThemeVariant) -> Self {
        Self {
            current_theme: Theme::from_variant(variant),
            current_variant: variant,
        }
    }

    pub fn current_theme(&self) -> &Theme {
        &self.current_theme
    }

    pub fn current_variant(&self) -> ThemeVariant {
        self.current_variant
    }

    pub fn set_theme(&mut self, variant: ThemeVariant) {
        self.current_variant = variant;
        self.current_theme = Theme::from_variant(variant);
    }

    pub fn next_theme(&mut self) {
        let variants = ThemeVariant::all();
        let current_index = variants
            .iter()
            .position(|&v| v == self.current_variant)
            .unwrap_or(0);
        let next_index = (current_index + 1) % variants.len();
        self.set_theme(variants[next_index]);
    }

    pub fn previous_theme(&mut self) {
        let variants = ThemeVariant::all();
        let current_index = variants
            .iter()
            .position(|&v| v == self.current_variant)
            .unwrap_or(0);
        let prev_index = if current_index == 0 {
            variants.len() - 1
        } else {
            current_index - 1
        };
        self.set_theme(variants[prev_index]);
    }

    pub fn is_dark(&self) -> bool {
        self.current_variant.is_dark()
    }
}