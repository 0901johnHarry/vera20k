//! Full original InRange6F7220 comparisons on physical FV/HoverMissile.
//! Sparse map membership, object placement and every row's range/minimum fields
//! are supplied controls; retail HoverMissile itself reads Range1536/Minimum256.

use super::tests::flat_terrain;
use super::*;
use crate::sim::intern::test_interner;
use serde::Deserialize;

#[derive(Deserialize)]
struct CellState {
    xy: [i16; 2],
    level: u8,
    flags: u32,
}

#[derive(Deserialize)]
struct Input {
    name: String,
    source_xyz: [i32; 3],
    target_xyz: [i32; 3],
    target_marked: u8,
    target_on_bridge: u8,
    range: i32,
    minimum_range: i32,
    dummy: CellState,
    mapped_source: CellState,
}

#[derive(Deserialize)]
struct Row {
    input: Input,
    result: u8,
    adjusted_target_xyz: Option<[i32; 3]>,
    dummy_xy: [i16; 2],
}

#[derive(Deserialize)]
struct Corpus {
    native_sha256: String,
    rows: Vec<Row>,
}

fn reset_dummy(terrain: &ResolvedTerrainGrid, state: &CellState) {
    let dummy = terrain.shared_cell_dummy();
    dummy.stamp_coord(i32::from(state.xy[0]), i32::from(state.xy[1]));
    dummy.set_level_slope(state.level as i8, 0);
    dummy.write_raw_flags(state.flags);
}

fn placed(id: u64, xyz: [i32; 3], marked: bool, on_bridge: bool) -> GameEntity {
    let [x, y, z] = xyz;
    let mut entity = GameEntity::test_default(id, "FV", "Test", (x / 256) as u16, (y / 256) as u16);
    entity.category = EntityCategory::Unit;
    entity.position.sub_x = SimFixed::from_num(x % 256);
    entity.position.sub_y = SimFixed::from_num(y % 256);
    entity.position.exact_z_leptons = Some(z);
    entity.lifecycle.cell_marked = marked;
    entity.on_bridge = on_bridge;
    entity
}

#[test]
fn original_fv_range_uses_live_dummy_ground_and_source_bridge_queries() {
    let Some((ini, art)) = crate::rules::retail_ini_fixture::retail_rules_and_art() else {
        return;
    };
    let mut rules = RuleSet::from_ini_with_fixed_art_for_test(&ini, &art).unwrap();
    rules.merge_art_data(&crate::rules::art_data::ArtRegistry::from_ini(&art));
    let corpus: Corpus = serde_json::from_str(include_str!(
        "../../../tools/spatial_oracle/in_range_cell_boundary.json"
    ))
    .unwrap();
    assert_eq!(
        corpus.native_sha256,
        "1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c"
    );
    assert_eq!(corpus.rows.len(), 6);
    assert_eq!(
        rules.object("FV").unwrap().weapon_list[0].as_deref(),
        Some("HoverMissile")
    );
    let ordinary = rules.weapon("HoverMissile").unwrap();
    assert_eq!(ordinary.range_leptons, 1536);
    assert_eq!(ordinary.minimum_range_leptons, 256);
    assert!(
        !rules
            .projectile(ordinary.projectile.as_deref().unwrap())
            .unwrap()
            .arcing
    );

    for row in corpus.rows {
        let input = row.input;
        let [mx, my] = input.mapped_source.xy.map(|value| value as u16);
        let mut terrain = flat_terrain(43, 43);
        let real = terrain.cell_mut(mx, my).unwrap();
        real.level = input.mapped_source.level;
        real.bridge_facts.raw_flags = input.mapped_source.flags;
        terrain.test_set_native_allocated_cells(&[(mx, my)]);
        reset_dummy(&terrain, &input.dummy);
        let initial_dummy = terrain.shared_cell_dummy().snapshot();
        let actor = placed(100, input.source_xyz, true, false);
        let target = placed(
            200,
            input.target_xyz,
            input.target_marked != 0,
            input.target_on_bridge != 0,
        );
        let mut entities = EntityStore::new();
        entities.insert(target);
        let interner = test_interner();
        // Match supplied retained fields in the native direct-call fixture,
        // independently of the physical reader values asserted above.
        let mut weapon = ordinary.clone();
        weapon.range_leptons = input.range;
        weapon.minimum_range_leptons = input.minimum_range;
        let target = TargetKind::Entity(200);

        // Execute the same production decision once in each identity domain.
        // Cursor isolation must preserve native answers within its own domain.
        for isolated in [true, false] {
            reset_dummy(&terrain, &input.dummy);
            let cells = if isolated {
                NativeCellQuery::isolated(&terrain)
            } else {
                NativeCellQuery::canonical(&terrain)
            };
            let [x, y, z] = input.source_xyz.map(i64::from);
            assert_eq!(
                compute_in_range_in_query(
                    &actor,
                    (x, y, z),
                    &target,
                    &weapon,
                    &rules,
                    &interner,
                    &entities,
                    &cells,
                    &LineOfFireInputs::terrain_only(),
                ),
                row.result != 0,
                "{} isolated={isolated}: full range",
                input.name,
            );
            assert_eq!(
                cells.dummy().snapshot().coord,
                (i32::from(row.dummy_xy[0]), i32::from(row.dummy_xy[1])),
                "{} isolated={isolated}: final Dummy",
                input.name,
            );
            if isolated {
                assert_eq!(
                    terrain.shared_cell_dummy().snapshot(),
                    initial_dummy,
                    "{}",
                    input.name
                );
            }
        }
        // Compare the shared target projection separately. The unlimited
        // sentinel has no target projection and is already covered above.
        if let Some(expected) = row.adjusted_target_xyz {
            reset_dummy(&terrain, &input.dummy);
            assert_eq!(
                resolve_target_coords_3d(
                    &target,
                    &entities,
                    &rules,
                    &interner,
                    &NativeCellQuery::canonical(&terrain),
                ),
                Some((
                    i64::from(expected[0]),
                    i64::from(expected[1]),
                    i64::from(expected[2])
                )),
                "{}: target geometry",
                input.name,
            );
        }
    }
}
