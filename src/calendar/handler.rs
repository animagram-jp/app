use alloc::{collections::BTreeMap, format, string::String, vec, vec::Vec};
use core::{
    cell::RefCell,
    option::Option::{self, None, Some},
    primitive::{f64, i32, i64, u32, u64, usize},
    result::Result::Ok,
};

use rectgrid::{
    BBox, IncrementFunction, Px, RectGrid, Unit as GridUnit, corner_test, drag_resize,
    drag_translate,
};

use crate::{
    Error, Lang,
    calendar::{
        data::{
            self, DataError,
            appointment::{self, Place, shift},
            format_hhmm, parse_date, parse_time, tag,
        },
        grid::{
            Cell, ColumnAxis, DAY_MAX, DAY_START_MINUTES, Grid, MONTH_AXIS, RESOURCE_COUNT,
            SLOT_COUNT, SLOT_MINUTES, TIME_AXIS, View, lane_box, lanes, span,
        },
        target::{CardPart, EditField, Target},
    },
    data_struct::DataStruct,
    event::{Event, Opened, Response},
    file_store::{Backend, FileStore, FileStoreError, StoreId},
    js_client::{
        Attribute, CanvasEvent, Command, Decimal, EventType, FullscreenEvent, Gesture, Keyword,
        Method, Pointer, StyleProperty, StyleValue, Unit, VisibilityState, dom::Id, parse,
        parse_url_search_params, stringify,
    },
    object::StaticModel,
    timestamp::{
        Format, Timezone, add_days, diff, display, from_ut, pack, sub_days, unpack, youbi,
    },
};

const CARD_POOL: usize = 380;
const DRAG_Z_INDEX: i32 = 1000;
const HANDLE_REM: f64 = 0.5;
const HANDLE_MAX: f64 = 0.35;
const STORE: StoreId = StoreId { name: "calendar", version: "0.1" };
const NEW_MINUTES: u32 = 60;
const SLOT_REM: f64 = 1.75;
const ZOOM_MIN: f64 = 1.0;
const ZOOM_MAX: f64 = 3.0;
const COLUMN_MIN_REM: f64 = 5.0;
const AXIS_REM: f64 = 4.0;
const HEAD_REM: f64 = 5.0;
const STEP_COUNT: u32 = 5;
const LANG: Lang = Lang::Ja;
const DAY: i64 = 8_640_000;
const STEP_TODAY: u32 = 3;
const LOAD_REQUEST: u32 = 1;
const LOAD_PATH: &str = "data/calendar.json";
const STATUS_OK: u16 = 200;
const STATUS_POOL: u32 = 4;
const CATEGORY_POOL: u32 = 4;
const BAND_POOL: usize = MONTH_AXIS.days() as usize;

struct RowRem(f64);

impl Decimal for RowRem {
    fn from_f64(value: f64) -> Self {
        Self(value)
    }

    fn to_f64(&self) -> f64 {
        self.0
    }
}

type Corner = [Option<bool>; 2];

enum DragKind {
    Move,
    Resize { corner: Corner, bx: BBox<2>, edge_offset: [f64; 2] },
}

struct Drag {
    n:       u32,
    key:     u32,
    cell:    usize,
    offset:  [f64; 2],
    pointer: [f64; 2],
    moved:   bool,
    kind:    DragKind,
}

