//! Native evidence for [`super`]: `tools/spatial_oracle/building_guard_attack.json`
//! replayed through the Rust handlers and, for its cadence rows, through the
//! production frame; and the retail inputs that keep the module's dormant arms
//! dormant.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::*;
use crate::map::resolved_terrain::test_flat_ground_grid;
use crate::rules::ini_parser::IniFile;
use crate::sim::combat::AttackTarget;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::game_entity::GameEntity;
use crate::sim::mission::MissionDispatchTimer;
use crate::sim::mission::state::MissionTestFixture;
use crate::sim::rng::SimRng;

/// The oracle fixture's frame.
const FRAME: u32 = 200;

/// GetFireError's codes by number, as the oracle's rows name them.
const CODES: [FireError; 12] = [
    FireError::Ok,
    FireError::Ammo,
    FireError::Facing,
    FireError::Rearm,
    FireError::Rotating,
    FireError::Illegal,
    FireError::Cant,
    FireError::Moving,
    FireError::Range,
    FireError::Cloaked,
    FireError::Busy,
    FireError::MustDeploy,
];

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tools/spatial_oracle/building_guard_attack.json"
    ))
    .unwrap()
}

fn rows<'a>(golden: &'a Value, set: &str) -> &'a [Value] {
    golden[set].as_array().unwrap()
}

/// The oracle's MissionControl rates (Guard `.030` and the row's AARate,
/// Sticky and Attack `.016`) and a building for each handler arm. `DEF`
/// cannot passive-acquire, so a cadence row's target comes only from its
/// events, as in the oracle; `SHED` is the unarmed enemy they aim at.
fn rules(guard_aa_rate: &str, rof: u32) -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str(&format!(
        "[Guard]\nRate=.030\nAARate={guard_aa_rate}\n\
         [Sticky]\nRate=.016\nAARate=.016\n\
         [Attack]\nRate=.016\nAARate=.016\n\
         [BuildingTypes]\n0=DEF\n1=TDEF\n2=EMPDEF\n3=AADEF\n4=PDEF\n5=HUT\n6=DEPOT\n\
         7=FACTORY\n8=SHED\n\
         [DEF]\nStrength=1000\nArmor=concrete\nFoundation=1x1\nPrimary=Gun\n\
         CanPassiveAquire=no\n\
         [TDEF]\nImage=TDEFART\nStrength=1000\nArmor=concrete\nFoundation=1x1\nPrimary=Gun\n\
         Turret=yes\nROT=5\n\
         [EMPDEF]\nStrength=1000\nArmor=concrete\nFoundation=1x1\nPrimary=Gun\nEMPulseCannon=yes\n\
         [AADEF]\nStrength=1000\nArmor=concrete\nFoundation=1x1\nPrimary=Flak\n\
         [PDEF]\nStrength=1000\nArmor=concrete\nFoundation=1x1\nPrimary=Gun\nPowered=yes\n\
         Power=-50\n\
         [HUT]\nStrength=1000\nArmor=wood\nFoundation=2x2\nHasStupidGuardMode=no\n\
         [DEPOT]\nStrength=1000\nArmor=wood\nFoundation=2x2\nUnitRepair=yes\n\
         HasStupidGuardMode=no\n\
         [FACTORY]\nStrength=1000\nArmor=wood\nFoundation=2x2\nWeaponsFactory=yes\n\
         HasStupidGuardMode=no\n\
         [SHED]\nStrength=1000\nArmor=wood\nFoundation=1x1\n\
         [Gun]\nDamage=10\nROF={rof}\nRange=6\nProjectile=Shell\nWarhead=AP\n\
         [Flak]\nDamage=10\nROF={rof}\nRange=6\nProjectile=FlakShell\nWarhead=AP\n\
         [Shell]\nAG=yes\n\
         [FlakShell]\nAA=yes\nAG=no\n\
         [AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n"
    )))
    .unwrap()
}

/// `TDEF` with `IsAnimDelayedFire=` and the row's `DelayedFireDelay=`.
fn with_delayed_fire(mut rules: RuleSet, delay: i64) -> RuleSet {
    let art = crate::rules::art_data::ArtRegistry::from_ini(&IniFile::from_str(&format!(
        "[TDEFART]\nIsAnimDelayedFire=yes\nDelayedFireDelay={delay}\n"
    )));
    rules.merge_art_data(&art);
    rules
}

