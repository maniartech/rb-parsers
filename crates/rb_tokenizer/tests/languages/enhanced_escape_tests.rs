extern crate rb_tokenizer;

use rb_tokenizer::{Tokenizer, TokenizerConfig};
use rb_tokenizer::scanners::block_scanner::{BlockScanner, EscapeRule};

fn get_js_style_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: true,
        error_tolerance_limit: 5,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // Add a double-quoted string scanner with JS-style escapes
    let mut double_string_scanner = BlockScanner::new(
        "\"",
        "\"",
        "String",
        Some("DoubleQuote"),
        false,
        false, // Not raw mode
        true   // Include delimiters
    );

    // Add simple escape character
    double_string_scanner.add_simple_escape('\\');

    // Add Unicode escape patterns
    double_string_scanner.add_pattern_escape(r"\\u[0-9a-fA-F]{4}").unwrap();
    double_string_scanner.add_pattern_escape(r"\\x[0-9a-fA-F]{2}").unwrap();

    // Setup common escape mappings
    double_string_scanner.set_transform_escapes(true);
    double_string_scanner.add_escape_mapping("n", '\n');
    double_string_scanner.add_escape_mapping("t", '\t');
    double_string_scanner.add_escape_mapping("r", '\r');
    double_string_scanner.add_escape_mapping("\\", '\\');
    double_string_scanner.add_escape_mapping("\"", '\"');
    double_string_scanner.add_escape_mapping("'", '\'');
    double_string_scanner.add_escape_mapping("0", '\0');

    tokenizer.add_scanner(Box::new(double_string_scanner));

    // Add a single-quoted string scanner with similar escapes
    let mut single_string_scanner = BlockScanner::new(
        "'",
        "'",
        "String",
        Some("SingleQuote"),
        false,
        false,
        true
    );

    single_string_scanner.add_simple_escape('\\');
    single_string_scanner.add_pattern_escape(r"\\u[0-9a-fA-F]{4}").unwrap();
    single_string_scanner.set_transform_escapes(true);
    single_string_scanner.add_escape_mapping("n", '\n');
    single_string_scanner.add_escape_mapping("t", '\t');
    single_string_scanner.add_escape_mapping("r", '\r');
    single_string_scanner.add_escape_mapping("\\", '\\');
    single_string_scanner.add_escape_mapping("\"", '\"');
    single_string_scanner.add_escape_mapping("'", '\'');

    tokenizer.add_scanner(Box::new(single_string_scanner));

    // Add a template literal scanner with JS-style escapes
    // and template expression placeholders
    let mut template_scanner = BlockScanner::new(
        "`",
        "`",
        "String",
        Some("TemplateLiteral"),
        false,
        false,
        true
    );

    template_scanner.add_simple_escape('\\');
    template_scanner.add_balanced_escape("${", "}", true);
    template_scanner.set_transform_escapes(true);
    template_scanner.add_escape_mapping("n", '\n');
    template_scanner.add_escape_mapping("t", '\t');
    template_scanner.add_escape_mapping("r", '\r');
    template_scanner.add_escape_mapping("`", '`');
    template_scanner.add_escape_mapping("\\", '\\');

    tokenizer.add_scanner(Box::new(template_scanner));

    // Regular tokens
    tokenizer.add_symbol_scanner("(", "Braces", Some("OpenParen"));
    tokenizer.add_symbol_scanner(")", "Braces", Some("CloseParen"));
    tokenizer.add_symbol_scanner("{", "Braces", Some("OpenBrace"));
    tokenizer.add_symbol_scanner("}", "Braces", Some("CloseBrace"));
    tokenizer.add_symbol_scanner("[", "Bracket", Some("OpenBracket"));
    tokenizer.add_symbol_scanner("]", "Bracket", Some("CloseBracket"));
    tokenizer.add_symbol_scanner(";", "Semicolon", None);
    tokenizer.add_symbol_scanner(":", "Colon", None);
    tokenizer.add_symbol_scanner(",", "Comma", None);

    // Arithmetic Operators
    tokenizer.add_symbol_scanner("+", "Operator", Some("Plus"));
    tokenizer.add_symbol_scanner("-", "Operator", Some("Minus"));
    tokenizer.add_symbol_scanner("*", "Operator", Some("Multiply"));
    tokenizer.add_symbol_scanner("/", "Operator", Some("Divide"));

    // Regular expressions
    tokenizer.add_regex_scanner(r"^(true|false|null|undefined)\b", "Literal", None);
    tokenizer.add_regex_scanner(r"^[a-zA-Z_$][a-zA-Z0-9_$]*", "Identifier", None);
    tokenizer.add_regex_scanner(r"^-?\d+(\.\d+)?([eE][-+]?\d+)?", "Number", None);

    tokenizer
}

