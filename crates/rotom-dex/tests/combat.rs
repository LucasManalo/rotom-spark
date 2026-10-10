use rotom_core::{
    calculate_move_damage, ActionTarget, BattlePhase, BattleState, FormId, Gender, MoveSlotIndex,
    PokemonAction, PokemonIndex, PokemonSet, SideId, SideState, StatAllocation, TeamState,
    TurnChoice,
};
use rotom_dex::champions_reg_mc;

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
    for (source_set, target_set, expected_attack, expected_hp, expected_damage) in
        showdown_damage_cases(&dex)
    {
        let source = dex.pokemon_state(&source_set).expect("source state");
        let target = dex.pokemon_state(&target_set).expect("target state");
        let move_id = source_set.moves[0];
        let attack = if dex.move_data(move_id).expect("move").category == "Physical" {
            source.stats.atk
        } else {
            source.stats.spa
        };
        assert_eq!(attack, expected_attack);
        assert_eq!(target.stats.hp, expected_hp);
        assert_eq!(
            calculate_move_damage(&dex, &source, &target, move_id, false, 100)
                .expect("supported damage")
                .final_damage,
            expected_damage
        );
    }
}

fn showdown_damage_cases(dex: &rotom_dex::DexData) -> Vec<(PokemonSet, PokemonSet, u16, u16, u16)> {
    vec![
        (
            set(
                dex,
                "goodra",
                "gooey",
                "modest",
                "dragonpulse",
                StatAllocation {
                    spa: 32,
                    ..StatAllocation::ZERO
                },
            ),
            set(
                dex,
                "arcanine",
                "intimidate",
                "serious",
                "splash",
                StatAllocation {
                    hp: 32,
                    ..StatAllocation::ZERO
                },
            ),
            178,
            197,
            102,
        ),
        (
            set(
                dex,
                "glimmora",
                "corrosion",
                "modest",
                "powergem",
                StatAllocation {
                    spa: 32,
                    ..StatAllocation::ZERO
                },
            ),
            set(
                dex,
                "charizard",
                "blaze",
                "serious",
                "splash",
                StatAllocation {
                    hp: 32,
                    ..StatAllocation::ZERO
                },
            ),
            200,
            185,
            412,
        ),
        (
            set(
                dex,
                "mudsdale",
                "owntempo",
                "adamant",
                "highhorsepower",
                StatAllocation {
                    atk: 32,
                    ..StatAllocation::ZERO
                },
            ),
            set(
                dex,
                "arcanine",
                "flashfire",
                "serious",
                "splash",
                StatAllocation {
                    hp: 32,
                    ..StatAllocation::ZERO
                },
            ),
            194,
            197,
            248,
        ),
    ]
}

#[test]
fn queued_turn_consumes_pp_and_updates_hp() {
    let dex = champions_reg_mc().expect("generated Dex");
    let goodra = set(
        &dex,
        "goodra",
        "gooey",
        "modest",
        "dragonpulse",
        StatAllocation::ZERO,
    );
    let make_side = || {
        let pokemon = dex.pokemon_state(&goodra).expect("state");
        SideState::new(TeamState::new(vec![
            pokemon.clone(),
            pokemon.clone(),
            pokemon.clone(),
            pokemon,
        ]))
    };
    let mut battle = BattleState::new_with_seed(make_side(), make_side(), 4, 19).expect("battle");
    let selection = vec![
        PokemonIndex::new(0),
        PokemonIndex::new(1),
        PokemonIndex::new(2),
        PokemonIndex::new(3),
    ];
    battle
        .select_team(SideId::P1, selection.clone())
        .expect("p1 preview");
    battle
        .select_team(SideId::P2, selection)
        .expect("p2 preview");
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
    battle.queue_turn(p1, p2).expect("queue");
    let outcome = battle.resolve_turn(&dex).expect("resolution");

    assert_eq!(outcome.actions.len(), 4);
    assert_eq!(battle.phase, BattlePhase::TurnSelection);
    assert_eq!(battle.turn, 2);
    assert_eq!(battle.queue.actions.len(), 0);
    for side in [SideId::P1, SideId::P2] {
        for index in [PokemonIndex::new(0), PokemonIndex::new(1)] {
            let pokemon = battle.side(side).pokemon(index).expect("active");
            assert_eq!(pokemon.moves[0].pp, pokemon.moves[0].max_pp - 1);
            assert!(pokemon.current_hp < pokemon.stats.hp);
            assert!(pokemon.current_hp > 0);
        }
    }
}

#[test]
fn unsupported_move_resolution_is_atomic() {
    let dex = champions_reg_mc().expect("generated Dex");
    let unsupported = set(
        &dex,
        "goodra",
        "gooey",
        "serious",
        "flamethrower",
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
    let mut battle = BattleState::new_with_seed(make_side(), make_side(), 4, 7).expect("battle");
    let selection = vec![
        PokemonIndex::new(0),
        PokemonIndex::new(1),
        PokemonIndex::new(2),
        PokemonIndex::new(3),
    ];
    battle
        .select_team(SideId::P1, selection.clone())
        .expect("preview");
    battle.select_team(SideId::P2, selection).expect("preview");
    let choice = TurnChoice {
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
    battle.queue_turn(choice, choice).expect("queue");
    let before = battle.clone();

    assert!(matches!(
        battle.resolve_turn(&dex),
        Err(rotom_core::BattleError::UnsupportedMechanic(_))
    ));
    assert_eq!(battle, before);
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
    let mut battle = BattleState::new_with_seed(make_side(&attacker), make_side(&defender), 4, 31)
        .expect("battle");
    let selection = vec![
        PokemonIndex::new(0),
        PokemonIndex::new(1),
        PokemonIndex::new(2),
        PokemonIndex::new(3),
    ];
    battle
        .select_team(SideId::P1, selection.clone())
        .expect("preview");
    battle.select_team(SideId::P2, selection).expect("preview");
    let attack = |target| PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Foe(target),
        mega: false,
    };
    battle
        .queue_turn(
            TurnChoice {
                actions: [
                    attack(rotom_core::ActiveSlot::Left),
                    attack(rotom_core::ActiveSlot::Right),
                ],
            },
            TurnChoice {
                actions: [
                    attack(rotom_core::ActiveSlot::Left),
                    attack(rotom_core::ActiveSlot::Right),
                ],
            },
        )
        .expect("queue");
    battle.resolve_turn(&dex).expect("resolution");

    assert_eq!(battle.phase, BattlePhase::ForcedSwitch);
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