/// A building of `kind` at (5, 5) and an enemy `SHED` at `target_cell`, on a
/// flat map, at the oracle's frame.
fn fixture(rules: &RuleSet, kind: &str, target_cell: (u16, u16)) -> (Simulation, u64, u64) {
    let mut sim = Simulation::new();
    sim.install_resolved_terrain_for_new_map(test_flat_ground_grid(16));
    let heights = BTreeMap::new();
    let building = sim
        .spawn_object(kind, "Americans", 5, 5, 0, rules, &heights)
        .unwrap();
    let target = sim
        .spawn_object(
            "SHED",
            "Russians",
            target_cell.0,
            target_cell.1,
            0,
            rules,
            &heights,
        )
        .unwrap();
    sim.session.binary_frame = FRAME;
    (sim, building, target)
}

fn mission_of(name: &str) -> MissionId {
    match name {
        "none" => MissionId::NONE,
        "attack" => MissionId::from_known(MissionType::Attack),
        "guard" => MissionId::from_known(MissionType::Guard),
        "sticky" => MissionId::from_known(MissionType::Sticky),
        "selling" => MissionId::from_known(MissionType::Selling),
        other => panic!("unexpected mission {other}"),
    }
}

/// The row's mission (+0xAC) and status (+0xBC), with the oracle's start
/// frame, timer and a clear `+0x6DD`.
fn set_mission(sim: &mut Simulation, id: u64, mission: MissionId, status: u32) {
    let entity = sim.substrate.entities.get_mut(id).unwrap();
    entity.mission.apply_test_fixture(MissionTestFixture {
        current: mission,
        suspended: MissionId::NONE,
        queued: MissionId::NONE,
        movement_bypass_latch: 0,
        handler_state: status,
        mission_start_frame: FRAME,
        ai_counter: 0,
        dispatch_timer: MissionDispatchTimer::from_raw(FRAME as i32, 0),
    });
    entity.mission_leaf.set_building_ready_latch(0);
}

fn aim_at(sim: &mut Simulation, id: u64, target: u64) {
    sim.substrate.entities.get_mut(id).unwrap().attack_target = Some(AttackTarget::new(target));
}

fn ready_latch(sim: &Simulation, id: u64) -> u8 {
    sim.substrate
        .entities
        .get(id)
        .unwrap()
        .mission_leaf
        .as_building()
        .unwrap()
        .ready_latch()
}

/// Runs `call` and answers its return, asserting the row's Scenario draws:
/// the state is the one that many `RandomRanged(0, 2)` leave.
fn with_draws(sim: &mut Simulation, row: &Value, call: impl FnOnce(&mut Simulation) -> i32) -> i32 {
    let mut expected = sim.scenario_rng.clone();
    for draw in row["draws"].as_array().unwrap() {
        assert_eq!(draw, &serde_json::json!(["random_ranged", 0, 2]));
        expected.next_range_u32_inclusive(0, 2);
    }
    let returns = call(sim);
    assert_eq!(
        sim.scenario_rng.logical_state(),
        expected.logical_state(),
        "{} draws",
        row["input"]["name"]
    );
    returns
}

/// The row's recorded end state: mission, queue, status, `+0x6DD`, TarCom,
/// `+0xC4` and the mission start frame.
fn assert_state(sim: &Simulation, id: u64, target: u64, row: &Value) {
    let name = &row["input"]["name"];
    let entity = sim.substrate.entities.get(id).unwrap();
    assert_eq!(
        entity.mission.current(),
        mission_of(row["mission"].as_str().unwrap()),
        "{name} mission"
    );
    assert_eq!(row["queued"], "none", "{name}");
    assert_eq!(entity.mission.queued(), MissionId::NONE, "{name} queue");
    assert_eq!(
        u64::from(entity.mission.handler_state()),
        row["status"].as_u64().unwrap(),
        "{name} status"
    );
    assert_eq!(
        u64::from(ready_latch(sim, id)),
        row["ready"].as_u64().unwrap(),
        "{name} +0x6DD"
    );
    assert_eq!(
        entity.attack_target.as_ref().map(|attack| attack.target),
        row["target"].as_str().map(|_| TargetKind::Entity(target)),
        "{name} target"
    );
    assert_eq!(
        u64::from(entity.mission.ai_counter()),
        row["counter"].as_u64().unwrap(),
        "{name} +0xC4"
    );
    assert_eq!(
        u64::from(entity.mission.mission_start_frame()),
        row["mission_start"].as_u64().unwrap(),
        "{name} mission start"
    );
}

