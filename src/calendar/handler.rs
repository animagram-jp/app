use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec, vec::Vec};
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
            Appointment, Calendar, DataError, Place, Record, format_hhmm, parse_date, parse_time,
        },
        grid::{Cell, ColumnAxis, Grid, MonthAxis, TimeAxis},
        layout::lanes,
        store::{self, Store},
        target::{CardPart, EditField, Target},
    },
    event::{Event, Response},
    file_store::FileStoreError,
    js_client::{
        Attribute, CanvasEvent, Command, EventType, Gesture, Keyword, Method, PointerState,
        StyleProperty, StyleValue, Unit, VisibilityState, dom::Id, from_url_search_params,
    },
    timestamp::{
        Format, Timezone, add_days, diff, display, from_ut, pack, sub_days, unpack, youbi,
    },
};

const RESOURCE_COUNT: u32 = 4;
const DAY_MAX: u32 = 7;
const SLOT_COUNT: u32 = 44;
const SLOT_MINUTES: u32 = 15;
const DAY_START_MINUTES: u32 = 9 * 60;
const TIME_AXIS: TimeAxis = TimeAxis::new(DAY_START_MINUTES, SLOT_MINUTES, SLOT_COUNT);
const CARD_POOL: usize = 380;
const DRAG_Z_INDEX: i32 = 1000;
const HANDLE_REM: f64 = 0.5;
const HANDLE_MAX: f64 = 0.35;
const EPSILON: f64 = 1e-9;
#[cfg(target_arch = "wasm32")]
const STORE_NAME: &str = "calendar";
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
const MONTH_WEEKS: u32 = 5;
const DAY_ROWS: u32 = 3;
const MONTH_ROW_REM: f64 = 3.0;
const MONTH_AXIS: MonthAxis = MonthAxis::new(MONTH_WEEKS, DAY_ROWS);
const BAND_POOL: usize = MONTH_AXIS.days() as usize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
    Day,
    ThreeDays,
    Week,
    Month,
}

impl View {
    pub fn days(self) -> u32 {
        match self {
            Self::Day => 1,
            Self::ThreeDays => 3,
            Self::Week | Self::Month => MonthAxis::WEEKDAYS,
        }
    }

    fn columns(self) -> u32 {
        match self {
            Self::Month => MonthAxis::WEEKDAYS,
            _ => ColumnAxis::new(self.days(), RESOURCE_COUNT).count(),
        }
    }
}

type Corner = [Option<bool>; 2];

enum DragKind {
    Move,
    Resize { corner: Corner, bx: BBox<2>, edge_offset: [f64; 2] },
}

struct Drag {
    n:       u32,
    index:   usize,
    cell:    usize,
    offset:  [f64; 2],
    pointer: [f64; 2],
    moved:   bool,
    kind:    DragKind,
}

#[derive(Clone, Copy)]
struct Placed {
    index: usize,
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
    Existing(usize),
    New(Vec<Place>),
}

struct CreateDrag {
    anchor:  [i32; 2],
    current: [i32; 2],
    moved:   bool,
}

