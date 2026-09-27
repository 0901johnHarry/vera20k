//! Native AStar height fragments. These compare original instruction results,
//! not a second Rust model; they do not establish full-route equivalence.

use super::*;
use serde_json::Value;
use std::cell::RefCell;

fn scalar_cell(level: u8, flags: u32, walkable_hint: bool) -> PathCell {
    PathCell {
        ground_level: level,
        bridge_structural: flags & 0x100 != 0,
        bridge_walkable: walkable_hint,
        // This derived byte is deliberately not the source of native height.
        bridge_deck_level: level.wrapping_add(4),
        ..DEFAULT_WALKABLE_CELL
    }
}

#[test]
fn original_signed_height_producer_and_blocked_goal_tail() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../tools/spatial_oracle/astar_signed_height.json"
    ))
    .unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 11);
    for case in cases {
        let input = &case["input"];
        let start = scalar_cell(
            input["source_level_raw"].as_u64().unwrap() as u8,
            input["source_flags"].as_u64().unwrap() as u32,
            false,
        );
        let goal = scalar_cell(
            input["goal_level_raw"].as_u64().unwrap() as u8,
            input["goal_flags"].as_u64().unwrap() as u32,
            false,
        );
        let layer = if input["source_on_bridge"].as_bool().unwrap() {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        let (initial, goal_height) = initial_search_heights(&start, layer, &goal);
        assert_eq!(
            i64::from(initial),
            case["produced_initial_current_height"].as_i64().unwrap(),
            "{input}"
        );
        assert_eq!(
            i64::from(goal_height),
            case["produced_goal_height"].as_i64().unwrap(),
            "{input}"
        );
        // Two controls supply a later node height after original initial-height
        // production. This distinguishes the tail's current node from its start.
        let current = input["current_height_after_hops_supplied"]
            .as_i64()
            .map_or(initial, |height| height as i16);
        assert_eq!(
            blocked_goal_height_matches(current, goal_height),
            case["branch"] == "blocked_goal_abort",
            "{input}"
        );
    }
}

#[test]
fn original_structural_node_height_and_closed_list_selection() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../tools/spatial_oracle/astar_structural_height.json"
    ))
    .unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 22);
    for case in cases {
        let input = &case["input"];
        let parent = scalar_cell(
            input["parent_ground_raw"].as_u64().unwrap() as u8,
            input["parent_flags"].as_u64().unwrap() as u32,
            input["parent_rust_bridge_walkable"]
                .as_bool()
                .unwrap_or(false),
        );
        let candidate = scalar_cell(
            input["candidate_ground_raw"].as_u64().unwrap() as u8,
            input["candidate_flags"].as_u64().unwrap() as u32,
            input["candidate_rust_bridge_walkable"]
                .as_bool()
                .unwrap_or(false),
        );
        let current = input["current_height"].as_i64().unwrap() as i16;
        let parent = (!input["initial_node"].as_bool().unwrap_or(false)).then_some(&parent);
        assert_eq!(
            i64::from(compute_node_height(current, parent, &candidate)),
            case["node_height"].as_i64().unwrap(),
            "{input}"
        );
        assert_eq!(
            is_at_bridge_level(current, &candidate),
            case["selected_list"] == "deck",
            "{input}"
        );
    }
}

struct SuppliedEntry {
    answer: Result<u8, String>,
    queries: RefCell<Vec<SearchEntryQuery>>,
}

impl SearchFootEntry for SuppliedEntry {
    fn classify(&self, query: SearchEntryQuery) -> Result<u8, String> {
        self.queries.borrow_mut().push(query);
        self.answer.clone()
    }
}

#[test]
fn canonical_entry_reaches_blocked_goal_before_static_grid_refusal() {
    // Production wiring check: the supplied class comes from concrete native
    // Infantry-entry controls. This two-cell Rust route is not a native route
    // golden; it exposes a second static-grid verdict overriding the live one.
    let native: Value = serde_json::from_str(include_str!(
        "../../../tools/spatial_oracle/astar_capture_neighbor.json"
    ))
    .unwrap();
    let source = scalar_cell(10, 0, false);
    let goal = PathCell {
        ground_walkable: false,
        ..source
    };
    let grid = PathGrid::from_cells(vec![source, goal], 2, 1);
    for name in ["synthetic_flat_capture", "synthetic_iron_curtain_control"] {
        let case = native["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["input"]["name"] == name)
            .unwrap();
        let entry = SuppliedEntry {
            answer: Ok(case["can_enter_class"].as_u64().unwrap() as u8),
            queries: RefCell::new(Vec::new()),
        };
        let result = astar_search(
            &grid,
            (0, 0),
            MovementLayer::Ground,
            (1, 0),
            &AStarOptions {
                foot_entry: Some(&entry),
                is_infantry: true,
                ..Default::default()
            },
        );
        assert_eq!(result.is_ok(), case["outcome"] == "admit_node_creation");
        let queries = entry.queries.borrow();
        assert_eq!(queries.len(), 1, "{name}");
        let query = queries[0];
        assert_eq!((query.from, query.candidate), ((0, 0), (1, 0)));
        assert_eq!((query.direction, query.path_height), (2, 10));
    }
}

#[test]
fn unavailable_live_entry_is_not_an_ordinary_blocked_route() {
    let entry = SuppliedEntry {
        answer: Err("unavailable canonical Cell input".into()),
        queries: RefCell::new(Vec::new()),
    };
    assert!(matches!(
        astar_search(
            &PathGrid::new(2, 1),
            (0, 0),
            MovementLayer::Ground,
            (1, 0),
            &AStarOptions { foot_entry: Some(&entry), ..Default::default() },
        ),
        Err(PathSearchFailure::CellEntryUnavailable(cause))
            if cause == "unavailable canonical Cell input"
    ));
    assert_eq!(entry.queries.borrow().len(), 1);
}
