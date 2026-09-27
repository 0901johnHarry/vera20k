//! Live host of Apply_area_damage's bridge admission and callback order.

use super::*;
use crate::map::cell_index::NativeCellIdentity;
use crate::sim::bridge_state::damage_dispatch::{self, CellFields, DamageHost};
use crate::sim::bridge_state::ramp_repair::Family;

struct LiveDamage<'a> {
    sim: &'a mut Simulation,
    event: &'a BridgeDamageEvent,
    strength: i32,
    publication: Option<(
        &'a RuleSet,
        Option<&'a crate::map::overlay_types::OverlayTypeRegistry>,
    )>,
    outcomes: Vec<StateOutcome>,
    collapsed: bool,
}

impl LiveDamage<'_> {
    fn terrain(&self) -> &ResolvedTerrainGrid {
        self.sim
            .resolved_terrain
            .as_ref()
            .expect("bridge damage terrain")
    }
}

impl DamageHost for LiveDamage<'_> {
    type Cell = NativeCellIdentity;

    fn fields(&self, cell: Self::Cell) -> CellFields {
        let terrain = self.terrain();
        // Ordinary overlay writes are live CellClass authority. Structural
        // head/state-machine writers still publish their retained runtime state
        // through the existing adapter after returning.
        let overlay = match cell {
            NativeCellIdentity::Real(index) => {
                let cell = &terrain.cells()[index];
                let overlay = cell.bridge_facts.overlay_id;
                if overlay.is_some_and(|overlay| {
                    crate::sim::bridge_state::ordinary::member(i32::from(overlay), Family::Low)
                        || crate::sim::bridge_state::ordinary::member(
                            i32::from(overlay),
                            Family::High,
                        )
                }) {
                    overlay
                } else {
                    self.sim
                        .bridge_state
                        .as_ref()
                        .and_then(|state| state.cell(cell.rx, cell.ry))
                        .map(|cell| cell.overlay_byte)
                        .or(overlay)
                }
            }
            NativeCellIdentity::Dummy => terrain.shared_cell_dummy().overlay_fields().0,
        };
        CellFields {
            flags: terrain.native_cell_flags(cell),
            tile: terrain.native_cell_tile_index(cell),
            overlay: overlay.filter(|value| *value != 0xff).map_or(-1, i32::from),
            level: terrain.native_cell_ground_fields(cell).0,
        }
    }

    fn resolve_anchor(&mut self, cell: Self::Cell) -> Option<Self::Cell> {
        let terrain = self.terrain();
        let retained = if terrain.native_cell_flags(cell) & BRIDGE_FLAG_ANCHOR_SELF != 0 {
            cell
        } else {
            terrain.native_cell_anchor(cell)?
        };
        Some(terrain.native_cell_identity(terrain.native_cell_coord(retained)))
    }

    fn tile_bases(&self) -> [i32; 2] {
        let terrain = self.terrain();
        [
            terrain
                .high_bridge_rim_tiles()
                .map_or_else(|| terrain.concrete_bridge_set_base(), |tiles| tiles.base),
            terrain.wood_bridge_set_base(),
        ]
    }

    fn middle_tiles(&self) -> Option<[i32; 2]> {
        self.terrain()
            .high_bridge_rim_tiles()
            .map(|tiles| tiles.middle)
    }

    fn roll_strength(&mut self) -> i32 {
        self.sim
            .scenario_rng
            .next_range_i32_inclusive(1, self.strength)
    }

    fn apply(&mut self, path: DispatchPath) -> bool {
        let input = (self.event.rx as i16, self.event.ry as i16);
        let path = if path.is_state_machine() {
            let cell = self.terrain().native_cell_identity(input);
            let Some(selected) = damage_dispatch::select_driver(self, cell) else {
                return false;
            };
            selected
        } else {
            path
        };
        if matches!(path, DispatchPath::LowDirect | DispatchPath::HighDirect)
            && let Some((rules, registry)) = self.publication
        {
            let family = if path == DispatchPath::LowDirect {
                Family::Low
            } else {
                Family::High
            };
            let result =
                live_publication::damage_ordinary(self.sim, rules, registry, input, family)
                    .unwrap_or_else(|error| {
                        panic!("ordinary {family:?} bridge publication at {input:?}: {error}")
                    });
            self.collapsed |= result.collapsed;
            return result.returned;
        }
        if matches!(path, DispatchPath::HighStateMachine)
            && let Some((rules, registry)) = self.publication
            && let Some(result) = live_publication::try_body(self.sim, rules, registry, input)
        {
            self.collapsed |= result.collapsed;
            return result.returned;
        }
        let outcome = {
            let terrain = self
                .sim
                .resolved_terrain
                .as_mut()
                .expect("bridge damage terrain");
            let state = self.sim.bridge_state.as_mut().expect("bridge damage state");
            match path {
                DispatchPath::HighStateMachine => {
                    state.advance_damage_state(self.event.rx, self.event.ry, true, terrain)
                }
                DispatchPath::LowStateMachine => {
                    state.advance_damage_state(self.event.rx, self.event.ry, false, terrain)
                }
                DispatchPath::LowDirect | DispatchPath::HighDirect => {
                    panic!("ordinary bridge damage requires the live rules/publication context")
                }
            }
        };
        let success = outcome.apply_damage_success();
        if outcome.has_effect() {
            apply_runtime_bridge_flag_transcript_from_outcome(self.sim, &outcome);
            self.outcomes.push(outcome);
        }
        success
    }

    fn detach(&mut self, cell: Self::Cell) {
        let NativeCellIdentity::Real(_) = cell else {
            return;
        };
        let (rx, ry) = self.terrain().native_cell_coord(cell);
        self.sim.stop_all_targeting_cell(
            rx as u16,
            ry as u16,
            self.publication.map(|(rules, _)| rules),
        );
    }

    fn dirty(&mut self, _path: DispatchPath) {
        // Tactical presentation rebuilds a complete frame; this cell marks the
        // same native damage attempt even when its driver returns false.
        self.sim
            .tactical_dirty_cells
            .push((self.event.rx, self.event.ry));
    }
}

pub(super) fn run(
    sim: &mut Simulation,
    events: &[BridgeDamageEvent],
    strength: i32,
    publication: Option<(
        &RuleSet,
        Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    )>,
) -> (Vec<StateOutcome>, bool) {
    if sim.resolved_terrain.is_none() || sim.bridge_state.is_none() {
        return (Vec::new(), false);
    }
    let mut outcomes = Vec::new();
    let mut collapsed = false;
    for event in events {
        let cell = sim
            .resolved_terrain
            .as_ref()
            .unwrap()
            .native_cell_identity((event.rx as i16, event.ry as i16));
        let mut host = LiveDamage {
            sim,
            event,
            strength,
            publication,
            outcomes: Vec::new(),
            collapsed: false,
        };
        damage_dispatch::dispatch(
            &mut host,
            cell,
            event.damage,
            event.impact_z_leptons,
            event.is_ion_cannon,
        );
        collapsed |= host.collapsed;
        outcomes.extend(host.outcomes);
    }
    (outcomes, collapsed)
}
