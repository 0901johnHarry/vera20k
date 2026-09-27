//! The process owner binds native sound IDs to fixed SOUNDMD names, retaining
//! valid values across root/LANG/mode/map readers (712FF1/7130A5/6699C8).

use super::NativeRulesProcessOwner;
use crate::rules::ini_parser::IniFile;
use crate::rules::sound_ini::SoundRegistry;

#[test]
fn sinking_sound_references_keep_valid_prior_ids_and_exact_reader_scope() {
    let ini = IniFile::from_str;
    let root = ini(
        "[VehicleTypes]\n0=SHIP\n[SHIP]\nSinkingSound=Hull\nVoiceSinking=Voice\n\
         [AudioVisual]\nSinkingSound=Fallback\n[General]\nSinkingSound=WrongSection\n",
    );
    let lang = ini("[SHIP]\nSinkingSound=unknown\nVoiceSinking=none\n");
    let mode = ini("[SHIP]\nSinkingSound=  hUlL  \n[AudioVisual]\nSinkingSound=unknown\n");
    let map = ini("[SHIP]\nSinkingSound=\nVoiceSinking=not_registered\nvoicesinking=Hull\n");
    let mut owner =
        NativeRulesProcessOwner::from_cold_start_sources(root, Some(lang), ini("")).unwrap();
    owner.select_fixed_sounds(SoundRegistry::from_ini(&ini(
        "[SoundList]\n0=Hull\n1=Voice\n2=Fallback\n3=WrongSection\n\
         [not_registered]\nSounds=sample\n",
    )));
    let (rules, _, _, _) = owner
        .load_noncampaign_scenario(Some(&mode), &map)
        .unwrap()
        .into_parts();
    let ship = rules.object("SHIP").unwrap();
    assert_eq!(ship.sinking_sound.as_deref(), Some("Hull"));
    assert_eq!(ship.voice_sinking.as_deref(), Some("Voice"));
    assert_eq!(rules.general.sinking_sound.as_deref(), Some("Fallback"));

    let mut empty_catalog = NativeRulesProcessOwner::from_cold_start_sources(
        ini("[VehicleTypes]\n0=SHIP\n[SHIP]\nSinkingSound=Hull\n[General]\nSinkingSound=Hull\n"),
        None,
        ini(""),
    )
    .unwrap();
    let (rules, _, _, _) = empty_catalog
        .load_noncampaign_scenario(None, &ini(""))
        .unwrap()
        .into_parts();
    assert!(rules.object("SHIP").unwrap().sinking_sound.is_none());
    assert!(rules.object("SHIP").unwrap().voice_sinking.is_none());
    assert!(rules.general.sinking_sound.is_none());
}

#[test]
fn sinking_sound_reference_uses_readstring128_before_lookup() {
    let sounds = SoundRegistry::from_ini(&IniFile::from_str("[SoundList]\n0=Hull\n"));
    // 123 spaces plus the four-character name fill the 127 usable bytes.
    let ini = IniFile::from_str(&format!("[SHIP]\nSinkingSound={}HullX\n", " ".repeat(123)));
    // The physical INI parser trims its value before the later ReadString.
    // Preserve the reader boundary explicitly in a projected section instead.
    let mut section = ini.section("SHIP").unwrap().clone();
    section.set("SinkingSound", &format!("{}HullX", " ".repeat(123)));
    assert_eq!(
        sounds
            .read_rules_reference(&section, "SinkingSound")
            .as_deref(),
        Some("Hull")
    );
    section.set("SinkingSound", &format!("{}Hull", " ".repeat(124)));
    assert!(
        sounds
            .read_rules_reference(&section, "SinkingSound")
            .is_none()
    );
}
