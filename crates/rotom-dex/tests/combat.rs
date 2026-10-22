use rotom_core::{
    calculate_move_damage, step, ActionTarget, ActiveSlot, BattleEvent, BattlePhase, BattleStat,
    BattleState, BattleStateBuilder, CombatData, CombatMove, ConditionState, EffectFraction,
    EffectInvocation, EffectOrder, EffectRegistry, EffectSource, EffectState, EventContext,
    EventDispatch, FaintTarget, FormId, Gender, JointAction, MajorStatus, Mechanics,
    MoveCapabilities, MoveCapability, MoveId, MoveOperation, MoveSlotIndex, MoveTarget,
    PokemonAction, PokemonIndex, PokemonPosition, PokemonRef, PokemonSet, PokemonTypes,
    ResolvedAction, RngOperation, RngReason, SideId, SideState, SpeciesId, StatAllocation,
    StatStages, TargetResult, TeamState, TurnChoice,
};
use rotom_dex::champions_reg_mc;
use serde::Deserialize;

const SHOWDOWN_COMMIT: &str = "b79ce06d06b8f0dc8de8bb348bc9cab5d931553f";

struct MoveFractionOverride<'a> {
    dex: &'a rotom_dex::DexData,
    move_id: MoveId,
    drain: Option<EffectFraction>,
    recoil: Option<EffectFraction>,
}

impl CombatData for MoveFractionOverride<'_> {
    fn move_data(&self, move_id: MoveId) -> Option<CombatMove> {
        let mut data = CombatData::move_data(self.dex, move_id)?;
        if move_id == self.move_id {
            data.drain = self.drain;
            data.recoil = self.recoil;
        }
        Some(data)
    }

    fn pokemon_types(&self, species: SpeciesId, form: Option<FormId>) -> Option<PokemonTypes> {
        self.dex.pokemon_types(species, form)
    }

    fn ability_has_behavior(
        &self,
        ability: rotom_core::AbilityId,
        behavior: rotom_core::AbilityBehavior,
    ) -> bool {
        self.dex.ability_has_behavior(ability, behavior)
    }

    fn ability_handlers(&self) -> Vec<rotom_core::AbilityHandler> {
        self.dex.ability_handlers()
    }

    fn type_modifier(
        &self,
        attacking: rotom_core::PokemonType,
        defending: rotom_core::PokemonType,
    ) -> Option<i8> {
        self.dex.type_modifier(attacking, defending)
    }

    fn move_intrinsic(&self, move_id: MoveId) -> rotom_core::MoveIntrinsic {
        self.dex.move_intrinsic(move_id)
    }

    fn primary_move_effects(&self, move_id: MoveId) -> Vec<rotom_core::PrimaryMoveEffect> {
        self.dex.primary_move_effects(move_id)
    }

    fn secondary_move_effects(&self, move_id: MoveId) -> Vec<rotom_core::SecondaryMoveEffect> {
        self.dex.secondary_move_effects(move_id)
    }

    fn self_move_effect(&self, move_id: MoveId) -> Option<rotom_core::SelfMoveEffect> {
        self.dex.self_move_effect(move_id)
    }

    fn move_is_protectable(&self, move_id: MoveId) -> bool {
        self.dex.move_is_protectable(move_id)
    }

    fn move_breaks_protect(&self, move_id: MoveId) -> bool {
        self.dex.move_breaks_protect(move_id)
    }

    fn field_effect_ids(&self) -> rotom_core::FieldEffectIds {
        self.dex.field_effect_ids()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn prevent_status(
    _: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), rotom_core::BattleError> {
    dispatch.cancel();
    Ok(())
}

fn doubles_targeting_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/doubles-targeting.json"
    ))
    .expect("doubles targeting fixture")
}

fn primary_non_damage_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/primary-non-damage.json"
    ))
    .expect("primary non-damage fixture")
}

fn sequenced_effects_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/sequenced-effects.json"
    ))
    .expect("sequenced effects fixture")
}

fn single_move_action(move_slot: u8, target: ActionTarget) -> JointAction {
    JointAction::Turn {
        p1: TurnChoice {
            actions: [
                PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(move_slot),
                    target,
                    mega: false,
                },
                PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(0),
                    target: ActionTarget::Automatic,
                    mega: false,
                },
            ],
        },
        p2: TurnChoice {
            actions: [
                PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(0),
                    target: ActionTarget::Automatic,
                    mega: false,
                },
                PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(0),
                    target: ActionTarget::Automatic,
                    mega: false,
                },
            ],
        },
    }
}

fn selected_primary_battle(
    dex: &rotom_dex::DexData,
    source: &PokemonSet,
    target: &PokemonSet,
    seed: u64,
) -> BattleState {
    let filler = set_with_moves(dex, "magikarp", "swiftswim", &["swordsdance"]);
    let p1 = vec![
        source.clone(),
        filler.clone(),
        filler.clone(),
        filler.clone(),
    ];
    let p2 = vec![target.clone(), filler.clone(), filler.clone(), filler];
    selected_battle_for_teams(dex, &p1, &p2, seed)
}

fn execute_single_move(
    dex: &rotom_dex::DexData,
    state: &BattleState,
    move_slot: u8,
    target: ActionTarget,
) -> rotom_core::StepResult {
    step(
        state,
        &single_move_action(move_slot, target),
        &Mechanics::new(dex, &EffectRegistry::new()),
    )
    .expect("supported primary non-damage move")
}

#[test]
#[allow(clippy::too_many_lines)]
fn primary_non_damage_operations_share_the_canonical_executor() {
    let dex = champions_reg_mc().unwrap();
    let p1 = set_with_moves(
        &dex,
        "charizard",
        "blaze",
        &["swordsdance", "recover", "willowisp", "sunnyday"],
    );
    let p2 = set_with_moves(&dex, "blastoise", "torrent", &["swordsdance"]);
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = |slot, target| PokemonAction::Move {
        move_slot: MoveSlotIndex::new(slot),
        target,
        mega: false,
    };
    let passive = action(0, ActionTarget::Automatic);

    let initial = selected_battle_for_sets(&dex, &p1, &p2, 41, false);
    let mut builder = BattleStateBuilder::from_state(initial).unwrap();
    builder
        .edit_pokemon(
            PokemonRef::new(SideId::P1, PokemonIndex::new(1)),
            |pokemon| {
                pokemon.current_hp -= 20;
            },
        )
        .unwrap();
    let battle = builder.build().unwrap();
    let result = step(
        &battle,
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    action(0, ActionTarget::Automatic),
                    action(1, ActionTarget::Automatic),
                ],
            },
            p2: TurnChoice {
                actions: [passive, passive],
            },
        },
        &mechanics,
    )
    .unwrap();
    assert_eq!(
        result
            .state
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .stat_stages
            .get(BattleStat::Attack),
        2
    );
    let healed = result
        .state
        .side(SideId::P1)
        .pokemon(PokemonIndex::new(1))
        .unwrap();
    assert_eq!(healed.current_hp, healed.stats.hp);

    let result = step(
        &selected_battle_for_sets(&dex, &p1, &p2, 42, false),
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    action(2, ActionTarget::Foe(ActiveSlot::Left)),
                    action(0, ActionTarget::Automatic),
                ],
            },
            p2: TurnChoice {
                actions: [passive, passive],
            },
        },
        &mechanics,
    )
    .unwrap();
    assert_eq!(
        result
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .status,
        Some(MajorStatus::Burn)
    );

    let mut prevention = EffectRegistry::new();
    prevention.register(
        BattleEvent::TrySetStatus,
        EffectSource::Ability(dex.resolve_ability("torrent").unwrap().0),
        EffectOrder::default(),
        prevent_status,
    );
    let blocked = step(
        &selected_battle_for_sets(&dex, &p1, &p2, 44, false),
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    action(2, ActionTarget::Foe(ActiveSlot::Left)),
                    action(0, ActionTarget::Automatic),
                ],
            },
            p2: TurnChoice {
                actions: [passive, passive],
            },
        },
        &Mechanics::new(&dex, &prevention),
    )
    .unwrap();
    assert_eq!(
        blocked
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .status,
        None
    );

    let field_result = step(
        &selected_battle_for_sets(&dex, &p1, &p2, 43, false),
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    action(3, ActionTarget::Automatic),
                    action(0, ActionTarget::Automatic),
                ],
            },
            p2: TurnChoice {
                actions: [passive, passive],
            },
        },
        &mechanics,
    )
    .unwrap();
    assert_eq!(
        field_result.state.field().weather.unwrap().id,
        dex.field_effect_ids().sun.unwrap()
    );
    assert_eq!(
        field_result.state.field().weather.unwrap().turns_remaining,
        Some(4)
    );
}

#[test]
fn healing_uses_showdown_rounding_and_full_hp_fails() {
    let dex = champions_reg_mc().unwrap();
    let healer = set_with_moves(&dex, "goodra", "sapsipper", &["recover"]);
    let foe = set_with_moves(&dex, "magikarp", "swiftswim", &["swordsdance"]);
    let state = selected_primary_battle(&dex, &healer, &foe, 0x1331);
    let mut builder = BattleStateBuilder::from_state(state).unwrap();
    builder
        .edit_pokemon(
            PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
            |pokemon| {
                pokemon.stats.hp = 155;
                pokemon.current_hp = 50;
            },
        )
        .unwrap();
    let recovery = execute_single_move(&dex, &builder.build().unwrap(), 0, ActionTarget::Automatic);
    assert_eq!(
        recovery
            .state
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .current_hp,
        128,
        "Gen 9 Showdown rounds 155 / 2 to 78 HP"
    );

    let full = selected_primary_battle(&dex, &healer, &foe, 0x1332);
    let unchanged = execute_single_move(&dex, &full, 0, ActionTarget::Automatic);
    let outcome = unchanged
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .find(|outcome| {
            outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0))
                && outcome.move_id == dex.resolve_move("recover").unwrap().0
        })
        .unwrap();
    assert_eq!(outcome.result, TargetResult::Cancelled);
}

#[test]
#[allow(clippy::too_many_lines)]
fn status_and_stat_moves_cover_success_immunity_prevention_miss_and_bounds() {
    let dex = champions_reg_mc().unwrap();
    let source = set_with_moves(
        &dex,
        "goodra",
        "sapsipper",
        &["willowisp", "thunderwave", "hypnosis", "growl"],
    );
    let ordinary = set_with_moves(&dex, "blastoise", "torrent", &["sunnyday"]);
    let foe_target = ActionTarget::Foe(ActiveSlot::Left);

    let burned = execute_single_move(
        &dex,
        &selected_primary_battle(&dex, &source, &ordinary, 0x1333),
        0,
        foe_target,
    );
    assert_eq!(
        burned
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .status,
        Some(MajorStatus::Burn)
    );

    for (species, move_slot) in [("charizard", 0), ("pikachu", 1), ("garchomp", 1)] {
        let target = set_with_moves(&dex, species, "pressure", &["sunnyday"]);
        let result = execute_single_move(
            &dex,
            &selected_primary_battle(&dex, &source, &target, 0x1334),
            move_slot,
            foe_target,
        );
        assert_eq!(
            result
                .state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .status,
            None,
            "{species} is immune"
        );
    }

    let already = selected_primary_battle(&dex, &source, &ordinary, 0x1335);
    let mut builder = BattleStateBuilder::from_state(already).unwrap();
    builder
        .edit_pokemon(
            PokemonRef::new(SideId::P2, PokemonIndex::new(0)),
            |pokemon| pokemon.status = Some(MajorStatus::Paralysis),
        )
        .unwrap();
    let result = execute_single_move(&dex, &builder.build().unwrap(), 0, foe_target);
    assert_eq!(
        result
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .status,
        Some(MajorStatus::Paralysis)
    );

    let misty = dex.field_effect_ids().misty_terrain.unwrap();
    let terrain = selected_primary_battle(&dex, &source, &ordinary, 0x1336);
    let mut builder = BattleStateBuilder::from_state(terrain).unwrap();
    builder.edit_field(|field| {
        field.terrain = Some(ConditionState::new(misty, Some(5)));
    });
    let result = execute_single_move(&dex, &builder.build().unwrap(), 0, foe_target);
    assert_eq!(
        result
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .status,
        None
    );

    let protected_target = set_with_moves(&dex, "blastoise", "torrent", &["protect"]);
    let protected_state = selected_primary_battle(&dex, &source, &protected_target, 0x1337);
    let protected_action = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(0),
                    target: foe_target,
                    mega: false,
                },
                PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(0),
                    target: ActionTarget::Automatic,
                    mega: false,
                },
            ],
        },
        p2: TurnChoice {
            actions: [
                PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(0),
                    target: ActionTarget::Automatic,
                    mega: false,
                },
                PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(0),
                    target: ActionTarget::Automatic,
                    mega: false,
                },
            ],
        },
    };
    let protected = step(
        &protected_state,
        &protected_action,
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .unwrap();
    assert_eq!(
        protected
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .status,
        None
    );
    assert!(protected
        .diagnostics
        .turn
        .unwrap()
        .target_outcomes
        .iter()
        .any(|outcome| outcome.result == TargetResult::Protected));

    let mut missed = None;
    for seed in 0..10_000 {
        let state = selected_primary_battle(&dex, &source, &ordinary, seed);
        let result = execute_single_move(&dex, &state, 0, foe_target);
        if result
            .diagnostics
            .turn
            .as_ref()
            .unwrap()
            .target_outcomes
            .iter()
            .any(|outcome| outcome.result == TargetResult::Missed)
        {
            missed = Some(result);
            break;
        }
    }
    assert!(missed.is_some(), "Will-O-Wisp miss remains reachable");

    let asleep = (0..10_000)
        .map(|seed| {
            execute_single_move(
                &dex,
                &selected_primary_battle(&dex, &source, &ordinary, seed),
                2,
                foe_target,
            )
        })
        .find(|result| {
            result
                .state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .status
                == Some(MajorStatus::Sleep)
        })
        .expect("controlled Hypnosis hit");
    let sleeping = asleep
        .state
        .side(SideId::P2)
        .pokemon(PokemonIndex::new(0))
        .unwrap();
    assert_eq!(sleeping.status, Some(MajorStatus::Sleep));
    assert!((1..=3).contains(&sleeping.status_state.sleep_turns));
    assert!(asleep
        .state
        .rng()
        .trace()
        .unwrap()
        .iter()
        .any(|entry| entry.reason == RngReason::SleepDuration));

    let electric = dex.field_effect_ids().electric_terrain.unwrap();
    let electric_state = selected_primary_battle(&dex, &source, &ordinary, 0x1339);
    let mut builder = BattleStateBuilder::from_state(electric_state).unwrap();
    builder.edit_field(|field| {
        field.terrain = Some(ConditionState::new(electric, Some(5)));
    });
    let prevented = execute_single_move(&dex, &builder.build().unwrap(), 2, foe_target);
    assert_eq!(
        prevented
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .status,
        None
    );

    let stat_source = set_with_moves(&dex, "goodra", "sapsipper", &["swordsdance", "growl"]);
    for (initial, expected) in [(0, 2), (5, 6), (6, 6)] {
        let state = selected_primary_battle(&dex, &stat_source, &ordinary, 0x1340);
        let mut builder = BattleStateBuilder::from_state(state).unwrap();
        builder
            .edit_pokemon(
                PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
                |pokemon| pokemon.stat_stages.atk = initial,
            )
            .unwrap();
        let result =
            execute_single_move(&dex, &builder.build().unwrap(), 0, ActionTarget::Automatic);
        assert_eq!(
            result
                .state
                .side(SideId::P1)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .stat_stages
                .atk,
            expected
        );
    }
    for (initial, expected) in [(0, -1), (-5, -6), (-6, -6)] {
        let state = selected_primary_battle(&dex, &stat_source, &ordinary, 0x1341);
        let mut builder = BattleStateBuilder::from_state(state).unwrap();
        builder
            .edit_pokemon(
                PokemonRef::new(SideId::P2, PokemonIndex::new(0)),
                |pokemon| pokemon.stat_stages.atk = initial,
            )
            .unwrap();
        let result =
            execute_single_move(&dex, &builder.build().unwrap(), 1, ActionTarget::Automatic);
        assert_eq!(
            result
                .state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .stat_stages
                .atk,
            expected
        );
    }

    let combined_source = set_with_moves(&dex, "goodra", "sapsipper", &["nobleroar"]);
    let combined = execute_single_move(
        &dex,
        &selected_primary_battle(&dex, &combined_source, &ordinary, 0x1342),
        0,
        foe_target,
    );
    let combined_target = combined
        .state
        .side(SideId::P2)
        .pokemon(PokemonIndex::new(0))
        .unwrap();
    assert_eq!(combined_target.stat_stages.atk, -1);
    assert_eq!(combined_target.stat_stages.spa, -1);
}

#[test]
#[allow(clippy::too_many_lines)]
fn field_side_and_volatile_reapplication_matches_showdown_semantics() {
    let dex = champions_reg_mc().unwrap();
    let foe = set_with_moves(&dex, "blastoise", "torrent", &["swordsdance"]);
    let field_user = set_with_moves(
        &dex,
        "goodra",
        "sapsipper",
        &["sunnyday", "raindance", "electricterrain", "trickroom"],
    );

    let initial = selected_primary_battle(&dex, &field_user, &foe, 0x1350);
    let sun = execute_single_move(&dex, &initial, 0, ActionTarget::Automatic);
    let sun_id = dex.field_effect_ids().sun.unwrap();
    assert_eq!(sun.state.field().weather.unwrap().id, sun_id);
    assert_eq!(sun.state.field().weather.unwrap().turns_remaining, Some(4));
    let repeated_sun = execute_single_move(&dex, &sun.state, 0, ActionTarget::Automatic);
    assert_eq!(
        repeated_sun.state.field().weather.unwrap().turns_remaining,
        Some(3),
        "same weather fails instead of refreshing"
    );
    let rain = execute_single_move(&dex, &repeated_sun.state, 1, ActionTarget::Automatic);
    assert_eq!(
        rain.state.field().weather.unwrap().id,
        dex.field_effect_ids().rain.unwrap(),
        "a different weather replaces the old weather"
    );
    assert_eq!(rain.state.field().weather.unwrap().turns_remaining, Some(4));

    let terrain = execute_single_move(&dex, &initial, 2, ActionTarget::Automatic);
    let electric = dex.field_effect_ids().electric_terrain.unwrap();
    assert_eq!(terrain.state.field().terrain.unwrap().id, electric);
    assert_eq!(
        terrain.state.field().terrain.unwrap().turns_remaining,
        Some(4)
    );
    let repeated_terrain = execute_single_move(&dex, &terrain.state, 2, ActionTarget::Automatic);
    assert_eq!(
        repeated_terrain
            .state
            .field()
            .terrain
            .unwrap()
            .turns_remaining,
        Some(3)
    );

    let room = execute_single_move(&dex, &initial, 3, ActionTarget::Automatic);
    let trick_room = dex.field_effect_ids().trick_room.unwrap();
    assert!(room
        .state
        .field()
        .conditions
        .iter()
        .any(|condition| condition.id == trick_room));
    let toggled = execute_single_move(&dex, &room.state, 3, ActionTarget::Automatic);
    assert!(!toggled
        .state
        .field()
        .conditions
        .iter()
        .any(|condition| condition.id == trick_room));

    let condition_user = set_with_moves(
        &dex,
        "goodra",
        "sapsipper",
        &["reflect", "lightscreen", "tailwind", "magnetrise"],
    );
    for (slot, condition, initial_turns, repeated_turns) in [
        (0_u8, dex.field_effect_ids().reflect.unwrap(), 4, 3),
        (1_u8, dex.field_effect_ids().light_screen.unwrap(), 4, 3),
        (2_u8, dex.field_effect_ids().tailwind.unwrap(), 3, 2),
    ] {
        let state = selected_primary_battle(&dex, &condition_user, &foe, 0x1351 + u64::from(slot));
        let first = execute_single_move(&dex, &state, slot, ActionTarget::Automatic);
        assert_eq!(
            first
                .state
                .side(SideId::P1)
                .side_conditions
                .iter()
                .find(|state| state.id == condition)
                .unwrap()
                .turns_remaining,
            Some(initial_turns)
        );
        let repeated = execute_single_move(&dex, &first.state, slot, ActionTarget::Automatic);
        assert_eq!(
            repeated
                .state
                .side(SideId::P1)
                .side_conditions
                .iter()
                .find(|state| state.id == condition)
                .unwrap()
                .turns_remaining,
            Some(repeated_turns),
            "reapplication must not refresh the condition"
        );
    }

    let state = selected_primary_battle(&dex, &condition_user, &foe, 0x1355);
    let risen = execute_single_move(&dex, &state, 3, ActionTarget::Automatic);
    let magnet_rise = dex.field_effect_ids().magnet_rise.unwrap();
    assert_eq!(
        risen
            .state
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .volatiles
            .iter()
            .find(|state| state.id == magnet_rise)
            .unwrap()
            .turns_remaining,
        Some(4)
    );
    let repeated = execute_single_move(&dex, &risen.state, 3, ActionTarget::Automatic);
    assert_eq!(
        repeated
            .state
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .volatiles
            .iter()
            .find(|state| state.id == magnet_rise)
            .unwrap()
            .turns_remaining,
        Some(3)
    );

    let mut gravity_builder = BattleStateBuilder::from_state(state).unwrap();
    gravity_builder.edit_field(|field| {
        field.conditions.push(ConditionState::new(
            dex.field_effect_ids().gravity.unwrap(),
            Some(5),
        ));
    });
    let blocked = execute_single_move(
        &dex,
        &gravity_builder.build().unwrap(),
        3,
        ActionTarget::Automatic,
    );
    assert!(!blocked
        .state
        .side(SideId::P1)
        .pokemon(PokemonIndex::new(0))
        .unwrap()
        .has_volatile(magnet_rise));
}

fn fixture_integer(case: &serde_json::Value, key: &str) -> i64 {
    case["expected"][key]
        .as_i64()
        .unwrap_or_else(|| panic!("{} missing integer {key}", case["name"]))
}

fn fixture_bool(case: &serde_json::Value, key: &str) -> bool {
    case["expected"][key]
        .as_bool()
        .unwrap_or_else(|| panic!("{} missing boolean {key}", case["name"]))
}

fn primary_outcome(result: &rotom_core::StepResult, move_id: MoveId) -> TargetResult {
    result
        .diagnostics
        .turn
        .as_ref()
        .expect("turn diagnostics")
        .target_outcomes
        .iter()
        .find(|outcome| {
            outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0))
                && outcome.move_id == move_id
        })
        .unwrap_or_else(|| panic!("missing target outcome for move {}", move_id.get()))
        .result
}

