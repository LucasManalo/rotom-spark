use crate::{Accuracy, DexData, MoveBehaviorKind, MoveData, SpeciesData, Stats};
use rotom_core::{
    AbilityBehavior, AbilityHandler, AbilityId, AbilitySuppression, BattleError, BattleStat,
    BattleStats, CombatData, CombatMove, ConditionId, EffectFraction, FieldEffectIds, HitSequence,
    MajorStatus, MoveAccuracy, MoveCapabilities, MoveCapability, MoveCategory, MoveId,
    MoveIntrinsic, MoveOperation, MoveSlot, MoveTarget, PokemonPosition, PokemonSet, PokemonState,
    PokemonType, PokemonTypes, PrimaryMoveEffect, PriorMoveEffect, SecondaryMoveEffect,
    SelfMoveEffect, StatAllocation, StatStages, StatusState, WeatherEnvironment,
};

impl CombatData for DexData {
    fn move_data(&self, move_id: MoveId) -> Option<CombatMove> {
        let data = self.move_data(move_id)?;
        let (operation, required_capabilities) = move_execution_requirements(data);
        Some(CombatMove {
            move_type: parse_type(&data.move_type)?,
            category: parse_category(&data.category)?,
            base_power: data.base_power,
            accuracy: match data.accuracy {
                Accuracy::Always => MoveAccuracy::Always,
                Accuracy::Percent { value } => MoveAccuracy::Percent { value },
            },
            crit_ratio: data.crit_ratio,
            target: data.target,
            operation,
            required_capabilities,
            hit_sequence: data.multi_hit.map_or(HitSequence::Single, |hits| {
                if hits.min == hits.max {
                    HitSequence::Fixed { hits: hits.min }
                } else {
                    HitSequence::Range {
                        min: hits.min,
                        max: hits.max,
                    }
                }
            }),
            multi_accuracy: data.multi_accuracy,
            fixed_damage: match data.damage {
                Some(crate::Damage::Fixed { value }) => Some(value),
                Some(crate::Damage::Level) | None => None,
            },
            drain: data.drain.map(|value| EffectFraction {
                numerator: value.numerator,
                denominator: value.denominator,
            }),
            recoil: data.recoil.map(|value| EffectFraction {
                numerator: value.numerator,
                denominator: value.denominator,
            }),
            self_damage: data.self_damage.map(|value| EffectFraction {
                numerator: value.numerator,
                denominator: value.denominator,
            }),
            prior_effect: if data.id == "facade" {
                PriorMoveEffect::Facade
            } else {
                PriorMoveEffect::None
            },
            offensive_stat: data.override_offensive_stat.as_deref().and_then(parse_stat),
            defensive_stat: data.override_defensive_stat.as_deref().and_then(parse_stat),
            ignore_negative_offensive: data.ignore_negative_offensive,
            ignore_positive_defensive: data.ignore_positive_defensive,
            interaction_flags: rotom_core::MoveInteractionFlags::default()
                .with_bullet(data.flags.iter().any(|flag| flag == "bullet"))
                .with_sound(data.flags.iter().any(|flag| flag == "sound"))
                .with_contact(data.flags.iter().any(|flag| flag == "contact"))
                .with_bite(data.flags.iter().any(|flag| flag == "bite"))
                .with_punch(data.flags.iter().any(|flag| flag == "punch"))
                .with_slicing(data.flags.iter().any(|flag| flag == "slicing"))
                .with_pulse(data.flags.iter().any(|flag| flag == "pulse"))
                .with_powder(data.flags.iter().any(|flag| flag == "powder"))
                .with_recoil(
                    data.recoil.is_some()
                        || data
                            .behavior
                            .special_properties
                            .iter()
                            .any(|property| property == "hasCrashDamage"),
                ),
        })
    }

    fn pokemon_types(
        &self,
        species: rotom_core::SpeciesId,
        form: Option<rotom_core::FormId>,
    ) -> Option<PokemonTypes> {
        let species = selected_species(self, species, form)?;
        let mut types = species.types.iter().filter_map(|value| parse_type(value));
        Some(PokemonTypes {
            primary: types.next()?,
            secondary: types.next(),
        })
    }

    fn type_modifier(&self, attacking: PokemonType, defending: PokemonType) -> Option<i8> {
        let defending = self.resolve_type(type_id(defending))?;
        let value = defending
            .damage_taken
            .iter()
            .find(|entry| entry.key == type_id(attacking))
            .map_or(0, |entry| entry.value);
        match value {
            0 => Some(0),
            1 => Some(1),
            2 => Some(-1),
            3..=u8::MAX => None,
        }
    }

    fn move_intrinsic(&self, move_id: MoveId) -> MoveIntrinsic {
        match self.move_data(move_id).map(|data| data.id.as_str()) {
            Some("protect") => MoveIntrinsic::Protect,
            Some("followme") => MoveIntrinsic::FollowMe,
            Some("ragepowder") => MoveIntrinsic::RagePowder,
            _ => MoveIntrinsic::Ordinary,
        }
    }

