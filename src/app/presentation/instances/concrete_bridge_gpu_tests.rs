//! Physical Anytown publication through the ordinary overlay owner and GPU.
//! Run the ignored test with RA2_DIR and optional VERA20K_ANYTOWN_MAP. Optional
//! VERA20K_ANYTOWN_RENDER_OUTPUT retains PNGs and source/readback hashes.
//!
//! Native state authority: tools/spatial_oracle/anytown_damage/navigation.json.gz
//! (loaded, first_damage, collapse, repair). Physical SHP bytes establish the
//! source frames/stencils. This is a production publication-to-pixels witness,
//! not a gamemd whole-frame, scene timing, shroud-composition or RGB oracle.
use super::{CellOverlayInputs, build_cell_overlay_instances};
use crate::app::presentation::lighting::MatchLighting;
use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::assets::shp_file::ShpFile;
use crate::headless_scenario::HeadlessScenario;
use crate::map::lighting::{CellLightGrid, parse_lighting};
use crate::map::overlay::OverlayEntry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::render::batch::{BatchRenderer, InstanceBufferPool, SpriteInstance};
use crate::render::overlay_atlas::{OverlayAtlas, OverlaySpriteKey, build_overlay_atlas_on_device};
use crate::render::tactical_draw_plan::RenderZPolicy;
use crate::render::terrain_draw_gpu_tests::{Gpu, camera, clear, encoded};
use crate::rules::ini_parser::IniFile;
use crate::sim::intern::InternedId;
use crate::sim::vision::FogState;
use crate::util::sha256::sha256_hex;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

const SIZE: [u32; 2] = [256, 160];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const CENTER: (u16, u16) = (87, 54);

struct Probe {
    gpu: Gpu,
    batch: BatchRenderer,
    atlas: OverlayAtlas,
    assets: AssetManager,
    palette: Palette,
    rules_ini: IniFile,
    terrain: ResolvedTerrainGrid,
    entries: Vec<OverlayEntry>,
    names: BTreeMap<u8, String>,
    lighting: MatchLighting,
    camera: [f32; 2],
    readbacks: BTreeMap<String, Vec<u8>>,
    receipts: Vec<Value>,
}

impl Probe {
    fn new(scene: &HeadlessScenario) -> Self {
        let root = PathBuf::from(std::env::var_os("RA2_DIR").expect("physical retail root"));
        let mut assets = AssetManager::new(&root).unwrap();
        let mode = IniFile::from_bytes(assets.get_ref("MPBattleMD.ini").unwrap()).unwrap();
        let (_, rules_ini, _, _) = crate::app::loading::init_helpers::load_rules_with_merged_ini(
            &assets,
            Some(&mode),
            Some(&scene.map.ini),
        )
        .unwrap();
        let theater = crate::map::theater::load_theater(&mut assets, &scene.map.header.theater)
            .expect("activate the physical map's theater before name lookup");
        assert_eq!(scene.map.header.theater, "TEMPERATE");
        let entries: Vec<_> = scene
            .map
            .overlays
            .iter()
            .filter(|entry| entry.ry == CENTER.1 && (86..=88).contains(&entry.rx))
            .cloned()
            .collect();
        assert_eq!(entries.len(), 3, "exact authored three-cell row");
        for entry in &entries {
            assert_eq!(
                (entry.overlay_id, entry.frame),
                (216, (entry.rx - 86) as u8)
            );
        }
        let rules = &scene.runtime.resources.rules;
        let registry = &scene.runtime.resources.overlay_registry;
        let mut names = BTreeMap::new();
        for entry in &entries {
            names.insert(
                entry.overlay_id,
                crate::render::overlay_assets::resolve_overlay_name_for_render(
                    registry,
                    entry.overlay_id,
                )
                .unwrap(),
            );
        }
        crate::app::frontend::skirmish::preregister_runtime_overlay_names(
            registry,
            &rules.crate_rules,
            &mut names,
        );
        let gpu = Gpu::new();
        let batch = BatchRenderer::new_with_device(&gpu.device, &gpu.queue, FORMAT);
        let atlas = build_overlay_atlas_on_device(
            &gpu.device,
            &gpu.queue,
            &batch,
            &entries,
            &[],
            &assets,
            &theater.iso_palette,
            &theater.unit_palette,
            &theater.tiberium_palette,
            theater.extension,
            &scene.map.header.theater,
            registry,
            &rules.tiberium_types,
            &rules.crate_rules,
            &rules_ini,
            &rules.art_registry,
            None,
        )
        .expect("production preloads the physical low-bridge variants");
        let terrain = scene.sim().resolved_terrain.as_ref().unwrap().clone();
        let mut lighting = MatchLighting::default();
        lighting.install(
            CellLightGrid::new(),
            parse_lighting(&scene.map.ini),
            2,
            Some((&terrain, scene.sim(), rules)),
        );
        let level = scene.runtime.resources.height_map[&CENTER];
        let point = crate::map::terrain::iso_to_screen(CENTER.0, CENTER.1, level);
        let camera = [
            point.0 + 30.0 - SIZE[0] as f32 / 2.0,
            point.1 + 15.0 - SIZE[1] as f32 / 2.0,
        ];
        Self {
            gpu,
            batch,
            atlas,
            assets,
            palette: theater.iso_palette,
            rules_ini,
            terrain,
            entries,
            names,
            lighting,
            camera,
            readbacks: BTreeMap::new(),
            receipts: Vec::new(),
        }
    }

