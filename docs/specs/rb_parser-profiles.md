# Spec: Parsing Profiles and Language Modes

**Status**: Ready for implementation
**Module**: `rb_parser::profiles`
**Depends on**: nothing (`rb_common` crate-level, no span or diagnostic dependency)
**Requirement source**: `docs/requirements/parsing-profiles-and-language-modes.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Profile vs runtime policy | Explicitly separated. `ResolvedProfile` carries language meaning (version, mode, features). `ResolvedRuntime` carries execution policy (recovery, diagnostics mode, renderer). They are assembled into `ParseSessionConfig` before parsing starts. |
| Stable profile identity | `ResolvedProfileId` is a deterministic hash (FNV-32) of the normalized `(language, version, mode, sorted_enabled_features)` tuple. Runtime settings do not contribute to the hash. |
| Compatibility model | Directional. `ProfileCompatibility` has six variants. Unknown profiles are treated as not safely substitutable by default. |
| Version format | `LanguageVersion { major: u32, minor: u32 }`. Display form is `"major.minor"`. Parsing from `&str` supported via `LanguageVersion::parse`. |
| Feature flags | `FeatureFlag(&'static str)`. Empty string is rejected at runtime. |
| Profile catalog | `ProfileCatalog` is a registry owned by the language author. `rb_parser` ships with no built-in language profiles; each language crate registers its own. |
| Safe defaults | Exact-same identity is compatible with itself. All other relationships are `Unknown` unless declared. `Unknown` means not safely substitutable. |
| Compatibility check timing | Compatibility is validated during `ProfileCatalog::resolve()` — before tokenizer or parser work begins. |

---

## Module Layout

```
rb_parser::profiles
├── LanguageVersion
├── ProfileMode
├── FeatureFlag
├── ProfileRequest
├── ResolvedProfile
├── ResolvedProfileId
├── ParseRuntimeRequest
├── ResolvedRuntime
├── ParseSessionConfig
├── RuleProfileGuard
├── ProfileCompatibility
├── ProfileCompatibilityRule
├── ProfileSelector
├── ProfileCatalog
└── ProfileCatalogBuilder
```

---

## Types

### LanguageVersion

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageVersion {
    pub major: u32,
    pub minor: u32,
}

impl LanguageVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        LanguageVersion { major, minor }
    }

    /// Parse from `"major"` or `"major.minor"`.
    pub fn parse(s: &str) -> Result<Self, LanguageVersionError> {
        let mut parts = s.splitn(2, '.');
        let major = parts.next()
            .ok_or(LanguageVersionError::Empty)?
            .parse::<u32>()
            .map_err(|_| LanguageVersionError::InvalidNumber)?;
        let minor = parts.next()
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
```

---

### ProfileMode

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileMode {
    /// Normal mode — sensible defaults for typical usage.
    Default,
    /// Reject any syntax not explicitly permitted by the language spec.
    Strict,
    /// Accept more inputs than `Default`; emit warnings for ambiguous constructs.
    Tolerant,
    /// Compatibility mode for older toolchain versions.
    Legacy,
    /// Named custom mode declared by the language author.
    Custom(&'static str),
}

impl Default for ProfileMode {
    fn default() -> Self { ProfileMode::Default }
}

impl std::fmt::Display for ProfileMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileMode::Default       => write!(f, "default"),
            ProfileMode::Strict        => write!(f, "strict"),
            ProfileMode::Tolerant      => write!(f, "tolerant"),
            ProfileMode::Legacy        => write!(f, "legacy"),
            ProfileMode::Custom(name)  => write!(f, "custom({name})"),
        }
    }
}
```

---

### FeatureFlag

```rust
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
```

---

### ProfileRequest

The caller's desired profile. All fields except `language` are optional; the
catalog fills in defaults from the registered base profile.

```rust
pub struct ProfileRequest<'a> {
    /// Language identifier (e.g. `"json"`, `"expr"`). Case-sensitive.
    pub language: &'static str,
    /// Desired version. `None` → use the catalog's default version for the language.
    pub version: Option<LanguageVersion>,
    /// Desired mode. `None` → `ProfileMode::Default`.
    pub mode: Option<ProfileMode>,
    /// Feature flags to enable on top of the base profile.
    pub enable_features: &'a [FeatureFlag],
    /// Feature flags to forcibly disable.
    pub disable_features: &'a [FeatureFlag],
}

impl<'a> ProfileRequest<'a> {
    pub fn for_language(language: &'static str) -> Self {
        ProfileRequest {
            language,
            version: None,
            mode: None,
            enable_features: &[],
            disable_features: &[],
        }
    }
}
```

---

### ResolvedProfileId

A deterministic, hashable identity for a resolved profile. Suitable as a
cache key for compiled grammar state.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolvedProfileId(pub u64);

impl ResolvedProfileId {
    /// Compute the ID for a normalized profile. Uses FNV-64.
    pub fn compute(
        language: &str,
        version: LanguageVersion,
        mode: ProfileMode,
        features: &[FeatureFlag],  // must be sorted
    ) -> Self {
        use std::hash::{Hash, Hasher};
        let mut h = FnvHasher::default();
        language.hash(&mut h);
        version.hash(&mut h);
        mode.hash(&mut h);
        for f in features { f.hash(&mut h); }
        ResolvedProfileId(h.finish())
    }
}
```

---

### ResolvedProfile

The immutable, fully-determined language profile produced by
`ProfileCatalog::resolve()`. Handed to `Grammar::compile()` and to each
`ParseContext`.

```rust
#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    /// Stable cache key.
    pub id: ResolvedProfileId,
    /// Language identifier.
    pub language: &'static str,
    /// Resolved version.
    pub version: LanguageVersion,
    /// Resolved mode.
    pub mode: ProfileMode,
    /// Complete sorted list of enabled feature flags after applying
    /// enable/disable overlays.
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
}
```

---

### RuleProfileGuard

Attached to a grammar rule via `.enabled_if(guard)`. Evaluated during parsing
to determine whether the rule branch is active for the current profile.

```rust
#[derive(Debug, Clone)]
pub struct RuleProfileGuard {
    /// Inclusive lower bound. `None` = no lower bound.
    pub since: Option<LanguageVersion>,
    /// Exclusive upper bound. `None` = no upper bound.
    pub until: Option<LanguageVersion>,
    /// All of these feature flags must be enabled.
    pub requires_all: Vec<FeatureFlag>,
    /// At least one of these feature flags must be enabled. Empty = no constraint.
    pub requires_any: Vec<FeatureFlag>,
    /// None of these feature flags may be enabled.
    pub forbids_any: Vec<FeatureFlag>,
    /// If non-empty, the profile mode must appear in this list.
    pub modes: Vec<ProfileMode>,
}

