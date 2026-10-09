use rotom_core::{AbilityId, FormId, ItemId, MoveId, SideId, SpeciesId};

#[test]
fn foundational_types_are_available_from_the_crate_root() {
    assert_eq!(AbilityId::new(1).get(), 1);
    assert_eq!(FormId::new(2).get(), 2);
    assert_eq!(ItemId::new(3).get(), 3);
    assert_eq!(MoveId::new(4).get(), 4);
    assert_eq!(SpeciesId::new(5).get(), 5);
    assert_eq!(SideId::P1.opponent(), SideId::P2);
}
