//! The glyph that a chart uses to draw a point.

use ratatui_core::symbols;

/// The set of glyphs that a chart uses to draw a point.
///
/// A terminal cell holds one character. Some characters divide the cell into
/// smaller parts. A chart that uses such a character can therefore draw more
/// points than the terminal has cells. This crate calls each such part a
/// sub-cell.
///
/// Braille is the default value, because more fonts hold the braille
/// characters than hold any other set. The octant characters look better, but
/// fewer fonts hold them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Marker {
    /// The Unicode Braille Patterns characters. Each cell holds 2 sub-cells
    /// across and 4 sub-cells down. More fonts hold these characters than hold
    /// any other set.
    #[default]
    Braille,
    /// The Unicode octant characters. Each cell holds 2 sub-cells across and 4
    /// sub-cells down, and no space separates one cell from the next. Unicode
    /// added these characters recently, so fewer fonts hold them.
    Octant,
    /// The Unicode half block characters. Each cell holds 1 sub-cell across and
    /// 2 sub-cells down. Every font holds these characters.
    HalfBlock,
    /// The Unicode full block character. Each cell holds one point. A plain
    /// terminal can always show this character.
    Block,
}

impl Marker {
    /// Returns the number of sub-cells across one terminal cell.
    ///
    /// The function `thunderbox_core::max_data_points` uses this number to decide
    /// how many points a panel asks for.
    pub fn horizontal_subcells(self) -> u16 {
        match self {
            Self::Braille | Self::Octant => 2,
            Self::HalfBlock | Self::Block => 1,
        }
    }

    /// Returns the number of sub-cells down one terminal cell.
    pub fn vertical_subcells(self) -> u16 {
        match self {
            Self::Braille | Self::Octant => 4,
            Self::HalfBlock => 2,
            Self::Block => 1,
        }
    }
}

impl From<Marker> for symbols::Marker {
    fn from(m: Marker) -> Self {
        match m {
            Marker::Braille => Self::Braille,
            Marker::Octant => Self::Octant,
            Marker::HalfBlock => Self::HalfBlock,
            Marker::Block => Self::Block,
        }
    }
}

impl std::str::FromStr for Marker {
    type Err = thunderbox_core::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "braille" => Ok(Self::Braille),
            "octant" => Ok(Self::Octant),
            "halfblock" | "half_block" => Ok(Self::HalfBlock),
            "block" | "tty" => Ok(Self::Block),
            other => Err(thunderbox_core::Error::Dashboard(format!(
                "unknown marker {other:?}; expected braille, octant, halfblock or block"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braille_and_octant_divide_a_cell_in_the_same_way() {
        assert_eq!(Marker::Braille.horizontal_subcells(), 2);
        assert_eq!(Marker::Octant.horizontal_subcells(), 2);
        assert_eq!(Marker::Braille.vertical_subcells(), 4);
        assert_eq!(Marker::Octant.vertical_subcells(), 4);
    }

    #[test]
    fn block_gives_one_point_for_each_cell() {
        assert_eq!(Marker::Block.horizontal_subcells(), 1);
        assert_eq!(Marker::Block.vertical_subcells(), 1);
    }

    #[test]
    fn from_str_reads_each_name_and_the_name_tty() {
        assert_eq!("braille".parse::<Marker>().unwrap(), Marker::Braille);
        assert_eq!("OCTANT".parse::<Marker>().unwrap(), Marker::Octant);
        assert_eq!("tty".parse::<Marker>().unwrap(), Marker::Block);
        assert!("swirly".parse::<Marker>().is_err());
    }
}