fn called(row: &Value, name: &str) -> bool {
    row["calls"]
        .as_array()
        .unwrap()
        .iter()
        .any(|call| call[0] == name)
}

/// Every `guard` row: Mission_Guard's return, Scenario draw, mission switch,
/// status and `+0x6DD` per arm, including a modded AARate whose x900 has a
/// fraction above .5 (ftol chops). The seeded rows pin the draw itself
/// (seeds 1, 2 and 8 draw 1, 2 and 0; seed 9 draws again); the other rows
/// draw from the oracle fixture's state, whose first draw is 1 like seed 1's.
/// RESIDUAL (module doc): `unarmed_factory_status1`'s ClearBibArea.
#[test]
fn mission_guard_matches_the_original() {
    let golden = golden();
    let guard = rows(&golden, "guard");
    assert_eq!(guard.len(), 17);
    for row in guard {
        let input = &row["input"];
        let name = input["name"].as_str().unwrap();
        let flag = |key: &str| input[key].as_bool().unwrap_or(false);
        let rules = rules(input["rates"]["guard"][1].as_str().unwrap_or(".016"), 20);
        let kind = if input["armed"].as_bool().unwrap_or(true) {
            if flag("emp_cannon") { "EMPDEF" } else { "DEF" }
        } else if flag("stupid_guard") {
            "SHED"
        } else if flag("unit_repair") {
            "DEPOT"
        } else if flag("weapons_factory") {
            "FACTORY"
        } else {
            "HUT"
        };
        let (mut sim, building, target) = fixture(&rules, kind, (8, 5));
        set_mission(
            &mut sim,
            building,
            mission_of(input["mission"].as_str().unwrap()),
            input["status"].as_u64().unwrap_or(0) as u32,
        );
        if flag("target") {
            aim_at(&mut sim, building, target);
        }
        sim.scenario_rng = SimRng::new(input["seed"].as_u64().unwrap_or(1));
        let returns = with_draws(&mut sim, row, |sim| mission_guard(sim, building, &rules));
        assert_eq!(
            i64::from(returns),
            row["returns"].as_i64().unwrap(),
            "{name}"
        );
        assert_state(&sim, building, target, row);
    }
}