impl RuleProfileGuard {
    /// Returns `true` when this guard allows the rule to run.
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
```

---

### ParseRuntimeRequest / ResolvedRuntime

Runtime execution policy, separated from language identity.

```rust
pub struct ParseRuntimeRequest {
    /// `None` → default from the language profile registration.
    pub recovery_mode: Option<RecoveryMode>,
    /// `None` → no hard cap. Some(n) stops parsing after n errors.
    pub max_errors: Option<usize>,
    /// `None` → `DiagnosticsMode::Collect`.
    pub diagnostics_mode: Option<DiagnosticsMode>,
}

pub struct ResolvedRuntime {
    pub recovery: RecoveryConfig,
    pub max_errors: Option<usize>,
    pub diagnostics_mode: DiagnosticsMode,
}
```

---

### ParseSessionConfig

Bundles a resolved profile and resolved runtime together before a parse
session begins.

```rust
pub struct ParseSessionConfig {
    pub profile: ResolvedProfile,
    pub runtime: ResolvedRuntime,
}
```

---

### ProfileCompatibility

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileCompatibility {
    /// Both profiles are meaningfully interchangeable.
    Equivalent,
    /// The left profile is a stricter or narrower form of the right.
    Refines,
    /// The left profile adds capabilities beyond the right.
    Extends,
    /// The two profile fragments may be composed but are not interchangeable.
    MergeAllowed,
    /// These profiles must not be combined or substituted.
    Incompatible,
    /// No compatibility rule has been established.
    Unknown,
}
```

---

### ProfileCompatibilityRule

A declared relationship between two profile selectors.

```rust
pub struct ProfileCompatibilityRule {
    /// The left-hand profile selector.
    pub left: ProfileSelector,
    /// The right-hand profile selector.
    pub right: ProfileSelector,
    /// Declared relationship. Directional when `directional = true`.
    pub relation: ProfileCompatibility,
    /// When `true`, the relation applies left→right but not right→left.
    /// E.g. "v1+strict Refines v1" is directional: v1 does not refine v1+strict.
    pub directional: bool,
    /// Human-readable reason for documentation and diagnostics.
    pub reason: &'static str,
}
```

---

### ProfileSelector

A pattern-match descriptor used in compatibility rules.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileSelector {
    /// Matches exactly one resolved profile by ID.
    ById(ResolvedProfileId),
    /// Matches any profile for the given language, version and mode.
    ByParams {
        language: &'static str,
        version: Option<LanguageVersion>,
        mode: Option<ProfileMode>,
    },
    /// Matches any profile that has the given feature flag enabled.
    HasFeature(FeatureFlag),
}

impl ProfileSelector {
    pub fn matches(&self, profile: &ResolvedProfile) -> bool {
        match self {
            Self::ById(id) => profile.id == *id,
            Self::ByParams { language, version, mode } => {
                profile.language == *language
                    && version.map_or(true, |v| profile.version == v)
                    && mode.map_or(true, |m| profile.mode == m)
            }
            Self::HasFeature(f) => profile.has_feature(*f),
        }
    }
}
```

---

### ProfileCatalog

Registered by each language crate. Contains base profiles and their
compatibility rules.

```rust
pub struct ProfileCatalog {
    bases:         Vec<BaseLanguageProfile>,
    rules:         Vec<ProfileCompatibilityRule>,
}

/// A base profile registration — the anchor definition for one language.
pub struct BaseLanguageProfile {
    pub language:           &'static str,
    pub default_version:    LanguageVersion,
    pub supported_versions: Vec<LanguageVersion>,
    pub default_features:   Vec<FeatureFlag>,
    pub available_features: Vec<FeatureFlag>,
}

impl ProfileCatalog {
    pub fn builder() -> ProfileCatalogBuilder { ProfileCatalogBuilder::new() }

