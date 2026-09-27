use super::super::ordinary::test_host::Host;
use super::*;
use serde_json::{Value, json};

impl OrdinaryDamageHost for Host {
    fn notify_span(&mut self, first: CellCoord, end: CellCoord) -> Result<(), ()> {
        self.trace
            .push(json!({"kind":"notify", "endpoints":[first, end]}));
        super::super::rim::visit_span_cells(first, end, |point| {
            let cell = self.lookup(point);
            self.coord(cell)
        });
        Ok(())
    }
}

#[test]
fn concrete_damage_matches_original_all_overlay_states_and_width_entries() {
    let corpus: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tools/spatial_oracle/bridge_ordinary_damage.json"
    )))
    .unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let input = &case["input"];
        let coord = |row: &Value| {
            (
                row[0].as_i64().unwrap() as i16,
                row[1].as_i64().unwrap() as i16,
            )
        };
        let mut host = Host {
            cells: input["cells"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| (coord(row), row[5].as_i64().unwrap() as i32))
                .collect(),
            dummy: ((0, 0), -1),
            variant: 0,
            trace: vec![],
        };
        let returned = damage(&mut host, coord(&input["start"])).unwrap();
        let final_cells: Vec<_> = input["cells"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let (x, y) = coord(row);
                [i32::from(x), i32::from(y), host.cells[&(x, y)]]
            })
            .collect();
        assert_eq!(
            json!({"returned":u8::from(returned), "trace":host.trace, "final":final_cells,
            "dummy":[i32::from(host.dummy.0.0),i32::from(host.dummy.0.1),host.dummy.1]}),
            case["result"],
            "{}",
            input["name"]
        );
    }
}