/// Every `attack` row: Mission_Attack's return and actions for each
/// GetFireError code (answered from the row, as the oracle answers it), the
/// null-target tail and the delayed-fire OK arm: FireAt becomes the combat
/// phase's request, Set_Desired the turret's desired facing, StartUncloaking
/// the cloak's state 3, and the delayed arm's `+0x704/+0x708/+0x714` the
/// pending shot. RESIDUAL (module doc): `+0x148`, the rows' `turret_count`.
#[test]
fn mission_attack_matches_the_original() {
    for (number, code) in CODES.iter().enumerate() {
        assert_eq!(*code as usize, number);
    }
    let golden = golden();
    let attack = rows(&golden, "attack");
    assert_eq!(attack.len(), 15);
    for row in attack {
        let input = &row["input"];
        let name = input["name"].as_str().unwrap();
        let mut rules = rules(".016", 20);
        if input["delayed_fire"] == true {
            rules = with_delayed_fire(rules, input["delayed_fire_delay"].as_i64().unwrap());
        }
        let (mut sim, building, target) = fixture(&rules, "TDEF", (8, 5));
        set_mission(&mut sim, building, mission_of("attack"), 0);
        if input["target"] == true {
            aim_at(&mut sim, building, target);
        }
        // Facing away from the target, and fully cloaked (as a CloakGenerator
        // leaves it), so Set_Desired and StartUncloaking each leave a mark.
        let toward = {
            let entity = sim.substrate.entities.get(building).unwrap();
            crate::sim::movement::turret::facing_toward_target(
                entity,
                &TargetKind::Entity(target),
                &sim.substrate.entities,
                Some(&rules),
                &sim.interner,
            )
            .unwrap()
        };
        let away = toward.wrapping_add(0x8000);
        let entity = sim.substrate.entities.get_mut(building).unwrap();
        entity.barrel_facing.as_mut().unwrap().snap(away, FRAME);
        let mut cloak = crate::sim::cloak_disguise::CloakRuntime::new(
            FRAME as i32,
            rules.general.cloaking_stages,
        );
        cloak.state = 2;
        entity.cloak = Some(cloak);

        let returns = with_draws(&mut sim, row, |sim| {
            let Some((target, weapon)) = attack_prelude(sim, building, &rules) else {
                return 1;
            };
            assert_eq!(weapon, 0, "{name}");
            let code = CODES[input["errors"][0].as_u64().unwrap() as usize];
            attack_arm(sim, building, &rules, target, weapon, code)
        });
        assert_eq!(
            i64::from(returns),
            row["returns"].as_i64().unwrap(),
            "{name}"
        );
        assert_state(&sim, building, target, row);

        let entity = sim.substrate.entities.get(building).unwrap();
        assert_eq!(
            sim.fire_requests.buildings.contains(&building),
            called(row, "fire_at"),
            "{name} FireAt"
        );
        let desired = entity.barrel_facing.unwrap().destination();
        assert_eq!(
            desired,
            if called(row, "set_desired") {
                toward
            } else {
                away
            },
            "{name} Set_Desired"
        );
        assert_eq!(
            entity.cloak.as_ref().unwrap().state,
            if called(row, "uncloak") { 3 } else { 2 },
            "{name} StartUncloaking"
        );
        let delayed = &row["delayed_fire"];
        assert_eq!(
            entity.pending_building_fire,
            (delayed[0] == 1).then(|| PendingBuildingFire {
                remaining_ticks: delayed[2].as_i64().unwrap() as i32,
                weapon_slot: WeaponSlot::Primary,
            }),
            "{name} delayed fire"
        );
        assert_eq!(delayed[1], 0, "{name}");
    }
}

/// Every `set_target` row through the production setter
/// (`Simulation::assign_target_represented`): BuildingClass::SetTarget keeps a
/// target in range, one its slot-0 weapon cannot aim (none, or an AA
/// projectile) and NULL, and takes none while Selling or not operational.
/// RESIDUAL (module doc): the `artillery_out_of_range` row's undeploy, dormant
/// with retail data ([`retail_building_mission_inputs`]).
#[test]
fn set_target_matches_the_original() {
    let golden = golden();
    let set_target = rows(&golden, "set_target");
    assert_eq!(set_target.len(), 10);
    let rules = rules(".016", 20);
    for row in set_target {
        let input = &row["input"];
        let name = input["name"].as_str().unwrap();
        let flag = |key: &str| input[key].as_bool().unwrap_or(false);
        let kind = if !input["armed"].as_bool().unwrap_or(true) {
            "HUT"
        } else if flag("anti_air") {
            "AADEF"
        } else if input["operational"] == false {
            "PDEF"
        } else {
            "DEF"
        };
        let cell = if flag("in_range") { (8, 5) } else { (15, 5) };
        let (mut sim, building, target) = fixture(&rules, kind, cell);
        set_mission(
            &mut sim,
            building,
            mission_of(input["mission"].as_str().unwrap()),
            0,
        );
        if kind == "PDEF" {
            let owner = sim.interner.intern("Americans");
            sim.power_states.insert(
                owner,
                crate::sim::power_system::PowerState {
                    total_drain: 100,
                    is_low_power: true,
                    ..Default::default()
                },
            );
        }
        let requested = (input["args"][0] != 0).then_some(TargetKind::Entity(target));
        sim.assign_target_represented(building, requested, Some(&rules))
            .unwrap();
        assert_eq!(
            sim.substrate
                .entities
                .get(building)
                .unwrap()
                .attack_target
                .as_ref()
                .map(|attack| attack.target),
            row["target"].as_str().map(|_| TargetKind::Entity(target)),
            "{name}"
        );
        assert_eq!(
            sim.building_admits_target(building, requested, &rules),
            row["target"].is_string() || requested.is_none(),
            "{name} admission"
        );
    }
}

