//! Normalized snapshot, replay, and protocol boundaries for Rotom Spark.
//!
//! Concrete protocol types will be added alongside the engine state they expose.

/// The crate's stable boundary name, useful for workspace smoke checks.
pub const CRATE_NAME: &str = "rotom-protocol";

#[cfg(test)]
mod tests {
    #[test]
    fn crate_boundary_is_available() {
        assert_eq!(super::CRATE_NAME, "rotom-protocol");
    }
}
