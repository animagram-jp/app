use alloc::{format, string::String, vec, vec::Vec};
use core::{
    matches,
    option::Option::{None, Some},
    primitive::{f64, i32, u8, u32, u64},
};

use arbitrary_int::u2;

use crate::{
    DataError, Error, Language,
    coc7th::{
        import,
        investigator::{
            Age, ArtAndCraft, ArtAndCraftCustom, Backstory, Birthplace, Build, Characteristic,
            DamageBonus, Fighting, FightingCustom, Firearms, FirearmsCustom, HitPoints,
            InterestSkillPoints, Investigator, LanguageOther, LanguageOwn, Luck, MagicPoints,
            Management, Memo, MoveRate, Name, Occupation, OccupationSkillPoints, Pilot,
            PilotCustom, Possession, Profile, Pronoun, Residence, SCHEMA_SIZE, Sanity, Science,
            ScienceCustom, SecondaryAttribute, Skill, Survival, SurvivalCustom, WealthField,
            read_skill,
        },
        page::{self, Page},
    },
    data_struct::{DataStruct, ID_IDENTITY},
    event::{self, Event, Opened, Response, gesture::Pointer, store},
    file_store::{FileStore, Mount, StoreId},
    host::{Attribute, CanvasEvent, Command, EventType, Method, dom::Id},
    object::{Predicate as _, SubjectTrait as _},
};

pub enum Dialog {
    None,
    Drawer,
    Select { step: u8, index: u32 },
    Input { step: u8, value: u32 },
}

pub struct Log;
const CHARACTER_STORE: StoreId = StoreId { name: "investigators", version: "0.0" };
const LOAD_REQUEST: u32 = 1;
const STATUS_OK: u16 = 200;

pub struct Handler<S: FileStore> {
    data_url:      String,
    dialog:        Dialog,
    language:      Language,
    last_toast:    u2,
    investigator:  DataStruct,
    pub now_in_ms: u64,
    investigators: Option<S>,
    lost:          Vec<(u32, Option<Vec<u8>>)>,
    startup:       Vec<Error>,
    logs:          Vec<Log>,
}

impl<S: FileStore> Handler<S> {
    pub fn save(&mut self) -> (Vec<Event>, Vec<Command>) {
        let Some(investigators) = self.investigators.as_mut() else {
            return (vec![], vec![]);
        };
        match investigators.save() {
            Ok(()) => (vec![], vec![]),
            Err(error) => store::failed(CHARACTER_STORE, error),
        }
    }

    fn persist(&mut self) -> (Vec<Event>, Vec<Command>) {
        let Some(store) = self.investigators.as_mut() else {
            return (vec![], vec![]);
        };
        let key = store.issue_id();
        store.set(key, self.investigator.to_bytes());
        self.save()
    }
}

impl<S: Mount> event::Handler for Handler<S> {
    type Store = S;

    async fn ready(
        _viewport_width_in_px: f64,
        _viewport_height_in_px: f64,
        _rem_in_px: f64,
        now_in_ms: u64,
        _timezone_offset_minutes: i32,
        data_url: String,
    ) -> Self {
        let mut handler = Self {
            data_url,
            dialog: Dialog::None,
            language: Language::Ja,
            last_toast: u2::new(1),
            investigator: DataStruct::new(0, 0, SCHEMA_SIZE),
            now_in_ms,
            investigators: None,
            lost: Vec::new(),
            startup: Vec::new(),
            logs: Vec::new(),
        };
        match store::open::<S>(CHARACTER_STORE).await {
            Ok(store) => handler.investigators = Some(store),
            Err(error) => handler.startup.push(Error::FileStore(error)),
        }
        handler
    }

    fn close(&self) -> Vec<Command> {
        if let Some(store) = &self.investigators {
            store.close();
        }
        vec![]
    }
    fn process_lost(&mut self) -> (Vec<Event>, Vec<Command>) {
        self.lost = store::lost(&mut self.investigators);
        (vec![], vec![])
    }
    fn process_opened(&mut self, opened: Opened<S>) -> (Vec<Event>, Vec<Command>) {
        store::opened(&mut self.investigators, &mut self.lost, opened)
    }