struct Card {
    index: usize,
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
    calendar:          Option<Calendar>,
    placed:            RefCell<Vec<Placed>>,
    bands:             RefCell<Vec<Band>>,
    editing:           Option<Editing>,
    pending_tap:       Option<[f64; 2]>,
    create:            Option<CreateDrag>,
    slot_rem:          f64,
    drag:              Option<Drag>,
    dirty:             bool,
    store:             Option<Box<dyn Store>>,
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
        match crate::file_store::FileStore::new(STORE_NAME).await {
            Ok(opened) => handler.attach(Box::new(opened)),
            Err(error) => handler.startup.push(Error::FileStore(error)),
        }
        handler
    }

    pub fn attach(&mut self, store: Box<dyn Store>) {
        match store::load(store.as_ref()) {
            Ok(Some(calendar)) => self.calendar = Some(calendar),
            Ok(None) => {}
            Err(error) => self.startup.push(Error::Data(error)),
        }
        self.store = Some(store);
    }

    pub fn new(viewport_width_px: f64, today: u64, rem_in_px: f64) -> Self {
        let view = View::Week;
        let column_px = column_px(viewport_width_px, view, rem_in_px);
        Self {
            view,
            now: 0,
            today,
            base: today,
            calendar: None,
            placed: RefCell::new(Vec::new()),
            bands: RefCell::new(Vec::new()),
            editing: None,
            pending_tap: None,
            create: None,
            slot_rem: SLOT_REM,
            drag: None,
            dirty: false,
            store: None,
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
        vec![]
    }

    pub fn view(&self) -> View {
        self.view
    }

    pub fn initial_draw(&mut self) -> (Vec<Event>, Vec<Command>) {
        let mut commands = self.view_commands();
        let failed = !self.startup.is_empty();
        commands.extend(self.startup.drain(..).map(|error| Command::Error { error }));
        if self.calendar.is_some() {
            commands.extend(self.loaded_commands());
        } else if !failed {
            commands.push(Command::Fetch {
                request: LOAD_REQUEST,
                method:  Method::Get,
                path:    String::from(LOAD_PATH),
                body:    Vec::new(),
            });
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
        match Calendar::decode(&response.body) {
            Ok(calendar) => {
                let mut commands = self.seed_commands(&calendar);
                self.calendar = Some(calendar);
                commands.extend(self.loaded_commands());
                (vec![], commands)
            }
            Err(error) => (vec![], vec![data_error(error)]),
        }
    }

    pub fn calendar(&self) -> Option<&Calendar> {
        self.calendar.as_ref()
    }

    pub fn process_canvas(
        &mut self,
        event: &CanvasEvent,
        _state: &PointerState,
    ) -> (Vec<Event>, Vec<Command>) {
        let target = Target::from_dom(&event.id);
        match event.event_type {
            EventType::Click => {
                (vec![], target.map_or(vec![], |target| self.click_control(target)))
            }
            EventType::Input if target == Some(Target::Zoom) => (vec![], self.zoom(&event.value)),
            EventType::Change => match target {
                Some(Target::ViewRadio(view)) => (vec![], self.change_view(view)),
                _ => (vec![], vec![]),
            },
            EventType::PointerDown => (vec![], self.press(event, target)),
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
        _state: &PointerState,
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
            index: placed.index,
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
        let bx = self.appointment_box(placed.index)?;
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

    fn appointment_box(&self, index: usize) -> Option<BBox<2>> {
        let calendar = self.calendar.as_ref()?;
        let appointment = calendar.appointments.get(index)?;
        let grid = self.grid();
        let (start, end) = grid.time.clip(appointment.start(), appointment.end())?;
        let flats = appointment.cells().into_iter().filter_map(|place| {
            let resource = calendar.resources.iter().position(|r| r.id() == place.resource)?;
            Some(grid.columns.flat((diff(self.base, place.day) / DAY) as i32, resource as u32))
        });
        let (first, last) = flats.fold(None, |range: Option<(i32, i32)>, flat| match range {
            Some((low, high)) => Some((low.min(flat), high.max(flat))),
            None => Some((flat, flat)),
        })?;
        let (from, to) = (grid.time.unit(start), grid.time.unit(end));
        Some(BBox::new(
            [GridUnit::new(first as f64), GridUnit::new(from)],
            [GridUnit::new((last - first + 1) as f64), GridUnit::new(to - from)],
        ))
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
            pointer[d] = Px::new(origin + local.clamp(0.0, extent[d]));
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
            DragKind::Move => match self.drop_target(&drag) {
                Some((cell, start)) => self.move_appointment(drag.index, drag.cell, cell, start),
                None => false,
            },
            DragKind::Resize { corner, .. } => match self.resized_box(&drag) {
                Some(resized) => self.resize_appointment(drag.index, *corner, &resized),
                None => false,
            },
        };
        if changed {
            commands.extend(self.persist(drag.index));
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
        commands.extend(self.edit_commands(drag.index, drag.cell));
        commands
    }

    fn form_commands(
        &mut self,
        editing: Editing,
        heading: String,
        draft: &Appointment,
        place: Place,
        hint: &str,
    ) -> Vec<Command> {
        let Some(calendar) = self.calendar.as_ref() else {
            return vec![];
        };
        let values = [
            (EditField::Title, String::from(draft.title())),
            (EditField::Category, option_number(draft.category(), calendar.categories.len())),
            (EditField::Status, option_number(draft.status(), calendar.statuses.len())),
            (EditField::Date, display(place.day, LANG, Format::Date)),
            (EditField::Resource, option_value(&calendar.resources, |r| r.id() == place.resource)),
            (EditField::Start, format_hhmm(draft.start())),
            (EditField::End, format_hhmm(draft.end())),
            (EditField::Note, String::from(draft.note())),
        ];
        let mut commands = vec![
            Command::SetText { id: Target::EditHeading.to_dom(), value: heading },
            Command::SetText { id: Target::EditMessage.to_dom(), value: String::from(hint) },
        ];
        commands.extend(values.into_iter().map(|(field, value)| Command::SetValue {
            id: Target::EditField(field).to_dom(),
            value,
        }));
        commands.push(Command::ShowModal { id: Target::Modal.to_dom() });
        self.editing = Some(editing);
        commands
    }

    fn zoom(&mut self, value: &str) -> Vec<Command> {
        let Ok(slot_rem) = value.parse::<f64>() else {
            return vec![];
        };
        let slot_rem = slot_rem.clamp(ZOOM_MIN, ZOOM_MAX);
        if slot_rem == self.slot_rem || self.month() {
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
        let Some(calendar) = self.calendar.as_ref() else {
            return commands;
        };
        let cells: Option<Vec<Place>> = resolved
            .cells
            .iter()
            .map(|cell| {
                let resource = calendar.resources.get(cell.resource as usize)?;
                Some(Place {
                    day:      add_days(self.base, i64::from(cell.day)),
                    resource: resource.id(),
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
        let draft = Appointment::new(0, &[place], start, end, "", 0, 0, "");
        self.form_commands(Editing::New(cells), String::from("新規"), &draft, place, hint)
    }

    fn create_commands(&mut self) -> Vec<Command> {
        let Some(pointer) = self.pending_tap.take() else {
            return vec![];
        };
        let Some(calendar) = self.calendar.as_ref() else {
            return vec![];
        };
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
            let Some(resource) = calendar.resources.first() else {
                return vec![];
            };
            let open = hours(calendar).map_or(DAY_START_MINUTES, |hours| hours.open);
            let start = TIME_AXIS.clamp_start(open, NEW_MINUTES);
            let place = Place {
                day:      add_days(self.first_day(), i64::from(day)),
                resource: resource.id(),
            };
            return self.new_form(vec![place], start, start + NEW_MINUTES, pointer);
        }
        let [Ok(column), Ok(row)] =
            self.rectgrid.point_to_unit([Px::new(pointer[0]), Px::new(pointer[1])])
        else {
            return vec![];
        };
        if column.get() < 0.0 || row.get() < 0.0 || row.get() >= SLOT_COUNT as f64 {
            return vec![];
        }
        let grid = self.grid();
        let Some(cell) = grid.columns.cell(libm::floor(column.get()) as u32) else {
            return vec![];
        };
        let Some(minutes) = grid.time.minutes(libm::floor(row.get()) as u32) else {
            return vec![];
        };
        let Some(resource) = calendar.resources.get(cell.resource as usize) else {
            return vec![];
        };
        let start = grid.time.clamp_start(minutes, NEW_MINUTES);
        let place =
            Place { day: add_days(self.base, i64::from(cell.day)), resource: resource.id() };
        self.new_form(vec![place], start, start + NEW_MINUTES, pointer)
    }

    fn edit_commands(&mut self, index: usize, cell: usize) -> Vec<Command> {
        let Some(calendar) = self.calendar.as_ref() else {
            return vec![];
        };
        let Some(appointment) = calendar.appointments.get(index) else {
            return vec![];
        };
        let cells = appointment.cells();
        let Some(place) = cells.get(cell).or_else(|| cells.first()) else {
            return vec![];
        };
        let (draft, place) = (appointment.clone(), *place);
        let heading = format!("#{}", appointment.id());
        self.form_commands(Editing::Existing(index), heading, &draft, place, "")
    }

    fn reject(message: &str) -> Vec<Command> {
        vec![Command::SetText { id: Target::EditMessage.to_dom(), value: String::from(message) }]
    }

    fn submit(&mut self, value: &str) -> Vec<Command> {
        let Some(editing) = self.editing.clone() else {
            return vec![];
        };
        let draft = match self.calendar.as_ref().map(|calendar| decode_form(value, calendar)) {
            Some(Ok(draft)) => draft,
            Some(Err(message)) => return Self::reject(message),
            None => return Self::reject("データがありません"),
        };
        match self.apply_form(editing, draft) {
            Ok(index) => {
                self.editing = None;
                let mut commands = self.persist(index);
                commands.extend(self.mark_dirty());
                commands.push(Command::CloseModal { id: Target::Modal.to_dom() });
                commands.extend(self.loaded_commands());
                commands
            }
            Err(message) => Self::reject(message),
        }
    }

    fn apply_form(&mut self, editing: Editing, draft: Appointment) -> Result<usize, &'static str> {
        let calendar = self.calendar.as_mut().ok_or("データがありません")?;
        let place = *draft.cells().first().ok_or("入力を読み取れません")?;
        let resource = calendar
            .resources
            .iter()
            .position(|r| r.id() == place.resource)
            .ok_or("資源が不正です")?;
        let (start, end) = (draft.start(), draft.end());
        match editing {
            Editing::New(pending) => {
                if !fits(calendar, start, end) {
                    return Err("営業時間外です");
                }
                let cells = shift_cells(calendar, &pending, place.day, resource)?;
                let id = calendar.appointments.iter().map(|a| a.id()).max().map_or(1, |id| id + 1);
                calendar.appointments.push(Appointment::new(
                    id,
                    &cells,
                    start,
                    end,
                    draft.title(),
                    draft.category(),
                    draft.status(),
                    draft.note(),
                ));
                Ok(calendar.appointments.len() - 1)
            }
            Editing::Existing(index) => {
                let appointment = calendar.appointments.get(index).ok_or("予約がありません")?;
                let current_cells = appointment.cells();
                let cells = shift_cells(calendar, &current_cells, place.day, resource)?;
                let current = &calendar.appointments[index];
                let changed = current.start() != start || current.end() != end;
                if changed && !fits(calendar, start, end) {
                    return Err("営業時間外です");
                }
                let appointment = &mut calendar.appointments[index];
                appointment.set_cells(&cells);
                appointment.set_start(start);
                appointment.set_end(end);
                appointment.set_title(draft.title());
                appointment.set_category(draft.category());
                appointment.set_status(draft.status());
                appointment.set_note(draft.note());
                Ok(index)
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

    fn resize_appointment(&mut self, index: usize, corner: Corner, bx: &BBox<2>) -> bool {
        let base = self.base;
        let grid = self.grid();
        let Some(calendar) = self.calendar.as_mut() else {
            return false;
        };
        let cells = (corner[0].is_some()).then(|| {
            let first = libm::floor(bx.base()[0].get() + EPSILON) as i32;
            let last = libm::ceil(bx.base()[0].get() + bx.offset()[0].get() - EPSILON) as i32;
            (first..last.max(first + 1))
                .map(|flat| {
                    let (day, resource) = grid.columns.locate(flat);
                    let resource = calendar.resources.get(resource as usize)?.id();
                    Some(Place { day: add_days(base, i64::from(day)), resource })
                })
                .collect::<Option<Vec<Place>>>()
        });
        let time = (corner[1].is_some()).then(|| {
            let first = libm::floor(bx.base()[1].get() + EPSILON).max(0.0) as u32;
            let last =
                libm::ceil(bx.base()[1].get() + bx.offset()[1].get() - EPSILON).max(0.0) as u32;
            Some((grid.time.minutes(first)?, grid.time.minutes(last)?))
        });
        let Some(current) = calendar.appointments.get(index) else {
            return false;
        };
        let current_cells = current.cells();
        let new_cells = match cells {
            Some(Some(cells)) => cells,
            _ => current_cells.clone(),
        };
        let (new_start, new_end) = match time {
            Some(Some((start, end))) if end > start => (start, end),
            _ => (current.start(), current.end()),
        };
        let changed =
            new_cells != current_cells || new_start != current.start() || new_end != current.end();
        if changed && !fits(calendar, new_start, new_end) {
            return false;
        }
        let appointment = &mut calendar.appointments[index];
        appointment.set_cells(&new_cells);
        appointment.set_start(new_start);
        appointment.set_end(new_end);
        changed
    }

    fn drop_target(&self, drag: &Drag) -> Option<(Cell, u32)> {
        let calendar = self.calendar.as_ref()?;
        let appointment = calendar.appointments.get(drag.index)?;
        let grid = self.grid();
        let pointer = [Px::new(drag.pointer[0]), Px::new(drag.pointer[1])];
        let [Ok(column), Ok(row)] = self.rectgrid.point_to_unit(pointer) else {
            return None;
        };
        if column.get() < 0.0 || row.get() < 0.0 || row.get() >= SLOT_COUNT as f64 {
            return None;
        }
        let top = [pointer[0], pointer[1] - Px::new(drag.offset[1])];
        let [_, Ok(top)] = self.rectgrid.point_to_unit(top) else {
            return None;
        };
        let slot = (libm::floor(top.get() + 0.5).max(0.0) as u32).min(SLOT_COUNT - 1);
        let duration = appointment.end() - appointment.start();
        let logical = BBox::new(
            [GridUnit::new(libm::floor(column.get())), GridUnit::new(slot as f64)],
            [GridUnit::new(1.0), GridUnit::new(duration as f64 / SLOT_MINUTES as f64)],
        );
        let resolved = grid.resolve(&logical)?;
        let cell = *resolved.cells.first()?;
        Some((cell, grid.time.clamp_start(resolved.start, duration)))
    }

    fn move_in_month(&mut self, drag: &Drag) -> bool {
        let first = self.first_day();
        let Some((target, _)) = self.unit_at(drag.pointer).and_then(|unit| MONTH_AXIS.locate(unit))
        else {
            return false;
        };
        let Some(appointment) =
            self.calendar.as_mut().and_then(|calendar| calendar.appointments.get_mut(drag.index))
        else {
            return false;
        };
        let cells = appointment.cells();
        let Some(pressed) = cells.get(drag.cell) else {
            return false;
        };
        let delta = i64::from(target) - diff(first, pressed.day) / DAY;
        if delta == 0 {
            return false;
        }
        let moved: Vec<Place> = cells
            .iter()
            .map(|place| Place { day: add_days(place.day, delta), resource: place.resource })
            .collect();
        appointment.set_cells(&moved);
        true
    }

    fn move_appointment(&mut self, index: usize, pressed: usize, target: Cell, start: u32) -> bool {
        let base = self.base;
        let axis = self.grid().columns;
        let Some(calendar) = self.calendar.as_mut() else {
            return false;
        };
        let Some(appointment) = calendar.appointments.get(index) else {
            return false;
        };
        let flat =
            |place: &Place| {
                calendar.resources.iter().position(|r| r.id() == place.resource).map(|position| {
                    axis.flat((diff(base, place.day) / DAY) as i32, position as u32)
                })
            };
        let Some(from) = appointment.cells().get(pressed).and_then(flat) else {
            return false;
        };
        let delta = axis.flat(target.day as i32, target.resource) - from;
        let moved: Option<Vec<Place>> = appointment
            .cells()
            .iter()
            .map(|place| {
                let (day, resource) = axis.locate(flat(place)? + delta);
                let resource = calendar.resources.get(resource as usize)?.id();
                Some(Place { day: add_days(base, i64::from(day)), resource })
            })
            .collect();
        let Some(moved) = moved else {
            return false;
        };
        let duration = appointment.end() - appointment.start();
        let changed = appointment.cells() != moved || appointment.start() != start;
        if changed && !fits(calendar, start, start + duration) {
            return false;
        }
        let Some(appointment) = calendar.appointments.get_mut(index) else {
            return false;
        };
        appointment.set_cells(&moved);
        appointment.set_start(start);
        appointment.set_end(start + duration);
        changed
    }

    pub fn cell_at(&mut self, root_origin: (f64, f64), x: f64, y: f64) -> Option<(u32, u32, u32)> {
        self.set_origin(root_origin);
        let grid = self.grid();
        let [Ok(column), Ok(slot)] = self.rectgrid.point_to_unit([Px::new(x), Px::new(y)]) else {
            return None;
        };
        let (column, slot) = (libm::floor(column.get()), libm::floor(slot.get()));
        if column < 0.0 || slot < 0.0 || slot >= SLOT_COUNT as f64 {
            return None;
        }
        let cell = grid.columns.cell(column as u32)?;
        Some((cell.day, cell.resource, slot as u32))
    }

    pub fn base(&self) -> u64 {
        self.base
    }

    fn click_control(&mut self, target: Target) -> Vec<Command> {
        match target {
            Target::Step(n) if (1..=STEP_COUNT).contains(&n) => {
                self.base = if n == STEP_TODAY {
                    self.today
                } else {
                    add_days(self.base, self.step_days(n))
                };
                self.date_commands()
            }
            Target::Modal => {
                self.editing = None;
                vec![Command::CloseModal { id: Target::Modal.to_dom() }]
            }
            Target::Save => self.save_commands(),
            Target::Reload => self.discard_commands(),
            _ => vec![],
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
        let value = match self.calendar.as_ref().and_then(|calendar| calendar.resources.get(index))
        {
            Some(resource) => String::from(resource.name()),
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
        let Some(calendar) = self.calendar.as_ref() else {
            return vec![];
        };
        let mut commands = Vec::new();
        let options = (1..=STATUS_POOL)
            .map(|n| {
                (Target::StatusOption(n), calendar.statuses.get(n as usize - 1).map(|s| s.label()))
            })
            .chain((1..=CATEGORY_POOL).map(|n| {
                (
                    Target::CategoryOption(n),
                    calendar.categories.get(n as usize - 1).map(|c| c.label()),
                )
            }))
            .chain((1..=RESOURCE_COUNT).map(|n| {
                (
                    Target::ResourceOption(n),
                    calendar.resources.get(n as usize - 1).map(|r| r.name()),
                )
            }));
        for (target, text) in options {
            commands.push(Command::SetText {
                id:    target.to_dom(),
                value: String::from(text.unwrap_or_default()),
            });
            commands.push(hidden(target.to_dom(), text.is_none()));
        }
        commands
    }

    fn visible_cards(&self, calendar: &Calendar) -> Vec<Card> {
        let grid = self.grid();
        let mut columns: BTreeMap<u32, Vec<(usize, usize, u32, u32)>> = BTreeMap::new();
        let mut extent: BTreeMap<usize, (i32, i32)> = BTreeMap::new();
        for (index, appointment) in calendar.appointments.iter().enumerate() {
            let Some((start, end)) = grid.time.clip(appointment.start(), appointment.end()) else {
                continue;
            };
            for (cell_index, place) in appointment.cells().iter().enumerate() {
                let Some(resource) =
                    calendar.resources.iter().position(|r| r.id() == place.resource)
                else {
                    continue;
                };
                let offset = diff(self.base, place.day) / DAY;
                let flat = grid.columns.flat(offset as i32, resource as u32);
                let range = extent.entry(index).or_insert((flat, flat));
                *range = (range.0.min(flat), range.1.max(flat));
                let Ok(day) = u32::try_from(offset) else {
                    continue;
                };
                let Some(unit) = grid.columns.unit(Cell { day, resource: resource as u32 }) else {
                    continue;
                };
                columns.entry(unit).or_default().push((index, cell_index, start, end));
            }
        }
        let mut cards = Vec::new();
        for (unit, items) in columns {
            let Some(cell) = grid.columns.cell(unit) else {
                continue;
            };
            let spans: Vec<(u32, u32)> =
                items.iter().map(|(_, _, start, end)| (*start, *end)).collect();
            for ((index, cell_index, start, end), (lane, lanes)) in
                items.into_iter().zip(lanes(&spans))
            {
                let Some(logical) = grid.bbox(cell, start, end) else {
                    continue;
                };
                let (first, last) = extent[&index];
                cards.push(Card {
                    index,
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

    fn month_cards(&self, calendar: &Calendar) -> Vec<Card> {
        let first = self.first_day();
        let mut days: BTreeMap<u32, Vec<(u32, usize, usize)>> = BTreeMap::new();
        for (index, appointment) in calendar.appointments.iter().enumerate() {
            for (cell, place) in appointment.cells().iter().enumerate() {
                let Ok(day) = u32::try_from(diff(first, place.day) / DAY) else {
                    continue;
                };
                if day >= MONTH_AXIS.days() {
                    continue;
                }
                let items = days.entry(day).or_default();
                if !items.iter().any(|(_, other, _)| *other == index) {
                    items.push((appointment.id(), index, cell));
                }
            }
        }
        let mut cards = Vec::new();
        for (day, mut items) in days {
            items.sort_unstable_by_key(|(id, ..)| *id);
            for (row, (_, index, cell)) in (0..MONTH_AXIS.rows()).zip(items) {
                let Some(bx) = MONTH_AXIS.bbox(day, Some(row)) else {
                    continue;
                };
                cards.push(Card { index, cell, bx, lane: 0, lanes: 1, outer: [false; 2] });
            }
        }
        cards.truncate(CARD_POOL);
        cards
    }

    fn band_list(&self, calendar: &Calendar) -> Vec<Band> {
        if self.month() {
            let first = self.first_day();
            return (0..MONTH_AXIS.days())
                .filter_map(|day| {
                    let bx = MONTH_AXIS.bbox(day, None)?;
                    Some(Band { kind: BandKind::Date(add_days(first, i64::from(day))), bx })
                })
                .collect();
        }
        let Some(hours) = hours(calendar) else {
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
        let mut commands = Vec::new();
        for n in 1..=heading_axis().count() {
            let person =
                heading_axis().cell(n - 1).filter(|cell| cell.day < days).and_then(|cell| {
                    let calendar = self.calendar.as_ref()?;
                    let resource = calendar.resources.get(cell.resource as usize)?;
                    calendar
                        .shifts
                        .iter()
                        .find(|s| {
                            s.day() == add_days(self.base, i64::from(cell.day))
                                && s.resource() == resource.id()
                        })
                        .map(|s| String::from(s.person()))
                });
            commands.push(Command::SetText {
                id:    Target::ResourcePerson(n).to_dom(),
                value: person.unwrap_or_default(),
            });
        }
        commands
    }

    fn band_commands(&self) -> Vec<Command> {
        let Some(calendar) = self.calendar.as_ref() else {
            return vec![];
        };
        let bands = self.band_list(calendar);
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
        let Some(calendar) = self.calendar.as_ref() else {
            return vec![];
        };
        let cards =
            if self.month() { self.month_cards(calendar) } else { self.visible_cards(calendar) };
        let boxes: Vec<BBox<2>> = cards.iter().map(|card| card.bx).collect();
        let resolved = self.rectgrid.box_as_px(&boxes);
        let mut commands = Vec::new();
        let mut placed: Vec<Placed> = Vec::new();
        for ((card, px), n) in cards.iter().zip(resolved).zip(1u32..) {
            let [Ok((x, width)), Ok((y, height))] = px else {
                continue;
            };
            placed.push(Placed {
                index: card.index,
                cell:  card.cell,
                base:  [x.get(), y.get()],
                bx:    card.bx,
                lane:  card.lane,
                lanes: card.lanes,
                outer: card.outer,
            });
            let appointment = &calendar.appointments[card.index];
            let status =
                calendar.statuses.get(appointment.status() as usize).map_or("", |s| s.label());
            let category =
                calendar.categories.get(appointment.category() as usize).map_or("", |c| c.label());
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
                (Target::CardPart(n, CardPart::Status).to_dom(), String::from(status)),
                (
                    Target::CardPart(n, CardPart::Time).to_dom(),
                    format!(
                        "{:02}:{:02}–{:02}:{:02}",
                        appointment.start() / 60,
                        appointment.start() % 60,
                        appointment.end() / 60,
                        appointment.end() % 60
                    ),
                ),
                (Target::CardPart(n, CardPart::Title).to_dom(), String::from(appointment.title())),
                (Target::CardPart(n, CardPart::Category).to_dom(), String::from(category)),
                (Target::CardPart(n, CardPart::Note).to_dom(), String::from(appointment.note())),
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

    fn seed_commands(&mut self, calendar: &Calendar) -> Vec<Command> {
        let Some(store) = self.store.as_mut() else {
            return vec![];
        };
        if let Err(error) = store::seed(store.as_mut(), calendar) {
            return vec![data_error(error)];
        }
        match store.save() {
            Ok(()) => vec![],
            Err(error) => vec![file_store_error(error)],
        }
    }

    fn persist(&mut self, index: usize) -> Vec<Command> {
        let now = self.now;
        let (Some(store), Some(calendar)) = (self.store.as_mut(), self.calendar.as_mut()) else {
            return vec![];
        };
        let Some(appointment) = calendar.appointments.get_mut(index) else {
            return vec![];
        };
        appointment.touch(now);
        match store::put(store.as_mut(), appointment) {
            Ok(()) => vec![],
            Err(error) => vec![data_error(error)],
        }
    }

    fn save_commands(&mut self) -> Vec<Command> {
        let Some(store) = self.store.as_mut() else {
            return vec![];
        };
        if let Err(error) = store.save() {
            return vec![file_store_error(error)];
        }
        self.dirty = false;
        vec![Command::SetAttribute {
            id:        Target::Save.to_dom(),
            attribute: Attribute::Disabled,
            value:     String::new(),
        }]
    }

    pub fn discard_commands(&mut self) -> Vec<Command> {
        let Some(store) = self.store.as_mut() else {
            return vec![];
        };
        if let Err(error) = store.discard() {
            return vec![file_store_error(error)];
        }
        match store::load(store.as_ref()) {
            Ok(Some(calendar)) => self.calendar = Some(calendar),
            Ok(None) => return vec![],
            Err(error) => return vec![data_error(error)],
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
        commands
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
        Grid::new(ColumnAxis::new(self.view.days(), RESOURCE_COUNT), TIME_AXIS)
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

    fn row_rem(&self) -> f64 {
        if self.month() { MONTH_ROW_REM } else { self.slot_rem }
    }

    fn fit_rectgrid(&mut self) {
        let _ = self.rectgrid.set_definition(
            IncrementFunction::Scale(column_px(self.viewport_width_px, self.view, self.rem_in_px)),
            0,
        );
        let _ = self
            .rectgrid
            .set_definition(IncrementFunction::Scale(self.row_rem() * self.rem_in_px), 1);
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
        let rem = self.row_rem();
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
        commands.push(toggle(Target::Zoom.to_dom(), Attribute::Disabled, self.month()));
        if !self.month() {
            commands.extend(self.axis_commands());
        }
        commands.extend(self.row_commands());
        commands
    }
}

fn shift_cells(
    calendar: &Calendar,
    cells: &[Place],
    day: u64,
    resource: usize,
) -> Result<Vec<Place>, &'static str> {
    let first = cells.first().ok_or("予約がありません")?;
    let position = |place: &Place| {
        calendar.resources.iter().position(|r| r.id() == place.resource).ok_or("資源が不正です")
    };
    let day_delta = diff(first.day, day) / DAY;
    let resource_delta = resource as i32 - position(first)? as i32;
    cells
        .iter()
        .map(|place| {
            let index = usize::try_from(position(place)? as i32 + resource_delta)
                .map_err(|_| "範囲外です")?;
            let moved = calendar.resources.get(index).ok_or("範囲外です")?;
            Ok(Place { day: add_days(place.day, day_delta), resource: moved.id() })
        })
        .collect()
}

fn decode_form(value: &str, calendar: &Calendar) -> Result<Appointment, &'static str> {
    let pairs = from_url_search_params(value);
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
    let resource = option_position(&calendar.resources, resource).ok_or("資源が不正です")?;
    let status = option_position(&calendar.statuses, status).ok_or("状態が不正です")?;
    let category = option_position(&calendar.categories, category).ok_or("カテゴリが不正です")?;
    let place = Place { day, resource: calendar.resources[resource].id() };
    Ok(Appointment::new(0, &[place], start, end, title, category as u32, status as u32, note))
}

fn option_number(index: u32, count: usize) -> String {
    if (index as usize) < count { format!("{}", index + 1) } else { String::new() }
}

fn option_value<T>(items: &[T], selected: impl Fn(&T) -> bool) -> String {
    items.iter().position(selected).map_or_else(String::new, |index| format!("{}", index + 1))
}

fn option_position<T>(items: &[T], value: &str) -> Option<usize> {
    let index = value.parse::<usize>().ok()?.checked_sub(1)?;
    (index < items.len()).then_some(index)
}

fn hours(calendar: &Calendar) -> Option<Hours> {
    let first = calendar.shifts.first()?;
    let mut hours = Hours { open: first.open(), close: first.close(), rest: first.break_range() };
    for shift in &calendar.shifts[1..] {
        hours.open = hours.open.min(shift.open());
        hours.close = hours.close.max(shift.close());
        hours.rest = match (hours.rest, shift.break_range()) {
            (Some((a, b)), Some((c, d))) if a.max(c) < b.min(d) => Some((a.max(c), b.min(d))),
            _ => None,
        };
    }
    Some(hours)
}

fn fits(calendar: &Calendar, start: u32, end: u32) -> bool {
    hours(calendar).is_none_or(|hours| hours.open <= start && end <= hours.close)
}

fn column_px(viewport_width_px: f64, view: View, rem_in_px: f64) -> f64 {
    let columns = view.columns() as f64;
    let visible = viewport_width_px - 2.0 * AXIS_REM * rem_in_px;
    (visible / columns).max(COLUMN_MIN_REM * rem_in_px)
}

fn heading_axis() -> ColumnAxis {
    ColumnAxis::new(DAY_MAX, RESOURCE_COUNT)
}

fn lane_box(logical: &BBox<2>, lane: u32, count: u32) -> BBox<2> {
    let width = logical.offset()[0].get() / count as f64;
    BBox::new(
        [GridUnit::new(logical.base()[0].get() + lane as f64 * width), logical.base()[1]],
        [GridUnit::new(width), logical.offset()[1]],
    )
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

fn file_store_error(error: FileStoreError) -> Command {
    Command::Error { error: Error::FileStore(error) }
}

fn data_error(error: DataError) -> Command {
    Command::Error { error: Error::Data(error) }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use core::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };
    use std::fs;

    use super::*;
    use crate::{
        calendar::{data::Record, store::memory::MemoryStore},
        data_struct::ID_MODIFIED_AT,
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

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
        }
    }

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

    fn state() -> PointerState {
        PointerState::default()
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
        handler.process_canvas(&event, &state()).1
    }

    fn press(handler: &mut Handler, id: Id) -> Vec<Command> {
        handler.process_canvas(&click(id, 10.0, 10.0), &state()).1
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
        let body =
            fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/distribution/calendar/data/calendar.json"))
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
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        assert!(handler.calendar().is_none());
        let (_, commands) = handler.process_fetched(&sample_response(1, 200));
        let calendar = handler.calendar().unwrap();
        assert_eq!(calendar.appointments.len(), 380);
        assert_eq!(text_of(&commands, &Target::ResourceName(1).to_dom()).unwrap(), "Studio 1");
        assert_eq!(text_of(&commands, &Target::ResourceName(6).to_dom()).unwrap(), "Studio 2");
        assert_eq!(text_of(&commands, &Target::ResourceName(28).to_dom()).unwrap(), "Studio 4");
    }

    #[test]
    fn view_change_keeps_loaded_resource_names() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        handler.process_fetched(&sample_response(1, 200));
        let commands = choose(&mut handler, View::Day);
        assert_eq!(text_of(&commands, &Target::ResourceName(2).to_dom()).unwrap(), "Studio 2");
    }

    #[test]
    fn fetched_failure_reports_a_data_error_and_keeps_nothing() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
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
        assert!(handler.calendar().is_none());
    }

    #[test]
    fn fetched_for_another_request_is_ignored() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let (_, commands) = handler.process_fetched(&sample_response(9, 200));
        assert!(commands.is_empty());
        assert!(handler.calendar().is_none());
    }

    fn appointment(
        id: u32,
        day: u64,
        resource: u32,
        start: u32,
        end: u32,
    ) -> crate::calendar::data::Appointment {
        crate::calendar::data::Appointment::new(
            id,
            &[crate::calendar::data::Place { day, resource }],
            start,
            end,
            &format!("t{id}"),
            0,
            0,
            "",
        )
    }

    fn calendar_with(appointments: Vec<crate::calendar::data::Appointment>) -> Calendar {
        Calendar {
            meta: crate::calendar::data::Meta::new(true),
            resources: (101..105)
                .map(|id| crate::calendar::data::Resource::new(id, &format!("Studio {}", id - 100)))
                .collect(),
            statuses: vec![crate::calendar::data::Status::new(0, "done", "D")],
            categories: vec![crate::calendar::data::Category::new(0, "c", "C")],
            shifts: (-3..11)
                .flat_map(|day| (101..105).map(move |resource| (day, resource)))
                .enumerate()
                .map(|(index, (day, resource))| {
                    crate::calendar::data::Shift::new(
                        index as u32,
                        after(today(), day),
                        resource,
                        "p",
                        540,
                        1200,
                        None,
                    )
                })
                .collect(),
            appointments,
        }
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
        handler.calendar = Some(calendar_with(vec![
            appointment(1, base, 101, 600, 660),
            appointment(2, base, 101, 630, 690),
            appointment(3, after(base, 10), 101, 600, 660),
            appointment(4, after(base, 1), 102, 480, 570),
            appointment(5, after(base, -1), 101, 600, 660),
            appointment(6, base, 999, 600, 660),
        ]));
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
        handler.calendar = Some(calendar_with(vec![
            appointment(1, base, 101, 600, 660),
            appointment(2, after(base, 1), 101, 600, 660),
            appointment(3, after(base, 2), 101, 600, 660),
        ]));
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
        handler.calendar = Some(calendar_with(vec![appointment(1, base, 103, 540, 600)]));
        let commands = choose(&mut handler, View::Day);
        let column_px = (VIEWPORT - 2.0 * AXIS_PX) / 4.0;
        let (x, y, width, height) = card_box(&commands, 1).unwrap();
        assert_eq!(x, (column_px * 2.0) as f32);
        assert_eq!((y, height), (0.0, 112.0));
        assert_eq!(width, column_px as f32);
    }

    #[test]
    fn sample_week_places_every_visible_appointment() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let (_, commands) = handler.process_fetched(&sample_response(1, 200));
        let calendar = handler.calendar().unwrap();
        let expected = calendar
            .appointments
            .iter()
            .flat_map(|a| a.cells())
            .filter(|cell| (0..7).contains(&(diff(today(), cell.day) / DAY)))
            .count();
        assert!(expected > 0);
        assert_eq!(shown_count(&commands), expected);
    }

    #[test]
    fn resize_replaces_the_cards() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        handler.calendar = Some(calendar_with(vec![appointment(1, base, 101, 600, 660)]));
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
        let library = include_str!("../../distribution/css/library/base.css");
        assert!(library.contains(&format!("--md-box-height:     {MONTH_ROW_REM}rem")));
    }

    #[test]
    fn cards_scale_with_the_root_font_size() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, 20.0);
        handler.calendar = Some(calendar_with(vec![appointment(1, base, 101, 600, 660)]));
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
        handler.calendar = Some(calendar_with(vec![
            appointment(1, base, 101, 600, 660),
            appointment(2, base, 102, 540, 600),
        ]));
        handler.card_commands();
        (handler, base)
    }

    fn grab(handler: &mut Handler, n: u32) -> (f64, f64) {
        let (ox, oy) = grid_origin();
        let (x, y) = (ox + GRAB_X, oy + 4.0 * SLOT_PX + GRAB_Y);
        handler.process_canvas(
            &pointer_down(Target::CardPart(n, CardPart::Title).to_dom(), x, y),
            &state(),
        );
        (x, y)
    }

    fn drag_to(handler: &mut Handler, x: f64, y: f64) -> Vec<Command> {
        handler.process_gesture(&Gesture::Drag { x, y }, &state(), None).1
    }

    fn end(handler: &mut Handler) -> Vec<Command> {
        handler.process_gesture(&Gesture::DragEnd, &state(), None).1
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
            &state(),
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
        let moved = &handler.calendar().unwrap().appointments[0];
        assert_eq!(moved.cells()[0].day, after(base, 2));
        assert_eq!(moved.cells()[0].resource, 103);
        assert_eq!((moved.start(), moved.end()), (540 + 150, 540 + 210));
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
            let moved = &handler.calendar().unwrap().appointments[0];
            assert_eq!(moved.start(), 540 + expected_slot * 15, "extra {extra}");
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
            &state(),
        );
        drag_to(&mut handler, ox + 5.0, oy + 43.2 * SLOT_PX + grab_y);
        end(&mut handler);
        let moved = &handler.calendar().unwrap().appointments[0];
        assert_eq!((moved.start(), moved.end()), (20 * 60 - 60, 20 * 60));
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
            let unchanged = &handler.calendar().unwrap().appointments[0];
            assert_eq!(
                (unchanged.cells()[0].day, unchanged.cells()[0].resource, unchanged.start()),
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
        let commands = handler.process_gesture(&Gesture::DragCancel, &state(), None).1;
        assert!(handler.drag.is_none());
        assert_eq!(translate_of(&commands, 1), Some((0.0, 112.0)));
        let unchanged = &handler.calendar().unwrap().appointments[0];
        assert_eq!((unchanged.cells()[0].day, unchanged.start()), (base, 600));
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
            &state(),
        );
        drag_to(&mut handler, ox + column_px * 3.0 + 5.0, oy + 4.0 * SLOT_PX + GRAB_Y);
        end(&mut handler);
        let moved = &handler.calendar().unwrap().appointments[0];
        assert_eq!((moved.cells()[0].day, moved.cells()[0].resource), (base, 104));
    }

    #[test]
    fn dropping_on_every_unit_of_every_view_resolves_like_the_column_axis() {
        for (view, days) in [(View::Day, 1u32), (View::ThreeDays, 3), (View::Week, 7)] {
            let base = today();
            let mut handler = Handler::new(VIEWPORT, base, REM);
            handler.calendar = Some(calendar_with(vec![appointment(1, base, 101, 600, 660)]));
            choose(&mut handler, view);
            let axis = ColumnAxis::new(days, RESOURCE_COUNT);
            let column_px = column_px(VIEWPORT, handler.view(), REM);
            let (ox, oy) = grid_origin();
            for unit in 0..axis.count() {
                handler.card_commands();
                let (grab_x, grab_y) = (ox + GRAB_X, oy + 4.0 * SLOT_PX + GRAB_Y);
                handler.process_canvas(
                    &pointer_down(Target::CardPart(1, CardPart::Title).to_dom(), grab_x, grab_y),
                    &state(),
                );
                drag_to(&mut handler, ox + unit as f64 * column_px + 3.0, grab_y);
                end(&mut handler);
                let moved = &handler.calendar().unwrap().appointments[0];
                let cell = axis.cell(unit).unwrap();
                assert_eq!(
                    moved.cells()[0].day,
                    after(base, i64::from(cell.day)),
                    "view {days} unit {unit}"
                );
                assert_eq!(
                    moved.cells()[0].resource,
                    101 + cell.resource,
                    "view {days} unit {unit}"
                );
                assert_eq!(moved.start(), 600);
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
        let mut spanning = appointment(1, base, 102, 600, 660);
        let mut cells = spanning.cells();
        cells.push(crate::calendar::data::Place { day: base, resource: 103 });
        spanning.set_cells(&cells);
        handler.calendar = Some(calendar_with(vec![spanning]));
        choose(&mut handler, view);
        handler.card_commands();
        (handler, base)
    }

    fn open_at(handler: &mut Handler, minutes: u32) {
        for shift in &mut handler.calendar.as_mut().unwrap().shifts {
            *shift = crate::calendar::data::Shift::new(
                crate::calendar::data::Record::identity(shift),
                shift.day(),
                shift.resource(),
                shift.person(),
                minutes,
                shift.close(),
                shift.break_range(),
            );
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
            &state(),
        );
        drag_to(handler, ox + to_unit as f64 * column_px + 5.0, y);
        end(handler);
    }

    fn places(handler: &Handler) -> Vec<(u64, u32)> {
        handler.calendar().unwrap().appointments[0]
            .cells()
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
        let mut crossing = appointment(1, base, 104, 600, 660);
        let mut cells = crossing.cells();
        cells.push(crate::calendar::data::Place { day: after(base, 1), resource: 101 });
        crossing.set_cells(&cells);
        handler.calendar = Some(calendar_with(vec![crossing]));
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
        handler.calendar = Some(calendar_with(vec![appointment(1, base, unit_resource, 600, 660)]));
        handler.card_commands();
        handler
    }

    fn press_local(handler: &mut Handler, n: u32, x: f64, y: f64) -> Vec<Command> {
        let (ox, oy) = grid_origin();
        handler
            .process_canvas(
                &pointer_down(Target::CardPart(n, CardPart::Title).to_dom(), ox + x, oy + y),
                &state(),
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
        let appointment = &handler.calendar().unwrap().appointments[0];
        (appointment.start(), appointment.end())
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
        handler.calendar = Some(calendar_with(vec![
            appointment(1, base, 101, 600, 660),
            appointment(2, base, 101, 630, 690),
        ]));
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
            let mut crossing = appointment(1, base, 104, 600, 660);
            let mut cells = crossing.cells();
            cells.push(crate::calendar::data::Place { day: after(base, 1), resource: 101 });
            crossing.set_cells(&cells);
            handler.calendar = Some(calendar_with(vec![crossing]));
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
        let commands = handler.process_gesture(&Gesture::DragCancel, &state(), None).1;
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
        handler.process_gesture(&Gesture::DragCancel, &state(), None);
        assert!(!handler.dirty());
    }

    fn with_store() -> (Handler, MemoryStore) {
        let store = MemoryStore::default();
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        handler.attach(Box::new(store.clone()));
        (handler, store)
    }

    fn drag_first_card_by(handler: &mut Handler, dy: f64) -> (usize, u32) {
        handler.card_commands();
        let (index, base) = {
            let placed = handler.placed.borrow();
            (placed[0].index, placed[0].base)
        };
        let before = handler.calendar().unwrap().appointments[index].start();
        let (ox, oy) = grid_origin();
        handler.process_canvas(
            &pointer_down(
                Target::CardPart(1, CardPart::Title).to_dom(),
                ox + base[0] + 40.0,
                oy + base[1] + 20.0,
            ),
            &state(),
        );
        drag_by(handler, 0.0, dy);
        end(handler);
        (index, before)
    }

    fn committed_calendar(store: &MemoryStore) -> Calendar {
        let reader = MemoryStore::default();
        reader.0.borrow_mut().committed = store.0.borrow().committed.clone();
        crate::calendar::store::load(&reader).unwrap().unwrap()
    }

    #[test]
    fn a_first_run_fetches_seeds_and_saves_the_records() {
        let (mut handler, store) = with_store();
        assert!(handler.calendar().is_none());
        let (_, commands) = handler.initial_draw();
        assert!(commands.iter().any(|command| matches!(command, Command::Fetch { .. })));
        assert_eq!(store.committed_len(), 0);
        handler.process_fetched(&sample_response(1, 200));
        assert_eq!(store.committed_len(), 594);
        assert_eq!(store.pending_len(), 0);
        assert!(!handler.dirty());
    }

    #[test]
    fn a_later_run_loads_from_the_store_without_fetching() {
        let (mut first, store) = with_store();
        first.process_fetched(&sample_response(1, 200));
        let mut second = Handler::new(VIEWPORT, today(), REM);
        second.attach(Box::new(store.clone()));
        assert_eq!(second.calendar().unwrap().appointments.len(), 380);
        let (_, commands) = second.initial_draw();
        assert!(!commands.iter().any(|command| matches!(command, Command::Fetch { .. })));
        assert_eq!(text_of(&commands, &Target::ResourceName(1).to_dom()).unwrap(), "Studio 1");
        assert!(shown_count(&commands) > 0);
    }

    #[test]
    fn an_edit_is_pending_until_save_and_survives_a_reload_only_after_it() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let after = handler.calendar().unwrap().appointments[index].start();
        assert_eq!(after, before + 30);
        assert!(handler.dirty());
        assert_eq!(store.pending_len(), 1);
        assert_eq!(committed_calendar(&store).appointments[index].start(), before);

        press(&mut handler, Target::Save.to_dom());
        assert!(!handler.dirty());
        assert_eq!(store.pending_len(), 0);
        assert_eq!(committed_calendar(&store).appointments[index].start(), after);

        let mut reloaded = Handler::new(VIEWPORT, today(), REM);
        reloaded.attach(Box::new(store.clone()));
        assert_eq!(reloaded.calendar().unwrap().appointments[index].start(), after);
    }

    #[test]
    fn an_unsaved_edit_is_gone_after_a_reload() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        store.0.borrow_mut().pending.clear();
        let mut reloaded = Handler::new(VIEWPORT, today(), REM);
        reloaded.attach(Box::new(store.clone()));
        assert_eq!(reloaded.calendar().unwrap().appointments[index].start(), before);
    }

    #[test]
    fn discard_restores_the_saved_state_and_redraws() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let commands = handler.discard_commands();
        assert_eq!(handler.calendar().unwrap().appointments[index].start(), before);
        assert!(!handler.dirty());
        assert_eq!(store.pending_len(), 0);
        assert_eq!(save_disabled(&commands), [true]);
        assert!(shown_count(&commands) > 0);
    }

    fn band_point(slot: f64) -> [Px; 2] {
        let (ox, oy) = grid_origin();
        [Px::new(ox + 10.0), Px::new(oy + slot * SLOT_PX)]
    }

    #[test]
    fn fetched_shifts_become_three_bands_with_names_and_a_hit_test() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let (_, commands) = handler.process_fetched(&sample_response(1, 200));
        handler.set_origin(ROOT);
        let calendar = handler.calendar().unwrap();
        let shift = calendar.shifts.iter().find(|s| s.day() == today()).unwrap().clone();
        let resource =
            calendar.resources.iter().position(|r| r.id() == shift.resource()).unwrap() as u32;
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
            shift.person()
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
        let mut handler = Handler::new(VIEWPORT, today(), REM);
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
        let (_, commands) = handler.process_gesture(&Gesture::Tap, &state(), None);
        let appointment = handler.calendar().unwrap().appointments[0].clone();
        assert_eq!(
            value_of(&commands, &Target::EditField(EditField::Title).to_dom()).unwrap(),
            appointment.title()
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
        handler.process_canvas(&pointer_down(Target::Surface.to_dom(), x, y), &state());
        handler.process_gesture(&Gesture::Tap, &state(), None).1
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
        handler.process_canvas(&event, &state()).1
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
        handler.process_canvas(&pointer_down(Target::Surface.to_dom(), x, y), &state());
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
        let added = handler.calendar().unwrap().appointments.last().unwrap();
        assert_eq!(added.cells().len(), 4);
        assert_eq!((added.start(), added.end()), (750, 870));
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
            &state(),
        );
        drag_to(&mut handler, ox + 10.0, oy + 3.0 * SLOT_PX);
        let (_, commands) = handler.process_gesture(&Gesture::DragCancel, &state(), None);
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
        let (_, commands) = handler.process_canvas(&input(Target::Zoom.to_dom(), "3"), &state());
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
        assert!(handler.process_canvas(&input(Target::Zoom.to_dom(), "3"), &state()).1.is_empty());
        handler.process_canvas(&input(Target::Zoom.to_dom(), "9"), &state());
        assert_eq!(handler.slot_rem, ZOOM_MAX);
    }

    #[test]
    fn a_drag_after_zooming_still_snaps_to_slots() {
        let (mut handler, _) = drag_fixture();
        handler.process_canvas(&input(Target::Zoom.to_dom(), "2.5"), &state());
        let (ox, oy) = grid_origin();
        let slot = 2.5 * REM;
        handler.process_canvas(
            &pointer_down(Target::Surface.to_dom(), ox + 10.0, oy + 5.0 * slot + 3.0),
            &state(),
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
        handler
            .process_canvas(&pointer_down(Target::Surface.to_dom(), ox + 10.0, oy - 5.0), &state());
        let (_, commands) = handler.process_gesture(&Gesture::Tap, &state(), None);
        assert!(commands.is_empty());
    }

    #[test]
    fn submitting_a_new_form_adds_a_persisted_appointment() {
        let (mut handler, store) = with_store();
        handler.now = pack(2026, 10, 2, 12, 0, 0, 0, 1, 0);
        handler.process_fetched(&sample_response(1, 200));
        let before = handler.calendar().unwrap().appointments.len();
        let shift = handler.calendar().unwrap().shifts[0].clone();
        tap_empty(&mut handler, 0.0, 0.0);
        let date = display(shift.day(), Lang::Ja, Format::Date);
        let (start, end) = (format_hhmm(shift.open()), format_hhmm(shift.open() + 60));
        let query = form_query("Neo", &date, shift.resource() - 100, &start, &end);
        let commands = submit(&mut handler, &query);
        let calendar = handler.calendar().unwrap();
        assert_eq!(calendar.appointments.len(), before + 1);
        let added = calendar.appointments.last().unwrap();
        assert_eq!(
            (added.title(), added.start(), added.end()),
            ("Neo", shift.open(), shift.open() + 60)
        );
        assert_eq!((added.category(), added.status()), (0, 0));
        assert!(handler.dirty());
        assert!(commands.iter().any(|c| matches!(c, Command::CloseModal { .. })));
        assert!(store.pending_len() == 1);
        assert!(handler.editing.is_none());
        let key = added.key().unwrap();
        let saved = Appointment::from_bytes(&store.get(key).unwrap()).unwrap();
        assert_eq!(saved.data().get(ID_MODIFIED_AT).unwrap(), handler.now.to_le_bytes());
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
        assert_eq!(handler.calendar().unwrap().appointments.len(), 2);
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
        assert_eq!(handler.calendar().unwrap().appointments.len(), 2);
        grab(&mut handler, 1);
        handler.process_gesture(&Gesture::Tap, &state(), None);
        let same = form_query("kept", &display(base, Lang::Ja, Format::Date), 1, "10:00", "11:00");
        submit(&mut handler, &same);
        assert_eq!(handler.calendar().unwrap().appointments[0].title(), "kept");
    }

    #[test]
    fn a_drag_before_the_open_hours_is_refused() {
        let (mut handler, _) = drag_fixture();
        open_at(&mut handler, 660);
        let before = handler.calendar().unwrap().appointments[0].clone();
        let (x, y) = grab(&mut handler, 1);
        drag_to(&mut handler, x, y + 2.0 * SLOT_PX);
        end(&mut handler);
        assert_eq!(handler.calendar().unwrap().appointments[0].start(), before.start());
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

    #[test]
    fn reload_button_discards_pending_edits() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let commands = press(&mut handler, Target::Reload.to_dom());
        assert_eq!(handler.calendar().unwrap().appointments[index].start(), before);
        assert_eq!(store.pending_len(), 0);
        assert_eq!(save_disabled(&commands), [true]);
    }

    #[test]
    fn a_failed_save_reports_the_error_and_stays_dirty() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        store.failing(true);
        let commands = press(&mut handler, Target::Save.to_dom());
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::FileStore(FileStoreError::QuotaExceeded(_)) }]
        ));
        assert!(handler.dirty());
        store.failing(false);
        press(&mut handler, Target::Save.to_dom());
        assert!(!handler.dirty());
    }

    #[test]
    fn a_corrupt_store_is_reported_and_nothing_is_fetched_over_it() {
        let store = MemoryStore::default();
        {
            let mut seeding = Handler::new(VIEWPORT, today(), REM);
            seeding.attach(Box::new(store.clone()));
            seeding.process_fetched(&sample_response(1, 200));
        }
        store.0.borrow_mut().committed.insert(
            crate::calendar::data::Record::key(
                &Calendar::decode(
                    &fs::read(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/distribution/calendar/data/calendar.json"
                    ))
                    .unwrap(),
                )
                .unwrap()
                .appointments[0],
            )
            .unwrap(),
            b"{".to_vec(),
        );
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        handler.attach(Box::new(store.clone()));
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
        assert!(handler.discard_commands().is_empty());
    }

    fn month_fixture(appointments: Vec<crate::calendar::data::Appointment>) -> Handler {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        handler.calendar = Some(calendar_with(appointments));
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
            oy + (bx.base()[1].get() + 0.5) * MONTH_ROW_REM * REM,
        )
    }

    fn placed_ids(handler: &Handler) -> Vec<(u32, [f64; 2])> {
        let calendar = handler.calendar().unwrap();
        handler
            .placed
            .borrow()
            .iter()
            .map(|placed| {
                let base = [placed.bx.base()[0].get(), placed.bx.base()[1].get()];
                (calendar.appointments[placed.index].id(), base)
            })
            .collect()
    }

    fn multi(id: u32, places: &[(u64, u32)]) -> crate::calendar::data::Appointment {
        let places: Vec<Place> =
            places.iter().map(|(day, resource)| Place { day: *day, resource: *resource }).collect();
        crate::calendar::data::Appointment::new(id, &places, 600, 660, "m", 0, 0, "")
    }

    #[test]
    fn month_button_lays_out_weeks_inside_the_shared_frame() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        handler.calendar = Some(calendar_with(vec![]));
        let commands = choose(&mut handler, View::Month);
        assert_eq!(handler.view(), View::Month);
        assert_eq!(grid_columns(&commands), week_grid(7, 7));
        let count = MONTH_WEEKS * (DAY_ROWS + 1);
        let axis = format!("var(--head-height) repeat({count}, {MONTH_ROW_REM}rem)");
        assert_eq!(
            grid_rows(&commands),
            [
                (Target::TimeAxis.to_dom(), axis),
                (
                    Target::Surface.to_dom(),
                    format!(
                        "var(--head-title) var(--head-resource) calc({MONTH_ROW_REM}rem * {count})"
                    )
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
        assert!(commands.iter().any(|command| matches!(command,
            Command::SetAttribute { id, attribute: Attribute::Disabled, .. }
                if *id == Target::Zoom.to_dom())));
    }

    #[test]
    fn month_cards_stack_up_to_the_row_count_per_day_in_id_order() {
        let base = today();
        let handler = month_fixture(vec![
            appointment(5, base, 101, 540, 600),
            appointment(2, base, 102, 900, 960),
            appointment(9, base, 103, 600, 660),
            appointment(7, base, 104, 1100, 1160),
            multi(3, &[(after(base, 1), 101), (after(base, 1), 102)]),
            appointment(4, after(base, -10), 101, 600, 660),
            appointment(8, after(base, 35), 101, 600, 660),
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
            &state(),
        );
        let (x, y) = month_point(13, Some(2));
        drag_to(&mut handler, x, y);
        end(&mut handler);
        let appointment = &handler.calendar().unwrap().appointments[0];
        assert_eq!(
            appointment.cells(),
            [
                Place { day: after(base, 9), resource: 101 },
                Place { day: after(base, 10), resource: 102 },
            ]
        );
        assert_eq!((appointment.start(), appointment.end()), (600, 660));
        assert!(handler.dirty());
        assert_eq!(placed_ids(&handler), [(1, [6.0, 5.0]), (1, [0.0, 9.0])]);
    }

    #[test]
    fn tapping_an_empty_month_cell_opens_a_new_form_on_that_day() {
        let mut handler = month_fixture(vec![]);
        let (x, y) = month_point(10, Some(2));
        handler.process_canvas(&pointer_down(Target::Surface.to_dom(), x, y), &state());
        let commands = handler.process_gesture(&Gesture::Tap, &state(), None).1;
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
        handler.process_canvas(&pointer_down(Target::Surface.to_dom(), x, y), &state());
        let commands = handler.process_gesture(&Gesture::Tap, &state(), None).1;
        assert_eq!(handler.view(), View::Day);
        assert_eq!(handler.base(), after(month_first(), 9));
        assert_eq!(text_of(&commands, &Target::AxisLabel(1, 2).to_dom()).unwrap(), "09:00");
        assert_eq!(text_of(&commands, &Target::AxisLabel(2, 45).to_dom()).unwrap(), "19:45");
        assert!(commands.iter().any(|command| matches!(command,
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. }
                if *id == Target::Band(4).to_dom())));
        assert!(commands.iter().any(|command| matches!(command,
            Command::RemoveAttribute { id, attribute: Attribute::Disabled }
                if *id == Target::Zoom.to_dom())));
        assert!(handler.editing.is_none());
    }

    #[test]
    fn the_zoom_slider_does_not_rescale_the_month() {
        let mut handler = month_fixture(vec![appointment(1, today(), 101, 600, 660)]);
        assert!(handler.process_canvas(&input(Target::Zoom.to_dom(), "3"), &state()).1.is_empty());
        assert_eq!(handler.slot_rem, SLOT_REM);
        let height = card_box(&handler.card_commands(), 1).unwrap().3;
        assert_eq!(height as f64, MONTH_ROW_REM * REM);
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
        let mut handler = Handler::new(VIEWPORT, today(), REM);
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
        assert_eq!(handler.calendar().unwrap().appointments.len(), 2);
    }
}
