use ratatui::style::Color;

pub struct Theme {
    pub name: &'static str,
    pub accent: Color,
    pub active: Color,
    pub paused: Color,
    pub muted: Color,
    pub border: Color,
    pub highlight_bg: Color,
    pub text: Color,
}

pub const THEMES: [Theme; 4] = [
    Theme {
        name: "tokyonight",
        accent: Color::Rgb(122, 162, 247),
        active: Color::Rgb(158, 206, 106),
        paused: Color::Rgb(224, 175, 104),
        muted: Color::Rgb(86, 95, 137),
        border: Color::Rgb(65, 72, 104),
        highlight_bg: Color::Rgb(41, 46, 66),
        text: Color::Rgb(192, 202, 245),
    },
    Theme {
        name: "gruvbox",
        accent: Color::Rgb(250, 189, 47),
        active: Color::Rgb(184, 187, 38),
        paused: Color::Rgb(254, 128, 25),
        muted: Color::Rgb(146, 131, 116),
        border: Color::Rgb(80, 73, 69),
        highlight_bg: Color::Rgb(60, 56, 54),
        text: Color::Rgb(235, 219, 178),
    },
    Theme {
        name: "dracula",
        accent: Color::Rgb(189, 147, 249),
        active: Color::Rgb(80, 250, 123),
        paused: Color::Rgb(255, 184, 108),
        muted: Color::Rgb(98, 114, 164),
        border: Color::Rgb(68, 71, 90),
        highlight_bg: Color::Rgb(68, 71, 90),
        text: Color::Rgb(248, 248, 242),
    },
    Theme {
        name: "mono",
        accent: Color::White,
        active: Color::Green,
        paused: Color::Yellow,
        muted: Color::DarkGray,
        border: Color::DarkGray,
        highlight_bg: Color::Rgb(60, 60, 60),
        text: Color::Reset,
    },
];
