//! Physical retail Cannon/120MM → production atlas → retained object builder →
//! layer lowering/upload/replay versus full original Object/Bullet/CCShape pixels.
//! Retained positions and flat cells are explicit fixture inputs, not a launch or
//! flight simulation claim. This supplements the independent GPU leaf checks.

use super::super::draw_plan_lowering::{NativeDisplayOrder, lower_object_instances};
use super::*;
use crate::app::presentation::instances::projectile_draw_instance;
use crate::assets::{asset_manager::AssetManager, pal_file::Palette};
use crate::map::houses::HouseColorMap;
use crate::map::resolved_terrain::{test_flat_cell, test_flat_ground_grid};
use crate::render::batch::{DepthAxis, InstanceBufferPool};
use crate::render::sprite_atlas::{SpriteAtlas, build_sprite_atlas};
use crate::render::terrain_draw::TerrainDrawRenderer;
use crate::render::terrain_draw_gpu_tests::{Gpu, camera, clear, encoded};
use crate::rules::{art_data::ArtRegistry, ini_parser::IniFile, ruleset::RuleSet};
use crate::sim::projectile::{
    ProjectileCollisionPolicy, ProjectileCoord, ProjectilePayload, ProjectileSpawn,
    ProjectileStore, ProjectileTarget, ProjectileTrajectory, ProjectileVelocity,
    ProjectileVisualState, TargetExpiryPolicy,
};
use crate::sim::world::{Simulation, display_layers::DisplayLayers};
use serde_json::Value;
use std::collections::HashSet;

fn physical_rules(assets: &AssetManager, anim_palette_control: bool) -> RuleSet {
    let original = IniFile::from_bytes(assets.get_ref("rulesmd.ini").unwrap()).unwrap();
    let mut original_art = IniFile::from_bytes(assets.get_ref("artmd.ini").unwrap()).unwrap();
    if anim_palette_control {
        // Native control supplies BulletType.AnimPalette=1. Exercise its actual
        // input owner (ART Image section), in a separate rules value so the
        // physical Cannon default remains unchanged.
        original_art.merge(&IniFile::from_str("[120MM]\nAnimPalette=yes\n"));
    }
    let cannon = original.section("Cannon").expect("physical Cannon section");
    let mtnk = original.section("MTNK").unwrap();
    let weapon_name = mtnk.get("Primary").unwrap();
    let weapon = original.section(weapon_name).unwrap();
    let projectile_name = weapon.get("Projectile").unwrap();
    assert_eq!((weapon_name, projectile_name), ("105mm", "Cannon"));
    // Narrow only the registry and irrelevant Techno/weapon fields. Every
    // ordinary Cannon input comes from physical RULESMD and fixed ART.
    let mut text = format!(
        "[VehicleTypes]\n0=MTNK\n[MTNK]\nPrimary={weapon_name}\n[{weapon_name}]\nProjectile={projectile_name}\n[Cannon]\n"
    );
    for key in cannon.keys() {
        text.push_str(&format!("{key}={}\n", cannon.get(key).unwrap()));
    }
    let mut rules =
        RuleSet::from_ini_with_fixed_art_for_test(&IniFile::from_str(&text), &original_art)
            .unwrap();
    rules.merge_art_data(&ArtRegistry::from_ini(&original_art));
    rules
}