    /// Resolve a `ProfileRequest` into a stable `ResolvedProfile`.
    ///
    /// Resolution order (7 steps):
    /// 1. Find the `BaseLanguageProfile` for `request.language`.
    ///    Error if not found: `ProfileResolutionError::UnknownLanguage`.
    /// 2. Apply version defaults (use `BaseLanguageProfile::default_version`
    ///    if `request.version` is `None`).
    /// 3. Validate the requested version is in `supported_versions`.
    ///    Error if missing: `ProfileResolutionError::UnsupportedVersion`.
    /// 4. Apply mode defaults (`ProfileMode::Default` if `None`).
    /// 5. Compute `enabled_features`:
    ///    a. Start from `default_features`.
    ///    b. Add all `enable_features` from the request.
    ///    c. Remove all `disable_features` from the request.
    ///    d. Validate that all added features are in `available_features`.
    ///       Error if unknown: `ProfileResolutionError::UnknownFeature`.
    /// 6. Evaluate all registered `ProfileCompatibilityRule`s for the
    ///    candidate profile. Any `Incompatible` rule fails resolution with
    ///    `ProfileResolutionError::IncompatibleCombination`.
    /// 7. Sort `enabled_features` and compute the stable `ResolvedProfileId`.
    ///    Return `ResolvedProfile`.
    pub fn resolve(
        &self,
        request: &ProfileRequest<'_>,
    ) -> Result<ResolvedProfile, ProfileResolutionError>;

