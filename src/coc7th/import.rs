use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    array::from_fn,
    iter::Iterator,
    option::Option::{self, None, Some},
    primitive::{i16, i64, str, u8, u16, u32, u64},
    result::Result::{self, Err, Ok},
};

use serde_json::Value;

use crate::{
    DataError, En, Language,
    coc7th::investigator::{
        Age, ArtAndCraft, ArtAndCraftCustom, Backstory, Birthplace, Build, CUSTOM_ID_BASE,
        Characteristic, Fighting, FightingCustom, Firearms, FirearmsCustom, HitPoints,
        LanguageOther, LanguageOwn, Luck, MAX_MEMO_SLOTS, MagicPoints, Memo, MoveRate, Name,
        Occupation, OccupationKind, Pilot, PilotCustom, Pronoun, Residence, SCHEMA_SIZE, Sanity,
        Science, ScienceCustom, Skill, Survival, SurvivalCustom, write_skill,
    },
    data_struct::DataStruct,
    object::Predicate,
};

const EN: Language = Language::En(En::Gb);

pub struct Imported {
    pub investigator: DataStruct,
    pub skipped:      Vec<String>,
}

#[derive(Default)]
struct Slots {
    next_id:        u32,
    art_and_craft:  u8,
    fighting:       u8,
    firearms:       u8,
    language_other: u8,
    pilot:          u8,
    science:        u8,
    survival:       u8,
}

impl Slots {
    fn ids(&mut self) -> [u32; 2] {
        let ids = [self.next_id, self.next_id + 1];
        self.next_id += 2;
        ids
    }
}

pub fn import(body: &[u8], now_in_ms: u64) -> Result<Imported, DataError> {
    let root: Value =
        serde_json::from_slice(body).map_err(|error| DataError::Parse(format!("{error}")))?;
    if !root.is_object() {
        return Err(DataError::Format(String::from("root")));
    }
    let id = root.get("ID").and_then(Value::as_u64).unwrap_or(0) as u32;
    let mut investigator = DataStruct::new(id, now_in_ms, SCHEMA_SIZE);
    let mut skipped = Vec::new();
    let mut slots = Slots { next_id: CUSTOM_ID_BASE, ..Slots::default() };

    if let Some(profile) = root.get("Profile") {
        profile_into(&mut investigator, profile)?;
    }
    if let Some(characteristics) = root.get("Characteristics").and_then(Value::as_object) {
        for (key, value) in characteristics {
            match Characteristic::list().iter().find(|c| c.display(EN) == key.as_str()) {
                Some(characteristic) => {
                    let n = numbers::<3>(value);
                    characteristic
                        .write(&mut investigator, (n[0] as u16, n[1] as i16, n[2] as i16));
                }
                None => skipped.push(key.clone()),
            }
        }
    }
    if let Some(secondary) = root.get("Secondary Attributes") {
        let number = |key: &str| secondary.get(key).and_then(Value::as_i64);
        if let Some(v) = number("Hit Points") {
            HitPoints::write(&mut investigator, v as u8);
        }
        if let Some(v) = number("Magic Points") {
            MagicPoints::write(&mut investigator, v as u8);
        }
        if let Some(v) = number("Luck") {
            Luck::write(&mut investigator, v as u8);
        }
        if let Some(v) = number("Sanity") {
            Sanity::write(&mut investigator, v as u8);
        }
        if let Some(v) = number("Build") {
            Build::write(&mut investigator, v as i8);
        }
        if let Some(v) = number("Move Rate") {
            MoveRate::write(&mut investigator, v as u8);
        }
    }
    if let Some(skills) = root.get("Skills").and_then(Value::as_object) {
        for (key, value) in skills {
            let n = numbers::<5>(value);
            let points = (n[0] as u16, n[1] as u16, n[2] as u16, n[3] as i16, n[4] as i16);
            if !skill_into(&mut investigator, &mut slots, key, points) {
                skipped.push(key.clone());
            }
        }
    }
    if let Some(backstory) = root.get("Backstory").and_then(Value::as_object) {
        for (key, value) in backstory {
            let Some(text) = value.as_str() else {
                continue;
            };
            match backstory_of(key) {
                Some(entry) if key == "Key Connection" => match backstory_of(text) {
                    Some(target) => {
                        let id = Backstory::KeyConnection(Box::new(entry)).id();
                        investigator.set(id, &[target.id() as u8], None)?;
                    }
                    None => skipped.push(key.clone()),
                },
                Some(entry) => {
                    investigator.set(entry.id(), text.as_bytes(), None)?;
                }
                None => skipped.push(key.clone()),
            }
        }
    }
    if let Some(notes) = root.get("Notes").and_then(Value::as_object) {
        for (slot, (key, value)) in notes.iter().take(MAX_MEMO_SLOTS).enumerate() {
            let body = match value {
                Value::String(text) => text.clone(),
                Value::Array(items) => {
                    items.iter().map(scenario_line).collect::<Vec<_>>().join("\n")
                }
                other => other.to_string(),
            };
            investigator.set(Memo { slot }.id(), &Memo::encode(key, &body), None)?;
        }
    }
    for key in ["Possessions", "Wealth"] {
        if root.get(key).is_some() {
            skipped.push(String::from(key));
        }
    }
    Ok(Imported { investigator, skipped })
}