fn fixture(row: &Value, kind: &crate::rules::projectile_type::ProjectileType) -> (Simulation, u64) {
    let input = &row["input"];
    let xyz = &row["retained_xyz"];
    let coord = ProjectileCoord::new(
        xyz[0].as_i64().unwrap() as i32,
        xyz[1].as_i64().unwrap() as i32,
        xyz[2].as_i64().unwrap() as i32,
    );
    let mut cell = test_flat_cell(10, 20);
    cell.level = input["level"].as_i64().unwrap() as u8;
    cell.bridge_facts.raw_flags = input["flags"].as_u64().unwrap() as u32;
    let mut sim = Simulation::with_seed(31);
    let mut terrain = test_flat_ground_grid(32);
    terrain.test_set_native_allocated_cells(&[(10, 20)]);
    *terrain.cell_mut(10, 20).unwrap() = cell;
    sim.install_resolved_terrain_for_new_map(terrain);
    let weapon = sim.interner.intern("105mm");
    let warhead = sim.interner.intern("AP");
    let id = sim.allocate_stable_id();
    sim.admit_projectile(
        id,
        ProjectileSpawn {
            flat: kind.flat,
            source_id: 0,
            origin: coord,
            target: ProjectileTarget::None,
            initial_target_position: coord,
            payload: ProjectilePayload {
                base_damage: 0,
                warhead,
                weapon,
            },
            speed_leptons_per_frame: 0,
            velocity: ProjectileVelocity::new(0, 0, 0),
            trajectory: ProjectileTrajectory::Straight,
            guidance: None,
            visual: ProjectileVisualState::new(
                u8::try_from(kind.anim_low).unwrap(),
                u8::try_from(kind.anim_high).unwrap(),
                u8::try_from(kind.anim_rate).unwrap(),
            ),
            arm_frames: 0,
            fuse_frames: None,
            ranged_fuse: false,
            tracks_target: false,
            target_expiry: TargetExpiryPolicy::Expire,
            collision: ProjectileCollisionPolicy::NONE,
        },
    );
    sim.projectiles.get_mut(id).unwrap().on_bridge = input["on_bridge"].as_i64().unwrap_or(0) != 0;
    (sim, id)
}

