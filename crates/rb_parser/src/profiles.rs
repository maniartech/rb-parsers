// ── LanguageVersion ───────────────────────────────────────────────────────────

/// A `major.minor` version tag for a language dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageVersion {
    /// Major version component.
    pub major: u32,
    /// Minor version component.
    pub minor: u32,
}

impl LanguageVersion {
    /// Constructs a new `LanguageVersion`.
    pub const fn new(major: u32, minor: u32) -> Self {
        LanguageVersion { major, minor }
    }

    /// Parse `"major"` or `"major.minor"`.
    pub fn parse(s: &str) -> Result<Self, LanguageVersionError> {
        let mut parts = s.splitn(2, '.');
        let major = parts
            .next()
            .ok_or(LanguageVersionError::Empty)?
            .parse::<u32>()
            .map_err(|_| LanguageVersionError::InvalidNumber)?;
        let minor = parts
            .next()
            .map(|m| m.parse::<u32>().map_err(|_| LanguageVersionError::InvalidNumber))
            .transpose()?
            .unwrap_or(0);
        Ok(LanguageVersion { major, minor })
    }
}

impl std::fmt::Display for LanguageVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Error returned by [`LanguageVersion::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageVersionError {
    /// The input string was empty.
    Empty,
    /// A numeric component could not be parsed as `u32`.
    InvalidNumber,
}

// ── ProfileMode ───────────────────────────────────────────────────────────────

/// The active language mode, controlling how strictly the grammar is enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProfileMode {
    /// Standard mode — the grammar's default behaviour.
    #[default]
    Default,
    /// Strict mode — zero tolerance for deprecated or ambiguous constructs.
    Strict,
    /// Tolerant mode — relaxed rules, wider recovery window.
    Tolerant,
    /// Legacy mode — compatibility with an older language version.
    Legacy,
    /// A user-defined custom mode identified by name.
    Custom(&'static str),
}

impl std::fmt::Display for ProfileMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileMode::Default      => write!(f, "default"),
            ProfileMode::Strict       => write!(f, "strict"),
            ProfileMode::Tolerant     => write!(f, "tolerant"),
            ProfileMode::Legacy       => write!(f, "legacy"),
            ProfileMode::Custom(name) => write!(f, "custom({name})"),
        }
    }
}

// ── FeatureFlag ───────────────────────────────────────────────────────────────

/// An opaque feature-flag name used to gate grammar rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureFlag(pub &'static str);

impl FeatureFlag {
    /// Returns the string name of this feature flag.
    pub fn as_str(self) -> &'static str { self.0 }
}

impl std::fmt::Display for FeatureFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

// ── ResolvedProfileId ─────────────────────────────────────────────────────────

/// Deterministic, hashable identity for a resolved profile. Suitable as a
/// cache key for compiled grammar state. Uses FNV-64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolvedProfileId(pub u64);

/// A1 — Deterministic FNV-64 hash, stable across runs, platforms, and Rust versions.
fn fnv64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 14695981039346656037;
    const PRIME: u64 = 1099511628211;
    bytes.iter().fold(OFFSET, |hash, &b| hash.wrapping_mul(PRIME) ^ (b as u64))
}

impl ResolvedProfileId {
    /// Computes the deterministic profile id from its constituent parts.
    ///
    /// `features` **must** be sorted; the caller is responsible for sorting them.
    pub fn compute(
        language: &str,
        version: LanguageVersion,
        mode: ProfileMode,
        features: &[FeatureFlag], // must be sorted by caller
    ) -> Self {
        let mut buf = String::new();
        buf.push_str(language);
        buf.push('\x00');
        buf.push_str(&version.to_string());
        buf.push('\x00');
        buf.push_str(&mode.to_string());
        for f in features {
            buf.push('\x00');
            buf.push_str(f.0);
        }
        ResolvedProfileId(fnv64(buf.as_bytes()))
    }
}

// ── ResolvedProfile ───────────────────────────────────────────────────────────

/// The immutable, fully-determined language profile produced by
/// `ProfileCatalog::resolve()`. Handed to `Grammar::compile()` and to each
/// `ParseContext`.
#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    /// The unique, computed identifier for this profile.
    pub id: ResolvedProfileId,
    /// The language name this profile applies to (e.g. `"ruby"`).
    pub language: &'static str,
    /// The language version this profile targets.
    pub version: LanguageVersion,
    /// The parse mode (strict, lenient, etc.) for this profile.
    pub mode: ProfileMode,
    /// Sorted list of enabled feature flags.
    pub enabled_features: Vec<FeatureFlag>,
}

impl ResolvedProfile {
    /// Returns `true` if `flag` is in the profile's enabled feature set.
    pub fn has_feature(&self, flag: FeatureFlag) -> bool {
        self.enabled_features.contains(&flag)
    }

    /// Returns `true` if the profile's language version is at least `version`.
    pub fn is_at_least(&self, version: LanguageVersion) -> bool {
        self.version >= version
    }

    /// Returns `true` if the profile's language version is strictly before `version`.
    pub fn is_before(&self, version: LanguageVersion) -> bool {
        self.version < version
    }

    /// Returns `true` if the profile's mode equals `mode`.
    pub fn mode_is(&self, mode: ProfileMode) -> bool {
        self.mode == mode
    }

    /// Construct a minimal profile for a language with default settings.
    pub fn simple(language: &'static str) -> Self {
        let version = LanguageVersion::new(1, 0);
        let mode = ProfileMode::Default;
        let features: Vec<FeatureFlag> = Vec::new();
        let id = ResolvedProfileId::compute(language, version, mode, &features);
        ResolvedProfile { id, language, version, mode, enabled_features: features }
    }

    /// Return a copy of this profile with a different mode.
    pub fn with_mode(self, mode: ProfileMode) -> Self {
        let id = ResolvedProfileId::compute(self.language, self.version, mode, &self.enabled_features);
        ResolvedProfile { id, mode, ..self }
    }
}

// ── RuleProfileGuard ──────────────────────────────────────────────────────────

/// Attached to a grammar rule via `.enabled_if(guard)`. Evaluated during
/// parsing to determine whether the rule branch is active for the current
/// profile.
/// Attached to a grammar rule via `.enabled_if(guard)`. Evaluated during
/// parsing to determine whether the rule branch is active for the current
/// profile.
#[derive(Debug, Clone)]
pub struct RuleProfileGuard {
    /// Minimum language version required for this rule to be active.
    pub since: Option<LanguageVersion>,
    /// Language version at which this rule becomes inactive (exclusive).
    pub until: Option<LanguageVersion>,
    /// All of these feature flags must be enabled.
    pub requires_all: Vec<FeatureFlag>,
    /// At least one of these feature flags must be enabled.
    pub requires_any: Vec<FeatureFlag>,
    /// None of these feature flags may be enabled.
    pub forbids_any: Vec<FeatureFlag>,
    /// If non-empty, the profile mode must be in this list.
    pub modes: Vec<ProfileMode>,
}

impl RuleProfileGuard {
    /// Returns `true` if all guard conditions are satisfied by `profile`.
    pub fn is_active(&self, profile: &ResolvedProfile) -> bool {
        if let Some(since) = self.since {
            if profile.version < since { return false; }
        }
        if let Some(until) = self.until {
            if profile.version >= until { return false; }
        }
        if !self.requires_all.iter().all(|f| profile.has_feature(*f)) {
            return false;
        }
        if !self.requires_any.is_empty()
            && !self.requires_any.iter().any(|f| profile.has_feature(*f))
        {
            return false;
        }
        if self.forbids_any.iter().any(|f| profile.has_feature(*f)) {
            return false;
        }
        if !self.modes.is_empty() && !self.modes.contains(&profile.mode) {
            return false;
        }
        true
    }
}

// ── ProfileGuardBuilder ───────────────────────────────────────────────────────

/// Fluent DSL for constructing [`RuleProfileGuard`] values.
#[derive(Default)]
pub struct ProfileGuardBuilder {
    since: Option<LanguageVersion>,
    until: Option<LanguageVersion>,
    requires_all: Vec<FeatureFlag>,
    requires_any: Vec<FeatureFlag>,
    forbids_any: Vec<FeatureFlag>,
    modes: Vec<ProfileMode>,
}

impl ProfileGuardBuilder {
    /// Sets the minimum language version for this guard.
    pub fn since(mut self, version: &'static str) -> Self {
        self.since = LanguageVersion::parse(version).ok();
        self
    }

    /// Sets the exclusive upper version bound for this guard.
    pub fn until(mut self, version: &'static str) -> Self {
        self.until = LanguageVersion::parse(version).ok();
        self
    }

    /// Requires that all of the given feature flags are enabled.
    pub fn feature(mut self, flag: &'static str) -> Self {
        self.requires_all.push(FeatureFlag(flag));
        self
    }

    /// Requires that at least one of the given feature flags is enabled.
    pub fn any_feature(mut self, flags: &[&'static str]) -> Self {
        self.requires_any.extend(flags.iter().map(|s| FeatureFlag(s)));
        self
    }

    /// Restricts this guard to the given profile mode.
    pub fn mode(mut self, mode: ProfileMode) -> Self {
        self.modes.push(mode);
        self
    }

    /// Consumes the builder and returns the completed [`RuleProfileGuard`].
    pub fn build(self) -> RuleProfileGuard {
        RuleProfileGuard {
            since: self.since,
            until: self.until,
            requires_all: self.requires_all,
            requires_any: self.requires_any,
            forbids_any: self.forbids_any,
            modes: self.modes,
        }
    }
}

/// Entry point for the fluent guard builder.
pub fn profile_guard() -> ProfileGuardBuilder {
    ProfileGuardBuilder::default()
}

/// Alias for `profile_guard()`.
pub fn profile() -> ProfileGuardBuilder {
    ProfileGuardBuilder::default()
}

// ── RecoveryConfig ────────────────────────────────────────────────────────────

/// Recovery configuration for the parser engine.
///
/// Re-exported from `rb_common::recovery` so callers have a single canonical type
/// across both the tokenizer pipeline and the parser.
pub use rb_common::recovery::RecoveryConfig;

// ── ProfileCatalog ────────────────────────────────────────────────────────────

/// A registry that stores named [`ResolvedProfile`]s, resolves feature
/// combinations to profiles, and caches compiled parsers keyed by their
/// [`ResolvedProfileId`].
///
/// # Example
/// ```rust,ignore
/// let mut catalog = ProfileCatalog::new();
/// let json_profile = ResolvedProfile::simple("json");
/// let json_parser  = json_grammar.compile(&json_profile)?;
/// catalog.register(json_profile, json_parser);
///
/// if let Some(parser) = catalog.resolve("json", &[]) {
///     let tree = parser.parse_tree(&tokens, &mut ctx);
/// }
/// ```
pub struct ProfileCatalog {
    /// language → list of registered profiles for that language (different versions / flags).
    profiles: std::collections::HashMap<String, Vec<ResolvedProfile>>,
    /// Cached compiled parsers, keyed by profile identity hash.
    cache: std::collections::HashMap<ResolvedProfileId, std::sync::Arc<crate::CompiledParser>>,
}

impl ProfileCatalog {
    /// Create an empty catalog.
    pub fn new() -> Self {
        ProfileCatalog {
            profiles: std::collections::HashMap::new(),
            cache: std::collections::HashMap::new(),
        }
    }

    /// Register a (profile, compiled parser) pair.
    ///
    /// If a profile with the same [`ResolvedProfileId`] was already registered
    /// the existing entry is **replaced**.
    pub fn register(&mut self, profile: ResolvedProfile, parser: crate::CompiledParser) {
        let id = profile.id;
        let language = profile.language.to_owned();
        // Insert or replace in the per-language list.
        let list = self.profiles.entry(language).or_default();
        if let Some(pos) = list.iter().position(|p| p.id == id) {
            list[pos] = profile;
        } else {
            list.push(profile);
        }
        self.cache.insert(id, std::sync::Arc::new(parser));
    }

    /// Look up a compiled parser for the given language and feature flags.
    ///
    /// Returns the **first** registered profile for `language` whose
    /// `enabled_features` is a superset of the requested `features`. If no
    /// match is found, returns `None`.
    ///
    /// For deterministic behaviour, register profiles from most-specific to
    /// least-specific so the most-specific match is found first.
    pub fn resolve(
        &self,
        language: &str,
        features: &[FeatureFlag],
    ) -> Option<std::sync::Arc<crate::CompiledParser>> {
        let list = self.profiles.get(language)?;
        let matched = list.iter().find(|p| {
            features.iter().all(|f| p.enabled_features.contains(f))
        })?;
        self.cache.get(&matched.id).cloned()
    }

    /// Look up by exact [`ResolvedProfileId`].
    pub fn get_by_id(&self, id: ResolvedProfileId) -> Option<std::sync::Arc<crate::CompiledParser>> {
        self.cache.get(&id).cloned()
    }

    /// Returns the names of all languages registered in the catalog.
    pub fn languages(&self) -> impl Iterator<Item = &str> {
        self.profiles.keys().map(|s| s.as_str())
    }

    /// Returns all profiles registered for the given language.
    pub fn profiles_for(&self, language: &str) -> &[ResolvedProfile] {
        self.profiles.get(language).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

impl Default for ProfileCatalog {
    fn default() -> Self {
        Self::new()
    }
}
