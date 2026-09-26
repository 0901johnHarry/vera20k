//! Live CellClass object-list queries shared by bridge and debris callbacks.
//!
//! Occupancy owns entity list order; ProductionState owns Terrain membership.
//! This view adds no stored links. Callers choose the native before/after-hit
//! continuation point (BlowUpBridge47DD96 versus AnimClass423A83).

use super::Simulation;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::CellObjectMember;

impl Simulation {
    pub(crate) fn cell_objects(
        &self,
        cell: (u16, u16),
        layer: MovementLayer,
    ) -> impl Iterator<Item = CellObjectMember> + '_ {
        self.substrate.occupancy.cell_objects(
            cell.0,
            cell.1,
            layer,
            self.production.terrain_object_cells.get(&cell).copied(),
        )
    }

    pub(crate) fn cell_object_list_location(
        &self,
        object: CellObjectMember,
    ) -> Option<((u16, u16), MovementLayer)> {
        match object {
            CellObjectMember::Entity(id) => {
                let entity = self.substrate.entities.get(id)?;
                entity.lifecycle.cell_marked.then_some((
                    (entity.position.rx, entity.position.ry),
                    entity.occupancy_list_layer()?,
                ))
            }
            CellObjectMember::Terrain(id) => {
                let terrain = self.production.terrain_objects.get(&id)?;
                (terrain.is_live()
                    && self.production.terrain_object_cells.get(&terrain.cell()) == Some(&id))
                .then_some((terrain.cell(), MovementLayer::Ground))
            }
        }
    }

    /// Read Object+30 from the current list. Removal clears the link; movement
    /// replaces it with the successor in the new list. Never retain a snapshot
    /// of all targets across a synchronous receiver.
    pub(crate) fn next_cell_object(&self, object: CellObjectMember) -> Option<CellObjectMember> {
        let (cell, layer) = self.cell_object_list_location(object)?;
        let mut members = self.cell_objects(cell, layer);
        members.find(|candidate| *candidate == object)?;
        members.next()
    }
}