/// The `unlimbo` rows' Scenario-init arm: a building placed without a
/// build-up queues Guard without commencing; the first opening's `+0x6DD`
/// (Grand_Opening `0x004467C9`) lets its first Update's ready check commence
/// it, and that Update's Mission_Guard runs at once. The build-up arm (mission
/// 18, Construction) is VERA's build-up owner, which queues and commences
/// Guard as it completes.
#[test]
fn a_placed_building_queues_guard_and_commences_it_on_its_first_update() {
    let golden = golden();
    let unlimbo = rows(&golden, "unlimbo");
    assert_eq!(unlimbo.len(), 12);
    for row in unlimbo {
        let args = &row["input"]["args"];
        let instant =
            args[0] == 0 || row["input"]["scenario_init"] == 1 || row["input"]["flag_ed6b"] == 1;
        assert_eq!(row["mission"], "none");
        assert_eq!(
            row["queued"],
            if instant {
                serde_json::json!("guard")
            } else {
                serde_json::json!(18)
            },
            "{}",
            row["input"]["name"]
        );
    }

    let rules = rules(".016", 20);
    let (mut sim, building, _) = fixture(&rules, "DEF", (15, 5));
    let entity = sim.substrate.entities.get(building).unwrap();
    assert_eq!(entity.mission.current(), MissionId::NONE);
    assert_eq!(entity.mission.queued(), mission_of("guard"));
    assert_eq!(ready_latch(&sim, building), 1);
    sim.session.binary_frame += 1;
    object_ai_visit(&mut sim, building, &rules);
    let entity = sim.substrate.entities.get(building).unwrap();
    assert_eq!(entity.mission.current(), mission_of("guard"));
    assert_eq!(entity.mission.queued(), MissionId::NONE);
    let timer = entity.mission.dispatch_timer();
    assert_eq!(timer.start_frame(), sim.session.binary_frame as i32);
    assert!((14..=16).contains(&timer.delay()), "Guard's AARate delay");
}

fn object_ai_visit(sim: &mut Simulation, id: u64, rules: &RuleSet) {
    sim.object_ai_visit_one(id, Some(rules), ObjectAiCtx::default());
}

/// End of Update: a target out of range of SelectWeapon's weapon is dropped;
/// an aircraft only while it is low.
#[test]
fn the_update_tail_drops_a_target_out_of_range() {
    let rules = rules(".016", 20);
    let (mut sim, building, target) = fixture(&rules, "DEF", (15, 5));
    aim_at(&mut sim, building, target);
    range_drop(&mut sim, building, &rules, ObjectAiCtx::default());
    assert!(
        sim.substrate
            .entities
            .get(building)
            .unwrap()
            .attack_target
            .is_none()
    );

    let (mut sim, building, target) = fixture(&rules, "DEF", (8, 5));
    aim_at(&mut sim, building, target);
    range_drop(&mut sim, building, &rules, ObjectAiCtx::default());
    assert!(
        sim.substrate
            .entities
            .get(building)
            .unwrap()
            .attack_target
            .is_some()
    );
}

/// One cadence row's event, applied between frames `k - 1` and `k`: the
/// passive scan's or a detach's TarCom write, or a player's Attack order
/// (Queue_Mission(Attack) with the target).
enum Event {
    Acquire,
    Lose,
    Order,
}

