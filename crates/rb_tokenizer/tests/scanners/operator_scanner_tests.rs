use rb_tokenizer::{
    scanners::operator_scanner::OperatorScanner,
    scanners::scanner::Scanner,
    scanners::scanner_types::ScannerType,
    Tokenizer,
};

#[cfg(test)]
mod operator_scanner_tests {
    use super::*;

    // ── OperatorScanner::new ──────────────────────────────────────────────────

    #[test]
    fn test_new_matches_exact_operator() {
        let scanner = OperatorScanner::new("Op", &["+", "-", "*", "/"]);
        let result = scanner.scan("+x").unwrap();
        assert!(result.is_some());
        let tok = result.unwrap();
        assert_eq!(tok.token_type, "Op");
        assert_eq!(tok.value, "+");
        assert_eq!(tok.token_sub_type, None);
    }

    #[test]
    fn test_new_no_match() {
        let scanner = OperatorScanner::new("Op", &["+", "-"]);
        assert!(scanner.scan("xyz").unwrap().is_none());
    }

    #[test]
    fn test_new_no_word_boundary_required() {
        // Operator scanners must match even when directly adjacent to identifiers.
        let scanner = OperatorScanner::new("Op", &["++"]);
        let tok = scanner.scan("++i").unwrap().unwrap();
        assert_eq!(tok.value, "++");
    }

    // ── Longest-match behaviour ───────────────────────────────────────────────

    #[test]
    fn test_longest_match_preferred() {
        // ++  should be preferred over  +  when input is "++"
        let scanner = OperatorScanner::new("Op", &["+", "++", "+="]);
        let tok = scanner.scan("++x").unwrap().unwrap();
        assert_eq!(tok.value, "++");
    }

    #[test]
    fn test_longest_match_triple_char() {
        let scanner = OperatorScanner::new("Op", &["<", "<<", "<<="]);
        let tok = scanner.scan("<<=foo").unwrap().unwrap();
        assert_eq!(tok.value, "<<=");
    }

    #[test]
    fn test_falls_back_to_shorter_when_longer_absent() {
        let scanner = OperatorScanner::new("Op", &["<", "<<="]);
        // "<<" is not in the list; should match "<" (first char)
        let tok = scanner.scan("<<x").unwrap().unwrap();
        assert_eq!(tok.value, "<");
    }

    #[test]
    fn test_with_subtypes_longest_match() {
        let scanner = OperatorScanner::with_subtypes("Op", &[
            ("<<=", "ShlAssign"),
            ("<<",  "Shl"),
            ("<=",  "Le"),
            ("<",   "Lt"),
        ]);
        assert_eq!(scanner.scan("<<=").unwrap().unwrap().token_sub_type, Some("ShlAssign"));
        assert_eq!(scanner.scan("<<x").unwrap().unwrap().token_sub_type,  Some("Shl"));
        assert_eq!(scanner.scan("<=x").unwrap().unwrap().token_sub_type,  Some("Le"));
        assert_eq!(scanner.scan("<x").unwrap().unwrap().token_sub_type,   Some("Lt"));
    }

    // ── with_subtypes ─────────────────────────────────────────────────────────

    #[test]
    fn test_with_subtypes_basic() {
        let scanner = OperatorScanner::with_subtypes("Op", &[
            ("+=", "AddAssign"),
            ("-=", "SubAssign"),
            ("++", "Inc"),
            ("--", "Dec"),
            ("+",  "Add"),
            ("-",  "Sub"),
        ]);

        let tok = scanner.scan("+=x").unwrap().unwrap();
        assert_eq!(tok.token_type, "Op");
        assert_eq!(tok.value, "+=");
        assert_eq!(tok.token_sub_type, Some("AddAssign"));

        let tok = scanner.scan("++n").unwrap().unwrap();
        assert_eq!(tok.value, "++");
        assert_eq!(tok.token_sub_type, Some("Inc"));

        let tok = scanner.scan("+x").unwrap().unwrap();
        assert_eq!(tok.value, "+");
        assert_eq!(tok.token_sub_type, Some("Add"));
    }

