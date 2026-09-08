//! Named theme colours. Ratatui-facing conversion (`TuiPalette`) lives in `sp-ui`, since
//! `sp-core` must not depend on ratatui.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An RGB colour, serialized as a `#rrggbb` hex string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid hex colour {0:?}, expected #rrggbb")]
pub struct ColorParseError(String);

impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        if hex.len() != 6 {
            return Err(ColorParseError(s.to_string()));
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ColorParseError(s.to_string()));
        Ok(Color::new(byte(0)?, byte(2)?, byte(4)?))
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// A named colour theme. Field names match the palette carried by the archived Excalith
/// `data/themes/*.json` fixtures (background/window/text plus the 8 ANSI-ish names).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub background: Color,
    pub window: Color,
    pub text: Color,
    pub black: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
    pub white: Color,
    pub gray: Color,
}

impl Default for Theme {
    /// Matches `data/themes/default.json`.
    fn default() -> Self {
        Theme {
            name: "Default".to_string(),
            background: Color::new(0x12, 0x13, 0x17),
            window: Color::new(0x1e, 0x21, 0x2b),
            text: Color::new(0xe2, 0xe2, 0xe2),
            black: Color::new(0x16, 0x16, 0x1e),
            red: Color::new(0xec, 0x61, 0x83),
            green: Color::new(0x2e, 0xd8, 0xa2),
            yellow: Color::new(0xe8, 0xb1, 0x95),
            blue: Color::new(0x2b, 0xc3, 0xde),
            magenta: Color::new(0xe0, 0x69, 0xaa),
            cyan: Color::new(0x62, 0xe0, 0xe2),
            white: Color::new(0xe2, 0xe2, 0xe2),
            gray: Color::new(0x97, 0x98, 0x9d),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_round_trips_through_hex() {
        let c = Color::new(0x2b, 0xc3, 0xde);
        assert_eq!(c.to_string(), "#2bc3de");
        assert_eq!("#2bc3de".parse::<Color>().unwrap(), c);
    }

    #[test]
    fn color_rejects_bad_hex() {
        assert!("not-a-color".parse::<Color>().is_err());
    }

    #[test]
    fn theme_round_trips_through_toml() {
        let theme = Theme::default();
        let toml = toml::to_string(&theme).unwrap();
        let back: Theme = toml::from_str(&toml).unwrap();
        assert_eq!(theme, back);
    }
}