    fn initial_draw(&mut self) -> (Vec<Event>, Vec<Command>) {
        let events = store::startup(CHARACTER_STORE, &self.startup);
        let mut commands = Vec::new();
        commands.extend(self.startup.drain(..).map(|error| Command::Error { error }));
        commands.extend(language_commands(self.language));
        commands.extend(Language::list().iter().flat_map(|&l| option_commands(l)));
        commands.extend(label_commands(self.language));
        if !self.data_url.is_empty() {
            commands.push(Command::Fetch {
                request: LOAD_REQUEST,
                method:  Method::Get,
                path:    self.data_url.clone(),
                body:    Vec::new(),
            });
        }
        commands.push(Command::RemoveAttribute {
            id:        Page::Body.to_dom(),
            attribute: Attribute::Hidden,
        });
        (events, commands)
    }

    fn process_fetched(&mut self, response: &Response) -> (Vec<Event>, Vec<Command>) {
        if response.request != LOAD_REQUEST {
            return (vec![], vec![]);
        }
        if response.status != STATUS_OK {
            return (vec![], vec![data_error(DataError::Status(response.status))]);
        }
        let imported = match import::import(&response.body, self.now_in_ms) {
            Ok(imported) => imported,
            Err(error) => return (vec![], vec![data_error(error)]),
        };
        self.investigator = imported.investigator;
        let mut commands = sheet_commands(&self.investigator, self.language);
        if !imported.skipped.is_empty() {
            commands.push(data_error(DataError::Format(format!(
                "skipped: {}",
                imported.skipped.join(", ")
            ))));
        }
        (vec![], commands)
    }

    fn process_canvas(
        &mut self,
        event: &CanvasEvent,
        _pointer: &Pointer,
    ) -> (Vec<Event>, Vec<Command>) {
        let page = Page::from_dom(&event.id);
        if !matches!(event.event_type, EventType::Click) {
            return (vec![], vec![]);
        }
        match page {
            Some(Page::LanguageOption(l)) => {
                self.language = l;
                let mut commands = Vec::from(language_commands(l));
                commands.extend(label_commands(l));
                commands.extend(sheet_commands(&self.investigator, l));
                (vec![], commands)
            }
            Some(Page::Save) => self.persist(),
            _ => (vec![], vec![]),
        }
    }
}

fn text(id: Id, value: impl Into<String>) -> Command {
    Command::SetText { id, value: value.into() }
}

fn value(id: Id, value: impl Into<String>) -> Command {
    Command::SetValue { id, value: value.into() }
}

fn stored_text(investigator: &DataStruct, id: u32) -> String {
    investigator.get(id).ok().map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default()
}

fn skill_sum(base: u16, (occupation, interest, change, modifier): (u16, u16, i16, i16)) -> i32 {
    base as i32 + occupation as i32 + interest as i32 + change as i32 + modifier as i32
}

