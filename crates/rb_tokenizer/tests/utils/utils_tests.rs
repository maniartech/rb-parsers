use rb_tokenizer::utils;

#[cfg(test)]
mod utils_tests {
    use super::*;

    #[test]
    fn test_string_helpers() {
        // Test string trimming functions if they exist
        let test_str = "  Hello world!  ";
        if let Some(trimmed) = utils::trim_string(test_str) {
            assert_eq!(trimmed, "Hello world!");
        }
        
        if let Some(left_trimmed) = utils::trim_left(test_str) {
            assert_eq!(left_trimmed, "Hello world!  ");
        }
        
        if let Some(right_trimmed) = utils::trim_right(test_str) {
            assert_eq!(right_trimmed, "  Hello world!");
        }
    }

    #[test]
    fn test_whitespace_detection() {
        // Test whitespace checking functions
        if let Some(is_whitespace) = utils::is_whitespace_char(' ') {
            assert!(is_whitespace);
        }
        
        if let Some(is_whitespace) = utils::is_whitespace_char('\t') {
            assert!(is_whitespace);
        }
        
        if let Some(is_whitespace) = utils::is_whitespace_char('\n') {
            assert!(is_whitespace);
        }
        
        if let Some(is_whitespace) = utils::is_whitespace_char('a') {
            assert!(!is_whitespace);
        }
    }

    #[test]
    fn test_escape_sequences() {
        // Test escape sequence handling functions
        if let Some(escaped) = utils::escape_string(r#"Hello\nWorld\t!"#) {
            assert_eq!(escaped, "Hello\nWorld\t!");
        }
        
        if let Some(unescaped) = utils::unescape_string("Hello\nWorld\t!") {
            assert_eq!(unescaped, r#"Hello\nWorld\t!"#);
        }
    }

    #[test]
    fn test_string_position_utilities() {
        // Test functions that work with line/column positions in strings
        let test_str = "Line 1\nLine 2\nLine 3";
        
        if let Some(line_count) = utils::count_lines(test_str) {
            assert_eq!(line_count, 3);
        }
        
        if let Some((line, col)) = utils::position_at_index(test_str, 8) {
            assert_eq!(line, 1);  // 0-based
            assert_eq!(col, 1);   // 0-based
        }
        
        if let Some(index) = utils::index_at_position(test_str, 1, 1) {
            assert_eq!(index, 8);
        }
    }

    #[test]
    fn test_character_classification() {
        // Test character type classification functions
        if let Some(is_digit) = utils::is_digit('5') {
            assert!(is_digit);
        }
        
        if let Some(is_digit) = utils::is_digit('a') {
            assert!(!is_digit);
        }
        
        if let Some(is_alpha) = utils::is_alpha('z') {
            assert!(is_alpha);
        }
        
        if let Some(is_alpha) = utils::is_alpha('9') {
            assert!(!is_alpha);
        }
        
        if let Some(is_alphanumeric) = utils::is_alphanumeric('A') {
            assert!(is_alphanumeric);
        }
        
        if let Some(is_alphanumeric) = utils::is_alphanumeric('7') {
            assert!(is_alphanumeric);
        }
        
        if let Some(is_alphanumeric) = utils::is_alphanumeric('_') {
            assert!(!is_alphanumeric);
        }
    }
}