#[test]
#[ignore = "requires GPU and physical retail archives; production atlas/builder/layer/replay versus original full120MM shape draws"]
fn retail_bullet_atlas_geometry_and_layer_replay_match_original_shape_pixels() {
    let root = std::env::var("RA2_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            crate::util::config::GameConfig::load()
                .unwrap()
                .paths
                .ra2_dir
        });
    let assets = AssetManager::new(&root).unwrap();
    let native: Value = serde_json::from_str(include_str!(
        "../../../../tools/projectile_oracle/bridge_render_shape.json"
    ))
    .unwrap();
    let rules = physical_rules(&assets, false);
    let control_rules = physical_rules(&assets, true);
    assert!(!rules.projectile("Cannon").unwrap().anim_palette);
    assert!(control_rules.projectile("Cannon").unwrap().anim_palette);
    let kind = rules.projectile("Cannon").unwrap();
    let selected = &native["selected_native_type"];
    assert_eq!(kind.image.as_deref(), selected["image"].as_str());
    for (key, actual) in [
        ("flat", kind.flat),
        ("shadow", kind.shadow),
        ("inviso", kind.inviso),
        ("voxel", kind.voxel),
        ("anim_palette", kind.anim_palette),
        ("firers_palette", kind.firers_palette),
        ("inverse_rotates", kind.rotates),
    ] {
        assert_eq!(
            actual,
            selected[key].as_bool().unwrap(),
            "physical Cannon {key}"
        );
    }
    assert_eq!(
        crate::util::sha256::sha256_hex(assets.get_ref("120mm.shp").unwrap()),
        native["shp_sha256"].as_str().unwrap()
    );
    for source in native["palette_loads"].as_array().unwrap() {
        assert_eq!(
            crate::util::sha256::sha256_hex(
                assets.get_ref(source["name"].as_str().unwrap()).unwrap()
            ),
            source["sha256"].as_str().unwrap()
        );
    }
    let palette = Palette::from_bytes(assets.get_ref("palette.pal").unwrap()).unwrap();
    let gpu = Gpu::new();
    let size = [
        native["surface"]["width"].as_u64().unwrap() as u32,
        native["surface"]["height"].as_u64().unwrap() as u32,
    ];
    let rows = native["rows"]
        .as_array()
        .unwrap()
        .iter()
        .chain(native["canonical_depth_rows"].as_array().unwrap());
    assert_eq!(rows.clone().count(), 40);
    for format in [
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ] {
        let batch = BatchRenderer::new_with_device(&gpu.device, &gpu.queue, format);
        // Append the independently selected AnimPalette context through the
        // production atlas refresh, preserving the ordinary resident entry.
        let mut atlas = None;
        for current_rules in [&rules, &control_rules] {
            atlas = build_sprite_atlas(
                &gpu.device,
                &gpu.queue,
                &batch,
                &crate::sim::entity_store::EntityStore::new(),
                &assets,
                &palette,
                "tem",
                "TEMPERATE",
                Some(current_rules),
                Some(&current_rules.art_registry),
                &HouseColorMap::new(),
                &[],
                &HashSet::new(),
                &HashSet::new(),
                None,
                atlas,
                None,
            );
        }
        let atlas = atlas.expect("physical Cannon atlas");
        let missing = SpriteAtlas::from_test_pages(Vec::new());
        let color = gpu.target(size, format);
        let cv = color.create_view(&Default::default());
        let depth = gpu.target(size, wgpu::TextureFormat::Depth32Float);
        let dv = depth.create_view(&Default::default());
        let mut renderer = TerrainDrawRenderer::new(&gpu.device, &gpu.queue, format, &batch);
        let mut pool = InstanceBufferPool::new();
        for row in rows.clone() {
            let input = &row["input"];
            let type_id = "Cannon";
            let current_rules = if input["anim_palette"].as_i64() == Some(1) {
                &control_rules
            } else {
                &rules
            };
            let mut kind = current_rules.projectile(type_id).unwrap().clone();
            if let Some(value) = input["shadow"].as_i64() {
                kind.shadow = value != 0;
            }
            if let Some(value) = input["inviso"].as_i64() {
                kind.inviso = value != 0;
            }
            let atlas = if input["missing_image"].as_bool() == Some(true) {
                &missing
            } else {
                &atlas
            };
            let (mut sim, id) = fixture(row, &kind);
            let grid = sim.resolved_terrain.as_ref().unwrap();
            grid.shared_cell_dummy().stamp_coord(123, -44);
            let dummy_before = grid.shared_cell_dummy().snapshot();
            let before_hash = sim.state_hash();
            let camera_pos = [
                row["actual_camera"][0].as_i64().unwrap() as f32,
                row["actual_camera"][1].as_i64().unwrap() as f32 + 15.,
            ];
            let viewport = [size[0] as f32, size[1] as f32];
            let axis = DepthAxis::NONE;
            let order = NativeDisplayOrder::from_display(sim.display_layers());
            let object = projectile_draw_instance(
                sim.projectiles.get(id).unwrap(),
                &kind,
                type_id,
                atlas,
                grid,
                camera_pos,
                viewport,
                axis,
                &order,
            );
            // Serialize the actual retained components, not a freshly sorted
            // list or new projectile constructor. Full savegame restoration is
            // tested by its owning runtime; this pins presentation consumers.
            let (restored_store, restored_display): (ProjectileStore, DisplayLayers) =
                bincode::deserialize(
                    &bincode::serialize(&(&sim.projectiles, sim.display_layers())).unwrap(),
                )
                .unwrap();
            let restored_order = NativeDisplayOrder::from_display(&restored_display);
            let restored = projectile_draw_instance(
                restored_store.get(id).unwrap(),
                &kind,
                type_id,
                atlas,
                grid,
                camera_pos,
                viewport,
                axis,
                &restored_order,
            );
            let layers = lower_object_instances(object.into_iter().collect());
            let restored_layers = lower_object_instances(restored.into_iter().collect());
            for (live, restored) in layers.iter().zip(&restored_layers) {
                assert_eq!(live.runs, restored.runs, "{} retained runs", input["name"]);
                assert_eq!(
                    live.owners, restored.owners,
                    "{} retained owners",
                    input["name"]
                );
                assert_eq!(
                    bytemuck::cast_slice::<_, u8>(&live.instances),
                    bytemuck::cast_slice::<_, u8>(&restored.instances),
                    "{} retained instances",
                    input["name"]
                );
            }
            let air = &restored_layers[3];
            assert!(
                layers
                    .iter()
                    .enumerate()
                    .all(|(index, layer)| index == 3 || layer.instances.is_empty())
            );
            let mut view = camera(size);
            view.camera_pos = camera_pos;
            // The original fixture explicitly supplies baseline1024; production
            // uses32768. A test-only origin rebase matches that supplied buffer
            // without changing producer Z terms. Canonical rows need no rebase.
            view.native_z_origin_y =
                input["depth_baseline"].as_i64().unwrap_or(1024) as f32 - 32768.;
            batch.write_camera(&gpu.queue, view);
            renderer.prepare(&gpu.device, &color, &dv, batch.camera_uniform());
            pool.upload_on_device(
                &gpu.device,
                &gpu.queue,
                "bullet_native_shape",
                &air.instances,
            );
            let old_z = input["old_z"].as_u64().unwrap_or(2000) as u16;
            let background = input["background"].as_u64().unwrap_or(65535) as u16;
            assert_eq!(background, 65535, "this physical fixture supplies white");
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            clear(
                &mut encoder,
                &cv,
                &dv,
                old_z,
                wgpu::LoadOp::Clear(wgpu::Color::WHITE),
            );
            // Native Draw rebases its point by -dirtyXY, then CCShape adds the
            // clip origin back. Absolute output stays fixed; only scissor narrows.
            let tactical = input["dirty"]
                .as_array()
                .map_or([0, 0, size[0], size[1]], |rect| {
                    std::array::from_fn(|index| rect[index].as_u64().unwrap() as u32)
                });
            draw_native_object_pass(
                &mut encoder,
                &cv,
                &dv,
                &mut renderer,
                tactical,
                &batch,
                pool.get("bullet_native_shape"),
                air,
                None,
                None,
                None,
                &VxlSlopeTransitionCache::default(),
                Some(atlas),
                None,
                batch.default_zshape_bind_group(),
            );
            let reads = [
                gpu.read(&mut encoder, &color),
                gpu.read(&mut encoder, &depth),
            ];
            let output = gpu.finish(encoder, &reads, size);
            let mut expected = vec![encoded(background, format); (size[0] * size[1]) as usize];
            for pixel in row["changed_pixels"].as_array().unwrap() {
                let index = (pixel[1].as_u64().unwrap() * u64::from(size[0])
                    + pixel[0].as_u64().unwrap()) as usize;
                expected[index] = encoded(pixel[2].as_u64().unwrap() as u16, format);
            }
            for (index, (pixel, expected)) in output[0].chunks_exact(4).zip(expected).enumerate() {
                assert_eq!(
                    pixel,
                    expected,
                    "{format:?}, {}, pixel({}, {})",
                    input["name"],
                    index % size[0] as usize,
                    index / size[0] as usize
                );
            }
            assert_eq!(row["z_unchanged"], true);
            assert!(
                output[1].chunks_exact(4).all(
                    |pixel| pixel == crate::render::native_z::stored_depth(old_z).to_le_bytes()
                ),
                "{format:?}, {} retained depth",
                input["name"]
            );
            assert_eq!(grid.shared_cell_dummy().snapshot(), dummy_before);
            assert_eq!(
                sim.state_hash(),
                before_hash,
                "rendering must not change simulation state"
            );
            // The late store entry survives retirement, but its Display slot
            // is already gone. No storage-order rendering is admissible.
            assert!(sim.retire_non_entity_object(id));
            assert!(sim.projectiles.get(id).is_some());
            let removed_order = NativeDisplayOrder::from_display(sim.display_layers());
            assert!(
                projectile_draw_instance(
                    sim.projectiles.get(id).unwrap(),
                    &kind,
                    type_id,
                    atlas,
                    sim.resolved_terrain.as_ref().unwrap(),
                    camera_pos,
                    viewport,
                    axis,
                    &removed_order
                )
                .is_none()
            );
        }
    }
}
