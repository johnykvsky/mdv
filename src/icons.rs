//! Embedded vector icons from Phosphor Icons (Regular weight)
#![allow(dead_code)]

pub const ARROW_LEFT: &str = "\u{E058}";
pub const ARROW_RIGHT: &str = "\u{E06C}";
pub const FOLDER_OPEN: &str = "\u{E256}";
pub const CIRCLE_HALF: &str = "\u{E18C}";
pub const SUN: &str = "\u{E472}";
pub const MOON: &str = "\u{E330}";
pub const CORNERS_OUT: &str = "\u{E1D0}";
pub const CORNERS_IN: &str = "\u{E1CE}";
pub const QUESTION: &str = "\u{E3E8}";
pub const ARROWS_HORIZONTAL: &str = "\u{EB06}";
pub const ARROWS_IN_LINE_HORIZONTAL: &str = "\u{E530}";
pub const TEXT_AA: &str = "\u{E6EE}";
pub const COPY: &str = "\u{E1CA}";
pub const CHECK: &str = "\u{E182}";
pub const MAGNIFYING_GLASS: &str = "\u{E30C}";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arrow_icons() {
        assert_ne!(ARROW_LEFT, ARROW_RIGHT);
        assert_eq!(ARROW_LEFT, "\u{E058}");
        assert_eq!(ARROW_RIGHT, "\u{E06C}");
    }
}