fn profile_into(investigator: &mut DataStruct, profile: &Value) -> Result<(), DataError> {
    let text = |key: &str| profile.get(key).and_then(Value::as_str).map(String::from);
    let name = text("Name").unwrap_or_default();
    Name::write(investigator, &(name, text("Pronunciation")), None)?;
    if let Some(occupation) = text("Occupation") {
        let kind = Occupation::list()
            .iter()
            .find(|kind| Occupation::display(kind, None, None, EN) == occupation)
            .copied();
        let parsed = match kind {
            Some(kind) => (kind, None, text("Title")),
            None => (OccupationKind::Custom, Some(occupation), text("Title")),
        };
        Occupation::write(investigator, &parsed, None)?;
    }
    if let Some(value) = text("Birthplace") {
        Birthplace::write(investigator, &value, None)?;
    }
    if let Some(value) = text("Pronoun") {
        Pronoun::write(investigator, &value, None)?;
    }
    if let Some(value) = text("Residence") {
        Residence::write(investigator, &value, None)?;
    }
    if let Some(age) = profile.get("Age").and_then(Value::as_u64) {
        Age::write(investigator, &(age as u16), None)?;
    }
    Ok(())
}

fn skill_into(
    investigator: &mut DataStruct,
    slots: &mut Slots,
    key: &str,
    (base, occupation, interest, change, modifier): (u16, u16, u16, i16, i16),
) -> bool {
    let (name, specialization) = split_name(key);
    let skill = match name {
        "Art/Craft" => Some(Skill::ArtAndCraft),
        "Electrical Repair" => Some(Skill::ElectricalRepair),
        "Mechanical Repair" => Some(Skill::MechanicalRepair),
        "Language (Own)" => Some(Skill::LanguageOwn),
        "Language, Other" => Some(Skill::LanguageOther),
        _ => Skill::list().iter().find(|skill| skill.display(EN) == name).cloned(),
    };
    let Some(skill) = skill else {
        return false;
    };
    let blank =
        specialization.is_none() && (occupation, interest, change, modifier) == (0, 0, 0, 0);
    let spec = specialization.unwrap_or("");
    match skill {
        Skill::LanguageOwn => {
            LanguageOwn.write(investigator, occupation, interest, change, modifier, spec);
        }
        Skill::LanguageOther => {
            if blank {
                return true;
            }
            let slot = slots.language_other;
            slots.language_other += 1;
            let ids = slots.ids();
            LanguageOther(slot).write(
                investigator,
                ids,
                occupation,
                interest,
                change,
                modifier,
                spec,
            );
        }
        Skill::ArtAndCraft => {
            let known = ArtAndCraft::list()
                .iter()
                .filter(|a| **a != ArtAndCraft::Custom)
                .find(|a| a.display(EN) == spec);
            match known {
                Some(a) => {
                    write_skill(investigator, a.base_id(), occupation, interest, change, modifier)
                }
                None if blank => {}
                None => {
                    let slot = slots.art_and_craft;
                    slots.art_and_craft += 1;
                    let ids = slots.ids();
                    ArtAndCraftCustom(slot).write(
                        investigator,
                        ids,
                        occupation,
                        interest,
                        change,
                        modifier,
                        spec,
                    );
                }
            }
        }
        Skill::Fighting => {
            let list_base = Skill::Fighting.base_id();
            let known = Fighting::list()
                .iter()
                .filter(|f| **f != Fighting::Custom)
                .find(|f| f.display(EN) == spec);
            match known {
                Some(f) => write_skill(
                    investigator,
                    f.id(list_base),
                    occupation,
                    interest,
                    change,
                    modifier,
                ),
                None if blank => {}
                None => {
                    let slot = slots.fighting;
                    slots.fighting += 1;
                    let ids = slots.ids();
                    FightingCustom(slot).write(
                        investigator,
                        ids,
                        base,
                        occupation,
                        interest,
                        change,
                        modifier,
                        spec,
                    );
                }
            }
        }
        Skill::Firearms => {
            let list_base = Skill::Firearms.base_id();
            let known = Firearms::list()
                .iter()
                .filter(|f| **f != Firearms::Custom)
                .find(|f| f.display(EN) == spec);
            match known {
                Some(f) => write_skill(
                    investigator,
                    f.id(list_base),
                    occupation,
                    interest,
                    change,
                    modifier,
                ),
                None if blank => {}
                None => {
                    let slot = slots.firearms;
                    slots.firearms += 1;
                    let ids = slots.ids();
                    FirearmsCustom(slot).write(
                        investigator,
                        ids,
                        base,
                        occupation,
                        interest,
                        change,
                        modifier,
                        spec,
                    );
                }
            }
        }
        Skill::Pilot => {
            let list_base = Skill::Pilot.base_id();
            let known = Pilot::list()
                .iter()
                .filter(|p| **p != Pilot::Custom)
                .find(|p| p.display(EN) == spec);
            match known {
                Some(p) => write_skill(
                    investigator,
                    p.id(list_base),
                    occupation,
                    interest,
                    change,
                    modifier,
                ),
                None if blank => {}
                None => {
                    let slot = slots.pilot;
                    slots.pilot += 1;
                    PilotCustom(slot).write(
                        investigator,
                        occupation,
                        interest,
                        change,
                        modifier,
                        spec,
                    );
                }
            }
        }
        Skill::Science => {
            let list_base = Skill::Science.base_id();
            let known = Science::list()
                .iter()
                .filter(|s| **s != Science::Custom)
                .find(|s| s.display(EN) == Some(spec));
            match known {
                Some(s) => write_skill(
                    investigator,
                    s.id(list_base),
                    occupation,
                    interest,
                    change,
                    modifier,
                ),
                None if blank => {}
                None => {
                    let slot = slots.science;
                    slots.science += 1;
                    ScienceCustom(slot).write(
                        investigator,
                        occupation,
                        interest,
                        change,
                        modifier,
                        spec,
                    );
                }
            }
        }
        Skill::Survival => {
            let list_base = Skill::Survival.base_id();
            let known = Survival::list()
                .iter()
                .filter(|s| **s != Survival::Custom)
                .find(|s| s.display(EN) == spec);
            match known {
                Some(s) => write_skill(
                    investigator,
                    s.id(list_base),
                    occupation,
                    interest,
                    change,
                    modifier,
                ),
                None if blank => {}
                None => {
                    let slot = slots.survival;
                    slots.survival += 1;
                    SurvivalCustom(slot).write(
                        investigator,
                        occupation,
                        interest,
                        change,
                        modifier,
                        spec,
                    );
                }
            }
        }
        Skill::Custom => return false,
        plain => write_skill(investigator, plain.base_id(), occupation, interest, change, modifier),
    }
    true
}

