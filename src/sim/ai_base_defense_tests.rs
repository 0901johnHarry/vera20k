//! Native comparisons for the base defense choice
//! (`tools/ai_base_defense_oracle.py`, `--check` regenerates the vectors).

use super::*;
use crate::rules::ini_parser::IniFile;
use serde_json::Value;

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../tools/ai_base_defense_oracle.json")).unwrap()
}

fn rows<'a>(oracle: &'a Value, section: &str) -> &'a [Value] {
    oracle[section].as_array().unwrap()
}

fn int(value: &Value) -> i64 {
    value.as_i64().unwrap()
}

fn ints(value: &Value) -> Vec<i32> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|value| int(value) as i32)
        .collect()
}

fn cell(value: &Value) -> (i16, i16) {
    (int(&value[0]) as i16, int(&value[1]) as i16)
}

fn rect(value: &Value) -> (i32, i32, i32, i32) {
    let parts = ints(value);
    (parts[0], parts[1], parts[2], parts[3])
}

fn grid_with(bounds: (i32, i32, i32, i32), cells: &[i32]) -> CoverageGrid {
    let mut grid = CoverageGrid::new(bounds);
    grid.cells_mut().copy_from_slice(cells);
    grid
}

#[test]
fn the_coverage_spread_matches_native() {
    let oracle = oracle();
    for row in rows(&oracle, "influence") {
        let mut grid = grid_with(rect(&row["rect"]), &ints(&row["before"]));
        grid.spread(cell(&row["cell"]), int(&row["value"]) as i32);
        assert_eq!(grid.cells(), ints(&row["after"]).as_slice(), "{row}");
    }
}

#[test]
fn the_defense_key_matches_native() {
    let oracle = oracle();
    for row in rows(&oracle, "key") {
        let grid = grid_with(rect(&row["rect"]), &ints(&row["grid"]));
        let key = grid.key(
            int(&row["index"]) as i32,
            cell(&row["cell"]),
            cell(&row["center"]),
            int(&row["argument"]) as i32,
        );
        assert_eq!(i64::from(key), int(&row["key"]), "{row}");
    }
}

/// Answers `RandomRanged` from a native transcript of `[low, high, answer]`,
/// checking each range.
struct Draws<'a> {
    label: &'a str,
    draws: Vec<Vec<i32>>,
    next: usize,
}

impl<'a> Draws<'a> {
    fn new(label: &'a str, draws: Vec<Vec<i32>>) -> Self {
        Self {
            label,
            draws,
            next: 0,
        }
    }

    fn draw(&mut self, low: i32, high: i32) -> i32 {
        let Some(draw) = self.draws.get(self.next) else {
            panic!("{}: native made no draw {}", self.label, self.next);
        };
        assert_eq!([low, high], draw[..2], "{}: draw {}", self.label, self.next);
        self.next += 1;
        draw[2]
    }

    fn finish(&self) {
        assert_eq!(
            self.next,
            self.draws.len(),
            "{}: native drew more",
            self.label
        );
    }
}

fn force_values(values: &Value) -> ForceValues {
    let [infantry, vehicles, air] = ints(values)[..] else {
        panic!("three values");
    };
    ForceValues {
        infantry,
        vehicles,
        air,
    }
}

#[test]
fn the_threat_ratios_and_their_draws_match_native() {
    let oracle = oracle();
    for row in rows(&oracle, "threat") {
        let label = row.to_string();
        let mut draws = Draws::new(
            &label,
            row["draws"].as_array().unwrap().iter().map(ints).collect(),
        );
        let enemy = (int(&row["enemy"]) != -1).then(|| force_values(&row["values"]));
        let fudge = ints(&row["fudge"])[int(&row["difficulty"]) as usize];
        let ratios = threat_ratios(enemy, fudge, |low, high| draws.draw(low, high));
        draws.finish();
        assert_eq!(
            [ratios.vehicles, ratios.air, ratios.infantry].map(NativeF32Bits::bits),
            [
                int(&row["vehicles"]) as u32,
                int(&row["air"]) as u32,
                int(&row["infantry"]) as u32
            ],
            "{row}"
        );
    }
}

/// The oracle's BuildingTypes in its array order: values (air, armor,
/// infantry), `IsBaseDefense=`, owner, `TechLevel=`, prerequisites. Owners
/// are countries: the house is the fourth, `C3`.
const DEFENSE_TYPES: [(&str, [i32; 3], bool, &str, i32, &str); 14] = [
    ("YARD", [0, 0, 0], false, "C3", 1, ""),
    ("POWR", [0, 0, 0], false, "C3", 1, ""),
    ("TECH", [0, 0, 0], false, "C3", 1, ""),
    ("WALLT", [0, 0, 0], false, "C3", 1, ""),
    ("PILL", [0, 10, 25], true, "C3", 1, "POWR"),
    ("SAM", [25, 0, 0], true, "C3", 1, "POWR"),
    ("TESL", [0, 25, 25], true, "C3", 3, "TECH"),
    ("GGUN", [25, 10, 25], true, "C3", 1, ""),
    ("FORN", [25, 25, 25], true, "C1", 1, ""),
    ("HITK", [0, 25, 0], true, "C3", 11, ""),
    ("BUNK", [0, 25, 10], true, "C3", 1, "POWER"),
    ("NEGA", [-5, 30, 0], true, "C3", 1, ""),
    ("WALLD", [40, 40, 40], false, "C3", 1, ""),
    ("TECHD", [0, 0, 0], true, "C3", 1, ""),
];

