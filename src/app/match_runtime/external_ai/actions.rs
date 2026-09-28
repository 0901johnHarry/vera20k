use crate::sim::command::Command;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::InternedId;
use crate::sim::production::{self, BuildOption};
use crate::sim::runtime::SimRuntime;

use super::observation::is_visible_enemy;
use super::protocol::ExternalAiAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActionValidationError {
    TooManyActions,
}

pub(super) fn validate_actions(
    runtime: &SimRuntime,
    owner: InternedId,
    actions: &[ExternalAiAction],
    max_actions: usize,
) -> Result<Vec<Command>, ActionValidationError> {
    if actions.len() > max_actions {
        return Err(ActionValidationError::TooManyActions);
    }

    let simulation = &runtime.simulation;
    let owner_name = simulation.interner.resolve(owner);
    let build_options = actions
        .iter()
        .any(|action| matches!(action, ExternalAiAction::QueueProduction { .. }))
        .then(|| {
            production::build_options_for_owner(simulation, &runtime.resources.rules, owner_name)
        });
    let (map_width, map_height) = (simulation.session.map_width, simulation.session.map_height);
    let mut accepted = Vec::with_capacity(actions.len());

    for action in actions {
        let command = match action {
            ExternalAiAction::QueueProduction { type_id } => {
                let Some(option) = build_options
                    .as_ref()
                    .and_then(|options| enabled_option(simulation, options, type_id))
                else {
                    continue;
                };
                Command::QueueProduction {
                    type_id: option.type_id,
                }
            }
            ExternalAiAction::Move {
                entity_id,
                target_rx,
                target_ry,
                queue,
            } if valid_mobile_owner_entity(runtime, owner, *entity_id)
                && cell_in_bounds(*target_rx, *target_ry, map_width, map_height) =>
            {
                Command::Move {
                    entity_id: *entity_id,
                    target_rx: *target_rx,
                    target_ry: *target_ry,
                    queue: *queue,
                }
            }
            ExternalAiAction::AttackMove {
                entity_id,
                target_rx,
                target_ry,
                queue,
            } if valid_mobile_owner_entity(runtime, owner, *entity_id)
                && cell_in_bounds(*target_rx, *target_ry, map_width, map_height) =>
            {
                Command::AttackMove {
                    entity_id: *entity_id,
                    target_rx: *target_rx,
                    target_ry: *target_ry,
                    queue: *queue,
                }
            }
            ExternalAiAction::Guard {
                entity_id,
                target_id,
            } if valid_mobile_owner_entity(runtime, owner, *entity_id)
                && valid_guard_target(runtime, owner, *target_id) =>
            {
                Command::Guard {
                    entity_id: *entity_id,
                    target_id: *target_id,
                }
            }
            _ => continue,
        };
        accepted.push(command);
    }
    Ok(accepted)
}

fn enabled_option<'a>(
    simulation: &crate::sim::world::Simulation,
    options: &'a [BuildOption],
    requested_type: &str,
) -> Option<&'a BuildOption> {
    options.iter().find(|option| {
        option.enabled
            && simulation
                .interner
                .resolve(option.type_id)
                .eq_ignore_ascii_case(requested_type)
    })
}

fn valid_mobile_owner_entity(runtime: &SimRuntime, owner: InternedId, id: u64) -> bool {
    runtime.simulation.entities().get(id).is_some_and(|entity| {
        is_live(entity)
            && entity.owner() == owner
            && entity.category != crate::map::entities::EntityCategory::Structure
    })
}

fn valid_guard_target(runtime: &SimRuntime, owner: InternedId, target_id: Option<u64>) -> bool {
    target_id.is_none_or(|id| {
        runtime.simulation.entities().get(id).is_some_and(|target| {
            if !is_live(target) {
                return false;
            }
            target.owner() == owner
                || is_visible_enemy(&runtime.simulation, &runtime.resources.rules, owner, target)
        })
    })
}

fn is_live(entity: &GameEntity) -> bool {
    entity.is_active() && entity.is_alive() && !entity.lifecycle.in_limbo
}

fn cell_in_bounds(rx: u16, ry: u16, width: u16, height: u16) -> bool {
    rx < width && ry < height
}

