use rotom_core::{
    AbilityId, ActionTarget, ActiveSlot, BattleError, BattleState, BattleStats, FaintTarget,
    MoveId, MoveSlot, MoveSlotIndex, MoveTarget, PokemonAction, PokemonIndex, PokemonPosition,
    PokemonState, SideId, SideState, SpeciesId, StatStages, StatusState, TeamState, TurnChoice,
};

fn index(value: u8) -> PokemonIndex {
    PokemonIndex::new(value)
}

fn pokemon(species: u32) -> PokemonState {
    PokemonState {
        species: SpeciesId::new(species),
        form: None,
        level: 50,
        stats: BattleStats {
            hp: 100,
            atk: 100,
            def: 100,
            spa: 100,
            spd: 100,
            spe: 100,
        },
        current_hp: 100,
        ability: AbilityId::new(1),
        item: None,
        moves: vec![MoveSlot {
            move_id: MoveId::new(1),
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

fn battle(picked_team_size: usize) -> BattleState {
    let p1 = (1..=4).map(pokemon).collect();
    let p2 = (101..=104).map(pokemon).collect();
    BattleState::new(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        picked_team_size,
    )
    .expect("fixture should be a valid preview state")
}

fn select(state: &mut BattleState, size: u8) {
    let selection = (0..size).map(index).collect::<Vec<_>>();
    state
        .select_team(SideId::P1, selection.clone())
        .expect("P1 selection should be legal");
    state
        .select_team(SideId::P2, selection)
        .expect("P2 selection should be legal");
}

#[test]
fn ordinary_actions_include_targets_switches_and_single_mega_combinations() {
    let mut state = battle(4);
    select(&mut state, 4);
    state
        .side_mut(SideId::P1)
        .pokemon_mut(index(0))
        .expect("left lead should exist")
        .can_mega_evolve = true;
    state
        .side_mut(SideId::P1)
        .pokemon_mut(index(1))
        .expect("right lead should exist")
        .can_mega_evolve = true;

    let legal = state
        .legal_actions(SideId::P1)
        .expect("turn choices should enumerate");

    assert_eq!(legal.slots[0].actions.len(), 8);
    assert_eq!(legal.slots[1].actions.len(), 8);
    assert!(legal.slots[0].actions.contains(&PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Ally(ActiveSlot::Right),
        mega: false,
    }));
    assert!(legal.slots[0].actions.contains(&PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Foe(ActiveSlot::Left),
        mega: true,
    }));
    assert!(legal.slots[0]
        .actions
        .contains(&PokemonAction::Switch { pokemon: index(2) }));
    assert_eq!(legal.choices.len(), 53);
    assert!(legal.choices.iter().all(|choice| {
        choice
            .actions
            .iter()
            .filter(|action| matches!(action, PokemonAction::Move { mega: true, .. }))
            .count()
            <= 1
    }));

    state.side_mut(SideId::P1).mega_evolution_used = true;
    let after_mega = state
        .legal_actions(SideId::P1)
        .expect("ordinary choices should remain available after Mega Evolution");
    assert!(after_mega.slots.iter().all(|slot| slot
        .actions
        .iter()
        .all(|action| !matches!(action, PokemonAction::Move { mega: true, .. }))));
}

#[test]
fn disabled_and_empty_moves_fall_back_to_struggle_while_trapping_blocks_switches() {
    let mut state = battle(4);
    select(&mut state, 4);
    let pokemon = state
        .side_mut(SideId::P1)
        .pokemon_mut(index(0))
        .expect("left lead should exist");
    pokemon.moves[0].pp = 0;
    pokemon.can_mega_evolve = true;
    pokemon.trapped = true;

    let legal = state
        .legal_actions(SideId::P1)
        .expect("turn choices should enumerate");

    assert_eq!(legal.slots[0].actions, vec![PokemonAction::Struggle]);
}

#[test]
fn duplicate_switch_targets_and_double_mega_are_rejected() {
    let mut state = battle(4);
    select(&mut state, 4);
    for active in [index(0), index(1)] {
        state
            .side_mut(SideId::P1)
            .pokemon_mut(active)
            .expect("active should exist")
            .can_mega_evolve = true;
    }

    let duplicate_switch = TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(2) },
            PokemonAction::Switch { pokemon: index(2) },
        ],
    };
    let double_mega = TurnChoice {
        actions: [
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(ActiveSlot::Left),
                mega: true,
            },
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(ActiveSlot::Left),
                mega: true,
            },
        ],
    };

    assert!(matches!(
        state.validate_choice(SideId::P1, &duplicate_switch),
        Err(BattleError::IllegalChoice(_))
    ));
    assert!(matches!(
        state.validate_choice(SideId::P1, &double_mega),
        Err(BattleError::IllegalChoice(_))
    ));
}

#[test]
fn forced_switch_enumeration_assigns_one_reserve_to_either_empty_slot() {
    let mut state = battle(3);
    select(&mut state, 3);
    state
        .process_faints(&[
            FaintTarget::new(SideId::P1, ActiveSlot::Left),
            FaintTarget::new(SideId::P1, ActiveSlot::Right),
        ])
        .expect("both leads should faint");

    let legal = state
        .legal_actions(SideId::P1)
        .expect("replacement choices should enumerate");
    assert_eq!(legal.choices.len(), 2);
    assert!(legal.choices.contains(&TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(2) },
            PokemonAction::Pass,
        ],
    }));
    assert!(legal.choices.contains(&TurnChoice {
        actions: [
            PokemonAction::Pass,
            PokemonAction::Switch { pokemon: index(2) },
        ],
    }));
}

#[test]
fn action_enumeration_rejects_team_preview() {
    let state = battle(4);

    assert!(matches!(
        state.legal_actions(SideId::P1),
        Err(BattleError::IllegalChoice(_))
    ));
}

#[test]
fn automatic_target_moves_do_not_accept_a_selected_target() {
    let mut state = battle(4);
    select(&mut state, 4);
    state
        .side_mut(SideId::P1)
        .pokemon_mut(index(0))
        .expect("left lead should exist")
        .moves[0]
        .target = MoveTarget::Self_;

    let legal = state
        .legal_actions(SideId::P1)
        .expect("turn choices should enumerate");
    assert!(legal.slots[0].actions.contains(&PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Automatic,
        mega: false,
    }));
    assert!(!legal.slots[0].actions.iter().any(|action| matches!(
        action,
        PokemonAction::Move {
            target: ActionTarget::Self_,
            ..
        }
    )));
}
