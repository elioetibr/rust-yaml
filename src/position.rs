//! Position tracking for YAML parsing

use std::fmt;

/// Represents a position in the YAML input stream
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Byte index in the input (0-based)
    pub index: usize,
}

impl Position {
    /// Create a new position
    pub const fn new() -> Self {
        Self {
            line: 1,
            column: 1,
            index: 0,
        }
    }

    /// Create a position with specific values
    pub const fn at(line: usize, column: usize, index: usize) -> Self {
        Self {
            line,
            column,
            index,
        }
    }

    /// Create a position at the start of input
    pub const fn start() -> Self {
        Self::new()
    }

    /// Advance position by one character
    pub const fn advance(self, ch: char) -> Self {
        if ch == '\n' {
            Self::at(self.line + 1, 1, self.index + ch.len_utf8())
        } else {
            Self::at(self.line, self.column + 1, self.index + ch.len_utf8())
        }
    }

    /// Advance position by a string
    pub fn advance_str(mut self, s: &str) -> Self {
        for ch in s.chars() {
            self = self.advance(ch);
        }
        self
    }

    /// Advance position by multiple characters
    pub const fn advance_by(mut self, count: usize, is_newline: bool) -> Self {
        if is_newline {
            self.line += count;
            self.column = 1;
        } else {
            self.column += count;
        }
        self.index += count;
        self
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::start()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Position", 3)?;
        state.serialize_field("line", &self.line)?;
        state.serialize_field("column", &self.column)?;
        state.serialize_field("index", &self.index)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Line,
            Column,
            Index,
        }

        struct PositionVisitor;

        impl<'de> Visitor<'de> for PositionVisitor {
            type Value = Position;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Position")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Position, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut line = None;
                let mut column = None;
                let mut index = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Line => {
                            if line.is_some() {
                                return Err(de::Error::duplicate_field("line"));
                            }
                            line = Some(map.next_value()?);
                        }
                        Field::Column => {
                            if column.is_some() {
                                return Err(de::Error::duplicate_field("column"));
                            }
                            column = Some(map.next_value()?);
                        }
                        Field::Index => {
                            if index.is_some() {
                                return Err(de::Error::duplicate_field("index"));
                            }
                            index = Some(map.next_value()?);
                        }
                    }
                }
                let line = line.ok_or_else(|| de::Error::missing_field("line"))?;
                let column = column.ok_or_else(|| de::Error::missing_field("column"))?;
                let index = index.ok_or_else(|| de::Error::missing_field("index"))?;
                Ok(Position::at(line, column, index))
            }
        }

        const FIELDS: &[&str] = &["line", "column", "index"];
        deserializer.deserialize_struct("Position", FIELDS, PositionVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::at(5, 10, 42);
        assert_eq!(pos.line, 5);
        assert_eq!(pos.column, 10);
        assert_eq!(pos.index, 42);
    }

    #[test]
    fn test_position_start() {
        let pos = Position::start();
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 1);
        assert_eq!(pos.index, 0);
    }

    #[test]
    fn test_position_advance() {
        let pos = Position::start();

        let pos1 = pos.advance('a');
        assert_eq!(pos1.line, 1);
        assert_eq!(pos1.column, 2);
        assert_eq!(pos1.index, 1);

        let pos2 = pos1.advance('\n');
        assert_eq!(pos2.line, 2);
        assert_eq!(pos2.column, 1);
        assert_eq!(pos2.index, 2);
    }

    #[test]
    fn test_position_advance_str() {
        let pos = Position::start();
        let pos1 = pos.advance_str("hello\nworld");
        assert_eq!(pos1.line, 2);
        assert_eq!(pos1.column, 6);
        assert_eq!(pos1.index, 11);
    }

    #[test]
    fn test_position_display() {
        let pos = Position::at(42, 13, 1000);
        assert_eq!(format!("{}", pos), "line 42, column 13");
    }

    #[test]
    fn test_position_ordering() {
        let pos1 = Position::at(1, 5, 10);
        let pos2 = Position::at(2, 3, 20);
        let pos3 = Position::at(1, 6, 11);

        assert!(pos1 < pos2);
        assert!(pos1 < pos3);
        assert!(pos3 < pos2);
    }

    // Temporarily commented out due to missing serde_json dependency
    // #[cfg(feature = "serde")]
    // #[test]
    // fn test_position_serde() {
    //     let pos = Position::at(10, 20, 100);
    //     let json = serde_json::to_string(&pos).unwrap();
    //     let deserialized: Position = serde_json::from_str(&json).unwrap();
    //     assert_eq!(pos, deserialized);
    // }
}
