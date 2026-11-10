use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    array::from_fn,
    fmt::{self, Display, Formatter},
    iter::Iterator,
    option::Option::{self, None, Some},
    primitive::{i16, i64, str, u8, u16, u32, u64},
    result::Result::{self, Err, Ok},
};

use serde_json::Value;

use crate::{
    En, Language,
    bit::Zero,
    coc7th::investigator::{
        ArtAndCraft, Backstory, Characteristic, Fighting, Firearms, Investigator, MEMO_SLOTS, Memo,
        Occupation, OccupationKind, Pilot, Points, Science, Score, Skill, Specialization,
        Specialty, Survival,
    },
    object::Record,
};

const EN: Language = Language::En(En::Gb);

#[derive(Debug)]
pub enum ImportError {
    Json(serde_json::Error),
    Root,
}

impl Display for ImportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Imported {
    pub investigator: Record<Investigator>,
    pub skipped:      Vec<String>,
}

pub fn import(body: &[u8], now_in_ms: u64) -> Result<Imported, ImportError> {
    let root: Value = serde_json::from_slice(body).map_err(ImportError::Json)?;
    if !root.is_object() {
        return Err(ImportError::Root);
    }
    let id = root.get("ID").and_then(Value::as_u64).unwrap_or(0) as u32;
    let mut record = Record::new(id, Investigator::ZERO);
    record.head.stamp(now_in_ms);
    let investigator = &mut record.body;
    let mut skipped = Vec::new();

    if let Some(profile) = root.get("Profile") {
        profile_into(investigator, profile);
    }
    if let Some(characteristics) = root.get("Characteristics").and_then(Value::as_object) {
        for (key, value) in characteristics {
            match Characteristic::list().iter().find(|c| c.display(EN) == key.as_str()) {
                Some(characteristic) => {
                    let n = numbers::<3>(value);
                    investigator.characteristics[characteristic.index()] = Score {
                        initial:  n[0] as u16,
                        change:   n[1] as i16,
                        modifier: n[2] as i16,
                    };
                }
                None => skipped.push(key.clone()),
            }
        }
    }
    if let Some(secondary) = root.get("Secondary Attributes") {
        let number = |key: &str| secondary.get(key).and_then(Value::as_i64);
        if let Some(v) = number("Hit Points") {
            investigator.hit_points = v as u8;
        }
        if let Some(v) = number("Magic Points") {
            investigator.magic_points = v as u8;
        }
        if let Some(v) = number("Luck") {
            investigator.luck = v as u8;
        }
        if let Some(v) = number("Sanity") {
            investigator.sanity = v as u8;
        }
        if let Some(v) = number("Build") {
            investigator.build = v as i8;
        }
        if let Some(v) = number("Move Rate") {
            investigator.move_rate = v as u8;
        }
    }
    if let Some(skills) = root.get("Skills").and_then(Value::as_object) {
        for (key, value) in skills {
            let n = numbers::<5>(value);
            let points = (n[0] as u16, n[1] as u16, n[2] as u16, n[3] as i16, n[4] as i16);
            if !skill_into(investigator, key, points) {
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
                Some(_) if key == "Key Connection" => match backstory_of(text) {
                    Some(target) => investigator.key_connection = Some(target),
                    None => skipped.push(key.clone()),
                },
                Some(entry) => {
                    if let Some(index) = entry.index() {
                        investigator.backstory[index] = String::from(text);
                    }
                }
                None => skipped.push(key.clone()),
            }
        }
    }
    if let Some(notes) = root.get("Notes").and_then(Value::as_object) {
        for (key, value) in notes.iter().take(MEMO_SLOTS) {
            let body = match value {
                Value::String(text) => text.clone(),
                Value::Array(items) => {
                    items.iter().map(scenario_line).collect::<Vec<_>>().join("\n")
                }
                other => other.to_string(),
            };
            investigator.memos.push(Memo { title: key.clone(), body });
        }
    }
    for key in ["Possessions", "Wealth"] {
        if root.get(key).is_some() {
            skipped.push(String::from(key));
        }
    }
    Ok(Imported { investigator: record, skipped })
}

fn profile_into(investigator: &mut Investigator, profile: &Value) {
    let text = |key: &str| profile.get(key).and_then(Value::as_str).map(String::from);
    investigator.name = text("Name").unwrap_or_default();
    investigator.pronunciation = text("Pronunciation").unwrap_or_default();
    if let Some(occupation) = text("Occupation") {
        let kind = Occupation::list()
            .iter()
            .find(|kind| Occupation::display(kind, None, None, EN) == occupation)
            .copied();
        match kind {
            Some(kind) => investigator.occupation = kind,
            None => {
                investigator.occupation = OccupationKind::Custom;
                investigator.occupation_name = occupation;
            }
        }
    }
    investigator.title = text("Title").unwrap_or_default();
    investigator.birthplace = text("Birthplace").unwrap_or_default();
    investigator.pronoun = text("Pronoun").unwrap_or_default();
    investigator.residence = text("Residence").unwrap_or_default();
    if let Some(age) = profile.get("Age").and_then(Value::as_u64) {
        investigator.age = age as u16;
    }
}