    fn primary_move_effects(&self, move_id: MoveId) -> Vec<PrimaryMoveEffect> {
        let Some(data) = self.move_data(move_id) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        if let Some(status) = data.status.as_deref().and_then(parse_status) {
            effects.push(PrimaryMoveEffect::InflictStatus { status });
        }
        for boost in &data.boosts {
            if let Some(stat) = parse_stat(&boost.key) {
                effects.push(PrimaryMoveEffect::ChangeStatStage {
                    stat,
                    delta: boost.value,
                });
            }
        }
        if let Some(heal) = data.heal {
            effects.push(PrimaryMoveEffect::Heal {
                numerator: heal.numerator,
                denominator: heal.denominator,
            });
        }
        if data.volatile_status.as_deref() == Some("magnetrise") {
            effects.push(PrimaryMoveEffect::AddVolatile {
                condition: ConditionId::new(move_id.get()),
                duration: Some(5),
            });
        }
        let condition = |name: &str| {
            self.resolve_move(&name.to_ascii_lowercase())
                .map(|(id, _)| ConditionId::new(id.get()))
        };
        if let Some(name) = data.weather.as_deref() {
            if let Some(condition) = condition(name) {
                effects.push(PrimaryMoveEffect::SetWeather {
                    condition,
                    duration: 5,
                });
            }
        }
        if let Some(name) = data.terrain.as_deref() {
            if let Some(condition) = condition(name) {
                effects.push(PrimaryMoveEffect::SetTerrain {
                    condition,
                    duration: 5,
                });
            }
        }
        if let Some(name) = data.pseudo_weather.as_deref() {
            if matches!(name, "trickroom" | "gravity" | "magicroom" | "wonderroom") {
                if let Some(condition) = condition(name) {
                    effects.push(if name == "gravity" {
                        PrimaryMoveEffect::AddFieldCondition {
                            condition,
                            duration: 5,
                        }
                    } else {
                        PrimaryMoveEffect::ToggleFieldCondition {
                            condition,
                            duration: 5,
                        }
                    });
                }
            }
        }
        if let Some(name) = data.side_condition.as_deref() {
            if matches!(name, "reflect" | "lightscreen" | "tailwind") {
                if let Some(condition) = condition(name) {
                    let duration = if name == "tailwind" { 4 } else { 5 };
                    effects.push(PrimaryMoveEffect::AddSideCondition {
                        condition,
                        duration,
                    });
                }
            }
        }
        effects
    }

    fn secondary_move_effects(&self, move_id: MoveId) -> Vec<SecondaryMoveEffect> {
        let Some(data) = self.move_data(move_id) else {
            return Vec::new();
        };
        data.secondaries
            .iter()
            .map(|secondary| {
                let mut effects = secondary
                    .status
                    .as_deref()
                    .and_then(parse_status)
                    .map(|status| PrimaryMoveEffect::InflictStatus { status })
                    .into_iter()
                    .collect::<Vec<_>>();
                effects.extend(secondary.boosts.iter().filter_map(|boost| {
                    Some(PrimaryMoveEffect::ChangeStatStage {
                        stat: parse_stat(&boost.key)?,
                        delta: boost.value,
                    })
                }));
                // Self-targeted secondary boosts are kept separate from target
                // effects by applying them through the move's self-effect path.
                SecondaryMoveEffect {
                    chance: secondary.chance,
                    effects,
                    self_effects: secondary
                        .self_boosts
                        .iter()
                        .filter_map(|boost| {
                            Some(PrimaryMoveEffect::ChangeStatStage {
                                stat: parse_stat(&boost.key)?,
                                delta: boost.value,
                            })
                        })
                        .collect(),
                }
            })
            .collect()
    }

    fn self_move_effect(&self, move_id: MoveId) -> Option<SelfMoveEffect> {
        let data = self.move_data(move_id)?;
        let value = data.self_effect.as_ref()?;
        Some(SelfMoveEffect {
            chance: value.chance,
            effects: value
                .boosts
                .iter()
                .filter_map(|boost| {
                    Some(PrimaryMoveEffect::ChangeStatStage {
                        stat: parse_stat(&boost.key)?,
                        delta: boost.value,
                    })
                })
                .collect(),
        })
    }

    fn move_is_protectable(&self, move_id: MoveId) -> bool {
        self.move_data(move_id)
            .is_some_and(|data| data.flags.iter().any(|flag| flag == "protect"))
    }

    fn move_breaks_protect(&self, move_id: MoveId) -> bool {
        self.move_data(move_id).is_some_and(|data| {
            data.behavior
                .special_properties
                .iter()
                .any(|property| property == "breaksProtect")
        })
    }