fn split_name(key: &str) -> (&str, Option<&str>) {
    let trimmed = key.trim();
    match trimmed.strip_suffix(')').and_then(|head| head.rsplit_once(" (")) {
        Some((name, specialization)) if name != "Language" => (name, Some(specialization)),
        _ => (trimmed, None),
    }
}

fn backstory_of(name: &str) -> Option<Backstory> {
    let entries = [
        Backstory::PersonalDescription,
        Backstory::IdeologyAndBeliefs,
        Backstory::SignificantPeople,
        Backstory::MeaningfulLocation,
        Backstory::TreasuredPossession,
        Backstory::Trait,
        Backstory::InjuresAndScars,
        Backstory::PhobiasAndManias,
        Backstory::ArcaneTomesAndSpells,
        Backstory::EncountersWithStrangeEntities,
    ];
    if name == "Key Connection" {
        return Some(Backstory::KeyConnection(Box::new(Backstory::PersonalDescription)));
    }
    let wanted = match name {
        "Traits" => "Trait",
        "Meaningful Locations" => "Meaningful Location",
        "Treasured Possessions" => "Treasured Possession",
        other => other,
    };
    entries.into_iter().find(|entry| entry.display(EN) == wanted)
}

fn scenario_line(item: &Value) -> String {
    let scenario = item.get("scenario").and_then(Value::as_str).unwrap_or("");
    match item.get("text").and_then(Value::as_str) {
        Some(text) => format!("{scenario}: {text}"),
        None => String::from(scenario),
    }
}