#[derive(Clone, Copy)]
struct Placed {
    key:   u32,
    cell:  usize,
    base:  [f64; 2],
    bx:    BBox<2>,
    lane:  u32,
    lanes: u32,
    outer: [bool; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandKind {
    Closed,
    Break,
    Date(u64),
}

#[derive(Clone, Copy)]
pub struct Band {
    pub kind: BandKind,
    pub bx:   BBox<2>,
}

pub struct Hours {
    pub open:  u32,
    pub close: u32,
    pub rest:  Option<(u32, u32)>,
}

#[derive(Clone)]
enum Editing {
    Existing(u32),
    New(Vec<Place>),
}

struct CreateDrag {
    anchor:  [i32; 2],
    current: [i32; 2],
    moved:   bool,
}

struct Card {
    key:   u32,
    cell:  usize,
    bx:    BBox<2>,
    lane:  u32,
    lanes: u32,
    outer: [bool; 2],
}

pub struct Handler {
    view:              View,
    now:               u64,
    today:             u64,
    base:              u64,
    placed:            RefCell<Vec<Placed>>,
    bands:             RefCell<Vec<Band>>,
    editing:           Option<Editing>,
    pending_tap:       Option<[f64; 2]>,
    create:            Option<CreateDrag>,
    slot_rem:          f64,
    drag:              Option<Drag>,
    dirty:             bool,
    store:             Option<Backend>,
    lost:              Vec<(u32, Option<Vec<u8>>)>,
    startup:           Vec<Error>,
    viewport_width_px: f64,
    rem_in_px:         f64,
    scroll_x:          f64,
    rectgrid:          RectGrid<2>,
}

impl Handler {
    pub async fn ready(
        viewport_width_px: f64,
        _viewport_height_px: f64,
        rem_in_px: f64,
        now: f64,
        timezone_offset_minutes: i32,
    ) -> Self {
        let minutes = libm::floor(now / 60_000.0) as i64 + i64::from(timezone_offset_minutes);
        let today = add_days(pack(1970, 1, 1, 0, 0, 0, 0, 0, 0), minutes.div_euclid(1440));
        #[allow(unused_mut)]
        let mut handler = Self::new(viewport_width_px, today, rem_in_px);
        handler.now = from_ut(now, true, &Timezone::None);
        #[cfg(target_arch = "wasm32")]
        match Backend::open(STORE, true).await.and_then(Backend::new) {
            Ok(opened) => handler.attach(opened),
            Err(error) => handler.startup.push(Error::FileStore(error)),
        }
        handler
    }

    pub fn attach(&mut self, store: Backend) {
        if let Err(error) = data::check(&store, &appointment::KINDS) {
            self.startup.push(Error::Data(error));
        }
        self.store = Some(store);
    }

    pub(crate) fn read(&self, kind: u32) -> Vec<DataStruct> {
        self.store.as_ref().and_then(|store| data::entries(store, kind).ok()).unwrap_or_default()
    }

    fn appointment(&self, key: u32) -> Option<DataStruct> {
        data::entry(self.store.as_ref()?, key, appointment::KIND)
    }

    fn tags(&self, parent: u32) -> Vec<DataStruct> {
        self.store.as_ref().map(|store| data::tags(store, parent)).unwrap_or_default()
    }

    fn shifts(&self) -> Vec<DataStruct> {
        let resources = self.tags(appointment::RESOURCE);
        let mut shifts = self.read(shift::KIND);
        shifts.retain(|s| column_of(&resources, shift::Resource::read(s)).is_some());
        shifts
    }

    pub(crate) fn loaded(&self) -> bool {
        self.store.as_ref().is_some_and(data::loaded)
    }

    pub fn new(viewport_width_px: f64, today: u64, rem_in_px: f64) -> Self {
        let view = View::Week;
        let column_px = column_px(viewport_width_px, view, rem_in_px);
        Self {
            view,
            now: 0,
            today,
            base: today,
            placed: RefCell::new(Vec::new()),
            bands: RefCell::new(Vec::new()),
            editing: None,
            pending_tap: None,
            create: None,
            slot_rem: SLOT_REM,
            drag: None,
            dirty: false,
            store: None,
            lost: Vec::new(),
            startup: Vec::new(),
            viewport_width_px,
            rem_in_px,
            scroll_x: 0.0,
            rectgrid: RectGrid::new(
                [Px::new(0.0), Px::new(0.0)],
                [
                    IncrementFunction::Scale(column_px),
                    IncrementFunction::Scale(SLOT_REM * rem_in_px),
                ],
            )
            .unwrap(),
        }
    }

    pub fn close(&self) -> Vec<Command> {
        if let Some(store) = &self.store {
            store.close();
        }
        vec![]
    }

    pub fn view(&self) -> View {
        self.view
    }

    pub fn initial_draw(&mut self) -> (Vec<Event>, Vec<Command>) {
        let mut commands = self.view_commands();
        let failed = !self.startup.is_empty();
        commands.extend(self.startup.drain(..).map(|error| Command::Error { error }));
        if !failed {
            if self.loaded() {
                commands.extend(self.loaded_commands());
            } else {
                commands.push(Command::Fetch {
                    request: LOAD_REQUEST,
                    method:  Method::Get,
                    path:    String::from(LOAD_PATH),
                    body:    Vec::new(),
                });
            }
        }
        commands.push(hidden(Target::Body.to_dom(), false));
        (vec![], commands)
    }

    pub fn process_fetched(&mut self, response: &Response) -> (Vec<Event>, Vec<Command>) {
        if response.request != LOAD_REQUEST {
            return (vec![], vec![]);
        }
        if response.status != STATUS_OK {
            return (vec![], vec![data_error(DataError::Status(response.status))]);
        }
        let (events, mut commands) = match self.import_commands(&response.body) {
            Ok(result) => result,
            Err(error) => return (vec![], vec![data_error(error)]),
        };
        commands.extend(self.loaded_commands());
        (events, commands)
    }

    pub fn process_canvas(
        &mut self,
        event: &CanvasEvent,
        _pointer: &Pointer,
    ) -> (Vec<Event>, Vec<Command>) {
        let target = Target::from_dom(&event.id);
        match event.event_type {
            EventType::Click => {
                target.map_or((vec![], vec![]), |target| self.click_control(target))
            }
            EventType::Input if target == Some(Target::Zoom) => (vec![], self.zoom(&event.value)),
            EventType::Change => match target {
                Some(Target::ViewRadio(view)) => (vec![], self.change_view(view)),
                _ => (vec![], vec![]),
            },
            EventType::PointerDown => (vec![], self.press(event, target)),
            EventType::Close if target == Some(Target::Modal) => {
                self.editing = None;
                (vec![], vec![])
            }
            EventType::Submit if target == Some(Target::EditForm) => {
                (vec![], self.submit(&event.value))
            }
            EventType::Scroll => {
                if target == Some(Target::Surface) {
                    self.scroll_x = event.x;
                }
                (vec![], vec![])
            }
            _ => (vec![], vec![]),
        }
    }

    pub fn process_gesture(
        &mut self,
        gesture: &Gesture,
        _pointer: &Pointer,
        _origin: Option<&CanvasEvent>,
    ) -> (Vec<Event>, Vec<Command>) {
        match gesture {
            Gesture::Drag { x, y } => (vec![], self.drag(*x, *y)),
            Gesture::Tap => (vec![], self.tap()),
            Gesture::DragEnd => (vec![], self.drag_end()),
            Gesture::DragCancel => (vec![], self.drag_cancel()),
            _ => (vec![], vec![]),
        }
    }

    pub fn process_resize(&mut self, width_px: f64, _height_px: f64) -> (Vec<Event>, Vec<Command>) {
        self.viewport_width_px = width_px;
        self.fit_rectgrid();
        let mut commands = self.band_commands();
        commands.extend(self.card_commands());
        (vec![], commands)
    }

    pub fn process_scroll(&mut self, _x: f64, _y: f64) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn process_fullscreen(&mut self, _event: FullscreenEvent) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn process_visibility(&mut self, _state: VisibilityState) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    fn set_origin(&mut self, root_origin: (f64, f64)) {
        self.rectgrid.origin = [
            Px::new(root_origin.0 + AXIS_REM * self.rem_in_px - self.scroll_x),
            Px::new(root_origin.1 + HEAD_REM * self.rem_in_px),
        ];
    }

    fn press(&mut self, event: &CanvasEvent, target: Option<Target>) -> Vec<Command> {
        self.drag = None;
        self.pending_tap = None;
        self.create = None;
        let Some(Target::Card(n) | Target::CardPart(n, _)) = target else {
            if target == Some(Target::Surface) {
                self.set_origin(event.root_origin());
                self.pending_tap = Some([event.x, event.y]);
                self.create = self
                    .unit_at([event.x, event.y])
                    .filter(|_| !self.month())
                    .filter(|[column, row]| {
                        (0..self.columns() as i32).contains(column)
                            && (0..SLOT_COUNT as i32).contains(row)
                    })
                    .map(|anchor| CreateDrag { anchor, current: anchor, moved: false });
            }
            return vec![];
        };
        let Some(placed) = self.placed.borrow().get(n as usize - 1).copied() else {
            return vec![];
        };
        self.set_origin(event.root_origin());
        let pointer = [Px::new(event.x), Px::new(event.y)];
        let offset =
            self.rectgrid.offset(pointer, [Px::new(placed.base[0]), Px::new(placed.base[1])]);
        let corner = if self.month() { None } else { self.handle_at(&placed, pointer) };
        let kind = match corner.and_then(|corner| self.resize_kind(&placed, pointer, corner)) {
            Some(kind) => kind,
            None => DragKind::Move,
        };
        let mut commands = vec![];
        if let DragKind::Resize { corner, .. } = &kind {
            if let Some(cursor) = corner_cursor(*corner) {
                commands.push(style(
                    Target::Card(n).to_dom(),
                    StyleProperty::Cursor,
                    StyleValue::Keyword(cursor),
                ));
            }
        }
        self.drag = Some(Drag {
            n,
            key: placed.key,
            cell: placed.cell,
            offset: [offset[0].get(), offset[1].get()],
            pointer: [event.x, event.y],
            moved: false,
            kind,
        });
        commands
    }

    fn handle_at(&self, placed: &Placed, pointer: [Px; 2]) -> Option<Corner> {
        let resolved = self.rectgrid.box_as_px(&[placed.bx]);
        let [Ok((_, width)), Ok((_, height))] = resolved.first()? else {
            return None;
        };
        let smallest = width.get().min(height.get());
        if smallest <= 0.0 {
            return None;
        }
        let threshold = (HANDLE_REM * self.rem_in_px / smallest).min(HANDLE_MAX);
        let (_, corner) = corner_test(&self.rectgrid, pointer, &placed.bx, threshold, None);
        let mut corner = corner?;
        let allowed = [
            placed.outer[0] && placed.lane == 0,
            placed.outer[1] && placed.lane + 1 == placed.lanes,
        ];
        corner[0] = match corner[0] {
            Some(true) if allowed[0] => Some(true),
            Some(false) if allowed[1] => Some(false),
            _ => None,
        };
        corner.iter().any(Option::is_some).then_some(corner)
    }

    fn resize_kind(&self, placed: &Placed, pointer: [Px; 2], corner: Corner) -> Option<DragKind> {
        let bx = self.appointment_box(placed.key)?;
        let resolved = self.rectgrid.box_as_px(&[bx]);
        let [Ok((base_x, size_x)), Ok((base_y, size_y))] = resolved.first()? else {
            return None;
        };
        let (base, size) = ([base_x.get(), base_y.get()], [size_x.get(), size_y.get()]);
        let mut edge_offset = [0.0; 2];
        for d in 0..2 {
            let Some(base_side) = corner[d] else { continue };
            let edge = if base_side { base[d] } else { base[d] + size[d] };
            edge_offset[d] = pointer[d].get() - self.rectgrid.origin[d].get() - edge;
        }
        Some(DragKind::Resize { corner, bx, edge_offset })
    }

    fn appointment_box(&self, key: u32) -> Option<BBox<2>> {
        let entry = self.appointment(key)?;
        let resources = self.tags(appointment::RESOURCE);
        let grid = self.grid();
        let (start, end) = appointment::Range::read(&entry);
        let flats = appointment::Places::read(&entry).into_iter().filter_map(|place| {
            let resource = column_of(&resources, place.resource)?;
            Some(grid.columns.flat(self.day_offset(place.day), resource as u32))
        });
        let (first, last) = flats.fold(None, |range: Option<(i32, i32)>, flat| match range {
            Some((low, high)) => Some((low.min(flat), high.max(flat))),
            None => Some((flat, flat)),
        })?;
        grid.bbox_flat(first, last, start, end)
    }

    fn drag(&mut self, x: f64, y: f64) -> Vec<Command> {
        let Some(drag) = self.drag.as_mut() else {
            return self.create_drag(x, y);
        };
        drag.pointer = [x, y];
        let first = !drag.moved;
        drag.moved = true;
        let n = drag.n;
        let mut commands = vec![];
        if first {
            commands.push(style(
                Target::Card(n).to_dom(),
                StyleProperty::ZIndex,
                StyleValue::Integer(DRAG_Z_INDEX),
            ));
        }
        let Some(drag) = self.drag.as_ref() else {
            return commands;
        };
        match &drag.kind {
            DragKind::Move => {
                let px = drag_translate(
                    &self.rectgrid,
                    [Px::new(x), Px::new(y)],
                    [Px::new(drag.offset[0]), Px::new(drag.offset[1])],
                );
                commands.push(style(
                    Target::Card(n).to_dom(),
                    StyleProperty::Translate,
                    StyleValue::List(vec![
                        rem(px[0].get(), self.rem_in_px),
                        rem(px[1].get(), self.rem_in_px),
                    ]),
                ));
            }
            DragKind::Resize { .. } => {
                if let Some(resized) = self.resized_box(drag) {
                    commands.extend(self.preview_commands(n, &resized));
                }
            }
        }
        commands
    }

    fn preview_commands(&self, n: u32, bx: &BBox<2>) -> Vec<Command> {
        let resolved = self.rectgrid.box_as_px(&[*bx]);
        let Some([Ok((x, width)), Ok((y, height))]) = resolved.first() else {
            return vec![];
        };
        vec![
            style(
                Target::Card(n).to_dom(),
                StyleProperty::Translate,
                StyleValue::List(vec![rem(x.get(), self.rem_in_px), rem(y.get(), self.rem_in_px)]),
            ),
            style(Target::Card(n).to_dom(), StyleProperty::Width, rem(width.get(), self.rem_in_px)),
            style(
                Target::Card(n).to_dom(),
                StyleProperty::Height,
                rem(height.get(), self.rem_in_px),
            ),
        ]
    }

    fn resized_box(&self, drag: &Drag) -> Option<BBox<2>> {
        let DragKind::Resize { corner, bx, edge_offset } = &drag.kind else {
            return None;
        };
        let unit_px = [
            column_px(self.viewport_width_px, self.view, self.rem_in_px),
            self.slot_rem * self.rem_in_px,
        ];
        let extent = [self.columns() as f64 * unit_px[0], SLOT_COUNT as f64 * unit_px[1]];
        let mut pointer = [Px::new(0.0); 2];
        for d in 0..2 {
            let origin = self.rectgrid.origin[d].get();
            let local = drag.pointer[d] - origin - edge_offset[d] + unit_px[d] / 2.0;
            pointer[d] = Px::new(origin + local.max(0.0).min(extent[d]));
        }
        drag_resize(&self.rectgrid, pointer, bx, *corner).ok()
    }

    fn drag_end(&mut self) -> Vec<Command> {
        let Some(drag) = self.drag.take() else {
            return self.finish_create();
        };
        let mut commands = self.release_commands(&drag);
        let changed = match &drag.kind {
            DragKind::Move if self.month() => self.move_in_month(&drag),
            DragKind::Move => self
                .drop_target(&drag)
                .and_then(|(cell, start)| self.move_appointment(drag.key, drag.cell, cell, start)),
            DragKind::Resize { corner, .. } => self
                .resized_box(&drag)
                .and_then(|resized| self.resize_appointment(drag.key, *corner, &resized)),
        };
        if let Some(appointment) = changed {
            self.persist(appointment);
            commands.extend(self.mark_dirty());
        }
        commands.extend(self.card_commands());
        commands
    }

    fn tap(&mut self) -> Vec<Command> {
        let Some(drag) = self.drag.take() else {
            return self.create_commands();
        };
        if drag.moved || !matches!(drag.kind, DragKind::Move) {
            return vec![];
        }
        let mut commands = self.release_commands(&drag);
        commands.extend(self.edit_commands(drag.key, drag.cell));
        commands
    }

    fn form_commands(
        &mut self,
        editing: Editing,
        heading: String,
        draft: &DataStruct,
        place: Place,
        hint: &str,
    ) -> Vec<Command> {
        if !self.loaded() {
            return vec![];
        }
        let (start, end) = appointment::Range::read(draft);
        let (category, status) =
            (appointment::Category::read(draft), appointment::Status::read(draft));
        let values = [
            (EditField::Title, appointment::Title::read(draft)),
            (
                EditField::Category,
                option_value(&self.tags(appointment::CATEGORY), |c| tag::Code::read(c) == category),
            ),
            (
                EditField::Status,
                option_value(&self.tags(appointment::STATUS), |s| tag::Code::read(s) == status),
            ),
            (EditField::Date, display(place.day, LANG, Format::Date)),
            (
                EditField::Resource,
                option_value(&self.tags(appointment::RESOURCE), |r| {
                    tag::Uid::read(r) == place.resource
                }),
            ),
            (EditField::Start, format_hhmm(start)),
            (EditField::End, format_hhmm(end)),
            (EditField::Note, appointment::Note::read(draft)),
        ];
        let mut commands = vec![
            Command::SetText { id: Target::EditHeading.to_dom(), value: heading },
            Command::SetText { id: Target::EditMessage.to_dom(), value: String::from(hint) },
        ];
        commands.extend(values.into_iter().map(|(field, value)| Command::SetValue {
            id: Target::EditField(field).to_dom(),
            value,
        }));
        commands.push(hidden(Target::Delete.to_dom(), matches!(editing, Editing::New(_))));
        commands.push(Command::ShowModal { id: Target::Modal.to_dom() });
        self.editing = Some(editing);
        commands
    }

    fn zoom(&mut self, value: &str) -> Vec<Command> {
        let Some(RowRem(slot_rem)) = parse(value.as_bytes()) else {
            return vec![];
        };
        let slot_rem = slot_rem.clamp(ZOOM_MIN, ZOOM_MAX);
        if slot_rem == self.slot_rem {
            return vec![];
        }
        self.slot_rem = slot_rem;
        self.fit_rectgrid();
        let mut commands = self.row_commands();
        commands.extend(self.band_commands());
        commands.extend(self.card_commands());
        commands
    }

    fn unit_at(&self, pointer: [f64; 2]) -> Option<[i32; 2]> {
        let [Ok(column), Ok(row)] =
            self.rectgrid.point_to_unit([Px::new(pointer[0]), Px::new(pointer[1])])
        else {
            return None;
        };
        Some([libm::floor(column.get()) as i32, libm::floor(row.get()) as i32])
    }

    fn create_box(create: &CreateDrag) -> BBox<2> {
        let low =
            [create.anchor[0].min(create.current[0]), create.anchor[1].min(create.current[1])];
        let high =
            [create.anchor[0].max(create.current[0]), create.anchor[1].max(create.current[1])];
        BBox::new(
            [GridUnit::new(low[0] as f64), GridUnit::new(low[1] as f64)],
            [
                GridUnit::new((high[0] - low[0] + 1) as f64),
                GridUnit::new((high[1] - low[1] + 1) as f64),
            ],
        )
    }

    fn create_drag(&mut self, x: f64, y: f64) -> Vec<Command> {
        let Some([column, row]) = self.unit_at([x, y]) else {
            return vec![];
        };
        let current =
            [column.clamp(0, self.columns() as i32 - 1), row.clamp(0, SLOT_COUNT as i32 - 1)];
        let Some(create) = self.create.as_mut() else {
            return vec![];
        };
        create.current = current;
        create.moved = true;
        let bx = Self::create_box(create);
        let [Ok((px, width)), Ok((py, height))] = self.rectgrid.box_as_px(&[bx])[0] else {
            return vec![];
        };
        vec![
            hidden(Target::Preview.to_dom(), false),
            style(
                Target::Preview.to_dom(),
                StyleProperty::Translate,
                StyleValue::List(vec![
                    rem(px.get(), self.rem_in_px),
                    rem(py.get(), self.rem_in_px),
                ]),
            ),
            style(Target::Preview.to_dom(), StyleProperty::Width, rem(width.get(), self.rem_in_px)),
            style(
                Target::Preview.to_dom(),
                StyleProperty::Height,
                rem(height.get(), self.rem_in_px),
            ),
        ]
    }

    fn finish_create(&mut self) -> Vec<Command> {
        let Some(create) = self.create.take() else {
            return vec![];
        };
        let mut commands = vec![hidden(Target::Preview.to_dom(), true)];
        if !create.moved || create.anchor == create.current {
            commands.extend(self.create_commands());
            return commands;
        }
        let Some(pointer) = self.pending_tap.take() else {
            return commands;
        };
        let Some(resolved) = self.grid().resolve(&Self::create_box(&create)) else {
            return commands;
        };
        let resources = self.tags(appointment::RESOURCE);
        let cells: Option<Vec<Place>> = resolved
            .cells
            .iter()
            .map(|cell| {
                let resource = resources.get(cell.resource as usize)?;
                Some(Place {
                    day:      add_days(self.base, i64::from(cell.day)),
                    resource: tag::Uid::read(resource),
                })
            })
            .collect();
        if let Some(cells) = cells {
            commands.extend(self.new_form(cells, resolved.start, resolved.end, pointer));
        }
        commands
    }

    fn new_form(
        &mut self,
        cells: Vec<Place>,
        start: u32,
        end: u32,
        pointer: [f64; 2],
    ) -> Vec<Command> {
        let Some(first) = cells.first() else {
            return vec![];
        };
        let hint = match self.band_hit([Px::new(pointer[0]), Px::new(pointer[1])]) {
            None | Some(BandKind::Date(_)) => "",
            Some(BandKind::Break) => "休憩中",
            Some(BandKind::Closed) => "営業時間外",
        };
        let place = *first;
        let (categories, statuses) =
            (self.tags(appointment::CATEGORY), self.tags(appointment::STATUS));
        let category = categories.first().map(tag::Code::read).unwrap_or_default();
        let status = statuses.first().map(tag::Code::read).unwrap_or_default();
        let draft = appointment::new(0, &[place], start, end, "", &category, &status, "");
        self.form_commands(Editing::New(cells), String::from("新規"), &draft, place, hint)
    }

    fn create_commands(&mut self) -> Vec<Command> {
        let Some(pointer) = self.pending_tap.take() else {
            return vec![];
        };
        if !self.loaded() {
            return vec![];
        }
        let resources = self.tags(appointment::RESOURCE);
        if self.month() {
            if let Some(BandKind::Date(day)) =
                self.band_hit([Px::new(pointer[0]), Px::new(pointer[1])])
            {
                self.base = day;
                return self.change_view(View::Day);
            }
            let Some((day, _)) = self.unit_at(pointer).and_then(|unit| MONTH_AXIS.locate(unit))
            else {
                return vec![];
            };
            let Some(resource) = resources.first() else {
                return vec![];
            };
            let open = hours(&self.shifts()).map_or(DAY_START_MINUTES, |hours| hours.open);
            let start = TIME_AXIS.clamp_start(open, NEW_MINUTES);
            let place = Place {
                day:      add_days(self.first_day(), i64::from(day)),
                resource: tag::Uid::read(resource),
            };
            return self.new_form(vec![place], start, start + NEW_MINUTES, pointer);
        }
        let [Ok(column), Ok(row)] =
            self.rectgrid.point_to_unit([Px::new(pointer[0]), Px::new(pointer[1])])
        else {
            return vec![];
        };
        let grid = self.grid();
        if column.get() < 0.0 || !grid.time.contains(row.get()) {
            return vec![];
        }
        let Some(cell) = grid.columns.cell(libm::floor(column.get()) as u32) else {
            return vec![];
        };
        let Some(minutes) = grid.time.minutes(libm::floor(row.get()) as u32) else {
            return vec![];
        };
        let Some(resource) = resources.get(cell.resource as usize) else {
            return vec![];
        };
        let start = grid.time.clamp_start(minutes, NEW_MINUTES);
        let place = Place {
            day:      add_days(self.base, i64::from(cell.day)),
            resource: tag::Uid::read(resource),
        };
        self.new_form(vec![place], start, start + NEW_MINUTES, pointer)
    }

    fn edit_commands(&mut self, key: u32, cell: usize) -> Vec<Command> {
        let Some(entry) = self.appointment(key) else {
            return vec![];
        };
        let cells = appointment::Places::read(&entry);
        let Some(place) = cells.get(cell).or_else(|| cells.first()) else {
            return vec![];
        };
        let place = *place;
        let heading = format!("#{}", data::key(&entry));
        self.form_commands(Editing::Existing(key), heading, &entry, place, "")
    }

    fn delete_commands(&mut self) -> Vec<Command> {
        let Some(Editing::Existing(key)) = self.editing.clone() else {
            return vec![];
        };
        if self.appointment(key).is_none() {
            return vec![];
        }
        let Some(store) = self.store.as_mut() else {
            return vec![];
        };
        store.delete(key);
        self.editing = None;
        let mut commands = Vec::new();
        commands.extend(self.mark_dirty());
        commands.push(Command::CloseModal { id: Target::Modal.to_dom() });
        commands.extend(self.card_commands());
        commands
    }

    fn reject(message: &str) -> Vec<Command> {
        vec![Command::SetText { id: Target::EditMessage.to_dom(), value: String::from(message) }]
    }

    fn submit(&mut self, value: &str) -> Vec<Command> {
        let Some(editing) = self.editing.clone() else {
            return vec![];
        };
        if !self.loaded() {
            return Self::reject("データがありません");
        }
        let draft = match decode_form(
            value,
            &self.tags(appointment::RESOURCE),
            &self.tags(appointment::STATUS),
            &self.tags(appointment::CATEGORY),
        ) {
            Ok(draft) => draft,
            Err(message) => return Self::reject(message),
        };
        match self.apply_form(editing, draft) {
            Ok(entry) => {
                self.editing = None;
                self.persist(entry);
                let mut commands = Vec::new();
                commands.extend(self.mark_dirty());
                commands.push(Command::CloseModal { id: Target::Modal.to_dom() });
                commands.extend(self.loaded_commands());
                commands
            }
            Err(message) => Self::reject(message),
        }
    }

    fn apply_form(
        &mut self,
        editing: Editing,
        draft: DataStruct,
    ) -> Result<DataStruct, &'static str> {
        let (resources, shifts) = (self.tags(appointment::RESOURCE), self.shifts());
        let places = appointment::Places::read(&draft);
        let place = *places.first().ok_or("入力を読み取れません")?;
        let resource = column_of(&resources, place.resource).ok_or("資源が不正です")?;
        let range = appointment::Range::read(&draft);
        match editing {
            Editing::New(pending) => {
                if !fits(&shifts, range) {
                    return Err("営業時間外です");
                }
                let cells = shift_cells(&resources, &pending, place.day, resource)?;
                let store = self.store.as_mut().ok_or("データがありません")?;
                let mut entry = appointment::new(
                    store.issue_id(),
                    &cells,
                    range.0,
                    range.1,
                    &appointment::Title::read(&draft),
                    &appointment::Category::read(&draft),
                    &appointment::Status::read(&draft),
                    &appointment::Note::read(&draft),
                );
                let _ = appointment::Uid::write(&mut entry, &data::new_uid(self.now), None);
                data::stamp(&mut entry, self.now);
                Ok(entry)
            }
            Editing::Existing(key) => {
                let mut entry = self.appointment(key).ok_or("予約がありません")?;
                let cells = shift_cells(
                    &resources,
                    &appointment::Places::read(&entry),
                    place.day,
                    resource,
                )?;
                let changed = appointment::Range::read(&entry) != range;
                if changed && !fits(&shifts, range) {
                    return Err("営業時間外です");
                }
                let _ = appointment::Places::write(&mut entry, &cells, None);
                let _ = appointment::Range::write(&mut entry, &range, None);
                let _ =
                    appointment::Title::write(&mut entry, &appointment::Title::read(&draft), None);
                let _ = appointment::Category::write(
                    &mut entry,
                    &appointment::Category::read(&draft),
                    None,
                );
                let _ = appointment::Status::write(
                    &mut entry,
                    &appointment::Status::read(&draft),
                    None,
                );
                let _ =
                    appointment::Note::write(&mut entry, &appointment::Note::read(&draft), None);
                Ok(entry)
            }
        }
    }

    fn drag_cancel(&mut self) -> Vec<Command> {
        let Some(drag) = self.drag.take() else {
            self.create = None;
            self.pending_tap = None;
            return vec![hidden(Target::Preview.to_dom(), true)];
        };
        let mut commands = self.release_commands(&drag);
        commands.extend(self.card_commands());
        commands
    }

    fn release_commands(&self, drag: &Drag) -> Vec<Command> {
        let mut commands = vec![Command::RemoveStyle {
            id:       Target::Card(drag.n).to_dom(),
            property: StyleProperty::ZIndex,
        }];
        if matches!(drag.kind, DragKind::Resize { .. }) {
            commands.push(Command::RemoveStyle {
                id:       Target::Card(drag.n).to_dom(),
                property: StyleProperty::Cursor,
            });
        }
        commands
    }

    fn resize_appointment(&self, key: u32, corner: Corner, bx: &BBox<2>) -> Option<DataStruct> {
        let base = self.base;
        let grid = self.grid();
        let resources = self.tags(appointment::RESOURCE);
        let cells = (corner[0].is_some()).then(|| {
            let (first, last) = span(bx.base()[0].get(), bx.offset()[0].get());
            (first..last)
                .map(|flat| {
                    let (day, resource) = grid.columns.locate(flat);
                    let resource = tag::Uid::read(resources.get(resource as usize)?);
                    Some(Place { day: add_days(base, i64::from(day)), resource })
                })
                .collect::<Option<Vec<Place>>>()
        });
        let time = (corner[1].is_some()).then(|| {
            let (first, last) = span(bx.base()[1].get(), bx.offset()[1].get());
            Some((grid.time.minutes(first.max(0) as u32)?, grid.time.minutes(last.max(0) as u32)?))
        });
        let mut entry = self.appointment(key)?;
        let current_cells = appointment::Places::read(&entry);
        let current_range = appointment::Range::read(&entry);
        let new_cells = match cells {
            Some(Some(cells)) => cells,
            _ => current_cells.clone(),
        };
        let new_range = match time {
            Some(Some((start, end))) if end > start => (start, end),
            _ => current_range,
        };
        let changed = new_cells != current_cells || new_range != current_range;
        if !changed || !fits(&self.shifts(), new_range) {
            return None;
        }
        let _ = appointment::Places::write(&mut entry, &new_cells, None);
        let _ = appointment::Range::write(&mut entry, &new_range, None);
        Some(entry)
    }

    fn drop_target(&self, drag: &Drag) -> Option<(Cell, u32)> {
        let entry = self.appointment(drag.key)?;
        let grid = self.grid();
        let pointer = [Px::new(drag.pointer[0]), Px::new(drag.pointer[1])];
        let [Ok(column), Ok(row)] = self.rectgrid.point_to_unit(pointer) else {
            return None;
        };
        if column.get() < 0.0 || !grid.time.contains(row.get()) {
            return None;
        }
        let top = [pointer[0], pointer[1] - Px::new(drag.offset[1])];
        let [_, Ok(top)] = self.rectgrid.point_to_unit(top) else {
            return None;
        };
        let slot = (libm::floor(top.get() + 0.5).max(0.0) as u32).min(SLOT_COUNT - 1);
        let (start, end) = appointment::Range::read(&entry);
        let duration = end - start;
        let logical = BBox::new(
            [GridUnit::new(libm::floor(column.get())), GridUnit::new(slot as f64)],
            [GridUnit::new(1.0), GridUnit::new(duration as f64 / SLOT_MINUTES as f64)],
        );
        let resolved = grid.resolve(&logical)?;
        let cell = *resolved.cells.first()?;
        Some((cell, grid.time.clamp_start(resolved.start, duration)))
    }

    fn move_in_month(&self, drag: &Drag) -> Option<DataStruct> {
        let first = self.first_day();
        let (target, _) = self.unit_at(drag.pointer).and_then(|unit| MONTH_AXIS.locate(unit))?;
        let mut entry = self.appointment(drag.key)?;
        let cells = appointment::Places::read(&entry);
        let pressed = cells.get(drag.cell)?;
        let delta = i64::from(target) - diff(first, pressed.day) / DAY;
        if delta == 0 {
            return None;
        }
        let moved: Vec<Place> = cells
            .iter()
            .map(|place| Place { day: add_days(place.day, delta), resource: place.resource })
            .collect();
        let _ = appointment::Places::write(&mut entry, &moved, None);
        Some(entry)
    }

    fn move_appointment(
        &self,
        key: u32,
        pressed: usize,
        target: Cell,
        start: u32,
    ) -> Option<DataStruct> {
        let base = self.base;
        let axis = self.grid().columns;
        let resources = self.tags(appointment::RESOURCE);
        let mut entry = self.appointment(key)?;
        let flat = |place: &Place| {
            column_of(&resources, place.resource)
                .map(|position| axis.flat((diff(base, place.day) / DAY) as i32, position as u32))
        };
        let cells = appointment::Places::read(&entry);
        let from = cells.get(pressed).and_then(flat)?;
        let delta = axis.flat(target.day as i32, target.resource) - from;
        let moved: Vec<Place> = cells
            .iter()
            .map(|place| {
                let (day, resource) = axis.locate(flat(place)? + delta);
                let resource = tag::Uid::read(resources.get(resource as usize)?);
                Some(Place { day: add_days(base, i64::from(day)), resource })
            })
            .collect::<Option<_>>()?;
        let current = appointment::Range::read(&entry);
        let range = (start, start + current.1 - current.0);
        let changed = cells != moved || current.0 != start;
        if !changed || !fits(&self.shifts(), range) {
            return None;
        }
        let _ = appointment::Places::write(&mut entry, &moved, None);
        let _ = appointment::Range::write(&mut entry, &range, None);
        Some(entry)
    }

    pub fn cell_at(&mut self, root_origin: (f64, f64), x: f64, y: f64) -> Option<(u32, u32, u32)> {
        self.set_origin(root_origin);
        let grid = self.grid();
        let [Ok(column), Ok(slot)] = self.rectgrid.point_to_unit([Px::new(x), Px::new(y)]) else {
            return None;
        };
        let (column, slot) = (libm::floor(column.get()), libm::floor(slot.get()));
        if column < 0.0 || !grid.time.contains(slot) {
            return None;
        }
        let cell = grid.columns.cell(column as u32)?;
        Some((cell.day, cell.resource, slot as u32))
    }

    pub fn base(&self) -> u64 {
        self.base
    }

    fn click_control(&mut self, target: Target) -> (Vec<Event>, Vec<Command>) {
        match target {
            Target::Step(n) if (1..=STEP_COUNT).contains(&n) => {
                self.base = if n == STEP_TODAY {
                    self.today
                } else {
                    add_days(self.base, self.step_days(n))
                };
                (vec![], self.date_commands())
            }
            Target::Modal => {
                self.editing = None;
                (vec![], vec![Command::CloseModal { id: Target::Modal.to_dom() }])
            }
            Target::Delete => (vec![], self.delete_commands()),
            Target::Save => self.save_commands(),
            Target::Reload => self.discard_commands(),
            _ => (vec![], vec![]),
        }
    }

    fn step_days(&self, n: u32) -> i64 {
        let (unit, page) = if self.month() {
            (7, i64::from(MONTH_AXIS.days()))
        } else {
            (1, i64::from(self.view.days()))
        };
        [-page, -unit, 0, unit, page][n as usize - 1]
    }

    fn resource_text(&self, n: u32) -> Command {
        let index = heading_axis().cell(n - 1).map_or(0, |cell| cell.resource as usize);
        let value = match self.tags(appointment::RESOURCE).get(index) {
            Some(resource) => tag::Label::read(resource),
            None => format!("R{}", index + 1),
        };
        Command::SetText { id: Target::ResourceName(n).to_dom(), value }
    }

    fn loaded_commands(&self) -> Vec<Command> {
        let mut commands: Vec<Command> =
            (1..=heading_axis().count()).map(|n| self.resource_text(n)).collect();
        commands.extend(self.option_commands());
        commands.extend(self.band_commands());
        commands.extend(self.card_commands());
        commands
    }

    fn option_commands(&self) -> Vec<Command> {
        if !self.loaded() {
            return vec![];
        }
        let (statuses, categories, resources) = (
            self.tags(appointment::STATUS),
            self.tags(appointment::CATEGORY),
            self.tags(appointment::RESOURCE),
        );
        let mut commands = Vec::new();
        let options = (1..=STATUS_POOL)
            .map(|n| (Target::StatusOption(n), statuses.get(n as usize - 1).map(tag::Label::read)))
            .chain((1..=CATEGORY_POOL).map(|n| {
                (Target::CategoryOption(n), categories.get(n as usize - 1).map(tag::Label::read))
            }))
            .chain((1..=RESOURCE_COUNT).map(|n| {
                (Target::ResourceOption(n), resources.get(n as usize - 1).map(tag::Label::read))
            }));
        for (target, text) in options {
            let shown = text.is_some();
            commands
                .push(Command::SetText { id: target.to_dom(), value: text.unwrap_or_default() });
            commands.push(hidden(target.to_dom(), !shown));
        }
        commands
    }

    fn visible_cards(&self, appointments: &[DataStruct], resources: &[DataStruct]) -> Vec<Card> {
        let grid = self.grid();
        let mut columns: BTreeMap<u32, Vec<(u32, usize, u32, u32)>> = BTreeMap::new();
        let mut extent: BTreeMap<u32, (i32, i32)> = BTreeMap::new();
        for entry in appointments {
            let key = data::key(entry);
            let (start, end) = appointment::Range::read(entry);
            let Some((start, end)) = grid.time.clip(start, end) else {
                continue;
            };
            for (cell_index, place) in appointment::Places::read(entry).iter().enumerate() {
                let Some(resource) = column_of(resources, place.resource) else {
                    continue;
                };
                let offset = self.day_offset(place.day);
                let flat = grid.columns.flat(offset, resource as u32);
                let range = extent.entry(key).or_insert((flat, flat));
                *range = (range.0.min(flat), range.1.max(flat));
                let Ok(day) = u32::try_from(offset) else {
                    continue;
                };
                let Some(unit) = grid.columns.unit(Cell { day, resource: resource as u32 }) else {
                    continue;
                };
                columns.entry(unit).or_default().push((key, cell_index, start, end));
            }
        }
        let mut cards = Vec::new();
        for (unit, items) in columns {
            let Some(cell) = grid.columns.cell(unit) else {
                continue;
            };
            let spans: Vec<(u32, u32)> =
                items.iter().map(|(_, _, start, end)| (*start, *end)).collect();
            for ((key, cell_index, start, end), (lane, lanes)) in
                items.into_iter().zip(lanes(&spans))
            {
                let Some(logical) = grid.bbox(cell, start, end) else {
                    continue;
                };
                let (first, last) = extent[&key];
                cards.push(Card {
                    key,
                    cell: cell_index,
                    bx: lane_box(&logical, lane, lanes),
                    lane,
                    lanes,
                    outer: [unit as i32 == first, unit as i32 == last],
                });
            }
        }
        cards.truncate(CARD_POOL);
        cards
    }

    fn month_cards(&self, appointments: &[DataStruct]) -> Vec<Card> {
        let first = self.first_day();
        let mut days: BTreeMap<u32, Vec<(u32, usize)>> = BTreeMap::new();
        for entry in appointments {
            let key = data::key(entry);
            for (cell, place) in appointment::Places::read(entry).iter().enumerate() {
                let Ok(day) = u32::try_from(diff(first, place.day) / DAY) else {
                    continue;
                };
                if day >= MONTH_AXIS.days() {
                    continue;
                }
                let items = days.entry(day).or_default();
                if !items.iter().any(|(other, _)| *other == key) {
                    items.push((key, cell));
                }
            }
        }
        let mut cards = Vec::new();
        for (day, mut items) in days {
            items.sort_unstable_by_key(|(key, _)| *key);
            for (row, (key, cell)) in (0..MONTH_AXIS.rows()).zip(items) {
                let Some(bx) = MONTH_AXIS.bbox(day, Some(row)) else {
                    continue;
                };
                cards.push(Card { key, cell, bx, lane: 0, lanes: 1, outer: [false; 2] });
            }
        }
        cards.truncate(CARD_POOL);
        cards
    }

    fn band_list(&self, shifts: &[DataStruct]) -> Vec<Band> {
        if self.month() {
            let first = self.first_day();
            return (0..MONTH_AXIS.days())
                .filter_map(|day| {
                    let bx = MONTH_AXIS.bbox(day, None)?;
                    Some(Band { kind: BandKind::Date(add_days(first, i64::from(day))), bx })
                })
                .collect();
        }
        let Some(hours) = hours(shifts) else {
            return vec![];
        };
        let grid = self.grid();
        let columns = grid.columns.count() as f64;
        let wide = |from: f64, to: f64, kind| {
            (to > from).then(|| Band {
                kind,
                bx: BBox::new(
                    [GridUnit::new(0.0), GridUnit::new(from)],
                    [GridUnit::new(columns), GridUnit::new(to - from)],
                ),
            })
        };
        let end = SLOT_COUNT as f64;
        let (open, close) = (
            grid.time.unit(hours.open).clamp(0.0, end),
            grid.time.unit(hours.close).clamp(0.0, end),
        );
        let mut bands: Vec<Band> =
            [wide(0.0, open, BandKind::Closed), wide(close, end, BandKind::Closed)]
                .into_iter()
                .flatten()
                .collect();
        if let Some((start, stop)) = hours.rest {
            bands.extend(wide(grid.time.unit(start), grid.time.unit(stop), BandKind::Break));
        }
        bands.truncate(BAND_POOL);
        bands
    }

    fn person_commands(&self) -> Vec<Command> {
        let days = if self.month() { 0 } else { self.view.days() };
        let (resources, shifts) = (self.tags(appointment::RESOURCE), self.shifts());
        let mut commands = Vec::new();
        for n in 1..=heading_axis().count() {
            let person =
                heading_axis().cell(n - 1).filter(|cell| cell.day < days).and_then(|cell| {
                    let uid = tag::Uid::read(resources.get(cell.resource as usize)?);
                    let day = add_days(self.base, i64::from(cell.day));
                    shifts
                        .iter()
                        .find(|s| shift::Day::read(s) == day && shift::Resource::read(s) == uid)
                        .map(shift::Person::read)
                });
            commands.push(Command::SetText {
                id:    Target::ResourcePerson(n).to_dom(),
                value: person.unwrap_or_default(),
            });
        }
        commands
    }

    fn band_commands(&self) -> Vec<Command> {
        if !self.loaded() {
            return vec![];
        }
        let bands = self.band_list(&self.shifts());
        let boxes: Vec<BBox<2>> = bands.iter().map(|band| band.bx).collect();
        let resolved = self.rectgrid.box_as_px(&boxes);
        let mut commands = self.person_commands();
        let mut placed = Vec::new();
        for ((band, px), n) in bands.iter().zip(resolved).zip(1u32..) {
            let [Ok((x, width)), Ok((y, height))] = px else {
                continue;
            };
            placed.push(*band);
            commands.push(hidden(Target::Band(n).to_dom(), false));
            commands.push(style(
                Target::Band(n).to_dom(),
                StyleProperty::Translate,
                StyleValue::List(vec![rem(x.get(), self.rem_in_px), rem(y.get(), self.rem_in_px)]),
            ));
            commands.push(style(
                Target::Band(n).to_dom(),
                StyleProperty::Width,
                rem(width.get(), self.rem_in_px),
            ));
            commands.push(style(
                Target::Band(n).to_dom(),
                StyleProperty::Height,
                rem(height.get(), self.rem_in_px),
            ));
            let (label, current) = match band.kind {
                BandKind::Date(day) => {
                    let (_, month, date, ..) = unpack(day);
                    let label =
                        if date == 1 { format!("{month}/{date}") } else { format!("{date}") };
                    (label, day == self.today)
                }
                _ => (String::new(), false),
            };
            commands.push(Command::SetText { id: Target::BandLabel(n).to_dom(), value: label });
            commands.push(if current {
                Command::SetAttribute {
                    id:        Target::Band(n).to_dom(),
                    attribute: Attribute::AriaCurrent,
                    value:     String::from("date"),
                }
            } else {
                Command::RemoveAttribute {
                    id:        Target::Band(n).to_dom(),
                    attribute: Attribute::AriaCurrent,
                }
            });
        }
        let shown = self.bands.borrow().len() as u32;
        for n in placed.len() as u32 + 1..=shown {
            commands.push(hidden(Target::Band(n).to_dom(), true));
        }
        *self.bands.borrow_mut() = placed;
        commands
    }

    pub fn band_hit(&self, point: [Px; 2]) -> Option<BandKind> {
        let bands = self.bands.borrow();
        let boxes: Vec<BBox<2>> = bands.iter().map(|band| band.bx).collect();
        let index = self.rectgrid.hit_test(point, &boxes, None)?;
        bands.get(index).map(|band| band.kind)
    }

    fn card_commands(&self) -> Vec<Command> {
        if !self.loaded() {
            return vec![];
        }
        let appointments = self.read(appointment::KIND);
        let (statuses, categories) =
            (self.tags(appointment::STATUS), self.tags(appointment::CATEGORY));
        let cards = if self.month() {
            self.month_cards(&appointments)
        } else {
            self.visible_cards(&appointments, &self.tags(appointment::RESOURCE))
        };
        let by_key: BTreeMap<u32, &DataStruct> =
            appointments.iter().map(|entry| (data::key(entry), entry)).collect();
        let boxes: Vec<BBox<2>> = cards.iter().map(|card| card.bx).collect();
        let resolved = self.rectgrid.box_as_px(&boxes);
        let mut commands = Vec::new();
        let mut placed: Vec<Placed> = Vec::new();
        for ((card, px), n) in cards.iter().zip(resolved).zip(1u32..) {
            let [Ok((x, width)), Ok((y, height))] = px else {
                continue;
            };
            placed.push(Placed {
                key:   card.key,
                cell:  card.cell,
                base:  [x.get(), y.get()],
                bx:    card.bx,
                lane:  card.lane,
                lanes: card.lanes,
                outer: card.outer,
            });
            let entry = by_key[&card.key];
            let (start, end) = appointment::Range::read(entry);
            let status_code = appointment::Status::read(entry);
            let status = statuses
                .iter()
                .find(|s| tag::Code::read(s) == status_code)
                .map(tag::Label::read)
                .unwrap_or_default();
            let category_code = appointment::Category::read(entry);
            let category = categories
                .iter()
                .find(|c| tag::Code::read(c) == category_code)
                .map(tag::Label::read)
                .unwrap_or_default();
            commands.push(Command::RemoveAttribute {
                id:        Target::Card(n).to_dom(),
                attribute: Attribute::Hidden,
            });
            commands.push(style(
                Target::Card(n).to_dom(),
                StyleProperty::Translate,
                StyleValue::List(vec![rem(x.get(), self.rem_in_px), rem(y.get(), self.rem_in_px)]),
            ));
            commands.push(style(
                Target::Card(n).to_dom(),
                StyleProperty::Width,
                rem(width.get(), self.rem_in_px),
            ));
            commands.push(style(
                Target::Card(n).to_dom(),
                StyleProperty::Height,
                rem(height.get(), self.rem_in_px),
            ));
            for (id, value) in [
                (Target::CardPart(n, CardPart::Status).to_dom(), status),
                (
                    Target::CardPart(n, CardPart::Time).to_dom(),
                    format!("{:02}:{:02}–{:02}:{:02}", start / 60, start % 60, end / 60, end % 60),
                ),
                (Target::CardPart(n, CardPart::Title).to_dom(), appointment::Title::read(entry)),
                (Target::CardPart(n, CardPart::Category).to_dom(), category),
                (Target::CardPart(n, CardPart::Note).to_dom(), appointment::Note::read(entry)),
            ] {
                commands.push(Command::SetText { id, value });
            }
        }
        let shown = self.placed.borrow().len() as u32;
        for n in placed.len() as u32 + 1..=shown {
            commands.push(hidden(Target::Card(n).to_dom(), true));
        }
        *self.placed.borrow_mut() = placed;
        commands
    }

    fn mark_dirty(&mut self) -> Option<Command> {
        if self.dirty {
            return None;
        }
        self.dirty = true;
        Some(Command::RemoveAttribute {
            id:        Target::Save.to_dom(),
            attribute: Attribute::Disabled,
        })
    }

    fn import_commands(&mut self, body: &[u8]) -> Result<(Vec<Event>, Vec<Command>), DataError> {
        let Some(store) = self.store.as_mut() else {
            return Ok((vec![], vec![]));
        };
        appointment::import(store, body)?;
        Ok(match store.save() {
            Ok(()) => (vec![], vec![]),
            Err(error) => file_store_error(error),
        })
    }

    fn persist(&mut self, mut entry: DataStruct) {
        if let Some(store) = self.store.as_mut() {
            data::touch(&mut entry, self.now);
            data::put(store, &entry);
        }
    }

    fn save_commands(&mut self) -> (Vec<Event>, Vec<Command>) {
        let Some(store) = self.store.as_mut() else {
            return (vec![], vec![]);
        };
        if let Err(error) = store.save() {
            return file_store_error(error);
        }
        self.dirty = false;
        (
            vec![],
            vec![Command::SetAttribute {
                id:        Target::Save.to_dom(),
                attribute: Attribute::Disabled,
                value:     String::new(),
            }],
        )
    }

    pub fn discard_commands(&mut self) -> (Vec<Event>, Vec<Command>) {
        let Some(store) = self.store.as_mut() else {
            return (vec![], vec![]);
        };
        if let Err(error) = store.discard() {
            return file_store_error(error);
        }
        let mut commands = self.card_commands();
        if self.dirty {
            self.dirty = false;
            commands.push(Command::SetAttribute {
                id:        Target::Save.to_dom(),
                attribute: Attribute::Disabled,
                value:     String::new(),
            });
        }
        (vec![], commands)
    }

    pub fn process_lost(&mut self) -> (Vec<Event>, Vec<Command>) {
        if let Some(store) = self.store.take() {
            self.lost = store.pending();
            store.close();
        }
        (vec![], vec![])
    }

    pub fn process_opened(&mut self, opened: Opened) -> (Vec<Event>, Vec<Command>) {
        match opened.and_then(Backend::new) {
            Ok(mut store) => {
                store.replay(core::mem::take(&mut self.lost));
                self.store = Some(store);
                (vec![], vec![])
            }
            Err(error) => {
                (vec![], vec![Command::Error { error: Error::FileStore(error) }, Command::Reload])
            }
        }
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    fn date_commands(&self) -> Vec<Command> {
        let first = self.first_day();
        let days = self.view.days();
        let mut commands: Vec<Command> = (1..=days)
            .map(|n| {
                let day = add_days(first, i64::from(n) - 1);
                Command::SetText {
                    id:    Target::DayTitle(n).to_dom(),
                    value: if self.month() {
                        String::from(youbi(day).label(LANG))
                    } else {
                        display(day, LANG, Format::Short)
                    },
                }
            })
            .collect();
        let span = if self.month() { MONTH_AXIS.days() } else { days };
        let start = display(first, LANG, Format::Long);
        let value = if span == 1 {
            start
        } else {
            format!(
                "{start} – {}",
                display(add_days(first, i64::from(span) - 1), LANG, Format::Long,)
            )
        };
        commands.push(Command::SetText { id: Target::Title.to_dom(), value });
        if self.month() {
            commands.extend(self.axis_commands());
        }
        commands.extend(self.band_commands());
        commands.extend(self.card_commands());
        commands
    }

    fn grid(&self) -> Grid {
        self.view.grid()
    }

    fn day_offset(&self, day: u64) -> i32 {
        (diff(self.base, day) / DAY) as i32
    }

    fn columns(&self) -> u32 {
        self.view.columns()
    }

    fn month(&self) -> bool {
        self.view == View::Month
    }

    fn first_day(&self) -> u64 {
        if self.month() { sub_days(self.base, youbi(self.base) as i64 - 1) } else { self.base }
    }

    fn fit_rectgrid(&mut self) {
        let _ = self.rectgrid.set_definition(
            IncrementFunction::Scale(column_px(self.viewport_width_px, self.view, self.rem_in_px)),
            0,
        );
        let _ = self
            .rectgrid
            .set_definition(IncrementFunction::Scale(self.slot_rem * self.rem_in_px), 1);
    }

    fn change_view(&mut self, view: View) -> Vec<Command> {
        if view == self.view {
            return vec![];
        }
        self.view = view;
        self.fit_rectgrid();
        self.view_commands()
    }

    fn row_commands(&self) -> Vec<Command> {
        let rem = stringify(&RowRem(self.slot_rem));
        let count = if self.month() { MONTH_AXIS.count() } else { SLOT_COUNT };
        let axis = format!("var(--head-height) repeat({count}, {rem}rem)");
        vec![
            style(
                Target::TimeAxis.to_dom(),
                StyleProperty::GridTemplateRows,
                StyleValue::Text(axis),
            ),
            style(
                Target::Surface.to_dom(),
                StyleProperty::GridTemplateRows,
                StyleValue::Text(format!(
                    "var(--head-title) var(--head-resource) calc({rem}rem * {count})"
                )),
            ),
        ]
    }

    fn axis_commands(&self) -> Vec<Command> {
        let first = self.first_day();
        let mut commands = Vec::new();
        for row in 0..SLOT_COUNT {
            let value = if self.month() {
                match MONTH_AXIS.locate([0, row as i32]) {
                    Some((day, None)) => {
                        let (_, month, date, ..) = unpack(add_days(first, i64::from(day)));
                        format!("{month}/{date}")
                    }
                    _ => String::new(),
                }
            } else {
                TIME_AXIS.minutes(row).map_or_else(String::new, format_hhmm)
            };
            for side in 1..=2 {
                commands.push(Command::SetText {
                    id:    Target::AxisLabel(side, row + 2).to_dom(),
                    value: value.clone(),
                });
            }
        }
        commands
    }

    fn view_commands(&self) -> Vec<Command> {
        let days = self.view.days();
        let columns = self.columns();
        let mut commands = vec![
            style(
                Target::Surface.to_dom(),
                StyleProperty::GridTemplateColumns,
                StyleValue::Text(format!("repeat({columns}, minmax(var(--column-width), 1fr))")),
            ),
            style(
                Target::DayList.to_dom(),
                StyleProperty::GridTemplateColumns,
                StyleValue::Text(format!("repeat({days}, 1fr)")),
            ),
        ];
        for n in 1..=DAY_MAX {
            commands.push(hidden(Target::Day(n).to_dom(), n > days));
        }
        commands.extend(self.date_commands());
        for n in 1..=heading_axis().count() {
            commands.push(hidden(Target::Resource(n).to_dom(), self.month() || n > columns));
            commands.push(self.resource_text(n));
        }
        for view in [View::Day, View::ThreeDays, View::Week, View::Month] {
            commands.push(toggle(
                Target::ViewRadio(view).to_dom(),
                Attribute::Checked,
                view == self.view,
            ));
        }
        if !self.month() {
            commands.extend(self.axis_commands());
        }
        commands.extend(self.row_commands());
        commands
    }
}

fn column_of(resources: &[DataStruct], uid: u128) -> Option<usize> {
    resources.iter().position(|resource| tag::Uid::read(resource) == uid)
}

fn shift_cells(
    resources: &[DataStruct],
    cells: &[Place],
    day: u64,
    resource: usize,
) -> Result<Vec<Place>, &'static str> {
    let first = cells.first().ok_or("予約がありません")?;
    let target = tag::Uid::read(resources.get(resource).ok_or("資源が不正です")?);
    let Some(origin) = column_of(resources, first.resource) else {
        return Ok(vec![Place { day, resource: target }]);
    };
    let day_delta = diff(first.day, day) / DAY;
    let resource_delta = resource as i32 - origin as i32;
    cells
        .iter()
        .map(|place| {
            let position = column_of(resources, place.resource).ok_or("資源が不正です")?;
            let index =
                usize::try_from(position as i32 + resource_delta).map_err(|_| "範囲外です")?;
            let moved = tag::Uid::read(resources.get(index).ok_or("範囲外です")?);
            Ok(Place { day: add_days(place.day, day_delta), resource: moved })
        })
        .collect()
}

fn decode_form(
    value: &str,
    resources: &[DataStruct],
    statuses: &[DataStruct],
    categories: &[DataStruct],
) -> Result<DataStruct, &'static str> {
    let pairs = parse_url_search_params(value);
    let field = |field: EditField| {
        let name = format!("{}", field.number());
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
            .ok_or("入力を読み取れません")
    };
    let [title, category, status, date, resource, start, end, note] = [
        EditField::Title,
        EditField::Category,
        EditField::Status,
        EditField::Date,
        EditField::Resource,
        EditField::Start,
        EditField::End,
        EditField::Note,
    ]
    .map(field);
    let (title, category, status, date) = (title?, category?, status?, date?);
    let (resource, start, end, note) = (resource?, start?, end?, note?);
    let day = parse_date(date).map_err(|_| "日付が不正です")?;
    let start = parse_time(start).map_err(|_| "開始時刻が不正です")?;
    let end = parse_time(end).map_err(|_| "終了時刻が不正です")?;
    if start >= end || start < TIME_AXIS.minutes(0).unwrap_or(0) || end > TIME_AXIS.end() {
        return Err("時間帯が不正です");
    }
    let resource = option_position(resources, resource).ok_or("資源が不正です")?;
    let status = option_position(statuses, status).ok_or("状態が不正です")?;
    let category = option_position(categories, category).ok_or("カテゴリが不正です")?;
    let place = Place { day, resource: tag::Uid::read(&resources[resource]) };
    let category = tag::Code::read(&categories[category]);
    let status = tag::Code::read(&statuses[status]);
    Ok(appointment::new(0, &[place], start, end, title, &category, &status, note))
}

