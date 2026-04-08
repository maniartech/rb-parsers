use std::sync::Arc;

/// Defines which characters count as "word characters" for the purpose of
/// keyword boundary checking.
///
/// A [`KeywordScanner`] rejects a keyword match when the character *immediately
/// after* the keyword satisfies this definition.  This prevents `if` from
/// matching inside `ifdef`, `class` from matching inside `classname`, etc.
///
/// Use the named language presets for common cases, or [`with_extra`] to extend
/// the default set with a few additional characters.
///
/// # Language presets
///
/// | Preset | Extra word chars | When to use |
/// |---|---|---|
/// | [`WordBoundaryDef::default()`] | *(none)* — alphanumeric + `_` | C, Java, JSON, most languages |
/// | [`WordBoundaryDef::ruby()`] | `?` `!` | Ruby method suffixes: `empty?`, `save!` |
/// | [`WordBoundaryDef::javascript()`] | `$` | JS/TS identifiers: `$foo`, `$$ready`, `$type` |
/// | [`WordBoundaryDef::r_lang()`] | `.` `$` | R: `.Machine`, `is.numeric`, `data$col` |
/// | [`WordBoundaryDef::css()`] | `-` | CSS property names: `background-color`, `flex-wrap` |
/// | [`WordBoundaryDef::lisp()`] | `?!+-*/<>=` | Lisps: `string->number`, `car?`, `set!`, `*default*` |
/// | [`WordBoundaryDef::haskell()`] | `'` | Haskell prime suffix: `where'`, `go'`, `f''` |
///
/// # Examples
///
/// ```rust,ignore
/// use rb_tokenizer::scanners::{KeywordScanner, WordBoundaryDef};
///
/// // Ruby — `save!` must not match `save` keyword
/// let scanner = KeywordScanner::with_subtypes("Keyword", &[("save", "Save")])
///     .with_word_boundary_def(WordBoundaryDef::ruby());
///
/// // CSS — `flex-wrap` must not match `flex` keyword
/// let scanner = KeywordScanner::new("Keyword", &["flex", "grid"])
///     .with_word_boundary_def(WordBoundaryDef::css());
///
/// // Custom: add `?` and `$` to the default set
/// let scanner = KeywordScanner::new("Keyword", &["end"])
///     .with_word_boundary_def(WordBoundaryDef::with_extra("?$"));
/// ```
///
/// [`KeywordScanner`]: crate::scanners::KeywordScanner
/// [`with_extra`]: WordBoundaryDef::with_extra
#[derive(Clone)]
pub enum WordBoundaryDef {
    /// Standard C-family word characters: `[a-zA-Z0-9_]`.
    ///
    /// This is the default used by [`KeywordScanner`](crate::scanners::KeywordScanner)
    /// when no explicit boundary is set.
    Default,

    /// Extends the default `[a-zA-Z0-9_]` set with additional literal characters.
    ///
    /// ```rust,ignore
    /// WordBoundaryDef::with_extra("?!")   // Ruby-style method name suffixes
    /// WordBoundaryDef::with_extra("$")    // JavaScript identifier sigil
    /// WordBoundaryDef::with_extra("-")    // CSS property-name hyphen
    /// ```
    WithExtra(&'static str),

    /// Fully custom character set — **only** the listed characters count as word chars.
    ///
    /// Alphanumeric and `_` are **not** included unless explicitly listed.
    /// Use this for languages where the standard alphanumeric set does not apply.
    Custom(&'static str),

    /// Full predicate closure for complex or Unicode-aware word-character rules.
    ///
    /// Wrapped in [`Arc`] so the definition is cheap to clone and can be shared
    /// across multiple scanner instances.
    ///
    /// ```rust,ignore
    /// WordBoundaryDef::predicate(|c| c.is_alphanumeric() || c == '_' || c == '\'')
    /// ```
    Predicate(Arc<dyn Fn(char) -> bool + Send + Sync>),
}

impl std::fmt::Debug for WordBoundaryDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "WordBoundaryDef::Default"),
            Self::WithExtra(s) => write!(f, "WordBoundaryDef::WithExtra({s:?})"),
            Self::Custom(s) => write!(f, "WordBoundaryDef::Custom({s:?})"),
            Self::Predicate(_) => write!(f, "WordBoundaryDef::Predicate(..)"),
        }
    }
}

