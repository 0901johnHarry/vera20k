//! Physical AEGIS type -> production model/raster -> parent waterline input.
use super::*;
use crate::render::sinking::SinkingWaterlines;
use crate::util::sha256::sha256_hex;

#[test]
#[ignore = "requires physical Shrapnel map, retail AEGIS/HVA/VPL and original draw observations"]
fn retail_aegis_raster_and_parent_waterline_match_original_ship_draw() {
    let retail = std::path::PathBuf::from(std::env::var_os("RA2_DIR").unwrap());
    let map = std::env::var("VERA20K_SHRAPNEL_MAP").unwrap_or_else(|_| "XShrapnel.MAP".into());
    let scene = crate::headless_scenario::load(&retail, &map, 0x0B21_D6E5).unwrap();
    let rules = &scene.runtime.resources.rules;
    let assets = crate::assets::asset_manager::AssetManager::new(&retail).unwrap();
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/spatial_oracle/naval_draw_bounds.json"
    ))
    .unwrap();
    for (name, provenance) in corpus["files"].as_object().unwrap() {
        let bytes = assets.get_ref(name).unwrap();
        assert_eq!(
            bytes.len() as u64,
            provenance["bytes"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(
            sha256_hex(bytes),
            provenance["sha256"].as_str().unwrap(),
            "{name}"
        );
    }
    let object = rules.object("AEGIS").unwrap();
    assert_eq!(
        object.image,
        corpus["reader"]["layers"][2]["after"].as_str().unwrap()
    );
    assert!(rules.art_registry.get(&object.image).unwrap().voxel);
    assert!(!object.has_turret);
    let model = crate::render::unit_atlas::UnitModel::load(
        &assets,
        "AEGIS",
        Some(rules),
        Some(&rules.art_registry),
    )
    .unwrap();
    let vpl = crate::assets::vpl_file::VplFile::from_bytes(assets.get_ref("VOXELS.VPL").unwrap())
        .unwrap();
    for row in corpus["cases"].as_array().unwrap() {
        let facing = row["input"]["facing_step32"].as_u64().unwrap() as u8 * 8;
        let key = UnitSpriteKey {
            type_id: "AEGIS".into(),
            facing,
            layer: VxlLayer::Composite,
            frame: 0,
            slope_type: 0,
        };
        let (sprite, native_draw_bounds) = model.render(&key, Some(&vpl), None, &mut None).unwrap();
        let rect: [i32; 6] =
            std::array::from_fn(|i| row["native_rect"][i].as_i64().unwrap() as i32);
        assert_eq!(
            native_draw_bounds,
            Some([rect[0], rect[1], rect[4], rect[5]]),
            "facing{facing}"
        );
        let mut canonical = vec![0; 65536];
        let left = sprite.offset_x as i32 + rect[2] - rect[0];
        let top = sprite.offset_y as i32 + rect[3] - rect[1];
        for y in 0..sprite.height as i32 {
            for x in 0..sprite.width as i32 {
                let byte = sprite.palette_indices[(y as u32 * sprite.width + x as u32) as usize];
                if byte != 0 {
                    canonical[((top + y) * 256 + left + x) as usize] = byte;
                }
            }
        }
        assert_eq!(
            sha256_hex(&canonical),
            row["pixels_sha256"].as_str().unwrap(),
            "facing{facing} physical raster"
        );
        let entry = UnitSpriteEntry {
            uv_origin: [0.0; 2],
            uv_size: [1.0; 2],
            pixel_size: [sprite.width as f32, sprite.height as f32],
            offset_x: sprite.offset_x,
            offset_y: sprite.offset_y,
            page: 0,
            native_draw_bounds,
        };
        // Original packet supplies camera1000 and draw anchor200. The app
        // composes their world row first; no source-crop Y participates.
        let bounds = composite_draw_bounds([(entry, [0.0, 1200.0])]);
        let [_, y, _, height] = bounds.native.unwrap();
        let mut cache = SinkingWaterlines::default();
        assert_eq!(cache.unit_draw(1, true, y + height), None);
        let expected = row["first_clip"]["waterline_after"].as_i64().unwrap() as i16;
        assert_eq!(cache.saved(), [(1, expected)]);
        assert_eq!(cache.unit_draw(1, true, y + height + 50), Some(expected));
        eprintln!(
            "AEGIS facing{facing} native rectangle {rect:?}, raster bytes match, retained waterline{expected}"
        );
    }
}
