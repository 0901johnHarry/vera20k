//! A building's mission work in `BuildingClass::Update` (`0x0043FB20`),
//! around and inside `TechnoClass::AI_Update`:
//!
//! - the ready check before the Techno AI (`0x0043FE27..0x0043FE54`) and the
//!   one after it (`0x0043FF91..0x0043FFB4`): with `+0x6DD` set, a successful
//!   Commence of the queued mission clears the byte
//!   ([`Simulation::mission_building_ready_commence`]). The first also needs
//!   BState (`+0x534`) out of 0, the construction body; a build-up's frames
//!   run in the late region (`sim::building_construction`) and never reach
//!   either check, and a sale past its stage 1 refuses every queue, so VERA
//!   has no state in which the BState test decides.
//! - `MissionClass::AI` (`0x005B3060`, called at `0x006FA655`): when the
//!   dispatch timer (`+0xC8`) is due and Health is above zero, the current
//!   mission's handler runs and its return is the next delay
//!   (`0x005B32F2..0x005B3302`). Guard and Sticky run Mission_Guard
//!   (`0x004496B0`, dispatch table `0x005B34E8`), Area Guard too (`0x00449A40`
//!   jumps to it), Attack runs Mission_Attack (`0x0044ACF0`), Selling runs
//!   Sell ([`Simulation::visit_building_down`]), and a mission the building
//!   has no handler for, or none, a MissionClass stub (450 frames).
//! - the range drop at the end of the Update (`0x00440378..0x004403C6`).
//!
//! Mission_Attack's FireAt (`0x0044B6D0`) is VERA's combat emission: the OK
//! arm asks the combat phase for the shot
//! ([`crate::sim::combat::FireRequests::buildings`]), which emits it without
//! asking GetFireError again. A building fires no ordinary shot without that
//! request.
//!
//! Evidence: `tools/spatial_oracle/building_guard_attack.json`, replayed in
//! the tests below: Mission_Guard's returns and Scenario draws per arm,
//! Mission_Attack's return and actions per fire-error code,
//! BuildingClass::SetTarget's decisions, Unlimbo's mission (`0x0044D6A0`) and
//! the dispatch cadence of Update's mission pieces run natively frame by
//! frame (the shot every ROF + 1 frames for an even ROF, every ROF frames
//! for an odd one).
//!
//! RESIDUALS, each with its later owner:
//! - The frame position of a building's shot. Native fires FireAt inside the
//!   building's own Logic visit and runs ProcessDelayedFire later in the same
//!   Update (`0x004400F4`); VERA emits both in the combat phase after the
//!   Logic pass, like every class's FireAt (the draw-order residual
//!   `docs/plans/2026-09-21-combat-parity.md` records for all of them).
//!   Trigger: every building shot. Effect: the shot's Scenario draws
//!   (GetROF's `RandomRanged(0, 2)` at `0x006FD09E`, the bullet's own) and
//!   its rearm and ammo writes follow the Logic visits of the objects after
//!   the building, not precede them; the bullet still takes its first AI at
//!   the tail of the same pass ([`Simulation::visit_combat_tail`]).
//!   Frequency: every frame a building fires while a later object draws.
//!   Downstream: the Scenario stream's order in that frame. Later owner:
//!   FireAt moving into each object's Logic visit.
//! - Gattling (D11): Mission_Guard's stage update (`0x004496C1..0x004496DF`),
//!   Mission_Attack's charge and decay calls (`0x70DE70`, `0x70E000`) and
//!   their `+0xC4` resets. Trigger: every `[YAGGUN]` visit. Effect: its
//!   stage stays at 0. Frequency: every Gattling Cannon engagement.
//!   Downstream: its rate of fire and weapon stage.
//! - Prism (D7): the PrismType arm (`0x0044B310..0x0044B62B`, recruiting a
//!   charged tower or arming its own delayed shot) and `+0x664 = 0` on the
//!   null-target and drop tails. VERA fires a Prism tower at once. Trigger:
//!   every `[ATESLA]` shot. Effect: no forwarding and no charge delay.
//! - The docking radio of an unarmed building's Guard (the depot/airfield
//!   docking chain). Status 1 of a `UnitRepair=` (`+0x16A9`), `UnitReload=`
//!   (`+0x16AA`) or `Bunker=` (`+0x16AB`) type walks its radio contacts
//!   (`0x00449817..0x00449918`): one on Enter under 64 leptons away that answers
//!   ROGER to message `0x13` gets the building a queued Repair and the
//!   handler returns 1 without its `RandomRanged(0, 2)`
//!   (`0x00449942..0x0044995C`). A `UnitReload=` type also sends its contact
//!   `0x1D`, then `0x13`, and queues Repair on ROGER before its usual draw
//!   (`0x00449970..0x004499B5`). VERA's handler stays on Guard and draws.
//!   Trigger: a Service Depot's repair, an airfield's reload, a Tank Bunker
//!   entry (GADEPT, NADEPT, YADEPT, GAAIRC, AMRADR, NATBNK). Effect: the
//!   building's queued mission and the Scenario draw count differ while the
//!   contact docks, so every later Scenario draw differs from native.
//!   Frequency: common in ordinary play. The contacts' answers to `0x13` and
//!   `0x1D`, and whether that Repair commences (the unarmed arm sets no
//!   `+0x6DD`), belong to that chain with BuildingClass::Mission_Repair
//!   (`0x0044B780`); the dock owners run the repair and reload meanwhile.
//!   The WeaponsFactory's ClearBibArea (`0x00449540`) after the walk is
//!   dormant: no retail WeaponsFactory type clears HasStupidGuardMode.
//! - The `+0x148` count of the OK and REARM arms (`0x0044B713`,
//!   `0x0044B23C`), a turret-animation counter only presentation reads.
//! - Status 0's `Begin_Mode(1)` (`0x0044995D`): the idle body, presentation.
//! - Dormant with retail data: the SAM arm (`0x0044AD07`, `SAM=` unset), the
//!   upgrade arm (`0x0044B2BC`, no `PowersUpBuilding=`), Mission_Guard's
//!   SuperWeapon gate (`0x00449716..0x00449753`, no armed type sets
//!   `SuperWeapon=`), the waypoint-planning hook (`0x0044AFB1`, VERA has no
//!   planning mode) and BuildingClass::SetTarget's TickTank/Artillary
//!   undeploy (`0x00443C07..0x00443C54`, neither key set).
//! - A turretless building's `+0x388` (its body) takes Mission_Attack's
//!   Set_Desired natively; VERA has no body facing for a building, so the
//!   FLH of its later shots leaves along the authored facing (a few
//!   pixels).
//! - `+0x6DD` has two more homes, `BuildingUp::done` and `BuildingDown::done`
//!   (`sim::components`), each written and read only by its own build-up or
//!   sale; a finished build-up queues and commences Guard itself
//!   (`Simulation::tick_building_up`) where the ready check after the Techno
//!   AI would. No player-visible effect while each byte has one reader.
//!   Later owner: Mission_Construction's frames moving into this dispatch.
//!
//! ## Dependency rules
//! - Part of sim/; sim/ never depends on render/, ui/, sidebar/, audio/, net/.

