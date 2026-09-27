//! Wooden low-overlay destruction walkers and shared overlay classification.
//!
//! Ordinary repair is owned by `ordinary_repair` / `ramp_repair` and their
//! synchronous live publication host. Concrete low-overlay damage is owned by
//! `ordinary_damage`; the wooden damage callers still use the walkers here.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::sim::bridge_state::{Axis, BridgeRuntimeState, StateOutcome};

impl BridgeRuntimeState {
    pub(crate) fn is_low_destroy_overlay(overlay: u8) -> bool {
        (0x4A..=0x65).contains(&overlay)
    }

    pub(crate) fn is_high_destroy_overlay(overlay: u8) -> bool {
        (0xCD..=0xE8).contains(&overlay)
    }

    pub(crate) fn low_destroy_overlay_axis(overlay: u8) -> Option<Axis> {
        if Self::is_ns_walker_overlay_low(overlay) {
            Some(Axis::NS)
        } else if Self::is_ew_walker_overlay_low(overlay) {
            Some(Axis::EW)
        } else {
            None
        }
    }

    pub(crate) fn high_destroy_overlay_axis(overlay: u8) -> Option<Axis> {
        if Self::is_ns_walker_overlay_high(overlay) {
            Some(Axis::NS)
        } else if Self::is_ew_walker_overlay_high(overlay) {
            Some(Axis::EW)
        } else {
            None
        }
    }

    /// `DestroyBridge_Low` 0x0057BAA0. Its axis classes are
    /// NS = 0x4A..=0x52 u 0x5C..=0x5F u {0x64} and
    /// EW = 0x53..=0x5B u 0x60..=0x63 u {0x65}; their union is the band
    /// [`Self::is_low_destroy_overlay`] tests.
    ///
    /// Overlay-direct LOW walker entry. Same native shape as concrete `ordinary_damage::damage`
    /// with overlay ranges shifted to the LOW body range
    /// (`[0x4A..=0x65]`).
    pub fn destroy_bridge_low(
        &mut self,
        rx: u16,
        ry: u16,
        terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        let Some(cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };
        let overlay = cell.overlay_byte;
        if !Self::is_low_destroy_overlay(overlay) {
            return StateOutcome::NoChange;
        }
        if Self::is_ns_walker_overlay_low(overlay) {
            let (sx, sy) = self.find_walker_start_low_ns(rx, ry);
            return self.destroy_bridge_walker_ns_low(sx, sy, terrain);
        }
        if Self::is_ew_walker_overlay_low(overlay) {
            let (sx, sy) = self.find_walker_start_low_ew(rx, ry);
            return self.destroy_bridge_walker_ew_low(sx, sy, terrain);
        }
        StateOutcome::NoChange
    }

    // ----- Pre-walk start-cell shift (mirror of binary's 3-case neighbor
    // check at the walker entries). -----

    fn find_walker_start_low_ns(&self, rx: u16, ry: u16) -> (u16, u16) {
        let in_range = |o: u8| (0x4A..=0x65).contains(&o);
        let north_off = ry == 0
            || self
                .cell(rx, ry - 1)
                .map(|c| !in_range(c.overlay_byte))
                .unwrap_or(true);
        if north_off {
            return (rx, ry.saturating_add(1));
        }
        let north2_on = ry >= 2
            && self
                .cell(rx, ry - 2)
                .map(|c| in_range(c.overlay_byte))
                .unwrap_or(false);
        if north2_on {
            return (rx, ry - 1);
        }
        (rx, ry)
    }

    fn find_walker_start_low_ew(&self, rx: u16, ry: u16) -> (u16, u16) {
        let in_range = |o: u8| (0x4A..=0x65).contains(&o);
        let west_off = rx == 0
            || self
                .cell(rx - 1, ry)
                .map(|c| !in_range(c.overlay_byte))
                .unwrap_or(true);
        if west_off {
            return (rx.saturating_add(1), ry);
        }
        let west2_on = rx >= 2
            && self
                .cell(rx - 2, ry)
                .map(|c| in_range(c.overlay_byte))
                .unwrap_or(false);
        if west2_on {
            return (rx - 1, ry);
        }
        (rx, ry)
    }

    // ----- Axis classification by overlay byte. -----

    pub(super) fn is_ns_walker_overlay_high(overlay: u8) -> bool {
        // HIGH NS axis sub-range:
        //   [0xCD..=0xD5] ∪ [0xDF..=0xE2] ∪ {0xE7}
        (0xCD..=0xD5).contains(&overlay) || (0xDF..=0xE2).contains(&overlay) || overlay == 0xE7
    }

    pub(super) fn is_ew_walker_overlay_high(overlay: u8) -> bool {
        // HIGH EW axis sub-range:
        //   [0xD6..=0xDE] ∪ [0xE3..=0xE6] ∪ {0xE8}
        (0xD6..=0xDE).contains(&overlay) || (0xE3..=0xE6).contains(&overlay) || overlay == 0xE8
    }

