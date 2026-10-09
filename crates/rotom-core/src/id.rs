use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id {
    ($name:ident) => {
        #[doc = concat!("Opaque identifier for a ", stringify!($name), " value.")]
        #[derive(
            Clone,
            Copy,
            Debug,
            Default,
            Deserialize,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            Serialize,
        )]
        #[serde(transparent)]
        pub struct $name(u32);

        impl $name {
            /// Creates an identifier from its stable numeric representation.
            #[must_use]
            pub const fn new(value: u32) -> Self {
                Self(value)
            }

            /// Returns the identifier's stable numeric representation.
            #[must_use]
            pub const fn get(self) -> u32 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

define_id!(AbilityId);
define_id!(ConditionId);
define_id!(FormId);
define_id!(ItemId);
define_id!(MoveId);
define_id!(NatureId);
define_id!(SpeciesId);

#[cfg(test)]
mod tests {
    use super::SpeciesId;

    #[test]
    fn id_round_trips_through_serde_json() {
        let id = SpeciesId::new(25);
        let encoded = serde_json::to_string(&id).expect("test value should serialize");
        let decoded: SpeciesId =
            serde_json::from_str(&encoded).expect("serialized test value should deserialize");

        assert_eq!(decoded, id);
        assert_eq!(encoded, "25");
    }
}