fn known_specialization(
    skill: &Skill,
    investigator: &DataStruct,
    language: Language,
) -> Option<(String, (u16, u16, i16, i16))> {
    let pick = |candidates: Vec<(String, u32)>| {
        candidates
            .into_iter()
            .map(|(name, id)| (name, read_skill(investigator, id)))
            .find(|(_, points)| *points != (0, 0, 0, 0))
    };
    match skill {
        Skill::ArtAndCraft => pick(
            ArtAndCraft::list()
                .iter()
                .filter(|a| **a != ArtAndCraft::Custom)
                .map(|a| (String::from(a.display(language)), a.base_id()))
                .collect(),
        ),
        Skill::Fighting => pick(
            Fighting::list()
                .iter()
                .filter(|f| **f != Fighting::Custom)
                .map(|f| (String::from(f.display(language)), f.id(Skill::Fighting.base_id())))
                .collect(),
        ),
        Skill::Firearms => pick(
            Firearms::list()
                .iter()
                .filter(|f| **f != Firearms::Custom)
                .map(|f| (String::from(f.display(language)), f.id(Skill::Firearms.base_id())))
                .collect(),
        ),
        Skill::Pilot => pick(
            Pilot::list()
                .iter()
                .filter(|p| **p != Pilot::Custom)
                .map(|p| (String::from(p.display(language)), p.id(Skill::Pilot.base_id())))
                .collect(),
        ),
        Skill::Science => pick(
            Science::list()
                .iter()
                .filter(|s| **s != Science::Custom)
                .map(|s| {
                    (
                        String::from(s.display(language).unwrap_or("")),
                        s.id(Skill::Science.base_id()),
                    )
                })
                .collect(),
        ),
        Skill::Survival => pick(
            Survival::list()
                .iter()
                .filter(|s| **s != Survival::Custom)
                .map(|s| (String::from(s.display(language)), s.id(Skill::Survival.base_id())))
                .collect(),
        ),
        _ => None,
    }
}

fn free_specialization(skill: &Skill, investigator: &DataStruct) -> (u16, u16, i16, i16, String) {
    match skill {
        Skill::ArtAndCraft => ArtAndCraftCustom(0).read(investigator),
        Skill::Fighting => {
            let (_, o, i, c, m, name) = FightingCustom(0).read(investigator);
            (o, i, c, m, name)
        }
        Skill::Firearms => {
            let (_, o, i, c, m, name) = FirearmsCustom(0).read(investigator);
            (o, i, c, m, name)
        }
        Skill::LanguageOther => LanguageOther(1).read(investigator),
        Skill::Pilot => PilotCustom(0).read(investigator),
        Skill::Science => ScienceCustom(0).read(investigator),
        Skill::Survival => SurvivalCustom(0).read(investigator),
        _ => (0, 0, 0, 0, String::new()),
    }
}

fn skill_commands(investigator: &DataStruct, language: Language) -> Vec<Command> {
    let mut commands = Vec::new();
    for skill in Skill::list() {
        if *skill == Skill::Custom {
            continue;
        }
        let base = skill.base_percent();
        let rows = skill.rows();
        let (name, points) = match skill {
            Skill::LanguageOwn => {
                let (o, i, c, m, name) = LanguageOwn.read(investigator);
                (name, (o, i, c, m))
            }
            Skill::LanguageOther => {
                let (o, i, c, m, name) = LanguageOther(0).read(investigator);
                (name, (o, i, c, m))
            }
            specialized if specialized.free_row().is_some() => {
                known_specialization(specialized, investigator, language)
                    .unwrap_or((String::new(), (0, 0, 0, 0)))
            }
            plain => (String::new(), read_skill(investigator, plain.base_id())),
        };
        let mut rows_values = Vec::from([(rows[0], name, points)]);
        if let Some(free) = rows.get(1) {
            let (o, i, c, m, name) = free_specialization(skill, investigator);
            rows_values.push((*free, name, (o, i, c, m)));
        }
        for (row, name, points) in rows_values {
            let cells = Skill::cells(row);
            commands.push(value(cells.specialization, name));
            commands.push(text(cells.base, format!("{base}")));
            commands.push(value(cells.occupation, format!("{}", points.0)));
            commands.push(value(cells.interest, format!("{}", points.1)));
            commands.push(value(cells.change, format!("{}", points.2)));
            commands.push(value(cells.modifier, format!("{}", points.3)));
            commands.push(text(cells.sum, format!("{}", skill_sum(base, points))));
        }
    }
    commands
}

