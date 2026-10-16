use rotom_core::{
    step, AbilityId, ActionTarget, ActiveSlot, BattleState, BattleStateBuilder, BattleStats,
    CombatData, CombatMove, ConditionId, ConditionState, EffectRegistry, ExpiredCondition,
    FieldEffectIds, FormId, ItemId, JointAction, MajorStatus, Mechanics, MoveAccuracy,
    MoveCapabilities, MoveCategory, MoveId, MoveOperation, MoveSlot, MoveSlotIndex, MoveTarget,
    PokemonAction, PokemonIndex, PokemonPosition, PokemonRef, PokemonState, PokemonType,
    PokemonTypes, ResolvedAction, RngOperation, RngReason, SideId, SideState, SpeciesId,
    StatStages, StatusState, TeamState, TurnChoice,
};

const MOVE: MoveId = MoveId::new(1);
const RAIN: ConditionId = ConditionId::new(10);
const SUN: ConditionId = ConditionId::new(11);
const SAND: ConditionId = ConditionId::new(12);
const SNOW: ConditionId = ConditionId::new(13);
const ELECTRIC: ConditionId = ConditionId::new(20);
const GRASSY: ConditionId = ConditionId::new(21);
const PSYCHIC: ConditionId = ConditionId::new(22);
const MISTY: ConditionId = ConditionId::new(23);
const REFLECT: ConditionId = ConditionId::new(30);
const LIGHT_SCREEN: ConditionId = ConditionId::new(31);
const AURORA_VEIL: ConditionId = ConditionId::new(32);
const TAILWIND: ConditionId = ConditionId::new(33);
const TRICK_ROOM: ConditionId = ConditionId::new(40);
const GRAVITY: ConditionId = ConditionId::new(41);
const MAGIC_ROOM: ConditionId = ConditionId::new(42);
const WONDER_ROOM: ConditionId = ConditionId::new(43);
const LEVITATE: AbilityId = AbilityId::new(50);
const IRON_BALL: ItemId = ItemId::new(51);
const AIR_BALLOON: ItemId = ItemId::new(52);
const PROTECT: ConditionId = ConditionId::new(53);
static EFFECTS: EffectRegistry = EffectRegistry::new();

#[test]
fn pinned_showdown_fixture_matches_field_effect_constants() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/field-effects.json"
    ))
    .unwrap();
    assert_eq!(
        fixture["showdown_commit"],
        "b79ce06d06b8f0dc8de8bb348bc9cab5d931553f"
    );
    assert_eq!(fixture["weather"]["rain_water"], 1.5);
    assert_eq!(fixture["weather"]["sand_residual_order"], 1);
    assert_eq!(
        fixture["terrain"]["electric_power"],
        serde_json::json!([5325, 4096])
    );
    assert_eq!(
        fixture["side_conditions"]["reflect_doubles"],
        serde_json::json!([2732, 4096])
    );
    assert_eq!(fixture["side_conditions"]["tailwind_duration"], 4);
    assert_eq!(fixture["rooms"]["trickroom"]["residual_sub_order"], 1);
}

#[derive(Clone, Copy)]
struct TestData {
    move_type: PokemonType,
    category: MoveCategory,
    priority: i8,
    accuracy: MoveAccuracy,
    target: MoveTarget,
}

impl TestData {
    const fn new(move_type: PokemonType, category: MoveCategory) -> Self {
        Self {
            move_type,
            category,
            priority: 0,
            accuracy: MoveAccuracy::Always,
            target: MoveTarget::Normal,
        }
    }
}

impl CombatData for TestData {
    fn move_data(&self, move_id: MoveId) -> Option<CombatMove> {
        (move_id == MOVE).then_some(CombatMove {
            move_type: self.move_type,
            category: self.category,
            base_power: 60,
            accuracy: self.accuracy,
            crit_ratio: 0,
            target: self.target,
            operation: MoveOperation::StandardDamage,
            required_capabilities: if matches!(self.target, MoveTarget::Normal) {
                MoveCapabilities::PR_13_1
            } else {
                MoveCapabilities::PR_13_2
            },
        })
    }