fn option_value<T>(items: &[T], selected: impl Fn(&T) -> bool) -> String {
    items.iter().position(selected).map_or_else(String::new, |index| format!("{}", index + 1))
}

fn option_position<T>(items: &[T], value: &str) -> Option<usize> {
    let index = value.parse::<usize>().ok()?.checked_sub(1)?;
    (index < items.len()).then_some(index)
}

fn hours(shifts: &[DataStruct]) -> Option<Hours> {
    let (first, rest) = shifts.split_first()?;
    let (open, close) = shift::Hours::read(first);
    let mut hours = Hours { open, close, rest: shift::Rest::read(first) };
    for s in rest {
        let (open, close) = shift::Hours::read(s);
        hours.open = hours.open.min(open);
        hours.close = hours.close.max(close);
        hours.rest = match (hours.rest, shift::Rest::read(s)) {
            (Some((a, b)), Some((c, d))) if a.max(c) < b.min(d) => Some((a.max(c), b.min(d))),
            _ => None,
        };
    }
    Some(hours)
}

fn fits(shifts: &[DataStruct], (start, end): (u32, u32)) -> bool {
    hours(shifts).is_none_or(|hours| hours.open <= start && end <= hours.close)
}

fn column_px(viewport_width_px: f64, view: View, rem_in_px: f64) -> f64 {
    let columns = view.columns() as f64;
    let visible = viewport_width_px - 2.0 * AXIS_REM * rem_in_px;
    (visible / columns).max(COLUMN_MIN_REM * rem_in_px)
}