fn label_commands(language: Language) -> Vec<Command> {
    let mut commands: Vec<Command> =
        page::every_label(language).into_iter().map(|(id, label)| text(id, label)).collect();
    commands
        .extend(Management::list().iter().map(|m| text(m.dom()[0].clone(), m.display(language))));
    commands.extend(
        Investigator::list().iter().map(|part| text(part.dom()[0].clone(), part.display(language))),
    );
    commands.extend(
        Profile::list().iter().map(|field| text(field.dom()[0].clone(), field.display(language))),
    );
    commands.extend(
        Characteristic::list().iter().map(|c| text(c.dom()[0].clone(), c.display(language))),
    );
    commands.extend(
        SecondaryAttribute::list().iter().map(|a| text(a.dom()[0].clone(), a.display(language))),
    );
    for skill in Skill::list() {
        for row in skill.rows() {
            commands.push(text(Skill::cells(row).name, skill.display(language)));
        }
        commands.extend(skill.options(language).into_iter().map(|(id, name)| text(id, name)));
    }
    commands
        .extend(Possession::list().iter().map(|p| text(p.dom()[0].clone(), p.display(language))));
    commands
        .extend(WealthField::list().iter().map(|f| text(f.dom()[0].clone(), f.display(language))));
    commands.extend(
        Backstory::list()
            .iter()
            .map(|entry| text(entry.dom().swap_remove(0), entry.display(language))),
    );
    commands
}

fn sheet_commands(investigator: &DataStruct, language: Language) -> Vec<Command> {
    let mut commands = Vec::new();
    let (name, complement) = Name::read(investigator);
    commands.push(text(Page::Investigator.to_dom(), name.clone()));
    let identity = investigator
        .get(ID_IDENTITY)
        .ok()
        .and_then(|b| b.get(0..4)?.try_into().ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0);
    let [_, identity_dom] = Management::Identity.dom();
    commands.push(text(identity_dom, format!("{identity}")));

    let name_dom = Profile::Name.dom();
    commands.push(value(name_dom[1].clone(), name));
    commands.push(value(name_dom[2].clone(), complement.unwrap_or_default()));
    let (kind, custom, title) = Occupation::read(investigator);
    let occupation_dom = Profile::Occupation.dom();
    commands.push(value(
        occupation_dom[1].clone(),
        Occupation::display(&kind, custom.as_deref(), None, language),
    ));
    commands.push(value(occupation_dom[2].clone(), title.unwrap_or_default()));
    commands.push(value(Profile::Birthplace.dom()[1].clone(), Birthplace::read(investigator)));
    commands.push(value(Profile::Pronoun.dom()[1].clone(), Pronoun::read(investigator)));
    commands.push(value(Profile::Residence.dom()[1].clone(), Residence::read(investigator)));
    commands.push(value(Profile::Age.dom()[1].clone(), format!("{}", Age::read(investigator))));

    for characteristic in Characteristic::list() {
        let (initial, change, modifier) = characteristic.read(investigator);
        let [_, initial_dom, change_dom, modifier_dom, sum_dom] = characteristic.dom();
        commands.push(value(initial_dom, format!("{initial}")));
        commands.push(value(change_dom, format!("{change}")));
        commands.push(value(modifier_dom, format!("{modifier}")));
        commands.push(text(sum_dom, format!("{}", characteristic.sum(investigator))));
    }

    for attribute in SecondaryAttribute::list() {
        let shown = match attribute {
            SecondaryAttribute::HitPoints => format!("{}", HitPoints::read(investigator)),
            SecondaryAttribute::MagicPoints => format!("{}", MagicPoints::read(investigator)),
            SecondaryAttribute::Luck => format!("{}", Luck::read(investigator)),
            SecondaryAttribute::Sanity => format!("{}", Sanity::read(investigator)),
            SecondaryAttribute::Build => format!("{}", Build::read(investigator)),
            SecondaryAttribute::DamageBonus => DamageBonus::display(investigator),
            SecondaryAttribute::MoveRate => format!("{}", MoveRate::read(investigator)),
            SecondaryAttribute::OccupationSkillPoints => {
                format!("{}", OccupationSkillPoints::derive(investigator).1)
            }
            SecondaryAttribute::InterestSkillPoints => {
                format!("{}", InterestSkillPoints::derive(investigator).1)
            }
        };
        let [_, slot] = attribute.dom();
        commands.push(if attribute.editable() { value(slot, shown) } else { text(slot, shown) });
    }

    commands.extend(skill_commands(investigator, language));

    let key_connection = Backstory::KeyConnection(alloc::boxed::Box::new(Backstory::Trait)).id();
    let connected = investigator.get(key_connection).ok().and_then(|b| b.first().copied());
    for entry in Backstory::list() {
        let textarea = entry.dom().swap_remove(1);
        commands.push(value(textarea, stored_text(investigator, entry.id())));
        let checkbox =
            Backstory::KeyConnection(alloc::boxed::Box::new(entry.clone())).dom().swap_remove(0);
        commands.push(if connected == Some(entry.id() as u8) {
            Command::SetAttribute {
                id:        checkbox,
                attribute: Attribute::Checked,
                value:     String::new(),
            }
        } else {
            Command::RemoveAttribute { id: checkbox, attribute: Attribute::Checked }
        });
    }

    for memo in Memo::list() {
        let (title, body) = Memo::decode(investigator.get(memo.id()).unwrap_or(&[]));
        let [title_dom, body_dom] = memo.dom();
        commands.push(value(title_dom, title));
        commands.push(value(body_dom, body));
    }
    commands
}