    fn instances(
        &self,
        scene: &HeadlessScenario,
        visibility: Option<(InternedId, &FogState)>,
        camera: [f32; 2],
    ) -> (Vec<SpriteInstance>, Vec<RenderZPolicy>) {
        let mut instances = Vec::new();
        let mut policies = Vec::new();
        build_cell_overlay_instances(
            &CellOverlayInputs {
                // Keep the initial map identities throughout; the production
                // owner must select the live grid's changed identity and data.
                entries: &self.entries,
                atlas: &self.atlas,
                names: &self.names,
                live_grid: scene.sim().overlay_grid.as_ref(),
                registry: Some(&scene.runtime.resources.overlay_registry),
                tiberium_types: Some(&scene.runtime.resources.rules.tiberium_types),
                terrain: Some(&self.terrain),
                heights: &scene.runtime.resources.height_map,
                lighting: self.lighting.grid(),
                visibility,
                camera,
                viewport: SIZE.map(|value| value as f32),
                origin_y: 0.0,
                world_height: f32::from(self.terrain.height()) * 30.0,
            },
            &mut instances,
            &mut policies,
        );
        (instances, policies)
    }

    fn observe(&mut self, phase: &str, scene: &HeadlessScenario) {
        let sim = scene.sim();
        let before = sim.state_hash();
        let rules = &scene.runtime.resources.rules;
        self.lighting.refresh(&self.terrain, sim, rules, 2);
        // Executed native navigation packet, case row87,54 and ordered overlay
        // stores in first_damage/collapse/repair. Native MapGen chooses215 here.
        let id = match phase {
            "loaded" => 216,
            "damaged" => 220,
            "collapsed" => 232,
            "repaired" => 215,
            _ => panic!("unknown concrete stage {phase}"),
        };
        for x in 86..=88 {
            let live = sim.overlay_grid.as_ref().unwrap().cell(x, CENTER.1);
            let cell = sim
                .resolved_terrain
                .as_ref()
                .unwrap()
                .cell(x, CENTER.1)
                .unwrap();
            assert_eq!(
                (live.overlay_id, live.overlay_data),
                (Some(id), (x - 86) as u8),
                "{phase}"
            );
            assert_eq!(cell.bridge_facts.overlay_id, live.overlay_id, "{phase}");
            assert!(
                !cell.bridge_facts.has_structural_bridge(),
                "ground-surface family"
            );
            assert_eq!((cell.level, cell.final_tile_index, cell.final_sub_tile), {
                let initial = self.terrain.cell(x, CENTER.1).unwrap();
                (
                    initial.level,
                    initial.final_tile_index,
                    initial.final_sub_tile,
                )
            });
        }
        let name = &self.names[&id];
        assert_eq!(
            rules
                .art_registry
                .resolve_overlay_image_id(name, &self.rules_ini),
            *name
        );
        let filename = format!("{}.tem", name.to_ascii_lowercase());
        let asset = self
            .assets
            .resolve_ref(&filename)
            .expect("active theater lookup");
        assert!(
            asset
                .source_archive
                .to_ascii_lowercase()
                .contains("temperat.mix")
        );
        let source = json!({"file":filename,"archive":asset.source_archive,
            "entry_id":format!("0x{:08X}",asset.entry_id as u32),
            "sha256":sha256_hex(asset.bytes),"bytes":asset.bytes.len()});
        let shp = ShpFile::from_bytes(asset.bytes).unwrap();
        assert_eq!((shp.width, shp.height, shp.frames.len()), (180, 120, 6));
        let frame = &shp.frames[1];
        for frame_index in [0, 2] {
            assert_eq!(
                (
                    shp.frames[frame_index].frame_width,
                    shp.frames[frame_index].frame_height
                ),
                (0, 0)
            );
            assert!(
                self.atlas
                    .get(&OverlaySpriteKey {
                        name: name.clone(),
                        frame: frame_index as u8
                    })
                    .is_none(),
                "flank must not substitute populated body art"
            );
        }
        if phase == "collapsed" {
            assert!(
                shp.frames
                    .iter()
                    .all(|frame| frame.frame_width == 0 && frame.frame_height == 0)
            );
            assert!(
                self.atlas
                    .get(&OverlaySpriteKey {
                        name: name.clone(),
                        frame: 1
                    })
                    .is_none()
            );
        } else {
            assert_eq!(
                (
                    frame.frame_x,
                    frame.frame_y,
                    frame.frame_width,
                    frame.frame_height
                ),
                (30, 19, 120, 70)
            );
            assert!(
                self.atlas
                    .get(&OverlaySpriteKey {
                        name: name.clone(),
                        frame: 1
                    })
                    .is_some()
            );
        }

        let owner = sim.session.current_house.unwrap();
        let mut fog = FogState {
            width: self.terrain.width(),
            height: self.terrain.height(),
            ..Default::default()
        };
        assert!(
            self.instances(scene, Some((owner, &fog)), self.camera)
                .0
                .is_empty(),
            "unrevealed row draws nothing"
        );
        fog.reveal_cells_for_owner(owner, self.entries.iter().map(|entry| (entry.rx, entry.ry)));
        assert!(
            self.instances(scene, Some((owner, &fog)), [1_000_000.0; 2])
                .0
                .is_empty(),
            "camera rejection uses production admission"
        );
        let (instances, policies) = self.instances(scene, Some((owner, &fog)), self.camera);
        assert_eq!(
            instances.len(),
            usize::from(phase != "collapsed"),
            "{phase}: only center frame1 owns art"
        );
        assert!(
            policies
                .iter()
                .all(|policy| *policy == RenderZPolicy::ReadWrite)
        );
        if let Some(instance) = instances.first() {
            assert_eq!(
                [
                    instance.position[0] - self.camera[0],
                    instance.position[1] - self.camera[1]
                ],
                [68.0, 39.0]
            );
            assert_eq!(instance.size, [120.0, 70.0]);
        }
        let mut view = camera(SIZE);
        view.camera_pos = self.camera;
        self.batch.write_camera(&self.gpu.queue, view);
        let color = self.gpu.target(SIZE, FORMAT);
        let depth = self.gpu.target(SIZE, wgpu::TextureFormat::Depth32Float);
        let cv = color.create_view(&Default::default());
        let dv = depth.create_view(&Default::default());
        let mut pool = InstanceBufferPool::new();
        pool.upload_on_device(&self.gpu.device, &self.gpu.queue, "overlay", &instances);
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        clear(
            &mut encoder,
            &cv,
            &dv,
            65535,
            wgpu::LoadOp::Clear(wgpu::Color::WHITE),
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Physical concrete bridge ordinary overlay submission"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &cv,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &dv,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            crate::app::presentation::render::draw_pooled_overlay_bodies(
                &mut pass,
                &self.batch,
                &pool,
                Some(&self.atlas),
                "overlay",
                &policies,
            );
        }
        let reads = [
            self.gpu.read(&mut encoder, &color),
            self.gpu.read(&mut encoder, &depth),
        ];
        let output = self.gpu.finish(encoder, &reads, SIZE);
        let mut opaque = 0usize;
        for y in 0..SIZE[1] as usize {
            for x in 0..SIZE[0] as usize {
                let index =
                    if !instances.is_empty() && (68..188).contains(&x) && (39..109).contains(&y) {
                        frame.pixels[(y - 39) * 120 + x - 68]
                    } else {
                        0
                    };
                let offset = (y * SIZE[0] as usize + x) * 4;
                let expected = if index == 0 {
                    [255; 4]
                } else {
                    opaque += 1;
                    let rgb = self.palette.colors[usize::from(index)];
                    // Existing native-backed LightConvert owner supplies the
                    // expected source color. Clear tactical A is 127
                    // (006D3F9F), matching palette_light.wgsl; A=0 selects the
                    // black row. This is GPU replay consistency, not an
                    // independent gamemd whole-scene RGB oracle.
                    encoded(
                        instances[0]
                            .palette_light
                            .rgb565([rgb.r, rgb.g, rgb.b], index, 127),
                        FORMAT,
                    )
                };
                assert_eq!(
                    &output[0][offset..offset + 4],
                    expected,
                    "{phase} pixel{x},{y}"
                );
                let z = crate::render::native_z::stored_z(f32::from_le_bytes(
                    output[1][offset..offset + 4].try_into().unwrap(),
                ));
                assert_eq!(
                    z != 65535,
                    index != 0,
                    "{phase}: source stencil owns color/depth at{x},{y}"
                );
            }
        }
        assert_eq!(
            opaque,
            frame.pixels.iter().filter(|&&index| index != 0).count()
        );
        assert_eq!(
            sim.state_hash(),
            before,
            "rendering cannot mutate simulation"
        );
        let receipt = json!({"phase":phase,"frame":sim.session.binary_frame,
            "overlay":id,"overlay_data":[0,1,2],"source":source,
            "source_indices_sha256":sha256_hex(&frame.pixels),
            "instances":instances.len(),"opaque_pixels":opaque,
            "color_sha256":sha256_hex(&output[0]),"depth_sha256":sha256_hex(&output[1])});
        eprintln!("ANYTOWN_RENDER {receipt}");
        self.receipts.push(receipt);
        self.readbacks.insert(phase.to_owned(), output[0].clone());
    }

    fn finish(self) {
        assert_ne!(self.readbacks["loaded"], self.readbacks["damaged"]);
        assert_ne!(self.readbacks["damaged"], self.readbacks["collapsed"]);
        assert_ne!(self.readbacks["collapsed"], self.readbacks["repaired"]);
        if let Some(path) = std::env::var_os("VERA20K_ANYTOWN_RENDER_OUTPUT") {
            let path = PathBuf::from(path);
            std::fs::create_dir_all(&path).unwrap();
            for (phase, bgra) in self.readbacks {
                let mut rgba = bgra;
                for pixel in rgba.chunks_exact_mut(4) {
                    pixel.swap(0, 2);
                }
                image::save_buffer(
                    path.join(format!("{phase}.png")),
                    &rgba,
                    SIZE[0],
                    SIZE[1],
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            serde_json::to_writer_pretty(std::fs::File::create(path.join("receipt.json")).unwrap(),&json!({
                "size":SIZE,"camera":self.camera,"stages":self.receipts,
                "scope":"Physical Anytown row86..88,y54, real damage and ordinary Engineer repair, retained physical atlas, shared ordinary-overlay builder, upload/submission and every color/depth pixel. Revealed/hidden and camera-rejection controls. No original composited frame, native timing or full-scene rendering parity claim."
            })).unwrap();
        }
    }
}

#[test]
#[ignore = "requires physical Anytown, retail assets and an offscreen GPU"]
fn retail_concrete_bridge_publication_reaches_gpu_pixels() {
    let mut probe = None;
    crate::sim::world::bridge_test_evidence::visit_anytown_concrete_stages(|phase, scene| {
        probe
            .get_or_insert_with(|| Probe::new(scene))
            .observe(phase, scene);
    });
    probe.expect("all four physical stages executed").finish();
}