#[test]
#[allow(clippy::too_many_lines)]
fn primary_non_damage_fixture_is_an_automated_differential_sweep() {
    let fixture = primary_non_damage_fixture();
    assert_eq!(fixture["showdown_commit"], SHOWDOWN_COMMIT);
    assert_eq!(fixture["format"], "gen9championsvgc2026regmc");
    assert_eq!(fixture["methodology"]["move_execution"], true);
    assert_eq!(fixture["methodology"]["full_transition_comparison"], false);
    assert_eq!(
        fixture["methodology"]["rng_comparison"],
        "semantic operation and result comparison; PRNG seed/state parity is not claimed"
    );

    let dex = champions_reg_mc().expect("generated Dex");
    let ordinary = set_with_moves(&dex, "blastoise", "torrent", &["recover"]);
    let foe_target = ActionTarget::Foe(ActiveSlot::Left);
    let cases = fixture["cases"].as_array().unwrap();
    let mut executed = Vec::new();

    for case in cases {
        let name = case["name"].as_str().expect("case name");
        executed.push(name);
        match name {
            "will-o-wisp-matrix" => {
                let source = set_with_moves(&dex, "goodra", "sapsipper", &["willowisp"]);
                let move_id = dex.resolve_move("willowisp").unwrap().0;
                let success = (0..10_000)
                    .map(|seed| {
                        execute_single_move(
                            &dex,
                            &selected_primary_battle(&dex, &source, &ordinary, seed),
                            0,
                            foe_target,
                        )
                    })
                    .find(|result| primary_outcome(result, move_id) == TargetResult::Hit)
                    .expect("controlled Will-O-Wisp hit");
                assert_eq!(
                    success
                        .state
                        .side(SideId::P2)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .status,
                    Some(MajorStatus::Burn)
                );
                assert_eq!(case["expected"]["success_status"], "brn");

                let target = set_with_moves(&dex, "charizard", "blaze", &["swordsdance"]);
                let result = (0..10_000)
                    .map(|seed| {
                        execute_single_move(
                            &dex,
                            &selected_primary_battle(&dex, &source, &target, seed),
                            0,
                            foe_target,
                        )
                    })
                    .find(|result| primary_outcome(result, move_id) == TargetResult::Cancelled)
                    .expect("controlled Will-O-Wisp hit rejected by Fire immunity");
                assert_eq!(
                    result
                        .state
                        .side(SideId::P2)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .status,
                    None
                );
                assert!(fixture_bool(case, "fire_immune"));

                let state = selected_primary_battle(&dex, &source, &ordinary, 0x3302);
                let mut builder = BattleStateBuilder::from_state(state).unwrap();
                builder
                    .edit_pokemon(
                        PokemonRef::new(SideId::P2, PokemonIndex::new(0)),
                        |pokemon| pokemon.status = Some(MajorStatus::Paralysis),
                    )
                    .unwrap();
                let already = execute_single_move(&dex, &builder.build().unwrap(), 0, foe_target);
                assert_eq!(
                    already
                        .state
                        .side(SideId::P2)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .status,
                    Some(MajorStatus::Paralysis)
                );
                assert_eq!(case["expected"]["already_statused"], "par");

                let state = selected_primary_battle(&dex, &source, &ordinary, 0x3303);
                let mut builder = BattleStateBuilder::from_state(state).unwrap();
                builder.edit_field(|field| {
                    field.terrain = Some(ConditionState::new(
                        dex.field_effect_ids().misty_terrain.unwrap(),
                        Some(5),
                    ));
                });
                let terrain = execute_single_move(&dex, &builder.build().unwrap(), 0, foe_target);
                assert_eq!(
                    terrain
                        .state
                        .side(SideId::P2)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .status,
                    None
                );
                assert!(fixture_bool(case, "terrain_prevented"));

                let missed = (0..10_000)
                    .map(|seed| {
                        execute_single_move(
                            &dex,
                            &selected_primary_battle(&dex, &source, &ordinary, seed),
                            0,
                            foe_target,
                        )
                    })
                    .find(|result| primary_outcome(result, move_id) == TargetResult::Missed)
                    .expect("controlled Will-O-Wisp miss");
                assert_eq!(primary_outcome(&missed, move_id), TargetResult::Missed);
                assert!(fixture_bool(case, "missed"));
            }
            "thunder-wave-immunities" => {
                let source = set_with_moves(&dex, "goodra", "sapsipper", &["thunderwave"]);
                let move_id = dex.resolve_move("thunderwave").unwrap().0;
                for (species, ability, key) in [
                    ("pikachu", "static", "electric_immune"),
                    ("garchomp", "sandveil", "ground_immune"),
                ] {
                    let target = set_with_moves(&dex, species, ability, &["swordsdance"]);
                    let result = (0..10_000)
                        .map(|seed| {
                            execute_single_move(
                                &dex,
                                &selected_primary_battle(&dex, &source, &target, seed),
                                0,
                                foe_target,
                            )
                        })
                        .find(|result| primary_outcome(result, move_id) == TargetResult::Cancelled)
                        .expect("controlled Thunder Wave hit rejected by status immunity");
                    assert_eq!(
                        result
                            .state
                            .side(SideId::P2)
                            .pokemon(PokemonIndex::new(0))
                            .unwrap()
                            .status,
                        None
                    );
                    assert!(fixture_bool(case, key));
                }
            }
            "hypnosis-sleep-rng" => {
                let source = set_with_moves(&dex, "goodra", "sapsipper", &["hypnosis"]);
                let move_id = dex.resolve_move("hypnosis").unwrap().0;
                let expected_turns = u8::try_from(fixture_integer(case, "sleep_turns")).unwrap();
                let asleep = (0..10_000)
                    .map(|seed| {
                        let state = selected_primary_battle(&dex, &source, &ordinary, seed);
                        let mut builder = BattleStateBuilder::from_state(state).unwrap();
                        builder
                            .edit_pokemon(
                                PokemonRef::new(SideId::P2, PokemonIndex::new(0)),
                                |pokemon| pokemon.stats.spe = u16::MAX,
                            )
                            .unwrap();
                        execute_single_move(&dex, &builder.build().unwrap(), 0, foe_target)
                    })
                    .find(|result| {
                        let pokemon = result
                            .state
                            .side(SideId::P2)
                            .pokemon(PokemonIndex::new(0))
                            .unwrap();
                        pokemon.status == Some(MajorStatus::Sleep)
                            && pokemon.status_state.sleep_turns == expected_turns
                    })
                    .expect("controlled Hypnosis hit and duration");
                let trace = asleep.state.rng().trace().unwrap();
                let relevant = trace
                    .iter()
                    .filter(|entry| {
                        matches!(entry.reason, RngReason::Accuracy | RngReason::SleepDuration)
                    })
                    .collect::<Vec<_>>();
                assert_eq!(relevant.len(), 2);
                assert_eq!(relevant[0].reason, RngReason::Accuracy);
                assert_eq!(
                    relevant[0].operation,
                    RngOperation::Chance {
                        numerator: 60,
                        denominator: 100,
                    }
                );
                assert!(relevant[0].result < 60);
                assert_eq!(relevant[1].reason, RngReason::SleepDuration);
                assert_eq!(relevant[1].operation, RngOperation::Sample { len: 3 });
                assert_eq!(
                    [2, 3, 3][usize::try_from(relevant[1].result).unwrap()],
                    expected_turns
                );
                assert_eq!(case["expected"]["status"], "slp");
                assert_eq!(case["expected"]["rng"][0]["reason"], "accuracy");
                assert_eq!(case["expected"]["rng"][1]["reason"], "sleep_duration");
                assert_eq!(primary_outcome(&asleep, move_id), TargetResult::Hit);

                let state = selected_primary_battle(&dex, &source, &ordinary, 0x3311);
                let mut builder = BattleStateBuilder::from_state(state).unwrap();
                builder.edit_field(|field| {
                    field.terrain = Some(ConditionState::new(
                        dex.field_effect_ids().electric_terrain.unwrap(),
                        Some(5),
                    ));
                });
                let prevented = execute_single_move(&dex, &builder.build().unwrap(), 0, foe_target);
                assert_eq!(
                    prevented
                        .state
                        .side(SideId::P2)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .status,
                    None
                );
                assert!(fixture_bool(case, "electric_terrain_prevented"));
            }
            "swords-dance-limits" | "growl-limits" => {
                let is_self = name == "swords-dance-limits";
                let move_name = if is_self { "swordsdance" } else { "growl" };
                let source = set_with_moves(&dex, "goodra", "sapsipper", &[move_name]);
                for (initial, key) in if is_self {
                    [(0, "from_0"), (5, "from_5"), (6, "from_6")]
                } else {
                    [(0, "from_0"), (-5, "from_-5"), (-6, "from_-6")]
                } {
                    let state = selected_primary_battle(&dex, &source, &ordinary, 0x3320);
                    let target = PokemonRef::new(
                        if is_self { SideId::P1 } else { SideId::P2 },
                        PokemonIndex::new(0),
                    );
                    let mut builder = BattleStateBuilder::from_state(state).unwrap();
                    builder
                        .edit_pokemon(target, |pokemon| pokemon.stat_stages.atk = initial)
                        .unwrap();
                    let result = execute_single_move(
                        &dex,
                        &builder.build().unwrap(),
                        0,
                        ActionTarget::Automatic,
                    );
                    assert_eq!(
                        i64::from(
                            result
                                .state
                                .side(target.side)
                                .pokemon(target.pokemon)
                                .unwrap()
                                .stat_stages
                                .atk
                        ),
                        fixture_integer(case, key)
                    );
                }
            }
            "noble-roar-combined" => {
                let source = set_with_moves(&dex, "goodra", "sapsipper", &["nobleroar"]);
                let result = execute_single_move(
                    &dex,
                    &selected_primary_battle(&dex, &source, &ordinary, 0x3330),
                    0,
                    foe_target,
                );
                let target = result
                    .state
                    .side(SideId::P2)
                    .pokemon(PokemonIndex::new(0))
                    .unwrap();
                assert_eq!(
                    i64::from(target.stat_stages.atk),
                    fixture_integer(case, "atk")
                );
                assert_eq!(
                    i64::from(target.stat_stages.spa),
                    fixture_integer(case, "spa")
                );
            }
            "recover-rounding" => {
                let source = set_with_moves(&dex, "goodra", "sapsipper", &["recover"]);
                for (max_hp, current_hp, key) in [
                    (None, None, "normal_healed"),
                    (Some(155), Some(50), "odd_max_hp_healed"),
                ] {
                    let state = selected_primary_battle(&dex, &source, &ordinary, 0x3340);
                    let mut builder = BattleStateBuilder::from_state(state).unwrap();
                    builder
                        .edit_pokemon(
                            PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
                            |pokemon| {
                                if let Some(max_hp) = max_hp {
                                    pokemon.stats.hp = max_hp;
                                }
                                pokemon.current_hp = current_hp
                                    .unwrap_or_else(|| pokemon.stats.hp.saturating_div(4));
                            },
                        )
                        .unwrap();
                    let state = builder.build().unwrap();
                    let before = state
                        .side(SideId::P1)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .current_hp;
                    let result = execute_single_move(&dex, &state, 0, ActionTarget::Automatic);
                    let after = result
                        .state
                        .side(SideId::P1)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .current_hp;
                    assert_eq!(i64::from(after - before), fixture_integer(case, key));
                }
                let state = selected_primary_battle(&dex, &source, &ordinary, 0x3341);
                let before = state
                    .side(SideId::P1)
                    .pokemon(PokemonIndex::new(0))
                    .unwrap()
                    .current_hp;
                let result = execute_single_move(&dex, &state, 0, ActionTarget::Automatic);
                assert_eq!(
                    result
                        .state
                        .side(SideId::P1)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .current_hp,
                    before
                );
                assert_eq!(
                    primary_outcome(&result, dex.resolve_move("recover").unwrap().0),
                    TargetResult::Cancelled
                );
                assert!(fixture_bool(case, "full_hp_failed"));
            }
            "sunny-day-reapplication" | "electric-terrain-reapplication" => {
                let is_weather = name == "sunny-day-reapplication";
                let moves: &[&str] = if is_weather {
                    &["sunnyday", "raindance"]
                } else {
                    &["electricterrain"]
                };
                let source = set_with_moves(&dex, "goodra", "sapsipper", moves);
                let state = selected_primary_battle(&dex, &source, &ordinary, 0x3350);
                let first = execute_single_move(&dex, &state, 0, ActionTarget::Automatic);
                let duration = if is_weather {
                    first
                        .state
                        .field()
                        .weather
                        .as_ref()
                        .unwrap()
                        .turns_remaining
                } else {
                    first
                        .state
                        .field()
                        .terrain
                        .as_ref()
                        .unwrap()
                        .turns_remaining
                };
                assert_eq!(
                    duration,
                    Some(u8::try_from(fixture_integer(case, "initial_duration") - 1).unwrap()),
                    "Showdown direct-use duration normalized through one Rotom residual boundary"
                );
                let repeated = execute_single_move(&dex, &first.state, 0, ActionTarget::Automatic);
                let repeated_duration = if is_weather {
                    repeated
                        .state
                        .field()
                        .weather
                        .as_ref()
                        .unwrap()
                        .turns_remaining
                } else {
                    repeated
                        .state
                        .field()
                        .terrain
                        .as_ref()
                        .unwrap()
                        .turns_remaining
                };
                assert_eq!(repeated_duration, duration.map(|value| value - 1));
                assert!(fixture_bool(case, "repeated_failed"));
                if is_weather {
                    let replaced =
                        execute_single_move(&dex, &repeated.state, 1, ActionTarget::Automatic);
                    assert_eq!(case["expected"]["replacement"], "raindance");
                    assert_eq!(
                        replaced.state.field().weather.as_ref().unwrap().id,
                        dex.field_effect_ids().rain.unwrap()
                    );
                }
            }
            "reflect-reapplication" | "light-screen-reapplication" | "tailwind-reapplication" => {
                let move_name = case["move"].as_str().unwrap();
                let source = set_with_moves(&dex, "goodra", "sapsipper", &[move_name]);
                let condition = dex.resolve_move(move_name).unwrap().0;
                let condition = rotom_core::ConditionId::new(condition.get());
                let state = selected_primary_battle(&dex, &source, &ordinary, 0x3360);
                let first = execute_single_move(&dex, &state, 0, ActionTarget::Automatic);
                let first_duration = first
                    .state
                    .side(SideId::P1)
                    .side_conditions
                    .iter()
                    .find(|value| value.id == condition)
                    .unwrap()
                    .turns_remaining;
                assert_eq!(
                    first_duration,
                    Some(u8::try_from(fixture_integer(case, "initial_duration") - 1).unwrap())
                );
                let repeated = execute_single_move(&dex, &first.state, 0, ActionTarget::Automatic);
                let repeated_duration = repeated
                    .state
                    .side(SideId::P1)
                    .side_conditions
                    .iter()
                    .find(|value| value.id == condition)
                    .unwrap()
                    .turns_remaining;
                assert_eq!(repeated_duration, first_duration.map(|value| value - 1));
                assert!(fixture_bool(case, "repeated_failed"));
            }
            "trick-room-toggle" => {
                let source = set_with_moves(&dex, "goodra", "sapsipper", &["trickroom"]);
                let condition = dex.field_effect_ids().trick_room.unwrap();
                let state = selected_primary_battle(&dex, &source, &ordinary, 0x3370);
                let first = execute_single_move(&dex, &state, 0, ActionTarget::Automatic);
                let is_active = |state: &BattleState| {
                    state
                        .field()
                        .conditions
                        .iter()
                        .any(|value| value.id == condition)
                };
                assert_eq!(
                    is_active(&first.state),
                    fixture_bool(case, "initial_active")
                );
                let repeated = execute_single_move(&dex, &first.state, 0, ActionTarget::Automatic);
                assert_eq!(
                    is_active(&repeated.state),
                    fixture_bool(case, "repeated_active")
                );
            }
            "magnet-rise-reapplication" => {
                let source = set_with_moves(&dex, "goodra", "sapsipper", &["magnetrise"]);
                let condition = dex.field_effect_ids().magnet_rise.unwrap();
                let state = selected_primary_battle(&dex, &source, &ordinary, 0x3380);
                let first = execute_single_move(&dex, &state, 0, ActionTarget::Automatic);
                let volatile = |state: &BattleState| {
                    state
                        .side(SideId::P1)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .volatiles
                        .iter()
                        .find(|value| value.id == condition)
                        .map(|value| value.turns_remaining)
                };
                assert_eq!(
                    volatile(&first.state),
                    Some(Some(
                        u8::try_from(fixture_integer(case, "initial_duration") - 1).unwrap()
                    ))
                );
                let repeated = execute_single_move(&dex, &first.state, 0, ActionTarget::Automatic);
                assert_eq!(volatile(&repeated.state), Some(Some(3)));
                assert!(fixture_bool(case, "repeated_failed"));
                let mut builder = BattleStateBuilder::from_state(state).unwrap();
                builder.edit_field(|field| {
                    field.conditions.push(ConditionState::new(
                        dex.field_effect_ids().gravity.unwrap(),
                        Some(5),
                    ));
                });
                let blocked = execute_single_move(
                    &dex,
                    &builder.build().unwrap(),
                    0,
                    ActionTarget::Automatic,
                );
                assert_eq!(volatile(&blocked.state), None);
                assert!(fixture_bool(case, "gravity_blocked"));
            }
            "protect-blocks-status" => {
                let source = set_with_moves(&dex, "goodra", "sapsipper", &["willowisp"]);
                let target = set_with_moves(&dex, "blastoise", "torrent", &["protect"]);
                let state = selected_primary_battle(&dex, &source, &target, 0x3390);
                let action = |target| PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(0),
                    target,
                    mega: false,
                };
                let result = step(
                    &state,
                    &JointAction::Turn {
                        p1: TurnChoice {
                            actions: [action(foe_target), action(ActionTarget::Automatic)],
                        },
                        p2: TurnChoice {
                            actions: [
                                action(ActionTarget::Automatic),
                                action(ActionTarget::Automatic),
                            ],
                        },
                    },
                    &Mechanics::new(&dex, &EffectRegistry::new()),
                )
                .unwrap();
                assert_eq!(
                    primary_outcome(&result, dex.resolve_move("willowisp").unwrap().0),
                    TargetResult::Protected
                );
                assert_eq!(
                    result
                        .state
                        .side(SideId::P2)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .status,
                    None
                );
                assert!(fixture_bool(case, "blocked"));
            }
            other => panic!("unexecuted primary non-damage fixture case {other}"),
        }
    }

    executed.sort_unstable();
    assert_eq!(
        executed,
        [
            "electric-terrain-reapplication",
            "growl-limits",
            "hypnosis-sleep-rng",
            "light-screen-reapplication",
            "magnet-rise-reapplication",
            "noble-roar-combined",
            "protect-blocks-status",
            "recover-rounding",
            "reflect-reapplication",
            "sunny-day-reapplication",
            "swords-dance-limits",
            "tailwind-reapplication",
            "thunder-wave-immunities",
            "trick-room-toggle",
            "will-o-wisp-matrix",
        ]
    );
}

#[derive(Debug, Deserialize)]
struct ReferenceTarget {
    side: String,
    slot: usize,
    species: String,
}

#[derive(Debug, Deserialize)]
struct ReferenceTargetOutcome {
    target: ReferenceTarget,
    result: String,
    applied_hp_loss: u16,
    final_hp: u16,
    fainted: bool,
}

#[derive(Debug, Deserialize)]
struct ReferenceStage {
    stage: String,
}

#[derive(Debug, Deserialize)]
struct ReferenceExecution {
    selected_target: ReferenceTarget,
    resolved_targets: Vec<ReferenceTarget>,
    stages: Vec<ReferenceStage>,
    outcomes: Vec<ReferenceTargetOutcome>,
}