    fn pokemon_types(&self, species: SpeciesId, _: Option<FormId>) -> Option<PokemonTypes> {
        let primary = match species.get() {
            3 => PokemonType::Flying,
            101 => PokemonType::Rock,
            102 => PokemonType::Ice,
            103 => PokemonType::Steel,
            _ => PokemonType::Normal,
        };
        Some(PokemonTypes {
            primary,
            secondary: None,
        })
    }

    fn type_modifier(&self, _: PokemonType, _: PokemonType) -> Option<i8> {
        Some(0)
    }

    fn field_effect_ids(&self) -> FieldEffectIds {
        ids()
    }

    fn move_is_protectable(&self, move_id: MoveId) -> bool {
        move_id == MOVE
    }
}

fn ids() -> FieldEffectIds {
    FieldEffectIds {
        rain: Some(RAIN),
        sun: Some(SUN),
        sandstorm: Some(SAND),
        snow: Some(SNOW),
        electric_terrain: Some(ELECTRIC),
        grassy_terrain: Some(GRASSY),
        psychic_terrain: Some(PSYCHIC),
        misty_terrain: Some(MISTY),
        reflect: Some(REFLECT),
        light_screen: Some(LIGHT_SCREEN),
        aurora_veil: Some(AURORA_VEIL),
        tailwind: Some(TAILWIND),
        trick_room: Some(TRICK_ROOM),
        gravity: Some(GRAVITY),
        magic_room: Some(MAGIC_ROOM),
        wonder_room: Some(WONDER_ROOM),
        levitate: Some(LEVITATE),
        iron_ball: Some(IRON_BALL),
        air_balloon: Some(AIR_BALLOON),
        protect: Some(PROTECT),
        thunder: Some(MOVE),
        blizzard: Some(MOVE),
        ..FieldEffectIds::default()
    }
}

fn index(value: u8) -> PokemonIndex {
    PokemonIndex::new(value)
}

fn pokemon(species: u32, speed: u16) -> PokemonState {
    PokemonState {
        species: SpeciesId::new(species),
        form: None,
        level: 50,
        stats: BattleStats {
            hp: 400,
            atk: 120,
            def: 100,
            spa: 120,
            spd: 100,
            spe: speed,
        },
        current_hp: 400,
        ability: AbilityId::new(1),
        item: None,
        moves: vec![MoveSlot {
            move_id: MOVE,
            target: MoveTarget::Normal,
            priority: 0,
            pp: 16,
            max_pp: 16,
            disabled: false,
        }],
        status: None,
        status_state: StatusState::default(),
        stat_stages: StatStages::default(),
        volatiles: Vec::new(),
        can_mega_evolve: false,
        trapped: false,
        position: PokemonPosition::Team,
    }
}

fn battle() -> BattleState {
    let p1 = vec![pokemon(1, 200), pokemon(2, 180)];
    let p2 = vec![pokemon(101, 100), pokemon(102, 80)];
    let initial = BattleState::new_with_seed(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        2,
        0x12_affe,
    )
    .unwrap();
    let mut builder = BattleStateBuilder::from_state(initial).unwrap();
    builder
        .select_teams(vec![index(0), index(1)], vec![index(0), index(1)])
        .unwrap();
    builder.build().unwrap()
}

fn move_at(target: ActiveSlot) -> PokemonAction {
    PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Foe(target),
        mega: false,
    }
}

fn turn() -> JointAction {
    JointAction::Turn {
        p1: TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        },
        p2: TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        },
    }
}

fn spread_turn() -> JointAction {
    let action = PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Automatic,
        mega: false,
    };
    JointAction::Turn {
        p1: TurnChoice {
            actions: [action, action],
        },
        p2: TurnChoice {
            actions: [action, action],
        },
    }
}

fn mechanics(data: &TestData) -> Mechanics<'_, TestData> {
    Mechanics::new(data, &EFFECTS)
}

