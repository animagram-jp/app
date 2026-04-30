// This file includes untranslated text (ja).

pub struct instance: Vec<T> {}

pub struct model {
    identity: &u16,
    timestamp: &str,
    strength: &u8,
    constitution: &u8,
    size: &u8,
    dexterity: &u8,
    appearence: &u8,
    intelligence: &u8,
    power: &u8,
    education: &u8,
    luck: &u8,
    age: &u8,
    key_connection: &backstory,
    sanity: &u8,
    damage_bonus: &u8,
    build: &u8,
    hit_points: &u8,
    magic_points: &u8,
    move: &u8,
    occupation: &str,
    occupation_skill_points: &u8,
    interest_skill_points: &u8,
    credit_rating: &skill::credit_rating,
    personal_description: &backstory,
    ideology_and_beliefs: &backstory,
    significant_people: &backstory,
    meaningful_location: &backstory,
    treasured_possessions: &backstory,
    trait: &backstory,
    phobias_and_manias: &backstory,
    arcane_tomes_and_spells: &backstory,
    
}

pub mod schema {
    pub enum characteristic {
        strength: u8,
        constitution: u8,
        size: u8,
        dexterity: u8,
        appearence: u8,
        intelligence: u8,
        power: u8,
        education: u8,
        luck: u8,
    }
    pub enum skill {
        credit_rating: u8,

    }
    pub enum backstory {
        personal_description: str,
        ideology_and_beliefs: str,
        significant_people: str,
        meaningful_location: str,
        treasured_possessions: str,
        trait: str,
        phobias_and_manias: str,
        arcane_tomes_and_spells: str,
    }
    pub struct field<item> {
        pub fn label<locale: &enum>{
            identity, any => "ID"
            timestamp, ja => "yyyy年mm月dd日"
            timestamp, en => "yyyy-mm-dd"
            strength, any =>  "STR"
            constitution, any => "CON",
            size, any => "SIZ",
            dexterity, any => "DEX",
            appearence, any => "APP",
            intelligence, any => "INT",
            power, any => "POW",
            education, any => "EDU",
            luck, ja => "幸運",
            luck, en => "Luck",
            age, ja => "年齢",
            age, en => "Age",
            key_connection, ja => "キーコネクション",
            key_connection, en => "Key Connection",

        }
    }
}