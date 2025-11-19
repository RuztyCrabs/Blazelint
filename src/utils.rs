use crate::errors::Position;

/// A global line tracking system for efficient position mapping.
/// 
/// LineTracker pre-computes line start positions for O(1) byte-to-position lookups.
/// This is more efficient than the previous O(n) approach when dealing with many diagnostics.
#[derive(Debug, Clone)]
pub struct LineTracker {
    /// Byte offsets for the start of each line, plus a sentinel at EOF
    line_starts: Vec<usize>,
    /// Reference to the original source for line text extraction
    source: String,
}

impl LineTracker {
    /// Creates a new LineTracker by pre-computing line start positions.
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0]; // First line starts at byte 0
        
        for (byte_offset, c) in source.char_indices() {
            if c == '\n' {
                line_starts.push(byte_offset + 1);
            }
        }
        
        // Add sentinel at EOF for safe upper bound lookups
        line_starts.push(source.len());
        
        Self {
            line_starts,
            source: source.to_string(),
        }
    }
    
    /// Converts a byte offset to line and column position (1-based).
    /// 
    /// Uses binary search for O(log n) lookup performance.
    pub fn byte_to_line_col(&self, offset: usize) -> Position {
        // Handle edge case: offset at or beyond EOF
        if offset >= self.source.len() {
            if let Some(&last_line_start) = self.line_starts.get(self.line_starts.len().saturating_sub(2)) {
                let line = self.line_starts.len().saturating_sub(1);
                let column = (self.source.len() - last_line_start) + 1;
                return Position::new(line, column);
            }
            return Position::new(1, 1);
        }
        
        // Binary search to find the line containing this offset
        match self.line_starts.binary_search(&offset) {
            Ok(line_index) => {
                // Exact match - we're at the start of a line
                Position::new(line_index + 1, 1)
            }
            Err(line_index) => {
                // Insert position tells us which line contains this offset
                let line = line_index; // 1-based line number
                let line_start = self.line_starts.get(line_index.saturating_sub(1)).copied().unwrap_or(0);
                let column = offset - line_start + 1;
                Position::new(line, column)
            }
        }
    }
    
    /// Extracts the text content of a specific line (1-based line number).
    /// 
    /// Returns the line text without trailing newlines.
    pub fn line_text(&self, line_number: usize) -> &str {
        if line_number == 0 || line_number > self.line_starts.len().saturating_sub(1) {
            return "";
        }
        
        let line_index = line_number - 1;
        let start = self.line_starts.get(line_index).copied().unwrap_or(0);
        let end = self.line_starts.get(line_index + 1).copied().unwrap_or(self.source.len());
        
        let line = &self.source[start..end];
        // Trim trailing newlines
        line.trim_end_matches(&['\n', '\r'])
    }
    
    /// Returns the source text for debugging purposes.
    pub fn source(&self) -> &str {
        &self.source
    }
    
    /// Returns the number of lines in the source.
    pub fn line_count(&self) -> usize {
        self.line_starts.len().saturating_sub(1)
    }
}

/// A utility function to get the line and column number from a byte offset.
/// 
/// **DEPRECATED**: Use LineTracker::byte_to_line_col for better performance.
/// This function is kept for backward compatibility but has O(n) performance.
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
    for (byte_offset, c) in source.char_indices() {
        if byte_offset == offset {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_tracker_basic() {
        let source = "line 1\nline 2\nline 3";
        let tracker = LineTracker::new(source);
        
        // Test line start positions
        assert_eq!(tracker.byte_to_line_col(0), Position::new(1, 1)); // Start of line 1
        assert_eq!(tracker.byte_to_line_col(7), Position::new(2, 1)); // Start of line 2  
        assert_eq!(tracker.byte_to_line_col(14), Position::new(3, 1)); // Start of line 3
        
        // Test positions within lines
        assert_eq!(tracker.byte_to_line_col(3), Position::new(1, 4)); // "e" in "line"
        assert_eq!(tracker.byte_to_line_col(10), Position::new(2, 4)); // "e" in second "line"
    }
    
    #[test]
    fn test_line_tracker_line_text() {
        let source = "first line\nsecond line\nthird line";
        let tracker = LineTracker::new(source);
        
        assert_eq!(tracker.line_text(1), "first line");
        assert_eq!(tracker.line_text(2), "second line");
        assert_eq!(tracker.line_text(3), "third line");
        assert_eq!(tracker.line_text(4), ""); // Out of bounds
    }
    
    #[test]
    fn test_line_tracker_empty_source() {
        let tracker = LineTracker::new("");
        assert_eq!(tracker.byte_to_line_col(0), Position::new(1, 1));
        assert_eq!(tracker.line_count(), 1);
    }
    
    #[test]
    fn test_compatibility_with_old_function() {
        let source = "line 1\nline 2\nline 3";
        let tracker = LineTracker::new(source);
        
        // Test that both methods give the same results
        for offset in 0..source.len() {
            let old_pos = get_line_and_column(offset, source);
            let new_pos = tracker.byte_to_line_col(offset);
            assert_eq!(old_pos, new_pos, "Mismatch at offset {}", offset);
        }
    }
}