#[test]
fn doubles_targeting_fixture_contains_executed_pinned_showdown_cases() {
    let fixture = doubles_targeting_fixture();
    assert_eq!(fixture["showdown_commit"], SHOWDOWN_COMMIT);
    let cases = &fixture["executed_cases"];
    assert!(cases["spread"]["two_valid_targets"]["result"]
        .as_bool()
        .unwrap());
    assert!(cases["spread"]["one_remaining_target"]["log"][0]
        .as_str()
        .unwrap()
        .contains("p2a: Arcanine"));
    assert!(cases["spread"]["mixed_type_immune"]["log"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line.as_str().unwrap().contains("|-immune|p2a: Charizard")));

    assert!(cases["target_resolution"]["selected_foe_fainted"]["log"][0]
        .as_str()
        .unwrap()
        .contains("p2b: Blastoise"));
    assert_eq!(
        cases["target_resolution"]["selected_slot_switched"]["resolved_target"],
        "p2: Clefairy"
    );
    assert_eq!(cases["target_resolution"]["missing_ally"]["result"], false);

    assert_eq!(cases["protection"]["first"]["result"], true);
    assert_eq!(cases["protection"]["repeat_success"]["result"], true);
    assert_eq!(cases["protection"]["repeat_failure"]["result"], false);
    assert_eq!(cases["protection"]["reset"]["result"], true);
    assert_eq!(
        cases["protection"]["denominators"],
        serde_json::json!([[1, 3], [1, 9]])
    );
    assert!(cases["protection"]["feint"]["log"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line.as_str().unwrap().contains("move: Feint")));

    assert!(cases["redirection"]["follow_me"]["log"][0]
        .as_str()
        .unwrap()
        .contains("p2a: Clefairy"));
    assert!(cases["redirection"]["grass_ignores_rage_powder"]["log"][0]
        .as_str()
        .unwrap()
        .contains("p2b: Blastoise"));
    assert!(
        cases["redirection"]["conflict_active_entry_order_wins_equal_speed"]["log"][0]
            .as_str()
            .unwrap()
            .contains("p2a: Clefairy")
    );
    assert!(
        cases["redirection"]["redirector_unavailable"]["first"]["log"]
            .as_array()
            .unwrap()
            .iter()
            .any(|line| line.as_str().unwrap().starts_with("|faint|"))
    );
    assert!(
        cases["redirection"]["redirector_unavailable"]["second"]["log"][0]
            .as_str()
            .unwrap()
            .contains("p2b: Blastoise")
    );

    let accuracy = cases["per_target_execution"]["independent_accuracy"]["rng_operations"]
        .as_array()
        .unwrap();
    let accuracy_results = accuracy
        .iter()
        .filter(|entry| entry["denominator"] == 100)
        .map(|entry| entry["result"].as_bool().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(accuracy_results, [true, false]);
    let psychic = &cases["per_target_execution"]["psychic_terrain_mixed_targets"];
    assert_eq!(
        psychic["rng_operations"].as_array().unwrap().as_slice(),
        &[] as &[serde_json::Value]
    );
    assert!(psychic["log"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line.as_str().unwrap().contains("Psychic Terrain")));
    assert!(psychic["log"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line.as_str().unwrap().contains("move: Protect")));
}

#[derive(Debug, Deserialize)]
struct DamageFixture {
    showdown_commit: String,
    format: String,
    random: DamageRollRange,
    cases: Vec<DamageCase>,
}

#[derive(Debug, Deserialize)]
struct DamageRollRange {
    damage_percent_min: u8,
    damage_percent_max: u8,
}

#[derive(Debug, Deserialize)]
struct DamageCase {
    name: String,
    #[serde(rename = "move")]
    move_name: String,
    critical: bool,
    source: FixturePokemon,
    target: FixturePokemon,
    rolls: Vec<DamageRoll>,
}

#[derive(Debug, Deserialize)]
struct FixturePokemon {
    species: String,
    ability: String,
    nature: String,
    stat_points: StatAllocation,
    stages: FixtureStages,
    level: u8,
    stats: rotom_core::BattleStats,
    types: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct FixtureStages {
    hp: i8,
    atk: i8,
    def: i8,
    spa: i8,
    spd: i8,
    spe: i8,
    accuracy: i8,
    evasion: i8,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct DamageRoll {
    percent: u8,
    damage: u16,
}

#[derive(Debug, Deserialize)]
struct CanonicalStepFixture {
    showdown_commit: String,
    format: String,
    methodology: StepMethodology,
    cases: Vec<CanonicalStepCase>,
}

#[derive(Debug, Deserialize)]
struct StepMethodology {
    damage_source: String,
    integration_scope: String,
    full_transition_comparison: String,
    rng_comparison: String,
    move_draw_order: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CanonicalStepCase {
    name: String,
    #[serde(rename = "move")]
    move_name: String,
    damage_case: String,
    rotom_seed: u64,
    controlled_outcomes: ControlledOutcomes,
    expected: StepExpectation,
}

#[derive(Debug, Deserialize)]
struct ControlledOutcomes {
    accuracy: String,
    critical_hit: bool,
    damage_percent: u8,
}

#[derive(Debug, Deserialize)]
struct StepExpectation {
    damage: u16,
    applied_hp_loss: u16,
    target_hp: u16,
    pp_consumed: u8,
    fainted: bool,
    replacement_scheduled: bool,
}

impl From<FixtureStages> for StatStages {
    fn from(value: FixtureStages) -> Self {
        assert_eq!(value.hp, 0, "HP cannot have a battle stat stage");
        Self {
            atk: value.atk,
            def: value.def,
            spa: value.spa,
            spd: value.spd,
            spe: value.spe,
            accuracy: value.accuracy,
            evasion: value.evasion,
        }
    }
}

fn set(
    dex: &rotom_dex::DexData,
    species: &str,
    ability: &str,
    nature: &str,
    move_name: &str,
    points: StatAllocation,
) -> PokemonSet {
    PokemonSet {
        nickname: None,
        species: dex
            .resolve_species(species)
            .map(|(id, _)| id)
            .expect("fixture species"),
        form: None::<FormId>,
        level: 50,
        ability: dex
            .resolve_ability(ability)
            .map(|(id, _)| id)
            .expect("fixture ability"),
        item: None,
        nature: dex
            .resolve_nature(nature)
            .map(|(id, _)| id)
            .expect("fixture nature"),
        stat_points: points,
        ivs: StatAllocation::PERFECT_IVS,
        moves: vec![dex
            .resolve_move(move_name)
            .map(|(id, _)| id)
            .expect("fixture move")],
        gender: None::<Gender>,
        shiny: false,
    }
}

#[test]
fn champions_stats_and_damage_match_pinned_showdown_fixtures() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture: DamageFixture = serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/basic-damage.json"
    ))
    .expect("Showdown damage fixture should deserialize");
    assert_eq!(fixture.showdown_commit, SHOWDOWN_COMMIT);
    assert_eq!(fixture.format, "gen9championsvgc2026regmc");
    assert_eq!(fixture.random.damage_percent_min, 85);
    assert_eq!(fixture.random.damage_percent_max, 100);
    assert!(fixture.cases.len() >= 12);
    assert!(fixture.cases.iter().any(|case| case.critical));
    assert!(fixture
        .cases
        .iter()
        .any(|case| case.source.stages.spa != 0 || case.target.stages.spd != 0));
    assert!(fixture
        .cases
        .iter()
        .any(|case| case.move_name == "dragonclaw"));

    for case in fixture.cases {
        let source_set = fixture_set(&dex, &case.source, &case.move_name);
        let target_set = fixture_set(&dex, &case.target, "splash");
        let mut source = dex.pokemon_state(&source_set).expect("source state");
        let mut target = dex.pokemon_state(&target_set).expect("target state");
        source.stat_stages = case.source.stages.into();
        target.stat_stages = case.target.stages.into();
        let move_id = source_set.moves[0];
        assert_eq!(
            source.level, case.source.level,
            "{} source level",
            case.name
        );
        assert_eq!(
            target.level, case.target.level,
            "{} target level",
            case.name
        );
        assert_eq!(
            source.stats, case.source.stats,
            "{} source stats",
            case.name
        );
        assert_eq!(
            target.stats, case.target.stats,
            "{} target stats",
            case.name
        );
        assert!(!case.source.types.is_empty(), "{} source types", case.name);
        assert!(!case.target.types.is_empty(), "{} target types", case.name);
        assert_eq!(case.rolls.len(), 16, "{} roll count", case.name);
        for roll in case.rolls {
            let actual =
                calculate_move_damage(&dex, &source, &target, move_id, case.critical, roll.percent)
                    .unwrap_or_else(|error| panic!("{} at {}%: {error}", case.name, roll.percent));
            assert_eq!(
                actual.final_damage, roll.damage,
                "{} at {}%",
                case.name, roll.percent
            );
        }
    }
}

fn fixture_set(dex: &rotom_dex::DexData, pokemon: &FixturePokemon, move_name: &str) -> PokemonSet {
    let mut result = set(
        dex,
        &pokemon.species,
        &pokemon.ability,
        &pokemon.nature,
        move_name,
        pokemon.stat_points,
    );
    result.level = pokemon.level;
    result
}

fn repeated_side(pokemon: &rotom_core::PokemonState) -> SideState {
    SideState::new(TeamState::new(vec![
        pokemon.clone(),
        pokemon.clone(),
        pokemon.clone(),
        pokemon.clone(),
    ]))
}

fn four_selection() -> Vec<PokemonIndex> {
    (0..4).map(PokemonIndex::new).collect()
}

fn fixture_battle(
    dex: &rotom_dex::DexData,
    case: &DamageCase,
    seed: u64,
    trace_rng: bool,
) -> (
    rotom_core::BattleState,
    rotom_core::PokemonState,
    rotom_core::PokemonState,
) {
    let source_set = fixture_set(dex, &case.source, &case.move_name);
    let target_set = fixture_set(dex, &case.target, &case.move_name);
    let mut source = dex.pokemon_state(&source_set).expect("source state");
    let mut target = dex.pokemon_state(&target_set).expect("target state");
    source.stat_stages = case.source.stages.into();
    target.stat_stages = case.target.stages.into();
    let mut builder =
        BattleStateBuilder::new_with_seed(repeated_side(&source), repeated_side(&target), 4, seed)
            .expect("battle");
    let selection = four_selection();
    builder
        .select_teams(selection.clone(), selection)
        .expect("preview");
    if trace_rng {
        builder.enable_rng_trace();
    }
    (builder.build().expect("selected battle"), source, target)
}

fn p1_damage(
    actions: &[rotom_core::ResolvedAction],
) -> (rotom_core::ActiveSlot, rotom_core::DamageResult) {
    actions
        .iter()
        .find_map(|action| match action {
            rotom_core::ResolvedAction::Damaged {
                side: SideId::P1,
                target_slot,
                damage,
                ..
            } => Some((*target_slot, *damage)),
            _ => None,
        })
        .expect("P1 damage result")
}

fn assert_step_state(
    result: &rotom_core::StepResult,
    input: &rotom_core::BattleState,
    source: &rotom_core::PokemonState,
    target: &rotom_core::PokemonState,
    damaged_slot: rotom_core::ActiveSlot,
    case: &CanonicalStepCase,
) {
    let damaged_index = match damaged_slot {
        rotom_core::ActiveSlot::Left => PokemonIndex::new(0),
        rotom_core::ActiveSlot::Right => PokemonIndex::new(1),
    };
    let target_state = result
        .state
        .side(SideId::P2)
        .pokemon(damaged_index)
        .expect("target");
    assert_eq!(
        target_state.current_hp, case.expected.target_hp,
        "{} applied HP loss",
        case.name
    );
    assert_eq!(
        case.expected.applied_hp_loss,
        target.stats.hp - target_state.current_hp,
        "{} actual applied HP loss",
        case.name
    );
    assert_eq!(
        case.expected.fainted,
        case.expected.target_hp == 0,
        "{}",
        case.name
    );
    if case.expected.fainted {
        assert_ko_state(result, target_state, damaged_slot, case);
    } else {
        assert_surviving_state(
            result,
            target_state,
            target,
            damaged_slot,
            damaged_index,
            case,
        );
    }
    assert_eq!(
        result
            .state
            .replacement_request(SideId::P2)
            .expect("settled state should have valid replacement indexes")
            .is_some(),
        case.expected.replacement_scheduled,
        "{} actual replacement scheduling",
        case.name
    );
    assert_eq!(
        result
            .state
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .expect("source")
            .moves[0]
            .pp,
        source.moves[0].max_pp - case.expected.pp_consumed,
        "{} PP",
        case.name
    );
    assert_eq!(input.rng().draws(), 0, "{} input RNG changed", case.name);
    assert_stable_serializable_state(&result.state, &case.name);
}

fn assert_immune_step_case(
    result: &rotom_core::StepResult,
    battle: &BattleState,
    source: &rotom_core::PokemonState,
    target: &rotom_core::PokemonState,
    step_case: &CanonicalStepCase,
) {
    let turn = result.diagnostics.turn.as_ref().expect("turn diagnostics");
    assert!(turn.target_outcomes.iter().any(|outcome| {
        outcome.target.side == SideId::P2
            && outcome.result == rotom_core::TargetResult::Immune
            && outcome.applied_hp_loss == 0
    }));
    assert_step_state(
        result,
        battle,
        source,
        target,
        rotom_core::ActiveSlot::Left,
        step_case,
    );
}

fn assert_ko_state(
    result: &rotom_core::StepResult,
    target_state: &rotom_core::PokemonState,
    damaged_slot: rotom_core::ActiveSlot,
    case: &CanonicalStepCase,
) {
    assert_eq!(target_state.current_hp, 0, "{} faint HP", case.name);
    assert_eq!(
        target_state.position,
        PokemonPosition::Fainted,
        "{} faint position",
        case.name
    );
    assert_eq!(
        result.state.side(SideId::P2).active[damaged_slot.index()],
        None,
        "{} cleared active slot",
        case.name
    );
    assert_eq!(
        result.state.phase(),
        BattlePhase::ForcedSwitch,
        "{} forced-switch phase",
        case.name
    );
    let request = result
        .state
        .replacement_request(SideId::P2)
        .expect("settled state should have valid replacement indexes")
        .expect("fainted target's side should need a replacement");
    assert!(
        request.slots.contains(&damaged_slot),
        "{} replacement request must include the cleared slot",
        case.name
    );
    assert!(request.required > 0, "{} replacement count", case.name);
    assert_eq!(
        result
            .state
            .replacement_request(SideId::P1)
            .expect("settled state should have valid replacement indexes"),
        None,
        "{} opposing side must not need a replacement",
        case.name
    );
}

fn assert_surviving_state(
    result: &rotom_core::StepResult,
    target_state: &rotom_core::PokemonState,
    initial_target: &rotom_core::PokemonState,
    damaged_slot: rotom_core::ActiveSlot,
    damaged_index: PokemonIndex,
    case: &CanonicalStepCase,
) {
    assert_eq!(
        target_state.position,
        PokemonPosition::Active { slot: damaged_slot },
        "{} surviving target position",
        case.name
    );
    assert_eq!(
        result.state.side(SideId::P2).active[damaged_slot.index()],
        Some(damaged_index),
        "{} surviving target active slot",
        case.name
    );
    assert_eq!(
        result.state.phase(),
        BattlePhase::TurnSelection,
        "{} non-KO phase",
        case.name
    );
    for side in [SideId::P1, SideId::P2] {
        assert_eq!(
            result
                .state
                .replacement_request(side)
                .expect("settled state should have valid replacement indexes"),
            None,
            "{} non-KO replacement request for {side:?}",
            case.name
        );
    }
    if case.expected.damage == 0 {
        assert_eq!(
            target_state.current_hp, initial_target.stats.hp,
            "{} immunity must preserve HP",
            case.name
        );
    }
}

fn assert_stable_serializable_state(state: &BattleState, case_name: &str) {
    state
        .validate_stable_state()
        .unwrap_or_else(|error| panic!("{case_name} unstable step output: {error}"));
    let serialized = serde_json::to_string(state)
        .unwrap_or_else(|error| panic!("{case_name} state serialization failed: {error}"));
    let restored: BattleState = serde_json::from_str(&serialized)
        .unwrap_or_else(|error| panic!("{case_name} state deserialization failed: {error}"));
    assert_eq!(restored, *state, "{case_name} state round trip");
}

fn round_trip_state(state: &BattleState) -> BattleState {
    serde_json::from_str(&serde_json::to_string(state).unwrap()).unwrap()
}

fn opposing_targets_choice() -> TurnChoice {
    TurnChoice {
        actions: [rotom_core::ActiveSlot::Left, rotom_core::ActiveSlot::Right].map(|target| {
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(target),
                mega: false,
            }
        }),
    }
}

fn set_with_moves(
    dex: &rotom_dex::DexData,
    species: &str,
    ability: &str,
    moves: &[&str],
) -> PokemonSet {
    let mut result = set(
        dex,
        species,
        ability,
        "serious",
        moves[0],
        StatAllocation::ZERO,
    );
    result.moves = moves
        .iter()
        .map(|name| dex.resolve_move(name).expect("move identity").0)
        .collect();
    result
}

fn selected_battle_for_sets(
    dex: &rotom_dex::DexData,
    p1: &PokemonSet,
    p2: &PokemonSet,
    seed: u64,
    trace: bool,
) -> BattleState {
    let make_side = |set: &PokemonSet| {
        let pokemon = dex.pokemon_state(set).expect("battle Pokémon");
        repeated_side(&pokemon)
    };
    let mut builder =
        BattleStateBuilder::new_with_seed(make_side(p1), make_side(p2), 4, seed).expect("battle");
    let selection = four_selection();
    builder
        .select_teams(selection.clone(), selection)
        .expect("selected teams");
    if trace {
        builder.enable_rng_trace();
    }
    builder.build().expect("stable battle")
}

fn side_from_sets(dex: &rotom_dex::DexData, sets: &[PokemonSet]) -> SideState {
    SideState::new(TeamState::new(
        sets.iter()
            .map(|set| dex.pokemon_state(set).expect("battle Pokémon"))
            .collect(),
    ))
}

fn selected_battle_for_teams(
    dex: &rotom_dex::DexData,
    p1: &[PokemonSet],
    p2: &[PokemonSet],
    seed: u64,
) -> BattleState {
    let mut builder = BattleStateBuilder::new_with_seed(
        side_from_sets(dex, p1),
        side_from_sets(dex, p2),
        4,
        seed,
    )
    .expect("battle");
    let selection = four_selection();
    builder
        .select_teams(selection.clone(), selection)
        .expect("selected teams");
    builder.enable_rng_trace();
    builder.build().expect("stable battle")
}

fn reference_execution(value: &serde_json::Value) -> ReferenceExecution {
    serde_json::from_value(value["execution"].clone()).expect("normalized Showdown execution")
}

fn normalized_failure_stage(result: TargetResult) -> &'static str {
    match result {
        TargetResult::Hit => "hit",
        TargetResult::Missed => "accuracy",
        TargetResult::Immune => "type_immunity",
        TargetResult::Protected | TargetResult::PriorityBlocked | TargetResult::Cancelled => {
            "try_hit"
        }
    }
}

fn controlled_reason(value: &serde_json::Value) -> RngReason {
    match value["reason"].as_str().expect("reference RNG reason") {
        "accuracy" => RngReason::Accuracy,
        "critical_hit" => RngReason::CriticalHit,
        "damage_roll" => RngReason::DamageRoll,
        "protect" => RngReason::Protect,
        reason => panic!("unknown reference RNG reason {reason}"),
    }
}

fn outcome_has_controlled_rng(result: &rotom_core::StepResult, expected: &[RngReason]) -> bool {
    let Some(trace) = result.state.rng().trace() else {
        return false;
    };
    if trace
        .iter()
        .map(|entry| entry.reason.clone())
        .collect::<Vec<_>>()
        != expected
    {
        return false;
    }
    trace.iter().all(|entry| match entry.reason {
        RngReason::CriticalHit => entry.result >= 1,
        RngReason::DamageRoll => entry.result == 100,
        RngReason::Accuracy | RngReason::Protect => match entry.operation {
            RngOperation::Chance { numerator, .. } => entry.result < numerator,
            _ => false,
        },
        _ => true,
    })
}

fn rng_trace_matches_reference(result: &rotom_core::StepResult, case: &serde_json::Value) -> bool {
    let expected = case["rng_operations"]
        .as_array()
        .expect("reference RNG operations");
    let Some(trace) = result.state.rng().trace() else {
        return false;
    };
    trace.len() == expected.len()
        && trace.iter().zip(expected).all(|(actual, expected)| {
            if actual.reason != controlled_reason(expected) {
                return false;
            }
            match actual.operation {
                RngOperation::Chance { numerator, .. } => {
                    (actual.result < numerator) == expected["result"].as_bool().unwrap()
                }
                RngOperation::Range { .. } => {
                    actual.result == expected["percent"].as_u64().unwrap()
                }
                RngOperation::Sample { .. } => false,
            }
        })
}

fn controlled_step(
    dex: &rotom_dex::DexData,
    p1: &[PokemonSet],
    p2: &[PokemonSet],
    action: &JointAction,
    expected_rng: &[RngReason],
) -> (BattleState, rotom_core::StepResult) {
    let effects = EffectRegistry::new();
    let mechanics = Mechanics::new(dex, &effects);
    for seed in 0..100_000 {
        let state = selected_battle_for_teams(dex, p1, p2, seed);
        let Ok(result) = step(&state, action, &mechanics) else {
            continue;
        };
        if outcome_has_controlled_rng(&result, expected_rng) {
            return (state, result);
        }
    }
    panic!("no deterministic seed produced the controlled reference outcomes");
}

fn controlled_step_for_reference(
    dex: &rotom_dex::DexData,
    p1: &[PokemonSet],
    p2: &[PokemonSet],
    action: &JointAction,
    reference_case: &serde_json::Value,
    edit: impl Fn(&mut BattleStateBuilder),
) -> rotom_core::StepResult {
    let effects = EffectRegistry::new();
    let mechanics = Mechanics::new(dex, &effects);
    let mut first_trace = None;
    for seed in 0..100_000 {
        let state = selected_battle_for_teams(dex, p1, p2, seed);
        let mut builder = BattleStateBuilder::from_state(state).unwrap();
        edit(&mut builder);
        let state = builder.build().unwrap();
        let Ok(result) = step(&state, action, &mechanics) else {
            continue;
        };
        first_trace.get_or_insert_with(|| result.state.rng().trace().unwrap().to_vec());
        if rng_trace_matches_reference(&result, reference_case) {
            return result;
        }
    }
    panic!(
        "no deterministic seed matched the controlled Showdown RNG outcomes; first trace: {first_trace:?}; reference: {:?}",
        reference_case["rng_operations"]
    );
}

fn assert_normalized_execution(
    dex: &rotom_dex::DexData,
    reference_case: &serde_json::Value,
    result: &rotom_core::StepResult,
    source: PokemonRef,
    move_id: MoveId,
    selected_slot: ActiveSlot,
) {
    let reference = reference_execution(reference_case);
    assert_eq!(reference.selected_target.side, "p2");
    assert_eq!(reference.selected_target.slot, selected_slot.index());
    let outcomes = result
        .diagnostics
        .turn
        .as_ref()
        .expect("turn diagnostics")
        .target_outcomes
        .iter()
        .filter(|outcome| outcome.source == source && outcome.move_id == move_id)
        .collect::<Vec<_>>();
    assert_eq!(outcomes.len(), reference.resolved_targets.len());
    assert_eq!(outcomes.len(), reference.outcomes.len());

    for ((actual, resolved), expected) in outcomes
        .iter()
        .zip(&reference.resolved_targets)
        .zip(&reference.outcomes)
    {
        assert_eq!(resolved.side, expected.target.side);
        assert_eq!(resolved.slot, expected.target.slot);
        let expected_side = match resolved.side.as_str() {
            "p1" => SideId::P1,
            "p2" => SideId::P2,
            side => panic!("unknown reference side {side}"),
        };
        assert_eq!(actual.target.side, expected_side);
        let target = result
            .state
            .side(actual.target.side)
            .pokemon(actual.target.pokemon)
            .expect("resolved target");
        assert_eq!(
            target.species,
            dex.resolve_species(&resolved.species).unwrap().0
        );
        if let PokemonPosition::Active { slot } = target.position {
            assert_eq!(slot.index(), resolved.slot);
        }
        assert_eq!(normalized_failure_stage(actual.result), expected.result);
        assert_eq!(actual.applied_hp_loss, expected.applied_hp_loss);
        assert_eq!(target.current_hp, expected.final_hp);
        assert_eq!(actual.fainted, expected.fainted);
    }

    let reference_stages = reference
        .stages
        .iter()
        .map(|stage| stage.stage.as_str())
        .collect::<Vec<_>>();
    assert!(reference_stages.starts_with(&["invulnerability", "try_hit"]));
    assert!(reference_stages
        .windows(2)
        .all(|window| window[0] != window[1]));
}

fn use_move(move_slot: u8, target: ActionTarget) -> PokemonAction {
    PokemonAction::Move {
        move_slot: MoveSlotIndex::new(move_slot),
        target,
        mega: false,
    }
}

fn assert_doubles_targeting_fixture_methodology(fixture: &serde_json::Value) {
    assert_eq!(fixture["showdown_commit"], SHOWDOWN_COMMIT);
    assert_eq!(
        fixture["spread"]["doubles_modifier"],
        serde_json::json!([3, 4])
    );
    assert_eq!(
        fixture["per_target_hit_order"],
        serde_json::json!([
            "try_hit",
            "type_immunity",
            "move_immunity",
            "accuracy",
            "break_protect",
            "damage"
        ])
    );
    assert!(fixture["methodology"]["execution"]
        .as_str()
        .unwrap()
        .contains("BattleActions.useMove"));
    assert_eq!(
        fixture["methodology"]["classification"]["full_transition"],
        serde_json::json!([])
    );
    assert!(fixture["methodology"]["rng_comparison"]
        .as_str()
        .unwrap()
        .contains("no raw PRNG"));
}

fn reference_mixed_protected_damage_count(fixture: &serde_json::Value) -> usize {
    let reference = &fixture["executed_cases"]["spread"]["mixed_protected"];
    reference["before"]
        .as_array()
        .unwrap()
        .iter()
        .zip(reference["after"].as_array().unwrap())
        .filter(|(before, after)| before["hp"] != after["hp"])
        .count()
}

fn four_member_team(first: PokemonSet, second: PokemonSet) -> Vec<PokemonSet> {
    vec![first, second.clone(), second.clone(), second]
}

fn protected_feint_state(
    dex: &rotom_dex::DexData,
    target_species: &str,
    seed: u64,
    accuracy_stage: i8,
    evasion_stage: i8,
) -> BattleState {
    let attacker = set_with_moves(dex, "goodra", "pressure", &["feint", "aquatail"]);
    let ally = set_with_moves(dex, "magikarp", "pressure", &["aquatail"]);
    let target = set_with_moves(dex, target_species, "pressure", &["aquatail"]);
    let other = set_with_moves(dex, "blastoise", "pressure", &["aquatail"]);
    let state = selected_battle_for_teams(
        dex,
        &four_member_team(attacker, ally),
        &four_member_team(target, other),
        seed,
    );
    let protect = dex.field_effect_ids().protect.expect("Protect condition");
    let mut builder = BattleStateBuilder::from_state(state).unwrap();
    builder
        .edit_pokemon(
            PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
            |pokemon| pokemon.stat_stages.accuracy = accuracy_stage,
        )
        .unwrap();
    builder
        .edit_pokemon(
            PokemonRef::new(SideId::P2, PokemonIndex::new(0)),
            |pokemon| {
                pokemon.stat_stages.evasion = evasion_stage;
                pokemon
                    .volatiles
                    .push(ConditionState::new(protect, Some(2)));
            },
        )
        .unwrap();
    builder.build().unwrap()
}

fn feint_turn() -> JointAction {
    JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
                use_move(0, ActionTarget::Foe(ActiveSlot::Right)),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
                use_move(0, ActionTarget::Foe(ActiveSlot::Right)),
            ],
        },
    }
}

fn feint_outcome(result: &rotom_core::StepResult, feint: MoveId) -> TargetResult {
    result
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .find(|outcome| {
            outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0))
                && outcome.move_id == feint
        })
        .expect("Feint target outcome")
        .result
}

fn spread_comparison_action(protect_left_foe: bool) -> JointAction {
    JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(0, ActionTarget::Automatic),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(u8::from(protect_left_foe), ActionTarget::Automatic),
                use_move(0, ActionTarget::Automatic),
            ],
        },
    }
}

fn reference_rng_reasons(case: &serde_json::Value) -> Vec<RngReason> {
    case["rng_operations"]
        .as_array()
        .expect("reference RNG operations")
        .iter()
        .map(controlled_reason)
        .collect()
}

fn redirection_comparison_action(redirect_move_slot: u8, selected: ActiveSlot) -> JointAction {
    JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(selected)),
                use_move(0, ActionTarget::Ally(ActiveSlot::Left)),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(redirect_move_slot, ActionTarget::Automatic),
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
            ],
        },
    }
}

