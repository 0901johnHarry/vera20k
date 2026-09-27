//! Zone-aware pathfinding — zone connectivity for fast unreachability detection
//! and hierarchical search-space reduction.
//!
//! ## What gamemd does
//!
//! `AStar_pathfind_search` @ `0x0042C900` is the single entrypoint, and the
//! hierarchical/regular split is one boolean it computes for itself:
//!
//! 1. Source zone from `MapClass::GetZoneID` with the mover's `Foot+0x8C`
//!    on-bridge flag; destination zone with the goal cell's `Flags & 0x100`.
//! 2. `MapClass::ResolvePathCoord_BridgeAware` projects both endpoints to
//!    logical coordinates. The projection feeds `Zone_precheck`, the playfield
//!    test below and the three failure logs — but **not** the search:
//!    `AStar_main_loop` @ `0x00429A90` receives the raw endpoints.
//! 3. `allowHS` = `GetTechnoType()->[+0xC94] == 0` **and** `mover->[+0x3D5] != 0`
//!    **and** `[mover vtable+0x320]()` (`0x004DA1D0`) `== 0` **and** both
//!    projected endpoints pass `MapClass::Is_Cell_In_Playfield(cell, 1)`
//!    (`0x0042CAC2` through `0x0042CB22`).
//! 4. **Zones equal** → if `allowHS`, run `Zone_precheck` @ `0x0042C290`; on
//!    failure log "Hierarchical findpath failure", clear `allowHS`, and run the
//!    A* anyway. **Zones differ** → if `allowHS`, return 0 with no A* at all;
//!    otherwise fall through to the A*.
//! 5. `AStar_main_loop` receives `allowHS` as its last argument, and it **is** a
//!    corridor filter: `Zone_precheck` stamps every zone on the coarse route
//!    into the level-0 array at `PathfinderClass+0x40` with the search serial
//!    from `+0x28`, and the neighbour loop at `0x00429EB1` skips any cell whose
//!    zone is unstamped unless the cell itself carries `CellClass+0x122`.
//!    [`super::core::HierarchyGate`] is that rule, with `BlockerNeighborCounts`
//!    standing in for `+0x122`. There is a **second exemption**: the whole skip
//!    is reached only when the layer byte at `[ESP+0x60]` is non-zero, and
//!    `0x00429E54`-`0x00429E7A` computes that byte as `1` *unless* the
//!    neighbour carries `Flags & 0x100` **and**
//!    `|PathfinderClass+0x30 − cell[+0x11B]| > 1`. A bridge-layer neighbour at a
//!    differing height therefore jumps from `0x00429EAF` straight to
//!    `0x00429F04` and is expanded regardless of the corridor stamp, the
//!    `+0x122` byte and `allowHS`.
//! 6. Retry budget is `param_6 != -1 ? 1 : 5`. Each retry calls
//!    `PathfinderClass::UpdateHierarchicalEdges` @ `0x0042CCD0`, re-reads
//!    `allowHS` from `PathfinderClass+0x38`, and re-runs `Zone_precheck`;
//!    a failed precheck ends the loop.
//!
//! ## What VERA does, and the remaining recorded gaps
//!
//! The live route is faithful in shape: `zone_precheck_flat` on the level-0
//! hierarchy, then a hierarchy-marked cell A*; a precheck failure with matching
//! zones falls back to the plain A*, and one with differing zones returns
//! `None`. What is missing:
//!
//! - **Two uncommon `allowHS` terms remain unmodelled.** VERA now threads the
//!   canonical `TechnoClass+0x3D5` byte and applies exact bridge-resolved mode-1
//!   endpoint membership before admitting hierarchy. It still does not model
//!   `IsTrain=` (the INI key
//!   behind `TechnoTypeClass+0xC94`, string `0x008444BC`, stored at
//!   `0x00712284` — absent from stock `rulesmd.ini`, so this term never fires);
//!   the `0x004DA1D0` predicate, which — given the other terms already hold at
//!   the call site — reduces to
//!   `mover+0x3D4 != 0` **or** current-or-queued mission == Retreat(4) **or**
//!   (`mover+0x5D4` non-null and `FUN_006EC300`). Player effect: for such a
//!   mover gamemd runs an unrestricted A*
//!   and can return a route where VERA answers "unreachable" from the zone map,
//!   so the unit refuses an order retail accepts. Frequency: a unit on Retreat
//!   or linked to the remaining team predicate — uncommon in ordinary
//!   skirmish, but not zero. Team6EC300 can perform mode1 waypoint lookups
//!   before endpoint membership, changing shared dummy state. Its Team+7F and
//!   action3 authority/order remain open; the current bool is supplied externally.
//! - **Without blocker counts there is no hierarchy search.** Every
//!   production search supplies blocker counts (`movement_tick`,
//!   `world_commands`, the miner system, the production queue), and every
//!   `ZoneGrid` carries levels 0/1/2. A caller without counts gets the plain
//!   cell A* once the zone test passes, where gamemd would run the
//!   hierarchy-marked search. No live trigger is established here.
//! - **gamemd's precheck retries are not modelled.** Its budget is
//!   `param_6 != -1 ? 1 : 5`, each retry re-running `UpdateHierarchicalEdges`
//!   and `Zone_precheck` (point 6); VERA runs one precheck.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/zone_map, sim/pathfinding, sim/locomotor.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeSet;