fn heading_axis() -> ColumnAxis {
    ColumnAxis::new(DAY_MAX, RESOURCE_COUNT)
}

fn rem(px: f64, rem_in_px: f64) -> StyleValue {
    StyleValue::Length((px / rem_in_px) as f32, Unit::Rem)
}

fn style(id: Id, property: StyleProperty, value: StyleValue) -> Command {
    Command::SetStyle { id, property, value }
}

fn hidden(id: Id, on: bool) -> Command {
    toggle(id, Attribute::Hidden, on)
}

fn toggle(id: Id, attribute: Attribute, on: bool) -> Command {
    if on {
        Command::SetAttribute { id, attribute, value: String::new() }
    } else {
        Command::RemoveAttribute { id, attribute }
    }
}

fn corner_cursor(corner: Corner) -> Option<Keyword> {
    match corner {
        [Some(x), Some(y)] => Some(if x == y { Keyword::NwseResize } else { Keyword::NeswResize }),
        [Some(_), None] => Some(Keyword::EwResize),
        [None, Some(_)] => Some(Keyword::NsResize),
        [None, None] => None,
    }
}

fn file_store_error(error: FileStoreError) -> (Vec<Event>, Vec<Command>) {
    match error {
        FileStoreError::InvalidState(_) => {
            (vec![Event::StoreLost(STORE)], vec![Command::Error { error: Error::FileStore(error) }])
        }
        _ => (vec![], vec![Command::Error { error: Error::FileStore(error) }]),
    }
}