    fn field_effect_ids(&self) -> FieldEffectIds {
        let condition = |id: &str| {
            self.resolve_move(id)
                .map(|(move_id, _)| ConditionId::new(move_id.get()))
        };
        FieldEffectIds {
            rain: condition("raindance"),
            sun: condition("sunnyday"),
            sandstorm: condition("sandstorm"),
            snow: condition("snowscape"),
            electric_terrain: condition("electricterrain"),
            grassy_terrain: condition("grassyterrain"),
            psychic_terrain: condition("psychicterrain"),
            misty_terrain: condition("mistyterrain"),
            reflect: condition("reflect"),
            light_screen: condition("lightscreen"),
            aurora_veil: condition("auroraveil"),
            tailwind: condition("tailwind"),
            trick_room: condition("trickroom"),
            gravity: condition("gravity"),
            magic_room: condition("magicroom"),
            wonder_room: condition("wonderroom"),
            ingrain: condition("ingrain"),
            smack_down: condition("smackdown"),
            magnet_rise: condition("magnetrise"),
            telekinesis: condition("telekinesis"),
            levitate: self.resolve_ability("levitate").map(|(id, _)| id),
            iron_ball: self.resolve_item("ironball").map(|(id, _)| id),
            air_balloon: self.resolve_item("airballoon").map(|(id, _)| id),
            earthquake: self.resolve_move("earthquake").map(|(id, _)| id),
            bulldoze: self.resolve_move("bulldoze").map(|(id, _)| id),
            magnitude: self.resolve_move("magnitude").map(|(id, _)| id),
            thunder: self.resolve_move("thunder").map(|(id, _)| id),
            hurricane: self.resolve_move("hurricane").map(|(id, _)| id),
            blizzard: self.resolve_move("blizzard").map(|(id, _)| id),
            protect: self
                .resolve_move("protect")
                .map(|(id, _)| ConditionId::new(id.get())),
            follow_me: self
                .resolve_move("followme")
                .map(|(id, _)| ConditionId::new(id.get())),
            rage_powder: self
                .resolve_move("ragepowder")
                .map(|(id, _)| ConditionId::new(id.get())),
            // `stall` is not a move-backed generated identity; reserve a stable
            // condition ID immediately above the compact move-ID domain.
            stall: Some(ConditionId::new(
                u32::try_from(self.moves.len()).unwrap_or(u32::MAX),
            )),
            overcoat: self.resolve_ability("overcoat").map(|(id, _)| id),
            safety_goggles: self.resolve_item("safetygoggles").map(|(id, _)| id),
        }
    }

    fn ability_has_behavior(&self, ability: AbilityId, behavior: AbilityBehavior) -> bool {
        self.ability(ability).is_some_and(|ability| {
            implemented_ability_behaviors(ability.id.as_str())
                .any(|candidate| candidate == behavior)
        })
    }

    fn ability_handlers(&self) -> Vec<AbilityHandler> {
        IMPLEMENTED_ABILITY_BEHAVIORS
            .iter()
            .filter_map(|(key, behavior, suppression)| {
                let ability = self.resolve_ability(key).map(|(ability, _)| ability)?;
                Some(AbilityHandler::new(ability, *behavior, *suppression))
            })
            .collect()
    }
}

