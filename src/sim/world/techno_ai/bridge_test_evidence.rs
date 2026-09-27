//! Read-only terrain/navigation evidence shared by physical bridge witnesses.
use crate::sim::world::Simulation;
use serde_json::{Value, json};

pub(super) fn navigation_snapshot(
    sim: &Simulation,
    name: &str,
    points: impl IntoIterator<Item = (u16, u16)>,
) -> Value {
    let zones = sim.zone_grid.as_ref().unwrap();
    let mut copied = zones.clone();
    let base = copied.base_topology_mut();
    let terrain = sim.resolved_terrain.as_ref().unwrap();
    let bridges = sim.bridge_state.as_ref().unwrap();
    let hierarchy = zones
        .hierarchy_for(crate::rules::locomotor_type::MovementZone::Normal)
        .unwrap();
    let side = usize::from(zones.width) + 1;
    let graphs: Vec<_> = (0..3)
        .map(|level| {
            let graph = hierarchy.level(level).unwrap();
            let records: Vec<_> =
                (0..graph.record_slot_count())
                    .map(|index| {
                        graph.record(index as u16).map_or(Value::Null, |record| {
                let edges: Vec<_> = graph.edges(index as u16).iter()
                    .map(|edge| [u32::from(edge.neighbor), u32::from(edge.flag)]).collect();
                json!({"parent":record.parent,"zone_type":record.zone_type,"edges":edges})
            })
                    })
                    .collect();
            let padding: Vec<_> = (0..side * side)
                .filter(|index| {
                    index % side >= usize::from(zones.width)
                        || index / side >= usize::from(zones.height)
                })
                .map(|index| graph.native_padding_zone(index))
                .collect();
            json!({"ids":graph.cell_zone_ids(),"padding_ids":padding,"records":records})
        })
        .collect();
    let records: Vec<_> = zones
        .bridge_records()
        .iter()
        .map(|r| {
            json!({
                "a": r.endpoint_a, "b": r.endpoint_b, "active": r.active,
                "kind": if r.is_high() { 0 } else { 1 },
            })
        })
        .collect();
    let mut points: std::collections::BTreeSet<_> = points.into_iter().collect();
    for r in zones.bridge_records() {
        points.extend([r.endpoint_a, r.endpoint_b]);
    }
    let cells: Vec<_> = points.into_iter().filter_map(|(x,y)| {
        let c = terrain.cell(x,y)?;
        let index = usize::from(y) * usize::from(zones.width) + usize::from(x);
        let cluster = *base.zone_ids.get(index)?;
        Some(json!({
            "coord": [x,y], "level": c.level, "slope": c.slope_type,
            "zone_type": c.zone_type, "land": c.yr_cell_land_type,
            "tile": c.final_tile_index, "subtile": c.final_sub_tile,
            "bridge_flags": c.bridge_facts.raw_flags,
            "overlay": c.bridge_facts.overlay_id,
            "cached_class": base.movement_classes[index], "cached_height": base.levels[index],
            "base_id": cluster, "raw_row7": base.raw_zone_ids_by_row[7].get(usize::from(cluster)),
        }))
    }).collect();
    json!({
        "name": name,
        "origin": {"boundary":name,"tick":sim.session.tick,"binary_frame":sim.session.binary_frame},
        "width": zones.width, "height": zones.height,
        "native_size": base.native_bridge_source_size,
        "classes": base.movement_classes, "levels": base.levels, "records": records,
        "graphs": graphs,
        "live_cell_levels": terrain.cells().iter().map(|cell| cell.level).collect::<Vec<_>>(),
        "live_cell_slopes": terrain.cells().iter().map(|cell| cell.slope_type).collect::<Vec<_>>(),
        "live_cell_allocated": terrain.cells().iter().map(|cell| {
            terrain.native_fixed_cell_index(cell.rx as i16, cell.ry as i16).is_some()
        }).collect::<Vec<_>>(),
        "records_match_bridge_authority": zones.bridge_records() == bridges.endpoint_records(),
        "bridge_authority_records": bridges.endpoint_records(),
        "rust": {"base_ids":base.zone_ids,"raw_rows":base.raw_zone_ids_by_row,
            "zone_count":base.raw_zone_ids_by_row[0].len().saturating_sub(1)},
        "cells": cells,
        "dummy": format!("{:?}", terrain.shared_cell_dummy().snapshot()),
    })
}
