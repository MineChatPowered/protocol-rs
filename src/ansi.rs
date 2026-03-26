use crate::types::MessageContent;
use kyori_component_json::{Color, Component, NamedColor};
use nu_ansi_term::{Color as NuColor, Style};

/// Converts a [`MessageContent`] to an ANSI escape code string for terminal display.
///
/// For CommonMark messages, returns the text as-is.
/// For Component messages, applies colors and formatting using ANSI escape codes.
pub fn message_content_to_ansi(content: &MessageContent) -> String {
    match content {
        MessageContent::CommonMark(text) => text.clone(),
        MessageContent::Components(component) => component_to_ansi(component),
    }
}

/// Converts a Minecraft [`Component`] to an ANSI escape code string for terminal display.
///
/// This function traverses the component tree and applies ANSI styles for:
/// - Colors (named and hex)
/// - Bold, italic, underline, strikethrough
pub fn component_to_ansi(component: &Component) -> String {
    let mut output = String::new();
    render_component(component, &Style::default(), &mut output);
    output
}

fn render_component(component: &Component, inherited_style: &Style, output: &mut String) {
    match component {
        Component::String(text) => {
            let styled = apply_style(inherited_style, text);
            output.push_str(&styled);
        }
        Component::Object(obj) => {
            let style = merge_style(inherited_style, obj);
            if let Some(text) = &obj.text {
                let styled = apply_style(&style, text);
                output.push_str(&styled);
            }
            if let Some(children) = &obj.extra {
                for child in children {
                    render_component(child, &style, output);
                }
            }
        }
        Component::Array(components) => {
            for c in components {
                render_component(c, inherited_style, output);
            }
        }
    }
}

fn merge_style(parent: &Style, obj: &kyori_component_json::ComponentObject) -> Style {
    let mut style = *parent;

    if let Some(color) = &obj.color {
        if let Some(nu_color) = color_to_nuansi(color) {
            style = style.fg(nu_color);
        }
    }

    if obj.bold == Some(true) {
        style = style.bold();
    }
    if obj.italic == Some(true) {
        style = style.italic();
    }
    if obj.underlined == Some(true) {
        style = style.underline();
    }
    if obj.strikethrough == Some(true) {
        style = style.strikethrough();
    }

    style
}

fn apply_style(style: &Style, text: &str) -> String {
    if style == &Style::default() {
        text.to_string()
    } else {
        style.paint(text).to_string()
    }
}

fn color_to_nuansi(color: &Color) -> Option<NuColor> {
    match color {
        Color::Named(named) => Some(named_color_to_nuansi(*named)),
        Color::Hex(hex) => {
            if hex.starts_with('#') && hex.len() == 7 {
                let r = u8::from_str_radix(&hex[1..3], 16).ok()?;
                let g = u8::from_str_radix(&hex[3..5], 16).ok()?;
                let b = u8::from_str_radix(&hex[5..7], 16).ok()?;
                Some(NuColor::Rgb(r, g, b))
            } else {
                None
            }
        }
    }
}

fn named_color_to_nuansi(color: NamedColor) -> NuColor {
    match color {
        NamedColor::Black => NuColor::Black,
        NamedColor::DarkBlue => NuColor::Fixed(21),
        NamedColor::DarkGreen => NuColor::Fixed(28),
        NamedColor::DarkAqua => NuColor::Fixed(30),
        NamedColor::DarkRed => NuColor::Fixed(124),
        NamedColor::DarkPurple => NuColor::Fixed(127),
        NamedColor::Gold => NuColor::Fixed(214),
        NamedColor::Gray => NuColor::Fixed(145),
        NamedColor::DarkGray => NuColor::Fixed(59),
        NamedColor::Blue => NuColor::Fixed(75),
        NamedColor::Green => NuColor::Fixed(82),
        NamedColor::Aqua => NuColor::Fixed(86),
        NamedColor::Red => NuColor::Fixed(203),
        NamedColor::LightPurple => NuColor::Fixed(212),
        NamedColor::Yellow => NuColor::Fixed(227),
        NamedColor::White => NuColor::White,
    }
}