/// Declarative identity-to-behavior join for the implemented PR 13.5 batch.
/// Ability metadata and numeric identity remain owned by the generated Dex;
/// callbacks remain owned by `rotom-core`.
const IMPLEMENTED_ABILITY_BEHAVIORS: &[(&str, AbilityBehavior, AbilitySuppression)] = &[
    (
        "hugepower",
        AbilityBehavior::DoubleAttack,
        AbilitySuppression::Unbreakable,
    ),
    (
        "purepower",
        AbilityBehavior::DoubleAttack,
        AbilitySuppression::Unbreakable,
    ),
    (
        "guts",
        AbilityBehavior::StatusedAttack,
        AbilitySuppression::Unbreakable,
    ),
    (
        "furcoat",
        AbilityBehavior::DoubleDefense,
        AbilitySuppression::Breakable,
    ),
    (
        "marvelscale",
        AbilityBehavior::StatusedDefense,
        AbilitySuppression::Breakable,
    ),
    (
        "technician",
        AbilityBehavior::LowPowerBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "multiscale",
        AbilityBehavior::FullHpDamageReduction,
        AbilitySuppression::Breakable,
    ),
    (
        "quickfeet",
        AbilityBehavior::StatusedSpeed,
        AbilitySuppression::Unbreakable,
    ),
    (
        "quickfeet",
        AbilityBehavior::IgnoreParalysisSpeedPenalty,
        AbilitySuppression::Unbreakable,
    ),
    (
        "superluck",
        AbilityBehavior::IncreaseCriticalRatio,
        AbilitySuppression::Unbreakable,
    ),
    (
        "battlearmor",
        AbilityBehavior::PreventCriticalHit,
        AbilitySuppression::Breakable,
    ),
    (
        "shellarmor",
        AbilityBehavior::PreventCriticalHit,
        AbilitySuppression::Breakable,
    ),
    (
        "sniper",
        AbilityBehavior::BoostCriticalDamage,
        AbilitySuppression::Unbreakable,
    ),
    (
        "moldbreaker",
        AbilityBehavior::BreaksTargetAbilities,
        AbilitySuppression::Unbreakable,
    ),
    (
        "chlorophyll",
        AbilityBehavior::SunSpeed,
        AbilitySuppression::Unbreakable,
    ),
    (
        "swiftswim",
        AbilityBehavior::RainSpeed,
        AbilitySuppression::Unbreakable,
    ),
    (
        "slushrush",
        AbilityBehavior::SnowSpeed,
        AbilitySuppression::Unbreakable,
    ),
    (
        "surgesurfer",
        AbilityBehavior::ElectricTerrainSpeed,
        AbilitySuppression::Unbreakable,
    ),
    (
        "sandrush",
        AbilityBehavior::SandRush,
        AbilitySuppression::Unbreakable,
    ),
    (
        "prankster",
        AbilityBehavior::Prankster,
        AbilitySuppression::Unbreakable,
    ),
    (
        "bulletproof",
        AbilityBehavior::BulletImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "soundproof",
        AbilityBehavior::SoundImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "telepathy",
        AbilityBehavior::AllyDamageImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "blaze",
        AbilityBehavior::LowHpFireBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "overgrow",
        AbilityBehavior::LowHpGrassBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "torrent",
        AbilityBehavior::LowHpWaterBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "swarm",
        AbilityBehavior::LowHpBugBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "ironfist",
        AbilityBehavior::PunchBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "strongjaw",
        AbilityBehavior::BiteBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "sharpness",
        AbilityBehavior::SlicingBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "toughclaws",
        AbilityBehavior::ContactBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "reckless",
        AbilityBehavior::RecoilBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "punkrock",
        AbilityBehavior::SoundPower,
        AbilitySuppression::Breakable,
    ),
    (
        "sandforce",
        AbilityBehavior::SandForce,
        AbilitySuppression::Unbreakable,
    ),
    (
        "steelyspirit",
        AbilityBehavior::AllySteelBoost,
        AbilitySuppression::Unbreakable,
    ),
    (
        "filter",
        AbilityBehavior::SuperEffectiveReduction,
        AbilitySuppression::Breakable,
    ),
    (
        "solidrock",
        AbilityBehavior::SuperEffectiveReduction,
        AbilitySuppression::Breakable,
    ),
    (
        "fluffy",
        AbilityBehavior::Fluffy,
        AbilitySuppression::Breakable,
    ),
    (
        "thickfat",
        AbilityBehavior::ThickFat,
        AbilitySuppression::Breakable,
    ),
    (
        "grasspelt",
        AbilityBehavior::GrassyDefense,
        AbilitySuppression::Breakable,
    ),
    (
        "galewings",
        AbilityBehavior::GaleWings,
        AbilitySuppression::Unbreakable,
    ),
    (
        "goodasgold",
        AbilityBehavior::OpposingStatusImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "immunity",
        AbilityBehavior::PoisonImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "insomnia",
        AbilityBehavior::SleepImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "vitalspirit",
        AbilityBehavior::SleepImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "limber",
        AbilityBehavior::ParalysisImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "magmaarmor",
        AbilityBehavior::FreezeImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "sweetveil",
        AbilityBehavior::AllySleepImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "waterabsorb",
        AbilityBehavior::WaterAbsorb,
        AbilitySuppression::Breakable,
    ),
    (
        "voltabsorb",
        AbilityBehavior::VoltAbsorb,
        AbilitySuppression::Breakable,
    ),
    (
        "eartheater",
        AbilityBehavior::EarthEater,
        AbilitySuppression::Breakable,
    ),
    (
        "motordrive",
        AbilityBehavior::MotorDrive,
        AbilitySuppression::Breakable,
    ),
    (
        "sapsipper",
        AbilityBehavior::GrassImmunityAttackBoost,
        AbilitySuppression::Breakable,
    ),
    (
        "levitate",
        AbilityBehavior::GroundImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "sandveil",
        AbilityBehavior::WeatherEvasion(WeatherEnvironment::Sandstorm),
        AbilitySuppression::Breakable,
    ),
    (
        "snowcloak",
        AbilityBehavior::WeatherEvasion(WeatherEnvironment::Snow),
        AbilitySuppression::Breakable,
    ),
    (
        "auraguard",
        AbilityBehavior::ContactDamageReduction,
        AbilitySuppression::Breakable,
    ),
    (
        "bigpecks",
        AbilityBehavior::DefenseDropImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "clearbody",
        AbilityBehavior::AllStatDropImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "hypercutter",
        AbilityBehavior::AttackDropImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "whitesmoke",
        AbilityBehavior::AllStatDropImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "keeneye",
        AbilityBehavior::AccuracyDropImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "keeneye",
        AbilityBehavior::IgnoreTargetEvasion,
        AbilitySuppression::Breakable,
    ),
    (
        "armortail",
        AbilityBehavior::PriorityProtection,
        AbilitySuppression::Breakable,
    ),
    (
        "queenlymajesty",
        AbilityBehavior::PriorityProtection,
        AbilitySuppression::Breakable,
    ),
    (
        "firemane",
        AbilityBehavior::FirePower,
        AbilitySuppression::Unbreakable,
    ),
    (
        "adaptability",
        AbilityBehavior::Adaptability,
        AbilitySuppression::Unbreakable,
    ),
    (
        "dryskin",
        AbilityBehavior::WaterAbsorb,
        AbilitySuppression::Breakable,
    ),
    (
        "dryskin",
        AbilityBehavior::DrySkin,
        AbilitySuppression::Breakable,
    ),
    (
        "solarpower",
        AbilityBehavior::SolarPower,
        AbilitySuppression::Unbreakable,
    ),
    (
        "purifyingsalt",
        AbilityBehavior::MajorStatusImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "purifyingsalt",
        AbilityBehavior::GhostResistance,
        AbilitySuppression::Breakable,
    ),
    (
        "earlybird",
        AbilityBehavior::EarlyBird,
        AbilitySuppression::Unbreakable,
    ),
    (
        "corrosion",
        AbilityBehavior::Corrosion,
        AbilitySuppression::Unbreakable,
    ),
    (
        "shielddust",
        AbilityBehavior::ShieldDust,
        AbilitySuppression::Breakable,
    ),
    (
        "skilllink",
        AbilityBehavior::SkillLink,
        AbilitySuppression::Unbreakable,
    ),
    (
        "longreach",
        AbilityBehavior::LongReach,
        AbilitySuppression::Unbreakable,
    ),
    (
        "infiltrator",
        AbilityBehavior::Infiltrator,
        AbilitySuppression::Unbreakable,
    ),
    (
        "stalwart",
        AbilityBehavior::Stalwart,
        AbilitySuppression::Unbreakable,
    ),
    (
        "sturdy",
        AbilityBehavior::Sturdy,
        AbilitySuppression::Breakable,
    ),
    (
        "unseenfist",
        AbilityBehavior::ContactProtectBypass,
        AbilitySuppression::Unbreakable,
    ),
    (
        "piercingdrill",
        AbilityBehavior::ContactProtectBypass,
        AbilitySuppression::Unbreakable,
    ),
    (
        "megasol",
        AbilityBehavior::PersonalSun,
        AbilitySuppression::Unbreakable,
    ),
    (
        "scrappy",
        AbilityBehavior::Scrappy,
        AbilitySuppression::Unbreakable,
    ),
    (
        "liquidvoice",
        AbilityBehavior::LiquidVoice,
        AbilitySuppression::Unbreakable,
    ),
    (
        "overcoat",
        AbilityBehavior::PowderWeatherImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "waterbubble",
        AbilityBehavior::BurnImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "waterbubble",
        AbilityBehavior::WaterBubble,
        AbilitySuppression::Breakable,
    ),
    (
        "thermalexchange",
        AbilityBehavior::BurnImmunity,
        AbilitySuppression::Breakable,
    ),
    (
        "thermalexchange",
        AbilityBehavior::ThermalExchange,
        AbilitySuppression::Breakable,
    ),
    (
        "naturalcure",
        AbilityBehavior::NaturalCure,
        AbilitySuppression::Unbreakable,
    ),
    (
        "regenerator",
        AbilityBehavior::Regenerator,
        AbilitySuppression::Unbreakable,
    ),
    (
        "hospitality",
        AbilityBehavior::Hospitality,
        AbilitySuppression::Unbreakable,
    ),
    (
        "hydration",
        AbilityBehavior::Hydration,
        AbilitySuppression::Unbreakable,
    ),
    (
        "shedskin",
        AbilityBehavior::ShedSkin,
        AbilitySuppression::Unbreakable,
    ),
    (
        "healer",
        AbilityBehavior::Healer,
        AbilitySuppression::Unbreakable,
    ),
    (
        "stamina",
        AbilityBehavior::Stamina,
        AbilitySuppression::Unbreakable,
    ),
    (
        "weakarmor",
        AbilityBehavior::WeakArmor,
        AbilitySuppression::Unbreakable,
    ),
    (
        "justified",
        AbilityBehavior::Justified,
        AbilitySuppression::Unbreakable,
    ),
    (
        "angerpoint",
        AbilityBehavior::AngerPoint,
        AbilitySuppression::Unbreakable,
    ),
    (
        "friendguard",
        AbilityBehavior::FriendGuard,
        AbilitySuppression::Breakable,
    ),
    (
        "megalauncher",
        AbilityBehavior::MegaLauncher,
        AbilitySuppression::Unbreakable,
    ),
    (
        "merciless",
        AbilityBehavior::Merciless,
        AbilitySuppression::Unbreakable,
    ),
    (
        "compoundeyes",
        AbilityBehavior::CompoundEyes,
        AbilitySuppression::Unbreakable,
    ),
    (
        "hustle",
        AbilityBehavior::Hustle,
        AbilitySuppression::Unbreakable,
    ),
    (
        "icebody",
        AbilityBehavior::IceBody,
        AbilitySuppression::Unbreakable,
    ),
    (
        "raindish",
        AbilityBehavior::RainDish,
        AbilitySuppression::Unbreakable,
    ),
    (
        "moxie",
        AbilityBehavior::Moxie,
        AbilitySuppression::Unbreakable,
    ),
    (
        "gooey",
        AbilityBehavior::Gooey,
        AbilitySuppression::Unbreakable,
    ),
    (
        "anticipation",
        AbilityBehavior::NoAuthoritativeEffect,
        AbilitySuppression::Unbreakable,
    ),
    (
        "forewarn",
        AbilityBehavior::NoAuthoritativeEffect,
        AbilitySuppression::Unbreakable,
    ),
    (
        "frisk",
        AbilityBehavior::NoAuthoritativeEffect,
        AbilitySuppression::Unbreakable,
    ),
    (
        "runaway",
        AbilityBehavior::NoAuthoritativeEffect,
        AbilitySuppression::Unbreakable,
    ),
];