    #[test]
    fn test_with_subtypes_arrow_operators() {
        let scanner = OperatorScanner::with_subtypes("Op", &[
            ("->", "Arrow"),
            ("=>", "FatArrow"),
            ("=",  "Assign"),
        ]);

        assert_eq!(scanner.scan("->foo").unwrap().unwrap().value, "->");
        assert_eq!(scanner.scan("=>bar").unwrap().unwrap().value, "=>");
        // "=" alone after ruling out "->" and "=>"
        assert_eq!(scanner.scan("=x").unwrap().unwrap().value, "=");
    }

    // ── ScannerType::Operator dispatch ───────────────────────────────────────

    #[test]
    fn test_scanner_type_operator_dispatch() {
        let st = ScannerType::Operator(OperatorScanner::new("Op", &["+=", "+"]));
        let tok = st.scan("+=1").unwrap().unwrap();
        assert_eq!(tok.value, "+=");
    }

    #[test]
    fn test_scanner_type_operator_no_match() {
        let st = ScannerType::Operator(OperatorScanner::new("Op", &["+", "-"]));
        assert!(st.scan("xyz").unwrap().is_none());
    }

    // ── Tokenizer integration ─────────────────────────────────────────────────

    #[test]
    fn test_add_operator_scanner_basic() {
        let mut t = Tokenizer::new();
        t.add_operator_scanner("Op", &["+=", "++", "+", "-=", "--", "-"]);
        t.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_"), "Ident", None);
        t.add_regex_scanner(r"^\s+", "Ws", None).unwrap();

        let tokens = t.tokenize("x++ + y -= z").unwrap();
        let non_ws: Vec<_> = tokens.iter().filter(|t| t.token_type != "Ws").collect();

        assert_eq!(non_ws[0].value, "x");
        assert_eq!(non_ws[1].value, "++");
        assert_eq!(non_ws[2].value, "+");
        assert_eq!(non_ws[3].value, "y");
        assert_eq!(non_ws[4].value, "-=");
        assert_eq!(non_ws[5].value, "z");
    }

    #[test]
    fn test_add_operator_scanner_with_subtypes_integration() {
        let mut t = Tokenizer::new();
        t.add_operator_scanner_with_subtypes("Op", &[
            ("<<=", "ShlAssign"),
            ("<<",  "Shl"),
            ("<=",  "Le"),
            ("<",   "Lt"),
        ]);
        t.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_"), "Ident", None);
        t.add_regex_scanner(r"^\s+", "Ws", None).unwrap();

        let tokens = t.tokenize("a <<= b").unwrap();
        let ops: Vec<_> = tokens.iter().filter(|t| t.token_type == "Op").collect();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].value, "<<=");
        assert_eq!(ops[0].token_sub_type, Some("ShlAssign"));
    }

    #[test]
    fn test_operator_does_not_enforce_word_boundary() {
        // Contrast with KeywordScanner: operators must match even inside identifier chars.
        // e.g. "not" operator in a language where "not_x" is also a valid identifier.
        let mut t = Tokenizer::new();
        // Register "!=" and "!" as operators
        t.add_operator_scanner("Op", &["!=", "!"]);
        t.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_"), "Ident", None);

        let tokens = t.tokenize("!flag").unwrap();
        assert_eq!(tokens[0].value, "!");
        assert_eq!(tokens[0].token_type, "Op");
        assert_eq!(tokens[1].value, "flag");
        assert_eq!(tokens[1].token_type, "Ident");
    }

    #[test]
    fn test_ordering_supplied_out_of_order_still_longest_first() {
        // Even if the caller supplies shorter operators before longer ones,
        // the scanner must sort internally and prefer the longest match.
        let scanner = OperatorScanner::new("Op", &["+", "++", "+="]);
        assert_eq!(scanner.scan("+=").unwrap().unwrap().value, "+=");
        assert_eq!(scanner.scan("++").unwrap().unwrap().value, "++");
        assert_eq!(scanner.scan("+x").unwrap().unwrap().value, "+");
    }
}