fn data_error(error: DataError) -> Command {
    Command::Error { error: Error::Data(error) }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use std::fs;

    use super::*;
    use crate::{
        Rng, block_on,
        calendar::{
            data::SCHEMA_SIZE,
            grid::{DAY_ROWS, MONTH_WEEKS},
        },
        data_struct::{ID_CREATED_AT, ID_MODIFIED_AT},
        file_store::{MemoryHandles, MemoryStore},
        js_client::KeyName,
    };

    const VIEWPORT: f64 = 1500.0;
    const REM: f64 = 16.0;
    const AXIS_PX: f64 = AXIS_REM * REM;
    const HEAD_PX: f64 = HEAD_REM * REM;
    const COLUMN_MIN_PX: f64 = COLUMN_MIN_REM * REM;
    const SLOT_PX: f64 = SLOT_REM * REM;
    const GRAB_X: f64 = 40.0;
    const GRAB_Y: f64 = 40.0;

    fn after(day: u64, n: i64) -> u64 {
        add_days(day, n)
    }

    fn today() -> u64 {
        pack(2026, 10, 2, 0, 0, 0, 0, 0, 0)
    }

    #[test]
    fn ready_follows_the_local_date() {
        let new_millennium = 946_684_800_000.0;
        for (now, offset, expected) in [
            (new_millennium, 0, (2000, 1, 1)),
            (new_millennium, 540, (2000, 1, 1)),
            (new_millennium, -1, (1999, 12, 31)),
            (new_millennium + 899.0 * 60_000.0, 540, (2000, 1, 1)),
            (new_millennium + 900.0 * 60_000.0, 540, (2000, 1, 2)),
        ] {
            let handler = block_on(Handler::ready(VIEWPORT, 0.0, REM, now, offset));
            let (year, month, day) = expected;
            assert_eq!(handler.base(), pack(year, month, day, 0, 0, 0, 0, 0, 0), "{now} {offset}");
        }
    }
    const ROOT: (f64, f64) = (0.0, 40.0);

    fn click(id: Id, x: f64, y: f64) -> CanvasEvent {
        CanvasEvent {
            event_type: EventType::Click,
            id,
            key: KeyName::Other,
            flags: 0,
            value: String::new(),
            x,
            y,
            local_x: x - ROOT.0,
            local_y: y - ROOT.1,
            time: 0.0,
            pointer_id: 0,
        }
    }

    fn pointer() -> Pointer {
        Pointer::default()
    }

    fn grid_columns(commands: &[Command]) -> Vec<(Id, String)> {
        commands
            .iter()
            .filter_map(|command| match command {
                Command::SetStyle {
                    id,
                    property: StyleProperty::GridTemplateColumns,
                    value: StyleValue::Text(text),
                } => Some((id.clone(), text.clone())),
                _ => None,
            })
            .collect()
    }

    fn grid_rows(commands: &[Command]) -> Vec<(Id, String)> {
        commands
            .iter()
            .filter_map(|command| match command {
                Command::SetStyle {
                    id,
                    property: StyleProperty::GridTemplateRows,
                    value: StyleValue::Text(text),
                } => Some((id.clone(), text.clone())),
                _ => None,
            })
            .collect()
    }

    fn week_grid(columns: u32, days: u32) -> Vec<(Id, String)> {
        vec![
            (
                Target::Surface.to_dom(),
                format!("repeat({columns}, minmax(var(--column-width), 1fr))"),
            ),
            (Target::DayList.to_dom(), format!("repeat({days}, 1fr)")),
        ]
    }

    fn hidden_count(commands: &[Command]) -> usize {
        commands
            .iter()
            .filter(|command| {
                matches!(command, Command::SetAttribute { attribute: Attribute::Hidden, .. })
            })
            .count()
    }

    #[test]
    fn initial_draw_sets_week_geometry() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let (_, commands) = handler.initial_draw();
        assert_eq!(grid_columns(&commands), week_grid(28, 7));
        assert_eq!(hidden_count(&commands), 0);
        assert!(matches!(commands.last(),
            Some(Command::RemoveAttribute { id, attribute: Attribute::Hidden })
                if *id == Target::Body.to_dom()));
    }

    #[test]
    fn view_radio_switches_days_columns_and_hides_the_rest() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let commands = choose(&mut handler, View::ThreeDays);
        assert_eq!(handler.view(), View::ThreeDays);
        assert_eq!(grid_columns(&commands), week_grid(12, 3));
        assert_eq!(hidden_count(&commands), 4 + 16);
        let checked: Vec<Id> = commands
            .iter()
            .filter_map(|command| match command {
                Command::SetAttribute { id, attribute: Attribute::Checked, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(checked, [Target::ViewRadio(View::ThreeDays).to_dom()]);

        let commands = choose(&mut handler, View::Day);
        assert_eq!(handler.view(), View::Day);
        assert_eq!(grid_columns(&commands), week_grid(4, 1));
        assert_eq!(hidden_count(&commands), 6 + 24);
    }

    #[test]
    fn view_radio_for_the_current_view_or_a_click_emits_nothing() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        assert!(choose(&mut handler, View::Week).is_empty());
        assert!(press(&mut handler, Target::ViewRadio(View::Day).to_dom()).is_empty());
        assert_eq!(handler.view(), View::Week);
    }

    #[test]
    fn cell_follows_column_width_of_the_view() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let x = AXIS_PX + COLUMN_MIN_PX * 5.0 + 1.0;
        let y = ROOT.1 + HEAD_PX + SLOT_PX * 3.0 + 1.0;
        assert_eq!(handler.cell_at(ROOT, x, y), Some((1, 1, 3)));

        handler.change_view(View::Day);
        let column_px = (VIEWPORT - 2.0 * AXIS_PX) / 4.0;
        let x = AXIS_PX + column_px * 3.0 + 1.0;
        assert_eq!(handler.cell_at(ROOT, x, y), Some((0, 3, 3)));
    }

    #[test]
    fn cell_outside_the_grid_is_none() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        assert_eq!(handler.cell_at(ROOT, AXIS_PX - 1.0, ROOT.1 + HEAD_PX + 1.0), None);
        assert_eq!(handler.cell_at(ROOT, AXIS_PX + 1.0, ROOT.1 + HEAD_PX - 1.0), None);
        let below = ROOT.1 + HEAD_PX + SLOT_PX * SLOT_COUNT as f64 + 1.0;
        assert_eq!(handler.cell_at(ROOT, AXIS_PX + 1.0, below), None);
    }

    #[test]
    fn cell_accounts_for_horizontal_scroll() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        handler.scroll_x = COLUMN_MIN_PX * 4.0;
        let x = AXIS_PX + 1.0;
        let y = ROOT.1 + HEAD_PX + 1.0;
        assert_eq!(handler.cell_at(ROOT, x, y), Some((1, 0, 0)));
    }

    fn texts(commands: &[Command]) -> Vec<(Id, String)> {
        commands
            .iter()
            .filter_map(|command| match command {
                Command::SetText { id, value } => Some((id.clone(), value.clone())),
                _ => None,
            })
            .collect()
    }

    fn text_of(commands: &[Command], id: &Id) -> Option<String> {
        texts(commands).into_iter().find(|(target, _)| target == id).map(|(_, value)| value)
    }

    fn choose(handler: &mut Handler, view: View) -> Vec<Command> {
        let event = CanvasEvent {
            event_type: EventType::Change,
            ..click(Target::ViewRadio(view).to_dom(), 10.0, 10.0)
        };
        handler.process_canvas(&event, &pointer()).1
    }

    fn press(handler: &mut Handler, id: Id) -> Vec<Command> {
        handler.process_canvas(&click(id, 10.0, 10.0), &pointer()).1
    }

    #[test]
    fn initial_draw_titles_the_week_from_today() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let (_, commands) = handler.initial_draw();
        assert_eq!(text_of(&commands, &Target::DayTitle(1).to_dom()).unwrap(), "10/2(金)");
        assert_eq!(text_of(&commands, &Target::DayTitle(7).to_dom()).unwrap(), "10/8(木)");
        assert_eq!(
            text_of(&commands, &Target::Title.to_dom()).unwrap(),
            "2026年10月2日(金) – 2026年10月8日(木)"
        );
    }

    #[test]
    fn step_buttons_move_the_base_and_today_returns() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        for (button, expected) in [(5, 7), (1, 0), (2, -1), (4, 0), (4, 1)] {
            press(&mut handler, Target::Step(button).to_dom());
            assert_eq!(diff(today(), handler.base()) / DAY, expected);
        }
        let commands = press(&mut handler, Target::Step(3).to_dom());
        assert_eq!(handler.base(), today());
        assert_eq!(text_of(&commands, &Target::DayTitle(1).to_dom()).unwrap(), "10/2(金)");
    }

    #[test]
    fn single_day_view_titles_one_date() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        choose(&mut handler, View::Day);
        let commands = press(&mut handler, Target::Step(4).to_dom());
        assert_eq!(text_of(&commands, &Target::Title.to_dom()).unwrap(), "2026年10月3日(土)");
        assert_eq!(texts(&commands).len(), 2);
    }

    fn sample_response(request: u32, status: u16) -> Response {
        let body = fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/distribution/calendar/data/calendar.json"
        ))
        .unwrap();
        Response { request, status, body }
    }

    #[test]
    fn initial_draw_requests_the_calendar_once() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let (_, commands) = handler.initial_draw();
        let fetches: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                Command::Fetch { request, method, path, body } => {
                    Some((*request, *method, path.clone(), body.len()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(fetches, [(1, Method::Get, String::from("data/calendar.json"), 0)]);
    }

    #[test]
    fn fetched_calendar_is_kept_and_names_the_resources() {
        let (mut handler, _) = with_store();
        assert!(!handler.loaded());
        let (_, commands) = handler.process_fetched(&sample_response(1, 200));
        assert_eq!(appointments(&handler).len(), 380);
        assert_eq!(text_of(&commands, &Target::ResourceName(1).to_dom()).unwrap(), "Studio 1");
        assert_eq!(text_of(&commands, &Target::ResourceName(6).to_dom()).unwrap(), "Studio 2");
        assert_eq!(text_of(&commands, &Target::ResourceName(28).to_dom()).unwrap(), "Studio 4");
    }

    #[test]
    fn view_change_keeps_loaded_resource_names() {
        let (mut handler, _) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let commands = choose(&mut handler, View::Day);
        assert_eq!(text_of(&commands, &Target::ResourceName(2).to_dom()).unwrap(), "Studio 2");
    }

    #[test]
    fn fetched_failure_reports_a_data_error_and_keeps_nothing() {
        let (mut handler, _) = with_store();
        let (_, commands) = handler.process_fetched(&sample_response(1, 404));
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::Data(DataError::Status(404)) }]
        ));
        let broken = Response { request: 1, status: 200, body: b"{".to_vec() };
        let (_, commands) = handler.process_fetched(&broken);
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::Data(DataError::Parse(_)) }]
        ));
        assert!(!handler.loaded());
    }

    #[test]
    fn fetched_for_another_request_is_ignored() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let (_, commands) = handler.process_fetched(&sample_response(9, 200));
        assert!(commands.is_empty());
        assert!(!handler.loaded());
    }

    fn entry(id: u32, day: u64, resource: u128, start: u32, end: u32) -> DataStruct {
        appointment::new(
            id,
            &[Place { day, resource }],
            start,
            end,
            &format!("t{id}"),
            CATEGORY_CODE,
            STATUS_CODE,
            "",
        )
    }

    const STATUS_KEY: u32 = 900;
    const CATEGORY_KEY: u32 = 901;
    const STATUS_CODE: &str = "done";
    const CATEGORY_CODE: &str = "c";
    const SHIFT_KEY: u32 = 1000;

    fn appointments(handler: &Handler) -> Vec<DataStruct> {
        handler.read(appointment::KIND)
    }

    fn shifts(handler: &Handler) -> Vec<DataStruct> {
        handler.read(shift::KIND)
    }

    fn load(handler: &mut Handler, appointments: Vec<DataStruct>) {
        if handler.store.is_none() {
            handler.attach(Backend::new(MemoryHandles::default()).unwrap());
        }
        let store = handler.store.as_mut().unwrap();
        for id in 101..105u32 {
            let uid = u128::from(id);
            data::put(
                store,
                &tag::new(id, appointment::RESOURCE, uid, "", &format!("Studio {}", id - 100)),
            );
        }
        data::put(store, &tag::new(STATUS_KEY, appointment::STATUS, 0, STATUS_CODE, "D"));
        data::put(store, &tag::new(CATEGORY_KEY, appointment::CATEGORY, 0, CATEGORY_CODE, "C"));
        let shifts = (-3..11).flat_map(|day| (101..105u128).map(move |resource| (day, resource)));
        for (index, (day, resource)) in shifts.enumerate() {
            let shift = shift::new(
                SHIFT_KEY + index as u32,
                after(today(), day),
                resource,
                "p",
                540,
                1200,
                None,
            );
            data::put(store, &shift);
        }
        for entry in &appointments {
            data::put(store, entry);
        }
        store.save().unwrap();
    }

    fn card_box(commands: &[Command], n: u32) -> Option<(f32, f32, f32, f32)> {
        let length = |property: StyleProperty| {
            commands.iter().find_map(|command| match command {
                Command::SetStyle { id, property: p, value }
                    if *id == Target::Card(n).to_dom() && *p == property =>
                {
                    Some(value.clone())
                }
                _ => None,
            })
        };
        let px = |value: &StyleValue| match value {
            StyleValue::Length(value, Unit::Rem) => Some(*value * REM as f32),
            _ => None,
        };
        let StyleValue::List(translate) = length(StyleProperty::Translate)? else { return None };
        let [x, y] = translate.as_slice() else { return None };
        Some((
            px(x)?,
            px(y)?,
            px(&length(StyleProperty::Width)?)?,
            px(&length(StyleProperty::Height)?)?,
        ))
    }

    fn shown_count(commands: &[Command]) -> usize {
        commands
            .iter()
            .filter(|command| {
                matches!(command, Command::RemoveAttribute { id, attribute: Attribute::Hidden }
                    if id.0.len() == 4 && id.0[2].n == Some(4))
            })
            .count()
    }

    #[test]
    fn cards_are_placed_in_one_batch_inside_the_range() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        load(
            &mut handler,
            vec![
                entry(1, base, 101, 600, 660),
                entry(2, base, 101, 630, 690),
                entry(3, after(base, 10), 101, 600, 660),
                entry(4, after(base, 1), 102, 480, 570),
                entry(5, after(base, -1), 101, 600, 660),
                entry(6, base, 999, 600, 660),
            ],
        );
        let commands = handler.card_commands();
        assert_eq!(shown_count(&commands), 3);
        assert_eq!(card_box(&commands, 1), Some((0.0, 112.0, 40.0, 112.0)));
        assert_eq!(card_box(&commands, 2), Some((40.0, 168.0, 40.0, 112.0)));
        let (x, y, width, height) = card_box(&commands, 3).unwrap();
        assert_eq!((x, y, width, height), (400.0, 0.0, 80.0, 56.0));
        assert_eq!(
            text_of(&commands, &Target::CardPart(3, CardPart::Status).to_dom()).unwrap(),
            "D"
        );
        assert_eq!(
            text_of(&commands, &Target::CardPart(1, CardPart::Time).to_dom()).unwrap(),
            "10:00–11:00"
        );
        assert_eq!(
            text_of(&commands, &Target::CardPart(2, CardPart::Title).to_dom()).unwrap(),
            "t2"
        );
    }

    #[test]
    fn narrowing_the_range_hides_the_surplus_cards_once() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        load(
            &mut handler,
            vec![
                entry(1, base, 101, 600, 660),
                entry(2, after(base, 1), 101, 600, 660),
                entry(3, after(base, 2), 101, 600, 660),
            ],
        );
        assert_eq!(shown_count(&handler.card_commands()), 3);
        let commands = choose(&mut handler, View::Day);
        assert_eq!(shown_count(&commands), 1);
        let hidden_cards: Vec<u32> = commands
            .iter()
            .filter_map(|command| match command {
                Command::SetAttribute { id, attribute: Attribute::Hidden, .. }
                    if id.0.len() == 4 && id.0[2].n == Some(4) =>
                {
                    id.0[3].n
                }
                _ => None,
            })
            .collect();
        assert_eq!(hidden_cards, [2, 3]);
        let commands = press(&mut handler, Target::Step(4).to_dom());
        assert_eq!(shown_count(&commands), 1);
        assert_eq!(
            text_of(&commands, &Target::CardPart(1, CardPart::Title).to_dom()).unwrap(),
            "t2"
        );
    }

    #[test]
    fn cards_follow_the_column_width_of_the_view() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        load(&mut handler, vec![entry(1, base, 103, 540, 600)]);
        let commands = choose(&mut handler, View::Day);
        let column_px = (VIEWPORT - 2.0 * AXIS_PX) / 4.0;
        let (x, y, width, height) = card_box(&commands, 1).unwrap();
        assert_eq!(x, (column_px * 2.0) as f32);
        assert_eq!((y, height), (0.0, 112.0));
        assert_eq!(width, column_px as f32);
    }

    #[test]
    fn sample_week_places_every_visible_appointment() {
        let (mut handler, _) = with_store();
        let (_, commands) = handler.process_fetched(&sample_response(1, 200));
        let expected = appointments(&handler)
            .iter()
            .flat_map(|a| appointment::Places::read(&a))
            .filter(|cell| (0..7).contains(&(diff(today(), cell.day) / DAY)))
            .count();
        assert!(expected > 0);
        assert_eq!(shown_count(&commands), expected);
    }

    #[test]
    fn resize_replaces_the_cards() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        load(&mut handler, vec![entry(1, base, 101, 600, 660)]);
        handler.card_commands();
        choose(&mut handler, View::Day);
        let (_, commands) = handler.process_resize(1000.0, 800.0);
        let column_px = (1000.0 - 2.0 * AXIS_PX) / 4.0;
        assert_eq!(card_box(&commands, 1).unwrap().2, column_px as f32);
    }

    #[test]
    fn rem_constants_match_the_markup() {
        let html = include_str!("../../distribution/calendar/index.html");
        let css = include_str!("../../distribution/calendar/css/style.css");
        assert!(html.contains(&format!("var(--head-height) repeat({SLOT_COUNT}, {SLOT_REM}rem)")));
        assert!(html.contains(&format!(
            "var(--head-title) var(--head-resource) calc({SLOT_REM}rem * {SLOT_COUNT})"
        )));
        assert!(css.contains(&format!("--column-width:  {COLUMN_MIN_REM}rem")));
        assert!(css.contains(&format!("--axis-width:    {AXIS_REM}rem")));
        assert!(css.contains("--head-title:    2rem"));
        assert!(css.contains("--head-resource: 3rem"));
        assert_eq!(HEAD_REM, 2.0 + 3.0);
    }

    #[test]
    fn cards_scale_with_the_root_font_size() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, 20.0);
        load(&mut handler, vec![entry(1, base, 101, 600, 660)]);
        let commands = handler.card_commands();
        let height = commands.iter().find_map(|command| match command {
            Command::SetStyle { property: StyleProperty::Height, value, .. } => Some(value.clone()),
            _ => None,
        });
        assert_eq!(height, Some(StyleValue::Length(4.0 * SLOT_REM as f32, Unit::Rem)));
    }

    fn pointer_down(id: Id, x: f64, y: f64) -> CanvasEvent {
        CanvasEvent { event_type: EventType::PointerDown, ..click(id, x, y) }
    }

    fn grid_origin() -> (f64, f64) {
        (ROOT.0 + AXIS_PX, ROOT.1 + HEAD_PX)
    }

    fn drag_fixture() -> (Handler, u64) {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        load(&mut handler, vec![entry(1, base, 101, 600, 660), entry(2, base, 102, 540, 600)]);
        handler.card_commands();
        (handler, base)
    }

    fn grab(handler: &mut Handler, n: u32) -> (f64, f64) {
        let (ox, oy) = grid_origin();
        let (x, y) = (ox + GRAB_X, oy + 4.0 * SLOT_PX + GRAB_Y);
        handler.process_canvas(
            &pointer_down(Target::CardPart(n, CardPart::Title).to_dom(), x, y),
            &pointer(),
        );
        (x, y)
    }

    fn drag_to(handler: &mut Handler, x: f64, y: f64) -> Vec<Command> {
        handler.process_gesture(&Gesture::Drag { x, y }, &pointer(), None).1
    }

    fn end(handler: &mut Handler) -> Vec<Command> {
        handler.process_gesture(&Gesture::DragEnd, &pointer(), None).1
    }

    fn translate_of(commands: &[Command], n: u32) -> Option<(f32, f32)> {
        commands.iter().find_map(|command| match command {
            Command::SetStyle { id, property: StyleProperty::Translate, value }
                if *id == Target::Card(n).to_dom() =>
            {
                let StyleValue::List(list) = value else { return None };
                let [StyleValue::Length(x, _), StyleValue::Length(y, _)] = list.as_slice() else {
                    return None;
                };
                Some((*x * REM as f32, *y * REM as f32))
            }
            _ => None,
        })
    }

    #[test]
    fn pressing_a_card_part_starts_a_pending_drag() {
        let (mut handler, _) = drag_fixture();
        grab(&mut handler, 1);
        assert!(handler.drag.as_ref().is_some_and(|drag| drag.n == 1 && !drag.moved));
        handler.process_canvas(
            &pointer_down(Target::ViewRadio(View::Day).to_dom(), 10.0, 10.0),
            &pointer(),
        );
        assert!(handler.drag.is_none());
    }

    #[test]
    fn dragging_moves_the_card_with_the_pointer_and_raises_it_once() {
        let (mut handler, _) = drag_fixture();
        let (x, y) = grab(&mut handler, 1);
        let first = drag_to(&mut handler, x + 100.0, y + 56.0);
        assert!(matches!(
            first.first(),
            Some(Command::SetStyle { property: StyleProperty::ZIndex, .. })
        ));
        assert_eq!(translate_of(&first, 1), Some((100.0, 112.0 + 56.0)));
        let second = drag_to(&mut handler, x + 120.0, y + 28.0);
        assert_eq!(second.len(), 1);
        assert_eq!(translate_of(&second, 1), Some((120.0, 112.0 + 28.0)));
    }

    #[test]
    fn drop_moves_to_the_snapped_column_and_slot_keeping_the_duration() {
        let (mut handler, base) = drag_fixture();
        let (_, oy) = grid_origin();
        grab(&mut handler, 1);
        let column = 4 * 2 + 2;
        let pointer_x = grid_origin().0 + column as f64 * COLUMN_MIN_PX + 10.0;
        let pointer_y = oy + 10.0 * SLOT_PX + GRAB_Y + 5.0;
        drag_to(&mut handler, pointer_x, pointer_y);
        let commands = end(&mut handler);
        let moved = &appointments(&handler)[0];
        assert_eq!(appointment::Places::read(&moved)[0].day, after(base, 2));
        assert_eq!(appointment::Places::read(&moved)[0].resource, 103);
        assert_eq!(appointment::Range::read(&moved), (540 + 150, 540 + 210));
        assert_eq!(shown_count(&commands), 2);
        assert!(matches!(
            commands.first(),
            Some(Command::RemoveStyle { property: StyleProperty::ZIndex, .. })
        ));
    }

    #[test]
    fn drop_rounds_the_card_top_to_the_nearest_slot() {
        for (extra, expected_slot) in [(0.4, 10), (0.6, 11)] {
            let (mut handler, _) = drag_fixture();
            let (_, oy) = grid_origin();
            grab(&mut handler, 1);
            let pointer_y = oy + (10.0 + extra) * SLOT_PX + GRAB_Y;
            drag_to(&mut handler, grid_origin().0 + 5.0, pointer_y);
            end(&mut handler);
            let moved = &appointments(&handler)[0];
            assert_eq!(
                appointment::Range::read(&moved).0,
                540 + expected_slot * 15,
                "extra {extra}"
            );
        }
    }

    #[test]
    fn drop_near_the_end_of_the_day_keeps_the_card_inside() {
        let (mut handler, _) = drag_fixture();
        let (ox, oy) = grid_origin();
        let grab_y = 14.0;
        handler.process_canvas(
            &pointer_down(
                Target::CardPart(1, CardPart::Title).to_dom(),
                ox + GRAB_X,
                oy + 4.0 * SLOT_PX + grab_y,
            ),
            &pointer(),
        );
        drag_to(&mut handler, ox + 5.0, oy + 43.2 * SLOT_PX + grab_y);
        end(&mut handler);
        let moved = &appointments(&handler)[0];
        assert_eq!(appointment::Range::read(&moved), (20 * 60 - 60, 20 * 60));
    }

    #[test]
    fn drop_outside_the_grid_leaves_the_data_and_restores_the_card() {
        let (mut handler, base) = drag_fixture();
        let (ox, oy) = grid_origin();
        grab(&mut handler, 1);
        for (x, y) in
            [(ox - 30.0, oy + 100.0), (ox + 30.0, oy - 20.0), (ox + 30.0, oy + 45.0 * SLOT_PX)]
        {
            drag_to(&mut handler, x, y);
            let commands = end(&mut handler);
            let unchanged = &appointments(&handler)[0];
            assert_eq!(
                (
                    appointment::Places::read(&unchanged)[0].day,
                    appointment::Places::read(&unchanged)[0].resource,
                    appointment::Range::read(&unchanged).0
                ),
                (base, 101, 600)
            );
            assert_eq!(translate_of(&commands, 1), Some((0.0, 112.0)));
            grab(&mut handler, 1);
        }
    }

    #[test]
    fn cancel_restores_the_card_and_keeps_the_data() {
        let (mut handler, base) = drag_fixture();
        let (x, y) = grab(&mut handler, 1);
        drag_to(&mut handler, x + 200.0, y + 200.0);
        let commands = handler.process_gesture(&Gesture::DragCancel, &pointer(), None).1;
        assert!(handler.drag.is_none());
        assert_eq!(translate_of(&commands, 1), Some((0.0, 112.0)));
        let unchanged = &appointments(&handler)[0];
        assert_eq!(
            (appointment::Places::read(&unchanged)[0].day, appointment::Range::read(&unchanged).0),
            (base, 600)
        );
    }

    #[test]
    fn drag_gestures_without_a_pressed_card_do_nothing() {
        let (mut handler, _) = drag_fixture();
        assert!(drag_to(&mut handler, 100.0, 100.0).is_empty());
        assert!(end(&mut handler).is_empty());
    }

    #[test]
    fn drag_in_the_day_view_uses_the_wider_columns() {
        let (mut handler, base) = drag_fixture();
        choose(&mut handler, View::Day);
        handler.card_commands();
        let column_px = (VIEWPORT - 2.0 * AXIS_PX) / 4.0;
        let (ox, oy) = grid_origin();
        handler.process_canvas(
            &pointer_down(
                Target::CardPart(1, CardPart::Title).to_dom(),
                ox + GRAB_X,
                oy + 4.0 * SLOT_PX + GRAB_Y,
            ),
            &pointer(),
        );
        drag_to(&mut handler, ox + column_px * 3.0 + 5.0, oy + 4.0 * SLOT_PX + GRAB_Y);
        end(&mut handler);
        let moved = &appointments(&handler)[0];
        assert_eq!(
            (
                appointment::Places::read(&moved)[0].day,
                appointment::Places::read(&moved)[0].resource
            ),
            (base, 104)
        );
    }

    #[test]
    fn dropping_on_every_unit_of_every_view_resolves_like_the_column_axis() {
        for (view, days) in [(View::Day, 1u32), (View::ThreeDays, 3), (View::Week, 7)] {
            let base = today();
            let mut handler = Handler::new(VIEWPORT, base, REM);
            load(&mut handler, vec![entry(1, base, 101, 600, 660)]);
            choose(&mut handler, view);
            let axis = ColumnAxis::new(days, RESOURCE_COUNT);
            let column_px = column_px(VIEWPORT, handler.view(), REM);
            let (ox, oy) = grid_origin();
            for unit in 0..axis.count() {
                handler.card_commands();
                let (grab_x, grab_y) = (ox + GRAB_X, oy + 4.0 * SLOT_PX + GRAB_Y);
                handler.process_canvas(
                    &pointer_down(Target::CardPart(1, CardPart::Title).to_dom(), grab_x, grab_y),
                    &pointer(),
                );
                drag_to(&mut handler, ox + unit as f64 * column_px + 3.0, grab_y);
                end(&mut handler);
                let moved = &appointments(&handler)[0];
                let cell = axis.cell(unit).unwrap();
                assert_eq!(
                    appointment::Places::read(&moved)[0].day,
                    after(base, i64::from(cell.day)),
                    "view {days} unit {unit}"
                );
                assert_eq!(
                    appointment::Places::read(&moved)[0].resource,
                    101 + u128::from(cell.resource),
                    "view {days} unit {unit}"
                );
                assert_eq!(appointment::Range::read(&moved).0, 600);
            }
        }
    }

    #[test]
    fn the_same_card_box_resolves_to_different_data_after_a_view_switch() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        let week = handler.grid();
        let bx = week.bbox(Cell { day: 1, resource: 1 }, 600, 660).unwrap();
        assert_eq!(week.resolve(&bx).unwrap().cells, [Cell { day: 1, resource: 1 }]);
        choose(&mut handler, View::Day);
        assert!(handler.grid().resolve(&bx).is_none());
        choose(&mut handler, View::Week);
        assert_eq!(handler.grid().resolve(&bx).unwrap().cells, [Cell { day: 1, resource: 1 }]);
    }

    fn multi_fixture(view: View) -> (Handler, u64) {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        let mut spanning = entry(1, base, 102, 600, 660);
        let mut cells = appointment::Places::read(&spanning);
        cells.push(Place { day: base, resource: 103 });
        appointment::Places::write(&mut spanning, &cells, None).unwrap();
        load(&mut handler, vec![spanning]);
        choose(&mut handler, view);
        handler.card_commands();
        (handler, base)
    }

    fn open_at(handler: &mut Handler, minutes: u32) {
        let store = handler.store.as_mut().unwrap();
        for shift in data::entries(store, shift::KIND).unwrap() {
            let opened = shift::new(
                data::key(&shift),
                shift::Day::read(&shift),
                shift::Resource::read(&shift),
                &shift::Person::read(&shift),
                minutes,
                shift::Hours::read(&shift).1,
                shift::Rest::read(&shift),
            );
            data::put(store, &opened);
        }
    }

    fn drag_piece(handler: &mut Handler, n: u32, from_unit: u32, to_unit: u32) {
        let column_px = column_px(VIEWPORT, handler.view(), REM);
        let (ox, oy) = grid_origin();
        let y = oy + 4.0 * SLOT_PX + GRAB_Y;
        handler.process_canvas(
            &pointer_down(
                Target::CardPart(n, CardPart::Title).to_dom(),
                ox + from_unit as f64 * column_px + GRAB_X,
                y,
            ),
            &pointer(),
        );
        drag_to(handler, ox + to_unit as f64 * column_px + 5.0, y);
        end(handler);
    }

    fn places(handler: &Handler) -> Vec<(u64, u128)> {
        appointment::Places::read(&appointments(&handler)[0])
            .iter()
            .map(|place| (place.day, place.resource))
            .collect()
    }

    #[test]
    fn a_multi_cell_appointment_renders_one_card_per_visible_cell() {
        let (handler, _) = multi_fixture(View::Week);
        let commands = handler.card_commands();
        assert_eq!(shown_count(&commands), 2);
        let (x1, y1, w1, h1) = card_box(&commands, 1).unwrap();
        let (x2, y2, w2, h2) = card_box(&commands, 2).unwrap();
        assert_eq!((x2 - x1, y1, w1, h1), (80.0, y2, w2, h2));
        assert_eq!(
            text_of(&commands, &Target::CardPart(1, CardPart::Title).to_dom()),
            text_of(&commands, &Target::CardPart(2, CardPart::Title).to_dom())
        );
    }

    #[test]
    fn dragging_any_cell_moves_every_cell_by_the_same_unit_difference() {
        let base = today();
        let (mut handler, _) = multi_fixture(View::Week);
        drag_piece(&mut handler, 2, 2, 3);
        assert_eq!(places(&handler), [(base, 103), (base, 104)]);

        let (mut handler, _) = multi_fixture(View::Week);
        drag_piece(&mut handler, 1, 1, 0);
        assert_eq!(places(&handler), [(base, 101), (base, 102)]);
    }

    #[test]
    fn a_shift_past_the_last_resource_continues_into_the_next_day() {
        let base = today();
        let (mut handler, _) = multi_fixture(View::Week);
        drag_piece(&mut handler, 1, 1, 3);
        assert_eq!(places(&handler), [(base, 104), (after(base, 1), 101)]);
        let commands = handler.card_commands();
        assert_eq!(shown_count(&commands), 2);
    }

    #[test]
    fn a_cell_outside_the_view_still_moves_with_the_others() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        let mut crossing = entry(1, base, 104, 600, 660);
        let mut cells = appointment::Places::read(&crossing);
        cells.push(Place { day: after(base, 1), resource: 101 });
        appointment::Places::write(&mut crossing, &cells, None).unwrap();
        load(&mut handler, vec![crossing]);
        choose(&mut handler, View::Day);
        let commands = handler.card_commands();
        assert_eq!(shown_count(&commands), 1);
        drag_piece(&mut handler, 1, 3, 0);
        assert_eq!(places(&handler), [(base, 101), (base, 102)]);
    }

    #[test]
    fn dragging_a_multi_cell_card_back_to_its_own_cell_changes_nothing() {
        let (mut handler, _) = multi_fixture(View::Week);
        let before = places(&handler);
        drag_piece(&mut handler, 2, 2, 2);
        assert_eq!(places(&handler), before);
    }

    fn lone(unit_resource: u32) -> Handler {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        load(&mut handler, vec![entry(1, base, u128::from(unit_resource), 600, 660)]);
        handler.card_commands();
        handler
    }

    fn press_local(handler: &mut Handler, n: u32, x: f64, y: f64) -> Vec<Command> {
        let (ox, oy) = grid_origin();
        handler
            .process_canvas(
                &pointer_down(Target::CardPart(n, CardPart::Title).to_dom(), ox + x, oy + y),
                &pointer(),
            )
            .1
    }

    fn resize_corner(handler: &Handler) -> Option<Corner> {
        match &handler.drag.as_ref()?.kind {
            DragKind::Resize { corner, .. } => Some(*corner),
            DragKind::Move => None,
        }
    }

    fn drag_by(handler: &mut Handler, dx: f64, dy: f64) -> Vec<Command> {
        let [x, y] = handler.drag.as_ref().unwrap().pointer;
        drag_to(handler, x + dx, y + dy)
    }

    fn cursor_of(commands: &[Command]) -> Option<Keyword> {
        commands.iter().find_map(|command| match command {
            Command::SetStyle {
                property: StyleProperty::Cursor,
                value: StyleValue::Keyword(k),
                ..
            } => Some(*k),
            _ => None,
        })
    }

    fn times(handler: &Handler) -> (u32, u32) {
        appointment::Range::read(&appointments(handler)[0])
    }

    #[test]
    fn pressing_an_edge_or_corner_starts_a_resize_with_a_matching_cursor() {
        let cases: [(f64, f64, Option<Corner>, Option<Keyword>); 8] = [
            (120.0, 221.0, Some([None, Some(false)]), Some(Keyword::NsResize)),
            (120.0, 115.0, Some([None, Some(true)]), Some(Keyword::NsResize)),
            (83.0, 168.0, Some([Some(true), None]), Some(Keyword::EwResize)),
            (157.0, 168.0, Some([Some(false), None]), Some(Keyword::EwResize)),
            (157.0, 221.0, Some([Some(false), Some(false)]), Some(Keyword::NwseResize)),
            (83.0, 221.0, Some([Some(true), Some(false)]), Some(Keyword::NeswResize)),
            (83.0, 115.0, Some([Some(true), Some(true)]), Some(Keyword::NwseResize)),
            (120.0, 168.0, None, None),
        ];
        for (x, y, corner, cursor) in cases {
            let mut handler = lone(102);
            let commands = press_local(&mut handler, 1, x, y);
            assert_eq!(resize_corner(&handler), corner, "({x},{y})");
            assert_eq!(cursor_of(&commands), cursor, "({x},{y})");
        }
    }

    #[test]
    fn dragging_the_bottom_edge_moves_the_end_in_whole_slots() {
        let mut handler = lone(102);
        press_local(&mut handler, 1, 120.0, 221.0);
        drag_by(&mut handler, 0.0, 2.0 * SLOT_PX);
        end(&mut handler);
        assert_eq!(times(&handler), (600, 690));
        assert_eq!(places(&handler), [(today(), 102)]);
    }

    #[test]
    fn a_small_movement_does_not_jump_a_slot() {
        let mut handler = lone(102);
        press_local(&mut handler, 1, 120.0, 221.0);
        drag_by(&mut handler, 0.0, 5.0);
        end(&mut handler);
        assert_eq!(times(&handler), (600, 660));
        let mut handler = lone(102);
        press_local(&mut handler, 1, 120.0, 115.0);
        drag_by(&mut handler, 0.0, -5.0);
        end(&mut handler);
        assert_eq!(times(&handler), (600, 660));
    }

    #[test]
    fn half_a_slot_rounds_to_the_nearest_boundary() {
        for (dy, end_minutes) in [(0.4 * SLOT_PX, 660), (0.6 * SLOT_PX, 675), (-0.6 * SLOT_PX, 645)]
        {
            let mut handler = lone(102);
            press_local(&mut handler, 1, 120.0, 221.0);
            drag_by(&mut handler, 0.0, dy);
            end(&mut handler);
            assert_eq!(times(&handler), (600, end_minutes), "dy {dy}");
        }
    }

    #[test]
    fn dragging_the_top_edge_moves_the_start() {
        let mut handler = lone(102);
        press_local(&mut handler, 1, 120.0, 115.0);
        drag_by(&mut handler, 0.0, -2.0 * SLOT_PX);
        end(&mut handler);
        assert_eq!(times(&handler), (570, 660));
    }

    #[test]
    fn the_size_never_drops_below_one_slot() {
        let mut handler = lone(102);
        press_local(&mut handler, 1, 120.0, 221.0);
        drag_by(&mut handler, 0.0, -20.0 * SLOT_PX);
        end(&mut handler);
        assert_eq!(times(&handler), (600, 615));
        let mut handler = lone(102);
        press_local(&mut handler, 1, 120.0, 115.0);
        drag_by(&mut handler, 0.0, 20.0 * SLOT_PX);
        end(&mut handler);
        assert_eq!(times(&handler), (645, 660));
    }

    #[test]
    fn the_edges_stop_at_the_ends_of_the_day() {
        let mut handler = lone(102);
        press_local(&mut handler, 1, 120.0, 221.0);
        drag_by(&mut handler, 0.0, 100.0 * SLOT_PX);
        end(&mut handler);
        assert_eq!(times(&handler), (600, 1200));
        let mut handler = lone(102);
        press_local(&mut handler, 1, 120.0, 115.0);
        drag_by(&mut handler, 0.0, -100.0 * SLOT_PX);
        end(&mut handler);
        assert_eq!(times(&handler), (540, 660));
    }

    #[test]
    fn dragging_the_right_edge_adds_cells_to_the_right() {
        let base = today();
        let mut handler = lone(102);
        press_local(&mut handler, 1, 157.0, 168.0);
        drag_by(&mut handler, 2.0 * COLUMN_MIN_PX, 0.0);
        end(&mut handler);
        assert_eq!(places(&handler), [(base, 102), (base, 103), (base, 104)]);
        assert_eq!(times(&handler), (600, 660));
    }

    #[test]
    fn the_right_edge_continues_into_the_next_day() {
        let base = today();
        let mut handler = lone(102);
        press_local(&mut handler, 1, 157.0, 168.0);
        drag_by(&mut handler, 4.0 * COLUMN_MIN_PX, 0.0);
        end(&mut handler);
        assert_eq!(
            places(&handler),
            [(base, 102), (base, 103), (base, 104), (after(base, 1), 101), (after(base, 1), 102)]
        );
    }

    #[test]
    fn dragging_the_left_edge_adds_cells_to_the_left() {
        let base = today();
        let mut handler = lone(102);
        press_local(&mut handler, 1, 83.0, 168.0);
        drag_by(&mut handler, -COLUMN_MIN_PX, 0.0);
        end(&mut handler);
        assert_eq!(places(&handler), [(base, 101), (base, 102)]);
    }

    #[test]
    fn dragging_the_right_edge_back_removes_cells() {
        let base = today();
        let (mut handler, _) = multi_fixture(View::Week);
        press_local(&mut handler, 2, 3.0 * COLUMN_MIN_PX - 3.0, 168.0);
        assert_eq!(resize_corner(&handler), Some([Some(false), None]));
        drag_by(&mut handler, -COLUMN_MIN_PX, 0.0);
        end(&mut handler);
        assert_eq!(places(&handler), [(base, 102)]);
    }

    #[test]
    fn a_corner_resizes_both_axes_at_once() {
        let base = today();
        let mut handler = lone(102);
        press_local(&mut handler, 1, 157.0, 221.0);
        drag_by(&mut handler, COLUMN_MIN_PX, 2.0 * SLOT_PX);
        end(&mut handler);
        assert_eq!(places(&handler), [(base, 102), (base, 103)]);
        assert_eq!(times(&handler), (600, 690));
    }

    #[test]
    fn horizontal_handles_exist_only_on_the_outer_cells() {
        let (mut handler, _) = multi_fixture(View::Week);
        press_local(&mut handler, 1, 2.0 * COLUMN_MIN_PX - 3.0, 168.0);
        assert_eq!(resize_corner(&handler), None);
        press_local(&mut handler, 2, 2.0 * COLUMN_MIN_PX + 3.0, 168.0);
        assert_eq!(resize_corner(&handler), None);
        press_local(&mut handler, 1, COLUMN_MIN_PX + 3.0, 168.0);
        assert_eq!(resize_corner(&handler), Some([Some(true), None]));
        press_local(&mut handler, 2, 3.0 * COLUMN_MIN_PX - 3.0, 168.0);
        assert_eq!(resize_corner(&handler), Some([Some(false), None]));
    }

    #[test]
    fn horizontal_handles_exist_only_on_the_outer_lanes() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        load(&mut handler, vec![entry(1, base, 101, 600, 660), entry(2, base, 101, 630, 690)]);
        handler.card_commands();
        press_local(&mut handler, 1, 40.0 - 3.0, 168.0);
        assert_eq!(resize_corner(&handler), None);
        press_local(&mut handler, 1, 3.0, 168.0);
        assert_eq!(resize_corner(&handler), Some([Some(true), None]));
        press_local(&mut handler, 2, 40.0 + 3.0, 196.0 + 56.0);
        assert_eq!(resize_corner(&handler), None);
        press_local(&mut handler, 2, 77.0, 196.0 + 28.0);
        assert_eq!(resize_corner(&handler), Some([Some(false), None]));
    }

    #[test]
    fn a_cell_outside_the_view_is_kept_by_a_vertical_resize_and_joined_by_a_horizontal_one() {
        let base = today();
        let make = || {
            let mut handler = Handler::new(VIEWPORT, base, REM);
            let mut crossing = entry(1, base, 104, 600, 660);
            let mut cells = appointment::Places::read(&crossing);
            cells.push(Place { day: after(base, 1), resource: 101 });
            appointment::Places::write(&mut crossing, &cells, None).unwrap();
            load(&mut handler, vec![crossing]);
            choose(&mut handler, View::Day);
            handler.card_commands();
            handler
        };
        let column_px = (VIEWPORT - 2.0 * AXIS_PX) / 4.0;
        let mut handler = make();
        press_local(&mut handler, 1, 3.0 * column_px + 100.0, 221.0);
        drag_by(&mut handler, 0.0, SLOT_PX);
        end(&mut handler);
        assert_eq!(places(&handler), [(base, 104), (after(base, 1), 101)]);
        assert_eq!(times(&handler), (600, 675));

        let mut handler = make();
        press_local(&mut handler, 1, 3.0 * column_px + 3.0, 168.0);
        assert_eq!(resize_corner(&handler), Some([Some(true), None]));
        drag_by(&mut handler, -2.0 * column_px, 0.0);
        end(&mut handler);
        assert_eq!(
            places(&handler),
            [(base, 102), (base, 103), (base, 104), (after(base, 1), 101)]
        );
    }

    #[test]
    fn the_resize_preview_spans_the_whole_box_and_the_end_clears_the_handle_styles() {
        let mut handler = lone(102);
        press_local(&mut handler, 1, 157.0, 221.0);
        let commands = drag_by(&mut handler, 2.0 * COLUMN_MIN_PX, SLOT_PX);
        let width = commands.iter().find_map(|command| match command {
            Command::SetStyle { property: StyleProperty::Width, value, .. } => Some(value.clone()),
            _ => None,
        });
        let height = commands.iter().find_map(|command| match command {
            Command::SetStyle { property: StyleProperty::Height, value, .. } => Some(value.clone()),
            _ => None,
        });
        assert_eq!(width, Some(StyleValue::Length(3.0 * COLUMN_MIN_REM as f32, Unit::Rem)));
        assert_eq!(height, Some(StyleValue::Length(5.0 * SLOT_REM as f32, Unit::Rem)));
        assert!(matches!(
            commands.first(),
            Some(Command::SetStyle { property: StyleProperty::ZIndex, .. })
        ));
        let released = end(&mut handler);
        let cleared: Vec<StyleProperty> = released
            .iter()
            .filter_map(|command| match command {
                Command::RemoveStyle { property, .. } => Some(*property),
                _ => None,
            })
            .collect();
        assert_eq!(cleared, [StyleProperty::ZIndex, StyleProperty::Cursor]);
    }

    #[test]
    fn cancelling_a_resize_keeps_the_data() {
        let mut handler = lone(102);
        press_local(&mut handler, 1, 157.0, 221.0);
        drag_by(&mut handler, COLUMN_MIN_PX, 3.0 * SLOT_PX);
        let commands = handler.process_gesture(&Gesture::DragCancel, &pointer(), None).1;
        assert_eq!(times(&handler), (600, 660));
        assert_eq!(places(&handler), [(today(), 102)]);
        assert_eq!(translate_of(&commands, 1), Some((COLUMN_MIN_PX as f32, 112.0)));
        assert!(commands.iter().any(|command| matches!(
            command,
            Command::RemoveStyle { property: StyleProperty::Cursor, .. }
        )));
    }

    #[test]
    fn a_move_drag_does_not_set_a_resize_cursor() {
        let mut handler = lone(102);
        let commands = press_local(&mut handler, 1, 120.0, 168.0);
        assert!(commands.is_empty());
        let released = {
            drag_by(&mut handler, 10.0, 0.0);
            end(&mut handler)
        };
        assert!(!released.iter().any(|command| matches!(
            command,
            Command::RemoveStyle { property: StyleProperty::Cursor, .. }
        )));
    }

    fn save_disabled(commands: &[Command]) -> Vec<bool> {
        commands
            .iter()
            .filter_map(|command| match command {
                Command::SetAttribute { id, attribute: Attribute::Disabled, .. }
                    if *id == Target::Save.to_dom() =>
                {
                    Some(true)
                }
                Command::RemoveAttribute { id, attribute: Attribute::Disabled }
                    if *id == Target::Save.to_dom() =>
                {
                    Some(false)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn edits_enable_the_save_button_once() {
        let mut handler = lone(102);
        assert!(!handler.dirty());
        press_local(&mut handler, 1, 120.0, 221.0);
        drag_by(&mut handler, 0.0, 2.0 * SLOT_PX);
        let commands = end(&mut handler);
        assert!(handler.dirty());
        assert_eq!(save_disabled(&commands), [false]);
        press_local(&mut handler, 1, 120.0, 168.0);
        drag_by(&mut handler, COLUMN_MIN_PX, 0.0);
        let commands = end(&mut handler);
        assert!(save_disabled(&commands).is_empty());
    }

    #[test]
    fn a_drop_that_changes_nothing_keeps_the_save_button_disabled() {
        let mut handler = lone(102);
        press_local(&mut handler, 1, 120.0, 168.0);
        drag_by(&mut handler, 10.0, 3.0);
        let commands = end(&mut handler);
        assert!(!handler.dirty());
        assert!(save_disabled(&commands).is_empty());
        press_local(&mut handler, 1, 120.0, 221.0);
        drag_by(&mut handler, 0.0, 5.0);
        end(&mut handler);
        assert!(!handler.dirty());
    }

    #[test]
    fn a_cancelled_or_out_of_grid_drag_does_not_make_the_data_dirty() {
        let mut handler = lone(102);
        press_local(&mut handler, 1, 120.0, 168.0);
        drag_by(&mut handler, -500.0, 0.0);
        end(&mut handler);
        press_local(&mut handler, 1, 120.0, 168.0);
        drag_by(&mut handler, 100.0, 100.0);
        handler.process_gesture(&Gesture::DragCancel, &pointer(), None);
        assert!(!handler.dirty());
    }

    fn with_store() -> (Handler, MemoryHandles) {
        let disk = MemoryHandles::default();
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        handler.attach(Backend::new(disk.clone()).unwrap());
        (handler, disk)
    }

    fn pending(handler: &Handler) -> usize {
        handler.store.as_ref().unwrap().index().count_pending()
    }

    fn drag_first_card_by(handler: &mut Handler, dy: f64) -> (u32, u32) {
        handler.card_commands();
        let (index, base) = {
            let placed = handler.placed.borrow();
            (placed[0].key, placed[0].base)
        };
        let before = appointment::Range::read(&appointment_of(&handler, index)).0;
        let (ox, oy) = grid_origin();
        handler.process_canvas(
            &pointer_down(
                Target::CardPart(1, CardPart::Title).to_dom(),
                ox + base[0] + 40.0,
                oy + base[1] + 20.0,
            ),
            &pointer(),
        );
        drag_by(handler, 0.0, dy);
        end(handler);
        (index, before)
    }

    fn fields(entries: &[DataStruct]) -> Vec<Vec<Option<Vec<u8>>>> {
        entries
            .iter()
            .map(|entry| {
                (1..=SCHEMA_SIZE).map(|id| entry.get(id).ok().map(<[u8]>::to_vec)).collect()
            })
            .collect()
    }

    fn committed_appointments(disk: &MemoryHandles) -> Vec<DataStruct> {
        data::entries(&MemoryStore::new(disk.clone()).unwrap(), appointment::KIND).unwrap()
    }

    fn committed(disk: &MemoryHandles, key: u32) -> DataStruct {
        data::entry(&MemoryStore::new(disk.clone()).unwrap(), key, appointment::KIND).unwrap()
    }

    fn appointment_of(handler: &Handler, key: u32) -> DataStruct {
        handler.appointment(key).unwrap()
    }

    #[test]
    fn a_first_run_fetches_seeds_and_saves_the_records() {
        let (mut handler, disk) = with_store();
        assert!(!handler.loaded());
        let (_, commands) = handler.initial_draw();
        assert!(commands.iter().any(|command| matches!(command, Command::Fetch { .. })));
        assert_eq!(disk.count_committed(), 0);
        handler.process_fetched(&sample_response(1, 200));
        assert_eq!(disk.count_committed(), 593);
        assert_eq!(pending(&handler), 0);
        assert!(!handler.dirty());
    }

    #[test]
    fn a_later_run_loads_from_the_store_without_fetching() {
        let (mut first, disk) = with_store();
        first.process_fetched(&sample_response(1, 200));
        let mut second = Handler::new(VIEWPORT, today(), REM);
        second.attach(Backend::new(disk).unwrap());
        assert_eq!(appointments(&second).len(), 380);
        let (_, commands) = second.initial_draw();
        assert!(!commands.iter().any(|command| matches!(command, Command::Fetch { .. })));
        assert_eq!(text_of(&commands, &Target::ResourceName(1).to_dom()).unwrap(), "Studio 1");
        assert!(shown_count(&commands) > 0);
    }

    #[test]
    fn an_edit_is_pending_until_save_and_survives_a_reload_only_after_it() {
        let (mut handler, disk) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let after = appointment::Range::read(&appointment_of(&handler, index)).0;
        assert_eq!(after, before + 30);
        assert!(handler.dirty());
        assert_eq!(pending(&handler), 1);
        assert_eq!(appointment::Range::read(&committed(&disk, index)).0, before);

        press(&mut handler, Target::Save.to_dom());
        assert!(!handler.dirty());
        assert_eq!(pending(&handler), 0);
        assert_eq!(appointment::Range::read(&committed(&disk, index)).0, after);

        let mut reloaded = Handler::new(VIEWPORT, today(), REM);
        reloaded.attach(Backend::new(disk).unwrap());
        assert_eq!(appointment::Range::read(&appointment_of(&reloaded, index)).0, after);
    }

    #[test]
    fn an_unsaved_edit_is_gone_after_a_reload() {
        let (mut handler, disk) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let mut reloaded = Handler::new(VIEWPORT, today(), REM);
        reloaded.attach(Backend::new(disk).unwrap());
        assert_eq!(appointment::Range::read(&appointment_of(&reloaded, index)).0, before);
    }

    #[test]
    fn discard_restores_the_saved_state_and_redraws() {
        let (mut handler, _) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let commands = handler.discard_commands().1;
        assert_eq!(appointment::Range::read(&appointment_of(&handler, index)).0, before);
        assert!(!handler.dirty());
        assert_eq!(pending(&handler), 0);
        assert_eq!(save_disabled(&commands), [true]);
        assert!(shown_count(&commands) > 0);
    }

    fn band_point(slot: f64) -> [Px; 2] {
        let (ox, oy) = grid_origin();
        [Px::new(ox + 10.0), Px::new(oy + slot * SLOT_PX)]
    }

    #[test]
    fn fetched_shifts_become_three_bands_with_names_and_a_hit_test() {
        let (mut handler, _) = with_store();
        let (_, commands) = handler.process_fetched(&sample_response(1, 200));
        handler.set_origin(ROOT);
        let shift = shifts(&handler).into_iter().find(|s| shift::Day::read(&s) == today()).unwrap();
        let resource = handler
            .tags(appointment::RESOURCE)
            .iter()
            .position(|r| tag::Uid::read(r) == shift::Resource::read(&shift));
        let resource = resource.unwrap() as u32;
        let kinds: Vec<BandKind> = handler.bands.borrow().iter().map(|band| band.kind).collect();
        assert_eq!(kinds, [BandKind::Closed, BandKind::Closed, BandKind::Break]);
        let hit = |slot| handler.band_hit(band_point(slot));
        assert_eq!(hit(0.5), Some(BandKind::Closed));
        assert_eq!(hit(1.9), Some(BandKind::Closed));
        assert_eq!(hit(5.0), None);
        assert_eq!(hit(14.5), Some(BandKind::Break));
        assert_eq!(hit(38.5), Some(BandKind::Closed));
        assert_eq!(hit(43.5), Some(BandKind::Closed));
        assert_eq!(
            text_of(&commands, &Target::ResourcePerson(resource + 1).to_dom()).unwrap(),
            shift::Person::read(&shift)
        );
        let shown = commands
            .iter()
            .filter(|command| {
                matches!(command, Command::RemoveAttribute { id, attribute: Attribute::Hidden }
                    if *id == Target::Band(3).to_dom())
            })
            .count();
        assert_eq!(shown, 1);
    }

    #[test]
    fn bands_span_every_visible_column() {
        let (mut handler, _) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let width = |handler: &Handler| handler.bands.borrow()[0].bx.offset()[0].get();
        assert_eq!(width(&handler), 28.0);
        choose(&mut handler, View::Day);
        assert_eq!(width(&handler), 4.0);
    }

    fn value_of(commands: &[Command], id: &Id) -> Option<String> {
        commands.iter().find_map(|command| match command {
            Command::SetValue { id: i, value } if i == id => Some(value.clone()),
            _ => None,
        })
    }

    #[test]
    fn clicking_a_card_without_moving_opens_the_edit_form() {
        let (mut handler, base) = drag_fixture();
        grab(&mut handler, 1);
        let (_, commands) = handler.process_gesture(&Gesture::Tap, &pointer(), None);
        let first = appointments(&handler)[0].clone();
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Title).to_dom()).unwrap(),
            appointment::Title::read(&first)
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Date).to_dom()).unwrap(),
            display(base, Lang::Ja, Format::Date)
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Resource).to_dom()).unwrap(),
            "1"
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Start).to_dom()).unwrap(),
            "10:00"
        );
        assert!(
            matches!(commands.last(), Some(Command::ShowModal { id }) if *id == Target::Modal.to_dom())
        );
        assert!(!handler.dirty());
    }

    fn tap_empty(handler: &mut Handler, column: f64, slot: f64) -> Vec<Command> {
        let (ox, oy) = grid_origin();
        let (x, y) = (
            ox + (column + 0.5) * column_px(VIEWPORT, handler.view(), REM),
            oy + (slot + 0.5) * SLOT_PX,
        );
        handler.process_canvas(&pointer_down(Target::Surface.to_dom(), x, y), &pointer());
        handler.process_gesture(&Gesture::Tap, &pointer(), None).1
    }

    fn form_query(title: &str, date: &str, resource: u32, start: &str, end: &str) -> String {
        format!("1={title}&2=1&3=1&4={date}&5={resource}&6={start}&7={end}&8=n")
    }

    fn submit(handler: &mut Handler, query: &str) -> Vec<Command> {
        let event = CanvasEvent {
            event_type: EventType::Submit,
            value: String::from(query),
            ..click(Target::EditForm.to_dom(), 0.0, 0.0)
        };
        handler.process_canvas(&event, &pointer()).1
    }

    #[test]
    fn tapping_an_empty_cell_opens_a_new_form_at_that_slot() {
        let (mut handler, base) = drag_fixture();
        let commands = tap_empty(&mut handler, 2.0, 4.0);
        assert_eq!(value_of(&commands, &Target::EditField(EditField::Title).to_dom()).unwrap(), "");
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Date).to_dom()).unwrap(),
            display(base, Lang::Ja, Format::Date)
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Resource).to_dom()).unwrap(),
            "3"
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Start).to_dom()).unwrap(),
            "10:00"
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::End).to_dom()).unwrap(),
            "11:00"
        );
        assert!(
            matches!(commands.last(), Some(Command::ShowModal { id }) if *id == Target::Modal.to_dom())
        );
        assert!(matches!(handler.editing, Some(Editing::New(_))));
    }

    fn drag_create(handler: &mut Handler, from: (f64, f64), to: (f64, f64)) -> Vec<Command> {
        let (ox, oy) = grid_origin();
        let column = column_px(VIEWPORT, handler.view(), REM);
        let at = |(c, s): (f64, f64)| (ox + (c + 0.5) * column, oy + (s + 0.5) * SLOT_PX);
        let (x, y) = at(from);
        handler.process_canvas(&pointer_down(Target::Surface.to_dom(), x, y), &pointer());
        let (x, y) = at(to);
        let mid = drag_to(handler, x, y);
        assert!(
            mid.iter().any(
                |c| matches!(c, Command::SetStyle { id, .. } if *id == Target::Preview.to_dom())
            )
        );
        end(handler)
    }

    #[test]
    fn dragging_over_empty_cells_opens_a_form_for_the_range() {
        let (mut handler, base) = drag_fixture();
        let commands = drag_create(&mut handler, (2.0, 8.0), (2.0, 11.0));
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Start).to_dom()).unwrap(),
            "11:00"
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::End).to_dom()).unwrap(),
            "12:00"
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Resource).to_dom()).unwrap(),
            "3"
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Date).to_dom()).unwrap(),
            display(base, Lang::Ja, Format::Date)
        );
        assert!(commands.iter().any(|c| matches!(c,
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. } if *id == Target::Preview.to_dom())));
        assert!(matches!(commands.last(), Some(Command::ShowModal { .. })));
    }

    #[test]
    fn dragging_upward_and_across_columns_makes_a_multi_cell_appointment() {
        let (mut handler, base) = drag_fixture();
        drag_create(&mut handler, (5.0, 14.0), (2.0, 10.0));
        let Some(Editing::New(cells)) = handler.editing.clone() else {
            panic!("no new form");
        };
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0], Place { day: base, resource: 103 });
        assert_eq!(cells[3], Place { day: after(base, 1), resource: 102 });
        let query = form_query("Span", &display(base, Lang::Ja, Format::Date), 3, "12:30", "14:30");
        submit(&mut handler, &query);
        let all = appointments(&handler);
        let added = all.last().unwrap();
        assert_eq!(appointment::Places::read(&added).len(), 4);
        assert_eq!(appointment::Range::read(&added), (750, 870));
    }

    #[test]
    fn a_small_drag_in_one_slot_behaves_like_a_tap() {
        let (mut handler, _) = drag_fixture();
        let commands = drag_create(&mut handler, (2.0, 8.0), (2.0, 8.0));
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::End).to_dom()).unwrap(),
            "12:00"
        );
    }

    #[test]
    fn cancelling_a_create_drag_hides_the_preview() {
        let (mut handler, _) = drag_fixture();
        let (ox, oy) = grid_origin();
        handler.process_canvas(
            &pointer_down(Target::Surface.to_dom(), ox + 10.0, oy + 10.0),
            &pointer(),
        );
        drag_to(&mut handler, ox + 10.0, oy + 3.0 * SLOT_PX);
        let (_, commands) = handler.process_gesture(&Gesture::DragCancel, &pointer(), None);
        assert!(commands.iter().any(|c| matches!(c,
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. } if *id == Target::Preview.to_dom())));
        assert!(handler.create.is_none());
    }

    fn input(id: Id, value: &str) -> CanvasEvent {
        CanvasEvent {
            event_type: EventType::Input,
            value: String::from(value),
            ..click(id, 0.0, 0.0)
        }
    }

    #[test]
    fn the_zoom_slider_rescales_the_rows_and_redraws() {
        let (mut handler, _) = drag_fixture();
        let before = handler.placed.borrow()[0].base[1];
        let (_, commands) = handler.process_canvas(&input(Target::Zoom.to_dom(), "3"), &pointer());
        assert_eq!(
            grid_rows(&commands),
            [
                (Target::TimeAxis.to_dom(), String::from("var(--head-height) repeat(44, 3rem)")),
                (
                    Target::Surface.to_dom(),
                    String::from("var(--head-title) var(--head-resource) calc(3rem * 44)")
                ),
            ]
        );
        let after = handler.placed.borrow()[0].base[1];
        assert_eq!(after, before / SLOT_REM * 3.0);
        assert!(
            handler.process_canvas(&input(Target::Zoom.to_dom(), "3"), &pointer()).1.is_empty()
        );
        handler.process_canvas(&input(Target::Zoom.to_dom(), "9"), &pointer());
        assert_eq!(handler.slot_rem, ZOOM_MAX);
    }

    #[test]
    fn a_drag_after_zooming_still_snaps_to_slots() {
        let (mut handler, _) = drag_fixture();
        handler.process_canvas(&input(Target::Zoom.to_dom(), "2.5"), &pointer());
        let (ox, oy) = grid_origin();
        let slot = 2.5 * REM;
        handler.process_canvas(
            &pointer_down(Target::Surface.to_dom(), ox + 10.0, oy + 5.0 * slot + 3.0),
            &pointer(),
        );
        drag_to(&mut handler, ox + 10.0, oy + 7.0 * slot + 3.0);
        let commands = end(&mut handler);
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Start).to_dom()).unwrap(),
            "10:15"
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::End).to_dom()).unwrap(),
            "11:00"
        );
    }

    #[test]
    fn tapping_the_header_does_not_open_a_form() {
        let (mut handler, _) = drag_fixture();
        let (ox, oy) = grid_origin();
        handler.process_canvas(
            &pointer_down(Target::Surface.to_dom(), ox + 10.0, oy - 5.0),
            &pointer(),
        );
        let (_, commands) = handler.process_gesture(&Gesture::Tap, &pointer(), None);
        assert!(commands.is_empty());
    }

    #[test]
    fn submitting_a_new_form_adds_a_persisted_appointment() {
        let (mut handler, _) = with_store();
        handler.now = pack(2026, 10, 2, 12, 0, 0, 0, 1, 0);
        handler.process_fetched(&sample_response(1, 200));
        let before = appointments(&handler).len();
        let shift = shifts(&handler)[0].clone();
        tap_empty(&mut handler, 0.0, 0.0);
        let date = display(shift::Day::read(&shift), Lang::Ja, Format::Date);
        let (start, end) = (
            format_hhmm(shift::Hours::read(&shift).0),
            format_hhmm(shift::Hours::read(&shift).0 + 60),
        );
        let column = handler
            .tags(appointment::RESOURCE)
            .iter()
            .position(|r| tag::Uid::read(r) == shift::Resource::read(&shift));
        let query = form_query("Neo", &date, column.unwrap() as u32 + 1, &start, &end);
        let commands = submit(&mut handler, &query);
        let all = appointments(&handler);
        assert_eq!(all.len(), before + 1);
        let added = all.last().unwrap();
        assert_eq!(
            (
                appointment::Title::read(added).as_str(),
                appointment::Range::read(&added).0,
                appointment::Range::read(&added).1
            ),
            ("Neo", shift::Hours::read(&shift).0, shift::Hours::read(&shift).0 + 60)
        );
        assert_eq!(
            (appointment::Category::read(&added), appointment::Status::read(&added)),
            (
                tag::Code::read(&handler.tags(appointment::CATEGORY)[0]),
                tag::Code::read(&handler.tags(appointment::STATUS)[0])
            )
        );
        assert_ne!(appointment::Uid::read(&added), 0);
        assert_eq!(
            added.get(ID_CREATED_AT).unwrap(),
            handler.now.to_le_bytes(),
            "a new record is created now"
        );
        assert!(handler.dirty());
        assert!(commands.iter().any(|c| matches!(c, Command::CloseModal { .. })));
        assert!(pending(&handler) == 1);
        assert!(handler.editing.is_none());
        let key = data::key(&added);
        let saved =
            DataStruct::from_bytes(handler.store.as_ref().unwrap().get(key).unwrap(), SCHEMA_SIZE)
                .unwrap();
        assert_eq!(saved.get(ID_MODIFIED_AT).unwrap(), handler.now.to_le_bytes());
    }

    #[test]
    fn a_form_missing_a_field_is_rejected() {
        let (mut handler, base) = drag_fixture();
        tap_empty(&mut handler, 0.0, 0.0);
        let query = form_query("x", &display(base, Lang::Ja, Format::Date), 1, "09:30", "10:30");
        let missing = query.replace("&8=n", "");
        let commands = submit(&mut handler, &missing);
        assert_eq!(
            text_of(&commands, &Target::EditMessage.to_dom()).unwrap(),
            "入力を読み取れません"
        );
        assert_eq!(appointments(&handler).len(), 2);
        assert!(handler.editing.is_some());
    }

    #[test]
    fn a_form_outside_the_open_hours_is_rejected_but_an_unchanged_time_is_not() {
        let (mut handler, base) = drag_fixture();
        tap_empty(&mut handler, 0.0, 0.0);
        open_at(&mut handler, 660);
        let early = form_query("x", &display(base, Lang::Ja, Format::Date), 1, "09:30", "10:30");
        let commands = submit(&mut handler, &early);
        assert_eq!(text_of(&commands, &Target::EditMessage.to_dom()).unwrap(), "営業時間外です");
        assert_eq!(appointments(&handler).len(), 2);
        grab(&mut handler, 1);
        handler.process_gesture(&Gesture::Tap, &pointer(), None);
        let same = form_query("kept", &display(base, Lang::Ja, Format::Date), 1, "10:00", "11:00");
        submit(&mut handler, &same);
        assert_eq!(appointment::Title::read(&appointments(&handler)[0]), "kept");
    }

    #[test]
    fn a_drag_before_the_open_hours_is_refused() {
        let (mut handler, _) = drag_fixture();
        open_at(&mut handler, 660);
        let before = appointments(&handler)[0].clone();
        let (x, y) = grab(&mut handler, 1);
        drag_to(&mut handler, x, y + 2.0 * SLOT_PX);
        end(&mut handler);
        assert_eq!(
            appointment::Range::read(&appointments(&handler)[0]).0,
            appointment::Range::read(&before).0
        );
        assert!(!handler.dirty());
    }

    #[test]
    fn clicking_the_backdrop_closes_the_modal() {
        let (mut handler, _) = drag_fixture();
        tap_empty(&mut handler, 0.0, 0.0);
        let commands = press(&mut handler, Target::Modal.to_dom());
        assert!(matches!(commands.as_slice(), [Command::CloseModal { .. }]));
        assert!(handler.editing.is_none());
    }

    fn modal_closed(handler: &mut Handler) -> Vec<Command> {
        let event =
            CanvasEvent { event_type: EventType::Close, ..click(Target::Modal.to_dom(), 0.0, 0.0) };
        handler.process_canvas(&event, &pointer()).1
    }

    #[test]
    fn closing_the_dialog_natively_clears_the_editing_state() {
        let (mut handler, _) = drag_fixture();
        tap_empty(&mut handler, 0.0, 0.0);
        assert!(handler.editing.is_some());
        assert!(modal_closed(&mut handler).is_empty());
        assert!(handler.editing.is_none());
    }

    #[test]
    fn the_close_echo_of_an_app_closed_dialog_changes_nothing() {
        let (mut handler, _) = drag_fixture();
        tap_empty(&mut handler, 0.0, 0.0);
        press(&mut handler, Target::Modal.to_dom());
        assert!(handler.editing.is_none());
        assert!(modal_closed(&mut handler).is_empty());
        assert!(handler.editing.is_none());
    }

    #[test]
    fn reload_button_discards_pending_edits() {
        let (mut handler, _) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let commands = press(&mut handler, Target::Reload.to_dom());
        assert_eq!(appointment::Range::read(&appointment_of(&handler, index)).0, before);
        assert_eq!(pending(&handler), 0);
        assert_eq!(save_disabled(&commands), [true]);
    }

    #[test]
    fn a_failed_save_reports_the_error_and_stays_dirty() {
        let (mut handler, _) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        handler.store.as_ref().unwrap().fail_writes_to.set(Some("log"));
        let commands = press(&mut handler, Target::Save.to_dom());
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::FileStore(FileStoreError::InvalidState(_)) }]
        ));
        assert!(handler.dirty());
        handler.store.as_ref().unwrap().fail_writes_to.set(None);
        press(&mut handler, Target::Save.to_dom());
        assert!(!handler.dirty());
    }

    #[test]
    fn closing_the_handler_releases_the_store() {
        let (mut handler, _) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        assert!(handler.close().is_empty());
        let (events, commands) = save_pressed(&mut handler);
        assert!(matches!(events.as_slice(), [Event::StoreLost(_)]));
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::FileStore(FileStoreError::InvalidState(_)) }]
        ));
    }

    #[test]
    fn only_an_invalid_handle_asks_for_a_new_one() {
        let (events, commands) = file_store_error(FileStoreError::InvalidState(String::new()));
        assert!(
            matches!(events.as_slice(), [Event::StoreLost(id)] if id.name == "calendar" && id.version == "0.1")
        );
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::FileStore(FileStoreError::InvalidState(_)) }]
        ));
        let (events, commands) = file_store_error(FileStoreError::QuotaExceeded(String::new()));
        assert!(events.is_empty());
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::FileStore(FileStoreError::QuotaExceeded(_)) }]
        ));
    }

    fn save_pressed(handler: &mut Handler) -> (Vec<Event>, Vec<Command>) {
        handler.process_canvas(&click(Target::Save.to_dom(), 10.0, 10.0), &pointer())
    }

    #[test]
    fn a_lost_store_is_replaced_and_the_pending_edit_survives() {
        let (mut handler, disk) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, _) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let after = appointment::Range::read(&appointment_of(&handler, index)).0;

        handler.close();
        let (events, _) = save_pressed(&mut handler);
        assert!(matches!(events.as_slice(), [Event::StoreLost(_)]));
        assert!(handler.process_lost().1.is_empty());
        assert!(save_pressed(&mut handler).1.is_empty());

        let reopened = handler.process_opened(Ok(disk.clone()));
        assert!(reopened.0.is_empty() && reopened.1.is_empty());
        assert!(handler.dirty());
        save_pressed(&mut handler);
        assert!(!handler.dirty());
        assert_eq!(appointment::Range::read(&committed(&disk, index)).0, after);
    }

    #[test]
    fn a_store_that_cannot_be_reopened_asks_for_a_reload() {
        let (mut handler, _) = with_store();
        let failed = Err(FileStoreError::InvalidState(String::new()));
        let (_, commands) = handler.process_opened(failed);
        assert!(matches!(
            commands.as_slice(),
            [
                Command::Error { error: Error::FileStore(FileStoreError::InvalidState(_)) },
                Command::Reload
            ]
        ));
    }

    #[test]
    fn a_corrupt_store_is_reported_and_nothing_is_fetched_over_it() {
        let (mut seeding, disk) = with_store();
        seeding.process_fetched(&sample_response(1, 200));
        let mut corrupting = MemoryStore::new(disk.clone()).unwrap();
        corrupting.set(data::key(&appointments(&seeding)[0]), b"{".to_vec());
        corrupting.save().unwrap();
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        handler.attach(Backend::new(disk).unwrap());
        let (_, commands) = handler.initial_draw();
        assert!(commands.iter().any(|command| matches!(
            command,
            Command::Error { error: Error::Data(DataError::Format(_)) }
        )));
        assert!(!commands.iter().any(|command| matches!(command, Command::Fetch { .. })));
    }

    #[test]
    fn save_and_discard_without_a_store_do_nothing() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        assert!(press(&mut handler, Target::Save.to_dom()).is_empty());
        assert!(handler.discard_commands().1.is_empty());
    }

    fn month_fixture(appointments: Vec<DataStruct>) -> Handler {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        load(&mut handler, appointments);
        choose(&mut handler, View::Month);
        handler
    }

    fn month_first() -> u64 {
        pack(2026, 9, 28, 0, 0, 0, 0, 0, 0)
    }

    fn month_point(day: u32, row: Option<u32>) -> (f64, f64) {
        let bx = MONTH_AXIS.bbox(day, row).unwrap();
        let (ox, oy) = grid_origin();
        (
            ox + (bx.base()[0].get() + 0.5) * column_px(VIEWPORT, View::Month, REM),
            oy + (bx.base()[1].get() + 0.5) * SLOT_REM * REM,
        )
    }

    fn placed_ids(handler: &Handler) -> Vec<(u32, [f64; 2])> {
        handler
            .placed
            .borrow()
            .iter()
            .map(|placed| {
                let base = [placed.bx.base()[0].get(), placed.bx.base()[1].get()];
                (placed.key, base)
            })
            .collect()
    }

    fn multi(id: u32, places: &[(u64, u128)]) -> DataStruct {
        let places: Vec<Place> =
            places.iter().map(|(day, resource)| Place { day: *day, resource: *resource }).collect();
        appointment::new(id, &places, 600, 660, "m", CATEGORY_CODE, STATUS_CODE, "")
    }

    #[test]
    fn month_button_lays_out_weeks_inside_the_shared_frame() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        load(&mut handler, vec![]);
        let commands = choose(&mut handler, View::Month);
        assert_eq!(handler.view(), View::Month);
        assert_eq!(grid_columns(&commands), week_grid(7, 7));
        let count = MONTH_WEEKS * (DAY_ROWS + 1);
        let axis = format!("var(--head-height) repeat({count}, {SLOT_REM}rem)");
        assert_eq!(
            grid_rows(&commands),
            [
                (Target::TimeAxis.to_dom(), axis),
                (
                    Target::Surface.to_dom(),
                    format!("var(--head-title) var(--head-resource) calc({SLOT_REM}rem * {count})")
                ),
            ]
        );
        assert_eq!(hidden_count(&commands), 28);
        let text = |target: Target| text_of(&commands, &target.to_dom()).unwrap();
        assert_eq!(text(Target::DayTitle(1)), "月");
        assert_eq!(text(Target::DayTitle(7)), "日");
        assert_eq!(text(Target::Title), "2026年9月28日(月) – 2026年11月1日(日)");
        assert_eq!(text(Target::BandLabel(1)), "28");
        assert_eq!(text(Target::BandLabel(4)), "10/1");
        assert_eq!(text(Target::BandLabel(35)), "11/1");
        assert_eq!(text(Target::AxisLabel(1, 2)), "9/28");
        assert_eq!(text(Target::AxisLabel(2, 2 + DAY_ROWS + 1)), "10/5");
        assert_eq!(text(Target::AxisLabel(1, 3)), "");
        assert!(commands.iter().any(|command| matches!(command,
            Command::SetAttribute { id, attribute: Attribute::AriaCurrent, .. }
                if *id == Target::Band(5).to_dom())));
    }

    #[test]
    fn month_cards_stack_up_to_the_row_count_per_day_in_id_order() {
        let base = today();
        let handler = month_fixture(vec![
            entry(5, base, 101, 540, 600),
            entry(2, base, 102, 900, 960),
            entry(9, base, 103, 600, 660),
            entry(7, base, 104, 1100, 1160),
            multi(3, &[(after(base, 1), 101), (after(base, 1), 102)]),
            entry(4, after(base, -10), 101, 600, 660),
            entry(8, after(base, 35), 101, 600, 660),
        ]);
        assert_eq!(
            placed_ids(&handler),
            [(2, [4.0, 1.0]), (5, [4.0, 2.0]), (7, [4.0, 3.0]), (3, [5.0, 1.0])]
        );
        assert!(
            handler
                .placed
                .borrow()
                .iter()
                .all(|placed| placed.bx.offset()[0].get() == 1.0
                    && placed.bx.offset()[1].get() == 1.0)
        );
    }

    #[test]
    fn dragging_a_month_card_shifts_its_days_and_keeps_the_time_and_resources() {
        let base = today();
        let mut handler = month_fixture(vec![multi(1, &[(base, 101), (after(base, 1), 102)])]);
        let (x, y) = month_point(4, Some(0));
        handler.process_canvas(
            &pointer_down(Target::CardPart(1, CardPart::Title).to_dom(), x, y),
            &pointer(),
        );
        let (x, y) = month_point(13, Some(2));
        drag_to(&mut handler, x, y);
        end(&mut handler);
        let moved = &appointments(&handler)[0];
        assert_eq!(
            appointment::Places::read(moved),
            [
                Place { day: after(base, 9), resource: 101 },
                Place { day: after(base, 10), resource: 102 },
            ]
        );
        assert_eq!(appointment::Range::read(moved), (600, 660));
        assert!(handler.dirty());
        assert_eq!(placed_ids(&handler), [(1, [6.0, 5.0]), (1, [0.0, 9.0])]);
    }

    #[test]
    fn tapping_an_empty_month_cell_opens_a_new_form_on_that_day() {
        let mut handler = month_fixture(vec![]);
        let (x, y) = month_point(10, Some(2));
        handler.process_canvas(&pointer_down(Target::Surface.to_dom(), x, y), &pointer());
        let commands = handler.process_gesture(&Gesture::Tap, &pointer(), None).1;
        let value = |field| value_of(&commands, &Target::EditField(field).to_dom()).unwrap();
        assert_eq!(value(EditField::Date), "2026-10-08");
        assert_eq!(value(EditField::Resource), "1");
        assert_eq!(value(EditField::Start), "09:00");
        assert_eq!(value(EditField::End), "10:00");
    }

    #[test]
    fn tapping_a_month_date_opens_its_day_and_restores_the_time_axis() {
        let mut handler = month_fixture(vec![]);
        let (x, y) = month_point(9, None);
        handler.process_canvas(&pointer_down(Target::Surface.to_dom(), x, y), &pointer());
        let commands = handler.process_gesture(&Gesture::Tap, &pointer(), None).1;
        assert_eq!(handler.view(), View::Day);
        assert_eq!(handler.base(), after(month_first(), 9));
        assert_eq!(text_of(&commands, &Target::AxisLabel(1, 2).to_dom()).unwrap(), "09:00");
        assert_eq!(text_of(&commands, &Target::AxisLabel(2, 45).to_dom()).unwrap(), "19:45");
        assert!(commands.iter().any(|command| matches!(command,
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. }
                if *id == Target::Band(4).to_dom())));
        assert!(handler.editing.is_none());
    }

    #[test]
    fn the_zoom_slider_rescales_the_month_like_the_time_views() {
        let mut handler = month_fixture(vec![entry(1, today(), 101, 600, 660)]);
        let before = card_box(&handler.card_commands(), 1).unwrap();
        let (_, commands) = handler.process_canvas(&input(Target::Zoom.to_dom(), "3"), &pointer());
        let count = MONTH_WEEKS * (DAY_ROWS + 1);
        assert_eq!(
            grid_rows(&commands),
            [
                (Target::TimeAxis.to_dom(), format!("var(--head-height) repeat({count}, 3rem)")),
                (
                    Target::Surface.to_dom(),
                    format!("var(--head-title) var(--head-resource) calc(3rem * {count})")
                ),
            ]
        );
        let after = card_box(&commands, 1).unwrap();
        assert_eq!((after.0, after.2), (before.0, before.2));
        assert_eq!((after.1, after.3), (before.1 / SLOT_REM as f32 * 3.0, 3.0 * REM as f32));
        let bx = MONTH_AXIS.bbox(10, Some(2)).unwrap();
        let (ox, oy) = grid_origin();
        let x = ox + (bx.base()[0].get() + 0.5) * column_px(VIEWPORT, View::Month, REM);
        let y = oy + (bx.base()[1].get() + 0.5) * 3.0 * REM;
        handler.process_canvas(&pointer_down(Target::Surface.to_dom(), x, y), &pointer());
        let commands = handler.process_gesture(&Gesture::Tap, &pointer(), None).1;
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Date).to_dom()).unwrap(),
            "2026-10-08"
        );
    }

    #[test]
    fn page_steps_skip_the_span_of_each_view() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        for (view, page) in [(View::Day, 1), (View::ThreeDays, 3), (View::Week, 7)] {
            choose(&mut handler, view);
            press(&mut handler, Target::Step(5).to_dom());
            assert_eq!(diff(today(), handler.base()) / DAY, page, "{view:?}");
            press(&mut handler, Target::Step(1).to_dom());
            assert_eq!(handler.base(), today(), "{view:?}");
        }
    }

    #[test]
    fn month_steps_scroll_a_week_or_a_page() {
        let mut handler = month_fixture(vec![]);
        let commands = press(&mut handler, Target::Step(4).to_dom());
        assert_eq!(text_of(&commands, &Target::BandLabel(1).to_dom()).unwrap(), "5");
        let commands = press(&mut handler, Target::Step(5).to_dom());
        assert_eq!(text_of(&commands, &Target::BandLabel(1).to_dom()).unwrap(), "9");
        let commands = press(&mut handler, Target::Step(1).to_dom());
        assert_eq!(text_of(&commands, &Target::BandLabel(1).to_dom()).unwrap(), "5");
        choose(&mut handler, View::Week);
        let commands = press(&mut handler, Target::Step(5).to_dom());
        assert_eq!(text_of(&commands, &Target::DayTitle(1).to_dom()).unwrap(), "10/16(金)");
    }

    #[test]
    fn options_follow_the_loaded_statuses_and_resources() {
        let (mut handler, _) = with_store();
        let commands = handler.process_fetched(&sample_response(1, 200)).1;
        assert_eq!(text_of(&commands, &Target::StatusOption(2).to_dom()).unwrap(), "C");
        assert_eq!(text_of(&commands, &Target::ResourceOption(4).to_dom()).unwrap(), "Studio 4");
        assert_eq!(text_of(&commands, &Target::CategoryOption(2).to_dom()).unwrap(), "Follow-up");

        let (handler, _) = drag_fixture();
        let hidden: Vec<Id> = handler
            .option_commands()
            .into_iter()
            .filter_map(|command| match command {
                Command::SetAttribute { id, attribute: Attribute::Hidden, .. } => Some(id),
                _ => None,
            })
            .collect();
        let surplus = (2..=4).map(Target::StatusOption).chain((2..=4).map(Target::CategoryOption));
        assert_eq!(hidden, surplus.map(Target::to_dom).collect::<Vec<_>>());
    }

    #[test]
    fn a_form_with_an_option_outside_the_data_is_rejected() {
        let (mut handler, base) = drag_fixture();
        let date = display(base, Lang::Ja, Format::Date);
        for (query, message) in [
            (form_query("x", &date, 1, "10:00", "11:00").replace("&3=1", "&3=2"), "状態が不正です"),
            (form_query("x", &date, 5, "10:00", "11:00"), "資源が不正です"),
            (form_query("x", &date, 0, "10:00", "11:00"), "資源が不正です"),
        ] {
            tap_empty(&mut handler, 0.0, 6.0);
            let commands = submit(&mut handler, &query);
            assert_eq!(text_of(&commands, &Target::EditMessage.to_dom()).unwrap(), message);
        }
        assert_eq!(appointments(&handler).len(), 2);
    }

    #[test]
    fn random_interaction_keeps_the_store_the_dirty_flag_and_the_data_consistent() {
        let views = [View::Day, View::ThreeDays, View::Week, View::Month];
        for seed in 0..30 {
            let mut rng = Rng::new(seed);
            let (mut handler, disk) = with_store();
            handler.process_fetched(&sample_response(1, 200));
            handler.card_commands();
            let mut saved = fields(&committed_appointments(&disk));
            for step in 0..60 {
                let context = format!("seed {seed} step {step}");
                match rng.below(14) {
                    0 => {
                        choose(&mut handler, views[rng.below(views.len())]);
                    }
                    1 => {
                        press(&mut handler, Target::Step(1 + rng.below(5) as u32).to_dom());
                    }
                    2..5 => {
                        let count = handler.placed.borrow().len();
                        if count > 0 {
                            let (x, y) = grab(&mut handler, 1 + rng.below(count) as u32);
                            let dx = rng.below(801) as f64 - 400.0;
                            let dy = rng.below(801) as f64 - 400.0;
                            drag_to(&mut handler, x + dx, y + dy);
                            if rng.chance(70) {
                                end(&mut handler);
                            } else {
                                handler.process_gesture(&Gesture::DragCancel, &pointer(), None);
                            }
                        }
                    }
                    5 => {
                        tap_empty(&mut handler, rng.below(7) as f64, rng.below(40) as f64);
                    }
                    6 => {
                        let (ox, oy) = grid_origin();
                        let column = column_px(VIEWPORT, handler.view(), REM);
                        let at = |rng: &mut Rng| {
                            (
                                ox + (rng.below(7) as f64 + 0.5) * column,
                                oy + (rng.below(40) as f64 + 0.5) * SLOT_PX,
                            )
                        };
                        let (x, y) = at(&mut rng);
                        handler.process_canvas(
                            &pointer_down(Target::Surface.to_dom(), x, y),
                            &pointer(),
                        );
                        let (x, y) = at(&mut rng);
                        drag_to(&mut handler, x, y);
                        end(&mut handler);
                    }
                    7 => {
                        let day = after(today(), rng.below(21) as i64 - 10);
                        let titles = ["", "Neo", "x"];
                        let query = form_query(
                            titles[rng.below(titles.len())],
                            &display(day, Lang::Ja, Format::Date),
                            1 + rng.below(5) as u32,
                            &format!("{:02}:{:02}", rng.below(24), rng.below(60)),
                            &format!("{:02}:{:02}", rng.below(24), rng.below(60)),
                        );
                        submit(&mut handler, &query);
                    }
                    8 => {
                        press(&mut handler, Target::Save.to_dom());
                        assert!(!handler.dirty(), "{context}: still dirty after save");
                        saved = fields(&appointments(&handler));
                        assert!(
                            fields(&committed_appointments(&disk)) == saved,
                            "{context}: the store differs from what was saved"
                        );
                    }
                    9 => {
                        press(&mut handler, Target::Reload.to_dom());
                        assert!(!handler.dirty(), "{context}: still dirty after reload");
                    }
                    10 => {
                        let value = format!("{}", 1.0 + rng.below(9) as f64 / 4.0);
                        handler.process_canvas(&input(Target::Zoom.to_dom(), &value), &pointer());
                    }
                    _ => {
                        let count = handler.placed.borrow().len();
                        if count > 0 {
                            press_local(
                                &mut handler,
                                1 + rng.below(count) as u32,
                                rng.below(160) as f64,
                                rng.below(120) as f64,
                            );
                            if handler.drag.is_some() {
                                drag_by(
                                    &mut handler,
                                    rng.below(201) as f64 - 100.0,
                                    rng.below(201) as f64 - 100.0,
                                );
                            }
                            end(&mut handler);
                        }
                    }
                }
                if !handler.dirty() {
                    assert!(
                        fields(&appointments(&handler)) == saved,
                        "{context}: clean but different from the last save"
                    );
                }
            }
            assert!(
                fields(&committed_appointments(&disk)) == saved,
                "seed {seed}: the store changed without a save"
            );
        }
    }

    fn delete_hidden(commands: &[Command]) -> Option<bool> {
        commands.iter().find_map(|command| match command {
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. }
                if *id == Target::Delete.to_dom() =>
            {
                Some(true)
            }
            Command::RemoveAttribute { id, attribute: Attribute::Hidden }
                if *id == Target::Delete.to_dom() =>
            {
                Some(false)
            }
            _ => None,
        })
    }

    fn open_first_card(handler: &mut Handler) -> (u32, Vec<Command>) {
        handler.card_commands();
        let (key, base) = {
            let placed = handler.placed.borrow();
            (placed[0].key, placed[0].base)
        };
        let (ox, oy) = grid_origin();
        handler.process_canvas(
            &pointer_down(
                Target::CardPart(1, CardPart::Title).to_dom(),
                ox + base[0] + 40.0,
                oy + base[1] + 20.0,
            ),
            &pointer(),
        );
        let commands = handler.process_gesture(&Gesture::Tap, &pointer(), None).1;
        (key, commands)
    }

    #[test]
    fn the_delete_button_belongs_to_the_form_of_an_existing_appointment() {
        let (mut handler, _) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (_, commands) = open_first_card(&mut handler);
        assert_eq!(delete_hidden(&commands), Some(false));
        handler.process_canvas(
            &CanvasEvent {
                event_type: EventType::Close,
                ..click(Target::Modal.to_dom(), 0.0, 0.0)
            },
            &pointer(),
        );
        let commands = tap_empty(&mut handler, 0.0, 0.0);
        assert_eq!(delete_hidden(&commands), Some(true));
        assert!(press(&mut handler, Target::Delete.to_dom()).is_empty());
        assert!(handler.editing.is_some(), "a new form has nothing to delete");
        assert!(!handler.dirty());
    }

    #[test]
    fn a_deletion_waits_in_memory_for_the_save_button_like_an_edit() {
        let (mut handler, disk) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let before = appointments(&handler).len();
        let (key, _) = open_first_card(&mut handler);
        let commands = press(&mut handler, Target::Delete.to_dom());
        assert!(commands.iter().any(|c| matches!(c, Command::CloseModal { .. })));
        assert_eq!(save_disabled(&commands), [false]);
        assert!(handler.editing.is_none());
        assert!(handler.dirty());
        assert_eq!(appointments(&handler).len(), before - 1);
        assert!(handler.appointment(key).is_none());
        assert_eq!(pending(&handler), 1);
        assert!(
            data::entry(&MemoryStore::new(disk.clone()).unwrap(), key, appointment::KIND).is_some()
        );

        press(&mut handler, Target::Save.to_dom());
        assert!(!handler.dirty());
        assert_eq!(pending(&handler), 0);
        assert!(
            data::entry(&MemoryStore::new(disk.clone()).unwrap(), key, appointment::KIND).is_none()
        );
        let mut reloaded = Handler::new(VIEWPORT, today(), REM);
        reloaded.attach(Backend::new(disk).unwrap());
        assert_eq!(appointments(&reloaded).len(), before - 1);
    }

    #[test]
    fn reload_brings_a_deleted_appointment_back() {
        let (mut handler, disk) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let before = appointments(&handler).len();
        let (key, _) = open_first_card(&mut handler);
        press(&mut handler, Target::Delete.to_dom());
        let commands = press(&mut handler, Target::Reload.to_dom());
        assert!(handler.appointment(key).is_some());
        assert_eq!(appointments(&handler).len(), before);
        assert!(!handler.dirty());
        assert_eq!(save_disabled(&commands), [true]);
        assert!(data::entry(&MemoryStore::new(disk).unwrap(), key, appointment::KIND).is_some());
    }

    #[test]
    fn a_deleted_card_leaves_the_surface_and_the_others_stay() {
        let (mut handler, _) = drag_fixture();
        let shown = handler.placed.borrow().len();
        assert_eq!(shown, 2);
        grab(&mut handler, 1);
        handler.process_gesture(&Gesture::Tap, &pointer(), None);
        let commands = press(&mut handler, Target::Delete.to_dom());
        assert_eq!(shown_count(&commands), 1);
        assert_eq!(
            text_of(&commands, &Target::CardPart(1, CardPart::Title).to_dom()).unwrap(),
            "t2"
        );
        assert_eq!(handler.placed.borrow().len(), 1);
    }

    #[test]
    fn deleting_a_resource_leaves_its_appointments_as_they_were_and_they_read_as_unassigned() {
        let (mut handler, _) = drag_fixture();
        let stored =
            |handler: &Handler, key| handler.store.as_ref().unwrap().get(key).map(<[u8]>::to_vec);
        let (appointment_1, shifts_before) = (stored(&handler, 1), handler.shifts().len());
        handler.store.as_mut().unwrap().delete(101);

        assert_eq!(stored(&handler, 1), appointment_1, "the referrer is not rewritten");
        assert_eq!(appointments(&handler).len(), 2);
        assert_eq!(handler.shifts().len(), shifts_before - 14);
        assert_eq!(shown_count(&handler.card_commands()), 1);

        choose(&mut handler, View::Month);
        assert_eq!(shown_count(&handler.card_commands()), 2, "the month view shows the days only");

        handler.edit_commands(1, 0);
        let commands = handler.edit_commands(1, 0);
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Resource).to_dom()),
            Some(String::new())
        );
        let query =
            form_query("t1", &display(today(), Lang::Ja, Format::Date), 2, "10:00", "11:00");
        let commands = submit(&mut handler, &query);
        assert!(commands.iter().any(|c| matches!(c, Command::CloseModal { .. })));
        let cells = appointment::Places::read(&appointment_of(&handler, 1));
        assert_eq!(cells, [Place { day: today(), resource: 103 }]);
    }

    #[test]
    fn a_deleted_status_or_category_reads_as_none() {
        let (mut handler, _) = drag_fixture();
        let stored = handler.store.as_ref().unwrap().get(1).map(<[u8]>::to_vec);
        handler.store.as_mut().unwrap().delete(STATUS_KEY);
        handler.store.as_mut().unwrap().delete(CATEGORY_KEY);
        let commands = handler.card_commands();
        assert_eq!(
            text_of(&commands, &Target::CardPart(1, CardPart::Status).to_dom()).unwrap(),
            ""
        );
        assert_eq!(
            text_of(&commands, &Target::CardPart(1, CardPart::Category).to_dom()).unwrap(),
            ""
        );
        assert_eq!(handler.store.as_ref().unwrap().get(1).map(<[u8]>::to_vec), stored);
        let commands = handler.edit_commands(1, 0);
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Status).to_dom()),
            Some(String::new())
        );
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Category).to_dom()),
            Some(String::new())
        );
    }
}
