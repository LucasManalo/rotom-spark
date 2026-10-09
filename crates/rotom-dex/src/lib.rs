//! Static Pokémon game data for Rotom Spark.
//!
//! Resolved Champions data will be introduced by the Showdown export milestone.

/// The crate's stable boundary name, useful for workspace smoke checks.
pub const CRATE_NAME: &str = "rotom-dex";

#[cfg(test)]
mod tests {
    #[test]
    fn crate_boundary_is_available() {
        assert_eq!(super::CRATE_NAME, "rotom-dex");
    }
}