/// Every `cadence` row through the production frame (`advance_tick`): per
/// frame the building's mission, TarCom, `+0x6DD`, dispatch timer and shot.
/// Native: a rearm R shoots every R + 1 frames when even and every R when
/// odd, since Mission_Attack returns 1 after a shot and 2 while it rearms;
/// R = 1 shoots every frame. The oracle answers FireAt with a rearm of exactly
/// the row's ROF, so the replay overwrites the one each shot wrote: GetROF's
/// own `RandomRanged(0, 2)` (`0x006FD09E`) is `combat::rof`'s, pinned by
/// `tools/spatial_oracle/rearm_timer.py`. The first Guard draw is pinned by
/// seeding the Scenario stream with 1 (its first draw is 1, as the oracle
/// fixture's is); a later Guard draw is not, so a row is compared up to the
/// dispatch its delay schedules, and that delay is checked against AARate's
/// 14 plus 0..=2.
#[test]
fn building_dispatch_cadence_matches_the_original() {
    let golden = golden();
    let cadence = rows(&golden, "cadence");
    assert_eq!(cadence.len(), 5);
    let mut shots_compared = 0;
    for row in cadence {
        let input = &row["input"];
        let name = input["name"].as_str().unwrap();
        assert_eq!(input["start"], "map");
        let rules = rules(".016", input["rof"].as_u64().unwrap() as u32);
        let mut sim = Simulation::new();
        sim.install_resolved_terrain_for_new_map(test_flat_ground_grid(16));
        let heights = BTreeMap::new();
        let building = sim
            .spawn_object("DEF", "Americans", 5, 5, 0, &rules, &heights)
            .unwrap();
        let target = sim
            .spawn_object("SHED", "Russians", 8, 5, 0, &rules, &heights)
            .unwrap();
        let owner = sim.interner.intern("Americans");
        let events: BTreeMap<u64, Event> = input["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|event| {
                let kind = match event[1].as_str().unwrap() {
                    "acquire" => Event::Acquire,
                    "lose" => Event::Lose,
                    "order" => Event::Order,
                    other => panic!("event {other}"),
                };
                (event[0].as_u64().unwrap(), kind)
            })
            .collect();
        sim.scenario_rng = SimRng::new(1);
        // The object pass of the frame `advance_tick` commits as `start + k`
        // runs at `base + k`, the frame its timers record.
        let start = sim.session.binary_frame;
        let base = i64::from(start) - 1;
        let mut guard_draws = 0;
        // A timer an unpinned Guard delay wrote: its start, and the frame it
        // schedules the next dispatch for.
        let mut unpinned: Option<(i64, i64)> = None;
        for frame in row["frames"].as_array().unwrap() {
            let k = frame["frame"].as_u64().unwrap();
            let oracle_timer = (
                frame["timer"][0].as_i64().unwrap() - i64::from(FRAME),
                frame["timer"][1].as_i64().unwrap(),
            );
            if let Some((unpinned_start, until)) = unpinned {
                if oracle_timer.0 == until {
                    break;
                }
                if oracle_timer.0 != unpinned_start {
                    // Rewritten by a Commence the delay did not schedule.
                    unpinned = None;
                }
            }
            let mut commands = Vec::new();
            match events.get(&k) {
                Some(Event::Acquire) => {
                    sim.assign_target_represented(
                        building,
                        Some(TargetKind::Entity(target)),
                        Some(&rules),
                    )
                    .unwrap();
                }
                Some(Event::Lose) => {
                    sim.assign_target_represented(building, None, Some(&rules))
                        .unwrap();
                }
                Some(Event::Order) | None => {}
            }
            // A command reaches the frame's tail, before the next frame's
            // ready check: the oracle's frame-start order.
            if matches!(events.get(&(k + 1)), Some(Event::Order)) {
                commands.push(CommandEnvelope::new(
                    owner,
                    sim.session.tick + 1,
                    Command::Attack {
                        attacker_id: building,
                        target_id: target,
                    },
                ));
            }
            sim.fire_events.clear();
            sim.advance_tick(&commands, Some(&rules), &heights, None, None, 67);
            assert_eq!(u64::from(sim.session.binary_frame - start), k, "{name}");

            let at = format!("{name} frame {k}");
            let entity = sim.substrate.entities.get(building).unwrap();
            assert_eq!(
                entity.mission.current(),
                mission_of(frame["mission"].as_str().unwrap()),
                "{at} mission"
            );
            // An order the frame's tail executed is the next frame's start in
            // the oracle, which records this frame before it.
            if !matches!(events.get(&(k + 1)), Some(Event::Order)) {
                assert_eq!(entity.mission.queued(), MissionId::NONE, "{at} queue");
                assert_eq!(
                    entity.attack_target.is_some(),
                    frame["target"].is_string(),
                    "{at} target"
                );
            }
            assert_eq!(
                u64::from(ready_latch(&sim, building)),
                frame["ready"].as_u64().unwrap(),
                "{at} +0x6DD"
            );
            let timer = entity.mission.dispatch_timer();
            let rust_timer = (
                i64::from(timer.start_frame()) - base,
                i64::from(timer.delay()),
            );
            let events = frame["events"].as_array().unwrap();
            let drew = events.iter().any(|event| event[0] == "random_ranged");
            if drew {
                guard_draws += 1;
            }
            if drew && guard_draws > 1 {
                assert_eq!(rust_timer.0, oracle_timer.0, "{at} timer start");
                assert!((14..=16).contains(&rust_timer.1), "{at} Guard delay");
                unpinned = Some((oracle_timer.0, oracle_timer.0 + oracle_timer.1));
            } else if unpinned.is_none() {
                assert_eq!(rust_timer, oracle_timer, "{at} timer");
            }
            let shot = sim
                .fire_events
                .iter()
                .any(|event| event.attacker_id == building);
            if shot {
                let rof = input["rof"].as_i64().unwrap() as i32;
                sim.substrate
                    .entities
                    .get_mut(building)
                    .unwrap()
                    .rearm_timer
                    .start((base + k as i64) as i32, rof);
            }
            let native_shot = events.iter().any(|event| event[0] == "fire_at");
            assert_eq!(shot, native_shot, "{at} shot");
            if native_shot {
                shots_compared += 1;
            }
        }
    }
    // Three shots each at ROF 20 and 21, eight at ROF 1, one before the
    // target is lost and one on the player's order.
    assert_eq!(shots_compared, 3 + 3 + 8 + 1 + 1);
}

