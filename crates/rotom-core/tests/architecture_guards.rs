//! Source-level guards for the canonical mechanics and transition pathways.
//!
//! These intentionally protect narrow architectural seams that can otherwise
//! be bypassed accidentally while adding large identity batches.

const COMBAT: &str = include_str!("../src/combat.rs");
const STATE_TRANSITION: &str = include_str!("../src/state/transition.rs");
const ARCHITECTURE: &str = include_str!("../../../docs/architecture/state-transition.md");

fn production_source(source: &str) -> &str {
    source.split("#[cfg(test)]").next().unwrap_or(source)
}

fn contains_direct_assignment(source: &str, pattern: &str) -> bool {
    source
        .match_indices(pattern)
        .any(|(index, _)| source.as_bytes().get(index + pattern.len()).copied() != Some(b'='))
}

#[test]
fn queued_moves_and_switches_keep_their_canonical_entry_points() {
    assert_eq!(
        COMBAT.matches("fn execute_move<").count(),
        1,
        "there must be exactly one canonical queued-move executor"
    );
    assert_eq!(
        COMBAT.matches("self.execute_move(").count(),
        1,
        "the action queue must invoke the canonical executor exactly once"
    );
    assert!(
        COMBAT.contains("StateKernel::new(self).switch_active_position"),
        "resolved switches must use the atomic kernel position operation"
    );
    assert!(
        STATE_TRANSITION.contains("StateKernel::new(self).switch_active_position"),
        "initial selection must use the same atomic kernel position operation"
    );
}

#[test]
fn effect_modules_cannot_bypass_kernel_or_transition_orchestration() {
    let modules = [
        ("abilities.rs", include_str!("../src/abilities.rs")),
        ("items.rs", include_str!("../src/items.rs")),
        ("conditions.rs", include_str!("../src/conditions.rs")),
        ("field_effects.rs", include_str!("../src/field_effects.rs")),
    ];
    let forbidden = [
        ".current_hp =",
        ".item =",
        ".ability =",
        ".ability_restore =",
        ".status =",
        ".phase =",
        ".turn =",
        ".queue.actions",
        ".queue.events",
    ];

    for (name, source) in modules {
        let production = production_source(source);
        for pattern in forbidden {
            assert!(
                !contains_direct_assignment(production, pattern),
                "{name} bypasses an authoritative pathway with `{pattern}`"
            );
        }
    }
}

#[test]
fn transition_orchestration_exception_is_explicit_and_narrow() {
    assert!(ARCHITECTURE
        .contains("The transition controller is the narrow orchestration exception to kernel"));
    assert!(ARCHITECTURE
        .contains("mechanics and event handlers decide why and when gameplay facts change"));
    assert!(ARCHITECTURE
        .contains("the transition controller advances phase/turn and manages its private queue"));
}
