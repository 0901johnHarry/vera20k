use serde::Serialize;

use crate::map::entities::EntityCategory;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::InternedId;
use crate::sim::production::{self, BuildOption};
use crate::sim::runtime::SimRuntime;
use crate::sim::world::Simulation;

#[derive(Serialize)]
struct Observation {
    frame: u32,
    owner: String,
    credits: i32,
    map_width: u16,
    map_height: u16,
    production_queue: Vec<QueueItem>,
    production_options: Vec<ProductionOption>,
    owned_entities: Vec<EntityFact>,
    visible_enemies: Vec<EntityFact>,
}

#[derive(Serialize)]
struct QueueItem {
    type_id: String,
    queue_category: &'static str,
    state: &'static str,
    progress: u16,
}

#[derive(Serialize)]
struct ProductionOption {
    type_id: String,
    queue_category: &'static str,
    cost: i32,
    enabled: bool,
}

#[derive(Serialize)]
struct EntityFact {
    id: u64,
    type_id: String,
    category: &'static str,
    rx: u16,
    ry: u16,
    health: i32,
    max_health: i32,
}

pub(super) fn build_observation(runtime: &SimRuntime, owner: InternedId) -> serde_json::Value {
    let simulation = &runtime.simulation;
    let rules = &runtime.resources.rules;
    let owner_name = simulation.interner.resolve(owner);

    let production_queue = production::queue_view_for_owner(simulation, rules, owner_name)
        .into_iter()
        .map(|item| QueueItem {
            type_id: simulation.interner.resolve(item.type_id).to_string(),
            queue_category: item.queue_category.label(),
            state: item.state.label(),
            progress: item.progress,
        })
        .collect();

    let mut options = production::build_options_for_owner(simulation, rules, owner_name);
    options.sort_by_key(|option| {
        (
            option.queue_category,
            simulation
                .interner
                .resolve(option.type_id)
                .to_ascii_uppercase(),
        )
    });
    let production_options = options
        .into_iter()
        .map(|option| production_option(simulation, option))
        .collect();

    let mut owned_entities = Vec::new();
    let mut visible_enemies = Vec::new();
    for (stable_id, entity) in simulation.entities().iter_sorted() {
        if !entity.is_active() || !entity.is_alive() || entity.lifecycle.in_limbo {
            continue;
        }
        if entity.owner() == owner {
            owned_entities.push(entity_fact(
                simulation,
                stable_id,
                entity,
                rules.object(simulation.interner.resolve(entity.type_ref())),
            ));
        } else if is_visible_enemy(simulation, rules, owner, entity) {
            visible_enemies.push(entity_fact(
                simulation,
                stable_id,
                entity,
                rules.object(simulation.interner.resolve(entity.type_ref())),
            ));
        }
    }

    serde_json::to_value(Observation {
        frame: simulation.session.binary_frame,
        owner: owner_name.to_string(),
        credits: simulation
            .houses
            .get(&owner)
            .map_or(0, |house| house.economy.credits),
        map_width: simulation.session.map_width,
        map_height: simulation.session.map_height,
        production_queue,
        production_options,
        owned_entities,
        visible_enemies,
    })
    .expect("external AI observations contain only JSON-compatible facts")
}

fn production_option(simulation: &Simulation, option: BuildOption) -> ProductionOption {
    ProductionOption {
        type_id: simulation.interner.resolve(option.type_id).to_string(),
        queue_category: option.queue_category.label(),
        cost: option.cost,
        enabled: option.enabled,
    }
}

fn entity_fact(
    simulation: &Simulation,
    stable_id: u64,
    entity: &GameEntity,
    object_type: Option<&ObjectType>,
) -> EntityFact {
    EntityFact {
        id: stable_id,
        type_id: simulation.interner.resolve(entity.type_ref()).to_string(),
        category: category_name(entity.category),
        rx: entity.position.rx,
        ry: entity.position.ry,
        health: entity.health.current,
        max_health: object_type.map_or(entity.health.current, |object| object.strength),
    }
}

fn category_name(category: EntityCategory) -> &'static str {
    match category {
        EntityCategory::Unit => "unit",
        EntityCategory::Infantry => "infantry",
        EntityCategory::Structure => "structure",
        EntityCategory::Aircraft => "aircraft",
    }
}