fn language_commands(language: Language) -> [Command; 1] {
    let (tag, _) = language.display();
    [Command::SetAttribute {
        id:        Page::Html.to_dom(),
        attribute: Attribute::Lang,
        value:     tag.into(),
    }]
}

fn data_error(error: DataError) -> Command {
    Command::Error { error: Error::Data(error) }
}

fn option_commands(language: Language) -> [Command; 2] {
    let (tag, label) = language.display();
    let id = Page::LanguageOption(language).to_dom();
    [
        Command::SetAttribute {
            id:        id.clone(),
            attribute: Attribute::Lang,
            value:     tag.into(),
        },
        Command::SetText { id, value: label.into() },
    ]
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use alloc::vec::Vec;

    use wasm_bindgen_test::*;

    use crate::{
        En, Language, Output,
        app::App,
        arena::{ARENA, COMMAND_RING, EVENT_RING, initialize, process_event},
        coc7th::page::Page,
        host::{EVENT_CANVAS, EventType, OPERATION_SET_ATTRIBUTE},
    };

    const CLICK: u8 = 3;

    #[wasm_bindgen_test]
    async fn process_event_emits_one_frame_per_command() {
        assert_eq!(EventType::from_u8(CLICK), EventType::Click);
        initialize();
        App::init(false, 0.0, 0.0, 16.0, 0, 0).await;
        while ARENA.read_ring(COMMAND_RING).is_some() {
            ARENA.advance_ring(COMMAND_RING);
        }

        let mut frame = Vec::new();
        frame.push(EVENT_CANVAS);
        frame.push(CLICK);
        Page::LanguageOption(Language::En(En::Gb)).to_dom().encode(&mut frame);
        frame.push(0);
        frame.push(0);
        str::encode("", &mut frame);
        0.0f32.encode(&mut frame);
        0.0f32.encode(&mut frame);
        0.0f32.encode(&mut frame);
        0.0f32.encode(&mut frame);
        frame.extend_from_slice(&0f64.to_le_bytes());
        frame.extend_from_slice(&0u32.to_le_bytes());
        assert!(ARENA.write_ring(EVENT_RING, &frame));

        process_event();

        let mut operations = Vec::new();
        while let Some(command) = ARENA.read_ring(COMMAND_RING) {
            operations.push(command[0]);
            ARENA.advance_ring(COMMAND_RING);
        }
        assert_eq!(operations, [OPERATION_SET_ATTRIBUTE]);
    }
}

#[cfg(test)]
mod handler_tests {
    use alloc::{string::String, vec::Vec};

