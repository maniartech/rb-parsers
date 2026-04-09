// ── LanguageVersion ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageVersion {
    pub major: u32,
    pub minor: u32,
}

impl LanguageVersion {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageVersionError {
    Empty,
    InvalidNumber,
}

// ── ProfileMode ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProfileMode {
    #[default]
    Default,
    Strict,
    Tolerant,
    Legacy,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureFlag(pub &'static str);

impl FeatureFlag {
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
    pub id: ResolvedProfileId,
    pub language: &'static str,
    pub version: LanguageVersion,
    pub mode: ProfileMode,
    /// Sorted list of enabled feature flags.
    pub enabled_features: Vec<FeatureFlag>,
}

impl ResolvedProfile {
    pub fn has_feature(&self, flag: FeatureFlag) -> bool {
        self.enabled_features.contains(&flag)
    }

    pub fn is_at_least(&self, version: LanguageVersion) -> bool {
        self.version >= version
    }

    pub fn is_before(&self, version: LanguageVersion) -> bool {
        self.version < version
    }

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
#[derive(Debug, Clone)]
pub struct RuleProfileGuard {
    pub since: Option<LanguageVersion>,
    pub until: Option<LanguageVersion>,
    pub requires_all: Vec<FeatureFlag>,
    pub requires_any: Vec<FeatureFlag>,
    pub forbids_any: Vec<FeatureFlag>,
    pub modes: Vec<ProfileMode>,
}

impl RuleProfileGuard {
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
    pub fn since(mut self, version: &'static str) -> Self {
        self.since = LanguageVersion::parse(version).ok();
        self
    }

    pub fn until(mut self, version: &'static str) -> Self {
        self.until = LanguageVersion::parse(version).ok();
        self
    }

    pub fn feature(mut self, flag: &'static str) -> Self {
        self.requires_all.push(FeatureFlag(flag));
        self
    }

    pub fn any_feature(mut self, flags: &[&'static str]) -> Self {
        self.requires_any.extend(flags.iter().map(|s| FeatureFlag(s)));
        self
    }

    pub fn mode(mut self, mode: ProfileMode) -> Self {
        self.modes.push(mode);
        self
    }

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

/// Controls how aggressively the engine pursues error recovery.
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub max_recovery_steps: usize,
    pub warn_on_limit: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        RecoveryConfig { max_recovery_steps: 20, warn_on_limit: true }
    }
}