use super::target_scan::{
    can_fire_at, fire_error_with_overlay, select_weapon, weapon_at_index_for,
};
use super::{ObjectAiCtx, mission_handlers_run};
use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::building_art::requested_damage_state;
use crate::sim::combat::TargetKind;
use crate::sim::combat::combat_weapon::{self, WeaponSlot};
use crate::sim::combat::fire_error::FireError;
use crate::sim::game_entity::PendingBuildingFire;
use crate::sim::mission::authority::LiveReadyInputProvider;
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::world::Simulation;

/// The MissionClass stubs a building's table holds for every mission it has
/// no handler of its own for (`0x005B2E10..0x005B2FC0`; its Capture,
/// Sabotage and Harvest slots jump to them, `0x0044B760`, `0x0044B770`):
/// 450 frames.
const DEFAULT_MISSION_DELAY: i32 = 450;
/// An unarmed HasStupidGuardMode building's Guard return (`0x004497F4`).
const STUPID_GUARD_DELAY: i32 = 100;
/// The building anim slots of `ActiveAnim=` and `SpecialAnim=` (Building
/// `+0x55C` + 4 x slot; their names at BuildingType `+0x1018` and `+0x11F4`).
const ACTIVE_ANIM_SLOT: u8 = 3;
const SPECIAL_ANIM_SLOT: u8 = 10;