impl Default for WordBoundaryDef {
    fn default() -> Self {
        Self::Default
    }
}

impl WordBoundaryDef {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Extends the default `[a-zA-Z0-9_]` set with additional literal characters.
    ///
    /// Pass a `&'static str` containing the extra characters, e.g. `"?!"` for Ruby.
    pub fn with_extra(extra: &'static str) -> Self {
        Self::WithExtra(extra)
    }

    /// Fully custom character set.  No implicit alphanumeric or `_`.
    ///
    /// Use only when the language's concept of a word character does not
    /// include the standard ASCII alphanumeric set.
    pub fn custom(chars: &'static str) -> Self {
        Self::Custom(chars)
    }

    /// Full predicate closure.
    ///
    /// The closure is wrapped in `Arc` so the resulting `WordBoundaryDef` is
    /// `Clone` and `Send + Sync`.
    pub fn predicate(f: impl Fn(char) -> bool + Send + Sync + 'static) -> Self {
        Self::Predicate(Arc::new(f))
    }

    // ── Named language presets ────────────────────────────────────────────────

    /// **Ruby** — method names may end with `?` or `!`:
    /// `empty?`, `include?`, `save!`, `freeze!`.
    ///
    /// Without this preset, `if?` would wrongly match the `if` keyword.
    pub fn ruby() -> Self {
        Self::with_extra("?!")
    }

    /// **JavaScript / TypeScript** — `$` is a valid identifier start and
    /// continuation character (used widely in `$el`, `$$`, `jQuery`, etc.).
    ///
    /// Without this preset, `true$` would match `true` as a keyword even
    /// though `true$` is a valid JS identifier name.
    pub fn javascript() -> Self {
        Self::with_extra("$")
    }

    /// **R language** — `.` and `$` appear in standard identifiers:
    /// `.Machine`, `.GlobalEnv`, `is.numeric`, `data$column`.
    pub fn r_lang() -> Self {
        Self::with_extra(".$")
    }

    /// **CSS / SCSS / Less** — hyphens are valid within property and class names:
    /// `background-color`, `flex-wrap`, `--custom-property`, `text-transform`.
    ///
    /// Without this preset, the keyword `flex` would wrongly match at the start
    /// of the identifier `flex-wrap`.
    pub fn css() -> Self {
        Self::with_extra("-")
    }

    /// **Lisp / Scheme / Clojure** — identifiers freely use symbol characters:
    /// `string->number`, `car?`, `set!`, `*default*`, `+`, `>=`, `loop*`.
    pub fn lisp() -> Self {
        Self::with_extra("?!+-*/<>=")
    }

    /// **Haskell** — identifiers may include a prime/tick suffix (`'`):
    /// `where'`, `go'`, `f''`, `x'`.
    ///
    /// Also used in ML-family languages (OCaml, F#).
    pub fn haskell() -> Self {
        Self::with_extra("'")
    }

    // ── Core logic ────────────────────────────────────────────────────────────

    /// Returns `true` when `c` counts as a word character under this definition.
    ///
    /// A keyword scanner rejects a match when the character *immediately after*
    /// the keyword satisfies this test — preventing keywords from matching as
    /// prefixes of longer identifiers.
    #[inline]
    pub fn is_word_char(&self, c: char) -> bool {
        match self {
            Self::Default => c.is_alphanumeric() || c == '_',
            Self::WithExtra(extra) => c.is_alphanumeric() || c == '_' || extra.contains(c),
            Self::Custom(chars) => chars.contains(c),
            Self::Predicate(f) => f(c),
        }
    }
}