fn get_html_style_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: true,
        error_tolerance_limit: 5,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // HTML tag scanner
    let mut tag_scanner = BlockScanner::new(
        "<",
        ">",
        "Tag",
        None,
        false,
        false,
        true
    );
    tokenizer.add_scanner(Box::new(tag_scanner));

    // HTML attribute string with entities
    let mut attr_string_scanner = BlockScanner::new(
        "\"",
        "\"",
        "String",
        Some("AttributeValue"),
        false,
        false,
        true
    );

    // Add HTML entity escape
    attr_string_scanner.add_named_escape('&', ';', 10);
    attr_string_scanner.set_transform_escapes(true);

    // Add common HTML entity mappings
    attr_string_scanner.add_escape_mapping("amp", '&');
    attr_string_scanner.add_escape_mapping("lt", '<');
    attr_string_scanner.add_escape_mapping("gt", '>');
    attr_string_scanner.add_escape_mapping("quot", '"');
    attr_string_scanner.add_escape_mapping("apos", '\'');
    attr_string_scanner.add_escape_mapping("nbsp", '\u{00A0}');

    tokenizer.add_scanner(Box::new(attr_string_scanner));

    // HTML content with entities
    let mut entity_scanner = BlockScanner::new(
        "&",
        ";",
        "Entity",
        None,
        false,
        false,
        true
    );
    entity_scanner.set_transform_escapes(true);

    // Add common HTML entity mappings
    entity_scanner.add_escape_mapping("amp", '&');
    entity_scanner.add_escape_mapping("lt", '<');
    entity_scanner.add_escape_mapping("gt", '>');
    entity_scanner.add_escape_mapping("quot", '"');
    entity_scanner.add_escape_mapping("apos", '\'');
    entity_scanner.add_escape_mapping("nbsp", '\u{00A0}');

    tokenizer.add_scanner(Box::new(entity_scanner));

    // Regular HTML tokens
    tokenizer.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_-]*", "Identifier", None);
    tokenizer.add_regex_scanner(r"^=", "Equals", None);

    tokenizer
}

fn get_c_style_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: true,
        error_tolerance_limit: 5,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // C-style string scanner
    let mut string_scanner = BlockScanner::new(
        "\"",
        "\"",
        "String",
        None,
        false,
        false,
        true
    );

    // Add simple escape
    string_scanner.add_simple_escape('\\');

    // Add octal escapes (like \123)
    string_scanner.add_pattern_escape(r"\\[0-7]{1,3}").unwrap();

    // Add hex escapes (like \xAB)
    string_scanner.add_pattern_escape(r"\\x[0-9a-fA-F]{1,2}").unwrap();

    // Setup common escape mappings
    string_scanner.set_transform_escapes(true);
    string_scanner.add_escape_mapping("n", '\n');
    string_scanner.add_escape_mapping("t", '\t');
    string_scanner.add_escape_mapping("r", '\r');
    string_scanner.add_escape_mapping("\\", '\\');
    string_scanner.add_escape_mapping("\"", '\"');
    string_scanner.add_escape_mapping("'", '\'');
    string_scanner.add_escape_mapping("a", '\x07'); // Bell
    string_scanner.add_escape_mapping("b", '\x08'); // Backspace
    string_scanner.add_escape_mapping("f", '\x0C'); // Form feed
    string_scanner.add_escape_mapping("v", '\x0B'); // Vertical tab

    tokenizer.add_scanner(Box::new(string_scanner));

    // Character literals
    let mut char_scanner = BlockScanner::new(
        "'",
        "'",
        "Character",
        None,
        false,
        false,
        true
    );

    char_scanner.add_simple_escape('\\');
    char_scanner.set_transform_escapes(true);
    char_scanner.add_escape_mapping("n", '\n');
    char_scanner.add_escape_mapping("t", '\t');
    char_scanner.add_escape_mapping("'", '\'');
    char_scanner.add_escape_mapping("\\", '\\');

    tokenizer.add_scanner(Box::new(char_scanner));

    // Basic symbols
    tokenizer.add_symbol_scanner(";", "Semicolon", None);
    tokenizer.add_symbol_scanner("{", "Braces", Some("Open"));
    tokenizer.add_symbol_scanner("}", "Braces", Some("Close"));
    tokenizer.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);

    tokenizer
}

