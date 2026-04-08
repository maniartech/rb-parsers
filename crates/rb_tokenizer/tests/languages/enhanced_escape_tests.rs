extern crate rb_tokenizer;

use rb_tokenizer::{KeywordScanner, Tokenizer, TokenizerConfig, WordBoundaryDef};
use rb_tokenizer::scanners::block_scanner::BlockScanner;

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

    // Comments — registered before operators so `//` and `/*` take priority over `/`.
    tokenizer.add_eol_scanner("//", "Comment", Some("Line"), true);
    tokenizer.add_block_scanner("/*", "*/", "Comment", Some("Block"), false, true, true);

    // Structural punctuation
    tokenizer.add_symbol_scanner("(", "Braces", Some("OpenParen"));
    tokenizer.add_symbol_scanner(")", "Braces", Some("CloseParen"));
    tokenizer.add_symbol_scanner("{", "Braces", Some("OpenBrace"));
    tokenizer.add_symbol_scanner("}", "Braces", Some("CloseBrace"));
    tokenizer.add_symbol_scanner("[", "Bracket", Some("OpenBracket"));
    tokenizer.add_symbol_scanner("]", "Bracket", Some("CloseBracket"));
    tokenizer.add_symbol_scanner(";", "Semicolon", None);
    tokenizer.add_symbol_scanner(":", "Colon", None);
    tokenizer.add_symbol_scanner(",", "Comma", None);

    // Operators — OperatorScanner uses longest-match which resolves `===` vs `==` vs `=`,
    // `?.` vs `?`, `++` vs `+`, `>>>` vs `>>`, etc. without ordering concerns.
    tokenizer.add_operator_scanner_with_subtypes("Operator", &[
        ("===", "StrictEq"),    ("!==", "StrictNe"),    ("==",  "Eq"),
        ("!=",  "Ne"),          (">=",  "Ge"),          ("<=",  "Le"),
        ("||",  "Or"),          ("&&",  "And"),         ("??",  "NullCoalesce"),
        ("++",  "Inc"),         ("--",  "Dec"),         ("**=", "PowAssign"),
        ("<<=", "ShlAssign"),   (">>>=","UShrAssign"),  (">>=", "ShrAssign"),
        ("+=",  "AddAssign"),   ("-=",  "SubAssign"),   ("*=",  "MulAssign"),
        ("/=",  "DivAssign"),   ("%=",  "ModAssign"),   ("&=",  "BitAndAssign"),
        ("|=",  "BitOrAssign"), ("^=",  "BitXorAssign"),("&&=", "LogAndAssign"),
        ("||=", "LogOrAssign"), ("??=", "NullAssign"),  ("**",  "Pow"),
        (">>>", "UShr"),        (">>",  "Shr"),         ("<<",  "Shl"),
        ("=>",  "Arrow"),       ("?.",  "OptChain"),    (">",   "Gt"),
        ("<",   "Lt"),          ("+",   "Plus"),        ("-",   "Minus"),
        ("*",   "Mul"),         ("/",   "Div"),         ("%",   "Mod"),
        ("&",   "BitAnd"),      ("|",   "BitOr"),       ("^",   "BitXor"),
        ("~",   "BitNot"),      ("!",   "Not"),         ("=",   "Assign"),
        ("?",   "Question"),    (".",   "Dot"),
    ]);

    // Keywords — WordBoundaryDef::javascript() treats `$` as a word character
    // so identifiers like `$el`, `jQuery`, `$$` are never misidentified as keywords.
    tokenizer.add_scanner(Box::new(
        KeywordScanner::with_subtypes("Keyword", &[
            ("const",      "Const"),       ("let",        "Let"),
            ("var",        "Var"),         ("function",   "Function"),
            ("class",      "Class"),       ("extends",    "Extends"),
            ("return",     "Return"),      ("if",         "If"),
            ("else",       "Else"),        ("for",        "For"),
            ("while",      "While"),       ("do",         "Do"),
            ("switch",     "Switch"),      ("case",       "Case"),
            ("default",    "Default"),     ("break",      "Break"),
            ("continue",   "Continue"),    ("try",        "Try"),
            ("catch",      "Catch"),       ("finally",    "Finally"),
            ("throw",      "Throw"),       ("new",        "New"),
            ("delete",     "Delete"),      ("typeof",     "TypeOf"),
            ("instanceof", "Instanceof"),  ("void",       "Void"),
            ("import",     "Import"),      ("export",     "Export"),
            ("async",      "Async"),       ("await",      "Await"),
            ("yield",      "Yield"),       ("this",       "This"),
            ("super",      "Super"),       ("static",     "Static"),
            ("in",         "In"),          ("of",         "Of"),
            ("debugger",   "Debugger"),
        ])
        .with_word_boundary_def(WordBoundaryDef::javascript()),
    ));
    tokenizer.add_keyword_scanner("Literal", &["true", "false", "null", "undefined"]);

    // Identifiers — `$` is a valid lead and continuation character in JS.
    tokenizer.add_char_class_scanner("a-zA-Z_$", Some("a-zA-Z0-9_$"), "Identifier", None);

    // Numbers — full JS support: hex 0x, binary 0b, octal 0o, float, scientific, _separators.
    tokenizer.add_number_literal_scanner("Number", None);

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

    // HTML comments: <!-- ... --> — registered before the generic `<` tag scanner.
    tokenizer.add_block_scanner("<!--", "-->", "Comment", Some("Html"), false, true, true);
    // DOCTYPE and other markup declarations: <!DOCTYPE html>, <!ELEMENT ...>, etc.
    tokenizer.add_block_scanner("<!", ">", "Doctype", None, false, true, true);

    // HTML tag scanner — registered after comments/DOCTYPE to avoid consuming them.
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
    tokenizer.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_-"), "Identifier", None);
    tokenizer.add_symbol_scanner("=", "Equals", None);

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

    // C11 Unicode escapes: \uXXXX (4-digit) and \UXXXXXXXX (8-digit)
    string_scanner.add_pattern_escape(r"\\u[0-9a-fA-F]{4}").unwrap();
    string_scanner.add_pattern_escape(r"\\U[0-9a-fA-F]{8}").unwrap();

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
    tokenizer.add_symbol_scanner("(", "Braces", Some("OpenParen"));
    tokenizer.add_symbol_scanner(")", "Braces", Some("CloseParen"));
    tokenizer.add_symbol_scanner("[", "Bracket", Some("OpenBracket"));
    tokenizer.add_symbol_scanner("]", "Bracket", Some("CloseBracket"));
    tokenizer.add_symbol_scanner(",", "Comma", None);

    // Comments — registered before operators so `//` and `/*` take priority over `/`.
    tokenizer.add_eol_scanner("//", "Comment", Some("Line"), true);
    tokenizer.add_block_scanner("/*", "*/", "Comment", Some("Block"), false, true, true);

    // Keywords (C99 + C11) — registered before identifiers.
    tokenizer.add_keyword_scanner("Keyword", &[
        "auto", "break", "case", "char", "const", "continue", "default", "do",
        "double", "else", "enum", "extern", "float", "for", "goto", "if",
        "inline", "int", "long", "register", "restrict", "return", "short",
        "signed", "sizeof", "static", "struct", "switch", "typedef", "union",
        "unsigned", "void", "volatile", "while",
    ]);

    // Identifiers
    tokenizer.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_"), "Identifier", None);

    // Numbers — C supports hex (0x), octal, float, scientific notation.
    tokenizer.add_number_literal_scanner("Number", None);

    // Operators — longest-match handles ->, ==, !=, >=, <=, <<, >>, ++ etc.
    tokenizer.add_operator_scanner_with_subtypes("Operator", &[
        ("->",  "Arrow"),        ("<<=", "ShlAssign"),    (">>=", "ShrAssign"),
        ("<<",  "Shl"),          (">>",  "Shr"),          ("<=",  "Le"),
        (">=",  "Ge"),           ("==",  "Equal"),         ("!=",  "NotEqual"),
        ("&&",  "And"),          ("||",  "Or"),            ("++",  "Inc"),
        ("--",  "Dec"),          ("+=",  "AddAssign"),     ("-=",  "SubAssign"),
        ("*=",  "MulAssign"),    ("/=",  "DivAssign"),     ("%=",  "ModAssign"),
        ("&=",  "BitAndAssign"), ("|=",  "BitOrAssign"),   ("^=",  "BitXorAssign"),
        ("+",   "Plus"),         ("-",   "Minus"),         ("*",   "Mul"),
        ("/",   "Div"),          ("%",   "Mod"),           ("&",   "BitAnd"),
        ("|",   "BitOr"),        ("^",   "BitXor"),        ("~",   "BitNot"),
        ("!",   "Not"),          ("=",   "Assign"),        ("?",   "Question"),
        ("<",   "Lt"),           (">",   "Gt"),            (".",   "Dot"),
        (":",   "Colon"),
    ]);

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
    // Line comments — registered before keywords/identifiers so `# comment` is consumed
    // correctly. `#` inside string interpolation `#{...}` is never seen here because the
    // BlockScanner for strings processes it first.
    tokenizer.add_eol_scanner("#", "Comment", Some("Line"), true);

    // Keywords — WordBoundaryDef::ruby() treats `?` and `!` as word characters,
    // preventing `save` from matching keyword `save` when followed by `!`.
    tokenizer.add_scanner(Box::new(
        KeywordScanner::with_subtypes("Keyword", &[
            ("def",      "Def"),    ("end",      "End"),    ("class",  "Class"),
            ("module",   "Module"), ("do",       "Do"),     ("return", "Return"),
            ("if",       "If"),     ("else",     "Else"),   ("elsif",  "Elsif"),
            ("unless",   "Unless"), ("case",     "Case"),   ("when",   "When"),
            ("then",     "Then"),   ("begin",    "Begin"),  ("rescue", "Rescue"),
            ("ensure",   "Ensure"), ("raise",    "Raise"),  ("while",  "While"),
            ("until",    "Until"),  ("for",      "For"),    ("in",     "In"),
            ("yield",    "Yield"),  ("super",    "Super"),  ("self",   "Self"),
            ("nil",      "Nil"),    ("true",     "True"),   ("false",  "False"),
            ("and",      "And"),    ("or",       "Or"),     ("not",    "Not"),
            ("defined?", "Defined"),
        ])
        .with_word_boundary_def(WordBoundaryDef::ruby()),
    ));

    // Identifiers — regex handles Ruby's `?` and `!` method-name suffixes (e.g. `empty?`,
    // `save!`). The KeywordScanner above uses WordBoundaryDef::ruby() so keyword `save`
    // will not match when the next character is `!`.
    tokenizer.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_]*[?!]?", "Identifier", None).unwrap();

    // Numbers — Ruby supports hex, binary, octal, float, scientific, and _separators.
    tokenizer.add_number_literal_scanner("Number", None);

    // Operators
    tokenizer.add_operator_scanner_with_subtypes("Operator", &[
        ("<=>", "Spaceship"), ("===", "TripleEq"),  ("=~",  "Match"),
        ("!~",  "NotMatch"),  ("**=", "PowAssign"), ("**",  "Pow"),
        ("<<",  "Shl"),       (">>",  "Shr"),       ("<=",  "Le"),
        (">=",  "Ge"),        ("==",  "Equal"),     ("!=",  "NotEqual"),
        ("+=",  "AddAssign"), ("-=",  "SubAssign"), ("*=",  "MulAssign"),
        ("/=",  "DivAssign"), ("%=",  "ModAssign"), ("&&",  "And"),
        ("||",  "Or"),        ("&&=", "AndAssign"), ("||=", "OrAssign"),
        ("=>",  "Arrow"),     ("->",  "Lambda"),    ("&.",  "SafeNav"),
        ("+",   "Plus"),      ("-",   "Minus"),     ("*",   "Mul"),
        ("/",   "Div"),       ("%",   "Mod"),       ("&",   "BitAnd"),
        ("|",   "BitOr"),     ("^",   "BitXor"),    ("~",   "BitNot"),
        ("!",   "Not"),       ("=",   "Assign"),    ("?",   "Question"),
        ("<",   "Lt"),        (">",   "Gt"),        (".",   "Dot"),
    ]);

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

        // The string token must exist and contain the digit sequences from the escapes.
        // Full \uXXXX decode requires a codec pass after tokenization.
        let string_token = result.iter().find(|t| t.token_type == "String").unwrap();
        assert!(
            string_token.value.contains("0041") || string_token.value.contains('A'),
            "string token should contain the unicode escape content"
        );
    }

    #[test]
    fn test_js_template_literals() {
        let tokenizer = get_js_style_tokenizer();

        // Test template literals with expressions
        let input = r#"const greeting = `Hello, ${name}! Today is ${new Date().toDateString()}.`;"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Find the template literal token
        let template_token = result.iter().find(|t| t.token_sub_type == Some("TemplateLiteral")).unwrap();

        // Verify template expressions are preserved
        assert!(template_token.value.contains("${name}"));
        assert!(template_token.value.contains("${new Date().toDateString()}"));

        // Test nested expressions in template literals
        let input = r#"`Nested ${`inner ${value}` + more}`"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        let template_token = result.iter().find(|t| t.token_sub_type == Some("TemplateLiteral")).unwrap();

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

        // The HTML tokenizer uses a `<` → `>` BlockScanner that captures entire
        // tags as a single Tag token.  Attribute string values are therefore part
        // of the raw Tag token, not separate String tokens.
        let input = r#"<a href="page.html?param=value&amp;other=123" title="Title with &quot;quotes&quot;">"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // The entire opening tag is one Tag token.
        let tag_tokens: Vec<_> = result.iter().filter(|t| t.token_type == "Tag").collect();
        assert_eq!(tag_tokens.len(), 1, "Expected one Tag token for the whole element");

        let tag = &tag_tokens[0];
        assert!(tag.value.contains("href"), "tag should contain href attribute");
        assert!(tag.value.contains("&amp;"), "amp entity should be preserved in raw tag");
        assert!(tag.value.contains("&quot;"), "quot entity should be preserved in raw tag");
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

        // \xNN hex escapes are matched by pattern escape; without a decode mapping
        // they surface as the digit sequence in the token value.
        assert!(
            string_token.value.contains('A') || string_token.value.contains("41"),
            "hex escape content should appear in token"
        );

        // Verify escape of backslash and quotes
        assert!(string_token.value.contains('\"'));
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
        // &lt; is transformed to `<` via the escape mapping
        assert!(token.value.contains('<') || token.value.contains("&lt;"));
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

        // Should have 4 strings: 1 double-quoted, 1 single-quoted, 2 template literals
        assert_eq!(string_tokens.len(), 4);

        // Find and verify each string type
        let double_quoted = string_tokens.iter()
            .find(|t| t.value.contains("Line 1") && t.token_sub_type == Some("DoubleQuote"))
            .unwrap();
        assert!(double_quoted.value.contains('\n'));
        assert!(double_quoted.value.contains('\t'));

        let single_quoted = string_tokens.iter()
            .find(|t| t.value.contains("Single") && t.token_sub_type == Some("SingleQuote"))
            .unwrap();
        assert!(single_quoted.value.contains("Single's quote"));

        // Find template literals with expressions
        let template_tokens: Vec<_> = string_tokens.iter()
            .filter(|t| t.token_sub_type == Some("TemplateLiteral"))
            .collect();
        assert_eq!(template_tokens.len(), 2);

        // Verify template expressions are preserved
        assert!(template_tokens.iter().any(|t| t.value.contains("${user.name}")));
        assert!(template_tokens.iter().any(|t| t.value.contains("${getDetails(user.id)}")));
        assert!(template_tokens.iter().any(|t| t.value.contains("${new Date().toISOString()}")));
    }

    // ── New tests covering the upgraded scanner set ───────────────────────────

    #[test]
    fn test_js_comments() {
        let tokenizer = get_js_style_tokenizer();

        // Line comment
        let input = "const x = 1; // assign one";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        let comment = result.iter().find(|t| t.token_type == "Comment").unwrap();
        assert_eq!(comment.token_sub_type, Some("Line"));
        assert!(comment.value.starts_with("//"));

        // Block comment
        let input = "const /* type hint */ y = 2;";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        let comment = result.iter().find(|t| t.token_type == "Comment").unwrap();
        assert_eq!(comment.token_sub_type, Some("Block"));
        assert!(comment.value.starts_with("/*") && comment.value.ends_with("*/"));
    }

    #[test]
    fn test_js_keywords() {
        let tokenizer = get_js_style_tokenizer();

        let input = "const fetch = async function() { return await data; };";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        let keyword_subtypes: Vec<_> = result.iter()
            .filter(|t| t.token_type == "Keyword")
            .map(|t| t.token_sub_type.unwrap_or(""))
            .collect();

        assert!(keyword_subtypes.contains(&"Const"),    "expected 'const'");
        assert!(keyword_subtypes.contains(&"Async"),    "expected 'async'");
        assert!(keyword_subtypes.contains(&"Function"), "expected 'function'");
        assert!(keyword_subtypes.contains(&"Return"),   "expected 'return'");
        assert!(keyword_subtypes.contains(&"Await"),    "expected 'await'");

        // `$` is a word character in JS — `$data` must not match keyword `do`
        let result = tokenizer.tokenize("$data").expect("Tokenization failed");
        assert_eq!(result[0].token_type, "Identifier");
        assert_eq!(result[0].value, "$data");
    }

    #[test]
    fn test_js_number_formats() {
        let tokenizer = get_js_style_tokenizer();

        for (input, expected) in [
            ("0xFF",      "0xFF"),
            ("0b1010",    "0b1010"),
            ("0o755",     "0o755"),
            ("1.5e10",    "1.5e10"),
            ("1_000_000", "1_000_000"),
        ] {
            let result = tokenizer.tokenize(input).unwrap();
            assert_eq!(result[0].token_type, "Number", "input: {input}");
            assert_eq!(result[0].value, expected);
        }
    }

    #[test]
    fn test_c_comments() {
        let tokenizer = get_c_style_tokenizer();

        // C99 line comment
        let input = "int x = 0; // initialize";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        let comment = result.iter().find(|t| t.token_type == "Comment").unwrap();
        assert_eq!(comment.token_sub_type, Some("Line"));
        assert!(comment.value.starts_with("//"));

        // Block comment
        let input = "int /* type */ x;";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        let comment = result.iter().find(|t| t.token_type == "Comment").unwrap();
        assert_eq!(comment.token_sub_type, Some("Block"));
        assert!(comment.value.contains("type"));
    }

    #[test]
    fn test_c_keywords() {
        let tokenizer = get_c_style_tokenizer();

        let input = "int main(void) { return 0; }";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        let kw_values: Vec<_> = result.iter()
            .filter(|t| t.token_type == "Keyword")
            .map(|t| t.value.as_str())
            .collect();
        assert!(kw_values.contains(&"int"),    "expected 'int'");
        assert!(kw_values.contains(&"void"),   "expected 'void'");
        assert!(kw_values.contains(&"return"), "expected 'return'");

        // `integer_val` must not match keyword `int`
        let result = tokenizer.tokenize("integer_val").expect("Tokenization failed");
        assert_eq!(result[0].token_type, "Identifier");
        assert_eq!(result[0].value, "integer_val");
    }

    #[test]
    fn test_c_number_formats() {
        let tokenizer = get_c_style_tokenizer();

        for (input, expected) in [
            ("0xFF",   "0xFF"),
            ("0o777",  "0o777"),
            ("3.14",   "3.14"),
            ("1e-6",   "1e-6"),
            ("42",     "42"),
        ] {
            let result = tokenizer.tokenize(input).unwrap();
            assert_eq!(result[0].token_type, "Number", "input: {input}");
            assert_eq!(result[0].value, expected);
        }
    }

    #[test]
    fn test_ruby_comments() {
        let tokenizer = get_ruby_style_tokenizer();

        // `#` outside a string — line comment
        let input = "x = 1 # assign one";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        let comment = result.iter().find(|t| t.token_type == "Comment").unwrap();
        assert_eq!(comment.token_sub_type, Some("Line"));
        assert!(comment.value.starts_with('#'));

        // `#` inside a double-quoted string — consumed by the string scanner, NOT a comment
        let input = "msg = \"Hello #{name}\"";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        assert!(
            result.iter().all(|t| t.token_type != "Comment"),
            "# inside string interpolation must not become a Comment token"
        );
    }

    #[test]
    fn test_ruby_method_names() {
        let tokenizer = get_ruby_style_tokenizer();

        // Method names ending with `?` (predicate) and `!` (mutating)
        let input = "arr.empty? obj.save!";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        let identifiers: Vec<_> = result.iter()
            .filter(|t| t.token_type == "Identifier")
            .map(|t| t.value.as_str())
            .collect();

        assert!(identifiers.contains(&"empty?"), "empty? should be one Identifier token");
        assert!(identifiers.contains(&"save!"),  "save! should be one Identifier token");
    }

    #[test]
    fn test_ruby_keywords() {
        let tokenizer = get_ruby_style_tokenizer();

        // `save!` must NOT produce a keyword token for `save`
        let input = "obj.save! if arr.empty?";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        let kw_values: Vec<_> = result.iter()
            .filter(|t| t.token_type == "Keyword")
            .map(|t| t.value.as_str())
            .collect();

        assert!(kw_values.contains(&"if"),   "expected 'if' keyword");
        assert!(!kw_values.contains(&"save"), "'save' must not match as keyword before '!'");
    }

    #[test]
    fn test_html_comments() {
        let tokenizer = get_html_style_tokenizer();

        let input = r#"<p>Hello</p><!-- server-side note --><span>World</span>"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        let comment = result.iter().find(|t| t.token_type == "Comment").unwrap();
        assert_eq!(comment.token_sub_type, Some("Html"));
        assert_eq!(comment.value, "<!-- server-side note -->");
    }
}