fn numbers<const N: usize>(value: &Value) -> [i64; N] {
    match value.as_array() {
        Some(items) => from_fn(|i| items.get(i).and_then(Value::as_i64).unwrap_or(0)),
        None => from_fn(|i| if i == 0 { value.as_i64().unwrap_or(0) } else { 0 }),
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec::Vec};

    use super::*;
    use crate::coc7th::investigator::{Dodge, FightingTrait as _, SkillTrait as _, Sword};

    const SAMPLE: &str = r#"{
        "ID": 5,
        "Profile": {
            "Name": "鏖", "Pronunciation": "じん", "Occupation": "Soldier",
            "Birthplace": "日本", "Residence": null, "Pronoun": "he/him", "Age": 39
        },
        "Characteristics": { "STR": [75, 0, 0], "EDU": [75, 5, 0] },
        "Secondary Attributes": { "Hit Points": 13, "Luck": 35, "Build": 1, "Damage Bonus": "+1D4" },
        "Skills": {
            "Dodge": [22, 48, 0, 0, 0],
            "Fighting (Sword)": [20, 60, 0, 0, 0],
            "Language, Other (English)": [1, 29, 0, 0, 0],
            "Language (Own)": [80, 0, 0, 0, 0],
            "Art/Craft": [5, 0, 0, 0, 0],
            "Art/Craft (Origami)": [5, 0, 10, 0, 0],
            "Cthulhu Mythos": [0, 0, 0, 24, 0],
            "Operate Heavy Machinery": [1, 0, 0, 0, 0]
        },
        "Possessions": [],
        "Backstory": {
            "Key Connection": "Ideology & Beliefs",
            "Ideology & Beliefs": "後悔無きように",
            "Traits": "低音声"
        },
        "Notes": {
            "Personality": "快楽殺人鬼",
            "Scenarios": [{ "scenario": "A" }, { "scenario": "B", "text": "共闘" }]
        }
    }"#;

    fn imported() -> Imported {
        import(SAMPLE.as_bytes(), 0).unwrap()
    }

    #[test]
    fn the_profile_is_read_into_the_sheet() {
        let sheet = imported().investigator;
        assert_eq!(Name::read(&sheet), ("鏖".to_string(), Some("じん".to_string())));
        let (kind, custom, title) = Occupation::read(&sheet);
        assert_eq!(Occupation::display(&kind, custom.as_deref(), title.as_deref(), EN), "Soldier");
        assert_eq!(Birthplace::read(&sheet), "日本");
        assert_eq!(Pronoun::read(&sheet), "he/him");
        assert_eq!(Age::read(&sheet), 39);
    }

    #[test]
    fn characteristics_and_secondary_attributes_are_read() {
        let sheet = imported().investigator;
        assert_eq!(Characteristic::Strength.read(&sheet), (75, 0, 0));
        assert_eq!(Characteristic::Education.read(&sheet), (75, 5, 0));
        assert_eq!(HitPoints::read(&sheet), 13);
        assert_eq!(Luck::read(&sheet), 35);
        assert_eq!(Build::read(&sheet), 1);
    }

    #[test]
    fn skills_are_placed_by_name_and_specialization() {
        let sheet = imported().investigator;
        let (occupation, ..) = Dodge.read(&sheet);
        assert_eq!(occupation.value(), 48);
        let (occupation, ..) = Sword.read(&sheet);
        assert_eq!(occupation.value(), 60);
        assert!(sheet.get(Skill::CthulhuMythos.base_id()).is_ok());
        assert_eq!(LanguageOther(0).read(&sheet), (29, 0, 0, 0, "English".to_string()));
        assert_eq!(LanguageOwn.read(&sheet), (0, 0, 0, 0, "".to_string()));
        assert_eq!(ArtAndCraftCustom(0).read(&sheet), (0, 10, 0, 0, "Origami".to_string()));
    }

    #[test]
    fn unknown_entries_are_reported_and_the_rest_is_kept() {
        let skipped = imported().skipped;
        assert_eq!(skipped, Vec::from(["Operate Heavy Machinery", "Possessions"]));
    }

    #[test]
    fn backstory_and_notes_are_read() {
        let sheet = imported().investigator;
        assert_eq!(
            sheet.get(Backstory::IdeologyAndBeliefs.id()).unwrap(),
            "後悔無きように".as_bytes()
        );
        assert_eq!(sheet.get(Backstory::Trait.id()).unwrap(), "低音声".as_bytes());
        let key = Backstory::KeyConnection(Box::new(Backstory::Trait)).id();
        assert_eq!(sheet.get(key).unwrap(), &[Backstory::IdeologyAndBeliefs.id() as u8]);
        let (title, body) = Memo::decode(sheet.get(Memo { slot: 0 }.id()).unwrap());
        assert_eq!((title.as_str(), body.as_str()), ("Personality", "快楽殺人鬼"));
        let (title, body) = Memo::decode(sheet.get(Memo { slot: 1 }.id()).unwrap());
        assert_eq!((title.as_str(), body.as_str()), ("Scenarios", "A\nB: 共闘"));
    }

    #[test]
    fn broken_json_is_a_parse_error_and_a_list_is_a_format_error() {
        assert!(matches!(import(b"{", 0), Err(DataError::Parse(_))));
        assert!(matches!(import(b"[]", 0), Err(DataError::Format(_))));
    }
}
