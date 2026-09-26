use std::path::Path;
use vera20k::assets::asset_manager::AssetManager;
use vera20k::rules::ini_parser::IniFile;
fn main() {
    let retail_root = std::env::var("RA2_DIR").expect("set RA2_DIR to the retail asset directory");
    let retail = Path::new(&retail_root);
    let assets = AssetManager::new(retail).unwrap();
    let mut rows = Vec::new();
    for name in [
        "RULESMD.INI",
        "ARTMD.INI",
        "LANGRULE.INI",
        "MPBattleMD.ini",
        "Hills.mmx",
    ] {
        let bytes = if name == "Hills.mmx" {
            std::fs::read(retail.join(name)).ok()
        } else {
            assets.get(name)
        };
        if let Some(bytes) = bytes {
            let ini = IniFile::from_bytes(&bytes).unwrap();
            for section in [
                "TREE01",
                "TIBTRE01",
                "DBRIS1LG",
                "DBRIS1SM",
                "DBRIS4SM",
                "D",
                "CombatDamage",
                "General",
                "AP",
                "HE",
                "Super",
            ] {
                for key in [
                    "Damage",
                    "DamageRadius",
                    "Warhead",
                    "Wood",
                    "Verses",
                    "CellSpread",
                    "PercentAtMax",
                    "Strength",
                    "Armor",
                    "Immune",
                    "SpawnsTiberium",
                    "C4Warhead",
                    "C4",
                    "TreeStrength",
                    "Bouncer",
                    "Elasticity",
                    "TemperateOccupationBits",
                    "SnowOccupationBits",
                    "Wall",
                ] {
                    if let Some(value) = ini.section(section).and_then(|s| s.get(key)) {
                        rows.push(format!("{{\"file\":{name:?},\"section\":{section:?},\"key\":{key:?},\"raw_value\":{value:?}}}"));
                    }
                }
            }
        } else {
            rows.push(format!("{{\"file\":{name:?},\"absent\":true}}"));
        }
    }
    println!("[{}]", rows.join(",\n"));
}