fn step_matching_reference_outcome(
    dex: &rotom_dex::DexData,
    p1: &[PokemonSet],
    p2: &[PokemonSet],
    action: &JointAction,
    reference_case: &serde_json::Value,
) -> rotom_core::StepResult {
    let expected = &reference_case["execution"]["outcomes"][0];
    let expected_species = expected["target"]["species"].as_str().unwrap();
    let expected_loss = u16::try_from(expected["applied_hp_loss"].as_u64().unwrap()).unwrap();
    let expected_result = expected["result"].as_str().unwrap();
    let move_id = p1[0].moves[0];
    let effects = EffectRegistry::new();
    let mechanics = Mechanics::new(dex, &effects);
    for seed in 0..10_000 {
        let state = selected_battle_for_teams(dex, p1, p2, seed);
        let Ok(result) = step(&state, action, &mechanics) else {
            continue;
        };
        let Some(outcome) = result
            .diagnostics
            .turn
            .as_ref()
            .unwrap()
            .target_outcomes
            .iter()
            .find(|outcome| {
                outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0))
                    && outcome.move_id == move_id
            })
        else {
            continue;
        };
        let species = result
            .state
            .side(outcome.target.side)
            .pokemon(outcome.target.pokemon)
            .unwrap()
            .species;
        if species == dex.resolve_species(expected_species).unwrap().0
            && outcome.applied_hp_loss == expected_loss
            && normalized_failure_stage(outcome.result) == expected_result
        {
            return result;
        }
    }
    panic!("no deterministic seed matched the normalized Showdown target outcome");
}

#[test]
fn normalized_spread_step_outcomes_match_executed_showdown() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    let spread = &fixture["executed_cases"]["spread"];
    let follow = set_with_moves(&dex, "magikarp", "pressure", &["followme"]);
    let left = set_with_moves(&dex, "arcanine", "pressure", &["followme", "protect"]);
    let right = set_with_moves(&dex, "blastoise", "pressure", &["followme"]);
    let p2 = four_member_team(left, right);

    for (name, move_name, protected) in [
        ("two_valid_targets", "dazzlinggleam", false),
        ("mixed_protected", "dazzlinggleam", true),
        ("surf_all_adjacent", "surf", false),
    ] {
        let attacker = set_with_moves(&dex, "goodra", "pressure", &[move_name]);
        let p1 = four_member_team(attacker, follow.clone());
        let action = spread_comparison_action(protected);
        let expected_rng = reference_rng_reasons(&spread[name]);
        let (_, result) = controlled_step(&dex, &p1, &p2, &action, &expected_rng);
        assert_normalized_execution(
            &dex,
            &spread[name],
            &result,
            PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
            dex.resolve_move(move_name).unwrap().0,
            ActiveSlot::Left,
        );
        assert_eq!(
            result
                .state
                .rng()
                .trace()
                .unwrap()
                .iter()
                .map(|entry| entry.reason.clone())
                .collect::<Vec<_>>(),
            expected_rng,
            "{name} RNG reason order"
        );
    }

    let attacker = set_with_moves(&dex, "garchomp", "pressure", &["earthquake"]);
    let p1 = four_member_team(attacker, follow);
    let immune_left = set_with_moves(&dex, "charizard", "pressure", &["followme", "protect"]);
    let immune_right = set_with_moves(&dex, "arcanine", "pressure", &["followme"]);
    let immune_p2 = four_member_team(immune_left, immune_right);
    let action = spread_comparison_action(false);
    let expected_rng = reference_rng_reasons(&spread["mixed_type_immune"]);
    let (_, result) = controlled_step(&dex, &p1, &immune_p2, &action, &expected_rng);
    assert_normalized_execution(
        &dex,
        &spread["mixed_type_immune"],
        &result,
        PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
        dex.resolve_move("earthquake").unwrap().0,
        ActiveSlot::Left,
    );
}

#[test]
fn normalized_one_target_and_switched_slot_match_executed_showdown() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    let spread = &fixture["executed_cases"]["spread"]["one_remaining_target"];
    let follow = set_with_moves(&dex, "magikarp", "pressure", &["followme"]);
    let attacker = set_with_moves(&dex, "goodra", "pressure", &["dazzlinggleam"]);
    let p1 = four_member_team(attacker, follow.clone());
    let left = set_with_moves(&dex, "arcanine", "pressure", &["followme"]);
    let right = set_with_moves(&dex, "blastoise", "pressure", &["followme"]);
    let p2 = four_member_team(left, right);
    let one_target_action = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(0, ActionTarget::Automatic),
            ],
        },
        p2: TurnChoice {
            actions: [use_move(0, ActionTarget::Automatic), PokemonAction::Pass],
        },
    };
    let effects = EffectRegistry::new();
    let mechanics = Mechanics::new(&dex, &effects);
    let mut matched = None;
    for seed in 0..100_000 {
        let mut builder = BattleStateBuilder::new_with_seed(
            side_from_sets(&dex, &p1),
            side_from_sets(&dex, &p2),
            2,
            seed,
        )
        .unwrap();
        let selection = vec![PokemonIndex::new(0), PokemonIndex::new(1)];
        builder
            .select_teams(selection.clone(), selection)
            .unwrap()
            .process_faints(&[FaintTarget::new(SideId::P2, ActiveSlot::Right)])
            .unwrap()
            .enable_rng_trace();
        let state = builder.build().unwrap();
        let result = step(&state, &one_target_action, &mechanics).unwrap();
        if rng_trace_matches_reference(&result, spread) {
            matched = Some(result);
            break;
        }
    }
    let one_target = matched.expect("controlled one-target spread result");
    assert_normalized_execution(
        &dex,
        spread,
        &one_target,
        PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
        dex.resolve_move("dazzlinggleam").unwrap().0,
        ActiveSlot::Left,
    );
}

#[test]
fn normalized_switched_slot_matches_executed_showdown() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    let target_resolution = &fixture["executed_cases"]["target_resolution"];
    let attack = set_with_moves(&dex, "goodra", "pressure", &["aquatail"]);
    let ally_attack = set_with_moves(&dex, "magikarp", "pressure", &["aquatail"]);
    let p1 = four_member_team(attack, ally_attack);
    let arcanine = set_with_moves(&dex, "arcanine", "pressure", &["aquatail"]);
    let blastoise = set_with_moves(&dex, "blastoise", "pressure", &["aquatail"]);
    let clefairy = set_with_moves(&dex, "clefairy", "pressure", &["aquatail"]);
    let p2 = vec![arcanine, blastoise, clefairy.clone(), clefairy];
    let switch_action = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
                use_move(0, ActionTarget::Ally(ActiveSlot::Left)),
            ],
        },
        p2: TurnChoice {
            actions: [
                PokemonAction::Switch {
                    pokemon: PokemonIndex::new(2),
                },
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
            ],
        },
    };
    let switched = step_matching_reference_outcome(
        &dex,
        &p1,
        &p2,
        &switch_action,
        &target_resolution["selected_slot_switched"],
    );
    assert_normalized_execution(
        &dex,
        &target_resolution["selected_slot_switched"],
        &switched,
        PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
        dex.resolve_move("aquatail").unwrap().0,
        ActiveSlot::Left,
    );
}

#[test]
fn normalized_missing_ally_matches_executed_showdown() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    let target_resolution = &fixture["executed_cases"]["target_resolution"];
    let effects = EffectRegistry::new();
    let mechanics = Mechanics::new(&dex, &effects);
    let attacker = set_with_moves(&dex, "goodra", "pressure", &["dragonpulse"]);
    let ally = set_with_moves(&dex, "arcanine", "pressure", &["aquatail"]);
    let p1 = four_member_team(attacker, ally);
    let foe = set_with_moves(&dex, "blastoise", "pressure", &["aquatail"]);
    let p2 = four_member_team(foe.clone(), foe);
    let missing_ally_action = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Ally(ActiveSlot::Right)),
                PokemonAction::Pass,
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
            ],
        },
    };
    let mut builder = BattleStateBuilder::new_with_seed(
        side_from_sets(&dex, &p1),
        side_from_sets(&dex, &p2),
        2,
        0x13_02_02,
    )
    .unwrap();
    let selection = vec![PokemonIndex::new(0), PokemonIndex::new(1)];
    builder
        .select_teams(selection.clone(), selection)
        .unwrap()
        .process_faints(&[FaintTarget::new(SideId::P1, ActiveSlot::Right)])
        .unwrap()
        .enable_rng_trace();
    let missing_ally = step(&builder.build().unwrap(), &missing_ally_action, &mechanics).unwrap();
    let reference = reference_execution(&target_resolution["missing_ally_damage"]);
    assert_eq!(reference.selected_target.side, "p1");
    assert_eq!(reference.selected_target.slot, ActiveSlot::Right.index());
    assert!(reference.resolved_targets.is_empty());
    assert!(reference.outcomes.is_empty());
    assert_eq!(
        target_resolution["missing_ally_damage"]["rng_operations"]
            .as_array()
            .unwrap()
            .as_slice(),
        &[] as &[serde_json::Value]
    );
    assert!(!missing_ally
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .any(|outcome| {
            outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0))
                && outcome.move_id == dex.resolve_move("dragonpulse").unwrap().0
        }));
}

#[test]
fn normalized_redirection_step_outcomes_match_executed_showdown() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    let reference = &fixture["executed_cases"]["redirection"];
    let ally = set_with_moves(&dex, "clefairy", "pressure", &["aquatail"]);
    let target = set_with_moves(&dex, "blastoise", "pressure", &["aquatail"]);

    for (name, source_species, source_ability, source_item, redirect_move, redirected) in [
        ("follow_me", "goodra", "pressure", None, "followme", true),
        (
            "rage_powder",
            "goodra",
            "pressure",
            None,
            "ragepowder",
            true,
        ),
        (
            "grass_ignores_rage_powder",
            "amoonguss",
            "effectspore",
            None,
            "ragepowder",
            false,
        ),
        (
            "overcoat_ignores_rage_powder",
            "goodra",
            "overcoat",
            None,
            "ragepowder",
            false,
        ),
        (
            "safety_goggles_ignores_rage_powder",
            "goodra",
            "pressure",
            Some("safetygoggles"),
            "ragepowder",
            false,
        ),
    ] {
        let mut source = set_with_moves(&dex, source_species, source_ability, &["aquatail"]);
        source.item = source_item.map(|item| dex.resolve_item(item).unwrap().0);
        let p1 = four_member_team(source, ally.clone());
        let redirector = set_with_moves(&dex, "clefairy", "pressure", &[redirect_move]);
        let p2 = four_member_team(redirector, target.clone());
        let action = redirection_comparison_action(0, ActiveSlot::Right);
        let result = step_matching_reference_outcome(&dex, &p1, &p2, &action, &reference[name]);
        assert_normalized_execution(
            &dex,
            &reference[name],
            &result,
            PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
            dex.resolve_move("aquatail").unwrap().0,
            ActiveSlot::Right,
        );
        let outcome = result
            .diagnostics
            .turn
            .as_ref()
            .unwrap()
            .target_outcomes
            .iter()
            .find(|outcome| outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
            .unwrap();
        assert_eq!(outcome.redirected, redirected, "{name}");
    }
}

#[test]
fn normalized_redirector_conflicts_match_executed_showdown() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    let reference = &fixture["executed_cases"]["redirection"];
    let ally = set_with_moves(&dex, "clefairy", "pressure", &["aquatail"]);
    let source = set_with_moves(&dex, "goodra", "pressure", &["aquatail"]);
    let p1 = four_member_team(source, ally);
    let redirector = set_with_moves(&dex, "clefairy", "pressure", &["followme"]);
    let conflict_action = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
                use_move(0, ActionTarget::Ally(ActiveSlot::Left)),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(0, ActionTarget::Automatic),
            ],
        },
    };
    for (name, faster_right) in [
        ("conflict_active_entry_order_wins_equal_speed", false),
        ("conflict_faster_redirector_wins", true),
    ] {
        let left = redirector.clone();
        let mut right = redirector.clone();
        if faster_right {
            right.stat_points.spe = 252;
        }
        let p2 = four_member_team(left, right);
        let result =
            step_matching_reference_outcome(&dex, &p1, &p2, &conflict_action, &reference[name]);
        assert_normalized_execution(
            &dex,
            &reference[name],
            &result,
            PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
            dex.resolve_move("aquatail").unwrap().0,
            ActiveSlot::Left,
        );
    }

    let p2 = vec![
        redirector.clone(),
        redirector.clone(),
        redirector.clone(),
        redirector,
    ];
    let entry_reference = &reference["conflict_newer_left_loses_equal_speed"];
    let expected_loss = u16::try_from(
        entry_reference["execution"]["outcomes"][0]["applied_hp_loss"]
            .as_u64()
            .unwrap(),
    )
    .unwrap();
    let effects = EffectRegistry::new();
    let mechanics = Mechanics::new(&dex, &effects);
    let mut entry_result = None;
    for seed in 0..10_000 {
        let state = selected_battle_for_teams(&dex, &p1, &p2, seed);
        let mut builder = BattleStateBuilder::from_state(state).unwrap();
        builder
            .switch_active(SideId::P2, ActiveSlot::Left, PokemonIndex::new(2))
            .unwrap();
        let result = step(&builder.build().unwrap(), &conflict_action, &mechanics).unwrap();
        let outcome = result
            .diagnostics
            .turn
            .as_ref()
            .unwrap()
            .target_outcomes
            .iter()
            .find(|outcome| outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
            .unwrap();
        if outcome.applied_hp_loss == expected_loss {
            entry_result = Some(result);
            break;
        }
    }
    assert_normalized_execution(
        &dex,
        entry_reference,
        &entry_result.expect("controlled active-entry conflict"),
        PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
        dex.resolve_move("aquatail").unwrap().0,
        ActiveSlot::Left,
    );
}

fn protect_chain_action(user_move_slot: u8) -> JointAction {
    JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(user_move_slot, ActionTarget::Automatic),
                use_move(1, ActionTarget::Automatic),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(1, ActionTarget::Automatic),
                use_move(1, ActionTarget::Automatic),
            ],
        },
    }
}

fn protect_result(result: &rotom_core::StepResult, protect: MoveId) -> TargetResult {
    result
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .find(|outcome| {
            outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0))
                && outcome.move_id == protect
        })
        .expect("Protect outcome")
        .result
}

#[test]
fn normalized_protect_chain_and_feint_match_executed_showdown() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    let reference = &fixture["executed_cases"]["protection"];
    let protect = dex.resolve_move("protect").unwrap().0;
    let protect_user = set_with_moves(&dex, "goodra", "pressure", &["protect", "followme"]);
    let support = set_with_moves(&dex, "clefairy", "pressure", &["protect", "followme"]);
    let team = four_member_team(protect_user, support);
    let effects = EffectRegistry::new();
    let mechanics = Mechanics::new(&dex, &effects);

    let mut matched = None;
    for seed in 0..10_000 {
        let initial = selected_battle_for_teams(&dex, &team, &team, seed);
        let first = step(&initial, &protect_chain_action(0), &mechanics).unwrap();
        let repeat_success = step(&first.state, &protect_chain_action(0), &mechanics).unwrap();
        let repeat_failure =
            step(&repeat_success.state, &protect_chain_action(0), &mechanics).unwrap();
        if protect_result(&first, protect) == TargetResult::Hit
            && protect_result(&repeat_success, protect) == TargetResult::Hit
            && protect_result(&repeat_failure, protect) == TargetResult::Cancelled
        {
            matched = Some((first, repeat_success, repeat_failure));
            break;
        }
    }
    let (first, repeat_success, repeat_failure) = matched.expect("controlled Protect chain");
    assert_eq!(reference["first"]["result"], true);
    assert_eq!(reference["repeat_success"]["result"], true);
    assert_eq!(reference["repeat_failure"]["result"], false);
    assert_eq!(protect_result(&first, protect), TargetResult::Hit);
    assert_eq!(protect_result(&repeat_success, protect), TargetResult::Hit);
    assert_eq!(
        protect_result(&repeat_failure, protect),
        TargetResult::Cancelled
    );
    let protect_draws = repeat_failure
        .state
        .rng()
        .trace()
        .unwrap()
        .iter()
        .filter(|entry| entry.reason == RngReason::Protect)
        .collect::<Vec<_>>();
    assert_eq!(protect_draws.len(), 2);
    assert_eq!(
        protect_draws
            .iter()
            .map(|entry| match entry.operation {
                RngOperation::Chance {
                    numerator,
                    denominator,
                } => [numerator, denominator],
                _ => panic!("Protect must use a chance draw"),
            })
            .collect::<Vec<_>>(),
        serde_json::from_value::<Vec<[u64; 2]>>(reference["denominators"].clone()).unwrap()
    );

    let intervening = step(&repeat_failure.state, &protect_chain_action(1), &mechanics).unwrap();
    let reset = step(&intervening.state, &protect_chain_action(0), &mechanics).unwrap();
    assert_eq!(protect_result(&reset, protect), TargetResult::Hit);
    assert_eq!(reference["reset"]["result"], true);
    assert_eq!(
        reset
            .state
            .rng()
            .trace()
            .unwrap()
            .iter()
            .filter(|entry| entry.reason == RngReason::Protect)
            .count(),
        2,
        "the reset Protect succeeds without another stall draw"
    );

    let feint_user = set_with_moves(&dex, "goodra", "pressure", &["feint"]);
    let guard = set_with_moves(&dex, "arcanine", "pressure", &["protect"]);
    let p1 = four_member_team(feint_user, guard.clone());
    let p2 = four_member_team(guard.clone(), guard);
    let feint_action = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
                use_move(0, ActionTarget::Automatic),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(0, ActionTarget::Automatic),
            ],
        },
    };
    let feint = step_matching_reference_outcome(&dex, &p1, &p2, &feint_action, &reference["feint"]);
    assert_normalized_execution(
        &dex,
        &reference["feint"],
        &feint,
        PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
        dex.resolve_move("feint").unwrap().0,
        ActiveSlot::Left,
    );
}

#[test]
fn normalized_accuracy_faint_and_psychic_terrain_match_executed_showdown() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    let reference = &fixture["executed_cases"]["per_target_execution"];
    let attacker = set_with_moves(&dex, "charizard", "pressure", &["aircutter"]);
    let ally = set_with_moves(&dex, "magikarp", "pressure", &["followme"]);
    let p1 = four_member_team(attacker, ally.clone());
    let left = set_with_moves(&dex, "arcanine", "pressure", &["followme"]);
    let right = set_with_moves(&dex, "blastoise", "pressure", &["followme"]);
    let p2 = four_member_team(left, right);
    let spread_action = spread_comparison_action(false);

    let accuracy = controlled_step_for_reference(
        &dex,
        &p1,
        &p2,
        &spread_action,
        &reference["independent_accuracy"],
        |_| {},
    );
    assert_normalized_execution(
        &dex,
        &reference["independent_accuracy"],
        &accuracy,
        PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
        dex.resolve_move("aircutter").unwrap().0,
        ActiveSlot::Left,
    );

    let faint = controlled_step_for_reference(
        &dex,
        &p1,
        &p2,
        &spread_action,
        &reference["target_level_faint"],
        |builder| {
            builder
                .edit_pokemon(
                    PokemonRef::new(SideId::P2, PokemonIndex::new(0)),
                    |pokemon| {
                        pokemon.current_hp = 1;
                    },
                )
                .unwrap();
        },
    );
    assert_normalized_execution(
        &dex,
        &reference["target_level_faint"],
        &faint,
        PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
        dex.resolve_move("aircutter").unwrap().0,
        ActiveSlot::Left,
    );
    assert_eq!(
        faint
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .position,
        PokemonPosition::Fainted
    );
}

#[test]
fn normalized_psychic_terrain_targets_match_executed_showdown() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    let reference = &fixture["executed_cases"]["per_target_execution"];
    let ally = set_with_moves(&dex, "magikarp", "pressure", &["followme"]);
    let psychic_source = set_with_moves(&dex, "goodra", "pressure", &["dazzlinggleam"]);
    let psychic_p1 = four_member_team(psychic_source, ally);
    let psychic_left = set_with_moves(&dex, "arcanine", "pressure", &["followme"]);
    let psychic_right = set_with_moves(&dex, "charizard", "pressure", &["protect"]);
    let psychic_p2 = four_member_team(psychic_left, psychic_right);
    let psychic_terrain = dex
        .field_effect_ids()
        .psychic_terrain
        .expect("Psychic Terrain condition");
    let effects = EffectRegistry::new();
    let mechanics = Mechanics::new(&dex, &effects);
    let state = selected_battle_for_teams(&dex, &psychic_p1, &psychic_p2, 0x1322);
    let mut builder = BattleStateBuilder::from_state(state).unwrap();
    builder
        .edit_pokemon(
            PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
            |pokemon| {
                pokemon.moves[0].priority = 1;
            },
        )
        .unwrap();
    builder.edit_field(|field| {
        field.terrain = Some(ConditionState::new(psychic_terrain, Some(5)));
    });
    let psychic = step(
        &builder.build().unwrap(),
        &spread_comparison_action(false),
        &mechanics,
    )
    .unwrap();
    assert_eq!(psychic.state.rng().trace().unwrap(), []);
    assert_normalized_execution(
        &dex,
        &reference["psychic_terrain_mixed_targets"],
        &psychic,
        PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
        dex.resolve_move("dazzlinggleam").unwrap().0,
        ActiveSlot::Left,
    );
    let outcomes = psychic
        .diagnostics
        .turn
        .unwrap()
        .target_outcomes
        .into_iter()
        .filter(|outcome| outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
        .map(|outcome| outcome.result)
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes,
        [TargetResult::PriorityBlocked, TargetResult::Protected]
    );
}

#[test]
fn spread_damage_keeps_protected_targets_in_the_spread_modifier_and_ignores_redirection() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    assert_doubles_targeting_fixture_methodology(&fixture);
    let p1 = set_with_moves(&dex, "goodra", "gooey", &["surf", "protect"]);
    let p2 = set_with_moves(&dex, "clefairy", "friendguard", &["protect", "followme"]);
    let battle = selected_battle_for_sets(&dex, &p1, &p2, 0x132, true);
    let result = step(
        &battle,
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Automatic),
                    use_move(1, ActionTarget::Automatic),
                ],
            },
            p2: TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Automatic),
                    use_move(1, ActionTarget::Automatic),
                ],
            },
        },
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .expect("spread/protection turn");
    let turn = result.diagnostics.turn.as_ref().expect("turn diagnostics");
    let surf = dex.resolve_move("surf").expect("Surf").0;
    let surf_outcomes = turn
        .events
        .iter()
        .find(|event| event.context.move_id == Some(surf));
    assert!(
        surf_outcomes.is_some(),
        "Surf entered the shared event pipeline"
    );
    assert_eq!(turn.target_outcomes.len(), 6);
    let p1_surf = &turn.target_outcomes[turn.target_outcomes.len() - 3..];
    assert_eq!(
        p1_surf
            .iter()
            .filter(|outcome| outcome.result == rotom_core::TargetResult::Protected)
            .count(),
        2
    );
    assert_eq!(
        p1_surf
            .iter()
            .filter(|outcome| outcome.result == rotom_core::TargetResult::Hit)
            .count(),
        1
    );
    let reference_damaged = reference_mixed_protected_damage_count(&fixture);
    assert_eq!(reference_damaged, 1);
    assert_eq!(
        reference_damaged,
        p1_surf
            .iter()
            .filter(|outcome| outcome.result == rotom_core::TargetResult::Hit)
            .count()
    );
    assert!(p1_surf.iter().all(|outcome| !outcome.redirected));
    assert_eq!(p1_surf[0].target.side, SideId::P1);
    assert_eq!(p1_surf[1].target.side, SideId::P2);
    assert_eq!(p1_surf[2].target.side, SideId::P2);
    let damage = turn
        .actions
        .iter()
        .find_map(|action| match action {
            rotom_core::ResolvedAction::Damaged {
                side: SideId::P1,
                damage,
                ..
            } => Some(*damage),
            _ => None,
        })
        .expect("unprotected spread target was damaged");
    let source = battle
        .side(SideId::P1)
        .pokemon(PokemonIndex::new(0))
        .unwrap();
    let target = battle
        .side(SideId::P2)
        .pokemon(PokemonIndex::new(1))
        .unwrap();
    let single = calculate_move_damage(
        &dex,
        source,
        target,
        surf,
        damage.critical_modifier != rotom_core::Modifier::NEUTRAL,
        u8::try_from(damage.random_modifier.numerator).unwrap(),
    )
    .unwrap();
    assert!(damage.final_damage < single.final_damage);
    assert_stable_serializable_state(&result.state, "spread/protection");
}