    pub(super) fn is_ns_walker_overlay_low(overlay: u8) -> bool {
        // LOW NS axis sub-range:
        //   [0x4A..=0x52] ∪ [0x5C..=0x5F] ∪ {0x64}
        (0x4A..=0x52).contains(&overlay) || (0x5C..=0x5F).contains(&overlay) || overlay == 0x64
    }

    pub(super) fn is_ew_walker_overlay_low(overlay: u8) -> bool {
        // LOW EW axis sub-range:
        //   [0x53..=0x5B] ∪ [0x60..=0x63] ∪ {0x65}
        (0x53..=0x5B).contains(&overlay) || (0x60..=0x63).contains(&overlay) || overlay == 0x65
    }

    // ----- Cell-triple iteration helpers. -----

    fn ns_triple(rx: u16, ry: u16) -> [Option<(u16, u16)>; 3] {
        let north = if ry > 0 { Some((rx, ry - 1)) } else { None };
        let south = Some((rx, ry.saturating_add(1)));
        [Some((rx, ry)), north, south]
    }

    fn ew_triple(rx: u16, ry: u16) -> [Option<(u16, u16)>; 3] {
        let west = if rx > 0 { Some((rx - 1, ry)) } else { None };
        let east = Some((rx.saturating_add(1), ry));
        [Some((rx, ry)), west, east]
    }

    /// Main low-damage walkers write both perpendicular neighbors before the
    /// center cell, then defer center-first RecalcAttributes until after their
    /// sibling-cascade calls return.
    fn ns_low_root_write_order(rx: u16, ry: u16) -> [Option<(u16, u16)>; 3] {
        let [center, north, south] = Self::ns_triple(rx, ry);
        [north, south, center]
    }

    fn ew_low_root_write_order(rx: u16, ry: u16) -> [Option<(u16, u16)>; 3] {
        let [center, west, east] = Self::ew_triple(rx, ry);
        [west, east, center]
    }

    // ----- LOW perpendicular neighbor classifiers. -----

    /// Classify the EW-axis perpendicular pattern at `(rx, ry)` for LOW
    /// bridges. Bit assignment (matches the binary's switch order at the
    /// LOW EW classifier — east-first, then west):
    /// - bit 0 (val 1): east in `{0x4E, 0x50, 0x52, 0x5D}`
    /// - bit 1 (val 2): east in `{0x51, 0x64}`
    /// - bit 2 (val 4): west in `{0x4F, 0x50, 0x51, 0x5F}`
    /// - bit 3 (val 8): west in `{0x52, 0x64}`
    /// `MapClass::CheckBridgeNeighbors_EW_Low` 0x0057B870 — the LOW twin of
    /// 0x0057CAB0.
    pub(super) fn check_bridge_neighbors_ew_low(&self, rx: u16, ry: u16) -> u8 {
        let east = self
            .cell(rx.saturating_add(1), ry)
            .map(|c| c.overlay_byte)
            .unwrap_or(0);
        let west = if rx > 0 {
            self.cell(rx - 1, ry).map(|c| c.overlay_byte).unwrap_or(0)
        } else {
            0
        };
        let mut idx = 0u8;
        match east {
            0x4E | 0x50 | 0x52 | 0x5D => idx |= 1,
            0x51 | 0x64 => idx |= 2,
            _ => {}
        }
        match west {
            0x4F | 0x50 | 0x51 | 0x5F => idx |= 4,
            0x52 | 0x64 => idx |= 8,
            _ => {}
        }
        idx
    }

    /// Classify the NS-axis perpendicular pattern at `(rx, ry)` for LOW
    /// bridges. Bit assignment (matches the binary — north-first, then
    /// south):
    /// - bit 0 (val 1): north in `{0x57, 0x59, 0x5B, 0x61}`
    /// - bit 1 (val 2): north in `{0x5A, 0x65}`
    /// - bit 2 (val 4): south in `{0x58, 0x59, 0x5A, 0x63}`
    /// - bit 3 (val 8): south in `{0x5B, 0x65}`
    /// `MapClass::CheckBridgeNeighbors_NS_Low` 0x0057B990 — the LOW twin of
    /// 0x0057CBE0.
    pub(super) fn check_bridge_neighbors_ns_low(&self, rx: u16, ry: u16) -> u8 {
        let north = if ry > 0 {
            self.cell(rx, ry - 1).map(|c| c.overlay_byte).unwrap_or(0)
        } else {
            0
        };
        let south = self
            .cell(rx, ry.saturating_add(1))
            .map(|c| c.overlay_byte)
            .unwrap_or(0);
        let mut idx = 0u8;
        match north {
            0x57 | 0x59 | 0x5B | 0x61 => idx |= 1,
            0x5A | 0x65 => idx |= 2,
            _ => {}
        }
        match south {
            0x58 | 0x59 | 0x5A | 0x63 => idx |= 4,
            0x5B | 0x65 => idx |= 8,
            _ => {}
        }
        idx
    }