fn p1_left_damage(result: &rotom_core::StepResult) -> u16 {
    result
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .actions
        .iter()
        .find_map(|action| match action {
            ResolvedAction::Damaged {
                side: SideId::P1,
                slot: ActiveSlot::Left,
                damage,
                ..
            } => Some(damage.final_damage),
            _ => None,
        })
        .unwrap()
}

fn with_weather(state: &BattleState, condition: ConditionId) -> BattleState {
    let mut builder = BattleStateBuilder::from_state(state.clone()).unwrap();
    builder.edit_field(|field| field.weather = Some(ConditionState::new(condition, Some(5))));
    builder.build().unwrap()
}

fn with_terrain(state: &BattleState, condition: ConditionId) -> BattleState {
    let mut builder = BattleStateBuilder::from_state(state.clone()).unwrap();
    builder.edit_field(|field| field.terrain = Some(ConditionState::new(condition, Some(5))));
    builder.build().unwrap()
}

#[test]
fn rain_and_sun_modify_fire_and_water_damage() {
    for move_type in [PokemonType::Water, PokemonType::Fire] {
        let data = TestData::new(move_type, MoveCategory::Special);
        let clear = step(&battle(), &turn(), &mechanics(&data)).unwrap();
        let rain = step(&with_weather(&battle(), RAIN), &turn(), &mechanics(&data)).unwrap();
        let sun = step(&with_weather(&battle(), SUN), &turn(), &mechanics(&data)).unwrap();
        let clear_damage = p1_left_damage(&clear);
        if move_type == PokemonType::Water {
            assert!(p1_left_damage(&rain) > clear_damage);
            assert!(p1_left_damage(&sun) < clear_damage);
        } else {
            assert!(p1_left_damage(&rain) < clear_damage);
            assert!(p1_left_damage(&sun) > clear_damage);
        }
    }
}

#[test]
fn terrain_power_and_grounded_status_rules_share_one_query() {
    for (move_type, terrain) in [
        (PokemonType::Electric, ELECTRIC),
        (PokemonType::Grass, GRASSY),
        (PokemonType::Psychic, PSYCHIC),
    ] {
        let data = TestData::new(move_type, MoveCategory::Special);
        let clear = step(&battle(), &turn(), &mechanics(&data)).unwrap();
        let boosted = step(
            &with_terrain(&battle(), terrain),
            &turn(),
            &mechanics(&data),
        )
        .unwrap();
        assert!(p1_left_damage(&boosted) > p1_left_damage(&clear));
    }
    let dragon = TestData::new(PokemonType::Dragon, MoveCategory::Special);
    let clear = step(&battle(), &turn(), &mechanics(&dragon)).unwrap();
    let misty = step(
        &with_terrain(&battle(), MISTY),
        &turn(),
        &mechanics(&dragon),
    )
    .unwrap();
    assert!(p1_left_damage(&misty) < p1_left_damage(&clear));

    let electric = TestData::new(PokemonType::Electric, MoveCategory::Special);
    let terrain_state = with_terrain(&battle(), ELECTRIC);
    assert!(!mechanics(&electric)
        .terrain_allows_status(
            &terrain_state,
            PokemonRef::new(SideId::P1, index(0)),
            MajorStatus::Sleep,
        )
        .unwrap());
    assert!(mechanics(&electric)
        .terrain_allows_status(
            &terrain_state,
            PokemonRef::new(SideId::P1, index(0)),
            MajorStatus::Burn,
        )
        .unwrap());

    let misty_state = with_terrain(&battle(), MISTY);
    assert!(!mechanics(&electric)
        .terrain_allows_status(
            &misty_state,
            PokemonRef::new(SideId::P1, index(0)),
            MajorStatus::Burn,
        )
        .unwrap());
}