/// One of Update's two ready checks (module doc).
pub(super) fn ready_commence(sim: &mut Simulation, id: u64) {
    if sim
        .substrate
        .entities
        .get(id)
        .is_none_or(|entity| entity.building_up.is_some())
    {
        return;
    }
    let now = sim.session.binary_frame;
    let _ = sim.mission_building_ready_commence(id, now);
}

/// `MissionClass::AI` (`0x005B3060`) for a building (module doc).
pub(super) fn dispatch(
    sim: &mut Simulation,
    id: u64,
    rules: Option<&RuleSet>,
    ctx: ObjectAiCtx<'_>,
) {
    let Some(entity) = sim.substrate.entities.get(id) else {
        return;
    };
    // A build-up's Construction mission belongs to its late-region owner.
    if entity.building_up.is_some() {
        return;
    }
    let current = entity.mission.current().known();
    if current == Some(MissionType::Selling) {
        // Sell returns 1 on every visit, so its dispatch is due on every frame
        // after the one that commenced it; the visit owns that frame test.
        sim.visit_building_down(id, rules, ctx.overlay_registry);
        return;
    }
    let Some(rules) = rules else {
        return;
    };
    let now = sim.session.binary_frame;
    if !entity.mission.dispatch_timer().due(now) || !mission_handlers_run(sim, id) {
        return;
    }
    let delay = match current {
        Some(MissionType::Guard | MissionType::Sticky | MissionType::AreaGuard) => {
            mission_guard(sim, id, rules)
        }
        Some(MissionType::Attack) => mission_attack(sim, id, rules, ctx),
        // BuildingClass's own Unload (`0x0044D880`), Construction
        // (`0x00449A50`), Repair (`0x0044B780`), Missile (`0x0044C980`) and
        // Open (`0x0044E440`) keep their existing owners.
        Some(
            MissionType::Unload
            | MissionType::Construction
            | MissionType::Repair
            | MissionType::Missile
            | MissionType::Open,
        ) => return,
        // Every other slot of the building's table, and no mission (above
        // `0x1F`, `0x005B30BB`), is a MissionClass stub.
        _ => DEFAULT_MISSION_DELAY,
    };
    if let Some(entity) = sim.substrate.entities.get_mut(id) {
        entity.mission.write_dispatch_epilogue(now as i32, delay);
    }
}

/// `Queue_Mission(mission, false)` then Commence, as both handlers switch
/// missions (`0x00449792`/`0x0044979C`, `0x0044B13E`/`0x0044B148`).
fn queue_and_commence(sim: &mut Simulation, id: u64, mission: MissionType, rules: &RuleSet) {
    let now = sim.session.binary_frame;
    let readiness = LiveReadyInputProvider { rules };
    let _ = sim.mission_queue_exact(id, MissionId::from_known(mission), 0, now, &readiness);
    let _ = sim.mission_commence_exact(id, now);
}

/// `ftol(rate x 900) + RandomRanged(0, 2)` on the Scenario stream, the rate
/// being the MissionControl entry of the building's current mission
/// (`0x005B3A00`).
fn rate_delay(sim: &mut Simulation, frames: u32, multiplier: i32) -> i32 {
    let base = frames.min(i32::MAX as u32) as i32;
    let jitter = sim.scenario_rng.next_range_u32_inclusive(0, 2) as i32;
    base.wrapping_mul(multiplier).wrapping_add(jitter)
}