#[test]
fn follow_me_redirects_single_target_moves_and_repeat_protect_uses_the_battle_rng() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    assert_eq!(
        fixture["protect"]["repeat_chances"],
        serde_json::json!([[1, 3], [1, 9], [1, 27]])
    );
    assert_eq!(fixture["redirection"]["spread_moves_redirected"], false);
    let p1 = set_with_moves(&dex, "goodra", "gooey", &["aquatail", "protect"]);
    let p2 = set_with_moves(&dex, "clefairy", "friendguard", &["followme", "protect"]);
    let battle = selected_battle_for_sets(&dex, &p1, &p2, 0x130, true);
    let turn = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(rotom_core::ActiveSlot::Right)),
                use_move(1, ActionTarget::Automatic),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(1, ActionTarget::Automatic),
            ],
        },
    };
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let first = step(&battle, &turn, &mechanics).expect("redirection turn");
    let first_turn = first.diagnostics.turn.as_ref().unwrap();
    let redirected = first_turn
        .target_outcomes
        .iter()
        .find(|outcome| outcome.redirected)
        .expect("Follow Me redirected Dragon Pulse");
    assert_eq!(redirected.target.side, SideId::P2);
    assert_eq!(redirected.target.pokemon, PokemonIndex::new(0));
    assert_eq!(redirected.result, rotom_core::TargetResult::Hit);

    let repeat = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(1, ActionTarget::Automatic),
                use_move(1, ActionTarget::Automatic),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(1, ActionTarget::Automatic),
                use_move(1, ActionTarget::Automatic),
            ],
        },
    };
    let replay_input = first.state.clone();
    let repeated = step(&first.state, &repeat, &mechanics).expect("repeat Protect turn");
    let replayed = step(&replay_input, &repeat, &mechanics).expect("repeat replay");
    assert_eq!(repeated, replayed);
    let protect_draws = repeated
        .state
        .rng()
        .trace()
        .unwrap()
        .iter()
        .filter(|entry| entry.reason == RngReason::Protect)
        .count();
    assert_eq!(
        protect_draws, 2,
        "only the two previous Protect users repeat"
    );
    let chained = repeated
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .filter(|outcome| outcome.source.pokemon == PokemonIndex::new(1))
        .map(|outcome| outcome.result)
        .collect::<Vec<_>>();
    assert!(chained.contains(&rotom_core::TargetResult::Hit));
    assert!(chained.contains(&rotom_core::TargetResult::Cancelled));
    assert_stable_serializable_state(&repeated.state, "redirection/repeat Protect");
}

#[test]
fn competing_redirectors_use_speed_then_active_entry_order_and_replay_deterministically() {
    let dex = champions_reg_mc().expect("generated Dex");
    let attacker = set_with_moves(&dex, "goodra", "gooey", &["aquatail", "protect"]);
    let redirector = set_with_moves(&dex, "clefairy", "friendguard", &["followme", "aquatail"]);
    let ids = dex.field_effect_ids();
    let follow_me = ids.follow_me.expect("Follow Me condition");
    let base = selected_battle_for_sets(&dex, &attacker, &redirector, 0x136, true);
    let attack = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(rotom_core::ActiveSlot::Left)),
                use_move(1, ActionTarget::Automatic),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(1, ActionTarget::Foe(rotom_core::ActiveSlot::Left)),
                use_move(1, ActionTarget::Foe(rotom_core::ActiveSlot::Right)),
            ],
        },
    };
    let run_fixture = |left_speed, right_speed| {
        let mut builder = BattleStateBuilder::from_state(base.clone()).unwrap();
        for (pokemon, speed) in [
            (PokemonIndex::new(0), left_speed),
            (PokemonIndex::new(1), right_speed),
        ] {
            builder
                .edit_pokemon(PokemonRef::new(SideId::P2, pokemon), |state| {
                    state.stats.spe = speed;
                    state
                        .volatiles
                        .push(ConditionState::new(follow_me, Some(1)));
                })
                .unwrap();
        }
        let state = builder.build().unwrap();
        let restored = round_trip_state(&state);
        assert_eq!(restored, state);
        let first = step(
            &state,
            &attack,
            &Mechanics::new(&dex, &EffectRegistry::new()),
        )
        .unwrap();
        let replay = step(
            &restored,
            &attack,
            &Mechanics::new(&dex, &EffectRegistry::new()),
        )
        .unwrap();
        assert_eq!(first, replay);
        first
            .diagnostics
            .turn
            .unwrap()
            .target_outcomes
            .into_iter()
            .find(|outcome| outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
            .expect("redirected attack outcome")
    };

    let faster = run_fixture(60, 120);
    assert_eq!(faster.target.pokemon, PokemonIndex::new(1));
    assert!(faster.redirected);

    let earlier = run_fixture(100, 100);
    assert_eq!(earlier.target.pokemon, PokemonIndex::new(0));
    assert!(earlier.redirected);

    let mut builder = BattleStateBuilder::from_state(base).unwrap();
    builder
        .switch_active(
            SideId::P2,
            rotom_core::ActiveSlot::Left,
            PokemonIndex::new(2),
        )
        .unwrap();
    for pokemon in [PokemonIndex::new(2), PokemonIndex::new(1)] {
        builder
            .edit_pokemon(PokemonRef::new(SideId::P2, pokemon), |state| {
                state.stats.spe = 100;
                state
                    .volatiles
                    .push(ConditionState::new(follow_me, Some(1)));
            })
            .unwrap();
    }
    let switched_state = builder.build().unwrap();
    let resumed = round_trip_state(&switched_state);
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let switched = step(&switched_state, &attack, &mechanics).unwrap();
    assert_eq!(switched, step(&resumed, &attack, &mechanics).unwrap());
    let switched_target = switched
        .diagnostics
        .turn
        .unwrap()
        .target_outcomes
        .into_iter()
        .find(|outcome| outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
        .unwrap();
    assert_eq!(switched_target.target.pokemon, PokemonIndex::new(1));
    assert!(switched_target.redirected);
}

#[test]
fn equal_speed_follow_me_users_use_active_entry_order_not_activation_order() {
    let dex = champions_reg_mc().expect("generated Dex");
    let attacker = set_with_moves(&dex, "goodra", "gooey", &["aquatail", "protect"]);
    let redirector = set_with_moves(&dex, "clefairy", "friendguard", &["followme"]);
    let battle = selected_battle_for_sets(&dex, &attacker, &redirector, 0x138, true);
    let turn = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(rotom_core::ActiveSlot::Left)),
                use_move(1, ActionTarget::Automatic),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(0, ActionTarget::Automatic),
            ],
        },
    };
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let result = step(&battle, &turn, &mechanics).unwrap();
    assert_eq!(result, step(&battle, &turn, &mechanics).unwrap());
    let follow_me = dex.resolve_move("followme").unwrap().0;
    let outcomes = &result.diagnostics.turn.as_ref().unwrap().target_outcomes;
    let first_activation = outcomes
        .iter()
        .find(|outcome| outcome.move_id == follow_me)
        .expect("first Follow Me activation")
        .source;
    let attack = outcomes
        .iter()
        .find(|outcome| outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
        .expect("attacking outcome");
    assert!(attack.redirected);
    assert_eq!(
        attack.target,
        PokemonRef::new(SideId::P2, PokemonIndex::new(0))
    );
    assert_ne!(first_activation, attack.target);
}

#[test]
fn redirection_ignores_a_redirector_that_fainted_before_the_later_attack() {
    let dex = champions_reg_mc().expect("generated Dex");
    let attacker = set_with_moves(&dex, "goodra", "gooey", &["aquatail"]);
    let defender = set_with_moves(&dex, "clefairy", "friendguard", &["aquatail"]);
    let follow_me = dex
        .field_effect_ids()
        .follow_me
        .expect("Follow Me condition");
    let base = selected_battle_for_sets(&dex, &attacker, &defender, 0x137, false);
    let mut builder = BattleStateBuilder::from_state(base).unwrap();
    builder
        .edit_pokemon(
            PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
            |pokemon| pokemon.stats.spe = 200,
        )
        .unwrap();
    builder
        .edit_pokemon(
            PokemonRef::new(SideId::P1, PokemonIndex::new(1)),
            |pokemon| pokemon.stats.spe = 100,
        )
        .unwrap();
    builder
        .edit_pokemon(
            PokemonRef::new(SideId::P2, PokemonIndex::new(0)),
            |pokemon| {
                pokemon.current_hp = 1;
                pokemon
                    .volatiles
                    .push(ConditionState::new(follow_me, Some(1)));
            },
        )
        .unwrap();
    let result = step(
        &builder.build().unwrap(),
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Foe(rotom_core::ActiveSlot::Right)),
                    use_move(0, ActionTarget::Foe(rotom_core::ActiveSlot::Right)),
                ],
            },
            p2: TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Foe(rotom_core::ActiveSlot::Left)),
                    use_move(0, ActionTarget::Foe(rotom_core::ActiveSlot::Right)),
                ],
            },
        },
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .unwrap();
    let attacks = result
        .diagnostics
        .turn
        .unwrap()
        .target_outcomes
        .into_iter()
        .filter(|outcome| outcome.source.side == SideId::P1)
        .collect::<Vec<_>>();
    assert_eq!(attacks.len(), 2);
    assert!(attacks[0].redirected);
    assert!(attacks[0].fainted);
    assert_eq!(attacks[0].target.pokemon, PokemonIndex::new(0));
    assert!(!attacks[1].redirected);
    assert_eq!(attacks[1].target.pokemon, PokemonIndex::new(1));
}

#[test]
fn feint_breaks_protect_only_after_immunity_and_accuracy() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture = doubles_targeting_fixture();
    let reference = &fixture["executed_cases"]["protection"];
    let effects = EffectRegistry::new();
    let mechanics = Mechanics::new(&dex, &effects);
    let feint = dex.resolve_move("feint").expect("Feint").0;
    let protect = dex.field_effect_ids().protect.expect("Protect condition");
    let target_kept_protect = |result: &rotom_core::StepResult| {
        result
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .has_volatile(protect)
    };
    for (case, expected) in [
        ("feint_immune", "type_immunity"),
        ("feint_miss", "accuracy"),
    ] {
        let execution = reference_execution(&reference[case]);
        assert_eq!(execution.outcomes[0].result, expected);
        assert!(reference[case]["after"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pokemon| pokemon["side"] == "p2" && pokemon["slot"] == 0)
            .unwrap()["volatiles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|volatile| volatile == "protect"));
    }

    let immune = step(
        &protected_feint_state(&dex, "gengar", 0x13_22_01, 0, 0),
        &feint_turn(),
        &mechanics,
    )
    .unwrap();
    assert_eq!(feint_outcome(&immune, feint), TargetResult::Immune);
    assert!(target_kept_protect(&immune));

    let missed = (0..1_000)
        .find_map(|seed| {
            let result = step(
                &protected_feint_state(&dex, "clefairy", seed, -6, 6),
                &feint_turn(),
                &mechanics,
            )
            .unwrap();
            (feint_outcome(&result, feint) == TargetResult::Missed).then_some(result)
        })
        .expect("a deterministic Feint accuracy miss");
    assert!(target_kept_protect(&missed));

    let hit = step(
        &protected_feint_state(&dex, "clefairy", 0x13_22_02, 0, 0),
        &feint_turn(),
        &mechanics,
    )
    .unwrap();
    assert_eq!(feint_outcome(&hit, feint), TargetResult::Hit);
    assert!(!target_kept_protect(&hit));
}

#[test]
fn rage_powder_respects_grass_overcoat_and_safety_goggles_and_feint_breaks_protect() {
    let dex = champions_reg_mc().expect("generated Dex");
    let redirector = set_with_moves(&dex, "clefairy", "friendguard", &["ragepowder", "protect"]);
    let make_turn = || JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(rotom_core::ActiveSlot::Right)),
                use_move(1, ActionTarget::Automatic),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(1, ActionTarget::Automatic),
            ],
        },
    };
    let ordinary = set_with_moves(&dex, "goodra", "gooey", &["aquatail", "protect"]);
    let mut grass = set_with_moves(&dex, "amoonguss", "effectspore", &["aquatail", "protect"]);
    let overcoat = set_with_moves(&dex, "goodra", "overcoat", &["aquatail", "protect"]);
    let mut goggles = ordinary.clone();
    goggles.item = Some(dex.resolve_item("safetygoggles").expect("Safety Goggles").0);
    // Keep the synthetic Grass source otherwise identical enough for the
    // target-resolution assertion; team legality is not under test here.
    grass.moves = ordinary.moves.clone();

    for (name, attacker, should_redirect) in [
        ("ordinary", ordinary, true),
        ("grass", grass, false),
        ("overcoat", overcoat, false),
        ("safety-goggles", goggles, false),
    ] {
        let attack_move = attacker.moves[0];
        let battle = selected_battle_for_sets(&dex, &attacker, &redirector, 0x134, false);
        let result = step(
            &battle,
            &make_turn(),
            &Mechanics::new(&dex, &EffectRegistry::new()),
        )
        .unwrap_or_else(|error| panic!("{name}: {error}"));
        let outcome = result
            .diagnostics
            .turn
            .as_ref()
            .unwrap()
            .target_outcomes
            .iter()
            .find(|outcome| outcome.move_id == attack_move)
            .unwrap_or_else(|| panic!("{name}: attacking outcome"));
        assert_eq!(outcome.redirected, should_redirect, "{name}");
        assert_eq!(
            outcome.target.pokemon,
            PokemonIndex::new(u8::from(!should_redirect)),
            "{name}"
        );
    }

    let attacker = set_with_moves(&dex, "goodra", "gooey", &["feint", "protect"]);
    let defender = set_with_moves(&dex, "clefairy", "friendguard", &["protect"]);
    let battle = selected_battle_for_sets(&dex, &attacker, &defender, 0x135, false);
    let result = step(
        &battle,
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Foe(rotom_core::ActiveSlot::Left)),
                    use_move(1, ActionTarget::Automatic),
                ],
            },
            p2: TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Automatic),
                    use_move(0, ActionTarget::Automatic),
                ],
            },
        },
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .expect("Feint turn");
    let feint_id = dex.resolve_move("feint").expect("Feint").0;
    let feint = result
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .find(|outcome| {
            outcome.move_id == feint_id && outcome.result == rotom_core::TargetResult::Hit
        })
        .expect("Feint bypassed Protect");
    assert_ne!(feint.result, rotom_core::TargetResult::Protected);
    assert!(feint.applied_hp_loss > 0);
}

#[test]
fn queued_turn_consumes_pp_and_updates_hp() {
    let dex = champions_reg_mc().expect("generated Dex");
    let special_attacker = set(
        &dex,
        "goodra",
        "gooey",
        "modest",
        "dragonpulse",
        StatAllocation::ZERO,
    );
    let physical_attacker = set(
        &dex,
        "goodra",
        "gooey",
        "adamant",
        "aquatail",
        StatAllocation::ZERO,
    );
    let make_side = |set: &PokemonSet| {
        let pokemon = dex.pokemon_state(set).expect("state");
        SideState::new(TeamState::new(vec![
            pokemon.clone(),
            pokemon.clone(),
            pokemon.clone(),
            pokemon,
        ]))
    };
    let mut builder = BattleStateBuilder::new_with_seed(
        make_side(&special_attacker),
        make_side(&physical_attacker),
        4,
        19,
    )
    .expect("battle");
    let selection = vec![
        PokemonIndex::new(0),
        PokemonIndex::new(1),
        PokemonIndex::new(2),
        PokemonIndex::new(3),
    ];
    builder
        .select_teams(selection.clone(), selection)
        .expect("preview");
    builder.enable_rng_trace();
    let battle = builder.build().expect("selected battle");
    let p1 = TurnChoice {
        actions: [
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(rotom_core::ActiveSlot::Left),
                mega: false,
            },
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(rotom_core::ActiveSlot::Right),
                mega: false,
            },
        ],
    };
    let p2 = TurnChoice {
        actions: [
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(rotom_core::ActiveSlot::Left),
                mega: false,
            },
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(rotom_core::ActiveSlot::Right),
                mega: false,
            },
        ],
    };
    let replay = battle.clone();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let result = step(&battle, &JointAction::Turn { p1, p2 }, &mechanics).expect("resolution");
    let replay_result =
        step(&replay, &JointAction::Turn { p1, p2 }, &mechanics).expect("replayed resolution");
    let outcome = result.diagnostics.turn.as_ref().unwrap();
    let replay_outcome = replay_result.diagnostics.turn.as_ref().unwrap();
    let battle = result.state;

    assert_eq!(outcome, replay_outcome);
    assert_eq!(battle, replay_result.state);
    assert_eq!(outcome.actions.len(), 4);
    assert_eq!(battle.phase(), BattlePhase::TurnSelection);
    assert_eq!(battle.turn(), 2);
    assert_eq!(battle.queue().actions.len(), 0);
    for side in [SideId::P1, SideId::P2] {
        for index in [PokemonIndex::new(0), PokemonIndex::new(1)] {
            let pokemon = battle.side(side).pokemon(index).expect("active");
            assert_eq!(pokemon.moves[0].pp, pokemon.moves[0].max_pp - 1);
            assert!(pokemon.current_hp < pokemon.stats.hp);
            assert!(pokemon.current_hp > 0);
        }
    }
    assert_combat_rng_trace(battle.rng().trace().expect("enabled trace"));
}

#[test]
fn canonical_step_matches_an_independently_selected_pinned_showdown_roll() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture: DamageFixture = serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/basic-damage.json"
    ))
    .expect("Showdown damage fixture should deserialize");
    let case = fixture
        .cases
        .iter()
        .find(|case| case.name == "dragon-pulse-neutral-stab")
        .expect("neutral Dragon Pulse fixture");
    // Seed 19's SplitMix64 trace is asserted below. Rotom Spark does not claim
    // seed parity with Showdown; the independently generated Showdown fixture
    // supplies the expected result for the preselected 96% damage outcome.
    let expected = case
        .rolls
        .iter()
        .find(|roll| roll.percent == 96)
        .expect("fixture's independently selected 96% roll");
    let (battle, source, target) = fixture_battle(&dex, case, 19, true);
    let choice = opposing_targets_choice();

    let result = step(
        &battle,
        &JointAction::Turn {
            p1: choice,
            p2: choice,
        },
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .expect("canonical move lifecycle");
    let trace = result.state.rng().trace().expect("enabled RNG trace");
    assert_eq!(
        trace.iter().map(|entry| entry.result).collect::<Vec<_>>(),
        [0, 2, 53, 1, 99, 11, 1, 85, 89, 10, 96, 9, 23, 98]
    );
    assert_eq!(trace[10].reason, RngReason::DamageRoll);
    assert_eq!(
        trace[10].operation,
        RngOperation::Range {
            start: 85,
            end_exclusive: 101,
        }
    );
    let (_, damage) = p1_damage(
        &result
            .diagnostics
            .turn
            .as_ref()
            .expect("turn diagnostics")
            .actions,
    );
    assert_eq!(damage.critical_modifier, rotom_core::Modifier::NEUTRAL);
    assert_eq!(damage.random_modifier.numerator, 96);
    assert_eq!(damage.random_modifier.denominator, 100);
    assert_eq!(damage.final_damage, expected.damage);
    assert_eq!(
        result
            .state
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .expect("source")
            .moves[0]
            .pp,
        source.moves[0].max_pp - 1
    );
    assert_eq!(
        battle
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .expect("unchanged input source")
            .moves[0]
            .pp,
        source.moves[0].max_pp
    );
    assert_eq!(
        result
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .expect("damaged target")
            .current_hp,
        target.stats.hp - expected.damage
    );
}

#[test]
fn canonical_step_matches_preselected_physical_special_effective_immune_and_critical_cases() {
    let dex = champions_reg_mc().expect("generated Dex");
    let damage_fixture: DamageFixture = serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/basic-damage.json"
    ))
    .expect("Showdown damage fixture should deserialize");
    let step_fixture: CanonicalStepFixture = serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/canonical-step.json"
    ))
    .expect("canonical step fixture should deserialize");
    assert_eq!(step_fixture.showdown_commit, SHOWDOWN_COMMIT);
    assert_eq!(step_fixture.format, "gen9championsvgc2026regmc");
    assert!(step_fixture
        .methodology
        .rng_comparison
        .contains("no seed-level parity"));
    assert!(step_fixture
        .methodology
        .damage_source
        .contains("Battle.actions.getDamage"));
    assert!(step_fixture
        .methodology
        .integration_scope
        .contains("Rotom Spark public step()"));
    assert!(step_fixture
        .methodology
        .full_transition_comparison
        .contains("not performed"));
    assert_eq!(
        step_fixture.methodology.move_draw_order,
        ["accuracy", "critical_hit", "damage_roll"]
    );

    for step_case in step_fixture.cases {
        let case = damage_fixture
            .cases
            .iter()
            .find(|case| case.name == step_case.damage_case)
            .unwrap_or_else(|| panic!("missing fixture case {}", step_case.damage_case));
        assert_eq!(step_case.move_name, case.move_name, "{}", step_case.name);
        assert_eq!(step_case.controlled_outcomes.accuracy, "hit");
        assert_eq!(
            step_case.controlled_outcomes.critical_hit, case.critical,
            "{}",
            step_case.name
        );
        let expected = case
            .rolls
            .iter()
            .find(|roll| roll.percent == step_case.controlled_outcomes.damage_percent)
            .unwrap_or_else(|| panic!("missing preselected roll for {}", step_case.name));
        assert_eq!(
            expected.damage, step_case.expected.damage,
            "{}",
            step_case.name
        );
        let (battle, source, target) = fixture_battle(&dex, case, step_case.rotom_seed, false);
        let result = step(
            &battle,
            &JointAction::Turn {
                p1: opposing_targets_choice(),
                p2: opposing_targets_choice(),
            },
            &Mechanics::new(&dex, &EffectRegistry::new()),
        )
        .unwrap_or_else(|error| panic!("{}: {error}", step_case.name));
        if step_case.expected.damage == 0 {
            assert_immune_step_case(&result, &battle, &source, &target, &step_case);
            continue;
        }
        let (damaged_slot, damage) = p1_damage(
            &result
                .diagnostics
                .turn
                .as_ref()
                .expect("turn diagnostics")
                .actions,
        );
        assert_eq!(
            damage.random_modifier.numerator,
            u16::from(step_case.controlled_outcomes.damage_percent),
            "{}",
            step_case.name
        );
        assert_eq!(
            damage.random_modifier.denominator, 100,
            "{}",
            step_case.name
        );
        assert_eq!(
            damage.critical_modifier != rotom_core::Modifier::NEUTRAL,
            step_case.controlled_outcomes.critical_hit,
            "{} critical outcome",
            step_case.name
        );
        assert_eq!(
            damage.final_damage, step_case.expected.damage,
            "{}",
            step_case.name
        );
        assert_step_state(&result, &battle, &source, &target, damaged_slot, &step_case);
    }
}

