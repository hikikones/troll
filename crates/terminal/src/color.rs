#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BlackBright,
    RedBright,
    GreenBright,
    YellowBright,
    BlueBright,
    MagentaBright,
    CyanBright,
    WhiteBright,
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl Color {
    pub const fn from_u8(i: u8) -> Color {
        match i {
            // 0-15: standard ANSI colors
            0 => Self::Black,
            1 => Self::Red,
            2 => Self::Green,
            3 => Self::Yellow,
            4 => Self::Blue,
            5 => Self::Magenta,
            6 => Self::Cyan,
            7 => Self::White,
            8 => Self::BlackBright,
            9 => Self::RedBright,
            10 => Self::GreenBright,
            11 => Self::YellowBright,
            12 => Self::BlueBright,
            13 => Self::MagentaBright,
            14 => Self::CyanBright,
            15 => Self::WhiteBright,

            // 16-231: 6x6x6 color cube
            16..=231 => {
                let i = i - 16;
                let r = (i / 36) % 6;
                let g = (i / 6) % 6;
                let b = i % 6;

                const fn scale(v: u8) -> u8 {
                    if v == 0 { 0 } else { v * 40 + 55 }
                }

                Color::Rgb(scale(r), scale(g), scale(b))
            }

            // 232-255: grayscale ramp
            232..=255 => {
                let gray = 8 + (i - 232) * 10;
                Color::Rgb(gray, gray, gray)
            }
        }
    }
}

impl std::str::FromStr for Color {
    type Err = ParseColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(c) = s.chars().next() else {
            return Err(ParseColorError {
                message: String::from("Failed to parse color from empty string."),
                source: None,
            });
        };

        if c == '#' {
            let hex = &s[1..];

            if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(ParseColorError {
                    message: format!(
                        "Failed to parse hex color '{s}'. Expected exactly 6 hexadecimal digits."
                    ),
                    source: None,
                });
            }

            let (r, g, b) = parse_hex_color(hex).map_err(|err| ParseColorError {
                message: format!("Failed to parse hex color '{s}'."),
                source: Some(err),
            })?;

            Ok(Self::Rgb(r, g, b))
        } else if c.is_ascii_digit() {
            let indexed_value = s.parse::<u8>().map_err(|err| ParseColorError {
                message: format!(
                    "Failed to parse indexed color value from '{s}'.\nNumber should be 0-255."
                ),
                source: Some(err),
            })?;

            Ok(Self::Indexed(indexed_value))
        } else {
            match COLOR_NAMES
                .iter()
                .find(|(name, _)| s.eq_ignore_ascii_case(name))
            {
                Some((_, color)) => Ok(*color),
                None => Err(ParseColorError {
                    message: format!("Failed to parse color by name '{s}'."),
                    source: None,
                }),
            }
        }
    }
}

fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8), std::num::ParseIntError> {
    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;
    Ok((r, g, b))
}

const COLOR_NAMES: &[(&str, Color)] = &[
    ("black", Color::Black),
    ("red", Color::Red),
    ("green", Color::Green),
    ("yellow", Color::Yellow),
    ("blue", Color::Blue),
    ("magenta", Color::Magenta),
    ("cyan", Color::Cyan),
    ("white", Color::White),
    ("blackbright", Color::BlackBright),
    ("redbright", Color::RedBright),
    ("greenbright", Color::GreenBright),
    ("yellowbright", Color::YellowBright),
    ("bluebright", Color::BlueBright),
    ("magentabright", Color::MagentaBright),
    ("cyanbright", Color::CyanBright),
    ("whitebright", Color::WhiteBright),
    ("default", Color::Default),
    ("normal", Color::Default),
    ("reset", Color::Default),
];

#[derive(Debug)]
pub struct ParseColorError {
    message: String,
    source: Option<std::num::ParseIntError>,
}

impl std::fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl std::error::Error for ParseColorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|err| err as _)
    }
}

impl<'de> serde::Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Black => f.write_str("Black"),
            Self::Red => f.write_str("Red"),
            Self::Green => f.write_str("Green"),
            Self::Yellow => f.write_str("Yellow"),
            Self::Blue => f.write_str("Blue"),
            Self::Magenta => f.write_str("Magenta"),
            Self::Cyan => f.write_str("Cyan"),
            Self::White => f.write_str("White"),

            Self::BlackBright => f.write_str("BlackBright"),
            Self::RedBright => f.write_str("RedBright"),
            Self::GreenBright => f.write_str("GreenBright"),
            Self::YellowBright => f.write_str("YellowBright"),
            Self::BlueBright => f.write_str("BlueBright"),
            Self::MagentaBright => f.write_str("MagentaBright"),
            Self::CyanBright => f.write_str("CyanBright"),
            Self::WhiteBright => f.write_str("WhiteBright"),

            Self::Default => f.write_str("Default"),
            Self::Indexed(i) => write!(f, "{i}"),
            Self::Rgb(r, g, b) => write!(f, "#{r:02x}{g:02x}{b:02x}"),
        }
    }
}

impl serde::Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}