/// `BuildingClass::Mission_Guard` (`0x004496B0`).
fn mission_guard(sim: &mut Simulation, id: u64, rules: &RuleSet) -> i32 {
    let Some(entity) = sim.substrate.entities.get(id) else {
        return 0;
    };
    let Some(obj) = rules.object(sim.interner.resolve(entity.type_ref())) else {
        return 0;
    };
    let current = entity
        .mission
        .current()
        .known()
        .unwrap_or(MissionType::Guard);
    // vt+0x2AC, `BuildingClass::Is_Armed 0x00458DB0`.
    if combat_weapon::is_armed(entity, obj) {
        let empty_garrison = obj.can_be_occupied
            && entity
                .passenger_role
                .cargo()
                .is_none_or(|cargo| cargo.is_empty());
        let has_target = entity.attack_target.is_some();
        if let Some(entity) = sim.substrate.entities.get_mut(id) {
            // `0x00449701`.
            entity.mission_leaf.set_building_ready_latch(1);
        }
        if !obj.emp_pulse_cannon && !empty_garrison && has_target {
            queue_and_commence(sim, id, MissionType::Attack, rules);
            return 1;
        }
        // `0x004497AF..0x004497DA`: the AARate delay.
        let frames = rules.mission_control.aa_rate_frames(current);
        return rate_delay(sim, frames, 1);
    }
    if obj.has_stupid_guard_mode {
        return STUPID_GUARD_DELAY;
    }
    let status = entity.mission.handler_state();
    if status == 0
        && let Some(entity) = sim.substrate.entities.get_mut(id)
    {
        // `0x0044995D..0x00449966`: the idle body, then status 1.
        entity.mission.set_handler_state(1);
    }
    // `0x004499BB..0x00449A36`: Rate for a depot, three times it otherwise.
    let frames = rules.mission_control.rate_frames(current);
    rate_delay(sim, frames, if obj.unit_repair { 1 } else { 3 })
}

/// `BuildingClass::Mission_Attack` (`0x0044ACF0`).
fn mission_attack(sim: &mut Simulation, id: u64, rules: &RuleSet, ctx: ObjectAiCtx<'_>) -> i32 {
    let Some((target, weapon)) = attack_prelude(sim, id, rules) else {
        return 1;
    };
    let mut code = fire_error_with_overlay(sim, rules, id, target, weapon, ctx.overlay_registry);
    if code == FireError::Facing && voxel_turret_snaps(sim, id, rules, target) {
        code = fire_error_with_overlay(sim, rules, id, target, weapon, ctx.overlay_registry);
    }
    attack_arm(sim, id, rules, target, weapon, code)
}

/// Mission_Attack up to its GetFireError (`0x0044B00F`): with no target, the
/// null-target tail (`0x0044AF86..0x0044AFE0`), which answers `None`;
/// otherwise SelectWeapon (`0x0044AFF2`) and `+0x6DD = 1` (`0x0044B008`),
/// answering the target and weapon.
fn attack_prelude(sim: &mut Simulation, id: u64, rules: &RuleSet) -> Option<(TargetKind, i32)> {
    let entity = sim.substrate.entities.get(id)?;
    let Some(target) = entity.attack_target.as_ref().map(|attack| attack.target) else {
        let _ = sim.assign_target_represented(id, None, Some(rules));
        if !waits(sim, id) {
            queue_and_commence(sim, id, MissionType::Guard, rules);
        }
        return None;
    };
    let weapon = select_weapon(sim, rules, id, Some(target));
    if let Some(entity) = sim.substrate.entities.get_mut(id) {
        entity.mission_leaf.set_building_ready_latch(1);
    }
    Some((target, weapon))
}

