//! Purpose: Ordered model registry for tier-2 live trials.
//! Responsibilities: hold the checked-in ordered registry of model ids
//! with OpenRouter `:free` ids as the default first candidates, expose
//! them verbatim, and define the repetition policy per scenario × model
//! cell. The default first candidate is OpenRouter's free-models router
//! (`openrouter/free`), which routes to its current free catalog.
//! Rationale: evals-guidelines free-tier entry and attribution — the
//! ordering defines scheduling priority (which cells run first), not
//! failover; model fallback is a caller-level decision that must be
//! recorded. Ids are never normalized because matrix comparisons across
//! time depend on the raw id, including the `:free` suffix.

/// One registry entry: a raw model id, verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    /// The model id exactly as reported by the provider, including the
    /// `:free` suffix when applicable.
    pub id: &'static str,
    /// Whether this entry is an OpenRouter free-tier id. Free ids sort
    /// first in the registry ordering.
    pub free: bool,
}

/// The checked-in ordered registry. `:free` ids first (scheduling
/// priority), paid ids after. Nothing is special-cased beyond ordering.
pub const REGISTRY: &[RegistryEntry] = &[
    RegistryEntry {
        id: "openrouter/free",
        free: true,
    },
    RegistryEntry {
        id: "deepseek/deepseek-v4-flash:free",
        free: true,
    },
    RegistryEntry {
        id: "qwen/qwen3-coder:free",
        free: true,
    },
    RegistryEntry {
        id: "meta-llama/llama-3.3-70b-instruct:free",
        free: true,
    },
];

/// Default repetitions per scenario × model cell (n>=3 by policy).
pub const DEFAULT_REPETITIONS: u32 = 3;

/// Registry ids in scheduling order, verbatim.
pub fn ordered_ids() -> Vec<&'static str> {
    REGISTRY.iter().map(|e| e.id).collect()
}

/// The free-tier entries, in registry order.
pub fn free_ids() -> Vec<&'static str> {
    REGISTRY.iter().filter(|e| e.free).map(|e| e.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Free-tier ids sort first and their ids are preserved verbatim.
    /// Free-tier membership is the `free` flag, not the `:free` suffix:
    /// `openrouter/free` routes to OpenRouter's free catalog without a
    /// suffix.
    #[test]
    fn free_ids_first_and_verbatim() {
        let ids = ordered_ids();
        let free_count = free_ids().len();
        for (i, id) in ids.iter().take(free_count).enumerate() {
            assert!(
                free_ids().contains(&id),
                "first {free_count} ids must be free-tier, position {i} is {id}"
            );
        }
        assert_eq!(free_ids().first().copied(), Some("openrouter/free"));
    }

    /// Ids are never normalized: the registry round-trips raw strings.
    #[test]
    fn ids_verbatim_no_normalization() {
        for e in REGISTRY {
            assert_eq!(e.id, e.id.trim());
            assert!(!e.id.contains('*'), "no wildcard ids");
        }
    }

    /// Repetition policy: at least three per cell.
    #[test]
    fn default_repetitions_at_least_three() {
        assert!(DEFAULT_REPETITIONS >= 3);
    }
}