    use super::*;
    use crate::{
        En, block_on,
        event::Handler as _,
        file_store::{FileStoreError, MemoryStore, Mount},
        host::{Flags, KeyName, dom},
    };

    fn click(handler: &mut Handler<MemoryStore>, id: dom::Id) -> Vec<Command> {
        let event = CanvasEvent {
            event_type: EventType::Click,
            id,
            key: KeyName::Other,
            flags: Flags::ZERO,
            value: String::new(),
            client_x: 0.0,
            client_y: 0.0,
            local_x: 0.0,
            local_y: 0.0,
            time_stamp: 0.0,
            pointer_id: 0,
        };
        handler.process_canvas(&event, &Pointer::NONE).1
    }

    fn hidden_change(command: &Command) -> (bool, dom::Id) {
        match command {
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. } => (true, id.clone()),
            Command::RemoveAttribute { id, attribute: Attribute::Hidden } => (false, id.clone()),
            _ => panic!("unexpected command"),
        }
    }

    fn written(command: &Command) -> (dom::Id, Option<Attribute>, String) {
        match command {
            Command::SetText { id, value } => (id.clone(), None, value.clone()),
            Command::SetAttribute { id, attribute, value } => {
                (id.clone(), Some(*attribute), value.clone())
            }
            _ => panic!("unexpected command"),
        }
    }

    fn tagged(root: Page, language: Language) -> Vec<(dom::Id, Option<Attribute>, String)> {
        let (tag, _) = language.display();
        Vec::from([(root.to_dom(), Some(Attribute::Lang), String::from(tag))])
    }

    fn labelled(target: Page, language: Language) -> Vec<(dom::Id, Option<Attribute>, String)> {
        let (_, label) = language.display();
        let mut written = tagged(target, language);
        written.push((target.to_dom(), None, String::from(label)));
        written
    }

    #[test]
    fn the_initial_draw_tags_the_language_and_reveals_the_body() {
        let mut handler =
            block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0, String::new()));
        let (events, commands) = handler.initial_draw();
        assert!(events.is_empty());
        let (texts, reveal) = commands.split_at(commands.len() - 1);
        let expected: Vec<_> = tagged(Page::Html, Language::Ja)
            .into_iter()
            .chain(Language::list().iter().flat_map(|&l| labelled(Page::LanguageOption(l), l)))
            .collect();
        assert_eq!(texts[..expected.len()].iter().map(written).collect::<Vec<_>>(), expected);
        let labels = &texts[expected.len()..];
        assert_eq!(labels.len(), label_commands(Language::Ja).len());
        assert!(labels.iter().all(|command| matches!(command, Command::SetText { .. })));
        assert!(labels.iter().any(|command| matches!(
            command,
            Command::SetText { value, .. } if value == Investigator::Skill.display(Language::Ja)
        )));
        assert_eq!(
            reveal.iter().map(hidden_change).collect::<Vec<_>>(),
            [(false, Page::Body.to_dom())]
        );
    }

    #[test]
    fn choosing_a_language_tags_the_document() {
        let mut handler =
            block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0, String::new()));
        let english = Language::En(En::Gb);
        let commands = click(&mut handler, Page::LanguageOption(english).to_dom());
        assert_eq!(
            commands.iter().take(1).map(written).collect::<Vec<_>>(),
            tagged(Page::Html, english)
        );
        assert_eq!(handler.language, english);
        let redrawn = commands.len();
        assert!(redrawn > 1);
        let commands = click(&mut handler, Page::LanguageOption(Language::Ja).to_dom());
        assert_eq!(
            commands.iter().take(1).map(written).collect::<Vec<_>>(),
            tagged(Page::Html, Language::Ja)
        );
        assert_eq!(commands.len(), redrawn);
        assert_eq!(handler.language, Language::Ja);
    }

    #[test]
    fn other_targets_and_events_change_nothing() {
        let mut handler =
            block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0, String::new()));
        assert!(click(&mut handler, Page::Sheet.to_dom()).is_empty());
        assert!(click(&mut handler, Page::Dice.to_dom()).is_empty());
    }
    const SAMPLE: &str = r#"{ "ID": 5, "Profile": { "Name": "鏖" }, "Wealth": {} }"#;

    fn response(status: u16) -> Response {
        Response { request: LOAD_REQUEST, status, body: SAMPLE.as_bytes().to_vec() }
    }

    #[test]
    fn a_data_url_is_fetched_on_the_initial_draw() {
        let url = String::from("investigator/jin.json");
        let mut handler =
            block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0, url.clone()));
        let (_, commands) = handler.initial_draw();
        let fetches: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                Command::Fetch { request, path, .. } => Some((*request, path.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(fetches, [(LOAD_REQUEST, url)]);
        let mut handler =
            block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0, String::new()));
        let (_, commands) = handler.initial_draw();
        assert!(!commands.iter().any(|command| matches!(command, Command::Fetch { .. })));
    }

    #[test]
    fn a_fetched_sheet_is_drawn_and_saved_on_demand() {
        let mut handler =
            block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0, String::new()));
        let (events, commands) = handler.process_fetched(&response(STATUS_OK));
        assert!(events.is_empty());
        let [
            Command::SetText { id, value },
            ..,
            Command::Error { error: Error::Data(DataError::Format(message)) },
        ] = commands.as_slice()
        else {
            panic!("unexpected commands");
        };
        assert_eq!(*id, Page::Investigator.to_dom());
        assert_eq!(value, "鏖");
        assert_eq!(message, "skipped: Wealth");
        assert_eq!(Name::read(&handler.investigator).0, "鏖");
        assert!(handler.investigators.as_ref().unwrap().get(1).is_none());
        let written: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                Command::SetValue { id, value } => Some((id.clone(), value.clone())),
                _ => None,
            })
            .collect();
        let name_input = Profile::Name.dom()[1].clone();
        assert!(written.contains(&(name_input, String::from("鏖"))));
        assert_eq!(handler.investigators.as_ref().unwrap().get(1), None);

        assert!(click(&mut handler, Page::Save.to_dom()).is_empty());
        let stored = handler.investigators.as_ref().unwrap().get(1).unwrap();
        assert_eq!(
            DataStruct::from_bytes(stored, SCHEMA_SIZE).map(|s| Name::read(&s).0).ok(),
            Some(String::from("鏖"))
        );
    }

    #[test]
    fn a_failed_fetch_reports_the_status_and_other_requests_are_ignored() {
        let mut handler =
            block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0, String::new()));
        let (_, commands) = handler.process_fetched(&response(404));
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::Data(DataError::Status(404)) }]
        ));
        let other = Response { request: 9, status: STATUS_OK, body: Vec::new() };
        assert!(handler.process_fetched(&other).1.is_empty());
    }

    #[test]
    fn closing_the_handler_releases_the_store() {
        let mut handler =
            block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0, String::new()));
        assert!(handler.save().1.is_empty());
        assert!(handler.close().is_empty());
        let (events, commands) = handler.save();
        assert!(matches!(events.as_slice(), [Event::StoreLost(_)]));
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::FileStore(FileStoreError::InvalidState(_)) }]
        ));
    }
    #[test]
    fn a_lost_store_is_replaced_and_the_pending_change_survives() {
        let mut handler =
            block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0, String::new()));
        handler.investigators.as_mut().unwrap().set(1, b"kept".to_vec());
        let disk = block_on(MemoryStore::open(CHARACTER_STORE, true)).unwrap();

        handler.close();
        let (events, _) = handler.save();
        assert!(matches!(events.as_slice(), [Event::StoreLost(_)]));
        handler.process_lost();
        handler.process_opened(Ok(disk.clone()));
        assert_eq!(handler.investigators.as_ref().unwrap().get(1), Some(&b"kept"[..]));
        assert!(handler.save().1.is_empty());
        assert_eq!(MemoryStore::new(disk).unwrap().get(1), Some(&b"kept"[..]));
    }
}
