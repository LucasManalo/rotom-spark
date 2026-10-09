use rotom_core::{
    AbilityId, FormId, Gender, ItemId, MoveId, NatureId, PokemonSet, SideId, SpeciesId,
    StatAllocation, TeamDefinition,
};

#[test]
fn foundational_types_are_available_from_the_crate_root() {
    assert_eq!(AbilityId::new(1).get(), 1);
    assert_eq!(FormId::new(2).get(), 2);
    assert_eq!(ItemId::new(3).get(), 3);
    assert_eq!(MoveId::new(4).get(), 4);
    assert_eq!(NatureId::new(5).get(), 5);
    assert_eq!(SpeciesId::new(6).get(), 6);
    assert_eq!(SideId::P1.opponent(), SideId::P2);
    let _: Option<Gender> = None;
    let _: StatAllocation = StatAllocation::ZERO;
    let _: Option<PokemonSet> = None;
    let _: TeamDefinition = TeamDefinition {
        members: Vec::new(),
    };
}