#[test]
fn resolved_dex_requirements_drive_support_without_a_move_id_allowlist() {
    let dex = champions_reg_mc().expect("generated Dex");
    let definition = |id: &str| {
        let move_id = dex.resolve_move(id).expect("move identity").0;
        <rotom_dex::DexData as CombatData>::move_data(&dex, move_id).expect("combat move")
    };

    let dragon_claw = definition("dragonclaw");
    assert_eq!(dragon_claw.operation, MoveOperation::StandardDamage);
    assert_eq!(
        dragon_claw
            .required_capabilities
            .first_missing_from(MoveCapabilities::PR_13_2),
        None
    );

    let surf = definition("surf");
    assert!(surf
        .required_capabilities
        .contains(MoveCapability::SpreadDamage));
    assert_eq!(
        surf.required_capabilities
            .first_missing_from(MoveCapabilities::PR_13_2),
        None
    );

    for (id, capability) in [
        ("flamethrower", MoveCapability::SecondaryEffect),
        ("shadowball", MoveCapability::SecondaryEffect),
        ("facade", MoveCapability::ConditionalPower),
        ("bulletseed", MoveCapability::MultiHit),
        ("populationbomb", MoveCapability::MultiHit),
        ("gigadrain", MoveCapability::Drain),
    ] {
        let move_data = definition(id);
        assert!(
            move_data.required_capabilities.contains(capability),
            "{id} must retain its intrinsic behavior capability"
        );
        assert_eq!(
            move_data
                .required_capabilities
                .first_missing_from(MoveCapabilities::PR_13_4),
            None,
            "{id} must be supported through its PR 13.4 operation"
        );
    }
    let electro_shot = definition("electroshot");
    assert!(electro_shot
        .required_capabilities
        .contains(MoveCapability::CustomCallback));
    assert!(electro_shot
        .required_capabilities
        .first_missing_from(MoveCapabilities::PR_13_4)
        .is_some());
}

fn pr_13_4_battle(dex: &rotom_dex::DexData, move_name: &str, seed: u64) -> BattleState {
    pr_13_4_battle_against(dex, move_name, "blastoise", seed)
}

fn pr_13_4_battle_against(
    dex: &rotom_dex::DexData,
    move_name: &str,
    target_species: &str,
    seed: u64,
) -> BattleState {
    let source = set_with_moves(dex, "goodra", "sapsipper", &[move_name]);
    let target_ability = match target_species {
        "charizard" => "blaze",
        "garchomp" => "roughskin",
        _ => "torrent",
    };
    let target = set_with_moves(dex, target_species, target_ability, &["recover"]);
    selected_primary_battle(dex, &source, &target, seed)
}

fn p1_left_hits(result: &rotom_core::StepResult) -> Vec<&rotom_core::TargetOutcome> {
    result
        .diagnostics
        .turn
        .as_ref()
        .expect("turn diagnostics")
        .target_outcomes
        .iter()
        .filter(|outcome| {
            outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0))
                && outcome.result == TargetResult::Hit
        })
        .collect()
}

fn move_rng_trace(result: &rotom_core::StepResult) -> Vec<&rotom_core::RngTraceEntry> {
    result
        .state
        .rng()
        .trace()
        .expect("enabled RNG trace")
        .iter()
        .filter(|entry| entry.reason != RngReason::SpeedTie)
        .collect()
}

fn traced_pr_13_4_move(
    dex: &rotom_dex::DexData,
    move_name: &str,
    seed: u64,
) -> rotom_core::StepResult {
    let mut builder = BattleStateBuilder::from_state(pr_13_4_battle(dex, move_name, seed)).unwrap();
    builder.enable_rng_trace();
    step(
        &builder.build().unwrap(),
        &single_move_action(0, ActionTarget::Foe(ActiveSlot::Left)),
        &Mechanics::new(dex, &EffectRegistry::new()),
    )
    .unwrap()
}

fn assert_move_rng_operations(trace: &[&rotom_core::RngTraceEntry]) {
    for entry in trace {
        match entry.reason {
            RngReason::CriticalHit => assert_eq!(
                entry.operation,
                RngOperation::Chance {
                    numerator: 1,
                    denominator: 24,
                }
            ),
            RngReason::DamageRoll => assert_eq!(
                entry.operation,
                RngOperation::Range {
                    start: 85,
                    end_exclusive: 101,
                }
            ),
            RngReason::SecondaryEffect => assert_eq!(
                entry.operation,
                RngOperation::Range {
                    start: 0,
                    end_exclusive: 100,
                }
            ),
            RngReason::Accuracy | RngReason::MultiHit => {}
            _ => panic!("unexpected move RNG reason: {:?}", entry.reason),
        }
    }
}

#[test]
fn facade_uses_ephemeral_power_and_effective_stat_hooks() {
    let dex = champions_reg_mc().unwrap();
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    let normal = step(
        &pr_13_4_battle(&dex, "facade", 73),
        &action,
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .unwrap();
    let mut builder = BattleStateBuilder::from_state(pr_13_4_battle(&dex, "facade", 73)).unwrap();
    builder
        .apply_major_status(
            PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
            MajorStatus::Burn,
        )
        .unwrap();
    let burned = step(
        &builder.build().unwrap(),
        &action,
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .unwrap();
    assert!(
        p1_left_hits(&burned)[0].calculated_damage > p1_left_hits(&normal)[0].calculated_damage
    );
    let turn = burned.diagnostics.turn.as_ref().unwrap();
    let relays = turn
        .events
        .iter()
        .filter(|event| {
            event.context.source == Some(PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
        })
        .map(|event| event.event)
        .collect::<Vec<_>>();
    assert!(relays.windows(3).any(|events| {
        events
            == [
                BattleEvent::ModifyBasePower,
                BattleEvent::ModifyAttack,
                BattleEvent::ModifyDefense,
            ]
    }));
}

#[test]
fn effective_stat_overrides_feed_the_shared_damage_relays() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    for (move_name, event, stat_owner, expected_stat) in [
        (
            "bodypress",
            BattleEvent::ModifySourceDefense,
            SideId::P1,
            BattleStat::Defense,
        ),
        (
            "psyshock",
            BattleEvent::ModifyDefense,
            SideId::P2,
            BattleStat::Defense,
        ),
    ] {
        let state = pr_13_4_battle(&dex, move_name, 97);
        let selected_stats = state
            .side(stat_owner)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .stats;
        let expected = match expected_stat {
            BattleStat::Attack => selected_stats.atk,
            BattleStat::Defense => selected_stats.def,
            BattleStat::SpecialAttack => selected_stats.spa,
            BattleStat::SpecialDefense => selected_stats.spd,
            BattleStat::Speed => selected_stats.spe,
            BattleStat::Accuracy | BattleStat::Evasion => unreachable!(),
        };
        let result = step(&state, &action, &mechanics).unwrap();
        let dispatch = result
            .diagnostics
            .turn
            .as_ref()
            .unwrap()
            .events
            .iter()
            .find(|dispatch| dispatch.event == event)
            .unwrap();
        assert_eq!(
            dispatch.value,
            rotom_core::EventValue::Integer(i64::from(expected))
        );
    }
}

#[test]
fn declarative_self_effects_apply_after_successful_damage() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    for (move_name, stat, expected) in [
        ("closecombat", BattleStat::Defense, -1),
        ("closecombat", BattleStat::SpecialDefense, -1),
        ("overheat", BattleStat::SpecialAttack, -2),
    ] {
        let result = step(&pr_13_4_battle(&dex, move_name, 101), &action, &mechanics).unwrap();
        assert_eq!(
            result
                .state
                .side(SideId::P1)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .stat_stages
                .get(stat),
            expected,
            "{move_name} must apply its declarative self effect"
        );
    }
}

#[test]
fn declarative_secondaries_roll_after_damage_and_use_shared_status_and_stage_rules() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    let flamethrower = (0..2_000)
        .find_map(|seed| {
            let mut builder =
                BattleStateBuilder::from_state(pr_13_4_battle(&dex, "flamethrower", seed)).unwrap();
            builder.enable_rng_trace();
            let result = step(&builder.build().unwrap(), &action, &mechanics).unwrap();
            (result
                .state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .status
                == Some(MajorStatus::Burn))
            .then_some(result)
        })
        .expect("deterministic seed with Flamethrower burn");
    let reasons = flamethrower
        .state
        .rng()
        .trace()
        .unwrap()
        .iter()
        .map(|entry| entry.reason.clone())
        .collect::<Vec<_>>();
    let damage = reasons
        .iter()
        .position(|reason| *reason == RngReason::DamageRoll)
        .unwrap();
    let secondary = reasons
        .iter()
        .position(|reason| *reason == RngReason::SecondaryEffect)
        .unwrap();
    assert!(secondary > damage);

    let shadow_ball = (0..2_000)
        .find_map(|seed| {
            let result = step(
                &pr_13_4_battle(&dex, "shadowball", seed),
                &action,
                &mechanics,
            )
            .unwrap();
            (result
                .state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .stat_stages
                .spd
                == -1)
                .then_some(result)
        })
        .expect("deterministic seed with Shadow Ball drop");
    shadow_ball.state.validate_stable_state().unwrap();
}

#[test]
fn multi_hit_and_fixed_damage_preserve_per_hit_rng_and_faint_boundaries() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    let mut builder =
        BattleStateBuilder::from_state(pr_13_4_battle(&dex, "bulletseed", 137)).unwrap();
    builder.enable_rng_trace();
    let bullet_seed = step(&builder.build().unwrap(), &action, &mechanics).unwrap();
    let hits = p1_left_hits(&bullet_seed).len();
    assert!((2..=5).contains(&hits));
    let trace = bullet_seed.state.rng().trace().unwrap();
    let multi_hit = trace
        .iter()
        .position(|entry| entry.reason == RngReason::MultiHit)
        .unwrap();
    let first_critical = trace
        .iter()
        .position(|entry| entry.reason == RngReason::CriticalHit)
        .unwrap();
    assert!(multi_hit < first_critical);
    assert_eq!(
        trace
            .iter()
            .filter(|entry| entry.reason == RngReason::CriticalHit)
            .count(),
        hits
    );

    let seismic = step(
        &pr_13_4_battle(&dex, "seismictoss", 19),
        &action,
        &mechanics,
    )
    .unwrap();
    assert_eq!(p1_left_hits(&seismic)[0].applied_hp_loss, 50);
}

#[test]
fn multi_accuracy_rechecks_each_later_hit_and_stops_the_sequence_on_a_miss() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    let mut builder =
        BattleStateBuilder::from_state(pr_13_4_battle(&dex, "populationbomb", 149)).unwrap();
    builder.enable_rng_trace();
    let result = step(&builder.build().unwrap(), &action, &mechanics).unwrap();
    let hits = p1_left_hits(&result).len();
    let accuracy_draws = result
        .state
        .rng()
        .trace()
        .unwrap()
        .iter()
        .filter(|entry| entry.reason == RngReason::Accuracy)
        .count();
    assert!((1..=10).contains(&hits));
    assert_eq!(accuracy_draws, hits + usize::from(hits < 10));
}

#[test]
fn pinned_move_rng_sequences_match_showdown_draw_count_and_order() {
    let dex = champions_reg_mc().unwrap();
    let bullet_seed = (0..1_000)
        .map(|seed| traced_pr_13_4_move(&dex, "bulletseed", seed))
        .find(|result| p1_left_hits(result).len() == 2)
        .expect("two-hit Bullet Seed seed");
    let bullet_trace = move_rng_trace(&bullet_seed);
    assert_eq!(
        bullet_trace
            .iter()
            .map(|entry| entry.reason.clone())
            .collect::<Vec<_>>(),
        [
            RngReason::Accuracy,
            RngReason::MultiHit,
            RngReason::CriticalHit,
            RngReason::DamageRoll,
            RngReason::CriticalHit,
            RngReason::DamageRoll,
        ]
    );
    assert_eq!(
        bullet_trace[0].operation,
        RngOperation::Chance {
            numerator: 100,
            denominator: 100
        }
    );
    assert_eq!(bullet_trace[1].operation, RngOperation::Sample { len: 20 });

    let population_bomb = (0..2_000)
        .map(|seed| traced_pr_13_4_move(&dex, "populationbomb", seed))
        .find(|result| p1_left_hits(result).len() == 2 && move_rng_trace(result).len() == 7)
        .expect("Population Bomb seed with a third-check miss");
    let population_trace = move_rng_trace(&population_bomb);
    assert_eq!(
        population_trace
            .iter()
            .map(|entry| entry.reason.clone())
            .collect::<Vec<_>>(),
        [
            RngReason::Accuracy,
            RngReason::CriticalHit,
            RngReason::DamageRoll,
            RngReason::Accuracy,
            RngReason::CriticalHit,
            RngReason::DamageRoll,
            RngReason::Accuracy,
        ]
    );

    let flamethrower = (0..2_000)
        .map(|seed| traced_pr_13_4_move(&dex, "flamethrower", seed))
        .find(|result| {
            result
                .state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .status
                == Some(MajorStatus::Burn)
        })
        .expect("Flamethrower secondary seed");
    let flamethrower_trace = move_rng_trace(&flamethrower);
    assert_eq!(
        flamethrower_trace
            .iter()
            .map(|entry| entry.reason.clone())
            .collect::<Vec<_>>(),
        [
            RngReason::Accuracy,
            RngReason::CriticalHit,
            RngReason::DamageRoll,
            RngReason::SecondaryEffect,
        ]
    );

    let close_combat = traced_pr_13_4_move(&dex, "closecombat", 0);
    let close_combat_trace = move_rng_trace(&close_combat);
    assert_eq!(
        close_combat_trace
            .iter()
            .map(|entry| entry.reason.clone())
            .collect::<Vec<_>>(),
        [
            RngReason::Accuracy,
            RngReason::CriticalHit,
            RngReason::DamageRoll,
            RngReason::SecondaryEffect,
        ]
    );

    assert_move_rng_operations(&bullet_trace);
    assert_move_rng_operations(&population_trace);
    assert_move_rng_operations(&flamethrower_trace);
    assert_move_rng_operations(&close_combat_trace);
}

#[test]
fn multi_hit_stops_on_ko_and_applies_scale_shot_self_effect_once() {
    let dex = champions_reg_mc().unwrap();
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    let mut builder =
        BattleStateBuilder::from_state(pr_13_4_battle(&dex, "bulletseed", 301)).unwrap();
    builder
        .edit_pokemon(
            PokemonRef::new(SideId::P2, PokemonIndex::new(0)),
            |pokemon| {
                pokemon.current_hp = 1;
            },
        )
        .unwrap()
        .enable_rng_trace();
    let ko = step(
        &builder.build().unwrap(),
        &action,
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .unwrap();
    assert_eq!(p1_left_hits(&ko).len(), 1);
    assert_eq!(
        move_rng_trace(&ko)
            .iter()
            .filter(|entry| entry.reason == RngReason::CriticalHit)
            .count(),
        1
    );
    assert_eq!(
        ko.state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .position,
        PokemonPosition::Fainted
    );

    let scale_shot = (0..1_000)
        .map(|seed| traced_pr_13_4_move(&dex, "scaleshot", seed))
        .find(|result| !p1_left_hits(result).is_empty())
        .expect("Scale Shot hit seed");
    let source = scale_shot
        .state
        .side(SideId::P1)
        .pokemon(PokemonIndex::new(0))
        .unwrap();
    assert_eq!(source.stat_stages.def, -1);
    assert_eq!(source.stat_stages.spe, 1);
    assert_eq!(
        move_rng_trace(&scale_shot)
            .iter()
            .filter(|entry| entry.reason == RngReason::SecondaryEffect)
            .count(),
        1,
        "Showdown applies Scale Shot's selfBoost once after the multi-hit sequence"
    );

    let twineedle = (0..2_000)
        .map(|seed| traced_pr_13_4_move(&dex, "twineedle", seed))
        .find(|result| {
            let rolls = move_rng_trace(result)
                .iter()
                .filter(|entry| entry.reason == RngReason::SecondaryEffect)
                .map(|entry| entry.result)
                .collect::<Vec<_>>();
            rolls.len() == 2 && rolls[0] < 20
        })
        .expect("Twineedle first-hit poison seed");
    assert_eq!(
        move_rng_trace(&twineedle)
            .iter()
            .filter(|entry| entry.reason == RngReason::SecondaryEffect)
            .count(),
        2,
        "each successful hit rolls its own secondary even after the first applies status"
    );
    assert_eq!(
        twineedle
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap()
            .status,
        Some(MajorStatus::Poison)
    );
}

#[test]
fn secondaries_do_not_roll_on_miss_protect_or_type_immunity() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    let source = PokemonRef::new(SideId::P1, PokemonIndex::new(0));
    let target = PokemonRef::new(SideId::P2, PokemonIndex::new(0));

    let miss = (0..2_000)
        .find_map(|seed| {
            let mut builder =
                BattleStateBuilder::from_state(pr_13_4_battle(&dex, "flamethrower", seed)).unwrap();
            builder
                .edit_pokemon(source, |pokemon| pokemon.stat_stages.accuracy = -6)
                .unwrap()
                .edit_pokemon(target, |pokemon| pokemon.stat_stages.evasion = 6)
                .unwrap()
                .enable_rng_trace();
            let result = step(&builder.build().unwrap(), &action, &mechanics).unwrap();
            result
                .diagnostics
                .turn
                .as_ref()
                .unwrap()
                .target_outcomes
                .iter()
                .any(|outcome| outcome.source == source && outcome.result == TargetResult::Missed)
                .then_some(result)
        })
        .expect("deterministic Flamethrower miss");
    assert!(!move_rng_trace(&miss)
        .iter()
        .any(|entry| entry.reason == RngReason::SecondaryEffect));

    let protect = dex.field_effect_ids().protect.expect("Protect condition");
    let mut protected_builder =
        BattleStateBuilder::from_state(pr_13_4_battle(&dex, "flamethrower", 401)).unwrap();
    protected_builder
        .edit_pokemon(target, |pokemon| {
            pokemon
                .volatiles
                .push(ConditionState::new(protect, Some(1)));
        })
        .unwrap()
        .enable_rng_trace();
    let protected = step(&protected_builder.build().unwrap(), &action, &mechanics).unwrap();
    assert!(protected
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .any(|outcome| outcome.source == source && outcome.result == TargetResult::Protected));
    assert!(!move_rng_trace(&protected)
        .iter()
        .any(|entry| entry.reason == RngReason::SecondaryEffect));

    let mut immune_builder = BattleStateBuilder::from_state(pr_13_4_battle_against(
        &dex,
        "thunderbolt",
        "garchomp",
        409,
    ))
    .unwrap();
    immune_builder.enable_rng_trace();
    let immune = step(&immune_builder.build().unwrap(), &action, &mechanics).unwrap();
    assert!(immune
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .any(|outcome| outcome.source == source && outcome.result == TargetResult::Immune));
    assert!(!move_rng_trace(&immune)
        .iter()
        .any(|entry| entry.reason == RngReason::SecondaryEffect));
}

#[test]
fn successful_secondary_rolls_respect_status_immunity_and_fainting() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    let target = PokemonRef::new(SideId::P2, PokemonIndex::new(0));

    let fire_type = (0..2_000)
        .find_map(|seed| {
            let mut builder = BattleStateBuilder::from_state(pr_13_4_battle_against(
                &dex,
                "flamethrower",
                "charizard",
                seed,
            ))
            .unwrap();
            builder.enable_rng_trace();
            let result = step(&builder.build().unwrap(), &action, &mechanics).unwrap();
            move_rng_trace(&result)
                .iter()
                .find(|entry| entry.reason == RngReason::SecondaryEffect)
                .is_some_and(|entry| entry.result < 10)
                .then_some(result)
        })
        .expect("successful burn roll against an immune Fire type");
    assert_eq!(
        fire_type
            .state
            .side(SideId::P2)
            .pokemon(target.pokemon)
            .unwrap()
            .status,
        None
    );

    let mut faint_builder =
        BattleStateBuilder::from_state(pr_13_4_battle(&dex, "flamethrower", 419)).unwrap();
    faint_builder
        .edit_pokemon(target, |pokemon| pokemon.current_hp = 1)
        .unwrap()
        .enable_rng_trace();
    let fainted = step(&faint_builder.build().unwrap(), &action, &mechanics).unwrap();
    assert!(move_rng_trace(&fainted)
        .iter()
        .any(|entry| entry.reason == RngReason::SecondaryEffect));
    let target_state = fainted
        .state
        .side(SideId::P2)
        .pokemon(target.pokemon)
        .unwrap();
    assert_eq!(target_state.status, None);
    assert_eq!(target_state.position, PokemonPosition::Fainted);
}

#[test]
fn drain_and_recoil_use_actual_clamped_damage_with_showdown_rounding() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    for (move_name, expected_delta) in [("gigadrain", 5_i32), ("doubleedge", -3_i32)] {
        let state = pr_13_4_battle(&dex, move_name, 211);
        let mut builder = BattleStateBuilder::from_state(state).unwrap();
        let source = PokemonRef::new(SideId::P1, PokemonIndex::new(0));
        let target = PokemonRef::new(SideId::P2, PokemonIndex::new(0));
        let source_max = builder
            .state()
            .side(SideId::P1)
            .pokemon(source.pokemon)
            .unwrap()
            .stats
            .hp;
        let source_before = if expected_delta > 0 {
            source_max / 2
        } else {
            source_max
        };
        builder
            .edit_pokemon(source, |pokemon| pokemon.current_hp = source_before)
            .unwrap()
            .edit_pokemon(target, |pokemon| pokemon.current_hp = 10)
            .unwrap();
        let result = step(&builder.build().unwrap(), &action, &mechanics).unwrap();
        assert_eq!(p1_left_hits(&result)[0].applied_hp_loss, 10);
        let source_after = result
            .state
            .side(SideId::P1)
            .pokemon(source.pokemon)
            .unwrap()
            .current_hp;
        assert_eq!(
            i32::from(source_after) - i32::from(source_before),
            expected_delta
        );
    }
}

#[test]
fn drain_and_recoil_handle_one_hp_overkill_and_recoil_self_ko() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    let source = PokemonRef::new(SideId::P1, PokemonIndex::new(0));
    let target = PokemonRef::new(SideId::P2, PokemonIndex::new(0));

    let mut drain_builder =
        BattleStateBuilder::from_state(pr_13_4_battle(&dex, "gigadrain", 431)).unwrap();
    let source_max = drain_builder
        .state()
        .side(source.side)
        .pokemon(source.pokemon)
        .unwrap()
        .stats
        .hp;
    drain_builder
        .edit_pokemon(source, |pokemon| pokemon.current_hp = source_max - 5)
        .unwrap()
        .edit_pokemon(target, |pokemon| pokemon.current_hp = 1)
        .unwrap();
    let drained = step(&drain_builder.build().unwrap(), &action, &mechanics).unwrap();
    assert_eq!(p1_left_hits(&drained)[0].applied_hp_loss, 1);
    assert_eq!(
        drained
            .state
            .side(source.side)
            .pokemon(source.pokemon)
            .unwrap()
            .current_hp,
        source_max - 4,
        "half of one actual damage rounds to one HP of drain"
    );

    let mut recoil_builder =
        BattleStateBuilder::from_state(pr_13_4_battle(&dex, "doubleedge", 439)).unwrap();
    recoil_builder
        .edit_pokemon(source, |pokemon| pokemon.current_hp = 1)
        .unwrap()
        .edit_pokemon(target, |pokemon| pokemon.current_hp = 1)
        .unwrap();
    let recoil = step(&recoil_builder.build().unwrap(), &action, &mechanics).unwrap();
    assert_eq!(p1_left_hits(&recoil)[0].applied_hp_loss, 1);
    let source_state = recoil
        .state
        .side(source.side)
        .pokemon(source.pokemon)
        .unwrap();
    assert_eq!(source_state.current_hp, 0);
    assert_eq!(source_state.position, PokemonPosition::Fainted);
    recoil.state.validate_stable_state().unwrap();
}

