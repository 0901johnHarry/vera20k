use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::Health;
use crate::sim::house_state::HouseState;
use crate::sim::intern::InternedId;
use crate::sim::production::ProductionCategory;
use crate::sim::runtime::SimRuntime;
use crate::sim::world::Simulation;

pub(super) fn runtime() -> (SimRuntime, InternedId) {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n0=E1\n1=E2\n[VehicleTypes]\n[AircraftTypes]\n\
         [BuildingTypes]\n0=GAPILE\n1=INVIS\n[E1]\nName=GI\nCost=200\nStrength=100\n\
         Speed=4\nTechLevel=1\nOwner=Americans\n[E2]\nName=Rocketeer\n\
         Cost=300\nStrength=100\nSpeed=4\nTechLevel=1\nOwner=Americans\n\
         [GAPILE]\nFactory=InfantryType\n[INVIS]\nInvisibleInGame=yes\n",
    ))
    .expect("external AI fixture rules");
    let mut simulation = Simulation::new();
    simulation.intern_rule_type_ids(&rules);
    let owner = simulation.interner.intern("Americans");
    simulation
        .houses
        .insert(owner, HouseState::new(owner, 0, None, false, 5000, 10));
    simulation.session.map_width = 40;
    simulation.session.map_height = 40;

    let mut runtime = SimRuntime::from_simulation(simulation);
    runtime.resources.rules = rules;
    add_entity(
        &mut runtime,
        1,
        "Americans",
        "GAPILE",
        EntityCategory::Structure,
        4,
        4,
    );
    (runtime, owner)
}

pub(super) fn add_entity(
    runtime: &mut SimRuntime,
    stable_id: u64,
    owner_name: &str,
    type_name: &str,
    category: EntityCategory,
    rx: u16,
    ry: u16,
) {
    let simulation = &mut runtime.simulation;
    let owner = simulation.interner.intern(owner_name);
    let type_ref = simulation.interner.intern(type_name);
    let mut entity = crate::sim::game_entity::GameEntity::new_at_frame_zero_for_test(
        stable_id,
        rx,
        ry,
        0,
        0,
        owner,
        Health { current: 100 },
        type_ref,
        category,
        0,
        5,
        true,
    );
    entity.lifecycle.in_limbo = false;
    entity.in_playfield = true;
    simulation.entities_mut().insert(entity);
}

pub(super) fn queue_fixture_types(runtime: &mut SimRuntime, owner: InternedId) {
    let infantry = runtime
        .simulation
        .interner
        .get("E1")
        .expect("E1 fixture type");
    let rocketeer = runtime
        .simulation
        .interner
        .get("E2")
        .expect("E2 fixture type");
    runtime
        .simulation
        .production
        .factory_shadow
        .test_enqueue_kernel(owner, ProductionCategory::Infantry, infantry, 1, 200);
    runtime
        .simulation
        .production
        .factory_shadow
        .test_enqueue_kernel(owner, ProductionCategory::Infantry, rocketeer, 2, 300);
}