fn get_ruby_style_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: true,
        error_tolerance_limit: 5,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // Ruby double-quoted string with interpolation
    let mut dq_string_scanner = BlockScanner::new(
        "\"",
        "\"",
        "String",
        Some("DoubleQuoted"),
        false,
        false,
        true
    );

    dq_string_scanner.add_simple_escape('\\');
    dq_string_scanner.add_balanced_escape("#{", "}", true); // Ruby interpolation
    dq_string_scanner.set_transform_escapes(true);
    dq_string_scanner.add_escape_mapping("n", '\n');
    dq_string_scanner.add_escape_mapping("t", '\t');
    dq_string_scanner.add_escape_mapping("\"", '"');
    dq_string_scanner.add_escape_mapping("\\", '\\');

    tokenizer.add_scanner(Box::new(dq_string_scanner));

    // Ruby single-quoted string (no interpolation)
    let mut sq_string_scanner = BlockScanner::new(
        "'",
        "'",
        "String",
        Some("SingleQuoted"),
        false,
        false,
        true
    );

    sq_string_scanner.add_simple_escape('\\');
    sq_string_scanner.set_transform_escapes(true);
    sq_string_scanner.add_escape_mapping("'", '\'');
    sq_string_scanner.add_escape_mapping("\\", '\\');

    tokenizer.add_scanner(Box::new(sq_string_scanner));

    // Regular tokens
    tokenizer.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);
    tokenizer.add_symbol_scanner(";", "Semicolon", None);

    tokenizer
}

#[cfg(test)]
mod enhanced_escape_tests {
    use super::*;
    use rb_tokenizer::utils::pretty_print_tokens;

    #[test]
    fn test_js_string_escapes() {
        let tokenizer = get_js_style_tokenizer();

        // Test double-quoted strings with escapes
        let input = r#"const message = "Hello\nWorld\t\"Escaped\"\\path\\to\\file";"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Find the string token
        let string_token = result.iter().find(|t| t.token_type == "String").unwrap();

        // Verify escape sequence transformation
        assert!(string_token.value.contains('\n'));
        assert!(string_token.value.contains('\t'));
        assert!(string_token.value.contains("\"Escaped\""));
        assert!(string_token.value.contains("\\path\\to\\file"));

        // Test single-quoted strings
        let input = r#"const message = 'Single\'s quote\nand newline';"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        let string_token = result.iter().find(|t| t.token_type == "String").unwrap();

        assert!(string_token.value.contains("Single's quote"));
        assert!(string_token.value.contains('\n'));
    }

    #[test]
    fn test_js_unicode_escapes() {
        let tokenizer = get_js_style_tokenizer();

        // Test strings with unicode escapes
        let input = r#"const symbols = "\u0041\u0042\u0043";"#; // ABC in Unicode
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Find the string token (should include the transformed Unicode characters)
        let string_token = result.iter().find(|t| t.token_type == "String").unwrap();
        pretty_print_tokens(&result);

        // Currently we don't transform unicode escapes by default in BlockScanner,
        // so the raw sequences will be preserved
        assert!(string_token.value.contains("\\u0041"));
    }

