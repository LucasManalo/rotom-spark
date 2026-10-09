//! Format rules and team-legality boundaries for Rotom Spark.
//!
//! Regulation M-C behavior is intentionally deferred to its dedicated milestone.

/// The crate's stable boundary name, useful for workspace smoke checks.
pub const CRATE_NAME: &str = "rotom-formats";

#[cfg(test)]
mod tests {
    #[test]
    fn crate_boundary_is_available() {
        assert_eq!(super::CRATE_NAME, "rotom-formats");
    }
}
