use iced::theme::Palette;
use iced::{Color, Theme};

/// GitHub Dark theme colors
fn github_dark_palette() -> Palette {
    Palette {
        background: Color::from_rgb(0.051, 0.067, 0.090), // #0d1117
        text: Color::from_rgb(0.902, 0.929, 0.953),       // #e6edf3
        primary: Color::from_rgb(0.345, 0.651, 1.0),      // #58a6ff
        success: Color::from_rgb(0.247, 0.725, 0.314),    // #3fb950
        warning: Color::from_rgb(0.824, 0.600, 0.133),    // #d29922
        danger: Color::from_rgb(0.973, 0.319, 0.286),     // #f85149
    }
}

/// GitHub Light theme colors
fn github_light_palette() -> Palette {
    Palette {
        background: Color::from_rgb(1.0, 1.0, 1.0),    // #ffffff
        text: Color::from_rgb(0.122, 0.137, 0.157),    // #1f2328
        primary: Color::from_rgb(0.035, 0.412, 0.855), // #0969da
        success: Color::from_rgb(0.102, 0.498, 0.216), // #1a7f37
        warning: Color::from_rgb(0.604, 0.404, 0.0),   // #9a6700
        danger: Color::from_rgb(0.812, 0.133, 0.180),  // #cf222e
    }
}

pub fn github_dark() -> Theme {
    Theme::custom("GitHub Dark".to_owned(), github_dark_palette())
}

pub fn github_light() -> Theme {
    Theme::custom("GitHub Light".to_owned(), github_light_palette())
}
