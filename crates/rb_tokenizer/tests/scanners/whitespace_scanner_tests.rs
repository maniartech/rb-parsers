use rb_tokenizer::{
    scanners::whitespace_scanner::WhitespaceScanner,
    scanners::scanner::Scanner,
    scanners::scanner_types::ScannerType,
    Tokenizer,
};

#[cfg(test)]
mod whitespace_scanner_tests {
    use super::*;

    // ── WhitespaceScanner::uniform ────────────────────────────────────────────

    #[test]
    fn test_uniform_matches_spaces() {
        let s = WhitespaceScanner::uniform("Ws");
        let tok = s.scan("   hello").unwrap().unwrap();
        assert_eq!(tok.token_type, "Ws");
        assert_eq!(tok.value, "   ");
    }

    #[test]
    fn test_uniform_matches_tabs() {
        let s = WhitespaceScanner::uniform("Ws");
        let tok = s.scan("\t\thello").unwrap().unwrap();
        assert_eq!(tok.value, "\t\t");
    }

    #[test]
    fn test_uniform_includes_newlines() {
        let s = WhitespaceScanner::uniform("Ws");
        // All whitespace (including \n) in one token
        let tok = s.scan("  \n  x").unwrap().unwrap();
        assert_eq!(tok.value, "  \n  ");
    }

    #[test]
    fn test_uniform_includes_crlf() {
        let s = WhitespaceScanner::uniform("Ws");
        let tok = s.scan("  \r\n  x").unwrap().unwrap();
        assert_eq!(tok.value, "  \r\n  ");
    }

    #[test]
    fn test_uniform_no_match_on_non_whitespace() {
        let s = WhitespaceScanner::uniform("Ws");
        assert!(s.scan("hello").unwrap().is_none());
        assert!(s.scan("").unwrap().is_none());
    }

    // ── WhitespaceScanner::split ──────────────────────────────────────────────

    #[test]
    fn test_split_horizontal_whitespace_emits_ws_token() {
        let s = WhitespaceScanner::split("Ws", "Nl");
        let tok = s.scan("  \nfoo").unwrap().unwrap();
        assert_eq!(tok.token_type, "Ws");
        assert_eq!(tok.value, "  ");  // stops before the newline
    }

    #[test]
    fn test_split_newline_emits_nl_token() {
        let s = WhitespaceScanner::split("Ws", "Nl");
        let tok = s.scan("\nfoo").unwrap().unwrap();
        assert_eq!(tok.token_type, "Nl");
        assert_eq!(tok.value, "\n");
    }

    #[test]
    fn test_split_crlf_emits_single_nl_token() {
        let s = WhitespaceScanner::split("Ws", "Nl");
        let tok = s.scan("\r\nfoo").unwrap().unwrap();
        assert_eq!(tok.token_type, "Nl");
        assert_eq!(tok.value, "\r\n");
    }

    #[test]
    fn test_split_bare_cr_emits_nl_token() {
        let s = WhitespaceScanner::split("Ws", "Nl");
        let tok = s.scan("\rfoo").unwrap().unwrap();
        assert_eq!(tok.token_type, "Nl");
        assert_eq!(tok.value, "\r");
    }

    #[test]
    fn test_split_no_match_on_non_whitespace() {
        let s = WhitespaceScanner::split("Ws", "Nl");
        assert!(s.scan("foo").unwrap().is_none());
    }

    #[test]
    fn test_split_sequence_produces_separate_tokens() {
        // Scanning "  \n  " in split mode should produce three tokens when
        // the scanner is called three times.
        let s = WhitespaceScanner::split("Ws", "Nl");
        let input = "  \n  ;";

        let t1 = s.scan(input).unwrap().unwrap();
        assert_eq!(t1.token_type, "Ws");
        assert_eq!(t1.value, "  ");

        let t2 = s.scan(&input[t1.value.len()..]).unwrap().unwrap();
        assert_eq!(t2.token_type, "Nl");
        assert_eq!(t2.value, "\n");

        let t3 = s.scan(&input[t1.value.len() + t2.value.len()..]).unwrap().unwrap();
        assert_eq!(t3.token_type, "Ws");
        assert_eq!(t3.value, "  ");
    }

    // ── WhitespaceScanner::with_continuation ─────────────────────────────────

    #[test]
    fn test_continuation_backslash_newline_emits_cont_token() {
        let s = WhitespaceScanner::with_continuation("Ws", "Nl", "Cont");
        let tok = s.scan("\\\nrest").unwrap().unwrap();
        assert_eq!(tok.token_type, "Cont");
        assert_eq!(tok.value, "\\\n");
    }

