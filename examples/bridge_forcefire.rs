//! Release-mode composition witness on unmodified retail Hills.mmx.
//! Run: cargo run --release --example bridge_forcefire -- /path/to/retail [collapse-save.bin] [map-file] [game-speed]
//! The optional map file supports an unchanged Hills payload exposed as a
//! loose `.yrm` in the ordinary chooser. Its name is retained by save validation.
//! Optional game-speed 0..6 uses the ordinary first-frame command; 6 leaves
//! more wall-clock time to capture live debris after restoring in the app.
//! Native scalar comparisons: tools/spatial_oracle/bridge_damage_admission.py.
//! Continues 200 frames after collapse through debris flight and expiration.
//! This uses the production headless loader/runtime; it is not rendered parity
//! or a native whole-frame comparison. Validated save/load continuation is
//! covered by the ignored `bridge_live_chain_tests` retail test (restore APIs
//! deliberately remain private to the simulation/application owners).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;
use vera20k::sim::command::{Command, CommandEnvelope};

fn main() {
    let retail = std::env::args().nth(1).expect("retail installation path");
    let snapshot_path = std::env::args().nth(2);
    let map_file = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "Hills.mmx".into());
    let game_speed = std::env::args().nth(4).map(|value| {
        let speed: u8 = value.parse().expect("game-speed must be 0..6");
        assert!(speed <= 6, "game-speed must be 0..6");
        speed
    });
    let mut scenario = vera20k::headless_scenario::load(Path::new(&retail), &map_file, 0x0B21_D6E5)
        .expect("load retail Hills through the production loader");
    let owner = scenario
        .sim()
        .session
        .current_house
        .expect("current launch house");
    let owner_name = scenario.sim().interner.resolve(owner).to_owned();
    let map_hash = scenario.map.ini.content_hash();
    let target = (64, 69);
    let runtime = &mut scenario.runtime;
    println!(
        "Loaded {map_file}: map {map_hash:016x}, rules {:016x}, current house {owner_name}; requested game speed {game_speed:?}",
        runtime.resources.rules.simulation_config_hash()
    );
    assert_eq!(runtime.resources.rules.bridge_rules.strength, 1500);
    assert!(
        runtime
            .simulation
            .bridge_state
            .as_ref()
            .unwrap()
            .is_bridge_walkable(target.0, target.1)
    );
    let attacker = runtime
        .simulation
        .spawn_object(
            "MTNK",
            &owner_name,
            64,
            72,
            0,
            &runtime.resources.rules,
            &runtime.resources.height_map,
        )
        .expect("spawn Grizzly on the retail bank");
    runtime
        .simulation
        .resolve_type_handles(&runtime.resources.rules);
    let mut seen = BTreeSet::new();
    let mut prior = BTreeMap::new();
    let mut moved = false;
    let mut ended = false;
    let mut previous_bridge = None;
    let mut collapsed_at = None;
    let mut debris =
        BTreeMap::<u64, (String, vera20k::sim::anim_class::AnimWorldCoord, bool)>::new();
    let mut moving_debris = BTreeSet::new();
    let mut removed_debris = BTreeSet::new();
    let mut selected_types = BTreeSet::new();
    let mut observed_anims = runtime
        .simulation
        .anims()
        .map(|(&id, _)| id)
        .collect::<BTreeSet<_>>();
    let mut later_followups = BTreeMap::<String, usize>::new();
    let metallic_names = runtime
        .resources
        .rules
        .general
        .metallic_debris
        .iter()
        .map(|name| name.to_ascii_uppercase())
        .collect::<BTreeSet<_>>();
    assert_eq!(metallic_names.len(), 15);
    assert!(metallic_names.contains("D"));
    let mut followup_names = BTreeSet::new();
    for name in &metallic_names {
        let config = runtime
            .resources
            .rules
            .art_registry
            .anim_runtime_config(name)
            .unwrap();
        followup_names.extend(
            config
                .bounce_anim
                .iter()
                .chain(&config.expire_anim)
                .chain(&config.trailer_anim)
                .cloned(),
        );
    }
    followup_names.insert(runtime.resources.rules.general.wake.name.clone());
    followup_names.extend(
        runtime
            .resources
            .rules
            .combat_damage
            .splash_list
            .iter()
            .cloned(),
    );
    for frame in 0..18000 {
        let mut commands = if collapsed_at.is_none()
            && (frame == 0
                || runtime
                    .simulation
                    .entities()
                    .get(attacker)
                    .unwrap()
                    .attack_target
                    .is_none())
        {
            vec![CommandEnvelope::new(
                owner,
                runtime.simulation.session.tick + 1,
                Command::ForceAttackCell {
                    attacker_id: attacker,
                    target_rx: target.0,
                    target_ry: target.1,
                },
            )]
        } else {
            Vec::new()
        };
        if frame == 0
            && let Some(speed) = game_speed
        {
            commands.push(CommandEnvelope::new(
                owner,
                runtime.simulation.session.tick + 1,
                Command::SetGameSpeed { speed },
            ));
        }
        runtime
            .advance_frame_for_tooling(&commands, vera20k::headless_scenario::SIM_TICK_MS)
            .expect("advance production frame");
        for (&id, anim) in runtime.simulation.anims() {
            let name = runtime.simulation.interner.resolve(anim.type_id);
            let fresh = observed_anims.insert(id);
            if metallic_names.contains(name) {
                let coord = runtime.simulation.anim_absolute_coord(id).unwrap();
                selected_types.insert(name.to_owned());
                match debris.get(&id) {
                    Some((_, first, bouncing)) if *bouncing && *first != coord => {
                        moving_debris.insert(id);
                    }
                    None => {
                        println!(
                            "frame {frame}: debris {id} {name} at {coord:?}, bouncer {}",
                            anim.bounce.is_some()
                        );
                        debris.insert(id, (name.to_owned(), coord, anim.bounce.is_some()));
                    }
                    _ => {}
                }
            } else if fresh && collapsed_at.is_some() && followup_names.contains(name) {
                *later_followups.entry(name.to_owned()).or_default() += 1;
            }
        }
        for (&id, (name, _, _)) in &debris {
            if runtime.simulation.anim(id).is_none() && removed_debris.insert(id) {
                println!("frame {frame}: debris {id} {name} completed its lifecycle");
            }
        }
        for (&id, &position) in &prior {
            match runtime.simulation.projectiles.get(id) {
                Some(shell) => moved |= shell.position != position,
                None => ended = true,
            }
        }
        prior.clear();
        for (&id, shell) in runtime
            .simulation
            .projectiles
            .iter()
            .filter(|(_, shell)| shell.source_id == attacker)
        {
            assert_eq!(shell.launch_target.z, 1040, "retail deck aim");
            seen.insert(id);
            prior.insert(id, shell.position);
        }
        let bridge = runtime
            .simulation
            .bridge_state
            .as_ref()
            .unwrap()
            .cell(target.0, target.1)
            .unwrap();
        let current_bridge = bridge.damage_state;
        if previous_bridge != Some(current_bridge) || frame % 2000 == 0 {
            println!(
                "frame {frame}: {} launched shells; bridge {current_bridge:?}",
                seen.len()
            );
        }
        previous_bridge = Some(current_bridge);
        if collapsed_at.is_none()
            && !runtime
                .simulation
                .bridge_state
                .as_ref()
                .unwrap()
                .is_bridge_walkable(target.0, target.1)
        {
            assert!(moved && ended && !seen.is_empty());
            assert!(
                runtime
                    .simulation
                    .entities()
                    .get(attacker)
                    .unwrap()
                    .attack_target
                    .is_none()
            );
            println!(
                "{map_file}: {target:?} collapsed at frame {frame}; {} Cannon shells, flight and target release observed",
                seen.len()
            );
            if let Some(path) = snapshot_path.as_deref() {
                let bytes = vera20k::sim::snapshot::GameSnapshot::save_validated(
                    &runtime.simulation,
                    map_hash,
                    runtime.resources.rules.simulation_config_hash(),
                    "Bridge debris at collapse - Hills",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("wall clock after Unix epoch")
                        .as_secs(),
                );
                // Tooling output uses the production envelope; no existing
                // player save may be overwritten. Load into the same Hills
                // scenario through the ordinary pause menu for visual checks.
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .expect("create a new collapse snapshot");
                file.write_all(&bytes).expect("write collapse snapshot");
                println!("Saved production collapse snapshot to {path}");
            }
            collapsed_at = Some(frame);
        }
        if collapsed_at.is_some_and(|collapse| frame == collapse + 200) {
            assert!(
                !debris.is_empty(),
                "collapse constructed no metallic debris"
            );
            assert!(
                !moving_debris.is_empty(),
                "no Bouncer changed its exact world coordinate"
            );
            assert!(
                moving_debris.iter().all(|id| removed_debris.contains(id)),
                "an observed flying chunk survived the 200-frame continuation"
            );
            assert!(
                !later_followups.is_empty(),
                "no post-collapse trailer/landing animation appeared"
            );
            println!(
                "200-frame continuation: {} debris, {} flew, {} removed; selected types {selected_types:?}; later follow-up types {later_followups:?}; final state hash {:016x}",
                debris.len(),
                moving_debris.len(),
                removed_debris.len(),
                runtime.simulation.state_hash()
            );
            if selected_types.contains("D") {
                assert!(
                    debris
                        .iter()
                        .filter(|(_, (name, _, _))| name == "D")
                        .all(|(id, (_, _, bouncing))| !bouncing && removed_debris.contains(id))
                );
                println!("Selected D used the unread non-Bouncer lifecycle and was removed.");
            } else {
                println!(
                    "This unchanged retail seed did not select D; native and focused tests cover it."
                );
            }
            return;
        }
    }
    panic!("bridge chain did not finish within 18000 ordinary frames");
}