#[test]
fn grounded_query_handles_flying_levitate_items_gravity_and_magic_room() {
    let data = TestData::new(PokemonType::Normal, MoveCategory::Physical);
    let mut builder = BattleStateBuilder::from_state(battle()).unwrap();
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(0)), |pokemon| {
            pokemon.species = SpeciesId::new(3);
            pokemon.item = Some(IRON_BALL);
        })
        .unwrap();
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(1)), |pokemon| {
            pokemon.ability = LEVITATE;
            pokemon.item = Some(AIR_BALLOON);
        })
        .unwrap();
    let state = builder.build().unwrap();
    assert!(mechanics(&data)
        .is_grounded(&state, PokemonRef::new(SideId::P1, index(0)))
        .unwrap());
    assert!(!mechanics(&data)
        .is_grounded(&state, PokemonRef::new(SideId::P1, index(1)))
        .unwrap());

    let mut builder = BattleStateBuilder::from_state(state).unwrap();
    builder.edit_field(|field| {
        field
            .conditions
            .push(ConditionState::new(MAGIC_ROOM, Some(5)));
    });
    let magic_room = builder.build().unwrap();
    assert!(!mechanics(&data)
        .is_grounded(&magic_room, PokemonRef::new(SideId::P1, index(0)))
        .unwrap());

    let mut builder = BattleStateBuilder::from_state(magic_room).unwrap();
    builder.edit_field(|field| {
        field.conditions.push(ConditionState::new(GRAVITY, Some(5)));
    });
    let gravity = builder.build().unwrap();
    assert!(mechanics(&data)
        .is_grounded(&gravity, PokemonRef::new(SideId::P1, index(1)))
        .unwrap());
}

#[test]
fn tailwind_and_trick_room_change_action_order_deterministically() {
    let data = TestData::new(PokemonType::Normal, MoveCategory::Physical);
    let base = battle();
    let mut builder = BattleStateBuilder::from_state(base.clone()).unwrap();
    builder.edit_side(SideId::P2, |side| {
        side.side_conditions
            .push(ConditionState::new(TAILWIND, Some(4)));
    });
    let tailwind = builder.build().unwrap();
    let queue = tailwind
        .inspect_turn_queue(
            match turn() {
                JointAction::Turn { p1, .. } => p1,
                _ => unreachable!(),
            },
            match turn() {
                JointAction::Turn { p2, .. } => p2,
                _ => unreachable!(),
            },
            &mechanics(&data),
        )
        .unwrap();
    assert_eq!(queue.actions[0].side, SideId::P2);

    let mut builder = BattleStateBuilder::from_state(base).unwrap();
    builder.edit_field(|field| {
        field
            .conditions
            .push(ConditionState::new(TRICK_ROOM, Some(5)));
    });
    let trick_room = builder.build().unwrap();
    let JointAction::Turn { p1, p2 } = turn() else {
        unreachable!();
    };
    let queue = trick_room
        .inspect_turn_queue(p1, p2, &mechanics(&data))
        .unwrap();
    assert_eq!(queue.actions[0].pokemon, index(1));
    assert_eq!(queue.actions[0].side, SideId::P2);
}