fn implemented_ability_behaviors(id: &str) -> impl Iterator<Item = AbilityBehavior> + '_ {
    IMPLEMENTED_ABILITY_BEHAVIORS
        .iter()
        .filter_map(move |(key, behavior, _)| (*key == id).then_some(*behavior))
}

fn move_execution_requirements(data: &MoveData) -> (MoveOperation, MoveCapabilities) {
    if data.id == "protect" {
        return (
            MoveOperation::NonDamage,
            MoveCapabilities::NONE
                .with(MoveCapability::NonDamagePrimary)
                .with(MoveCapability::Protection)
                .with(MoveCapability::Volatile),
        );
    }
    if matches!(data.id.as_str(), "followme" | "ragepowder") {
        return (
            MoveOperation::NonDamage,
            MoveCapabilities::NONE
                .with(MoveCapability::NonDamagePrimary)
                .with(MoveCapability::Redirection)
                .with(MoveCapability::Volatile),
        );
    }
    let (operation, mut required) = match data.damage {
        Some(crate::Damage::Fixed { .. }) => (
            MoveOperation::FixedDamage,
            MoveCapabilities::NONE.with(MoveCapability::FixedDamage),
        ),
        Some(crate::Damage::Level) => (
            MoveOperation::LevelDamage,
            MoveCapabilities::NONE.with(MoveCapability::FixedDamage),
        ),
        None if data.category != "Status" && data.base_power > 0 => (
            MoveOperation::StandardDamage,
            MoveCapabilities::NONE.with(MoveCapability::StandardDamage),
        ),
        None => (
            MoveOperation::NonDamage,
            MoveCapabilities::NONE.with(MoveCapability::NonDamagePrimary),
        ),
    };

    required = required.with(MoveCapability::Accuracy);
    required = required.with(target_capability(data.target));
    if data.category != "Status" && data.base_power > 0 {
        required = required
            .with(MoveCapability::PerTargetHit)
            .with(MoveCapability::Protection)
            .with(MoveCapability::Retargeting);
        if matches!(
            data.target,
            MoveTarget::AllAdjacent | MoveTarget::AllAdjacentFoes
        ) {
            required = required
                .with(MoveCapability::TargetSet)
                .with(MoveCapability::SpreadDamage);
        } else if matches!(
            data.target,
            MoveTarget::Normal | MoveTarget::AdjacentFoe | MoveTarget::Any
        ) {
            required = required.with(MoveCapability::Redirection);
        }
    }
    if data.multi_hit.is_some() {
        required = required.with(MoveCapability::MultiHit);
    }
    if data.drain.is_some() {
        required = required.with(MoveCapability::Drain);
    }
    if data.recoil.is_some() || data.self_damage.is_some() {
        required = required.with(MoveCapability::Recoil);
    }
    if data.id == "facade" {
        required = required.with(MoveCapability::ConditionalPower);
    }
    if data.override_offensive_stat.is_some()
        || data.override_defensive_stat.is_some()
        || data.ignore_negative_offensive
        || data.ignore_positive_defensive
    {
        required = required.with(MoveCapability::EffectiveStatOverride);
    }
    if data.status.is_some() {
        required = required.with(MoveCapability::MajorStatus);
    }
    if data.volatile_status.is_some() {
        required = required.with(MoveCapability::Volatile);
    }
    (operation, behavior_capabilities(data, required))
}