/// Apply the same owner-specific disclosure gates when building observations
/// and when revalidating entity targets after a delayed API response.
pub(super) fn is_visible_enemy(
    simulation: &Simulation,
    rules: &RuleSet,
    owner: InternedId,
    entity: &GameEntity,
) -> bool {
    if !entity.is_active()
        || !entity.is_alive()
        || entity.lifecycle.in_limbo
        || entity.owner() == owner
    {
        return false;
    }
    let fog = &simulation.fog;
    let owner_name = simulation.interner.resolve(owner);
    let entity_owner = simulation.interner.resolve(entity.owner());
    if fog.is_friendly(owner_name, entity_owner)
        || !fog.is_cell_visible(owner, entity.position.rx, entity.position.ry)
        || fog.is_cell_gap_covered(owner, entity.position.rx, entity.position.ry)
    {
        return false;
    }

    if entity
        .cloak
        .as_ref()
        .is_some_and(|cloak| cloak.is_fully_cloaked())
        && !fog.has_sensor_for_house(owner, entity.position.rx, entity.position.ry)
    {
        return false;
    }
    if crate::sim::cloak_disguise::object_disguised_to(
        entity,
        owner,
        Some(fog),
        Some(&fog.alliances),
        &simulation.interner,
    ) {
        return false;
    }
    if rules
        .object(simulation.interner.resolve(entity.type_ref()))
        .is_some_and(|object| object.invisible_in_game)
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::map::entities::EntityCategory;
    use crate::sim::production::ProductionCategory;

    use super::super::test_support::{add_entity, queue_fixture_types, runtime};
    use super::build_observation;

    #[test]
    fn observation_contains_only_owned_and_currently_visible_enemy_facts() {
        let (mut runtime, owner) = runtime();
        add_entity(
            &mut runtime,
            2,
            "Americans",
            "E1",
            EntityCategory::Infantry,
            8,
            8,
        );
        add_entity(
            &mut runtime,
            10,
            "Soviet",
            "VisibleEnemy",
            EntityCategory::Unit,
            12,
            12,
        );
        add_entity(
            &mut runtime,
            11,
            "Yuri",
            "HiddenSecretTank",
            EntityCategory::Unit,
            31,
            32,
        );
        add_entity(
            &mut runtime,
            12,
            "Soviet",
            "HiddenCloakedSub",
            EntityCategory::Unit,
            13,
            13,
        );
        runtime.simulation.entities_mut().get_mut(12).unwrap().cloak = Some({
            let mut cloak = crate::sim::cloak_disguise::CloakRuntime::new(0, 1);
            cloak.establish_unlimbo_fully_cloaked();
            cloak
        });
        add_entity(
            &mut runtime,
            13,
            "Yuri",
            "HiddenDisguisedSpy",
            EntityCategory::Infantry,
            14,
            14,
        );
        runtime
            .simulation
            .entities_mut()
            .get_mut(13)
            .unwrap()
            .disguise = Some(crate::sim::cloak_disguise::DisguiseRuntime {
            disguised: true,
            ..Default::default()
        });
        add_entity(
            &mut runtime,
            14,
            "Soviet",
            "INVIS",
            EntityCategory::Structure,
            15,
            15,
        );
        runtime.simulation.fog.mark_visible_for_owner(owner, 12, 12);
        for cell in [(13, 13), (14, 14), (15, 15)] {
            runtime
                .simulation
                .fog
                .mark_visible_for_owner(owner, cell.0, cell.1);
        }

        let observation = build_observation(&runtime, owner);
        let enemies = observation["visible_enemies"]
            .as_array()
            .expect("visible enemy array");
        assert_eq!(enemies.len(), 1);
        assert_eq!(enemies[0]["id"], 10);
        assert_eq!(enemies[0]["type_id"], "VisibleEnemy");
        let serialized = observation.to_string();
        assert!(!serialized.contains("HiddenSecretTank"));
        assert!(!serialized.contains("\"id\":11"));
        assert!(!serialized.contains("\"rx\":31"));
        assert!(!serialized.contains("HiddenCloakedSub"));
        assert!(!serialized.contains("HiddenDisguisedSpy"));
        assert!(!serialized.contains("INVIS"));
    }

    #[test]
    fn observation_uses_stable_entity_queue_and_option_ordering() {
        let (mut runtime, owner) = runtime();
        add_entity(
            &mut runtime,
            7,
            "Americans",
            "E2",
            EntityCategory::Infantry,
            7,
            7,
        );
        add_entity(
            &mut runtime,
            3,
            "Americans",
            "E1",
            EntityCategory::Infantry,
            3,
            3,
        );
        queue_fixture_types(&mut runtime, owner);

        let first = build_observation(&runtime, owner);
        let second = build_observation(&runtime, owner);
        assert_eq!(first, second);

        let owned_ids = first["owned_entities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entity| entity["id"].as_u64().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(owned_ids, vec![1, 3, 7]);

        let queue = first["production_queue"].as_array().unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0]["type_id"], "E1");
        assert_eq!(queue[1]["type_id"], "E2");

        let options = first["production_options"].as_array().unwrap();
        assert_eq!(options.len(), 2);
        assert_eq!(
            options[0]["queue_category"],
            ProductionCategory::Infantry.label()
        );
        assert_eq!(options[0]["type_id"], "E1");
        assert_eq!(options[1]["type_id"], "E2");
    }
}