#[test]
fn screens_sand_snow_and_wonder_room_modify_the_damage_pipeline() {
    let physical = TestData::new(PokemonType::Normal, MoveCategory::Physical);
    let special = TestData::new(PokemonType::Normal, MoveCategory::Special);
    let clear_physical = step(&battle(), &turn(), &mechanics(&physical)).unwrap();
    let clear_special = step(&battle(), &turn(), &mechanics(&special)).unwrap();

    let mut builder = BattleStateBuilder::from_state(battle()).unwrap();
    builder.edit_side(SideId::P2, |side| {
        side.side_conditions
            .push(ConditionState::new(REFLECT, Some(5)));
    });
    let reflected = step(&builder.build().unwrap(), &turn(), &mechanics(&physical)).unwrap();
    assert!(p1_left_damage(&reflected) < p1_left_damage(&clear_physical));

    let mut builder = BattleStateBuilder::from_state(battle()).unwrap();
    builder.edit_side(SideId::P2, |side| {
        side.side_conditions
            .push(ConditionState::new(LIGHT_SCREEN, Some(5)));
    });
    let screened = step(&builder.build().unwrap(), &turn(), &mechanics(&special)).unwrap();
    assert!(p1_left_damage(&screened) < p1_left_damage(&clear_special));

    let mut builder = BattleStateBuilder::from_state(battle()).unwrap();
    builder.edit_side(SideId::P2, |side| {
        side.side_conditions
            .push(ConditionState::new(AURORA_VEIL, Some(5)));
    });
    let veiled = step(&builder.build().unwrap(), &turn(), &mechanics(&physical)).unwrap();
    assert!(p1_left_damage(&veiled) < p1_left_damage(&clear_physical));

    let sand = step(
        &with_weather(&battle(), SAND),
        &turn(),
        &mechanics(&special),
    )
    .unwrap();
    assert!(p1_left_damage(&sand) < p1_left_damage(&clear_special));

    let mut snow_battle = battle();
    let mut builder = BattleStateBuilder::from_state(snow_battle).unwrap();
    builder.edit_side(SideId::P2, |side| {
        side.team.members[0].species = SpeciesId::new(102);
    });
    builder.edit_field(|field| field.weather = Some(ConditionState::new(SNOW, Some(5))));
    snow_battle = builder.build().unwrap();
    let snow = step(&snow_battle, &turn(), &mechanics(&physical)).unwrap();
    assert!(p1_left_damage(&snow) < p1_left_damage(&clear_physical));

    let mut builder = BattleStateBuilder::from_state(battle()).unwrap();
    builder.edit_side(SideId::P2, |side| {
        side.team.members[0].stats.def = 200;
        side.team.members[0].stats.spd = 50;
    });
    let unequal = builder.build().unwrap();
    let normal = step(&unequal, &turn(), &mechanics(&physical)).unwrap();
    let mut builder = BattleStateBuilder::from_state(unequal).unwrap();
    builder.edit_field(|field| {
        field
            .conditions
            .push(ConditionState::new(WONDER_ROOM, Some(5)));
    });
    let wonder = step(&builder.build().unwrap(), &turn(), &mechanics(&physical)).unwrap();
    assert!(p1_left_damage(&wonder) > p1_left_damage(&normal));
}

#[test]
fn weather_and_terrain_resolve_before_ordered_duration_expiration() {
    let data = TestData::new(PokemonType::Normal, MoveCategory::Physical);
    let mut builder = BattleStateBuilder::from_state(battle()).unwrap();
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(0)), |pokemon| {
            pokemon.current_hp = 300;
        })
        .unwrap();
    builder.edit_field(|field| {
        field.weather = Some(ConditionState::new(SAND, Some(1)));
        field.terrain = Some(ConditionState::new(GRASSY, Some(1)));
        field.conditions.extend([
            ConditionState::new(MAGIC_ROOM, Some(1)),
            ConditionState::new(WONDER_ROOM, Some(1)),
            ConditionState::new(GRAVITY, Some(1)),
            ConditionState::new(TRICK_ROOM, Some(1)),
        ]);
    });
    builder.edit_side(SideId::P1, |side| {
        side.side_conditions
            .push(ConditionState::new(TAILWIND, Some(1)));
    });
    let result = step(&builder.build().unwrap(), &turn(), &mechanics(&data)).unwrap();
    let diagnostics = result.diagnostics.turn.unwrap();

    assert!(diagnostics
        .field_hp_changes
        .iter()
        .any(|entry| entry.condition == SAND && entry.amount < 0));
    assert!(diagnostics
        .field_hp_changes
        .iter()
        .any(|entry| entry.condition == GRASSY && entry.amount > 0));
    assert_eq!(
        diagnostics.expired_conditions,
        vec![
            ExpiredCondition::Weather { condition: SAND },
            ExpiredCondition::Side {
                side: SideId::P1,
                condition: TAILWIND,
            },
            ExpiredCondition::Field {
                condition: TRICK_ROOM,
            },
            ExpiredCondition::Field { condition: GRAVITY },
            ExpiredCondition::Field {
                condition: WONDER_ROOM,
            },
            ExpiredCondition::Field {
                condition: MAGIC_ROOM,
            },
            ExpiredCondition::Terrain { condition: GRASSY },
        ]
    );
    assert_eq!(result.state.field().weather, None);
    assert_eq!(result.state.field().terrain, None);
    assert_eq!(result.state.field().conditions, []);
    assert_eq!(result.state.side(SideId::P1).side_conditions, []);
}

