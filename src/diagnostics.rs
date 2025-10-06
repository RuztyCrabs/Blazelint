use crate::errors::Diagnostic;
use std::io::Write;

/// Writes a diagnostic error message to the given writer.
///
/// # Arguments
/// * `writer` - Where to write the error (e.g., stdout, file).
/// * `file_path` - Path of the source file with the error.
/// * `source_code` - Full text of the source code.
/// * `diagnostic` - The error information (message, span, notes).
///
/// # Returns
/// Returns `Ok(())` if writing succeeds, or an `Err` if writing fails.
pub fn report_error(
  writer: &mut impl Write,
  file_path: &str,
  source_code: &str,
  diagnostic: &Diagnostic,
) -> std::io::Result<()> {
  let (line_number, column_number) = get_line_and_column(source_code, diagnostic.span.start);
  let line_str = source_code.lines().nth(line_number - 1).unwrap_or("");

  writeln!(writer, "error: {}", diagnostic.message)?;
  writeln!(
    writer,
    "  --> {}:{}:{}",
    file_path, line_number, column_number
  )?;
  writeln!(writer, "   |")?;
  writeln!(writer, "{: >2} | {}", line_number, line_str)?;
  writeln!(
    writer,
    "   | {}{}",
    " ".repeat(column_number - 1),
    "^".repeat(diagnostic.span.end - diagnostic.span.start)
  )?;

  for note in &diagnostic.notes {
    writeln!(writer, "   = note: {}", note)?;
  }
  Ok(())
}

/// Converts a byte index in the source code to a line and column number.
///
/// # Arguments
/// * `source_code` - The full source code string.
/// * `position` - Byte position to convert.
///
/// # Returns
/// A tuple `(line, column)` representing the 1-based line and column numbers.
fn get_line_and_column(source_code: &str, position: usize) -> (usize, usize) {
  let mut line = 1;
  let mut column = 1;
  for (i, char) in source_code.char_indices() {
    if i == position {
      break;
    }
    if char == '\n' {
      line += 1;
      column = 1;
    } else {
      column += 1;
    }
  }
  (line, column)
}