fn skill_into(
    investigator: &mut Investigator,
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
    let points = Points::from((occupation, interest, change, modifier));
    let known = |candidates: &[Specialty]| {
        candidates.iter().find(|candidate| candidate.display(EN) == spec).copied()
    };
    let specialty = match skill {
        Skill::LanguageOwn => Specialty::LanguageOwn,
        Skill::LanguageOther if blank => return true,
        Skill::LanguageOther => Specialty::LanguageOther,
        Skill::Custom => return false,
        Skill::ArtAndCraft => known(
            &ArtAndCraft::list().iter().map(|a| Specialty::ArtAndCraft(*a)).collect::<Vec<_>>(),
        )
        .unwrap_or(Specialty::ArtAndCraft(ArtAndCraft::Custom)),
        Skill::Fighting => {
            known(&Fighting::list().iter().map(|f| Specialty::Fighting(*f)).collect::<Vec<_>>())
                .unwrap_or(Specialty::Fighting(Fighting::Custom))
        }
        Skill::Firearms => {
            known(&Firearms::list().iter().map(|f| Specialty::Firearms(*f)).collect::<Vec<_>>())
                .unwrap_or(Specialty::Firearms(Firearms::Custom))
        }
        Skill::Pilot => {
            known(&Pilot::list().iter().map(|p| Specialty::Pilot(*p)).collect::<Vec<_>>())
                .unwrap_or(Specialty::Pilot(Pilot::Custom))
        }
        Skill::Science => {
            known(&Science::list().iter().map(|s| Specialty::Science(*s)).collect::<Vec<_>>())
                .unwrap_or(Specialty::Science(Science::Custom))
        }
        Skill::Survival => {
            known(&Survival::list().iter().map(|s| Specialty::Survival(*s)).collect::<Vec<_>>())
                .unwrap_or(Specialty::Survival(Survival::Custom))
        }
        plain => {
            *investigator.points_mut(&plain) = points;
            return true;
        }
    };
    if !specialty.known() && blank {
        return true;
    }
    investigator.specializations.push(Specialization {
        specialty,
        name: if specialty.known() { String::new() } else { String::from(spec) },
        base: if specialty.known() { specialty.base_percent() as u8 } else { base as u8 },
        points,
    });
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
    if name == "Key Connection" {
        return Some(Backstory::KeyConnection(alloc::boxed::Box::new(
            Backstory::PersonalDescription,
        )));
    }
    let wanted = match name {
        "Traits" => "Trait",
        "Meaningful Locations" => "Meaningful Location",
        "Treasured Possessions" => "Treasured Possession",
        other => other,
    };
    Backstory::list().into_iter().find(|entry| entry.display(EN) == wanted)
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
    use alloc::vec::Vec;

    use super::*;

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
        assert_eq!(sheet.head.id, 5);
        assert_eq!((sheet.name.as_str(), sheet.pronunciation.as_str()), ("鏖", "じん"));
        assert_eq!(Occupation::display(&sheet.occupation, None, None, EN), "Soldier");
        assert_eq!(sheet.birthplace, "日本");
        assert_eq!(sheet.pronoun, "he/him");
        assert_eq!(sheet.age, 39);
    }

    #[test]
    fn characteristics_and_secondary_attributes_are_read() {
        let sheet = imported().investigator;
        assert_eq!(
            Characteristic::Strength.score(&sheet),
            Score { initial: 75, change: 0, modifier: 0 }
        );
        assert_eq!(
            Characteristic::Education.score(&sheet),
            Score { initial: 75, change: 5, modifier: 0 }
        );
        assert_eq!((sheet.hit_points, sheet.luck, sheet.build), (13, 35, 1));
    }

    #[test]
    fn skills_are_placed_by_name_and_specialization() {
        let sheet = imported().investigator;
        assert_eq!(sheet.points(&Skill::Dodge).occupation, 48);
        assert_eq!(sheet.points(&Skill::CthulhuMythos), Points::from((0, 0, 24, 0)));
        let sword = sheet.known_specialization(&Skill::Fighting).unwrap();
        assert_eq!(sword.specialty, Specialty::Fighting(Fighting::Sword));
        assert_eq!((sword.base, sword.points.occupation), (20, 60));
        let english = sheet.custom_specialization(&Skill::LanguageOther, 0).unwrap();
        assert_eq!((english.name.as_str(), english.points.occupation), ("English", 29));
        assert!(sheet.custom_specialization(&Skill::LanguageOwn, 0).is_none());
        let origami = sheet.custom_specialization(&Skill::ArtAndCraft, 0).unwrap();
        assert_eq!((origami.name.as_str(), origami.points.interest), ("Origami", 10));
        assert!(sheet.known_specialization(&Skill::ArtAndCraft).is_none());
    }

    #[test]
    fn unknown_entries_are_reported_and_the_rest_is_kept() {
        let skipped = imported().skipped;
        assert_eq!(skipped, Vec::from(["Operate Heavy Machinery", "Possessions"]));
    }

    #[test]
    fn backstory_and_notes_are_read() {
        let sheet = imported().investigator;
        assert_eq!(sheet.text(&Backstory::IdeologyAndBeliefs), "後悔無きように");
        assert_eq!(sheet.text(&Backstory::Trait), "低音声");
        assert_eq!(sheet.key_connection, Some(Backstory::IdeologyAndBeliefs));
        assert_eq!(
            sheet.memos,
            [
                Memo { title: "Personality".to_string(), body: "快楽殺人鬼".to_string() },
                Memo { title: "Scenarios".to_string(), body: "A\nB: 共闘".to_string() },
            ]
        );
    }

    #[test]
    fn broken_json_is_a_json_error_and_a_list_is_a_root_error() {
        assert!(matches!(import(b"{", 0), Err(ImportError::Json(_))));
        assert!(matches!(import(b"[]", 0), Err(ImportError::Root)));
    }
}