/// The retail inputs the handlers read, through the production reader:
/// HasStupidGuardMode is cleared on these fourteen types; the unarmed ones
/// among them are the depots, the airfields, the Tank Bunker and the missile
/// silo, which take the unarmed status arm, and none is a WeaponsFactory
/// (ClearBibArea dormant). No building sets `TickTank=`, `Artillary=` or
/// `SAM=`, and no armed one `SuperWeapon=`, so those arms are dormant.
#[test]
fn retail_building_mission_inputs() {
    let Some((rules_ini, _)) = crate::rules::retail_ini_fixture::retail_rules_and_art() else {
        return;
    };
    let rules = RuleSet::from_ini(&rules_ini).unwrap();
    let buildings: Vec<&crate::rules::object_type::ObjectType> = rules
        .building_ids
        .iter()
        .filter_map(|id| rules.object(id))
        .collect();
    let armed = |obj: &crate::rules::object_type::ObjectType| {
        let mut entity = GameEntity::test_default(1, &obj.id, "Americans", 1, 1);
        entity.category = EntityCategory::Structure;
        combat_weapon::is_armed(&entity, obj)
    };
    let cleared: BTreeSet<&str> = buildings
        .iter()
        .filter(|obj| !obj.has_stupid_guard_mode)
        .map(|obj| obj.id.as_str())
        .collect();
    assert_eq!(
        cleared,
        BTreeSet::from([
            "AMRADR", "ATESLA", "GAAIRC", "GADEPT", "GAPILL", "NADEPT", "NAFLAK", "NALASR",
            "NAMISL", "NASAM", "NATBNK", "TESLA", "YADEPT", "YAGGUN",
        ])
    );
    let unarmed: BTreeSet<&str> = buildings
        .iter()
        .filter(|obj| !obj.has_stupid_guard_mode && !armed(obj))
        .map(|obj| obj.id.as_str())
        .collect();
    assert_eq!(
        unarmed,
        BTreeSet::from([
            "AMRADR", "GAAIRC", "GADEPT", "NADEPT", "NAMISL", "NATBNK", "YADEPT"
        ])
    );
    for obj in &buildings {
        let id = obj.id.as_str();
        assert!(!(obj.weapons_factory && !obj.has_stupid_guard_mode), "{id}");
        assert!(!obj.tick_tank && !obj.artillary, "{id}");
        assert!(!(armed(obj) && obj.super_weapon.is_some()), "{id}");
        let sam = rules_ini
            .section(id)
            .and_then(|section| section.get_bool("SAM"))
            .unwrap_or(false);
        assert!(!sam, "{id}");
    }
}