const fn target_capability(target: MoveTarget) -> MoveCapability {
    match target {
        MoveTarget::Normal | MoveTarget::Any | MoveTarget::AdjacentFoe => {
            MoveCapability::SelectedSingleTarget
        }
        MoveTarget::AdjacentAlly
        | MoveTarget::AdjacentAllyOrSelf
        | MoveTarget::All
        | MoveTarget::AllAdjacent
        | MoveTarget::AllAdjacentFoes
        | MoveTarget::Allies
        | MoveTarget::AllySide
        | MoveTarget::AllyTeam
        | MoveTarget::FoeSide
        | MoveTarget::RandomNormal
        | MoveTarget::Scripted
        | MoveTarget::Self_ => MoveCapability::AdvancedTargeting,
    }
}

fn behavior_capabilities(data: &MoveData, mut required: MoveCapabilities) -> MoveCapabilities {
    if data
        .behavior
        .effect_kinds
        .contains(&MoveBehaviorKind::SecondaryEffect)
    {
        required = required.with(MoveCapability::SecondaryEffect);
    }
    if data
        .behavior
        .effect_kinds
        .contains(&MoveBehaviorKind::SelfEffect)
    {
        required = required.with(MoveCapability::SelfEffect);
    }
    if data
        .behavior
        .effect_kinds
        .contains(&MoveBehaviorKind::StatStage)
    {
        required = required.with(MoveCapability::StatStage);
    }
    if data
        .behavior
        .effect_kinds
        .contains(&MoveBehaviorKind::Healing)
    {
        required = required.with(MoveCapability::Healing);
    }
    if data
        .behavior
        .effect_kinds
        .contains(&MoveBehaviorKind::FieldOrSideCondition)
    {
        required = required.with(MoveCapability::FieldOrSideCondition);
    }
    if data
        .behavior
        .effect_kinds
        .contains(&MoveBehaviorKind::CallsMove)
    {
        required = required.with(MoveCapability::CallsMove);
    }
    if data
        .behavior
        .effect_kinds
        .contains(&MoveBehaviorKind::SwitchControl)
    {
        required = required.with(MoveCapability::SwitchControl);
    }
    let has_unsupported_callback = data.behavior.callbacks.iter().any(|callback| {
        !((data.id == "magnetrise" && callback == "onTry")
            || (data.id == "facade" && callback == "onBasePower"))
    });
    if has_unsupported_callback {
        required = required.with(MoveCapability::CustomCallback);
    }
    if data.volatile_status.is_some() && data.volatile_status.as_deref() != Some("magnetrise") {
        required = required.with(MoveCapability::SpecialBehavior);
    }
    let unsupported_condition = data
        .side_condition
        .as_deref()
        .is_some_and(|condition| !matches!(condition, "reflect" | "lightscreen" | "tailwind"))
        || data.pseudo_weather.as_deref().is_some_and(|condition| {
            !matches!(
                condition,
                "trickroom" | "gravity" | "magicroom" | "wonderroom"
            )
        });
    let unsupported_special = data.behavior.special_properties.iter().any(|property| {
        !matches!(
            property.as_str(),
            "breaksProtect"
                | "ignoreImmunity"
                | "ignoreNegativeOffensive"
                | "ignorePositiveDefensive"
                | "overrideOffensiveStat"
                | "overrideDefensiveStat"
                | "multiaccuracy"
                | "chloroblastRecoil"
                | "mindBlownRecoil"
        )
    });
    let unsupported_flag = data.behavior.intrinsic_flags.iter().any(|flag| {
        (flag != "heal" || (data.heal.is_none() && data.drain.is_none()))
            && !(data.id == "magnetrise" && flag == "gravity")
    });
    if data.id == "toxic"
        || unsupported_condition
        || unsupported_flag
        || unsupported_special
        || !data.behavior.structured_effects_supported
        || matches!(data.target, MoveTarget::RandomNormal | MoveTarget::Scripted)
    {
        required = required.with(MoveCapability::SpecialBehavior);
    }
    required
}