/// Mission_Attack's arm for GetFireError's `code` (the table at
/// `0x0044B728`): its actions, then its return.
fn attack_arm(
    sim: &mut Simulation,
    id: u64,
    rules: &RuleSet,
    target: TargetKind,
    weapon: i32,
    code: FireError,
) -> i32 {
    match code {
        FireError::Ok => {
            fire_arm(sim, id, rules, weapon);
            1
        }
        // `0x0044B0DE`: the drop tail.
        FireError::Ammo | FireError::Illegal | FireError::Cant | FireError::Range => {
            let _ = sim.assign_target_represented(id, None, Some(rules));
            if waits(sim, id) {
                return 1;
            }
            queue_and_commence(sim, id, MissionType::Guard, rules);
            clear_ai_counter(sim, id);
            1
        }
        // `0x0044B187` and `0x0044B1DE`.
        FireError::Facing | FireError::Rearm => {
            aim_turret(sim, id, rules, target);
            2
        }
        // `0x0044B284`: uncloak, then the `0x0044B14E` tail.
        FireError::Cloaked => {
            crate::sim::combat::world_receiver::start_uncloaking_to_fire(sim, rules, id);
            aim_turret(sim, id, rules, target);
            clear_ai_counter(sim, id);
            1
        }
        // `0x0044B24F`.
        FireError::Busy => 1,
        // Codes 4, 7 and above 10 (`0x0044B14E`).
        FireError::Rotating | FireError::Moving | FireError::MustDeploy => {
            aim_turret(sim, id, rules, target);
            clear_ai_counter(sim, id);
            1
        }
    }
}

/// `Get_Mission() == Wait` (vt+0x184, `0x0044AFBA`/`0x0044B108`): the current
/// mission, else the queued one.
fn waits(sim: &Simulation, id: u64) -> bool {
    sim.substrate.entities.get(id).is_some_and(|entity| {
        entity.mission.effective() == MissionId::from_known(MissionType::Deliberate)
    })
}

/// `+0xC4 = 0` (`0x0044B174`).
fn clear_ai_counter(sim: &mut Simulation, id: u64) {
    if let Some(entity) = sim.substrate.entities.get_mut(id) {
        entity.mission.clear_ai_counter();
    }
}

/// The OK arm (`0x0044B2BC`): a Prism tower (D7, module doc) and an ordinary
/// building shoot, an `IsAnimDelayedFire=` one arms its delayed shot
/// (`0x0044B630..0x0044B666`: `+0x714` = DelayedFireDelay, `+0x708` = the
/// weapon, `+0x704` = 1) for ProcessDelayedFire (`0x004503F0`), which native
/// runs later in this same Update (`0x004400F4`) and VERA in the combat phase
/// (module doc).
///
/// Arming also swaps the building's anims (`0x0044B66E..0x0044B6C2`): the
/// Active anim's slot 3 is emptied (`0x00451E40`) and the SpecialAnim plays
/// in slot 10, its Damaged variant at or below ConditionYellow (`0x00451890`,
/// the Tesla Coil's charge and its `Report=`). Nothing in play gives the
/// Active anim back when the SpecialAnim ends: BuildingClass's vt+0x28
/// (`0x0044E9AA`) replays it only for a `Grinding=` type, so it returns with
/// a power restore.
fn fire_arm(sim: &mut Simulation, id: u64, rules: &RuleSet, weapon: i32) {
    let Some(obj) = sim
        .substrate
        .entities
        .get(id)
        .and_then(|entity| rules.object(sim.interner.resolve(entity.type_ref())))
    else {
        return;
    };
    let prism = rules
        .general
        .prism_type
        .as_deref()
        .is_some_and(|prism_type| obj.id.eq_ignore_ascii_case(prism_type));
    let delayed_fire_delay = rules
        .art_registry
        .resolve_metadata_entry(&obj.id, &obj.image)
        .filter(|art| art.is_anim_delayed_fire)
        .map(|art| art.delayed_fire_delay);
    match delayed_fire_delay {
        Some(delay) if !prism => {
            let Some(entity) = sim.substrate.entities.get_mut(id) else {
                return;
            };
            entity.pending_building_fire = Some(PendingBuildingFire {
                remaining_ticks: delay,
                weapon_slot: if weapon == 1 {
                    WeaponSlot::Secondary
                } else {
                    WeaponSlot::Primary
                },
            });
            let damaged =
                requested_damage_state(entity.health, obj.strength, rules.general.condition_yellow);
            sim.clear_building_anim_slot(id, ACTIVE_ANIM_SLOT);
            let _ = sim.set_building_anim_slot(id, SPECIAL_ANIM_SLOT, damaged, false, 0, rules);
        }
        _ => {
            sim.fire_requests.buildings.insert(id);
        }
    }
}