#[test]
fn multi_hit_drain_rounds_per_hit_while_recoil_uses_total_actual_damage() {
    let dex = champions_reg_mc().unwrap();
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    let source = PokemonRef::new(SideId::P1, PokemonIndex::new(0));
    let base = pr_13_4_battle(&dex, "bulletseed", 0);
    let move_id = base
        .side(source.side)
        .pokemon(source.pokemon)
        .unwrap()
        .moves[0]
        .move_id;

    let drain_data = MoveFractionOverride {
        dex: &dex,
        move_id,
        drain: Some(EffectFraction {
            numerator: 1,
            denominator: 2,
        }),
        recoil: None,
    };
    let drained = (0..1_000)
        .find_map(|seed| {
            let mut builder =
                BattleStateBuilder::from_state(pr_13_4_battle(&dex, "bulletseed", seed)).unwrap();
            let source_max = builder
                .state()
                .side(source.side)
                .pokemon(source.pokemon)
                .unwrap()
                .stats
                .hp;
            builder
                .edit_pokemon(source, |pokemon| pokemon.current_hp = source_max - 100)
                .unwrap();
            let result = step(
                &builder.build().unwrap(),
                &action,
                &Mechanics::new(&drain_data, &EffectRegistry::new()),
            )
            .unwrap();
            (p1_left_hits(&result).len() == 2).then_some((result, source_max))
        })
        .expect("two-hit drain fixture");
    let expected_healing = p1_left_hits(&drained.0)
        .iter()
        .map(|outcome| outcome.applied_hp_loss.div_ceil(2))
        .sum::<u16>();
    assert_eq!(
        drained
            .0
            .state
            .side(source.side)
            .pokemon(source.pokemon)
            .unwrap()
            .current_hp,
        drained.1 - 100 + expected_healing
    );

    let recoil_data = MoveFractionOverride {
        dex: &dex,
        move_id,
        drain: None,
        recoil: Some(EffectFraction {
            numerator: 1,
            denominator: 3,
        }),
    };
    let recoiled = (0..1_000)
        .find_map(|seed| {
            let state = pr_13_4_battle(&dex, "bulletseed", seed);
            let before = state
                .side(source.side)
                .pokemon(source.pokemon)
                .unwrap()
                .current_hp;
            let result = step(
                &state,
                &action,
                &Mechanics::new(&recoil_data, &EffectRegistry::new()),
            )
            .unwrap();
            (p1_left_hits(&result).len() == 2).then_some((result, before))
        })
        .expect("two-hit recoil fixture");
    let total_damage = p1_left_hits(&recoiled.0)
        .iter()
        .map(|outcome| u32::from(outcome.applied_hp_loss))
        .sum::<u32>();
    let expected_recoil = (total_damage + 1) / 3;
    assert_eq!(
        u32::from(recoiled.1)
            - u32::from(
                recoiled
                    .0
                    .state
                    .side(source.side)
                    .pokemon(source.pokemon)
                    .unwrap()
                    .current_hp
            ),
        expected_recoil.max(1)
    );
}

#[test]
fn max_hp_self_damage_runs_after_a_successful_hit() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let action = single_move_action(0, ActionTarget::Foe(ActiveSlot::Left));
    let state = pr_13_4_battle(&dex, "chloroblast", 227);
    let source_before = state
        .side(SideId::P1)
        .pokemon(PokemonIndex::new(0))
        .unwrap()
        .current_hp;
    let result = step(&state, &action, &mechanics).unwrap();
    let source_after = result
        .state
        .side(SideId::P1)
        .pokemon(PokemonIndex::new(0))
        .unwrap()
        .current_hp;
    assert_eq!(source_before - source_after, source_before.div_ceil(2));
}

#[test]
fn pinned_showdown_fixture_covers_pr_13_4_effect_ordering_and_rounding() {
    let fixture = sequenced_effects_fixture();
    assert_eq!(fixture["showdown_commit"], SHOWDOWN_COMMIT);
    let cases = &fixture["cases"];
    assert!(
        cases["facade"]["burned_damage"].as_u64().unwrap()
            > cases["facade"]["clear_damage"].as_u64().unwrap()
    );
    assert_eq!(cases["flamethrower"]["status"], "brn");
    assert!((2..=5).contains(&cases["bullet_seed"]["hits"].as_u64().unwrap()));
    assert_eq!(cases["population_bomb"]["hits"], 2);
    assert_eq!(cases["seismic_toss"]["damage"], 50);
    assert_eq!(cases["close_combat"]["source_boosts"]["def"], -1);
    assert_eq!(cases["close_combat"]["source_boosts"]["spd"], -1);
    assert!(cases["chloroblast"]["self_damage"].as_u64().unwrap() > 0);
    assert_eq!(cases["giga_drain"]["damage"], 10);
    assert_eq!(cases["giga_drain"]["healing"], 5);
    assert_eq!(cases["double_edge"]["damage"], 10);
    assert_eq!(cases["double_edge"]["recoil"], 3);

    let reasons = |case: &str| {
        cases[case]["rng_trace"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["reason"].as_str().unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        reasons("bullet_seed"),
        [
            "accuracy",
            "multi_hit",
            "critical_hit",
            "damage_roll",
            "critical_hit",
            "damage_roll",
        ]
    );
    assert_eq!(
        reasons("population_bomb"),
        [
            "accuracy",
            "critical_hit",
            "damage_roll",
            "accuracy",
            "critical_hit",
            "damage_roll",
            "accuracy",
        ]
    );
    for case in ["flamethrower", "close_combat"] {
        assert_eq!(
            reasons(case),
            [
                "accuracy",
                "critical_hit",
                "damage_roll",
                "secondary_effect",
            ]
        );
    }
}

fn assert_combat_rng_trace(trace: &[rotom_core::RngTraceEntry]) {
    assert_eq!(trace.len(), 15);
    assert!(trace[..3]
        .iter()
        .all(|entry| entry.reason == RngReason::SpeedTie));
    let mut accuracy_numerators = Vec::new();
    for action_trace in trace[3..].chunks_exact(3) {
        assert_eq!(action_trace[0].reason, RngReason::Accuracy);
        let RngOperation::Chance {
            numerator,
            denominator,
        } = action_trace[0].operation
        else {
            panic!("accuracy must use a chance draw")
        };
        assert_eq!(denominator, 100);
        accuracy_numerators.push(numerator);
        assert_eq!(action_trace[1].reason, RngReason::CriticalHit);
        assert_eq!(
            action_trace[1].operation,
            RngOperation::Chance {
                numerator: 1,
                denominator: 24,
            }
        );
        assert_eq!(action_trace[2].reason, RngReason::DamageRoll);
        assert_eq!(
            action_trace[2].operation,
            RngOperation::Range {
                start: 85,
                end_exclusive: 101,
            }
        );
        assert!((85..=100).contains(&action_trace[2].result));
    }
    accuracy_numerators.sort_unstable();
    assert_eq!(accuracy_numerators, [90, 90, 100, 100]);
}

#[test]
fn unsupported_move_capabilities_are_explicit_and_atomic() {
    let dex = champions_reg_mc().expect("generated Dex");
    let (move_name, expected_capability) = ("electroshot", "custom_callback");
    let unsupported = set(
        &dex,
        "goodra",
        "gooey",
        "serious",
        move_name,
        StatAllocation::ZERO,
    );
    let make_side = || {
        let pokemon = dex.pokemon_state(&unsupported).expect("state");
        SideState::new(TeamState::new(vec![
            pokemon.clone(),
            pokemon.clone(),
            pokemon.clone(),
            pokemon,
        ]))
    };
    let mut builder =
        BattleStateBuilder::new_with_seed(make_side(), make_side(), 4, 7).expect("battle");
    let selection = vec![
        PokemonIndex::new(0),
        PokemonIndex::new(1),
        PokemonIndex::new(2),
        PokemonIndex::new(3),
    ];
    builder
        .select_teams(selection.clone(), selection)
        .expect("preview");
    let battle = builder.build().unwrap();
    let targets = [
        ActionTarget::Foe(rotom_core::ActiveSlot::Left),
        ActionTarget::Foe(rotom_core::ActiveSlot::Right),
    ];
    let choice = TurnChoice {
        actions: targets.map(|target| PokemonAction::Move {
            move_slot: MoveSlotIndex::new(0),
            target,
            mega: false,
        }),
    };
    let before = battle.clone();

    let error = step(
        &battle,
        &JointAction::Turn {
            p1: choice,
            p2: choice,
        },
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .unwrap_err();
    assert!(
        matches!(error, rotom_core::BattleError::UnsupportedMechanic(_)),
        "{move_name}: {error}"
    );
    assert!(
        error.to_string().contains(expected_capability),
        "{move_name}: {error}"
    );
    assert_eq!(battle, before, "{move_name} mutated its public input");
}

#[derive(Debug, Deserialize)]
struct MechanicsManifest {
    entries: Vec<MechanicsManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct MechanicsManifestEntry {
    id: String,
    kind: String,
    implementation: String,
}

fn representative_action_target(target: MoveTarget) -> ActionTarget {
    match target {
        MoveTarget::AdjacentAlly => ActionTarget::Ally(ActiveSlot::Right),
        MoveTarget::AdjacentAllyOrSelf => ActionTarget::Self_,
        MoveTarget::AdjacentFoe | MoveTarget::Any | MoveTarget::Normal => {
            ActionTarget::Foe(ActiveSlot::Left)
        }
        MoveTarget::All
        | MoveTarget::AllAdjacent
        | MoveTarget::AllAdjacentFoes
        | MoveTarget::Allies
        | MoveTarget::AllySide
        | MoveTarget::AllyTeam
        | MoveTarget::FoeSide
        | MoveTarget::RandomNormal
        | MoveTarget::Scripted
        | MoveTarget::Self_ => ActionTarget::Automatic,
    }
}

fn manifest_move_battle(dex: &rotom_dex::DexData, move_name: &str, seed: u64) -> BattleState {
    let source = set_with_moves(dex, "goodra", "sapsipper", &[move_name]);
    let target = set_with_moves(dex, "blastoise", "torrent", &["recover"]);
    let state = selected_primary_battle(dex, &source, &target, seed);
    let mut builder = BattleStateBuilder::from_state(state).unwrap();
    for side in [SideId::P1, SideId::P2] {
        for pokemon in [PokemonIndex::new(0), PokemonIndex::new(1)] {
            builder
                .edit_pokemon(PokemonRef::new(side, pokemon), |state| {
                    state.current_hp = state.stats.hp.saturating_div(2).max(1);
                })
                .unwrap();
        }
    }
    builder.build().unwrap()
}

fn primary_move_succeeded(result: &rotom_core::StepResult, move_id: MoveId) -> bool {
    let turn = result.diagnostics.turn.as_ref().expect("turn diagnostics");
    turn.target_outcomes.iter().any(|outcome| {
        outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0))
            && outcome.move_id == move_id
            && outcome.result == TargetResult::Hit
    }) || turn.actions.iter().any(|action| {
        matches!(
            action,
            ResolvedAction::Used {
                side: SideId::P1,
                slot: ActiveSlot::Left,
                move_slot,
            } | ResolvedAction::Damaged {
                side: SideId::P1,
                slot: ActiveSlot::Left,
                move_slot,
                ..
            } if *move_slot == MoveSlotIndex::new(0)
        )
    })
}

#[test]
fn mechanics_manifest_drives_every_move_capability_and_execution_smoke_case() {
    let manifest: MechanicsManifest = serde_json::from_str(include_str!(
        "../../../data/generated/reg-mc-mechanics-manifest.json"
    ))
    .expect("mechanics manifest");
    let dex = champions_reg_mc().expect("generated Dex");
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let moves = manifest
        .entries
        .iter()
        .filter(|entry| entry.kind == "move")
        .collect::<Vec<_>>();
    let implemented = moves
        .iter()
        .filter(|entry| entry.implementation == "implemented")
        .count();
    let unsupported = moves
        .iter()
        .filter(|entry| entry.implementation == "unsupported")
        .count();
    assert_eq!(implemented, 223);
    assert_eq!(unsupported, 287);

    for entry in moves {
        let move_id = dex
            .resolve_move(&entry.id)
            .unwrap_or_else(|| panic!("manifest move {} missing from Dex", entry.id))
            .0;
        let move_data = <rotom_dex::DexData as CombatData>::move_data(&dex, move_id)
            .unwrap_or_else(|| panic!("manifest move {} missing combat data", entry.id));
        let missing = move_data
            .required_capabilities
            .first_missing_from(MoveCapabilities::PR_13_4);
        let target = representative_action_target(move_data.target);

        match entry.implementation.as_str() {
            "implemented" => {
                assert_eq!(missing, None, "{} manifest/runtime support drift", entry.id);
                let successful = (0..1_024).find_map(|seed| {
                    let state = manifest_move_battle(&dex, &entry.id, seed);
                    let result = step(&state, &single_move_action(0, target), &mechanics)
                        .unwrap_or_else(|error| {
                            panic!("implemented move {} failed: {error}", entry.id)
                        });
                    assert_stable_serializable_state(&result.state, &entry.id);
                    primary_move_succeeded(&result, move_id).then_some(result)
                });
                assert!(
                    successful.is_some(),
                    "implemented move {} had no successful execution in the deterministic seed sweep",
                    entry.id
                );
            }
            "unsupported" => {
                assert!(
                    missing.is_some(),
                    "{} is unsupported in the manifest but runtime preflight has no missing capability",
                    entry.id
                );
                let state = manifest_move_battle(&dex, &entry.id, 0x13_03);
                let before = state.clone();
                let error = step(&state, &single_move_action(0, target), &mechanics)
                    .expect_err("unsupported move must fail preflight");
                assert!(
                    matches!(error, rotom_core::BattleError::UnsupportedMechanic(_)),
                    "{} returned the wrong capability-gate error: {error}",
                    entry.id
                );
                assert_eq!(state, before, "{} mutated the public input", entry.id);
            }
            implementation => panic!(
                "move {} has unhandled manifest implementation {implementation}",
                entry.id
            ),
        }
    }
}

#[test]
fn combat_damage_processes_faints_and_requests_replacements() {
    let dex = champions_reg_mc().expect("generated Dex");
    let attacker = set(
        &dex,
        "glimmora",
        "corrosion",
        "modest",
        "powergem",
        StatAllocation {
            spa: 32,
            ..StatAllocation::ZERO
        },
    );
    let defender = set(
        &dex,
        "charizard",
        "blaze",
        "serious",
        "powergem",
        StatAllocation::ZERO,
    );
    let make_side = |set: &PokemonSet| {
        let pokemon = dex.pokemon_state(set).expect("state");
        SideState::new(TeamState::new(vec![
            pokemon.clone(),
            pokemon.clone(),
            pokemon.clone(),
            pokemon,
        ]))
    };
    let mut builder =
        BattleStateBuilder::new_with_seed(make_side(&attacker), make_side(&defender), 4, 31)
            .expect("battle");
    let selection = vec![
        PokemonIndex::new(0),
        PokemonIndex::new(1),
        PokemonIndex::new(2),
        PokemonIndex::new(3),
    ];
    builder
        .select_teams(selection.clone(), selection)
        .expect("preview");
    let battle = builder.build().unwrap();
    let attack = |target| PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Foe(target),
        mega: false,
    };
    let result = step(
        &battle,
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    attack(rotom_core::ActiveSlot::Left),
                    attack(rotom_core::ActiveSlot::Right),
                ],
            },
            p2: TurnChoice {
                actions: [
                    attack(rotom_core::ActiveSlot::Left),
                    attack(rotom_core::ActiveSlot::Right),
                ],
            },
        },
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .expect("resolution");
    let battle = result.state;

    assert_eq!(battle.phase(), BattlePhase::ForcedSwitch);
    assert!(battle
        .side(SideId::P2)
        .pokemon(PokemonIndex::new(0))
        .expect("target")
        .is_fainted());
    assert!(battle
        .side(SideId::P2)
        .pokemon(PokemonIndex::new(1))
        .expect("target")
        .is_fainted());
    assert_eq!(
        battle
            .replacement_request(SideId::P2)
            .expect("request")
            .expect("needed")
            .required,
        2
    );
}

fn ability_test_damage(
    dex: &rotom_dex::DexData,
    source_ability: &str,
    move_name: &str,
    target_ability: &str,
    source_status: Option<MajorStatus>,
    target_status: Option<MajorStatus>,
    chip_target: bool,
) -> u16 {
    ability_test_damage_case(
        dex,
        source_ability,
        move_name,
        target_ability,
        source_status,
        target_status,
        chip_target,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn ability_test_damage_case(
    dex: &rotom_dex::DexData,
    source_ability: &str,
    move_name: &str,
    target_ability: &str,
    source_status: Option<MajorStatus>,
    target_status: Option<MajorStatus>,
    chip_target: bool,
    expected_critical: Option<bool>,
) -> u16 {
    let source = set_with_moves(dex, "medicham", source_ability, &[move_name]);
    let target = set_with_moves(dex, "snorlax", target_ability, &["recover"]);
    let attack = |target| PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Foe(target),
        mega: false,
    };
    let recovery = PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Automatic,
        mega: false,
    };
    let action = JointAction::Turn {
        p1: TurnChoice {
            actions: [attack(ActiveSlot::Left), attack(ActiveSlot::Right)],
        },
        p2: TurnChoice {
            actions: [recovery, recovery],
        },
    };
    for seed in 0..10_000 {
        let battle = selected_battle_for_sets(dex, &source, &target, seed, true);
        let mut builder = BattleStateBuilder::from_state(battle).unwrap();
        for index in [PokemonIndex::new(0), PokemonIndex::new(1)] {
            builder
                .edit_pokemon(PokemonRef::new(SideId::P1, index), |pokemon| {
                    pokemon.status = source_status;
                })
                .unwrap();
            builder
                .edit_pokemon(PokemonRef::new(SideId::P2, index), |pokemon| {
                    pokemon.status = target_status;
                    if chip_target {
                        pokemon.current_hp = pokemon.current_hp.saturating_sub(1);
                    }
                })
                .unwrap();
        }
        let battle = builder.build().unwrap();
        let result = step(
            &battle,
            &action,
            &Mechanics::new(dex, &EffectRegistry::new()),
        )
        .expect("ability modifier turn");
        let first_roll = result.state.rng().trace().and_then(|trace| {
            trace
                .iter()
                .find(|entry| entry.reason == RngReason::DamageRoll)
                .map(|entry| entry.result)
        });
        let first_critical = result.state.rng().trace().and_then(|trace| {
            trace
                .iter()
                .find(|entry| entry.reason == RngReason::CriticalHit)
                .map(|entry| match entry.operation {
                    RngOperation::Chance { numerator, .. } => entry.result < numerator,
                    _ => unreachable!("critical-hit traces use chance operations"),
                })
        });
        let expected_critical_matches = match expected_critical {
            Some(expected) => first_critical == Some(expected),
            None => true,
        };
        if first_roll == Some(100) && expected_critical_matches {
            return result
                .diagnostics
                .turn
                .unwrap()
                .actions
                .into_iter()
                .find_map(|action| match action {
                    ResolvedAction::Damaged {
                        side: SideId::P1,
                        damage,
                        ..
                    } => Some(damage.final_damage),
                    _ => None,
                })
                .expect("source damage");
        }
    }
    panic!("no seed produced a 100% first damage roll")
}

fn ability_test_speed(
    dex: &rotom_dex::DexData,
    ability: &str,
    status: Option<MajorStatus>,
    weather: Option<&str>,
    terrain: Option<&str>,
) -> u16 {
    let source = set_with_moves(dex, "medicham", ability, &["tackle"]);
    let target = set_with_moves(dex, "snorlax", "immunity", &["recover"]);
    let battle = selected_battle_for_sets(dex, &source, &target, 0, false);
    let mut builder = BattleStateBuilder::from_state(battle).unwrap();
    if let Some(status) = status {
        builder
            .edit_pokemon(
                PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
                |pokemon| pokemon.status = Some(status),
            )
            .unwrap();
    }
    let ids = dex.field_effect_ids();
    if let Some(weather) = weather {
        let condition = match weather {
            "sunnyday" => ids.sun,
            "raindance" => ids.rain,
            "snowscape" => ids.snow,
            "sandstorm" => ids.sandstorm,
            other => panic!("unsupported fixture weather {other}"),
        }
        .unwrap();
        builder.edit_field(|field| {
            field.weather = Some(ConditionState::new(condition, Some(5)));
        });
    }
    if let Some(terrain) = terrain {
        let condition = match terrain {
            "electricterrain" => ids.electric_terrain,
            other => panic!("unsupported fixture terrain {other}"),
        }
        .unwrap();
        builder.edit_field(|field| {
            field.terrain = Some(ConditionState::new(condition, Some(5)));
        });
    }
    let battle = builder.build().unwrap();
    let action = TurnChoice {
        actions: [
            use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
            use_move(0, ActionTarget::Foe(ActiveSlot::Right)),
        ],
    };
    let queue = battle
        .inspect_turn_queue(
            action,
            TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Automatic),
                    use_move(0, ActionTarget::Automatic),
                ],
            },
            &Mechanics::new(dex, &EffectRegistry::new()),
        )
        .unwrap();
    queue
        .actions
        .iter()
        .find(|action| action.side == SideId::P1 && action.slot == ActiveSlot::Left)
        .unwrap()
        .speed
}

fn ability_test_try_hit(
    dex: &rotom_dex::DexData,
    source_ability: &str,
    target_ability: &str,
    move_name: &str,
    ally_target: bool,
) -> bool {
    let source = set_with_moves(dex, "medicham", source_ability, &[move_name]);
    let target = set_with_moves(dex, "snorlax", target_ability, &["recover"]);
    let ally = set_with_moves(dex, "magikarp", target_ability, &["tackle"]);
    let passive = set_with_moves(dex, "snorlax", "immunity", &["recover"]);
    let battle = if ally_target {
        selected_battle_for_teams(
            dex,
            &four_member_team(source, ally),
            &four_member_team(passive.clone(), passive),
            0,
        )
    } else {
        selected_battle_for_teams(
            dex,
            &four_member_team(
                source,
                set_with_moves(dex, "magikarp", "swiftswim", &["tackle"]),
            ),
            &four_member_team(target, passive),
            0,
        )
    };
    let target_ref = PokemonRef::new(
        if ally_target { SideId::P1 } else { SideId::P2 },
        PokemonIndex::new(u8::from(ally_target)),
    );
    let result = step(
        &battle,
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    use_move(
                        0,
                        if ally_target || move_name == "hypervoice" {
                            ActionTarget::Automatic
                        } else {
                            ActionTarget::Foe(ActiveSlot::Left)
                        },
                    ),
                    use_move(0, ActionTarget::Foe(ActiveSlot::Right)),
                ],
            },
            p2: TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Automatic),
                    use_move(0, ActionTarget::Automatic),
                ],
            },
        },
        &Mechanics::new(dex, &EffectRegistry::new()),
    )
    .unwrap();
    let turn = result.diagnostics.turn.as_ref().unwrap();
    let allowed = turn
        .events
        .iter()
        .find_map(|event| {
            (event.event == BattleEvent::TryHit
                && event.context.source == Some(PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
                && event.context.target == Some(target_ref))
            .then_some(event.value == rotom_core::EventValue::Boolean(true))
        })
        .unwrap();
    let outcome = turn
        .target_outcomes
        .iter()
        .find(|outcome| {
            outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0))
                && outcome.target == target_ref
        })
        .unwrap();
    if allowed {
        assert_eq!(outcome.result, TargetResult::Hit);
        assert!(outcome.applied_hp_loss > 0);
    } else {
        assert_eq!(outcome.result, TargetResult::Immune);
        assert_eq!(outcome.applied_hp_loss, 0);
    }
    allowed
}

fn ability_test_critical_ratio(dex: &rotom_dex::DexData, ability: &str) -> i64 {
    let source = set_with_moves(dex, "medicham", ability, &["dragonclaw"]);
    let target = set_with_moves(dex, "snorlax", "immunity", &["recover"]);
    let action = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
                use_move(0, ActionTarget::Foe(ActiveSlot::Right)),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(0, ActionTarget::Automatic),
            ],
        },
    };
    let battle = selected_battle_for_sets(dex, &source, &target, 0, false);
    let result = step(
        &battle,
        &action,
        &Mechanics::new(dex, &EffectRegistry::new()),
    )
    .unwrap();
    result
        .diagnostics
        .turn
        .unwrap()
        .events
        .into_iter()
        .find_map(|event| {
            (event.event == BattleEvent::ModifyCritRatio
                && event.context.source == Some(PokemonRef::new(SideId::P1, PokemonIndex::new(0))))
            .then_some(event.value)
        })
        .and_then(|value| match value {
            rotom_core::EventValue::Integer(value) => Some(value),
            _ => None,
        })
        .unwrap()
}

