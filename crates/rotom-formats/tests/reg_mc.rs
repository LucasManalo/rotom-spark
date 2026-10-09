use rotom_core::{AbilityId, Gender, StatAllocation};
use rotom_dex::champions_reg_mc;
use rotom_formats::{
    parse_showdown_team, BattleFormat, ChampionsRegMC, FormatId, GameType, ValidationProblem,
};

const VALID_TEAM: &str = include_str!("../../../data/fixtures/teams/valid-reg-mc.txt");

#[test]
fn parses_and_validates_showdown_export() {
    let dex = champions_reg_mc().expect("generated data should load");
    let team = parse_showdown_team(VALID_TEAM, &dex).expect("fixture should parse");

    ChampionsRegMC
        .validate_team(&team, &dex)
        .expect("fixture should be legal");
    assert_eq!(team.members.len(), 6);
    assert!(team.members[0].form.is_some());
    assert_eq!(team.members[0].stat_points.total(), 66);
    assert_eq!(ChampionsRegMC.id(), FormatId::ChampionsRegMC);
    assert_eq!(ChampionsRegMC.game_type(), GameType::Doubles);
    assert_eq!(ChampionsRegMC.team_size(), 6);
    assert_eq!(ChampionsRegMC.picked_team_size(), 4);
}

#[test]
fn team_definition_round_trips_through_json() {
    let dex = champions_reg_mc().expect("generated data should load");
    let team = parse_showdown_team(VALID_TEAM, &dex).expect("fixture should parse");
    let json = serde_json::to_string(&team).expect("team should serialize");
    let decoded = serde_json::from_str(&json).expect("team should deserialize");

    assert_eq!(team, decoded);
}

#[test]
fn rejects_wrong_team_size() {
    let dex = champions_reg_mc().expect("generated data should load");
    let mut team = parse_showdown_team(VALID_TEAM, &dex).expect("fixture should parse");
    team.members.pop();

    let error = ChampionsRegMC
        .validate_team(&team, &dex)
        .expect_err("five members must be rejected");
    assert!(error.problems.contains(&ValidationProblem::TeamSize {
        expected: 6,
        actual: 5,
    }));
}

#[test]
fn rejects_clause_and_set_legality_violations() {
    let dex = champions_reg_mc().expect("generated data should load");
    let mut team = parse_showdown_team(VALID_TEAM, &dex).expect("fixture should parse");
    team.members[1].species = team.members[0].species;
    team.members[1].item = team.members[0].item;
    team.members[0].ability = AbilityId::new(u32::MAX);
    team.members[2].stat_points = StatAllocation {
        hp: 33,
        atk: 32,
        def: 0,
        spa: 0,
        spd: 0,
        spe: 32,
    };
    team.members[3].ivs.atk = 30;

    let problems = ChampionsRegMC
        .validate_team(&team, &dex)
        .expect_err("mutated team must be rejected")
        .problems;
    assert!(problems
        .iter()
        .any(|problem| matches!(problem, ValidationProblem::DuplicateSpecies { .. })));
    assert!(problems
        .iter()
        .any(|problem| matches!(problem, ValidationProblem::DuplicateItem { .. })));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        ValidationProblem::UnknownReference {
            kind: "ability",
            ..
        }
    )));
    assert!(problems
        .iter()
        .any(|problem| matches!(problem, ValidationProblem::StatPointLimit { .. })));
    assert!(problems
        .iter()
        .any(|problem| matches!(problem, ValidationProblem::StatPointTotal { .. })));
    assert!(problems
        .iter()
        .any(|problem| matches!(problem, ValidationProblem::InvalidIv { .. })));
}