#[cfg(test)]
mod tests {
    use crate::map::entities::EntityCategory;
    use crate::sim::command::Command;
    use crate::sim::production::ProductionCategory;

    use super::super::protocol::ExternalAiAction;
    use super::super::test_support::{add_entity, runtime};
    use super::{ActionValidationError, validate_actions};

    fn move_action(entity_id: u64, target_rx: u16, target_ry: u16) -> ExternalAiAction {
        ExternalAiAction::Move {
            entity_id,
            target_rx,
            target_ry,
            queue: false,
        }
    }

    #[test]
    fn rejects_a_unit_owned_by_another_house() {
        let (mut runtime, owner) = runtime();
        add_entity(
            &mut runtime,
            2,
            "Soviet",
            "Enemy",
            EntityCategory::Unit,
            5,
            5,
        );
        assert!(
            validate_actions(&runtime, owner, &[move_action(2, 8, 8)], 16)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_a_missing_entity() {
        let (runtime, owner) = runtime();
        assert!(
            validate_actions(&runtime, owner, &[move_action(99, 8, 8)], 16)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_a_dead_entity() {
        let (mut runtime, owner) = runtime();
        add_entity(
            &mut runtime,
            2,
            "Americans",
            "E1",
            EntityCategory::Infantry,
            5,
            5,
        );
        runtime
            .simulation
            .entities_mut()
            .get_mut(2)
            .unwrap()
            .health
            .current = 0;
        assert!(
            validate_actions(&runtime, owner, &[move_action(2, 8, 8)], 16)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_a_guard_target_that_is_no_longer_visible() {
        let (mut runtime, owner) = runtime();
        add_entity(
            &mut runtime,
            2,
            "Americans",
            "E1",
            EntityCategory::Infantry,
            5,
            5,
        );
        add_entity(
            &mut runtime,
            3,
            "Soviet",
            "Enemy",
            EntityCategory::Unit,
            12,
            12,
        );
        let action = ExternalAiAction::Guard {
            entity_id: 2,
            target_id: Some(3),
        };
        assert!(
            validate_actions(&runtime, owner, &[action], 16)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_out_of_bounds_destinations() {
        let (mut runtime, owner) = runtime();
        add_entity(
            &mut runtime,
            2,
            "Americans",
            "E1",
            EntityCategory::Infantry,
            5,
            5,
        );
        assert!(
            validate_actions(&runtime, owner, &[move_action(2, 40, 5)], 16)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_unavailable_production_types() {
        let (mut runtime, owner) = runtime();
        add_entity(
            &mut runtime,
            2,
            "Americans",
            "E1",
            EntityCategory::Infantry,
            5,
            5,
        );
        let action = ExternalAiAction::QueueProduction {
            type_id: "NOT_A_RULE_TYPE".to_string(),
        };
        assert!(
            validate_actions(&runtime, owner, &[action], 16)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn structurally_valid_actions_keep_valid_siblings_and_map_to_commands() {
        let (mut runtime, owner) = runtime();
        add_entity(
            &mut runtime,
            2,
            "Americans",
            "E1",
            EntityCategory::Infantry,
            5,
            5,
        );
        let commands = validate_actions(
            &runtime,
            owner,
            &[
                ExternalAiAction::QueueProduction {
                    type_id: "E1".to_string(),
                },
                move_action(99, 8, 8),
                ExternalAiAction::AttackMove {
                    entity_id: 2,
                    target_rx: 8,
                    target_ry: 9,
                    queue: true,
                },
            ],
            16,
        )
        .unwrap();
        assert_eq!(
            commands,
            vec![
                Command::QueueProduction {
                    type_id: runtime.simulation.interner.get("E1").unwrap(),
                },
                Command::AttackMove {
                    entity_id: 2,
                    target_rx: 8,
                    target_ry: 9,
                    queue: true,
                },
            ]
        );
        assert_eq!(ProductionCategory::Infantry.label(), "Infantry");
    }

    #[test]
    fn rejects_the_whole_decision_when_it_exceeds_the_action_limit() {
        let (runtime, owner) = runtime();
        let error = validate_actions(&runtime, owner, &[move_action(1, 2, 2)], 0).unwrap_err();
        assert_eq!(error, ActionValidationError::TooManyActions);
    }
}
