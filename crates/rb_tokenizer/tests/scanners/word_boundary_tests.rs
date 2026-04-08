use rb_tokenizer::{
    scanners::word_boundary::WordBoundaryDef,
    scanners::keyword_scanner::KeywordScanner,
    scanners::scanner::Scanner,
    Tokenizer,
};

#[cfg(test)]
mod word_boundary_tests {
    use super::*;

    // ── WordBoundaryDef::Default ──────────────────────────────────────────────

    #[test]
    fn test_default_alphanumeric_is_word_char() {
        let def = WordBoundaryDef::Default;
        assert!(def.is_word_char('a'));
        assert!(def.is_word_char('Z'));
        assert!(def.is_word_char('9'));
        assert!(def.is_word_char('_'));
    }

    #[test]
    fn test_default_symbols_not_word_char() {
        let def = WordBoundaryDef::Default;
        assert!(!def.is_word_char('?'));
        assert!(!def.is_word_char('!'));
        assert!(!def.is_word_char('$'));
        assert!(!def.is_word_char('-'));
        assert!(!def.is_word_char('.'));
        assert!(!def.is_word_char('\''));
    }

    // ── WordBoundaryDef::WithExtra ────────────────────────────────────────────

    #[test]
    fn test_with_extra_includes_default_plus_extra() {
        let def = WordBoundaryDef::with_extra("?!");
        assert!(def.is_word_char('a'));   // default
        assert!(def.is_word_char('_'));   // default
        assert!(def.is_word_char('?'));   // extra
        assert!(def.is_word_char('!'));   // extra
        assert!(!def.is_word_char('$'));  // not included
        assert!(!def.is_word_char('-'));  // not included
    }

    // ── WordBoundaryDef::Custom ───────────────────────────────────────────────

    #[test]
    fn test_custom_only_listed_chars_are_word_chars() {
        let def = WordBoundaryDef::custom("abc");
        assert!(def.is_word_char('a'));
        assert!(def.is_word_char('b'));
        assert!(def.is_word_char('c'));
        // Alphanumeric are NOT automatically included
        assert!(!def.is_word_char('d'));
        assert!(!def.is_word_char('1'));
        assert!(!def.is_word_char('_'));
    }

    // ── WordBoundaryDef::Predicate ────────────────────────────────────────────

    #[test]
    fn test_predicate_uses_closure() {
        let def = WordBoundaryDef::predicate(|c| c == '@' || c == '#');
        assert!(def.is_word_char('@'));
        assert!(def.is_word_char('#'));
        assert!(!def.is_word_char('a'));
        assert!(!def.is_word_char('_'));
    }

    #[test]
    fn test_predicate_is_clone() {
        let def = WordBoundaryDef::predicate(|c| c.is_uppercase());
        let def2 = def.clone();
        assert!(def.is_word_char('A'));
        assert!(def2.is_word_char('B'));
        assert!(!def2.is_word_char('a'));
    }

    // ── Named language presets ────────────────────────────────────────────────

    #[test]
    fn test_ruby_includes_question_and_bang() {
        let def = WordBoundaryDef::ruby();
        assert!(def.is_word_char('?'));
        assert!(def.is_word_char('!'));
        assert!(def.is_word_char('a'));  // still includes defaults
        assert!(!def.is_word_char('$'));
    }

    #[test]
    fn test_javascript_includes_dollar() {
        let def = WordBoundaryDef::javascript();
        assert!(def.is_word_char('$'));
        assert!(def.is_word_char('a'));
        assert!(!def.is_word_char('?'));
    }

    #[test]
    fn test_r_lang_includes_dot_and_dollar() {
        let def = WordBoundaryDef::r_lang();
        assert!(def.is_word_char('.'));
        assert!(def.is_word_char('$'));
        assert!(def.is_word_char('a'));
    }

    #[test]
    fn test_css_includes_hyphen() {
        let def = WordBoundaryDef::css();
        assert!(def.is_word_char('-'));
        assert!(def.is_word_char('a'));
        assert!(!def.is_word_char('?'));
    }

    #[test]
    fn test_lisp_includes_symbol_chars() {
        let def = WordBoundaryDef::lisp();
        for ch in "?!+-*/<>=".chars() {
            assert!(def.is_word_char(ch), "expected {ch:?} to be a word char");
        }
        assert!(def.is_word_char('a'));
    }