fn defense_rules(wall_tower: Option<&str>) -> RuleSet {
    let mut text = String::from("[General]\n");
    if let Some(wall_tower) = wall_tower {
        text += &format!("WallTower={wall_tower}\n");
    }
    text += "[AI]\nAlliedBaseDefenses=PILL,SAM,TESL,GGUN,FORN,HITK,BUNK,NEGA\n\
             SovietBaseDefenses=BUNK,PILL,SAM,PILL\nThirdBaseDefenses=GGUN\n\
             BuildPower=POWR\nBuildRefinery=YARD\nBuildBarracks=YARD\nBuildWeapons=YARD\n\
             BuildRadar=TECH\nBuildTech=TECH\n\
             [Countries]\n0=C0\n1=C1\n2=C2\n3=C3\n\
             [InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n";
    for (index, (name, ..)) in DEFENSE_TYPES.iter().enumerate() {
        text += &format!("{index}={name}\n");
    }
    for (name, [air, armor, infantry], defense, owner, tech, prerequisite) in DEFENSE_TYPES {
        text += &format!(
            "[{name}]\nAntiAirValue={air}\nAntiArmorValue={armor}\nAntiInfantryValue={infantry}\n\
             IsBaseDefense={defense}\nOwner={owner}\nTechLevel={tech}\n"
        );
        if !prerequisite.is_empty() {
            text += &format!("Prerequisite={prerequisite}\n");
        }
    }
    RuleSet::from_ini(&IniFile::from_str(&text)).unwrap()
}

fn node(value: &Value) -> BasePlanNode {
    BasePlanNode {
        type_or_control: int(&value[0]) as i32,
        packed_cell: pack_base_plan_cell(int(&value[1]) as i32, int(&value[2]) as i32),
        filled: false,
        retry_count: 0,
    }
}

/// Whole choices: the draws and the site search call (type, argument and the
/// grid the key reads) in native order, then the node writes.
#[test]
fn whole_choices_match_native() {
    let oracle = oracle();
    let choices = rows(&oracle, "choose");
    let mut placed = 0;
    for row in choices {
        let label = row["label"].as_str().unwrap();
        let rules = defense_rules(row["wall_tower"].as_str());
        let country = rules.trigger_house_type_index("C3").unwrap();
        let buildings = row["buildings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|building| {
                let ty = rules.object(building[0].as_str().unwrap()).unwrap();
                (cell(&building[1]), ty)
            })
            .collect();
        let mut nodes: Vec<BasePlanNode> =
            row["nodes"].as_array().unwrap().iter().map(node).collect();
        let index = int(&row["index"]) as usize;
        let choice = DefenseChoice {
            rules: &rules,
            center: cell(&row["center"]),
            bounds: rect(&row["rect"]),
            buildings,
            side_index: int(&row["side"]) as u8,
            country_bit: country_bit(country),
            tech_level: int(&row["tech"]) as i32,
            enemy: (int(&row["enemy"]) != -1).then(|| force_values(&row["values"])),
            fudge: ints(&row["fudge"])[int(&row["difficulty"]) as usize],
            node_type: nodes[index].type_or_control,
        };
        let events = row["events"].as_array().unwrap();
        let draw_events = events
            .iter()
            .filter(|event| event[0] == "draw")
            .map(|event| {
                event.as_array().unwrap()[1..]
                    .iter()
                    .map(|v| int(v) as i32)
                    .collect()
            })
            .collect();
        let mut draws = Draws::new(label, draw_events);
        let pick = choice.pick(|low, high| draws.draw(low, high));
        draws.finish();
        let site_calls: Vec<&Value> = events.iter().filter(|event| event[0] == "site").collect();
        let result = match pick {
            None => {
                assert!(site_calls.is_empty(), "{label}: native searched a site");
                false
            }
            Some(pick) => {
                let [call] = site_calls[..] else {
                    panic!("{label}: native made {} site calls", site_calls.len());
                };
                assert_eq!(
                    (
                        i64::from(pick.place.base_plan_type_index),
                        int(&call[2]),
                        i64::from(pick.argument)
                    ),
                    (int(&call[1]), 0x0050_5FD0, int(&call[3])),
                    "{label}: site search"
                );
                assert_eq!(
                    pick.grid.cells(),
                    ints(&call[4]).as_slice(),
                    "{label}: grid"
                );
                let site = cell(&row["site"]);
                if site == (0, 0) {
                    false
                } else {
                    write_nodes(&mut nodes, index, pick.defense, site);
                    placed += 1;
                    true
                }
            }
        };
        assert_eq!(i64::from(result), int(&row["result"]), "{label}");
        let after: Vec<BasePlanNode> = row["nodes_after"]
            .as_array()
            .unwrap()
            .iter()
            .map(node)
            .collect();
        assert_eq!(nodes, after, "{label}");
        assert_eq!(int(&row["grid_after"]), 0, "{label}");
    }
    assert!(
        choices.len() >= 50 && placed >= 30,
        "{} choices, {placed} placed",
        choices.len()
    );
}