fn parse_status(value: &str) -> Option<MajorStatus> {
    match value {
        "brn" => Some(MajorStatus::Burn),
        "frz" => Some(MajorStatus::Freeze),
        "par" => Some(MajorStatus::Paralysis),
        "psn" => Some(MajorStatus::Poison),
        "tox" => Some(MajorStatus::BadlyPoisoned),
        "slp" => Some(MajorStatus::Sleep),
        _ => None,
    }
}

fn parse_stat(value: &str) -> Option<BattleStat> {
    match value {
        "atk" => Some(BattleStat::Attack),
        "def" => Some(BattleStat::Defense),
        "spa" => Some(BattleStat::SpecialAttack),
        "spd" => Some(BattleStat::SpecialDefense),
        "spe" => Some(BattleStat::Speed),
        "accuracy" => Some(BattleStat::Accuracy),
        "evasion" => Some(BattleStat::Evasion),
        _ => None,
    }
}

impl DexData {
    /// Builds mutable battle state from a validated Regulation M-C set.
    ///
    /// This applies the pinned Champions stat-point formula and resolved move
    /// PP rules. Team legality remains the format layer's responsibility.
    ///
    /// # Errors
    ///
    /// Returns a data-mismatch error when the set references missing static
    /// data or calculated move PP cannot be represented by runtime state.
    pub fn pokemon_state(&self, set: &PokemonSet) -> Result<PokemonState, BattleError> {
        let species = selected_species(self, set.species, set.form).ok_or_else(|| {
            BattleError::DataMismatch("set species/form is missing from the Dex".to_owned())
        })?;
        let nature = self.nature(set.nature).ok_or_else(|| {
            BattleError::DataMismatch("set nature is missing from the Dex".to_owned())
        })?;
        let stats = champions_stats(
            species.base_stats,
            set.stat_points,
            set.level,
            nature.plus.as_deref(),
            nature.minus.as_deref(),
        );
        let moves = set
            .moves
            .iter()
            .copied()
            .map(|move_id| self.move_slot(move_id))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PokemonState {
            species: set.species,
            form: set.form,
            level: set.level,
            stats,
            current_hp: stats.hp,
            ability: set.ability,
            item: set.item,
            moves,
            status: None,
            status_state: StatusState::default(),
            stat_stages: StatStages::default(),
            volatiles: Vec::new(),
            can_mega_evolve: false,
            trapped: false,
            position: PokemonPosition::Team,
        })
    }

    fn move_slot(&self, move_id: MoveId) -> Result<MoveSlot, BattleError> {
        let data = self.move_data(move_id).ok_or_else(|| {
            BattleError::DataMismatch(format!("move {move_id} is missing from the Dex"))
        })?;
        let max_pp = calculate_pp(data)?;
        Ok(MoveSlot {
            move_id,
            target: data.target,
            priority: data.priority,
            pp: max_pp,
            max_pp,
            disabled: false,
        })
    }
}