    // ----- LOW sibling-cascade leaves. -----

    /// Sibling-cascade leaf for the LOW NS body axis. Mirror of
    /// `apply_bridge_destruction_ns_high` with LOW outer gate
    /// (`[0x4A..=0x65]`), LOW NS table, LOW intermediates 0x5C/0x5E, and
    /// final 0x64.
    fn apply_bridge_destruction_ns_low(
        &mut self,
        rx: u16,
        ry: u16,
        radar: &mut Vec<(u16, u16)>,
    ) -> Vec<(u16, u16)> {
        use crate::sim::bridge_specs::pick_destruction_overlay;
        use crate::sim::bridge_state::{Axis, DamageState};

        let mut final_cells = Vec::new();
        let Some(cell) = self.cell(rx, ry).copied() else {
            return final_cells;
        };
        let cur = cell.overlay_byte;
        if !(0x4A..=0x65).contains(&cur) {
            return final_cells;
        }

        let idx = self.check_bridge_neighbors_ew_low(rx, ry);
        if idx == 0 {
            return final_cells;
        }

        let next = if cur < 0x5C {
            match pick_destruction_overlay(idx, Axis::NS, false) {
                Some(n) if n != cur => n,
                _ => return final_cells,
            }
        } else if cur == 0x5C {
            0x5D
        } else if cur == 0x5E {
            0x5F
        } else {
            return final_cells;
        };

        let triple = Self::ns_triple(rx, ry);
        for pos in triple.into_iter().flatten() {
            let _ = self.write_overlay_byte_deferred_recalc(pos.0, pos.1, next);
        }
        for slot in triple {
            if let Some(pos) = slot {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    // Every touched cell (final OR intermediate) is minimap-dirty.
                    radar.push(pos);
                    if next == 0x64 {
                        c.damage_state = DamageState::Destroyed;
                        final_cells.push(pos);
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }
        for pos in triple.into_iter().flatten() {
            self.queue_overlay_recalc(pos.0, pos.1);
        }
        final_cells
    }

    /// Sibling-cascade leaf for the LOW EW body axis. Intermediates
    /// 0x60/0x62 → 0x61/0x63; final 0x65.
    fn apply_bridge_destruction_ew_low(
        &mut self,
        rx: u16,
        ry: u16,
        radar: &mut Vec<(u16, u16)>,
    ) -> Vec<(u16, u16)> {
        use crate::sim::bridge_specs::pick_destruction_overlay;
        use crate::sim::bridge_state::{Axis, DamageState};

        let mut final_cells = Vec::new();
        let Some(cell) = self.cell(rx, ry).copied() else {
            return final_cells;
        };
        let cur = cell.overlay_byte;
        if !(0x4A..=0x65).contains(&cur) {
            return final_cells;
        }

        let idx = self.check_bridge_neighbors_ns_low(rx, ry);
        if idx == 0 {
            return final_cells;
        }

        let next = if cur < 0x60 {
            match pick_destruction_overlay(idx, Axis::EW, false) {
                Some(n) if n != cur => n,
                _ => return final_cells,
            }
        } else if cur == 0x60 {
            0x61
        } else if cur == 0x62 {
            0x63
        } else {
            return final_cells;
        };

        let triple = Self::ew_triple(rx, ry);
        for pos in triple.into_iter().flatten() {
            let _ = self.write_overlay_byte_deferred_recalc(pos.0, pos.1, next);
        }
        for slot in triple {
            if let Some(pos) = slot {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    // Every touched cell (final OR intermediate) is minimap-dirty.
                    radar.push(pos);
                    if next == 0x65 {
                        c.damage_state = DamageState::Destroyed;
                        final_cells.push(pos);
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }
        for pos in triple.into_iter().flatten() {
            self.queue_overlay_recalc(pos.0, pos.1);
        }
        final_cells
    }

    // ----- LOW walker bodies. -----

    /// `MapClass::DestroyBridgeWalker_NS_Low` 0x0057BCF0 — verified against
    /// the decompile: same body shape as 0x0057CF60 with the LOW constants,
    /// down to the three `RadarClass::MarkTerrainDirty` calls and the
    /// `RebuildZoneConnectivity` on final collapse. Cascades through
    /// `MapClass::ApplyBridgeDestruction_NS_Low` 0x0057DD50 and
    /// `MapClass::FindBridgeEndpoints_NS_Low` 0x0057C990.
    ///
    /// LOW NS-axis walker. Native twin of concrete57CF60 with
    /// LOW case values:
    /// - `0x5C` → write 0x5D to (this, north, south); cascade west sibling
    /// - `0x5E` → write 0x5F to triple; cascade east sibling
    /// - `< 0x50` → write 0x50 to triple; cascade BOTH (rx±1, ry)
    /// - `[0x50..=0x52]` → write 0x64 to triple (FINAL); cascade BOTH;
    ///   mark zones_dirty
    /// - else → no-op
    pub(super) fn destroy_bridge_walker_ns_low(
        &mut self,
        rx: u16,
        ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        use crate::sim::bridge_specs::{CellAction, SetBridgeDirectionResult};
        use crate::sim::bridge_state::{Axis, DamageState, compute_adjacent_bridges_dirty};

        let Some(cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };
        let cur = cell.overlay_byte;

        let (next, siblings, is_final): (u8, Vec<(u16, u16)>, bool) = if cur == 0x5C {
            (0x5D, vec![(rx.wrapping_sub(1), ry)], false)
        } else if cur == 0x5E {
            (0x5F, vec![(rx.saturating_add(1), ry)], false)
        } else if cur < 0x50 {
            (
                0x50,
                vec![(rx.wrapping_sub(1), ry), (rx.saturating_add(1), ry)],
                false,
            )
        } else if (0x50..=0x52).contains(&cur) {
            (
                0x64,
                vec![(rx.wrapping_sub(1), ry), (rx.saturating_add(1), ry)],
                true,
            )
        } else {
            return StateOutcome::NoChange;
        };

        let mut destroyed: Vec<(u16, u16)> = Vec::new();
        let mut actions: Vec<((u16, u16), usize, CellAction)> = Vec::new();
        // BR-16: cells whose overlay this collapse touches (triple + cascade),
        // fed to the minimap radar-dirty channel by the orchestrator.
        let mut radar_cells: Vec<(u16, u16)> = Vec::new();

        let triple = Self::ns_triple(rx, ry);
        for pos in Self::ns_low_root_write_order(rx, ry).into_iter().flatten() {
            let _ = self.write_overlay_byte_deferred_recalc(pos.0, pos.1, next);
        }
        for (slot, opt_pos) in triple.into_iter().enumerate() {
            if let Some(pos) = opt_pos {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    radar_cells.push(pos);
                    if is_final {
                        c.damage_state = DamageState::Destroyed;
                        destroyed.push(pos);
                        actions.push((pos, slot, CellAction::BlowUpBridge));
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }

        for (sx, sy) in siblings {
            if sx == u16::MAX {
                continue;
            }
            let sibling_finals = self.apply_bridge_destruction_ns_low(sx, sy, &mut radar_cells);
            for pos in sibling_finals {
                if !destroyed.contains(&pos) {
                    destroyed.push(pos);
                    actions.push((pos, 0, CellAction::BlowUpBridge));
                }
            }
        }

        for pos in triple.into_iter().flatten() {
            self.queue_overlay_recalc(pos.0, pos.1);
        }

        if !is_final && destroyed.is_empty() {
            return StateOutcome::Absorbed {
                damaged_variant_cells: Vec::new(),
            };
        }

        let adj = compute_adjacent_bridges_dirty(rx, ry, Axis::NS);
        StateOutcome::Collapsed {
            binary_success: true,
            destroyed_cells: destroyed,
            set_bridge_direction: SetBridgeDirectionResult {
                actions,
                flag_stamp: None,
            },
            setter_transcript: Vec::new(),
            adjacent_bridges_dirty: adj,
            zones_dirty: is_final,
            radar_cells,
            damaged_variant_cells: Vec::new(),
        }
    }

    /// `MapClass::DestroyBridgeWalker_EW_Low` 0x0057C2B0 — the compiled twin
    /// of 0x0057BCF0 with the EW constants. Cascades through
    /// `MapClass::ApplyBridgeDestruction_EW_Low` 0x0057E2A0 and
    /// `MapClass::FindBridgeEndpoints_EW_Low` 0x0057C870.
    ///
    /// LOW EW-axis walker. Mirror of NS LOW with EW case values:
    /// - `0x60` → write 0x61 to (this, west, east); cascade south sibling
    /// - `0x62` → write 0x63 to triple; cascade north sibling
    /// - `< 0x59` → write 0x59 to triple; cascade BOTH (rx, ry±1)
    /// - `[0x59..=0x5B]` → write 0x65 to triple (FINAL); cascade BOTH;
    ///   mark zones_dirty
    /// - else → no-op
    pub(super) fn destroy_bridge_walker_ew_low(
        &mut self,
        rx: u16,
        ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        use crate::sim::bridge_specs::{CellAction, SetBridgeDirectionResult};
        use crate::sim::bridge_state::{Axis, DamageState, compute_adjacent_bridges_dirty};

        let Some(cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };
        let cur = cell.overlay_byte;

        let (next, siblings, is_final): (u8, Vec<(u16, u16)>, bool) = if cur == 0x60 {
            (0x61, vec![(rx, ry.saturating_add(1))], false)
        } else if cur == 0x62 {
            (0x63, vec![(rx, ry.wrapping_sub(1))], false)
        } else if cur < 0x59 {
            (
                0x59,
                vec![(rx, ry.wrapping_sub(1)), (rx, ry.saturating_add(1))],
                false,
            )
        } else if (0x59..=0x5B).contains(&cur) {
            (
                0x65,
                vec![(rx, ry.wrapping_sub(1)), (rx, ry.saturating_add(1))],
                true,
            )
        } else {
            return StateOutcome::NoChange;
        };

        let mut destroyed: Vec<(u16, u16)> = Vec::new();
        let mut actions: Vec<((u16, u16), usize, CellAction)> = Vec::new();
        // BR-16: cells whose overlay this collapse touches (triple + cascade),
        // fed to the minimap radar-dirty channel by the orchestrator.
        let mut radar_cells: Vec<(u16, u16)> = Vec::new();

        let triple = Self::ew_triple(rx, ry);
        for pos in Self::ew_low_root_write_order(rx, ry).into_iter().flatten() {
            let _ = self.write_overlay_byte_deferred_recalc(pos.0, pos.1, next);
        }
        for (slot, opt_pos) in triple.into_iter().enumerate() {
            if let Some(pos) = opt_pos {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    radar_cells.push(pos);
                    if is_final {
                        c.damage_state = DamageState::Destroyed;
                        destroyed.push(pos);
                        actions.push((pos, slot, CellAction::BlowUpBridge));
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }

        for (sx, sy) in siblings {
            if sy == u16::MAX {
                continue;
            }
            let sibling_finals = self.apply_bridge_destruction_ew_low(sx, sy, &mut radar_cells);
            for pos in sibling_finals {
                if !destroyed.contains(&pos) {
                    destroyed.push(pos);
                    actions.push((pos, 0, CellAction::BlowUpBridge));
                }
            }
        }

        for pos in triple.into_iter().flatten() {
            self.queue_overlay_recalc(pos.0, pos.1);
        }

        if !is_final && destroyed.is_empty() {
            return StateOutcome::Absorbed {
                damaged_variant_cells: Vec::new(),
            };
        }

        let adj = compute_adjacent_bridges_dirty(rx, ry, Axis::EW);
        StateOutcome::Collapsed {
            binary_success: true,
            destroyed_cells: destroyed,
            set_bridge_direction: SetBridgeDirectionResult {
                actions,
                flag_stamp: None,
            },
            setter_transcript: Vec::new(),
            adjacent_bridges_dirty: adj,
            zones_dirty: is_final,
            radar_cells,
            damaged_variant_cells: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::bridge_state::{
        Axis, BridgeCellRole, BridgeEndpointRecord, BridgeRecordKind, BridgeRuntimeCell,
        BridgeheadAnchorClass, DamageState,
    };

    fn empty_terrain() -> ResolvedTerrainGrid {
        ResolvedTerrainGrid::from_cells(0, 0, Vec::new())
    }

    #[test]
    fn destroy_overlay_predicates_match_gamemd_ranges() {
        assert!(!BridgeRuntimeState::is_low_destroy_overlay(0x49));
        assert!(BridgeRuntimeState::is_low_destroy_overlay(0x4A));
        assert!(BridgeRuntimeState::is_low_destroy_overlay(0x65));
        assert!(!BridgeRuntimeState::is_low_destroy_overlay(0x66));

        assert!(!BridgeRuntimeState::is_high_destroy_overlay(0xCC));
        assert!(BridgeRuntimeState::is_high_destroy_overlay(0xCD));
        assert!(BridgeRuntimeState::is_high_destroy_overlay(0xE8));
        assert!(!BridgeRuntimeState::is_high_destroy_overlay(0xE9));
    }

    #[test]
    fn destroy_overlay_axis_helpers_match_representative_subranges() {
        assert_eq!(
            BridgeRuntimeState::low_destroy_overlay_axis(0x4A),
            Some(Axis::NS)
        );
        assert_eq!(
            BridgeRuntimeState::low_destroy_overlay_axis(0x53),
            Some(Axis::EW)
        );
        assert_eq!(BridgeRuntimeState::low_destroy_overlay_axis(0x49), None);

        assert_eq!(
            BridgeRuntimeState::high_destroy_overlay_axis(0xCD),
            Some(Axis::NS)
        );
        assert_eq!(
            BridgeRuntimeState::high_destroy_overlay_axis(0xD6),
            Some(Axis::EW)
        );
        assert_eq!(BridgeRuntimeState::high_destroy_overlay_axis(0xCC), None);
    }

    fn seed_high_body_cell(state: &mut BridgeRuntimeState, rx: u16, ry: u16, overlay: u8) {
        state.test_seed_cell(
            rx,
            ry,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 5,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: overlay,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
    }

    #[test]
    fn destroy_bridge_low_returns_nochange_for_high_overlay() {
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 0, 0, 0xD0);
        let terrain = empty_terrain();
        assert_eq!(
            state.destroy_bridge_low(0, 0, &terrain),
            StateOutcome::NoChange
        );
    }

    #[test]
    fn axis_classifiers_partition_high_range() {
        // Sample a few overlay values from each sub-range.
        for v in [0xCDu8, 0xD0, 0xD5, 0xDF, 0xE0, 0xE2, 0xE7] {
            assert!(BridgeRuntimeState::is_ns_walker_overlay_high(v));
            assert!(!BridgeRuntimeState::is_ew_walker_overlay_high(v));
        }
        for v in [0xD6u8, 0xDA, 0xDE, 0xE3, 0xE5, 0xE6, 0xE8] {
            assert!(BridgeRuntimeState::is_ew_walker_overlay_high(v));
            assert!(!BridgeRuntimeState::is_ns_walker_overlay_high(v));
        }
        // Out-of-range values match neither.
        for v in [0u8, 0x4A, 0x65, 0xCC, 0xE9, 0xFF] {
            assert!(!BridgeRuntimeState::is_ns_walker_overlay_high(v));
            assert!(!BridgeRuntimeState::is_ew_walker_overlay_high(v));
        }
    }

    #[test]
    fn axis_classifiers_partition_low_range() {
        for v in [0x4Au8, 0x4F, 0x52, 0x5C, 0x5F, 0x64] {
            assert!(BridgeRuntimeState::is_ns_walker_overlay_low(v));
            assert!(!BridgeRuntimeState::is_ew_walker_overlay_low(v));
        }
        for v in [0x53u8, 0x57, 0x5B, 0x60, 0x63, 0x65] {
            assert!(BridgeRuntimeState::is_ew_walker_overlay_low(v));
            assert!(!BridgeRuntimeState::is_ns_walker_overlay_low(v));
        }
        for v in [0u8, 0x49, 0x66, 0xCC, 0xD0, 0xFF] {
            assert!(!BridgeRuntimeState::is_ns_walker_overlay_low(v));
            assert!(!BridgeRuntimeState::is_ew_walker_overlay_low(v));
        }
    }

    fn seed_low_body_cell(
        state: &mut BridgeRuntimeState,
        rx: u16,
        ry: u16,
        axis: Axis,
        overlay: u8,
    ) {
        state.test_seed_cell(
            rx,
            ry,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 2,
                bridge_group_id: Some(2),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(axis),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(2),
                overlay_byte: overlay,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
    }

    #[test]
    fn ns_low_walker_intermediate_writes_0x50_to_triple() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x4A);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        assert!(matches!(outcome, StateOutcome::Absorbed { .. }));
        for y in 0..3 {
            let c = state.cell(2, y).unwrap();
            assert_eq!(c.overlay_byte, 0x50);
            assert_eq!(c.damage_state, DamageState::Damaged);
        }
    }

    #[test]
    fn gsi_04_13_low_overlay_projection_ops_preserve_native_phases_and_repeated_writes() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x4A);
        }

        assert!(!state.write_overlay_byte(2, 1, 0x4A));
        assert_eq!(
            state.take_overlay_projection_ops(),
            vec![
                crate::sim::bridge_state::BridgeOverlayProjectionOp::Write {
                    rx: 2,
                    ry: 1,
                    overlay_byte: 0x4A,
                },
                crate::sim::bridge_state::BridgeOverlayProjectionOp::Recalc { rx: 2, ry: 1 },
            ],
            "an overlapping native writer still performs RecalcAttributes"
        );

        let outcome = state.destroy_bridge_walker_ns_low(2, 1, &empty_terrain());
        assert!(matches!(outcome, StateOutcome::Absorbed { .. }));
        assert_eq!(
            state.take_overlay_projection_ops(),
            projection_ops(
                &[((2, 0), 0x50), ((2, 2), 0x50), ((2, 1), 0x50)],
                &[(2, 1), (2, 0), (2, 2)],
            ),
            "NS root writes north/south/center before center/north/south recalc"
        );

        let mut ew = BridgeRuntimeState::default();
        for x in 0..3u16 {
            seed_low_body_cell(&mut ew, x, 2, Axis::EW, 0x53);
        }
        let outcome = ew.destroy_bridge_walker_ew_low(1, 2, &empty_terrain());
        assert!(matches!(outcome, StateOutcome::Absorbed { .. }));
        assert_eq!(
            ew.take_overlay_projection_ops(),
            projection_ops(
                &[((0, 2), 0x59), ((2, 2), 0x59), ((1, 2), 0x59)],
                &[(1, 2), (0, 2), (2, 2)],
            ),
            "EW root writes west/east/center before center/west/east recalc"
        );

        let mut leaf = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut leaf, 2, y, Axis::NS, 0x4A);
        }
        seed_low_body_cell(&mut leaf, 3, 1, Axis::NS, 0x4E);
        let mut radar = Vec::new();
        let _ = leaf.apply_bridge_destruction_ns_low(2, 1, &mut radar);
        let leaf_byte = leaf.cell(2, 1).expect("leaf center").overlay_byte;
        assert_eq!(
            leaf.take_overlay_projection_ops(),
            projection_ops(
                &[
                    ((2, 1), leaf_byte),
                    ((2, 0), leaf_byte),
                    ((2, 2), leaf_byte),
                ],
                &[(2, 1), (2, 0), (2, 2)],
            ),
            "cascade leaf completes center/north/south writes before recalc"
        );

        let mut ew_leaf = BridgeRuntimeState::default();
        for x in 0..3u16 {
            seed_low_body_cell(&mut ew_leaf, x, 2, Axis::EW, 0x53);
        }
        seed_low_body_cell(&mut ew_leaf, 1, 1, Axis::EW, 0x57);
        let mut radar = Vec::new();
        let _ = ew_leaf.apply_bridge_destruction_ew_low(1, 2, &mut radar);
        let leaf_byte = ew_leaf.cell(1, 2).expect("EW leaf center").overlay_byte;
        assert_eq!(
            ew_leaf.take_overlay_projection_ops(),
            projection_ops(
                &[
                    ((1, 2), leaf_byte),
                    ((0, 2), leaf_byte),
                    ((2, 2), leaf_byte),
                ],
                &[(1, 2), (0, 2), (2, 2)],
            ),
            "EW cascade leaf completes center/west/east writes before recalc"
        );
    }

    fn projection_ops(
        writes: &[((u16, u16), u8)],
        recalcs: &[(u16, u16)],
    ) -> Vec<crate::sim::bridge_state::BridgeOverlayProjectionOp> {
        writes
            .iter()
            .map(|&((rx, ry), overlay_byte)| {
                crate::sim::bridge_state::BridgeOverlayProjectionOp::Write {
                    rx,
                    ry,
                    overlay_byte,
                }
            })
            .chain(recalcs.iter().map(|&(rx, ry)| {
                crate::sim::bridge_state::BridgeOverlayProjectionOp::Recalc { rx, ry }
            }))
            .collect()
    }

    #[test]
    fn low_direct_first_hit_damages_without_deactivating_zone_record_then_second_hit_collapses() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x4A);
        }
        state.test_set_endpoint_records(vec![BridgeEndpointRecord {
            endpoint_a: (2, 0),
            endpoint_b: (2, 2),
            group_id: 2,
            active: true,
            bridge_kind: BridgeRecordKind::Low,
        }]);
        let terrain = empty_terrain();

        let first = state.destroy_bridge_low(2, 1, &terrain);
        assert!(
            matches!(first, StateOutcome::Absorbed { .. }),
            "healthy low bridge hit must be an intermediate damage transition"
        );
        for y in 0..3 {
            let cell = state.cell(2, y).unwrap();
            assert_eq!(cell.overlay_byte, 0x50);
            assert_eq!(cell.damage_state, DamageState::Damaged);
            assert!(
                state.is_bridge_walkable(2, y),
                "damaged low bridge cell (2, {y}) must remain bridge-walkable"
            );
        }
        state.refresh_endpoint_active_flags();
        assert!(
            state.endpoint_records()[0].active,
            "intermediate damage must not remove low-bridge zone connectivity"
        );

        let second = state.destroy_bridge_low(2, 1, &terrain);
        match second {
            StateOutcome::Collapsed {
                destroyed_cells,
                zones_dirty,
                ..
            } => {
                assert!(
                    zones_dirty,
                    "destroyed-anchor transition rebuilds bridge zones"
                );
                for y in 0..3 {
                    assert!(destroyed_cells.contains(&(2, y)));
                    let cell = state.cell(2, y).unwrap();
                    assert_eq!(cell.overlay_byte, 0x64);
                    assert_eq!(cell.damage_state, DamageState::Destroyed);
                    assert!(
                        !state.is_bridge_walkable(2, y),
                        "destroyed low bridge cell (2, {y}) must stop being bridge-walkable"
                    );
                }
            }
            other => panic!("expected final collapse on second hit, got {:?}", other),
        }
        state.refresh_endpoint_active_flags();
        assert!(
            !state.endpoint_records()[0].active,
            "destroyed low bridge must remove bridge-zone connectivity"
        );
    }

    #[test]
    fn ns_low_walker_final_writes_0x64_marks_destroyed_zones_dirty() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x51);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                zones_dirty,
                ..
            } => {
                assert!(zones_dirty);
                for y in 0..3 {
                    let c = state.cell(2, y).unwrap();
                    assert_eq!(c.overlay_byte, 0x64);
                    assert_eq!(c.damage_state, DamageState::Destroyed);
                    assert!(destroyed_cells.contains(&(2, y)));
                }
            }
            other => panic!("expected Collapsed, got {:?}", other),
        }
    }

    #[test]
    fn ns_low_walker_0x5c_writes_0x5d_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x5C);
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0x5D);
        }
    }

    #[test]
    fn ns_low_walker_0x5e_writes_0x5f_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x5E);
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0x5F);
        }
    }

    #[test]
    fn ns_low_walker_returns_nochange_above_0x52_below_0x5c() {
        // 0x55 is in the LOW EW sub-range (would route to EW walker). NS
        // walker hit on it must be a no-op.
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 2, 1, Axis::NS, 0x55);
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
        assert_eq!(state.cell(2, 1).unwrap().overlay_byte, 0x55);
    }

    #[test]
    fn ew_low_walker_final_writes_0x65_marks_destroyed_zones_dirty() {
        let mut state = BridgeRuntimeState::default();
        for x in 0..3u16 {
            seed_low_body_cell(&mut state, x, 2, Axis::EW, 0x5A);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ew_low(1, 2, &terrain);
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                zones_dirty,
                ..
            } => {
                assert!(zones_dirty);
                for x in 0..3 {
                    let c = state.cell(x, 2).unwrap();
                    assert_eq!(c.overlay_byte, 0x65);
                    assert_eq!(c.damage_state, DamageState::Destroyed);
                    assert!(destroyed_cells.contains(&(x, 2)));
                }
            }
            other => panic!("expected Collapsed, got {:?}", other),
        }
    }

    #[test]
    fn ew_low_walker_0x60_writes_0x61_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for x in 0..3u16 {
            seed_low_body_cell(&mut state, x, 2, Axis::EW, 0x60);
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ew_low(1, 2, &terrain);
        for x in 0..3 {
            assert_eq!(state.cell(x, 2).unwrap().overlay_byte, 0x61);
        }
    }

    #[test]
    fn ew_low_walker_0x62_writes_0x63_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for x in 0..3u16 {
            seed_low_body_cell(&mut state, x, 2, Axis::EW, 0x62);
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ew_low(1, 2, &terrain);
        for x in 0..3 {
            assert_eq!(state.cell(x, 2).unwrap().overlay_byte, 0x63);
        }
    }

    #[test]
    fn check_bridge_neighbors_ew_low_bit_layout() {
        // bit 0 (east in {0x4E, 0x50, 0x52, 0x5D}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 1, 0, Axis::NS, 0x4A);
        seed_low_body_cell(&mut state, 2, 0, Axis::NS, 0x4E);
        assert_eq!(state.check_bridge_neighbors_ew_low(1, 0), 1);
        // bit 1 (east in {0x51, 0x64}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 1, 0, Axis::NS, 0x4A);
        seed_low_body_cell(&mut state, 2, 0, Axis::NS, 0x64);
        assert_eq!(state.check_bridge_neighbors_ew_low(1, 0), 2);
        // bit 2 (west in {0x4F, 0x50, 0x51, 0x5F}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::NS, 0x4F);
        seed_low_body_cell(&mut state, 1, 0, Axis::NS, 0x4A);
        assert_eq!(state.check_bridge_neighbors_ew_low(1, 0), 4);
        // bit 3 (west in {0x52, 0x64}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::NS, 0x52);
        seed_low_body_cell(&mut state, 1, 0, Axis::NS, 0x4A);
        assert_eq!(state.check_bridge_neighbors_ew_low(1, 0), 8);
    }

    #[test]
    fn check_bridge_neighbors_ns_low_bit_layout() {
        // bit 0 (north in {0x57, 0x59, 0x5B, 0x61}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::EW, 0x57);
        seed_low_body_cell(&mut state, 0, 1, Axis::EW, 0x4A);
        assert_eq!(state.check_bridge_neighbors_ns_low(0, 1), 1);
        // bit 1 (north in {0x5A, 0x65}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::EW, 0x65);
        seed_low_body_cell(&mut state, 0, 1, Axis::EW, 0x4A);
        assert_eq!(state.check_bridge_neighbors_ns_low(0, 1), 2);
        // bit 2 (south in {0x58, 0x59, 0x5A, 0x63}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::EW, 0x4A);
        seed_low_body_cell(&mut state, 0, 1, Axis::EW, 0x63);
        assert_eq!(state.check_bridge_neighbors_ns_low(0, 0), 4);
        // bit 3 (south in {0x5B, 0x65}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::EW, 0x4A);
        seed_low_body_cell(&mut state, 0, 1, Axis::EW, 0x5B);
        assert_eq!(state.check_bridge_neighbors_ns_low(0, 0), 8);
    }

    #[test]
    fn destroy_bridge_low_classifies_ns_axis_into_walker() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x4A);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_low(2, 1, &terrain);
        assert!(matches!(outcome, StateOutcome::Absorbed { .. }));
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0x50);
        }
    }
}
