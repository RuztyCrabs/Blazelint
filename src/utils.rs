use crate::errors::Position;

/// A utility function to get the line and column number from a byte offset.
///
/// # Arguments
///
/// * `offset` - The byte offset to get the line and column number for.
/// * `source` - The source code to search within.
///
/// # Returns
///
/// A `Position` struct with the calculated line and column numbers.
pub fn get_line_and_column(offset: usize, source: &str) -> Position {
    let mut line = 1;
    let mut column = 1;
    for (i, c) in source.chars().enumerate() {
        if i == offset {
            return Position::new(line, column);
        }
        if c == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    Position::new(line, column)
}