#[test]
fn psychic_terrain_blocks_positive_priority_against_grounded_foes() {
    let data = TestData {
        move_type: PokemonType::Normal,
        category: MoveCategory::Physical,
        priority: 1,
        accuracy: MoveAccuracy::Always,
        target: MoveTarget::Normal,
    };
    let mut builder = BattleStateBuilder::from_state(with_terrain(&battle(), PSYCHIC)).unwrap();
    for side in [SideId::P1, SideId::P2] {
        builder.edit_side(side, |state| {
            for pokemon in &mut state.team.members {
                pokemon.moves[0].priority = data.priority;
                pokemon.moves[0].target = data.target;
            }
        });
    }
    let result = step(&builder.build().unwrap(), &turn(), &mechanics(&data)).unwrap();
    assert!(result
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .actions
        .iter()
        .all(|action| matches!(action, ResolvedAction::Skipped { .. })));
    assert!(result
        .diagnostics
        .turn
        .unwrap()
        .target_outcomes
        .iter()
        .all(|outcome| outcome.result == rotom_core::TargetResult::PriorityBlocked));
    assert_eq!(
        result
            .state
            .side(SideId::P1)
            .pokemon(index(0))
            .unwrap()
            .moves[0]
            .pp,
        15
    );
}

#[test]
fn psychic_terrain_is_resolved_per_target_before_protect_and_move_rng() {
    let data = TestData {
        move_type: PokemonType::Normal,
        category: MoveCategory::Physical,
        priority: 1,
        accuracy: MoveAccuracy::Always,
        target: MoveTarget::AllAdjacentFoes,
    };
    let mut builder = BattleStateBuilder::from_state(with_terrain(&battle(), PSYCHIC)).unwrap();
    for side in [SideId::P1, SideId::P2] {
        builder.edit_side(side, |state| {
            for pokemon in &mut state.team.members {
                pokemon.moves[0].priority = data.priority;
                pokemon.moves[0].target = data.target;
            }
        });
    }
    builder
        .edit_pokemon(PokemonRef::new(SideId::P2, index(1)), |pokemon| {
            pokemon.species = SpeciesId::new(3);
            pokemon
                .volatiles
                .push(ConditionState::new(PROTECT, Some(1)));
        })
        .unwrap();
    builder.enable_rng_trace();
    let state = builder.build().unwrap();
    let replay = state.clone();
    let result = step(&state, &spread_turn(), &mechanics(&data)).unwrap();
    let replayed = step(&replay, &spread_turn(), &mechanics(&data)).unwrap();
    assert_eq!(result, replayed);

    let outcomes = result
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .filter(|outcome| {
            outcome.source == PokemonRef::new(SideId::P1, index(0))
                && outcome.target.side == SideId::P2
        })
        .collect::<Vec<_>>();
    assert_eq!(outcomes.len(), 2);
    assert_eq!(
        outcomes[0].result,
        rotom_core::TargetResult::PriorityBlocked
    );
    assert_eq!(outcomes[1].result, rotom_core::TargetResult::Protected);
    assert!(outcomes.iter().all(|outcome| outcome.applied_hp_loss == 0));
    assert_eq!(result.state.rng().draws(), state.rng().draws());
}