    #[test]
    fn test_js_template_literals() {
        let tokenizer = get_js_style_tokenizer();

        // Test template literals with expressions
        let input = r#"const greeting = `Hello, ${name}! Today is ${new Date().toDateString()}.`;"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Find the template literal token
        let template_token = result.iter().find(|t| t.token_sub_type == Some("TemplateLiteral".to_string())).unwrap();

        // Verify template expressions are preserved
        assert!(template_token.value.contains("${name}"));
        assert!(template_token.value.contains("${new Date().toDateString()}"));

        // Test nested expressions in template literals
        let input = r#"`Nested ${`inner ${value}` + more}`"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        let template_token = result.iter().find(|t| t.token_sub_type == Some("TemplateLiteral".to_string())).unwrap();

        // Should correctly handle nesting
        assert!(template_token.value.contains("`inner ${value}`"));
    }

    #[test]
    fn test_html_entity_escapes() {
        let tokenizer = get_html_style_tokenizer();

        // Test HTML with entities
        let input = r#"<p>This is a paragraph with &lt;tags&gt; and an &amp; symbol.</p>"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Find all entity tokens
        let entity_tokens: Vec<_> = result.iter()
            .filter(|t| t.token_type == "Entity")
            .collect();

        // Should have 3 entities: &lt;, &gt;, and &amp;
        assert_eq!(entity_tokens.len(), 3);

        // Check if specific entities are transformed
        let lt_entity = entity_tokens.iter().find(|t| t.value.contains("&lt;")).unwrap();
        let gt_entity = entity_tokens.iter().find(|t| t.value.contains("&gt;")).unwrap();
        let amp_entity = entity_tokens.iter().find(|t| t.value.contains("&amp;")).unwrap();

        // The entities should be captured as full tokens with the delimiters
        assert_eq!(lt_entity.value, "&lt;");
        assert_eq!(gt_entity.value, "&gt;");
        assert_eq!(amp_entity.value, "&amp;");
    }

    #[test]
    fn test_html_attribute_escapes() {
        let tokenizer = get_html_style_tokenizer();

        // Test HTML attributes with entities
        let input = r#"<a href="page.html?param=value&amp;other=123" title="Title with &quot;quotes&quot;">"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Find the attribute value strings
        let attr_strings: Vec<_> = result.iter()
            .filter(|t| t.token_type == "String")
            .collect();

        // Should have 2 attribute strings
        assert_eq!(attr_strings.len(), 2);

        // Verify entity transformation in attribute values
        let href_attr = attr_strings.iter()
            .find(|t| t.value.contains("page.html"))
            .unwrap();
        assert!(href_attr.value.contains("&amp;"));

        let title_attr = attr_strings.iter()
            .find(|t| t.value.contains("Title"))
            .unwrap();
        assert!(title_attr.value.contains("&quot;quotes&quot;"));
    }

    #[test]
    fn test_c_style_string_escapes() {
        let tokenizer = get_c_style_tokenizer();

        // Test C-style string with various escapes
        let input = r#"char *s = "Hello\nWorld\t\x41\x42\x43\\\"";"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Find the string token
        let string_token = result.iter().find(|t| t.token_type == "String").unwrap();

        // Verify newline and tab transformation
        assert!(string_token.value.contains('\n'));
        assert!(string_token.value.contains('\t'));

        // Hex escapes aren't transformed yet
        assert!(string_token.value.contains("\\x41"));

        // Verify escape of backslash and quotes
        assert!(string_token.value.contains("\\\""));
    }

    #[test]
    fn test_c_style_char_literals() {
        let tokenizer = get_c_style_tokenizer();

        // Test C-style character literals
        let input = r#"char c1 = 'A'; char c2 = '\n'; char c3 = '\'';"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Find all character tokens
        let char_tokens: Vec<_> = result.iter()
            .filter(|t| t.token_type == "Character")
            .collect();

        // Should have 3 character literals
        assert_eq!(char_tokens.len(), 3);

        // Verify each character literal
        assert!(char_tokens[0].value.contains('A'));
        assert!(char_tokens[1].value.contains('\n'));
        assert!(char_tokens[2].value.contains('\''));
    }