/// The voxel-turret retry (`0x0044B017..0x0044B0CC`): a building with a turret
/// whose `TurretAnimIsVoxel=` is set, within one `ROT=` step of the target's
/// direction (`abs(low-byte ROT << 8)` as signed16, without FacingClass
/// SetROT's clamp; any miss at ROT 0), snaps its turret (`0x0044B0AC`) and
/// asks GetFireError again. Original decisions: building_fire_turn.json.
fn voxel_turret_snaps(sim: &mut Simulation, id: u64, rules: &RuleSet, target: TargetKind) -> bool {
    let now = sim.session.binary_frame;
    let Some(entity) = sim.substrate.entities.get(id) else {
        return false;
    };
    let Some(obj) = rules.object(sim.interner.resolve(entity.type_ref())) else {
        return false;
    };
    let (Some(barrel), true) = (
        entity.barrel_facing,
        obj.has_turret && obj.turret_anim_is_voxel,
    ) else {
        return false;
    };
    let Some(direction) = crate::sim::movement::turret::facing_toward_target(
        entity,
        &target,
        &sim.substrate.entities,
        Some(rules),
        &sim.interner,
    ) else {
        return false;
    };
    let delta = i32::from(barrel.current(now).wrapping_sub(direction) as i16);
    let rot_step = i32::from(((obj.turret_rot as u8 as u16) << 8) as i16).abs();
    if obj.turret_rot != 0 && delta.abs() > rot_step {
        return false;
    }
    if let Some(barrel) = sim
        .substrate
        .entities
        .get_mut(id)
        .and_then(|entity| entity.barrel_facing.as_mut())
    {
        barrel.snap(direction, now);
    }
    true
}

/// `turret(+0x388).Set_Desired(DirectionTo(Target))` (`0x0044B16F`,
/// `0x0044B1A8`, `0x0044B1FF`), at the type's `ROT=`.
fn aim_turret(sim: &mut Simulation, id: u64, rules: &RuleSet, target: TargetKind) {
    let now = sim.session.binary_frame;
    let Some(entity) = sim.substrate.entities.get(id) else {
        return;
    };
    let Some(rot) = rules
        .object(sim.interner.resolve(entity.type_ref()))
        .map(|obj| obj.turret_rot)
    else {
        return;
    };
    let Some(desired) = crate::sim::movement::turret::facing_toward_target(
        entity,
        &target,
        &sim.substrate.entities,
        Some(rules),
        &sim.interner,
    ) else {
        return;
    };
    if let Some(barrel) = sim
        .substrate
        .entities
        .get_mut(id)
        .and_then(|entity| entity.barrel_facing.as_mut())
    {
        barrel.set_rot(rot);
        barrel.set(desired, now);
    }
}