fn selected_species(
    dex: &DexData,
    species: rotom_core::SpeciesId,
    form: Option<rotom_core::FormId>,
) -> Option<&SpeciesData> {
    match form {
        None => dex.species(species),
        Some(form) => dex
            .form(form)
            .and_then(|data| dex.resolve_species(&data.id).map(|(_, value)| value)),
    }
}

fn calculate_pp(data: &MoveData) -> Result<u8, BattleError> {
    if data.no_pp_boosts {
        return Ok(data.pp);
    }
    let value = (u16::from(data.pp) / 5 + 1) * 4;
    u8::try_from(value)
        .map_err(|_| BattleError::DataMismatch("calculated PP exceeds u8".to_owned()))
}

fn champions_stats(
    base: Stats,
    points: StatAllocation,
    level: u8,
    nature_plus: Option<&str>,
    nature_minus: Option<&str>,
) -> BattleStats {
    let hp = champion_stat(base.hp, points.hp, level, true);
    let mut atk = champion_stat(base.atk, points.atk, level, false);
    let mut def = champion_stat(base.def, points.def, level, false);
    let mut spa = champion_stat(base.spa, points.spa, level, false);
    let mut spd = champion_stat(base.spd, points.spd, level, false);
    let mut spe = champion_stat(base.spe, points.spe, level, false);
    for (name, stat) in [
        ("atk", &mut atk),
        ("def", &mut def),
        ("spa", &mut spa),
        ("spd", &mut spd),
        ("spe", &mut spe),
    ] {
        if nature_plus == Some(name) {
            *stat = u16::try_from(u32::from(*stat) * 110 / 100).unwrap_or(u16::MAX);
        } else if nature_minus == Some(name) {
            *stat = u16::try_from(u32::from(*stat) * 90 / 100).unwrap_or(u16::MAX);
        }
    }
    BattleStats {
        hp,
        atk,
        def,
        spa,
        spd,
        spe,
    }
}

fn champion_stat(base: u16, points: u8, level: u8, hp: bool) -> u16 {
    let point_term = u32::from(points).saturating_mul(2).saturating_sub(1);
    let scaled = (u32::from(base) * 2 + 31 + point_term) * u32::from(level) / 100;
    let value = if hp {
        scaled + u32::from(level) + 10
    } else {
        scaled + 5
    };
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn parse_category(value: &str) -> Option<MoveCategory> {
    match value {
        "Physical" => Some(MoveCategory::Physical),
        "Special" => Some(MoveCategory::Special),
        "Status" => Some(MoveCategory::Status),
        _ => None,
    }
}

fn parse_type(value: &str) -> Option<PokemonType> {
    match value.to_ascii_lowercase().as_str() {
        "bug" => Some(PokemonType::Bug),
        "dark" => Some(PokemonType::Dark),
        "dragon" => Some(PokemonType::Dragon),
        "electric" => Some(PokemonType::Electric),
        "fairy" => Some(PokemonType::Fairy),
        "fighting" => Some(PokemonType::Fighting),
        "fire" => Some(PokemonType::Fire),
        "flying" => Some(PokemonType::Flying),
        "ghost" => Some(PokemonType::Ghost),
        "grass" => Some(PokemonType::Grass),
        "ground" => Some(PokemonType::Ground),
        "ice" => Some(PokemonType::Ice),
        "normal" => Some(PokemonType::Normal),
        "poison" => Some(PokemonType::Poison),
        "psychic" => Some(PokemonType::Psychic),
        "rock" => Some(PokemonType::Rock),
        "steel" => Some(PokemonType::Steel),
        "stellar" => Some(PokemonType::Stellar),
        "water" => Some(PokemonType::Water),
        _ => None,
    }
}

const fn type_id(value: PokemonType) -> &'static str {
    match value {
        PokemonType::Bug => "bug",
        PokemonType::Dark => "dark",
        PokemonType::Dragon => "dragon",
        PokemonType::Electric => "electric",
        PokemonType::Fairy => "fairy",
        PokemonType::Fighting => "fighting",
        PokemonType::Fire => "fire",
        PokemonType::Flying => "flying",
        PokemonType::Ghost => "ghost",
        PokemonType::Grass => "grass",
        PokemonType::Ground => "ground",
        PokemonType::Ice => "ice",
        PokemonType::Normal => "normal",
        PokemonType::Poison => "poison",
        PokemonType::Psychic => "psychic",
        PokemonType::Rock => "rock",
        PokemonType::Steel => "steel",
        PokemonType::Stellar => "stellar",
        PokemonType::Water => "water",
    }
}