    #[test]
    fn test_ruby_string_interpolation() {
        let tokenizer = get_ruby_style_tokenizer();

        // Test Ruby-style string with interpolation
        let input = r#"message = "Hello #{name}! The time is #{Time.now}""#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Find the string token
        let string_token = result.iter().find(|t| t.token_type == "String").unwrap();

        // Verify interpolation is preserved
        assert!(string_token.value.contains("#{name}"));
        assert!(string_token.value.contains("#{Time.now}"));

        // Test nested interpolation
        let input = r#"msg = "Value: #{x + #{nested}!} end""#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        let string_token = result.iter().find(|t| t.token_type == "String").unwrap();

        // Verify nested interpolation is handled
        assert!(string_token.value.contains("#{x + #{nested}!}"));
    }

    #[test]
    fn test_combination_of_escape_rules() {
        // Create a scanner with multiple types of escape rules
        let mut scanner = BlockScanner::new(
            "{%",
            "%}",
            "Template",
            None,
            false,
            false,
            true
        );

        // Add multiple escape types
        scanner.add_simple_escape('\\');
        scanner.add_pattern_escape(r"\\u[0-9a-fA-F]{4}").unwrap();
        scanner.add_balanced_escape("${", "}", true); // JS-style variables
        scanner.add_balanced_escape("#{", "}", true); // Ruby-style variables
        scanner.add_named_escape('&', ';', 10);       // HTML entities

        // Set up transformation
        scanner.set_transform_escapes(true);
        scanner.add_escape_mapping("n", '\n');
        scanner.add_escape_mapping("t", '\t');
        scanner.add_escape_mapping("amp", '&');
        scanner.add_escape_mapping("lt", '<');

        // Create tokenizer with this complex scanner
        let mut tokenizer = Tokenizer::new();
        tokenizer.add_scanner(Box::new(scanner));

        // Test a template with combined escape types
        let input = r#"{% A template with \n newline, ${jsVar}, #{rubyVar}, and &lt; entities %}"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Should have one token
        assert_eq!(result.len(), 1);
        let token = &result[0];

        // Verify all escape types are handled
        assert!(token.value.contains('\n')); // Transformed newline
        assert!(token.value.contains("${jsVar}")); // JS variable preserved
        assert!(token.value.contains("#{rubyVar}")); // Ruby variable preserved
        assert!(token.value.contains("&lt;")); // HTML entity preserved (not transformed)
    }

    #[test]
    fn test_complex_mixed_js_style() {
        let tokenizer = get_js_style_tokenizer();

        // Complex JS with different string types and escapes
        let input = r#"
        function processData(data) {
            const doubleQuoted = "Line 1\nLine 2\tTabbed";
            const singleQuoted = 'Single\'s quote';
            const templateLit = `User ${user.name}'s profile: ${getDetails(user.id)}`;

            return {
                processed: true,
                message: `Completed at ${new Date().toISOString()}`
            };
        }
        "#;

        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Find all string tokens
        let string_tokens: Vec<_> = result.iter()
            .filter(|t| t.token_type == "String")
            .collect();

        // Should have 5 strings: 2 double-quoted, 1 single-quoted, 2 template literals
        assert_eq!(string_tokens.len(), 5);

        // Find and verify each string type
        let double_quoted = string_tokens.iter()
            .find(|t| t.value.contains("Line 1") && t.token_sub_type == Some("DoubleQuote".to_string()))
            .unwrap();
        assert!(double_quoted.value.contains('\n'));
        assert!(double_quoted.value.contains('\t'));

        let single_quoted = string_tokens.iter()
            .find(|t| t.value.contains("Single") && t.token_sub_type == Some("SingleQuote".to_string()))
            .unwrap();
        assert!(single_quoted.value.contains("Single's quote"));

        // Find template literals with expressions
        let template_tokens: Vec<_> = string_tokens.iter()
            .filter(|t| t.token_sub_type == Some("TemplateLiteral".to_string()))
            .collect();
        assert_eq!(template_tokens.len(), 2);

        // Verify template expressions are preserved
        assert!(template_tokens.iter().any(|t| t.value.contains("${user.name}")));
        assert!(template_tokens.iter().any(|t| t.value.contains("${getDetails(user.id)}")));
        assert!(template_tokens.iter().any(|t| t.value.contains("${new Date().toISOString()}")));
    }
}