//! The existing total-only score model through native House save/load values.
//! Native field retention is compared; per-house kill-table aggregation and
//! the score model's harvested/kill split are not newly certified by this test.

use super::*;
use crate::sim::game_entity::GameEntity;
use crate::sim::rng::SimRng;
use crate::sim::snapshot::GameSnapshot;
use crate::sim::world::Simulation;
use serde_json::Value;

fn native_totals(fields: &Value) -> MatchStatistics {
    let count = |field: &Value| u32::try_from(field.as_i64().unwrap()).unwrap();
    let table_total = |name: &str| fields[name].as_array().unwrap().iter().map(count).sum();
    MatchStatistics {
        units_killed: table_total("units_killed"),
        buildings_killed: table_total("buildings_killed"),
        units_lost: count(&fields["units_lost"]),
        buildings_lost: count(&fields["buildings_lost"]),
        built: fields["built"]
            .as_array()
            .unwrap()
            .iter()
            .map(|counter| count(&counter["total"]))
            .sum(),
        // These fixtures have no harvested credits, so the existing split
        // represents the native combined score with this one signed value.
        score_points: i32::try_from(fields["score"].as_i64().unwrap()).unwrap(),
    }
}

#[test]
fn native_house_statistics_survive_snapshot_and_retained_ship_terminal_record() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../tools/spatial_oracle/naval_house_stats.json"
    ))
    .unwrap();
    let rows = corpus["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let mut sim = Simulation::new();
        sim.session.map_name = "HOUSE-STATS.MAP".to_owned();
        // Whole Scenario load has its own seed-zero contract. The native
        // comparison here establishes the House fields across its Save/Load.
        sim.scenario_rng = SimRng::new(0);
        let owner = sim.interner.intern("Americans");
        let mut house = HouseState::new(owner, 0, None, false, 0, 10);
        house.stats = native_totals(&row["before"]);
        sim.houses.insert(owner, house);
        sim.session.house_order.push(owner);
        let id = sim.allocate_stable_id();
        let mut hull = GameEntity::test_default(id, "AEGIS", "Americans", 113, 59);
        hull.type_ref = sim.interner.intern("AEGIS");
        hull.owner = owner;
        hull.category = crate::map::entities::EntityCategory::Unit;
        hull.health.current = 1;
        sim.substrate.entities.insert(hull);
        let bytes = GameSnapshot::save_validated(&sim, 1, 2, "native House state", 3);
        let mut restored = GameSnapshot::load_validated(&bytes, 1, 2, "HOUSE-STATS.MAP")
            .unwrap()
            .sim;
        restored.retain_in_scenario_process_state_from(&sim);
        restored.restore_after_snapshot_load().unwrap();
        assert_eq!(restored.houses[&owner].stats, native_totals(&row["after"]));
        assert_eq!(restored.state_hash(), sim.state_hash());
        assert_eq!(restored.rng_state(), sim.rng_state());
        if let Some(expected) = row["terminal_record_loss"].as_u64() {
            let rng = restored.rng_state();
            restored.record_sinking_terminal_kill(id);
            assert_eq!(
                u64::from(restored.houses[&owner].stats.units_lost),
                expected
            );
            assert_eq!(restored.rng_state(), rng);
        }
    }
}

#[test]
fn saved_live_statistics_preserve_terminal_score_and_rng_continuation() {
    let mut sim = Simulation::new();
    sim.session.map_name = "SCORE.MAP".to_owned();
    sim.scenario_rng = SimRng::new(0);
    let owner = sim.interner.intern("Americans");
    let mut house = HouseState::new(owner, 0, None, false, 0, 10);
    house.stats = MatchStatistics {
        units_killed: 2,
        buildings_killed: 1,
        units_lost: 1,
        buildings_lost: 3,
        built: 4,
        score_points: 700,
    };
    sim.houses.insert(owner, house);
    sim.session.house_order.push(owner);
    let bytes = GameSnapshot::save_validated(&sim, 1, 2, "before score edge", 3);
    let mut restored = GameSnapshot::load_validated(&bytes, 1, 2, "SCORE.MAP")
        .unwrap()
        .sim;
    restored.retain_in_scenario_process_state_from(&sim);
    restored.restore_after_snapshot_load().unwrap();
    let before_rng = sim.rng_state();
    assert!(sim.finalize_terminal_score_snapshot());
    assert!(restored.finalize_terminal_score_snapshot());
    assert_ne!(
        sim.rng_state(),
        before_rng,
        "positive score consumes its existing bonus draw"
    );
    assert_eq!(
        restored.terminal_score_snapshot(),
        sim.terminal_score_snapshot()
    );
    assert_eq!(restored.rng_state(), sim.rng_state());
    assert_eq!(restored.state_hash(), sim.state_hash());
}