#[test]
fn rejects_illegal_species_and_unlearnable_move() {
    let dex = champions_reg_mc().expect("generated data should load");
    let mut team = parse_showdown_team(VALID_TEAM, &dex).expect("fixture should parse");
    team.members[0].species = dex.resolve_species("mew").expect("Mew exists").0;
    team.members[1].moves[0] = dex
        .resolve_move("thunderclap")
        .expect("Thunderclap exists")
        .0;

    let problems = ChampionsRegMC
        .validate_team(&team, &dex)
        .expect_err("illegal set choices must be rejected")
        .problems;
    assert!(problems
        .iter()
        .any(|problem| matches!(problem, ValidationProblem::IllegalSpecies { .. })));
    assert!(problems
        .iter()
        .any(|problem| matches!(problem, ValidationProblem::MoveNotLearnable { .. })));
}

#[test]
fn rejects_duplicate_moves_like_showdown() {
    let dex = champions_reg_mc().expect("generated data should load");
    let mut team = parse_showdown_team(VALID_TEAM, &dex).expect("fixture should parse");
    let protect = dex.resolve_move("protect").expect("Protect exists").0;
    team.members[0].moves = vec![protect; 4];

    let problems = ChampionsRegMC
        .validate_team(&team, &dex)
        .expect_err("duplicate moves must be rejected")
        .problems;
    assert!(problems.iter().any(|problem| matches!(
        problem,
        ValidationProblem::DuplicateMove { slot: 1, name } if name == "Protect"
    )));
}

#[test]
fn validates_required_mega_item_and_fixed_gender() {
    let dex = champions_reg_mc().expect("generated data should load");
    let mut team = parse_showdown_team(VALID_TEAM, &dex).expect("fixture should parse");
    team.members[0].item = Some(dex.resolve_item("gengarite").expect("Gengarite exists").0);
    team.members[5].species = dex.resolve_species("ditto").expect("Ditto exists").0;
    team.members[5].form = None;
    team.members[5].ability = dex.resolve_ability("imposter").expect("Imposter exists").0;
    team.members[5].moves = vec![dex.resolve_move("transform").expect("Transform exists").0];
    team.members[5].gender = Some(Gender::Male);

    let problems = ChampionsRegMC
        .validate_team(&team, &dex)
        .expect_err("wrong Mega item and genderless Pokémon must be rejected")
        .problems;
    assert!(problems
        .iter()
        .any(|problem| matches!(problem, ValidationProblem::RequiredItem { slot: 1, .. })));
    assert!(problems
        .iter()
        .any(|problem| matches!(problem, ValidationProblem::WrongMegaStone { slot: 1, .. })));
    assert!(problems
        .iter()
        .any(|problem| matches!(problem, ValidationProblem::GenderMismatch { slot: 6 })));
}

#[test]
fn parser_rejects_unknown_names_and_fields() {
    let dex = champions_reg_mc().expect("generated data should load");
    let unknown_species = parse_showdown_team(
        "Definitely Not A Pokemon\nAbility: Blaze\nAdamant Nature\n- Protect",
        &dex,
    )
    .expect_err("unknown species must fail parsing");
    assert_eq!(unknown_species.line, 1);

    let unsupported_field = parse_showdown_team(
        "Charizard\nAbility: Blaze\nTera Type: Fire\nAdamant Nature\n- Protect",
        &dex,
    )
    .expect_err("unsupported fields must not be ignored");
    assert_eq!(unsupported_field.line, 3);
    assert!(unsupported_field.message.contains("unsupported"));
}

#[test]
fn parser_rejects_repeated_singleton_fields_and_stats() {
    let dex = champions_reg_mc().expect("generated data should load");
    let repeated_ability = parse_showdown_team(
        "Charizard\nAbility: Blaze\nAbility: Solar Power\nAdamant Nature\n- Protect",
        &dex,
    )
    .expect_err("repeated ability must be rejected");
    assert_eq!(repeated_ability.line, 3);
    assert!(repeated_ability.message.contains("more than once"));

    let repeated_stat = parse_showdown_team(
        "Charizard\nAbility: Blaze\nEVs: 2 HP / 32 Atk / 32 Atk\nAdamant Nature\n- Protect",
        &dex,
    )
    .expect_err("repeated stat must be rejected");
    assert_eq!(repeated_stat.line, 3);
    assert!(repeated_stat.message.contains("Atk"));
}