use super::{BlockerNeighborCounts, LayeredEntityBlockMap, MoverSearchFacts, SearchMarkerOverlay};

use super::terrain_cost::TerrainCostGrid;
use super::zone_hierarchy::{ZonePrecheckExclusions, ZonePrecheckOutcome, zone_precheck_flat};
use super::zone_map::ZoneGrid;
use super::{
    LayeredPathStep, PathGrid, find_layered_path_hierarchy_marker, find_layered_path_marker,
    find_path_with_costs_hierarchy_marker, find_path_with_costs_marker,
};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::tube_facts::TubeSource;
use crate::rules::locomotor_type::MovementZone;
use crate::sim::cell_rect::{PlayfieldBounds, cell_is_in_playfield_height_aware};
use crate::sim::movement::locomotor::MovementLayer;

/// Provenance of a returned path failure. The Foot wrapper must not turn
/// unavailable caches or compatibility-only rejection into a native core NULL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathSearchFailure {
    ///42CB22 rejected unequal native raw labels before cell A*.
    NativeEntryRejected,
    ///A required hierarchy cell had no represented native topology.
    MissingHierarchyCell,
    ///The reduced compatibility graph rejected without native raw evidence.
    CompatibilityZoneRejected,
    ///The existing cell search ran and returned no route.
    CellSearchExhausted,
}

/// Whether the path entry may answer reachability from the reduced per-row zone
/// map before running A*.
///
/// Gamemd gates every row: `Can_Reach_Zone` short-circuits to "reachable" only on
/// `mzRow == -1`, and the A*-entry precheck reads whatever row the type's
/// `MovementZone=` gives. Stock rulesmd puts every main battle tank in
/// `Destroyer`, every ore miner in `Crusher` and the Battle Fortress in
/// `CrusherAll`, so excluding those rows bypassed the gate for the majority of
/// all path searches in a match.
///
/// `Water` / `WaterBeach` remain excluded — VERA-internal, gamemd equivalent
/// UNCHECKED. The terrain-aware zone builder's water/beach surface legality is
/// still coarser than the runtime water-surface predicate, so hard-gating naval
/// movers here would refuse orders gamemd accepts. Remove the exception once the
/// naval surface classes are pinned.
fn can_use_reduced_zone_precheck(movement_zone: Option<MovementZone>) -> bool {
    match movement_zone {
        None => true,
        // `mzRow == -1` short-circuits to "reachable" in the engine, so the
        // reduced zone gate must NOT be allowed to refuse the search.
        Some(MovementZone::Invalid) => false,
        Some(MovementZone::Water | MovementZone::WaterBeach) => false,
        Some(_) => true,
    }
}

fn can_reach_same_or_zoned(
    zg: &ZoneGrid,
    mz: MovementZone,
    from: (u16, u16),
    from_layer: MovementLayer,
    to: (u16, u16),
    to_layer: MovementLayer,
) -> bool {
    from == to || zg.can_reach(mz, from, from_layer, to, to_layer)
}

fn can_reach_through_explicit_tube(
    zg: &ZoneGrid,
    mz: MovementZone,
    start: (u16, u16),
    start_layer: MovementLayer,
    goal: (u16, u16),
    resolved_terrain: Option<&ResolvedTerrainGrid>,
) -> bool {
    let Some(terrain) = resolved_terrain else {
        return false;
    };
    terrain.tube_facts().iter().any(|tube| {
        tube.source == TubeSource::ExplicitMap
            && tube.path_len() > 0
            && tube.exit != (0, 0)
            && can_reach_same_or_zoned(
                zg,
                mz,
                start,
                start_layer,
                tube.entry,
                MovementLayer::Ground,
            )
            && can_reach_same_or_zoned(
                zg,
                mz,
                tube.exit,
                MovementLayer::Ground,
                goal,
                MovementLayer::Ground,
            )
    })
}