/// The end of `BuildingClass::Update` (`0x00440378..0x004403C6`), whatever the
/// mission: a target out of range of the weapon SelectWeapon picks for it
/// (`vt+0x3AC`, `0x006F7780`) is dropped; an aircraft only while it is low
/// (`AircraftClass 0x0041B980`, the V3 and Dreadnought rockets asking their
/// locomotor: [`crate::sim::movement::air_movement::is_low_flying`]).
pub(super) fn range_drop(sim: &mut Simulation, id: u64, rules: &RuleSet, ctx: ObjectAiCtx<'_>) {
    let Some(entity) = sim.substrate.entities.get(id) else {
        return;
    };
    // Health 0 returned before it (`0x00440072`).
    if entity.dying || entity.health.current == 0 {
        return;
    }
    let Some(target) = entity.attack_target.as_ref().map(|attack| attack.target) else {
        return;
    };
    let weapon = select_weapon(sim, rules, id, Some(target));
    if can_fire_at(sim, rules, id, target, weapon, ctx.overlay_registry) {
        return;
    }
    if let TargetKind::Entity(target_id) = target
        && sim.substrate.entities.get(target_id).is_some_and(|target| {
            target.category == EntityCategory::Aircraft
                && !crate::sim::movement::air_movement::is_low_flying(
                    target,
                    sim.resolved_terrain.as_ref(),
                    Some((rules, &sim.interner)),
                )
        })
    {
        return;
    }
    let _ = sim.assign_target_represented(id, None, Some(rules));
}

impl Simulation {
    /// `BuildingClass::SetTarget` (vt+0x3C8, `0x00443B90`)'s admission of a
    /// requested target for a building: a Selling building (`+0xAC`) or one
    /// not operational (vt+0x350, `0x004555D0`) takes none; any other keeps a
    /// target its slot-0 weapon cannot aim (none, or an `AA=` projectile,
    /// BulletType `+0x2A4`) or one in range of SelectWeapon's weapon
    /// (vt+0x3AC). Other objects admit every target here.
    ///
    /// RESIDUALS:
    /// - InRange's line of fire reads no OverlayTypeClass table here, because
    ///   `Simulation` holds none, so a wall between them is not seen. Trigger:
    ///   retaliation against, or an order onto, a target behind a wall.
    ///   Effect: the building takes a target native refuses; its next
    ///   Mission_Attack asks GetFireError with the table and drops it, and
    ///   the Guard visit that took it queued Attack without its
    ///   `RandomRanged(0, 2)`. Frequency: rare. Later owner: the overlay table
    ///   moving into `Simulation`.
    /// - The restore after a cell target expires
    ///   (`combat::combat_aoe::expire_cell_target_references`) holds only the
    ///   entity store and puts the suspended target back without this
    ///   admission. Trigger: a building whose retaliation suspended its
    ///   mission and whose later cell target expires. Effect: a Selling,
    ///   unpowered or out-of-range building keeps that target until its next
    ///   Mission_Attack or range drop. Frequency: rare. Later owner: that
    ///   restore moving onto `Simulation`.
    pub(crate) fn building_admits_target(
        &self,
        id: u64,
        requested: Option<TargetKind>,
        rules: &RuleSet,
    ) -> bool {
        let Some(entity) = self.substrate.entities.get(id) else {
            return true;
        };
        if entity.category != EntityCategory::Structure {
            return true;
        }
        if entity.mission.current().known() == Some(MissionType::Selling)
            || self.building_operational_state(id, rules) == Some(false)
        {
            return false;
        }
        let Some(target) = requested else {
            return true;
        };
        let Some(weapon0) = weapon_at_index_for(self, rules, id, Some(target), 0) else {
            return true;
        };
        let anti_air = weapon0
            .projectile
            .as_deref()
            .and_then(|projectile| rules.projectile(projectile))
            .is_some_and(|projectile| projectile.aa);
        if anti_air {
            return true;
        }
        let weapon = select_weapon(self, rules, id, Some(target));
        can_fire_at(self, rules, id, target, weapon, None)
    }

    /// The phase-level combat fixture's stand-in for a building's object-pass
    /// visit (`combat::receiver_fixture`): Mission_Attack, whose request or
    /// delayed shot the receiver then serves, as BuildingClass::Update runs it
    /// before the frame's combat. The fixture honours no mission or dispatch
    /// timer.
    #[cfg(test)]
    pub(crate) fn fixture_building_attack_visit(
        &mut self,
        id: u64,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) {
        let ctx = ObjectAiCtx {
            overlay_registry,
            ..Default::default()
        };
        let _ = mission_attack(self, id, rules, ctx);
    }
}

#[cfg(test)]
#[path = "building_missions_tests.rs"]
mod tests;