    #[test]
    fn test_haskell_includes_prime() {
        let def = WordBoundaryDef::haskell();
        assert!(def.is_word_char('\''));
        assert!(def.is_word_char('a'));
    }

    // ── Clone supports all variants ───────────────────────────────────────────

    #[test]
    fn test_all_variants_are_clone() {
        let _ = WordBoundaryDef::Default.clone();
        let _ = WordBoundaryDef::with_extra("$").clone();
        let _ = WordBoundaryDef::custom("abc").clone();
        let _ = WordBoundaryDef::predicate(|c| c == '@').clone();
        let _ = WordBoundaryDef::ruby().clone();
    }

    // ── Integration: KeywordScanner using WordBoundaryDef ─────────────────────

    #[test]
    fn test_keyword_scanner_ruby_boundary_blocks_question_suffix() {
        // `save!` must NOT match keyword `save` in Ruby context
        let scanner = KeywordScanner::new("Keyword", &["save", "if"])
            .with_word_boundary_def(WordBoundaryDef::ruby());

        // bare `save` does match
        let tok = scanner.scan("save ").unwrap().unwrap();
        assert_eq!(tok.value, "save");

        // `save!` must NOT match `save` (! is a word char in Ruby boundary)
        assert!(scanner.scan("save!").unwrap().is_none());
        assert!(scanner.scan("save?").unwrap().is_none());
    }

    #[test]
    fn test_keyword_scanner_default_boundary_accepts_after_punct() {
        // With default boundary, `if?` DOES match `if` because `?` is not a word char
        let scanner = KeywordScanner::new("Keyword", &["if"]);
        let tok = scanner.scan("if?true").unwrap().unwrap();
        assert_eq!(tok.value, "if");
    }

    #[test]
    fn test_keyword_scanner_javascript_boundary() {
        // `true$` must NOT match `true` — `$` is a word char in JS context
        let scanner = KeywordScanner::new("Keyword", &["true", "false"])
            .with_word_boundary_def(WordBoundaryDef::javascript());

        assert!(scanner.scan("true$").unwrap().is_none());
        assert!(scanner.scan("true_bar").unwrap().is_none());  // _ also blocked
        // bare `true` is fine
        let tok = scanner.scan("true ").unwrap().unwrap();
        assert_eq!(tok.value, "true");
    }

    #[test]
    fn test_keyword_scanner_css_boundary_blocks_hyphenated_identifiers() {
        // `flex-wrap` must NOT match keyword `flex` — `-` is a word char in CSS
        let scanner = KeywordScanner::new("Keyword", &["flex", "grid"])
            .with_word_boundary_def(WordBoundaryDef::css());

        assert!(scanner.scan("flex-wrap").unwrap().is_none());
        assert!(scanner.scan("grid-template").unwrap().is_none());
        // standalone `flex:` is fine
        let tok = scanner.scan("flex:").unwrap().unwrap();
        assert_eq!(tok.value, "flex");
    }

    #[test]
    fn test_keyword_scanner_with_word_boundary_closure_still_works() {
        // Backward-compat: with_word_boundary() still accepts a closure
        let scanner = KeywordScanner::new("Keyword", &["end"])
            .with_word_boundary(|c| c.is_alphanumeric() || c == '_' || c == '?');

        assert!(scanner.scan("end?").unwrap().is_none());
        let tok = scanner.scan("end.").unwrap().unwrap();
        assert_eq!(tok.value, "end");
    }

    // ── Tokenizer integration ─────────────────────────────────────────────────

    #[test]
    fn test_tokenizer_with_ruby_boundary_on_keywords() {
        let mut t = Tokenizer::new();
        // Keywords: `def`, `end` — Ruby boundary prevents `end?` from matching `end`
        t.add_scanner(Box::new(
            KeywordScanner::new("Keyword", &["def", "end"])
                .with_word_boundary_def(WordBoundaryDef::ruby()),
        ));
        t.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_?!"), "Ident", None);
        t.add_regex_scanner(r"^\s+", "Ws", None).unwrap();

        let tokens = t.tokenize("def end? end").unwrap();
        let non_ws: Vec<_> = tokens.iter().filter(|t| t.token_type != "Ws").collect();

        assert_eq!(non_ws[0].token_type, "Keyword"); // def
        assert_eq!(non_ws[1].token_type, "Ident");   // end? — NOT the keyword
        assert_eq!(non_ws[1].value, "end?");
        assert_eq!(non_ws[2].token_type, "Keyword"); // end
    }
}