    /// Check the compatibility relationship between two already-resolved profiles.
    /// Returns `ProfileCompatibility::Unknown` if no rule covers the pair.
    pub fn compatibility(
        &self,
        left: &ResolvedProfile,
        right: &ResolvedProfile,
    ) -> ProfileCompatibility;
}
```

---

### ProfileResolutionError

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileResolutionError {
    UnknownLanguage { language: String },
    UnsupportedVersion { language: String, version: LanguageVersion },
    UnknownFeature { language: String, feature: String },
    IncompatibleCombination {
        language: String,
        reason: String,
    },
}

impl std::fmt::Display for ProfileResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownLanguage { language } =>
                write!(f, "no profile registered for language `{language}`"),
            Self::UnsupportedVersion { language, version } =>
                write!(f, "language `{language}` does not support version `{version}`"),
            Self::UnknownFeature { language, feature } =>
                write!(f, "feature `{feature}` is not available for language `{language}`"),
            Self::IncompatibleCombination { language, reason } =>
                write!(f, "incompatible profile combination for `{language}`: {reason}"),
        }
    }
}
```

---

### ProfileCatalogBuilder

```rust
pub struct ProfileCatalogBuilder { /* opaque */ }

impl ProfileCatalogBuilder {
    pub fn new() -> Self;

    /// Register a base language profile.
    pub fn language(self, profile: BaseLanguageProfile) -> Self;

    /// Declare a compatibility rule.
    pub fn rule(self, rule: ProfileCompatibilityRule) -> Self;

    pub fn build(self) -> ProfileCatalog;
}
```

---

## Compatibility Model Examples

```rust
// json/v1+strict Refines json/v1  (directional)
ProfileCompatibilityRule {
    left: ProfileSelector::ByParams {
        language: "json",
        version: Some(LanguageVersion::new(1, 0)),
        mode: Some(ProfileMode::Strict),
    },
    right: ProfileSelector::ByParams {
        language: "json",
        version: Some(LanguageVersion::new(1, 0)),
        mode: Some(ProfileMode::Default),
    },
    relation: ProfileCompatibility::Refines,
    directional: true,
    reason: "json/v1+strict rejects a subset of constructs permitted by json/v1",
}

// json/v2 MergeAllowed with "comments" feature  (non-directional)
ProfileCompatibilityRule {
    left: ProfileSelector::ByParams {
        language: "json", version: Some(LanguageVersion::new(2, 0)), mode: None,
    },
    right: ProfileSelector::HasFeature(FeatureFlag("comments")),
    relation: ProfileCompatibility::MergeAllowed,
    directional: false,
    reason: "comments can be composed with json/v2 but are not part of the base language",
}

// strict Incompatible with legacy_compat
ProfileCompatibilityRule {
    left:  ProfileSelector::ByParams { language: "json", version: None, mode: Some(ProfileMode::Strict)  },
    right: ProfileSelector::HasFeature(FeatureFlag("legacy_compat")),
    relation: ProfileCompatibility::Incompatible,
    directional: false,
    reason: "strict mode rejects legacy_compat constructs by design",
}
```

---

## Usage Example

```rust
let catalog = ProfileCatalog::builder()
    .language(BaseLanguageProfile {
        language:           "json",
        default_version:    LanguageVersion::new(1, 0),
        supported_versions: vec![
            LanguageVersion::new(1, 0),
            LanguageVersion::new(2, 0),
        ],
        default_features:   vec![],
        available_features: vec![
            FeatureFlag("comments"),
            FeatureFlag("trailing_commas"),
            FeatureFlag("legacy_compat"),
        ],
    })
    .build();

let request = ProfileRequest {
    language: "json",
    version: Some(LanguageVersion::new(2, 0)),
    mode: Some(ProfileMode::Strict),
    enable_features: &[FeatureFlag("trailing_commas")],
    disable_features: &[],
};

let profile = catalog.resolve(&request)?;
let parser = grammar.compile(&profile)?;
```