fn ability_test_priority(
    dex: &rotom_dex::DexData,
    ability: &str,
    move_name: &str,
    target_species: &str,
) -> serde_json::Value {
    let source = set_with_moves(dex, "medicham", ability, &[move_name]);
    let target = set_with_moves(dex, target_species, "immunity", &["recover"]);
    let battle = selected_battle_for_sets(dex, &source, &target, 0, false);
    let p1 = opposing_targets_choice();
    let p2 = TurnChoice {
        actions: [
            use_move(0, ActionTarget::Automatic),
            use_move(0, ActionTarget::Automatic),
        ],
    };
    let mechanics = Mechanics::new(dex, &EffectRegistry::new());
    let queue = battle.inspect_turn_queue(p1, p2, &mechanics).unwrap();
    let queued = queue
        .actions
        .iter()
        .find(|action| action.side == SideId::P1 && action.slot == ActiveSlot::Left)
        .unwrap();
    let result = step(&battle, &JointAction::Turn { p1, p2 }, &mechanics).unwrap();
    let dark_immune = result
        .diagnostics
        .turn
        .unwrap()
        .target_outcomes
        .iter()
        .any(|outcome| {
            outcome.source == PokemonRef::new(SideId::P1, PokemonIndex::new(0))
                && outcome.target == PokemonRef::new(SideId::P2, PokemonIndex::new(0))
                && outcome.result == TargetResult::Immune
        });
    serde_json::json!({
        "priority": queued.priority,
        "prankster_boosted": queued.prankster_boosted,
        "dark_immune": dark_immune,
    })
}

fn ability_test_weather_immunity(dex: &rotom_dex::DexData, ability: &str, weather: &str) -> bool {
    let source = set_with_moves(dex, "medicham", ability, &["tackle"]);
    let target = set_with_moves(dex, "snorlax", "immunity", &["recover"]);
    let battle = selected_battle_for_sets(dex, &source, &target, 0, false);
    let mut builder = BattleStateBuilder::from_state(battle).unwrap();
    let condition = match weather {
        "sandstorm" => dex.field_effect_ids().sandstorm,
        other => panic!("unsupported fixture weather immunity {other}"),
    }
    .unwrap();
    builder.edit_field(|field| {
        field.weather = Some(ConditionState::new(condition, Some(2)));
    });
    let result = step(
        &builder.build().unwrap(),
        &JointAction::Turn {
            p1: opposing_targets_choice(),
            p2: TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Automatic),
                    use_move(0, ActionTarget::Automatic),
                ],
            },
        },
        &Mechanics::new(dex, &EffectRegistry::new()),
    )
    .unwrap();
    result.diagnostics.turn.unwrap().events.iter().any(|event| {
        event.event == BattleEvent::WeatherImmunity
            && event.context.target == Some(PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
            && event.value == rotom_core::EventValue::Boolean(false)
    })
}

#[test]
fn built_in_ability_modifiers_compose_with_the_unified_executor() {
    let dex = champions_reg_mc().unwrap();
    let neutral_physical = ability_test_damage(
        &dex,
        "telepathy",
        "dragonclaw",
        "immunity",
        None,
        None,
        false,
    );
    let huge_power = ability_test_damage(
        &dex,
        "hugepower",
        "dragonclaw",
        "immunity",
        None,
        None,
        false,
    );
    assert!(huge_power > neutral_physical);

    let neutral_special = ability_test_damage(
        &dex,
        "telepathy",
        "dragonpulse",
        "immunity",
        None,
        None,
        false,
    );
    let huge_power_special = ability_test_damage(
        &dex,
        "hugepower",
        "dragonpulse",
        "immunity",
        None,
        None,
        false,
    );
    assert_eq!(huge_power_special, neutral_special);

    let burned = ability_test_damage(
        &dex,
        "telepathy",
        "dragonclaw",
        "immunity",
        Some(MajorStatus::Burn),
        None,
        false,
    );
    let guts = ability_test_damage(
        &dex,
        "guts",
        "dragonclaw",
        "immunity",
        Some(MajorStatus::Burn),
        None,
        false,
    );
    assert!(guts > burned.saturating_mul(2));

    let weak_neutral =
        ability_test_damage(&dex, "telepathy", "tackle", "immunity", None, None, false);
    let technician =
        ability_test_damage(&dex, "technician", "tackle", "immunity", None, None, false);
    assert!(technician > weak_neutral);

    let multiscale = ability_test_damage(
        &dex,
        "telepathy",
        "dragonclaw",
        "multiscale",
        None,
        None,
        false,
    );
    let chipped_multiscale = ability_test_damage(
        &dex,
        "telepathy",
        "dragonclaw",
        "multiscale",
        None,
        None,
        true,
    );
    assert!(chipped_multiscale > multiscale);

    let marvel_scale = ability_test_damage(
        &dex,
        "telepathy",
        "dragonclaw",
        "marvelscale",
        None,
        Some(MajorStatus::Poison),
        false,
    );
    assert!(marvel_scale < neutral_physical);
}

#[test]
fn ability_modifier_damage_matches_pinned_showdown_fixture() {
    let dex = champions_reg_mc().unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/ability-modifiers.json"
    ))
    .unwrap();
    assert_eq!(fixture["showdown_commit"], SHOWDOWN_COMMIT);
    for case in fixture["cases"].as_array().unwrap() {
        let status = |value: &serde_json::Value| match value.as_str() {
            Some("brn") => Some(MajorStatus::Burn),
            Some("par") => Some(MajorStatus::Paralysis),
            Some("psn") => Some(MajorStatus::Poison),
            Some(other) => panic!("unsupported fixture status {other}"),
            None => None,
        };
        let actual = match case["operation"].as_str().unwrap() {
            "damage" => serde_json::json!(ability_test_damage_case(
                &dex,
                case["source_ability"].as_str().unwrap(),
                case["move"].as_str().unwrap(),
                case["target_ability"].as_str().unwrap(),
                status(&case["source_status"]),
                status(&case["target_status"]),
                false,
                case["critical"].as_bool(),
            )),
            "speed" => serde_json::json!(ability_test_speed(
                &dex,
                case["ability"].as_str().unwrap(),
                status(&case["status"]),
                case["weather"].as_str(),
                case["terrain"].as_str(),
            )),
            "critical_ratio" => serde_json::json!(ability_test_critical_ratio(
                &dex,
                case["ability"].as_str().unwrap(),
            )),
            "try_hit" => serde_json::json!(ability_test_try_hit(
                &dex,
                case["source_ability"].as_str().unwrap(),
                case["target_ability"].as_str().unwrap(),
                case["move"].as_str().unwrap(),
                case["ally_target"].as_bool().unwrap(),
            )),
            "priority" => ability_test_priority(
                &dex,
                case["ability"].as_str().unwrap(),
                case["move"].as_str().unwrap(),
                case["target_species"].as_str().unwrap(),
            ),
            "weather_immunity" => serde_json::json!(ability_test_weather_immunity(
                &dex,
                case["ability"].as_str().unwrap(),
                case["weather"].as_str().unwrap(),
            )),
            operation => panic!("unsupported ability fixture operation {operation}"),
        };
        let expected = match case["operation"].as_str().unwrap() {
            "damage" => case["damage"].clone(),
            "speed" => case["speed"].clone(),
            "critical_ratio" => case["critical_ratio"].clone(),
            "try_hit" => case["allowed"].clone(),
            "priority" => serde_json::json!({
                "priority": case["priority"].clone(),
                "prankster_boosted": case["prankster_boosted"].clone(),
                "dark_immune": case["dark_immune"].clone(),
            }),
            "weather_immunity" => case["immune"].clone(),
            _ => unreachable!(),
        };
        assert_eq!(actual, expected, "{}", case["id"].as_str().unwrap());
    }
}

#[test]
fn ability_batch_interactions_replay_deterministically() {
    let dex = champions_reg_mc().unwrap();
    let p1 = four_member_team(
        set_with_moves(&dex, "medicham", "quickfeet", &["tackle"]),
        set_with_moves(&dex, "medicham", "superluck", &["dragonclaw"]),
    );
    let p2 = four_member_team(
        set_with_moves(&dex, "snorlax", "battlearmor", &["recover"]),
        set_with_moves(&dex, "snorlax", "shellarmor", &["recover"]),
    );
    let action = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Foe(ActiveSlot::Left)),
                use_move(0, ActionTarget::Foe(ActiveSlot::Right)),
            ],
        },
        p2: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(0, ActionTarget::Automatic),
            ],
        },
    };
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let (state, first) = (0..10_000)
        .find_map(|seed| {
            let battle = selected_battle_for_teams(&dex, &p1, &p2, seed);
            let mut builder = BattleStateBuilder::from_state(battle).unwrap();
            builder
                .edit_pokemon(
                    PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
                    |pokemon| pokemon.status = Some(MajorStatus::Paralysis),
                )
                .unwrap();
            let state = builder.build().unwrap();
            let result = step(&state, &action, &mechanics).unwrap();
            let armor_prevented_critical = result
                .diagnostics
                .turn
                .as_ref()
                .unwrap()
                .events
                .iter()
                .any(|event| {
                    event.event == BattleEvent::CriticalHit
                        && event.context.source
                            == Some(PokemonRef::new(SideId::P1, PokemonIndex::new(1)))
                        && event.value == rotom_core::EventValue::Boolean(false)
                });
            armor_prevented_critical.then_some((state, result))
        })
        .expect("a seed should exercise Shell Armor critical prevention");

    let quick_feet_speed = first
        .diagnostics
        .queue_events
        .iter()
        .find(|event| {
            event.event == BattleEvent::ModifySpeed
                && event.context.source == Some(PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
        })
        .unwrap();
    assert_eq!(quick_feet_speed.value, rotom_core::EventValue::Integer(150));
    assert_eq!(step(&state, &action, &mechanics).unwrap(), first);
}

#[test]
fn sand_rush_tracks_weather_changes_and_prevents_residual_damage() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());

    let sand_source = set_with_moves(&dex, "houndstone", "sandrush", &["tackle"]);
    let sand_target = set_with_moves(&dex, "snorlax", "immunity", &["recover"]);
    let clear = selected_battle_for_sets(&dex, &sand_source, &sand_target, 7, true);
    let choices = (
        opposing_targets_choice(),
        TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(0, ActionTarget::Automatic),
            ],
        },
    );
    let clear_speed = clear
        .inspect_turn_queue(choices.0, choices.1, &mechanics)
        .unwrap()
        .actions
        .into_iter()
        .find(|action| action.side == SideId::P1 && action.slot == ActiveSlot::Left)
        .unwrap()
        .speed;
    let mut builder = BattleStateBuilder::from_state(clear).unwrap();
    builder.edit_field(|field| {
        field.weather = Some(ConditionState::new(
            dex.field_effect_ids().sandstorm.unwrap(),
            Some(1),
        ));
    });
    let sand = builder.build().unwrap();
    let sand_queue = sand
        .inspect_turn_queue(choices.0, choices.1, &mechanics)
        .unwrap();
    let sand_action = sand_queue
        .actions
        .iter()
        .find(|action| action.side == SideId::P1 && action.slot == ActiveSlot::Left)
        .unwrap();
    assert_eq!(sand_action.speed, clear_speed.saturating_mul(2));
    let sand_result = step(
        &sand,
        &JointAction::Turn {
            p1: choices.0,
            p2: choices.1,
        },
        &mechanics,
    )
    .unwrap();
    assert_eq!(
        step(
            &sand,
            &JointAction::Turn {
                p1: choices.0,
                p2: choices.1,
            },
            &mechanics,
        )
        .unwrap(),
        sand_result
    );
    let sand_turn = sand_result.diagnostics.turn.as_ref().unwrap();
    assert!(sand_turn
        .events
        .iter()
        .any(|event| event.event == BattleEvent::WeatherImmunity
            && event.context.target == Some(PokemonRef::new(SideId::P1, PokemonIndex::new(0)))
            && event.value == rotom_core::EventValue::Boolean(false)));
    assert!(!sand_turn
        .field_hp_changes
        .iter()
        .any(|change| change.pokemon.side == SideId::P1));
    assert!(sand_result.state.field().weather.is_none());
    let expired_speed = sand_result
        .state
        .inspect_turn_queue(choices.0, choices.1, &mechanics)
        .unwrap()
        .actions
        .into_iter()
        .find(|action| action.side == SideId::P1 && action.slot == ActiveSlot::Left)
        .unwrap()
        .speed;
    assert_eq!(expired_speed, clear_speed);
}

#[test]
fn prankster_priority_and_dark_immunity_are_per_target_and_replayable() {
    let dex = champions_reg_mc().unwrap();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let p1 = four_member_team(
        set_with_moves(&dex, "medicham", "prankster", &["stringshot"]),
        set_with_moves(&dex, "snorlax", "immunity", &["recover"]),
    );
    let p2 = four_member_team(
        set_with_moves(&dex, "umbreon", "immunity", &["recover"]),
        set_with_moves(&dex, "snorlax", "immunity", &["recover"]),
    );
    let prankster = selected_battle_for_teams(&dex, &p1, &p2, 19);
    let p1_choice = TurnChoice {
        actions: [
            use_move(0, ActionTarget::Automatic),
            use_move(0, ActionTarget::Automatic),
        ],
    };
    let p2_choice = TurnChoice {
        actions: [
            use_move(0, ActionTarget::Automatic),
            use_move(0, ActionTarget::Automatic),
        ],
    };
    let prankster_action = JointAction::Turn {
        p1: p1_choice,
        p2: p2_choice,
    };
    let queue = prankster
        .inspect_turn_queue(p1_choice, p2_choice, &mechanics)
        .unwrap();
    let queued = queue
        .actions
        .iter()
        .find(|action| action.side == SideId::P1 && action.slot == ActiveSlot::Left)
        .unwrap();
    assert_eq!(queued.priority, 1);
    assert!(queued.prankster_boosted);

    let result = step(&prankster, &prankster_action, &mechanics).unwrap();
    assert_eq!(
        step(&prankster, &prankster_action, &mechanics).unwrap(),
        result
    );
    let turn = result.diagnostics.turn.as_ref().unwrap();
    let source = PokemonRef::new(SideId::P1, PokemonIndex::new(0));
    let outcomes = turn
        .target_outcomes
        .iter()
        .filter(|outcome| outcome.source == source)
        .map(|outcome| (outcome.target, outcome.result))
        .collect::<Vec<_>>();
    assert!(outcomes.contains(&(
        PokemonRef::new(SideId::P2, PokemonIndex::new(0)),
        TargetResult::Immune
    )));
    assert!(outcomes.contains(&(
        PokemonRef::new(SideId::P2, PokemonIndex::new(1)),
        TargetResult::Hit
    )));
    assert_eq!(
        result
            .state
            .rng()
            .trace()
            .unwrap()
            .iter()
            .filter(|entry| entry.reason == RngReason::Accuracy)
            .count(),
        1
    );

    let protected_p2 = four_member_team(
        set_with_moves(&dex, "umbreon", "immunity", &["protect"]),
        set_with_moves(&dex, "snorlax", "immunity", &["recover"]),
    );
    let protected = step(
        &selected_battle_for_teams(&dex, &p1, &protected_p2, 29),
        &prankster_action,
        &mechanics,
    )
    .unwrap();
    let protected_turn = protected.diagnostics.turn.as_ref().unwrap();
    assert!(protected_turn.target_outcomes.iter().any(|outcome| {
        outcome.source == source
            && outcome.target == PokemonRef::new(SideId::P2, PokemonIndex::new(0))
            && outcome.result == TargetResult::Protected
    }));
    assert!(!protected_turn.events.iter().any(|event| {
        event.event == BattleEvent::TryImmunity
            && event.context.target == Some(PokemonRef::new(SideId::P2, PokemonIndex::new(0)))
    }));
}

#[test]
fn protection_precedes_ability_immunity_per_target() {
    let dex = champions_reg_mc().unwrap();
    let p1 = four_member_team(
        set_with_moves(&dex, "medicham", "telepathy", &["hypervoice"]),
        set_with_moves(&dex, "snorlax", "immunity", &["recover"]),
    );
    let p2 = four_member_team(
        set_with_moves(&dex, "snorlax", "soundproof", &["protect"]),
        set_with_moves(&dex, "snorlax", "soundproof", &["recover"]),
    );
    let battle = selected_battle_for_teams(&dex, &p1, &p2, 23);
    let result = step(
        &battle,
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Automatic),
                    use_move(0, ActionTarget::Automatic),
                ],
            },
            p2: TurnChoice {
                actions: [
                    use_move(0, ActionTarget::Automatic),
                    use_move(0, ActionTarget::Automatic),
                ],
            },
        },
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .unwrap();
    let source = PokemonRef::new(SideId::P1, PokemonIndex::new(0));
    let outcomes = result
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .filter(|outcome| outcome.source == source)
        .map(|outcome| (outcome.target, outcome.result))
        .collect::<Vec<_>>();
    assert!(outcomes.contains(&(
        PokemonRef::new(SideId::P2, PokemonIndex::new(0)),
        TargetResult::Protected
    )));
    assert!(outcomes.contains(&(
        PokemonRef::new(SideId::P2, PokemonIndex::new(1)),
        TargetResult::Immune
    )));
}

#[test]
fn critical_prevention_preserves_showdown_event_and_rng_order() {
    let dex = champions_reg_mc().unwrap();
    let source = set_with_moves(&dex, "medicham", "superluck", &["dragonclaw"]);
    let target = set_with_moves(&dex, "snorlax", "battlearmor", &["recover"]);
    let action = JointAction::Turn {
        p1: opposing_targets_choice(),
        p2: TurnChoice {
            actions: [
                use_move(0, ActionTarget::Automatic),
                use_move(0, ActionTarget::Automatic),
            ],
        },
    };
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let (state, result) = (0..10_000)
        .find_map(|seed| {
            let state = selected_battle_for_sets(&dex, &source, &target, seed, true);
            let result = step(&state, &action, &mechanics).unwrap();
            result
                .state
                .rng()
                .trace()
                .unwrap()
                .iter()
                .any(|entry| {
                    entry.reason == RngReason::CriticalHit
                        && matches!(entry.operation, RngOperation::Chance { numerator, .. } if entry.result < numerator)
                })
                .then_some((state, result))
        })
        .expect("a deterministic seed should roll a critical hit");

    let trace = result.state.rng().trace().unwrap();
    let critical_rng = trace
        .iter()
        .position(|entry| entry.reason == RngReason::CriticalHit)
        .unwrap();
    let damage_rng = trace
        .iter()
        .position(|entry| entry.reason == RngReason::DamageRoll)
        .unwrap();
    assert!(critical_rng < damage_rng);

    let source_ref = PokemonRef::new(SideId::P1, PokemonIndex::new(0));
    let target_ref = PokemonRef::new(SideId::P2, PokemonIndex::new(0));
    let turn = result.diagnostics.turn.as_ref().unwrap();
    let event_position = |event| {
        turn.events.iter().position(|dispatch| {
            dispatch.event == event
                && dispatch.context.source == Some(source_ref)
                && dispatch.context.target == Some(target_ref)
        })
    };
    let ratio = event_position(BattleEvent::ModifyCritRatio).unwrap();
    let prevention = event_position(BattleEvent::CriticalHit).unwrap();
    let power = event_position(BattleEvent::ModifyBasePower).unwrap();
    let damage = event_position(BattleEvent::ModifyDamage).unwrap();
    assert!(ratio < prevention && prevention < power && power < damage);
    assert_eq!(turn.events[ratio].value, rotom_core::EventValue::Integer(2));
    assert_eq!(
        turn.events[prevention].value,
        rotom_core::EventValue::Boolean(false)
    );
    assert_eq!(step(&state, &action, &mechanics).unwrap(), result);
}

use rotom_core::AbilitySuppression::{Breakable, Unbreakable};
const EXPECTED_ABILITY_HANDLERS: &[(
    &str,
    rotom_core::AbilityBehavior,
    rotom_core::AbilitySuppression,
)] = &[
    (
        "hugepower",
        rotom_core::AbilityBehavior::DoubleAttack,
        Unbreakable,
    ),
    (
        "purepower",
        rotom_core::AbilityBehavior::DoubleAttack,
        Unbreakable,
    ),
    (
        "guts",
        rotom_core::AbilityBehavior::StatusedAttack,
        Unbreakable,
    ),
    (
        "furcoat",
        rotom_core::AbilityBehavior::DoubleDefense,
        Breakable,
    ),
    (
        "marvelscale",
        rotom_core::AbilityBehavior::StatusedDefense,
        Breakable,
    ),
    (
        "technician",
        rotom_core::AbilityBehavior::LowPowerBoost,
        Unbreakable,
    ),
    (
        "multiscale",
        rotom_core::AbilityBehavior::FullHpDamageReduction,
        Breakable,
    ),
    (
        "quickfeet",
        rotom_core::AbilityBehavior::StatusedSpeed,
        Unbreakable,
    ),
    (
        "quickfeet",
        rotom_core::AbilityBehavior::IgnoreParalysisSpeedPenalty,
        Unbreakable,
    ),
    (
        "superluck",
        rotom_core::AbilityBehavior::IncreaseCriticalRatio,
        Unbreakable,
    ),
    (
        "battlearmor",
        rotom_core::AbilityBehavior::PreventCriticalHit,
        Breakable,
    ),
    (
        "shellarmor",
        rotom_core::AbilityBehavior::PreventCriticalHit,
        Breakable,
    ),
    (
        "sniper",
        rotom_core::AbilityBehavior::BoostCriticalDamage,
        Unbreakable,
    ),
    (
        "moldbreaker",
        rotom_core::AbilityBehavior::BreaksTargetAbilities,
        Unbreakable,
    ),
    (
        "chlorophyll",
        rotom_core::AbilityBehavior::SunSpeed,
        Unbreakable,
    ),
    (
        "swiftswim",
        rotom_core::AbilityBehavior::RainSpeed,
        Unbreakable,
    ),
    (
        "slushrush",
        rotom_core::AbilityBehavior::SnowSpeed,
        Unbreakable,
    ),
    (
        "surgesurfer",
        rotom_core::AbilityBehavior::ElectricTerrainSpeed,
        Unbreakable,
    ),
    (
        "sandrush",
        rotom_core::AbilityBehavior::SandRush,
        Unbreakable,
    ),
    (
        "prankster",
        rotom_core::AbilityBehavior::Prankster,
        Unbreakable,
    ),
    (
        "bulletproof",
        rotom_core::AbilityBehavior::BulletImmunity,
        Breakable,
    ),
    (
        "soundproof",
        rotom_core::AbilityBehavior::SoundImmunity,
        Breakable,
    ),
    (
        "telepathy",
        rotom_core::AbilityBehavior::AllyDamageImmunity,
        Breakable,
    ),
];

#[test]
fn ability_handler_registration_is_declarative_and_leaves_other_abilities_unsupported() {
    let dex = champions_reg_mc().unwrap();
    let handlers = dex.ability_handlers();
    assert_eq!(handlers.len(), EXPECTED_ABILITY_HANDLERS.len());

    for &(id, expected, suppression) in EXPECTED_ABILITY_HANDLERS {
        let ability = dex.resolve_ability(id).unwrap().0;
        assert!(dex.ability_has_behavior(ability, expected));
        assert!(handlers
            .iter()
            .any(|handler| handler
                == &rotom_core::AbilityHandler::new(ability, expected, suppression)));
    }

    let unsupported = dex.resolve_ability("pressure").unwrap().0;
    assert!(!handlers
        .iter()
        .any(|handler| handler.ability == unsupported));
}