// Original42C900 compares raw GetZoneID results, including distinct invalid
// row labels1/FFFF. Compatibility-only test/cache grids have no raw row.
#[cfg(test)]
fn native_path_zone_equality(
    zones: &ZoneGrid,
    terrain: Option<&ResolvedTerrainGrid>,
    movement: MovementZone,
    start: (u16, u16),
    start_bridge: bool,
    goal: (u16, u16),
    goal_bridge: bool,
) -> Option<bool> {
    let terrain = terrain?;
    Some(
        zones.get_path_zone_id_native(terrain, start, movement, start_bridge)?
            == zones.get_path_zone_id_native(terrain, goal, movement, goal_bridge)?,
    )
}

include!("native_path_entry.rs");

/// The flat (ground-only) zone-aware search without a marker overlay, blocker
/// counts or playfield bounds: the zone test gates a plain cell A*, and no
/// hierarchy-marked search runs without counts.
///
/// TODO(RE): terrain-aware nodeIndex connectivity can still be a little looser than
/// final movement legality because the recovered node flood-fill is 8-neighbor while
/// the actual step predicate also applies tighter per-move checks. Treat zone gating
/// here as a best-effort reject, not closed parity.
#[cfg(test)]
pub fn find_path_zoned(
    grid: &PathGrid,
    start: (u16, u16),
    goal: (u16, u16),
    costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    urgency: u8,
    mover_is_crusher: bool,
    is_infantry: bool,
) -> Option<Vec<(u16, u16)>> {
    find_path_zoned_marker(
        grid,
        start,
        goal,
        costs,
        entity_blocks,
        zone_grid,
        mz,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        None,
        None,
        urgency,
        mover_is_crusher,
        is_infantry,
        true,
        None,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn find_path_zoned_marker(
    grid: &PathGrid,
    start: (u16, u16),
    goal: (u16, u16),
    costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    marker_overlay: Option<&SearchMarkerOverlay>,
    blocker_neighbor_counts: Option<&BlockerNeighborCounts>,
    urgency: u8,
    mover_is_crusher: bool,
    is_infantry: bool,
    allow_zone_hierarchy: bool,
    playfield_bounds: Option<PlayfieldBounds>,
) -> Option<Vec<(u16, u16)>> {
    find_path_zoned_marker_detailed(
        grid,
        start,
        goal,
        costs,
        entity_blocks,
        zone_grid,
        mz,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        blocker_neighbor_counts,
        MoverSearchFacts {
            urgency,
            mover_is_crusher,
            is_infantry,
            wall_cost: None,
        },
        allow_zone_hierarchy,
        playfield_bounds,
    )
    .ok()
}

pub(crate) fn find_path_zoned_marker_detailed(
    grid: &PathGrid,
    start: (u16, u16),
    goal: (u16, u16),
    costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    marker_overlay: Option<&SearchMarkerOverlay>,
    blocker_neighbor_counts: Option<&BlockerNeighborCounts>,
    facts: MoverSearchFacts<'_>,
    allow_zone_hierarchy: bool,
    playfield_bounds: Option<PlayfieldBounds>,
) -> Result<Vec<(u16, u16)>, PathSearchFailure> {
    let entry = prepare_native_path_entry(
        zone_grid,
        resolved_terrain,
        movement_zone.unwrap_or(mz),
        start,
        false,
        goal,
        allow_zone_hierarchy,
        playfield_bounds,
    );
    find_path_zoned_marker_inner_detailed(
        grid,
        start,
        goal,
        entry.hierarchy_start,
        entry.hierarchy_goal,
        entry.raw_equal,
        costs,
        entity_blocks,
        if entry.endpoints_in_playfield {
            zone_grid
        } else {
            None
        },
        mz,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        facts,
        blocker_neighbor_counts,
    )
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
fn find_path_zoned_marker_inner(
    grid: &PathGrid,
    start: (u16, u16),
    goal: (u16, u16),
    hierarchy_start: (u16, u16),
    hierarchy_goal: (u16, u16),
    native_zone_equal: Option<bool>,
    costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    marker_overlay: Option<&SearchMarkerOverlay>,
    urgency: u8,
    mover_is_crusher: bool,
    is_infantry: bool,
    blocker_neighbor_counts: Option<&BlockerNeighborCounts>,
) -> Option<Vec<(u16, u16)>> {
    find_path_zoned_marker_inner_detailed(
        grid,
        start,
        goal,
        hierarchy_start,
        hierarchy_goal,
        native_zone_equal,
        costs,
        entity_blocks,
        zone_grid,
        mz,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        MoverSearchFacts {
            urgency,
            mover_is_crusher,
            is_infantry,
            wall_cost: None,
        },
        blocker_neighbor_counts,
    )
    .ok()
}

fn find_path_zoned_marker_inner_detailed(
    grid: &PathGrid,
    start: (u16, u16),
    goal: (u16, u16),
    hierarchy_start: (u16, u16),
    hierarchy_goal: (u16, u16),
    native_zone_equal: Option<bool>,
    costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    marker_overlay: Option<&SearchMarkerOverlay>,
    facts: MoverSearchFacts<'_>,
    blocker_neighbor_counts: Option<&BlockerNeighborCounts>,
) -> Result<Vec<(u16, u16)>, PathSearchFailure> {
    if !can_use_reduced_zone_precheck(movement_zone) {
        return find_path_with_costs_marker(
            grid,
            start,
            goal,
            costs,
            entity_blocks,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            facts,
        )
        .ok_or(PathSearchFailure::CellSearchExhausted);
    }

    let Some(zg) = zone_grid else {
        return find_path_with_costs_marker(
            grid,
            start,
            goal,
            costs,
            entity_blocks,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            facts,
        )
        .ok_or(PathSearchFailure::CellSearchExhausted);
    };

    let Some(zone_map) = zg.map_for(mz) else {
        return find_path_with_costs_marker(
            grid,
            start,
            goal,
            costs,
            entity_blocks,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            facts,
        )
        .ok_or(PathSearchFailure::CellSearchExhausted);
    };
    let start_zone = zone_map.zone_at(start.0, start.1, MovementLayer::Ground);
    let goal_bridge = resolved_terrain
        .and_then(|terrain| terrain.cell(goal.0, goal.1))
        .is_some_and(|cell| cell.bridge_facts.has_structural_bridge());
    let goal_zone = zone_map.zone_at(
        goal.0,
        goal.1,
        if goal_bridge {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        },
    );
    let zones_match = native_zone_equal.unwrap_or(start_zone == goal_zone);

    let hierarchy_counts_available = blocker_neighbor_counts.is_some();
    if hierarchy_counts_available
        && let Some(hierarchy) = zg.hierarchy_for(mz)
        && let Some(level0_zones) = hierarchy.level(0)
    {
        //42CB22..42CB3F rejects unequal native base labels before precheck.
        if !zones_match {
            return Err(if native_zone_equal.is_some() {
                PathSearchFailure::NativeEntryRejected
            } else {
                PathSearchFailure::CompatibilityZoneRejected
            });
        }
        let hierarchy_start_zone = zg
            .hierarchy_zone_at_native(0, hierarchy_start)
            .ok_or(PathSearchFailure::MissingHierarchyCell)?;
        let hierarchy_goal_zone = zg
            .hierarchy_zone_at_native(0, hierarchy_goal)
            .ok_or(PathSearchFailure::MissingHierarchyCell)?;
        match zone_precheck_flat(
            hierarchy,
            hierarchy_start_zone,
            hierarchy_goal_zone,
            movement_zone.unwrap_or(mz),
            &ZonePrecheckExclusions::default(),
        ) {
            ZonePrecheckOutcome::Passed(result) => {
                return find_path_with_costs_hierarchy_marker(
                    grid,
                    start,
                    goal,
                    costs,
                    entity_blocks,
                    level0_zones,
                    &result.marked[0],
                    blocker_neighbor_counts.expect("checked above"),
                    movement_zone,
                    resolved_terrain,
                    entity_block_map,
                    marker_overlay,
                    facts,
                )
                .ok_or(PathSearchFailure::CellSearchExhausted);
            }
            ZonePrecheckOutcome::Failed if zones_match => {
                return find_path_with_costs_marker(
                    grid,
                    start,
                    goal,
                    costs,
                    entity_blocks,
                    movement_zone,
                    resolved_terrain,
                    entity_block_map,
                    marker_overlay,
                    facts,
                )
                .ok_or(PathSearchFailure::CellSearchExhausted);
            }
            ZonePrecheckOutcome::Failed => {
                return Err(PathSearchFailure::CompatibilityZoneRejected);
            }
        }
    }

    let zone_precheck_passed = zg.can_reach(
        mz,
        start,
        MovementLayer::Ground,
        goal,
        MovementLayer::Ground,
    );

    // Same-zone precheck failures disable hierarchy and still run cell A*.
    if !zone_precheck_passed && zones_match {
        return find_path_with_costs_marker(
            grid,
            start,
            goal,
            costs,
            entity_blocks,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            facts,
        )
        .ok_or(PathSearchFailure::CellSearchExhausted);
    }

    // Cross-zone precheck failure aborts without cell A*.
    if !zone_precheck_passed {
        if can_reach_through_explicit_tube(
            zg,
            mz,
            start,
            MovementLayer::Ground,
            goal,
            resolved_terrain,
        ) {
            return find_path_with_costs_marker(
                grid,
                start,
                goal,
                costs,
                entity_blocks,
                movement_zone,
                resolved_terrain,
                entity_block_map,
                marker_overlay,
                facts,
            )
            .ok_or(PathSearchFailure::CellSearchExhausted);
        }
        log::trace!(
            "zone_search: unreachable {:?} ({:?}→{:?}), skipping A*",
            mz,
            start,
            goal,
        );
        return Err(PathSearchFailure::CompatibilityZoneRejected);
    }

    find_path_with_costs_marker(
        grid,
        start,
        goal,
        costs,
        entity_blocks,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        facts,
    )
    .ok_or(PathSearchFailure::CellSearchExhausted)
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn find_layered_path_zoned_marker(
    grid: &PathGrid,
    ground_blocks: Option<&BTreeSet<(u16, u16)>>,
    bridge_blocks: Option<&BTreeSet<(u16, u16)>>,
    start: (u16, u16),
    start_layer: MovementLayer,
    goal: (u16, u16),
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    terrain_costs: Option<&TerrainCostGrid>,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    marker_overlay: Option<&SearchMarkerOverlay>,
    blocker_neighbor_counts: Option<&BlockerNeighborCounts>,
    urgency: u8,
    mover_is_crusher: bool,
    is_infantry: bool,
    allow_zone_hierarchy: bool,
    playfield_bounds: Option<PlayfieldBounds>,
) -> Option<Vec<LayeredPathStep>> {
    find_layered_path_zoned_marker_detailed(
        grid,
        ground_blocks,
        bridge_blocks,
        start,
        start_layer,
        goal,
        zone_grid,
        mz,
        terrain_costs,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        blocker_neighbor_counts,
        MoverSearchFacts {
            urgency,
            mover_is_crusher,
            is_infantry,
            wall_cost: None,
        },
        allow_zone_hierarchy,
        playfield_bounds,
    )
    .ok()
}

pub(crate) fn find_layered_path_zoned_marker_detailed(
    grid: &PathGrid,
    ground_blocks: Option<&BTreeSet<(u16, u16)>>,
    bridge_blocks: Option<&BTreeSet<(u16, u16)>>,
    start: (u16, u16),
    start_layer: MovementLayer,
    goal: (u16, u16),
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    terrain_costs: Option<&TerrainCostGrid>,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    marker_overlay: Option<&SearchMarkerOverlay>,
    blocker_neighbor_counts: Option<&BlockerNeighborCounts>,
    facts: MoverSearchFacts<'_>,
    allow_zone_hierarchy: bool,
    playfield_bounds: Option<PlayfieldBounds>,
) -> Result<Vec<LayeredPathStep>, PathSearchFailure> {
    // `AStar @ 0x0042CAD6` admits hierarchy only while the mover's stored
    // TechnoClass+0x3D5 byte is true. False is not a hard failure: it bypasses
    // zone/hierarchy admission and runs the ordinary flat/layered cell A*.
    let source_layer = if start_layer == MovementLayer::Bridge {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    };
    let entry = prepare_native_path_entry(
        zone_grid,
        resolved_terrain,
        movement_zone.unwrap_or(mz),
        start,
        source_layer == MovementLayer::Bridge,
        goal,
        allow_zone_hierarchy,
        playfield_bounds,
    );
    let goal_layer = if entry.goal_bridge {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    };
    let hierarchy_start = entry.hierarchy_start;
    let hierarchy_goal = entry.hierarchy_goal;
    let zone_grid = if entry.endpoints_in_playfield {
        zone_grid
    } else {
        None
    };
    if !can_use_reduced_zone_precheck(movement_zone) {
        return find_layered_path_marker(
            grid,
            ground_blocks,
            bridge_blocks,
            start,
            start_layer,
            goal,
            terrain_costs,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            facts,
        )
        .ok_or(PathSearchFailure::CellSearchExhausted);
    }

    if let Some(zg) = zone_grid {
        // Raw results were captured before retained-cell projection/playfield.
        let zones_match = entry.raw_equal.unwrap_or_else(|| {
            zg.map_for(mz).is_some_and(|zone_map| {
                zone_map.zone_at(start.0, start.1, source_layer)
                    == zone_map.zone_at(goal.0, goal.1, goal_layer)
            })
        });

        if blocker_neighbor_counts.is_some()
            && resolved_terrain.is_some()
            && let Some(hierarchy) = zg.hierarchy_for(mz)
            && let Some(level0_zones) = hierarchy.level(0)
        {
            if !zones_match {
                return Err(if entry.raw_equal.is_some() {
                    PathSearchFailure::NativeEntryRejected
                } else {
                    PathSearchFailure::CompatibilityZoneRejected
                });
            }
            match zone_precheck_flat(
                hierarchy,
                zg.hierarchy_zone_at_native(0, hierarchy_start)
                    .ok_or(PathSearchFailure::MissingHierarchyCell)?,
                zg.hierarchy_zone_at_native(0, hierarchy_goal)
                    .ok_or(PathSearchFailure::MissingHierarchyCell)?,
                movement_zone.unwrap_or(mz),
                &ZonePrecheckExclusions::default(),
            ) {
                ZonePrecheckOutcome::Passed(result) => {
                    return find_layered_path_hierarchy_marker(
                        grid,
                        ground_blocks,
                        bridge_blocks,
                        start,
                        start_layer,
                        goal,
                        terrain_costs,
                        level0_zones,
                        &result.marked[0],
                        blocker_neighbor_counts.expect("checked above"),
                        movement_zone,
                        resolved_terrain,
                        entity_block_map,
                        marker_overlay,
                        facts,
                    )
                    .ok_or(PathSearchFailure::CellSearchExhausted);
                }
                ZonePrecheckOutcome::Failed if zones_match => {
                    return find_layered_path_marker(
                        grid,
                        ground_blocks,
                        bridge_blocks,
                        start,
                        start_layer,
                        goal,
                        terrain_costs,
                        movement_zone,
                        resolved_terrain,
                        entity_block_map,
                        marker_overlay,
                        facts,
                    )
                    .ok_or(PathSearchFailure::CellSearchExhausted);
                }
                ZonePrecheckOutcome::Failed => {
                    return Err(PathSearchFailure::CompatibilityZoneRejected);
                }
            }
        }

        if !zg.can_reach(mz, start, source_layer, goal, goal_layer) {
            if zones_match {
                return find_layered_path_marker(
                    grid,
                    ground_blocks,
                    bridge_blocks,
                    start,
                    start_layer,
                    goal,
                    terrain_costs,
                    movement_zone,
                    resolved_terrain,
                    entity_block_map,
                    marker_overlay,
                    facts,
                )
                .ok_or(PathSearchFailure::CellSearchExhausted);
            }
            if can_reach_through_explicit_tube(zg, mz, start, start_layer, goal, resolved_terrain) {
                return find_layered_path_marker(
                    grid,
                    ground_blocks,
                    bridge_blocks,
                    start,
                    start_layer,
                    goal,
                    terrain_costs,
                    movement_zone,
                    resolved_terrain,
                    entity_block_map,
                    marker_overlay,
                    facts,
                )
                .ok_or(PathSearchFailure::CellSearchExhausted);
            }
            log::trace!(
                "zone_search: layered unreachable {:?} ({:?} layer={:?} -> {:?}), skipping A*",
                mz,
                start,
                start_layer,
                goal,
            );
            return Err(PathSearchFailure::CompatibilityZoneRejected);
        }
    }

    find_layered_path_marker(
        grid,
        ground_blocks,
        bridge_blocks,
        start,
        start_layer,
        goal,
        terrain_costs,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        facts,
    )
    .ok_or(PathSearchFailure::CellSearchExhausted)
}

#[cfg(test)]
#[path = "zone_search_tests.rs"]
mod zone_search_tests;