    #[test]
    fn test_continuation_backslash_crlf_emits_cont_token() {
        let s = WhitespaceScanner::with_continuation("Ws", "Nl", "Cont");
        let tok = s.scan("\\\r\nrest").unwrap().unwrap();
        assert_eq!(tok.token_type, "Cont");
        assert_eq!(tok.value, "\\\r\n");
    }

    #[test]
    fn test_continuation_backslash_not_before_newline_returns_none() {
        // `\x` is not a line continuation — let an operator scanner handle `\`
        let s = WhitespaceScanner::with_continuation("Ws", "Nl", "Cont");
        assert!(s.scan("\\x").unwrap().is_none());
        assert!(s.scan("\\ ").unwrap().is_none()); // backslash + space — not cont
    }

    #[test]
    fn test_continuation_newlines_still_split() {
        let s = WhitespaceScanner::with_continuation("Ws", "Nl", "Cont");
        let tok = s.scan("\n").unwrap().unwrap();
        assert_eq!(tok.token_type, "Nl");
    }

    #[test]
    fn test_continuation_horizontal_still_works() {
        let s = WhitespaceScanner::with_continuation("Ws", "Nl", "Cont");
        let tok = s.scan("   next").unwrap().unwrap();
        assert_eq!(tok.token_type, "Ws");
        assert_eq!(tok.value, "   ");
    }

    // ── ScannerType::Whitespace dispatch ─────────────────────────────────────

    #[test]
    fn test_scanner_type_whitespace_dispatches_scan() {
        let st = ScannerType::Whitespace(WhitespaceScanner::uniform("Ws"));
        let tok = st.scan("  x").unwrap().unwrap();
        assert_eq!(tok.token_type, "Ws");
        assert_eq!(tok.value, "  ");
    }

    #[test]
    fn test_scanner_type_whitespace_no_match() {
        let st = ScannerType::Whitespace(WhitespaceScanner::uniform("Ws"));
        assert!(st.scan("abc").unwrap().is_none());
    }

    // ── Tokenizer integration ─────────────────────────────────────────────────

    #[test]
    fn test_add_whitespace_scanner_uniform_mode() {
        let mut t = Tokenizer::new();
        t.add_keyword_scanner_with_subtypes("Kw", &[("let", "Let"), ("const", "Const")]);
        t.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_"), "Ident", None);
        t.add_whitespace_scanner(WhitespaceScanner::uniform("Ws"));

        let tokens = t.tokenize("let x\n  const y").unwrap();
        let ws_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == "Ws").collect();
        // All whitespace including the newline is one type
        assert!(!ws_tokens.is_empty());
        for ws in &ws_tokens {
            assert_eq!(ws.token_type, "Ws");
        }
        // No "Nl" tokens in uniform mode
        assert!(!tokens.iter().any(|t| t.token_type == "Nl"));
    }

    #[test]
    fn test_add_whitespace_scanner_split_mode_emits_separate_newlines() {
        let mut t = Tokenizer::new();
        t.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_"), "Ident", None);
        t.add_whitespace_scanner(WhitespaceScanner::split("Ws", "Nl"));

        let tokens = t.tokenize("a\nb").unwrap();
        // Expected: Ident("a"), Nl("\n"), Ident("b")
        assert_eq!(tokens[0].value, "a");
        assert_eq!(tokens[1].token_type, "Nl");
        assert_eq!(tokens[1].value, "\n");
        assert_eq!(tokens[2].value, "b");
    }

    #[test]
    fn test_add_whitespace_scanner_continuation_mode() {
        let mut t = Tokenizer::new();
        t.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_"), "Ident", None);
        t.add_whitespace_scanner(WhitespaceScanner::with_continuation("Ws", "Nl", "Cont"));

        // "foo \\\nbar" → Ident, Ws, Cont, Ident
        let tokens = t.tokenize("foo \\\nbar").unwrap();
        let types: Vec<_> = tokens.iter().map(|t| t.token_type).collect();
        assert_eq!(types, ["Ident", "Ws", "Cont", "Ident"]);
        assert_eq!(tokens[2].value, "\\\n");
    }

    #[test]
    fn test_split_mode_tokenizer_line_counting() {
        // Each Nl token represents one logical newline boundary.
        let mut t = Tokenizer::new();
        t.add_char_class_scanner("a-zA-Z", Some("a-zA-Z"), "Ident", None);
        t.add_whitespace_scanner(WhitespaceScanner::split("Ws", "Nl"));

        let tokens = t.tokenize("a\nb\nc").unwrap();
        let nl_count = tokens.iter().filter(|t| t.token_type == "Nl").count();
        assert_eq!(nl_count, 2);
    }
}