#[test]
fn psychic_terrain_leaves_ungrounded_and_clear_field_targets_unaffected() {
    let mut data = TestData {
        move_type: PokemonType::Normal,
        category: MoveCategory::Physical,
        priority: 1,
        accuracy: MoveAccuracy::Always,
        target: MoveTarget::Normal,
    };
    let configure = |terrain: bool, target: MoveTarget| {
        let base = if terrain {
            with_terrain(&battle(), PSYCHIC)
        } else {
            battle()
        };
        let mut builder = BattleStateBuilder::from_state(base).unwrap();
        for side in [SideId::P1, SideId::P2] {
            builder.edit_side(side, |state| {
                for pokemon in &mut state.team.members {
                    pokemon.moves[0].priority = 1;
                    pokemon.moves[0].target = target;
                }
            });
        }
        builder
            .edit_pokemon(PokemonRef::new(SideId::P2, index(0)), |pokemon| {
                pokemon.species = SpeciesId::new(3);
            })
            .unwrap();
        builder.build().unwrap()
    };

    let airborne = step(
        &configure(true, MoveTarget::Normal),
        &turn(),
        &mechanics(&data),
    )
    .unwrap();
    assert!(airborne
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .any(|outcome| {
            outcome.target == PokemonRef::new(SideId::P2, index(0))
                && outcome.result == rotom_core::TargetResult::Hit
                && outcome.applied_hp_loss > 0
        }));

    data.target = MoveTarget::AllAdjacentFoes;
    let clear = step(
        &configure(false, MoveTarget::AllAdjacentFoes),
        &spread_turn(),
        &mechanics(&data),
    )
    .unwrap();
    assert!(clear
        .diagnostics
        .turn
        .as_ref()
        .unwrap()
        .target_outcomes
        .iter()
        .all(|outcome| outcome.result != rotom_core::TargetResult::PriorityBlocked));
    assert!(clear
        .diagnostics
        .turn
        .unwrap()
        .target_outcomes
        .iter()
        .all(|outcome| outcome.applied_hp_loss > 0));
}

#[test]
fn weather_accuracy_overrides_use_the_authoritative_rng_only_when_needed() {
    let data = TestData {
        move_type: PokemonType::Electric,
        category: MoveCategory::Special,
        priority: 0,
        accuracy: MoveAccuracy::Percent { value: 70 },
        target: MoveTarget::Normal,
    };
    let mut builder = BattleStateBuilder::from_state(with_weather(&battle(), RAIN)).unwrap();
    builder.enable_rng_trace();
    let rain = step(&builder.build().unwrap(), &turn(), &mechanics(&data)).unwrap();
    assert!(rain
        .state
        .rng()
        .trace()
        .unwrap()
        .iter()
        .all(|entry| entry.reason != RngReason::Accuracy));

    let mut builder = BattleStateBuilder::from_state(with_weather(&battle(), SUN)).unwrap();
    builder.enable_rng_trace();
    let sun = step(&builder.build().unwrap(), &turn(), &mechanics(&data)).unwrap();
    assert!(sun.state.rng().trace().unwrap().iter().any(|entry| {
        entry.reason == RngReason::Accuracy
            && entry.operation
                == RngOperation::Chance {
                    numerator: 50,
                    denominator: 100,
                }
    }));
}

#[test]
fn gravity_modifies_accuracy_before_the_single_rng_check() {
    let data = TestData {
        move_type: PokemonType::Normal,
        category: MoveCategory::Physical,
        priority: 0,
        accuracy: MoveAccuracy::Percent { value: 60 },
        target: MoveTarget::Normal,
    };
    let mut builder = BattleStateBuilder::from_state(battle()).unwrap();
    builder.edit_field(|field| {
        field.conditions.push(ConditionState::new(GRAVITY, Some(5)));
    });
    builder.enable_rng_trace();
    let result = step(&builder.build().unwrap(), &turn(), &mechanics(&data)).unwrap();
    assert!(result.state.rng().trace().unwrap().iter().any(|entry| {
        entry.reason == RngReason::Accuracy
            && entry.operation
                == RngOperation::Chance {
                    numerator: 100,
                    denominator: 100,
                }
    }));
}
