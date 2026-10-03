use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec, vec::Vec};
use core::{
    cell::RefCell,
    option::Option::{self, None, Some},
    primitive::{f64, i32, u32, usize},
    result::Result::Ok,
};

use rectgrid::{
    BBox, IncrementFunction, Px, RectGrid, Unit as GridUnit, corner_test, drag_resize,
    drag_translate,
};

use crate::{
    Error,
    calendar::{
        data::{Appointment, Calendar, DataError, Place, parse_date, parse_time},
        date::{
            MONTH_CELL_COUNT, civil_from_days, format_hhmm, format_iso, long_title, month_origin,
            short_title,
        },
        grid::{Cell, ColumnAxis, Grid, TimeAxis},
        layout::lanes,
        store::{self, Store},
    },
    event::{Event, Response},
    file_store::FileStoreError,
    js_client::{
        Attribute, CanvasEvent, Command, EventType, Gesture, Keyword, Method, PointerState,
        StyleProperty, StyleValue, Unit, VisibilityState,
        dom::{Id, Tag},
        from_url_search_params,
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
const SAVE_BUTTON: u32 = 3;
const BAND_POOL: usize = 3;
const NEW_MINUTES: u32 = 60;
const RELOAD_BUTTON: u32 = 1;
const SLOT_REM: f64 = 1.75;
const ZOOM_MIN: f64 = 1.0;
const ZOOM_MAX: f64 = 3.0;
const COLUMN_MIN_REM: f64 = 5.0;
const AXIS_REM: f64 = 4.0;
const HEAD_REM: f64 = 5.0;
const STEP_DAYS: [i32; 5] = [-7, -1, 0, 1, 7];
const STEP_TODAY: u32 = 3;
const MONTH_BUTTON: u32 = 2;
const MONTH_COLUMNS: u32 = 7;
const LOAD_REQUEST: u32 = 1;
const LOAD_PATH: &str = "data/calendar.json";
const STATUS_OK: u16 = 200;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
    Day,
    ThreeDays,
    Week,
}

impl View {
    pub fn days(self) -> u32 {
        match self {
            Self::Day => 1,
            Self::ThreeDays => 3,
            Self::Week => 7,
        }
    }

    fn button(self) -> u32 {
        match self {
            Self::Day => 1,
            Self::ThreeDays => 2,
            Self::Week => 3,
        }
    }

    fn from_button(n: u32) -> Option<Self> {
        match n {
            1 => Some(Self::Day),
            2 => Some(Self::ThreeDays),
            3 => Some(Self::Week),
            _ => None,
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

struct EditForm {
    title:    String,
    category: String,
    status:   String,
    date:     String,
    resource: String,
    start:    String,
    end:      String,
    note:     String,
}

impl EditForm {
    fn parse(value: &str) -> Option<Self> {
        let pairs = from_url_search_params(value);
        let field =
            |name: &str| pairs.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone());
        Some(Self {
            title:    field("title")?,
            category: field("category")?,
            status:   field("status")?,
            date:     field("date")?,
            resource: field("resource")?,
            start:    field("start")?,
            end:      field("end")?,
            note:     field("note")?,
        })
    }
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
    today:             i32,
    base:              i32,
    month:             (i32, u32),
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
        #[allow(unused_mut)]
        let mut handler =
            Self::new(viewport_width_px, day_number(now, timezone_offset_minutes), rem_in_px);
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

    pub fn new(viewport_width_px: f64, today: i32, rem_in_px: f64) -> Self {
        let view = View::Week;
        let column_px = column_px(viewport_width_px, view, rem_in_px);
        let (year, month, _) = civil_from_days(today);
        Self {
            view,
            today,
            base: today,
            month: (year, month),
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
        (vec![], commands)
    }

    pub fn process_fetched(&mut self, response: &Response) -> (Vec<Event>, Vec<Command>) {
        if response.request != LOAD_REQUEST {
            return (vec![], vec![]);
        }
        if response.status != STATUS_OK {
            return (vec![], vec![data_error(DataError::Status(response.status))]);
        }
        match Calendar::parse(&response.body) {
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
        match event.event_type {
            EventType::Click => {
                if let Some(view) = view_button_at(&event.id) {
                    return (vec![], self.change_view(view));
                }
                if let Some(commands) = self.click_control(&event.id) {
                    return (vec![], commands);
                }
                (vec![], vec![])
            }
            EventType::Input if event.id == zoom_input() => (vec![], self.zoom(&event.value)),
            EventType::PointerDown => (vec![], self.press(event)),
            EventType::Submit if event.id == edit_form() => (vec![], self.submit(&event.value)),
            EventType::Scroll => {
                if event.id == surface() {
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

    fn press(&mut self, event: &CanvasEvent) -> Vec<Command> {
        self.drag = None;
        self.pending_tap = None;
        self.create = None;
        let Some(n) = card_at(&event.id) else {
            if surface_at(&event.id) {
                self.set_origin(event.root_origin());
                self.pending_tap = Some([event.x, event.y]);
                self.create = self.unit_at([event.x, event.y]).map(|anchor| CreateDrag {
                    anchor,
                    current: anchor,
                    moved: false,
                });
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
        let corner = self.handle_at(&placed, pointer);
        let kind = match corner.and_then(|corner| self.resize_kind(&placed, pointer, corner)) {
            Some(kind) => kind,
            None => DragKind::Move,
        };
        let mut commands = vec![];
        if let DragKind::Resize { corner, .. } = &kind {
            if let Some(cursor) = corner_cursor(*corner) {
                commands.push(style(
                    card_item(n),
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
        let (start, end) = grid.time.clip(appointment.start, appointment.end)?;
        let flats = appointment.cells.iter().filter_map(|place| {
            let resource = calendar.resources.iter().position(|r| r.id == place.resource)?;
            Some(grid.columns.flat(place.day - self.base, resource as u32))
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
                card_item(n),
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
                    card_item(n),
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
                card_item(n),
                StyleProperty::Translate,
                StyleValue::List(vec![rem(x.get(), self.rem_in_px), rem(y.get(), self.rem_in_px)]),
            ),
            style(card_item(n), StyleProperty::Width, rem(width.get(), self.rem_in_px)),
            style(card_item(n), StyleProperty::Height, rem(height.get(), self.rem_in_px)),
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
        fields: EditForm,
        hint: &str,
    ) -> Vec<Command> {
        self.editing = Some(editing);
        let value = |field, value: String| Command::SetValue { id: edit_field(field), value };
        vec![
            hidden(month_form(), true),
            hidden(edit_form(), false),
            Command::SetText { id: edit_heading(), value: heading },
            Command::SetText { id: edit_message(), value: String::from(hint) },
            value(EditField::Title, fields.title),
            value(EditField::Category, fields.category),
            value(EditField::Status, fields.status),
            value(EditField::Date, fields.date),
            value(EditField::Resource, fields.resource),
            value(EditField::Start, fields.start),
            value(EditField::End, fields.end),
            value(EditField::Note, fields.note),
            Command::ShowModal { id: modal() },
        ]
    }

    fn zoom(&mut self, value: &str) -> Vec<Command> {
        let Ok(slot_rem) = value.parse::<f64>() else {
            return vec![];
        };
        let slot_rem = slot_rem.clamp(ZOOM_MIN, ZOOM_MAX);
        if slot_rem == self.slot_rem {
            return vec![];
        }
        self.slot_rem = slot_rem;
        self.fit_rectgrid();
        let mut commands = self.slot_rows();
        commands.extend(self.band_commands());
        commands.extend(self.card_commands());
        commands
    }

    fn slot_rows(&self) -> Vec<Command> {
        let rem = self.slot_rem;
        let axis = format!("var(--head-height) repeat({SLOT_COUNT}, {rem}rem)");
        let surface_rows =
            format!("var(--head-title) var(--head-resource) calc({rem}rem * {SLOT_COUNT})");
        vec![
            style(time_axis(1), StyleProperty::GridTemplateRows, StyleValue::Text(axis.clone())),
            style(time_axis(2), StyleProperty::GridTemplateRows, StyleValue::Text(axis)),
            style(surface(), StyleProperty::GridTemplateRows, StyleValue::Text(surface_rows)),
        ]
    }

    fn unit_at(&self, pointer: [f64; 2]) -> Option<[i32; 2]> {
        let [Ok(column), Ok(row)] =
            self.rectgrid.point_to_unit([Px::new(pointer[0]), Px::new(pointer[1])])
        else {
            return None;
        };
        let (column, row) = (libm::floor(column.get()), libm::floor(row.get()));
        let columns = self.columns() as f64;
        (column >= 0.0 && column < columns && row >= 0.0 && row < SLOT_COUNT as f64)
            .then_some([column as i32, row as i32])
    }

    fn clamped_unit_at(&self, pointer: [f64; 2]) -> Option<[i32; 2]> {
        let [Ok(column), Ok(row)] =
            self.rectgrid.point_to_unit([Px::new(pointer[0]), Px::new(pointer[1])])
        else {
            return None;
        };
        let columns = self.columns() as i32;
        Some([
            (libm::floor(column.get()) as i32).clamp(0, columns - 1),
            (libm::floor(row.get()) as i32).clamp(0, SLOT_COUNT as i32 - 1),
        ])
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
        let Some(current) = self.clamped_unit_at([x, y]) else {
            return vec![];
        };
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
            hidden(preview_item(), false),
            style(
                preview_item(),
                StyleProperty::Translate,
                StyleValue::List(vec![
                    rem(px.get(), self.rem_in_px),
                    rem(py.get(), self.rem_in_px),
                ]),
            ),
            style(preview_item(), StyleProperty::Width, rem(width.get(), self.rem_in_px)),
            style(preview_item(), StyleProperty::Height, rem(height.get(), self.rem_in_px)),
        ]
    }

    fn finish_create(&mut self) -> Vec<Command> {
        let Some(create) = self.create.take() else {
            return vec![];
        };
        let mut commands = vec![hidden(preview_item(), true)];
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
                Some(Place { day: self.base + cell.day as i32, resource: resource.id })
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
        let Some(calendar) = self.calendar.as_ref() else {
            return vec![];
        };
        let Some(first) = cells.first() else {
            return vec![];
        };
        let hint = match self.band_hit([Px::new(pointer[0]), Px::new(pointer[1])]) {
            None => "",
            Some(BandKind::Break) => "休憩中",
            Some(BandKind::Closed) => "営業時間外",
        };
        let fields = EditForm {
            title:    String::new(),
            category: String::new(),
            status:   calendar.statuses.first().map_or_else(String::new, |s| s.code.clone()),
            date:     format_iso(first.day),
            resource: format!("{}", first.resource),
            start:    format_hhmm(start),
            end:      format_hhmm(end),
            note:     String::new(),
        };
        self.form_commands(Editing::New(cells), String::from("新規"), fields, hint)
    }

    fn create_commands(&mut self) -> Vec<Command> {
        let Some(pointer) = self.pending_tap.take() else {
            return vec![];
        };
        let Some(calendar) = self.calendar.as_ref() else {
            return vec![];
        };
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
        let place = Place { day: self.base + cell.day as i32, resource: resource.id };
        self.new_form(vec![place], start, start + NEW_MINUTES, pointer)
    }

    fn edit_commands(&mut self, index: usize, cell: usize) -> Vec<Command> {
        let Some(appointment) = self.calendar.as_ref().and_then(|c| c.appointments.get(index))
        else {
            return vec![];
        };
        let Some(place) = appointment.cells.get(cell).or_else(|| appointment.cells.first()) else {
            return vec![];
        };
        let fields = EditForm {
            title:    appointment.title.clone(),
            category: appointment.category.clone(),
            status:   appointment.status.clone(),
            date:     format_iso(place.day),
            resource: format!("{}", place.resource),
            start:    format_hhmm(appointment.start),
            end:      format_hhmm(appointment.end),
            note:     appointment.note.clone(),
        };
        let heading = format!("#{}", appointment.id);
        self.form_commands(Editing::Existing(index), heading, fields, "")
    }

    fn reject(message: &str) -> Vec<Command> {
        vec![Command::SetText { id: edit_message(), value: String::from(message) }]
    }

    fn submit(&mut self, value: &str) -> Vec<Command> {
        let Some(editing) = self.editing.clone() else {
            return vec![];
        };
        let Some(form) = EditForm::parse(value) else {
            return Self::reject("入力を読み取れません");
        };
        match self.apply_form(editing, form) {
            Ok(index) => {
                self.editing = None;
                let mut commands = self.persist(index);
                commands.extend(self.persist_meta());
                commands.extend(self.mark_dirty());
                commands.push(Command::CloseModal { id: modal() });
                commands.extend(self.loaded_commands());
                commands
            }
            Err(message) => Self::reject(message),
        }
    }

    fn apply_form(&mut self, editing: Editing, form: EditForm) -> Result<usize, &'static str> {
        let day = parse_date(&form.date).map_err(|_| "日付が不正です")?;
        let start = parse_time(&form.start).map_err(|_| "開始時刻が不正です")?;
        let end = parse_time(&form.end).map_err(|_| "終了時刻が不正です")?;
        if start >= end || start < TIME_AXIS.minutes(0).unwrap_or(0) || end > TIME_AXIS.end() {
            return Err("時間帯が不正です");
        }
        let calendar = self.calendar.as_mut().ok_or("データがありません")?;
        let resource_id: u32 = form.resource.parse().map_err(|_| "資源が不正です")?;
        let resource =
            calendar.resources.iter().position(|r| r.id == resource_id).ok_or("資源が不正です")?;
        if !calendar.statuses.iter().any(|s| s.code == form.status) {
            return Err("状態が不正です");
        }
        match editing {
            Editing::New(pending) => {
                if !fits(calendar, start, end) {
                    return Err("営業時間外です");
                }
                let cells = shift_cells(calendar, &pending, day, resource)?;
                let id = calendar.appointments.iter().map(|a| a.id).max().map_or(1, |id| id + 1);
                calendar.appointments.push(Appointment {
                    id,
                    cells,
                    start,
                    end,
                    title: form.title,
                    category: form.category,
                    status: form.status,
                    note: form.note,
                });
                Ok(calendar.appointments.len() - 1)
            }
            Editing::Existing(index) => {
                let appointment = calendar.appointments.get(index).ok_or("予約がありません")?;
                let cells = shift_cells(calendar, &appointment.cells, day, resource)?;
                let current = &calendar.appointments[index];
                let changed = current.start != start || current.end != end;
                if changed && !fits(calendar, start, end) {
                    return Err("営業時間外です");
                }
                let appointment = &mut calendar.appointments[index];
                appointment.cells = cells;
                appointment.start = start;
                appointment.end = end;
                appointment.title = form.title;
                appointment.category = form.category;
                appointment.status = form.status;
                appointment.note = form.note;
                Ok(index)
            }
        }
    }

    fn persist_meta(&mut self) -> Vec<Command> {
        let (Some(store), Some(calendar)) = (self.store.as_mut(), self.calendar.as_ref()) else {
            return vec![];
        };
        match store::put_meta(store.as_mut(), calendar) {
            Ok(()) => vec![],
            Err(error) => vec![data_error(error)],
        }
    }

    fn drag_cancel(&mut self) -> Vec<Command> {
        let Some(drag) = self.drag.take() else {
            self.create = None;
            self.pending_tap = None;
            return vec![hidden(preview_item(), true)];
        };
        let mut commands = self.release_commands(&drag);
        commands.extend(self.card_commands());
        commands
    }

    fn release_commands(&self, drag: &Drag) -> Vec<Command> {
        let mut commands = vec![Command::RemoveStyle {
            id:       card_item(drag.n),
            property: StyleProperty::ZIndex,
        }];
        if matches!(drag.kind, DragKind::Resize { .. }) {
            commands.push(Command::RemoveStyle {
                id:       card_item(drag.n),
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
                    let resource = calendar.resources.get(resource as usize)?.id;
                    Some(Place { day: base + day, resource })
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
        let new_cells = match cells {
            Some(Some(cells)) => cells,
            _ => current.cells.clone(),
        };
        let (new_start, new_end) = match time {
            Some(Some((start, end))) if end > start => (start, end),
            _ => (current.start, current.end),
        };
        let changed =
            new_cells != current.cells || new_start != current.start || new_end != current.end;
        if changed && !fits(calendar, new_start, new_end) {
            return false;
        }
        let appointment = &mut calendar.appointments[index];
        appointment.cells = new_cells;
        appointment.start = new_start;
        appointment.end = new_end;
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
        let duration = appointment.end - appointment.start;
        let logical = BBox::new(
            [GridUnit::new(libm::floor(column.get())), GridUnit::new(slot as f64)],
            [GridUnit::new(1.0), GridUnit::new(duration as f64 / SLOT_MINUTES as f64)],
        );
        let resolved = grid.resolve(&logical)?;
        let cell = *resolved.cells.first()?;
        Some((cell, grid.time.clamp_start(resolved.start, duration)))
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
        let flat = |place: &Place| {
            calendar
                .resources
                .iter()
                .position(|r| r.id == place.resource)
                .map(|position| axis.flat(place.day - base, position as u32))
        };
        let Some(from) = appointment.cells.get(pressed).and_then(flat) else {
            return false;
        };
        let delta = axis.flat(target.day as i32, target.resource) - from;
        let moved: Option<Vec<Place>> = appointment
            .cells
            .iter()
            .map(|place| {
                let (day, resource) = axis.locate(flat(place)? + delta);
                let resource = calendar.resources.get(resource as usize)?.id;
                Some(Place { day: base + day, resource })
            })
            .collect();
        let Some(moved) = moved else {
            return false;
        };
        let duration = appointment.end - appointment.start;
        let changed = appointment.cells != moved || appointment.start != start;
        if changed && !fits(calendar, start, start + duration) {
            return false;
        }
        let Some(appointment) = calendar.appointments.get_mut(index) else {
            return false;
        };
        appointment.cells = moved;
        appointment.start = start;
        appointment.end = start + duration;
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

    pub fn base(&self) -> i32 {
        self.base
    }

    fn click_control(&mut self, id: &Id) -> Option<Vec<Command>> {
        if let Some(n) = step_button_at(id) {
            self.base =
                if n == STEP_TODAY { self.today } else { self.base + STEP_DAYS[n as usize - 1] };
            return Some(self.date_commands());
        }
        if *id == modal() {
            self.editing = None;
            return Some(vec![Command::CloseModal { id: modal() }]);
        }
        if *id == save_button() {
            return Some(self.save_commands());
        }
        if *id == reload_button() {
            return Some(self.discard_commands());
        }
        if *id == month_button() {
            self.month = month_of(self.base);
            let mut commands = vec![hidden(edit_form(), true), hidden(month_form(), false)];
            commands.extend(self.month_commands());
            commands.push(Command::ShowModal { id: modal() });
            return Some(commands);
        }
        if let Some(delta) = month_step_at(id) {
            let index = self.month.0 * 12 + (self.month.1 as i32 - 1) + delta;
            self.month = (index.div_euclid(12), index.rem_euclid(12) as u32 + 1);
            return Some(self.month_commands());
        }
        if let Some(index) = month_cell_at(id) {
            self.base = month_origin(self.month.0, self.month.1) + index as i32;
            let mut commands = self.date_commands();
            commands.push(Command::CloseModal { id: modal() });
            return Some(commands);
        }
        None
    }

    fn resource_text(&self, n: u32) -> Command {
        let index = heading_axis().cell(n - 1).map_or(0, |cell| cell.resource as usize);
        let value = match self.calendar.as_ref().and_then(|calendar| calendar.resources.get(index))
        {
            Some(resource) => resource.name.clone(),
            None => format!("R{}", index + 1),
        };
        Command::SetText { id: resource_name(n), value }
    }

    fn loaded_commands(&self) -> Vec<Command> {
        let mut commands: Vec<Command> =
            (1..=heading_axis().count()).map(|n| self.resource_text(n)).collect();
        commands.extend(self.band_commands());
        commands.extend(self.card_commands());
        commands
    }

    fn visible_cards(&self, calendar: &Calendar) -> Vec<Card> {
        let grid = self.grid();
        let mut columns: BTreeMap<u32, Vec<(usize, usize, u32, u32)>> = BTreeMap::new();
        let mut extent: BTreeMap<usize, (i32, i32)> = BTreeMap::new();
        for (index, appointment) in calendar.appointments.iter().enumerate() {
            let Some((start, end)) = grid.time.clip(appointment.start, appointment.end) else {
                continue;
            };
            for (cell_index, place) in appointment.cells.iter().enumerate() {
                let Some(resource) = calendar.resources.iter().position(|r| r.id == place.resource)
                else {
                    continue;
                };
                let flat = grid.columns.flat(place.day - self.base, resource as u32);
                let range = extent.entry(index).or_insert((flat, flat));
                *range = (range.0.min(flat), range.1.max(flat));
                let Ok(day) = u32::try_from(place.day - self.base) else {
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

    fn band_list(&self, calendar: &Calendar) -> Vec<Band> {
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
        let days = self.view.days();
        let mut commands = Vec::new();
        for n in 1..=heading_axis().count() {
            let person =
                heading_axis().cell(n - 1).filter(|cell| cell.day < days).and_then(|cell| {
                    let calendar = self.calendar.as_ref()?;
                    let resource = calendar.resources.get(cell.resource as usize)?;
                    calendar
                        .shifts
                        .iter()
                        .find(|s| s.day == self.base + cell.day as i32 && s.resource == resource.id)
                        .map(|s| s.person.clone())
                });
            commands.push(Command::SetText {
                id:    resource_person(n),
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
            commands.push(hidden(band_item(n), false));
            commands.push(style(
                band_item(n),
                StyleProperty::Translate,
                StyleValue::List(vec![rem(x.get(), self.rem_in_px), rem(y.get(), self.rem_in_px)]),
            ));
            commands.push(style(
                band_item(n),
                StyleProperty::Width,
                rem(width.get(), self.rem_in_px),
            ));
            commands.push(style(
                band_item(n),
                StyleProperty::Height,
                rem(height.get(), self.rem_in_px),
            ));
            commands.push(style(
                band_item(n),
                StyleProperty::Background,
                StyleValue::Text(String::from("var(--color-paper-mix)")),
            ));
        }
        let shown = self.bands.borrow().len() as u32;
        for n in placed.len() as u32 + 1..=shown {
            commands.push(hidden(band_item(n), true));
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
        let cards = self.visible_cards(calendar);
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
            let status = calendar
                .statuses
                .iter()
                .find(|s| s.code == appointment.status)
                .map_or(appointment.status.as_str(), |s| s.label.as_str());
            commands.push(Command::RemoveAttribute {
                id:        card_item(n),
                attribute: Attribute::Hidden,
            });
            commands.push(style(
                card_item(n),
                StyleProperty::Translate,
                StyleValue::List(vec![rem(x.get(), self.rem_in_px), rem(y.get(), self.rem_in_px)]),
            ));
            commands.push(style(
                card_item(n),
                StyleProperty::Width,
                rem(width.get(), self.rem_in_px),
            ));
            commands.push(style(
                card_item(n),
                StyleProperty::Height,
                rem(height.get(), self.rem_in_px),
            ));
            for (id, value) in [
                (card_part(n, CardPart::Status), String::from(status)),
                (
                    card_part(n, CardPart::Time),
                    format!(
                        "{:02}:{:02}–{:02}:{:02}",
                        appointment.start / 60,
                        appointment.start % 60,
                        appointment.end / 60,
                        appointment.end % 60
                    ),
                ),
                (card_part(n, CardPart::Title), appointment.title.clone()),
                (card_part(n, CardPart::Category), appointment.category.clone()),
                (card_part(n, CardPart::Note), appointment.note.clone()),
            ] {
                commands.push(Command::SetText { id, value });
            }
        }
        let shown = self.placed.borrow().len() as u32;
        for n in placed.len() as u32 + 1..=shown {
            commands.push(hidden(card_item(n), true));
        }
        *self.placed.borrow_mut() = placed;
        commands
    }

    fn mark_dirty(&mut self) -> Option<Command> {
        if self.dirty {
            return None;
        }
        self.dirty = true;
        Some(Command::RemoveAttribute { id: save_button(), attribute: Attribute::Disabled })
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
        let (Some(store), Some(calendar)) = (self.store.as_mut(), self.calendar.as_ref()) else {
            return vec![];
        };
        let Some(appointment) = calendar.appointments.get(index) else {
            return vec![];
        };
        match store::put_appointment(store.as_mut(), appointment) {
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
            id:        save_button(),
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
                id:        save_button(),
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
        let days = self.view.days();
        let mut commands: Vec<Command> = (1..=days)
            .map(|n| Command::SetText {
                id:    day_title(n),
                value: short_title(self.base + n as i32 - 1),
            })
            .collect();
        let first = long_title(self.base);
        let value = if days == 1 {
            first
        } else {
            format!("{first} – {}", long_title(self.base + days as i32 - 1))
        };
        commands.push(Command::SetText { id: title(), value });
        commands.extend(self.band_commands());
        commands.extend(self.card_commands());
        commands
    }

    fn month_commands(&self) -> Vec<Command> {
        let (year, month) = self.month;
        let origin = month_origin(year, month);
        let mut commands =
            vec![Command::SetText { id: month_title(), value: format!("{year}年{month}月") }];
        for index in 0..MONTH_CELL_COUNT {
            let day = origin + index as i32;
            let (_, cell_month, cell_day) = civil_from_days(day);
            let id = month_cell(index);
            commands.push(Command::SetText { id: id.clone(), value: format!("{cell_day}") });
            commands.push(if cell_month == month {
                Command::RemoveAttribute { id: id.clone(), attribute: Attribute::Disabled }
            } else {
                Command::SetAttribute {
                    id:        id.clone(),
                    attribute: Attribute::Disabled,
                    value:     String::new(),
                }
            });
            let current = day == self.base;
            commands.push(if current {
                Command::SetAttribute {
                    id:        id.clone(),
                    attribute: Attribute::AriaCurrent,
                    value:     String::from("date"),
                }
            } else {
                Command::RemoveAttribute {
                    id:        id.clone(),
                    attribute: Attribute::AriaCurrent,
                }
            });
            commands.push(Command::SetAttribute {
                id,
                attribute: Attribute::DataSurround,
                value: String::from(if current { "fill" } else { "transparent" }),
            });
        }
        commands
    }

    fn grid(&self) -> Grid {
        Grid::new(ColumnAxis::new(self.view.days(), RESOURCE_COUNT), TIME_AXIS)
    }

    fn columns(&self) -> u32 {
        self.grid().columns.count()
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

    fn view_commands(&self) -> Vec<Command> {
        let days = self.view.days();
        let columns = self.columns();
        let mut commands = vec![
            style(
                surface(),
                StyleProperty::GridTemplateColumns,
                StyleValue::Text(format!("repeat({columns}, minmax(var(--column-width), 1fr))")),
            ),
            style(
                day_list(),
                StyleProperty::GridTemplateColumns,
                StyleValue::Text(format!("repeat({days}, 1fr)")),
            ),
        ];
        for n in 1..=DAY_MAX {
            commands.push(hidden(day_item(n), n > days));
        }
        commands.extend(self.date_commands());
        for n in 1..=heading_axis().count() {
            commands.push(hidden(resource_item(n), n > columns));
            commands.push(self.resource_text(n));
        }
        for view in [View::Day, View::ThreeDays, View::Week] {
            let id = view_button(view.button());
            commands.push(if view == self.view {
                Command::SetAttribute { id, attribute: Attribute::Disabled, value: format!("") }
            } else {
                Command::RemoveAttribute { id, attribute: Attribute::Disabled }
            });
        }
        commands
    }
}

fn shift_cells(
    calendar: &Calendar,
    cells: &[Place],
    day: i32,
    resource: usize,
) -> Result<Vec<Place>, &'static str> {
    let first = cells.first().ok_or("予約がありません")?;
    let position = |place: &Place| {
        calendar.resources.iter().position(|r| r.id == place.resource).ok_or("資源が不正です")
    };
    let day_delta = day - first.day;
    let resource_delta = resource as i32 - position(first)? as i32;
    cells
        .iter()
        .map(|place| {
            let index = usize::try_from(position(place)? as i32 + resource_delta)
                .map_err(|_| "範囲外です")?;
            let moved = calendar.resources.get(index).ok_or("範囲外です")?;
            Ok(Place { day: place.day + day_delta, resource: moved.id })
        })
        .collect()
}

fn preview_item() -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, None), (Tag::Ol, Some(5)), (Tag::Li, None)])
}

fn hours(calendar: &Calendar) -> Option<Hours> {
    let first = calendar.shifts.first()?;
    let mut hours = Hours { open: first.open, close: first.close, rest: first.break_range };
    for shift in &calendar.shifts[1..] {
        hours.open = hours.open.min(shift.open);
        hours.close = hours.close.max(shift.close);
        hours.rest = match (hours.rest, shift.break_range) {
            (Some((a, b)), Some((c, d))) if a.max(c) < b.min(d) => Some((a.max(c), b.min(d))),
            _ => None,
        };
    }
    Some(hours)
}

fn fits(calendar: &Calendar, start: u32, end: u32) -> bool {
    hours(calendar).is_none_or(|hours| hours.open <= start && end <= hours.close)
}

fn day_number(now: f64, timezone_offset_minutes: i32) -> i32 {
    let minutes = libm::floor(now / 60_000.0) as i64 + timezone_offset_minutes as i64;
    minutes.div_euclid(1440) as i32
}

fn column_px(viewport_width_px: f64, view: View, rem_in_px: f64) -> f64 {
    let columns = ColumnAxis::new(view.days(), RESOURCE_COUNT).count() as f64;
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
    if on {
        Command::SetAttribute { id, attribute: Attribute::Hidden, value: format!("") }
    } else {
        Command::RemoveAttribute { id, attribute: Attribute::Hidden }
    }
}

fn surface() -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, None)])
}

fn time_axis(n: u32) -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Ol, Some(n))])
}

fn day_list() -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, None), (Tag::Ol, Some(1))])
}

fn day_item(n: u32) -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, None), (Tag::Ol, Some(1)), (Tag::Li, Some(n))])
}

fn day_title(n: u32) -> Id {
    Id::new(&[
        (Tag::Main, None),
        (Tag::Section, None),
        (Tag::Ol, Some(1)),
        (Tag::Li, Some(n)),
        (Tag::H3, None),
    ])
}

fn resource_item(n: u32) -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, None), (Tag::Ol, Some(2)), (Tag::Li, Some(n))])
}

fn resource_name(n: u32) -> Id {
    Id::new(&[
        (Tag::Main, None),
        (Tag::Section, None),
        (Tag::Ol, Some(2)),
        (Tag::Li, Some(n)),
        (Tag::Strong, None),
    ])
}

fn view_button(n: u32) -> Id {
    Id::new(&[
        (Tag::Header, None),
        (Tag::Nav, Some(1)),
        (Tag::Div, Some(1)),
        (Tag::Button, Some(n)),
    ])
}

fn step_button(n: u32) -> Id {
    Id::new(&[
        (Tag::Header, None),
        (Tag::Nav, Some(1)),
        (Tag::Div, Some(2)),
        (Tag::Button, Some(n)),
    ])
}

enum CardPart {
    Status,
    Time,
    Title,
    Category,
    Note,
}

fn band_item(n: u32) -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, None), (Tag::Ol, Some(3)), (Tag::Li, Some(n))])
}

fn resource_person(n: u32) -> Id {
    Id::new(&[
        (Tag::Main, None),
        (Tag::Section, None),
        (Tag::Ol, Some(2)),
        (Tag::Li, Some(n)),
        (Tag::Span, None),
    ])
}

fn card_item(n: u32) -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, None), (Tag::Ol, Some(4)), (Tag::Li, Some(n))])
}

fn corner_cursor(corner: Corner) -> Option<Keyword> {
    match corner {
        [Some(x), Some(y)] => Some(if x == y { Keyword::NwseResize } else { Keyword::NeswResize }),
        [Some(_), None] => Some(Keyword::EwResize),
        [None, Some(_)] => Some(Keyword::NsResize),
        [None, None] => None,
    }
}

fn card_at(id: &Id) -> Option<u32> {
    let segments = &id.0;
    let [main, section, list, item, ..] = segments.as_slice() else {
        return None;
    };
    (main.tag == Tag::Main
        && section.tag == Tag::Section
        && list.tag == Tag::Ol
        && list.n == Some(4)
        && item.tag == Tag::Li)
        .then_some(item.n)
        .flatten()
}

fn card_part(n: u32, part: CardPart) -> Id {
    let mut segments = card_item(n).0;
    let tail: &[(Tag, Option<u32>)] = match part {
        CardPart::Status => &[(Tag::P, Some(1)), (Tag::Span, Some(1))],
        CardPart::Time => &[(Tag::P, Some(1)), (Tag::Span, Some(2))],
        CardPart::Title => &[(Tag::H3, None)],
        CardPart::Category => &[(Tag::P, Some(2))],
        CardPart::Note => &[(Tag::P, Some(3))],
    };
    segments.extend(
        tail.iter().map(|(tag, n)| crate::js_client::dom::Segment { tag: tag.clone(), n: *n }),
    );
    Id(segments)
}

fn file_store_error(error: FileStoreError) -> Command {
    Command::Error { error: Error::FileStore(error) }
}

fn data_error(error: DataError) -> Command {
    Command::Error { error: Error::Data(error) }
}

fn save_button() -> Id {
    Id::new(&[(Tag::Header, None), (Tag::Nav, Some(2)), (Tag::Button, Some(SAVE_BUTTON))])
}

fn reload_button() -> Id {
    Id::new(&[(Tag::Header, None), (Tag::Nav, Some(2)), (Tag::Button, Some(RELOAD_BUTTON))])
}

fn zoom_input() -> Id {
    Id::new(&[(Tag::Header, None), (Tag::Nav, Some(2)), (Tag::Input, None)])
}

fn month_button() -> Id {
    Id::new(&[(Tag::Header, None), (Tag::Nav, Some(2)), (Tag::Button, Some(MONTH_BUTTON))])
}

fn modal() -> Id {
    Id::new(&[(Tag::Modal, None)])
}

fn month_form() -> Id {
    Id::new(&[(Tag::Modal, None), (Tag::Form, Some(1))])
}

fn edit_form() -> Id {
    Id::new(&[(Tag::Modal, None), (Tag::Form, Some(2))])
}

fn edit_message() -> Id {
    Id::new(&[(Tag::Modal, None), (Tag::Form, Some(2)), (Tag::Output, None)])
}

fn surface_at(id: &Id) -> bool {
    matches!(id.0.as_slice(), [main, section, ..] if main.tag == Tag::Main && section.tag == Tag::Section)
}

fn edit_heading() -> Id {
    Id::new(&[(Tag::Modal, None), (Tag::Form, Some(2)), (Tag::Header, None), (Tag::H3, None)])
}

fn edit_field(field: EditField) -> Id {
    let (n, tag) = match field {
        EditField::Title => (1, Tag::Input),
        EditField::Category => (2, Tag::Input),
        EditField::Status => (3, Tag::Select),
        EditField::Date => (4, Tag::Input),
        EditField::Resource => (5, Tag::Select),
        EditField::Start => (6, Tag::Input),
        EditField::End => (7, Tag::Input),
        EditField::Note => (8, Tag::Textarea),
    };
    Id::new(&[
        (Tag::Modal, None),
        (Tag::Form, Some(2)),
        (Tag::Dl, None),
        (Tag::Dd, Some(n)),
        (tag, None),
    ])
}

#[derive(Clone, Copy)]
enum EditField {
    Title,
    Category,
    Status,
    Date,
    Resource,
    Start,
    End,
    Note,
}

fn month_title() -> Id {
    Id::new(&[(Tag::Modal, None), (Tag::Form, Some(1)), (Tag::Header, None), (Tag::H3, None)])
}

fn month_step(n: u32) -> Id {
    Id::new(&[
        (Tag::Modal, None),
        (Tag::Form, Some(1)),
        (Tag::Header, None),
        (Tag::Button, Some(n)),
    ])
}

fn month_cell(index: usize) -> Id {
    let (row, column) = (index as u32 / MONTH_COLUMNS + 1, index as u32 % MONTH_COLUMNS + 1);
    Id::new(&[
        (Tag::Modal, None),
        (Tag::Form, Some(1)),
        (Tag::Table, None),
        (Tag::Tbody, None),
        (Tag::Tr, Some(row)),
        (Tag::Td, Some(column)),
        (Tag::Button, None),
    ])
}

fn title() -> Id {
    Id::new(&[(Tag::Header, None), (Tag::H2, None)])
}

fn view_button_at(id: &Id) -> Option<View> {
    (1..=3).find(|n| *id == view_button(*n)).and_then(View::from_button)
}

fn step_button_at(id: &Id) -> Option<u32> {
    (1..=STEP_DAYS.len() as u32).find(|n| *id == step_button(*n))
}

fn month_step_at(id: &Id) -> Option<i32> {
    [(1, -1), (2, 1)].into_iter().find(|(n, _)| *id == month_step(*n)).map(|(_, delta)| delta)
}

fn month_cell_at(id: &Id) -> Option<usize> {
    (0..MONTH_CELL_COUNT).find(|index| *id == month_cell(*index))
}

fn month_of(days: i32) -> (i32, u32) {
    let (year, month, _) = civil_from_days(days);
    (year, month)
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use std::fs;

    use super::*;
    use crate::{
        calendar::{date::days_from_civil, store::memory::MemoryStore},
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

    fn today() -> i32 {
        days_from_civil(2026, 10, 2)
    }

    #[test]
    fn day_number_follows_the_local_date() {
        let new_millennium = 946_684_800_000.0;
        assert_eq!(day_number(new_millennium, 0), days_from_civil(2000, 1, 1));
        assert_eq!(day_number(new_millennium, 540), days_from_civil(2000, 1, 1));
        assert_eq!(day_number(new_millennium, -1), days_from_civil(1999, 12, 31));
        assert_eq!(day_number(new_millennium + 899.0 * 60_000.0, 540), days_from_civil(2000, 1, 1));
        assert_eq!(day_number(new_millennium + 900.0 * 60_000.0, 540), days_from_civil(2000, 1, 2));
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
            (surface(), format!("repeat({columns}, minmax(var(--column-width), 1fr))")),
            (day_list(), format!("repeat({days}, 1fr)")),
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
    }

    #[test]
    fn view_button_switches_days_columns_and_hides_the_rest() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let (_, commands) = handler.process_canvas(&click(view_button(2), 10.0, 10.0), &state());
        assert_eq!(handler.view(), View::ThreeDays);
        assert_eq!(grid_columns(&commands), week_grid(12, 3));
        assert_eq!(hidden_count(&commands), 4 + 16);

        let (_, commands) = handler.process_canvas(&click(view_button(1), 10.0, 10.0), &state());
        assert_eq!(handler.view(), View::Day);
        assert_eq!(grid_columns(&commands), week_grid(4, 1));
        assert_eq!(hidden_count(&commands), 6 + 24);
    }

    #[test]
    fn view_button_for_the_current_view_emits_nothing() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let (_, commands) = handler.process_canvas(&click(view_button(3), 10.0, 10.0), &state());
        assert!(commands.is_empty());
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

    fn press(handler: &mut Handler, id: Id) -> Vec<Command> {
        handler.process_canvas(&click(id, 10.0, 10.0), &state()).1
    }

    #[test]
    fn initial_draw_titles_the_week_from_today() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let (_, commands) = handler.initial_draw();
        assert_eq!(text_of(&commands, &day_title(1)).unwrap(), "10/2(金)");
        assert_eq!(text_of(&commands, &day_title(7)).unwrap(), "10/8(木)");
        assert_eq!(text_of(&commands, &title()).unwrap(), "2026年10月2日(金) – 2026年10月8日(木)");
    }

    #[test]
    fn step_buttons_move_the_base_and_today_returns() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        for (button, expected) in [(5, 7), (1, 0), (2, -1), (4, 0), (4, 1)] {
            press(&mut handler, step_button(button));
            assert_eq!(handler.base() - today(), expected);
        }
        let commands = press(&mut handler, step_button(3));
        assert_eq!(handler.base(), today());
        assert_eq!(text_of(&commands, &day_title(1)).unwrap(), "10/2(金)");
    }

    #[test]
    fn single_day_view_titles_one_date() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        press(&mut handler, view_button(1));
        let commands = press(&mut handler, step_button(4));
        assert_eq!(text_of(&commands, &title()).unwrap(), "2026年10月3日(土)");
        assert_eq!(texts(&commands).len(), 2);
    }

    #[test]
    fn month_button_opens_the_modal_filled_with_the_base_month() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let commands = press(&mut handler, month_button());
        assert!(matches!(commands.last(), Some(Command::ShowModal { id }) if *id == modal()));
        assert_eq!(text_of(&commands, &month_title()).unwrap(), "2026年10月");
        assert_eq!(text_of(&commands, &month_cell(0)).unwrap(), "27");
        assert_eq!(text_of(&commands, &month_cell(5)).unwrap(), "2");
        let current: Vec<_> = commands
            .iter()
            .filter(|command| {
                matches!(command, Command::SetAttribute { attribute: Attribute::AriaCurrent, .. })
            })
            .collect();
        assert!(matches!(
            current.as_slice(),
            [Command::SetAttribute { id, value, .. }] if *id == month_cell(5) && value == "date"
        ));
        let enabled = commands
            .iter()
            .filter(|command| {
                matches!(command, Command::RemoveAttribute { attribute: Attribute::Disabled, .. })
            })
            .count();
        assert_eq!(enabled, 31);
        let filled: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                Command::SetAttribute { id, attribute: Attribute::DataSurround, value }
                    if value == "fill" =>
                {
                    Some(id.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(filled, [month_cell(5)]);
    }

    #[test]
    fn month_steps_wrap_the_year() {
        let mut handler = Handler::new(VIEWPORT, days_from_civil(2026, 1, 15), REM);
        press(&mut handler, month_button());
        let commands = press(&mut handler, month_step(1));
        assert_eq!(text_of(&commands, &month_title()).unwrap(), "2025年12月");
        let commands = press(&mut handler, month_step(2));
        let commands_next = press(&mut handler, month_step(2));
        assert_eq!(text_of(&commands, &month_title()).unwrap(), "2026年1月");
        assert_eq!(text_of(&commands_next, &month_title()).unwrap(), "2026年2月");
    }

    #[test]
    fn month_cell_selects_the_date_and_closes_the_modal() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        press(&mut handler, month_button());
        press(&mut handler, month_step(2));
        let commands = press(&mut handler, month_cell(10));
        assert_eq!(handler.base(), days_from_civil(2026, 11, 11));
        assert!(matches!(commands.last(), Some(Command::CloseModal { id }) if *id == modal()));
        assert_eq!(text_of(&commands, &day_title(1)).unwrap(), "11/11(水)");
    }

    fn sample_response(request: u32, status: u16) -> Response {
        let body =
            fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/calendar/data/calendar.json"))
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
        assert_eq!(text_of(&commands, &resource_name(1)).unwrap(), "Studio 1");
        assert_eq!(text_of(&commands, &resource_name(6)).unwrap(), "Studio 2");
        assert_eq!(text_of(&commands, &resource_name(28)).unwrap(), "Studio 4");
    }

    #[test]
    fn view_change_keeps_loaded_resource_names() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        handler.process_fetched(&sample_response(1, 200));
        let commands = press(&mut handler, view_button(1));
        assert_eq!(text_of(&commands, &resource_name(2)).unwrap(), "Studio 2");
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
        day: i32,
        resource: u32,
        start: u32,
        end: u32,
    ) -> crate::calendar::data::Appointment {
        crate::calendar::data::Appointment {
            id,
            cells: vec![crate::calendar::data::Place { day, resource }],
            start,
            end,
            title: format!("t{id}"),
            category: String::from("c"),
            status: String::from("done"),
            note: String::new(),
        }
    }

    fn calendar_with(appointments: Vec<crate::calendar::data::Appointment>) -> Calendar {
        Calendar {
            headers: Default::default(),
            resources: (101..105)
                .map(|id| crate::calendar::data::Resource {
                    id,
                    name: format!("Studio {}", id - 100),
                    accent: String::from("blue"),
                })
                .collect(),
            statuses: vec![crate::calendar::data::Status {
                code:   String::from("done"),
                label:  String::from("D"),
                accent: String::from("green"),
            }],
            shifts: (-3..11)
                .flat_map(|day| {
                    (101..105).map(move |resource| crate::calendar::data::Shift {
                        day: today() + day,
                        resource,
                        person: String::from("p"),
                        open: 540,
                        close: 1200,
                        break_range: None,
                    })
                })
                .collect(),
            appointments,
            complete: true,
        }
    }

    fn card_box(commands: &[Command], n: u32) -> Option<(f32, f32, f32, f32)> {
        let length = |property: StyleProperty| {
            commands.iter().find_map(|command| match command {
                Command::SetStyle { id, property: p, value }
                    if *id == card_item(n) && *p == property =>
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
            appointment(3, base + 10, 101, 600, 660),
            appointment(4, base + 1, 102, 480, 570),
            appointment(5, base - 1, 101, 600, 660),
            appointment(6, base, 999, 600, 660),
        ]));
        let commands = handler.card_commands();
        assert_eq!(shown_count(&commands), 3);
        assert_eq!(card_box(&commands, 1), Some((0.0, 112.0, 40.0, 112.0)));
        assert_eq!(card_box(&commands, 2), Some((40.0, 168.0, 40.0, 112.0)));
        let (x, y, width, height) = card_box(&commands, 3).unwrap();
        assert_eq!((x, y, width, height), (400.0, 0.0, 80.0, 56.0));
        assert_eq!(text_of(&commands, &card_part(3, CardPart::Status)).unwrap(), "D");
        assert_eq!(text_of(&commands, &card_part(1, CardPart::Time)).unwrap(), "10:00–11:00");
        assert_eq!(text_of(&commands, &card_part(2, CardPart::Title)).unwrap(), "t2");
    }

    #[test]
    fn narrowing_the_range_hides_the_surplus_cards_once() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        handler.calendar = Some(calendar_with(vec![
            appointment(1, base, 101, 600, 660),
            appointment(2, base + 1, 101, 600, 660),
            appointment(3, base + 2, 101, 600, 660),
        ]));
        assert_eq!(shown_count(&handler.card_commands()), 3);
        let commands = press(&mut handler, view_button(1));
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
        let commands = press(&mut handler, step_button(4));
        assert_eq!(shown_count(&commands), 1);
        assert_eq!(text_of(&commands, &card_part(1, CardPart::Title)).unwrap(), "t2");
    }

    #[test]
    fn cards_follow_the_column_width_of_the_view() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        handler.calendar = Some(calendar_with(vec![appointment(1, base, 103, 540, 600)]));
        let commands = press(&mut handler, view_button(1));
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
            .flat_map(|a| a.cells.iter())
            .filter(|cell| (0..7).contains(&(cell.day - today())))
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
        press(&mut handler, view_button(1));
        let (_, commands) = handler.process_resize(1000.0, 800.0);
        let column_px = (1000.0 - 2.0 * AXIS_PX) / 4.0;
        assert_eq!(card_box(&commands, 1).unwrap().2, column_px as f32);
    }

    #[test]
    fn rem_constants_match_the_markup() {
        let html = include_str!("../../examples/calendar/index.html");
        let css = include_str!("../../examples/calendar/css/style.css");
        assert!(html.contains(&format!("var(--head-height) repeat({SLOT_COUNT}, {SLOT_REM}rem)")));
        assert!(html.contains(&format!(
            "var(--head-title) var(--head-resource) calc({SLOT_REM}rem * {SLOT_COUNT})"
        )));
        assert!(css.contains(&format!("--column-width:  {COLUMN_MIN_REM}rem")));
        assert!(css.contains(&format!("inline-size: {AXIS_REM}rem")));
        assert!(css.contains("--head-title:    2rem"));
        assert!(css.contains("--head-resource: 3rem"));
        assert_eq!(HEAD_REM, 2.0 + 3.0);
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

    fn drag_fixture() -> (Handler, i32) {
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
        handler.process_canvas(&pointer_down(card_part(n, CardPart::Title), x, y), &state());
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
                if *id == card_item(n) =>
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
        handler.process_canvas(&pointer_down(view_button(1), 10.0, 10.0), &state());
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
        assert_eq!(moved.cells[0].day, base + 2);
        assert_eq!(moved.cells[0].resource, 103);
        assert_eq!((moved.start, moved.end), (540 + 150, 540 + 210));
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
            assert_eq!(moved.start, 540 + expected_slot * 15, "extra {extra}");
        }
    }

    #[test]
    fn drop_near_the_end_of_the_day_keeps_the_card_inside() {
        let (mut handler, _) = drag_fixture();
        let (ox, oy) = grid_origin();
        let grab_y = 14.0;
        handler.process_canvas(
            &pointer_down(card_part(1, CardPart::Title), ox + GRAB_X, oy + 4.0 * SLOT_PX + grab_y),
            &state(),
        );
        drag_to(&mut handler, ox + 5.0, oy + 43.2 * SLOT_PX + grab_y);
        end(&mut handler);
        let moved = &handler.calendar().unwrap().appointments[0];
        assert_eq!((moved.start, moved.end), (20 * 60 - 60, 20 * 60));
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
                (unchanged.cells[0].day, unchanged.cells[0].resource, unchanged.start),
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
        assert_eq!((unchanged.cells[0].day, unchanged.start), (base, 600));
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
        press(&mut handler, view_button(1));
        handler.card_commands();
        let column_px = (VIEWPORT - 2.0 * AXIS_PX) / 4.0;
        let (ox, oy) = grid_origin();
        handler.process_canvas(
            &pointer_down(card_part(1, CardPart::Title), ox + GRAB_X, oy + 4.0 * SLOT_PX + GRAB_Y),
            &state(),
        );
        drag_to(&mut handler, ox + column_px * 3.0 + 5.0, oy + 4.0 * SLOT_PX + GRAB_Y);
        end(&mut handler);
        let moved = &handler.calendar().unwrap().appointments[0];
        assert_eq!((moved.cells[0].day, moved.cells[0].resource), (base, 104));
    }

    #[test]
    fn dropping_on_every_unit_of_every_view_resolves_like_the_column_axis() {
        for (button, days) in [(1, 1u32), (2, 3), (3, 7)] {
            let base = today();
            let mut handler = Handler::new(VIEWPORT, base, REM);
            handler.calendar = Some(calendar_with(vec![appointment(1, base, 101, 600, 660)]));
            press(&mut handler, view_button(button));
            let axis = ColumnAxis::new(days, RESOURCE_COUNT);
            let column_px = column_px(VIEWPORT, handler.view(), REM);
            let (ox, oy) = grid_origin();
            for unit in 0..axis.count() {
                handler.card_commands();
                let (grab_x, grab_y) = (ox + GRAB_X, oy + 4.0 * SLOT_PX + GRAB_Y);
                handler.process_canvas(
                    &pointer_down(card_part(1, CardPart::Title), grab_x, grab_y),
                    &state(),
                );
                drag_to(&mut handler, ox + unit as f64 * column_px + 3.0, grab_y);
                end(&mut handler);
                let moved = &handler.calendar().unwrap().appointments[0];
                let cell = axis.cell(unit).unwrap();
                assert_eq!(moved.cells[0].day, base + cell.day as i32, "view {days} unit {unit}");
                assert_eq!(moved.cells[0].resource, 101 + cell.resource, "view {days} unit {unit}");
                assert_eq!(moved.start, 600);
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
        press(&mut handler, view_button(1));
        assert!(handler.grid().resolve(&bx).is_none());
        press(&mut handler, view_button(3));
        assert_eq!(handler.grid().resolve(&bx).unwrap().cells, [Cell { day: 1, resource: 1 }]);
    }

    fn multi_fixture(view_button_number: u32) -> (Handler, i32) {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        let mut spanning = appointment(1, base, 102, 600, 660);
        spanning.cells.push(crate::calendar::data::Place { day: base, resource: 103 });
        handler.calendar = Some(calendar_with(vec![spanning]));
        press(&mut handler, view_button(view_button_number));
        handler.card_commands();
        (handler, base)
    }

    fn drag_piece(handler: &mut Handler, n: u32, from_unit: u32, to_unit: u32) {
        let column_px = column_px(VIEWPORT, handler.view(), REM);
        let (ox, oy) = grid_origin();
        let y = oy + 4.0 * SLOT_PX + GRAB_Y;
        handler.process_canvas(
            &pointer_down(
                card_part(n, CardPart::Title),
                ox + from_unit as f64 * column_px + GRAB_X,
                y,
            ),
            &state(),
        );
        drag_to(handler, ox + to_unit as f64 * column_px + 5.0, y);
        end(handler);
    }

    fn places(handler: &Handler) -> Vec<(i32, u32)> {
        handler.calendar().unwrap().appointments[0]
            .cells
            .iter()
            .map(|place| (place.day, place.resource))
            .collect()
    }

    #[test]
    fn a_multi_cell_appointment_renders_one_card_per_visible_cell() {
        let (handler, _) = multi_fixture(3);
        let commands = handler.card_commands();
        assert_eq!(shown_count(&commands), 2);
        let (x1, y1, w1, h1) = card_box(&commands, 1).unwrap();
        let (x2, y2, w2, h2) = card_box(&commands, 2).unwrap();
        assert_eq!((x2 - x1, y1, w1, h1), (80.0, y2, w2, h2));
        assert_eq!(
            text_of(&commands, &card_part(1, CardPart::Title)),
            text_of(&commands, &card_part(2, CardPart::Title))
        );
    }

    #[test]
    fn dragging_any_cell_moves_every_cell_by_the_same_unit_difference() {
        let base = today();
        let (mut handler, _) = multi_fixture(3);
        drag_piece(&mut handler, 2, 2, 3);
        assert_eq!(places(&handler), [(base, 103), (base, 104)]);

        let (mut handler, _) = multi_fixture(3);
        drag_piece(&mut handler, 1, 1, 0);
        assert_eq!(places(&handler), [(base, 101), (base, 102)]);
    }

    #[test]
    fn a_shift_past_the_last_resource_continues_into_the_next_day() {
        let base = today();
        let (mut handler, _) = multi_fixture(3);
        drag_piece(&mut handler, 1, 1, 3);
        assert_eq!(places(&handler), [(base, 104), (base + 1, 101)]);
        let commands = handler.card_commands();
        assert_eq!(shown_count(&commands), 2);
    }

    #[test]
    fn a_cell_outside_the_view_still_moves_with_the_others() {
        let base = today();
        let mut handler = Handler::new(VIEWPORT, base, REM);
        let mut crossing = appointment(1, base, 104, 600, 660);
        crossing.cells.push(crate::calendar::data::Place { day: base + 1, resource: 101 });
        handler.calendar = Some(calendar_with(vec![crossing]));
        press(&mut handler, view_button(1));
        let commands = handler.card_commands();
        assert_eq!(shown_count(&commands), 1);
        drag_piece(&mut handler, 1, 3, 0);
        assert_eq!(places(&handler), [(base, 101), (base, 102)]);
    }

    #[test]
    fn dragging_a_multi_cell_card_back_to_its_own_cell_changes_nothing() {
        let (mut handler, _) = multi_fixture(3);
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
            .process_canvas(&pointer_down(card_part(n, CardPart::Title), ox + x, oy + y), &state())
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
        (appointment.start, appointment.end)
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
            [(base, 102), (base, 103), (base, 104), (base + 1, 101), (base + 1, 102)]
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
        let (mut handler, _) = multi_fixture(3);
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
        let (mut handler, _) = multi_fixture(3);
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
            crossing.cells.push(crate::calendar::data::Place { day: base + 1, resource: 101 });
            handler.calendar = Some(calendar_with(vec![crossing]));
            press(&mut handler, view_button(1));
            handler.card_commands();
            handler
        };
        let column_px = (VIEWPORT - 2.0 * AXIS_PX) / 4.0;
        let mut handler = make();
        press_local(&mut handler, 1, 3.0 * column_px + 100.0, 221.0);
        drag_by(&mut handler, 0.0, SLOT_PX);
        end(&mut handler);
        assert_eq!(places(&handler), [(base, 104), (base + 1, 101)]);
        assert_eq!(times(&handler), (600, 675));

        let mut handler = make();
        press_local(&mut handler, 1, 3.0 * column_px + 3.0, 168.0);
        assert_eq!(resize_corner(&handler), Some([Some(true), None]));
        drag_by(&mut handler, -2.0 * column_px, 0.0);
        end(&mut handler);
        assert_eq!(places(&handler), [(base, 102), (base, 103), (base, 104), (base + 1, 101)]);
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
                    if *id == save_button() =>
                {
                    Some(true)
                }
                Command::RemoveAttribute { id, attribute: Attribute::Disabled }
                    if *id == save_button() =>
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
        let before = handler.calendar().unwrap().appointments[index].start;
        let (ox, oy) = grid_origin();
        handler.process_canvas(
            &pointer_down(card_part(1, CardPart::Title), ox + base[0] + 40.0, oy + base[1] + 20.0),
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
        assert_eq!(store.committed_len(), 590);
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
        assert_eq!(text_of(&commands, &resource_name(1)).unwrap(), "Studio 1");
        assert!(shown_count(&commands) > 0);
    }

    #[test]
    fn an_edit_is_pending_until_save_and_survives_a_reload_only_after_it() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let after = handler.calendar().unwrap().appointments[index].start;
        assert_eq!(after, before + 30);
        assert!(handler.dirty());
        assert_eq!(store.pending_len(), 1);
        assert_eq!(committed_calendar(&store).appointments[index].start, before);

        press(&mut handler, save_button());
        assert!(!handler.dirty());
        assert_eq!(store.pending_len(), 0);
        assert_eq!(committed_calendar(&store).appointments[index].start, after);

        let mut reloaded = Handler::new(VIEWPORT, today(), REM);
        reloaded.attach(Box::new(store.clone()));
        assert_eq!(reloaded.calendar().unwrap().appointments[index].start, after);
    }

    #[test]
    fn an_unsaved_edit_is_gone_after_a_reload() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        store.0.borrow_mut().pending.clear();
        let mut reloaded = Handler::new(VIEWPORT, today(), REM);
        reloaded.attach(Box::new(store.clone()));
        assert_eq!(reloaded.calendar().unwrap().appointments[index].start, before);
    }

    #[test]
    fn discard_restores_the_saved_state_and_redraws() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let commands = handler.discard_commands();
        assert_eq!(handler.calendar().unwrap().appointments[index].start, before);
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
        let shift = calendar.shifts.iter().find(|s| s.day == today()).unwrap().clone();
        let resource =
            calendar.resources.iter().position(|r| r.id == shift.resource).unwrap() as u32;
        let kinds: Vec<BandKind> = handler.bands.borrow().iter().map(|band| band.kind).collect();
        assert_eq!(kinds, [BandKind::Closed, BandKind::Closed, BandKind::Break]);
        let hit = |slot| handler.band_hit(band_point(slot));
        assert_eq!(hit(0.5), Some(BandKind::Closed));
        assert_eq!(hit(1.9), Some(BandKind::Closed));
        assert_eq!(hit(5.0), None);
        assert_eq!(hit(14.5), Some(BandKind::Break));
        assert_eq!(hit(38.5), Some(BandKind::Closed));
        assert_eq!(hit(43.5), Some(BandKind::Closed));
        assert_eq!(text_of(&commands, &resource_person(resource + 1)).unwrap(), shift.person);
        let shown = commands
            .iter()
            .filter(|command| {
                matches!(command, Command::RemoveAttribute { id, attribute: Attribute::Hidden }
                    if *id == band_item(3))
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
        press(&mut handler, view_button(1));
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
        assert_eq!(value_of(&commands, &edit_field(EditField::Title)).unwrap(), appointment.title);
        assert_eq!(value_of(&commands, &edit_field(EditField::Date)).unwrap(), format_iso(base));
        assert_eq!(value_of(&commands, &edit_field(EditField::Resource)).unwrap(), "101");
        assert_eq!(value_of(&commands, &edit_field(EditField::Start)).unwrap(), "10:00");
        assert!(matches!(commands.last(), Some(Command::ShowModal { id }) if *id == modal()));
        assert!(!handler.dirty());
    }

    fn tap_empty(handler: &mut Handler, column: f64, slot: f64) -> Vec<Command> {
        let (ox, oy) = grid_origin();
        let (x, y) = (
            ox + (column + 0.5) * column_px(VIEWPORT, handler.view(), REM),
            oy + (slot + 0.5) * SLOT_PX,
        );
        handler.process_canvas(&pointer_down(surface(), x, y), &state());
        handler.process_gesture(&Gesture::Tap, &state(), None).1
    }

    fn form_query(title: &str, date: &str, resource: u32, start: &str, end: &str) -> String {
        format!(
            "title={title}&category=k&status=done&date={date}&resource={resource}&start={start}&end={end}&note=n"
        )
    }

    fn submit(handler: &mut Handler, query: &str) -> Vec<Command> {
        let event = CanvasEvent {
            event_type: EventType::Submit,
            value: String::from(query),
            ..click(edit_form(), 0.0, 0.0)
        };
        handler.process_canvas(&event, &state()).1
    }

    #[test]
    fn tapping_an_empty_cell_opens_a_new_form_at_that_slot() {
        let (mut handler, base) = drag_fixture();
        let commands = tap_empty(&mut handler, 2.0, 4.0);
        assert_eq!(value_of(&commands, &edit_field(EditField::Title)).unwrap(), "");
        assert_eq!(value_of(&commands, &edit_field(EditField::Date)).unwrap(), format_iso(base));
        assert_eq!(value_of(&commands, &edit_field(EditField::Resource)).unwrap(), "103");
        assert_eq!(value_of(&commands, &edit_field(EditField::Start)).unwrap(), "10:00");
        assert_eq!(value_of(&commands, &edit_field(EditField::End)).unwrap(), "11:00");
        assert!(matches!(commands.last(), Some(Command::ShowModal { id }) if *id == modal()));
        assert!(matches!(handler.editing, Some(Editing::New(_))));
    }

    fn drag_create(handler: &mut Handler, from: (f64, f64), to: (f64, f64)) -> Vec<Command> {
        let (ox, oy) = grid_origin();
        let column = column_px(VIEWPORT, handler.view(), REM);
        let at = |(c, s): (f64, f64)| (ox + (c + 0.5) * column, oy + (s + 0.5) * SLOT_PX);
        let (x, y) = at(from);
        handler.process_canvas(&pointer_down(surface(), x, y), &state());
        let (x, y) = at(to);
        let mid = drag_to(handler, x, y);
        assert!(
            mid.iter().any(|c| matches!(c, Command::SetStyle { id, .. } if *id == preview_item()))
        );
        end(handler)
    }

    #[test]
    fn dragging_over_empty_cells_opens_a_form_for_the_range() {
        let (mut handler, base) = drag_fixture();
        let commands = drag_create(&mut handler, (2.0, 8.0), (2.0, 11.0));
        assert_eq!(value_of(&commands, &edit_field(EditField::Start)).unwrap(), "11:00");
        assert_eq!(value_of(&commands, &edit_field(EditField::End)).unwrap(), "12:00");
        assert_eq!(value_of(&commands, &edit_field(EditField::Resource)).unwrap(), "103");
        assert_eq!(value_of(&commands, &edit_field(EditField::Date)).unwrap(), format_iso(base));
        assert!(commands.iter().any(|c| matches!(c,
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. } if *id == preview_item())));
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
        assert_eq!(cells[3], Place { day: base + 1, resource: 102 });
        let query = form_query("Span", &format_iso(base), 103, "12:30", "14:30")
            .replace("done", "scheduled");
        handler.calendar.as_mut().unwrap().statuses[0].code = String::from("scheduled");
        submit(&mut handler, &query);
        let added = handler.calendar().unwrap().appointments.last().unwrap();
        assert_eq!(added.cells.len(), 4);
        assert_eq!((added.start, added.end), (750, 870));
    }

    #[test]
    fn a_small_drag_in_one_slot_behaves_like_a_tap() {
        let (mut handler, _) = drag_fixture();
        let commands = drag_create(&mut handler, (2.0, 8.0), (2.0, 8.0));
        assert_eq!(value_of(&commands, &edit_field(EditField::End)).unwrap(), "12:00");
    }

    #[test]
    fn cancelling_a_create_drag_hides_the_preview() {
        let (mut handler, _) = drag_fixture();
        let (ox, oy) = grid_origin();
        handler.process_canvas(&pointer_down(surface(), ox + 10.0, oy + 10.0), &state());
        drag_to(&mut handler, ox + 10.0, oy + 3.0 * SLOT_PX);
        let (_, commands) = handler.process_gesture(&Gesture::DragCancel, &state(), None);
        assert!(commands.iter().any(|c| matches!(c,
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. } if *id == preview_item())));
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
        let (_, commands) = handler.process_canvas(&input(zoom_input(), "3"), &state());
        let axis = String::from("var(--head-height) repeat(44, 3rem)");
        assert_eq!(
            grid_rows(&commands),
            [
                (time_axis(1), axis.clone()),
                (time_axis(2), axis),
                (surface(), String::from("var(--head-title) var(--head-resource) calc(3rem * 44)")),
            ]
        );
        let after = handler.placed.borrow()[0].base[1];
        assert_eq!(after, before / SLOT_REM * 3.0);
        assert!(handler.process_canvas(&input(zoom_input(), "3"), &state()).1.is_empty());
        handler.process_canvas(&input(zoom_input(), "9"), &state());
        assert_eq!(handler.slot_rem, ZOOM_MAX);
    }

    #[test]
    fn a_drag_after_zooming_still_snaps_to_slots() {
        let (mut handler, _) = drag_fixture();
        handler.process_canvas(&input(zoom_input(), "2.5"), &state());
        let (ox, oy) = grid_origin();
        let slot = 2.5 * REM;
        handler
            .process_canvas(&pointer_down(surface(), ox + 10.0, oy + 5.0 * slot + 3.0), &state());
        drag_to(&mut handler, ox + 10.0, oy + 7.0 * slot + 3.0);
        let commands = end(&mut handler);
        assert_eq!(value_of(&commands, &edit_field(EditField::Start)).unwrap(), "10:15");
        assert_eq!(value_of(&commands, &edit_field(EditField::End)).unwrap(), "11:00");
    }

    #[test]
    fn tapping_the_header_does_not_open_a_form() {
        let (mut handler, _) = drag_fixture();
        let (ox, oy) = grid_origin();
        handler.process_canvas(&pointer_down(surface(), ox + 10.0, oy - 5.0), &state());
        let (_, commands) = handler.process_gesture(&Gesture::Tap, &state(), None);
        assert!(commands.is_empty());
    }

    #[test]
    fn submitting_a_new_form_adds_a_persisted_appointment() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let before = handler.calendar().unwrap().appointments.len();
        let shift = handler.calendar().unwrap().shifts[0].clone();
        tap_empty(&mut handler, 0.0, 0.0);
        let date = format_iso(shift.day);
        let (start, end) = (format_hhmm(shift.open), format_hhmm(shift.open + 60));
        let query =
            form_query("Neo", &date, shift.resource, &start, &end).replace("done", "scheduled");
        let commands = submit(&mut handler, &query);
        let calendar = handler.calendar().unwrap();
        assert_eq!(calendar.appointments.len(), before + 1);
        let added = calendar.appointments.last().unwrap();
        assert_eq!(
            (added.title.as_str(), added.start, added.end),
            ("Neo", shift.open, shift.open + 60)
        );
        assert!(handler.dirty());
        assert!(commands.iter().any(|c| matches!(c, Command::CloseModal { .. })));
        assert!(store.pending_len() >= 2);
        assert!(handler.editing.is_none());
    }

    #[test]
    fn a_form_missing_a_field_is_rejected() {
        let (mut handler, base) = drag_fixture();
        tap_empty(&mut handler, 0.0, 0.0);
        let query = form_query("x", &format_iso(base), 101, "09:30", "10:30");
        let missing = query.replace("&note=n", "");
        let commands = submit(&mut handler, &missing);
        assert_eq!(text_of(&commands, &edit_message()).unwrap(), "入力を読み取れません");
        assert_eq!(handler.calendar().unwrap().appointments.len(), 2);
        assert!(handler.editing.is_some());
    }

    #[test]
    fn a_form_outside_the_open_hours_is_rejected_but_an_unchanged_time_is_not() {
        let (mut handler, base) = drag_fixture();
        tap_empty(&mut handler, 0.0, 0.0);
        for shift in &mut handler.calendar.as_mut().unwrap().shifts {
            shift.open = 660;
        }
        let early = form_query("x", &format_iso(base), 101, "09:30", "10:30");
        let commands = submit(&mut handler, &early);
        assert_eq!(text_of(&commands, &edit_message()).unwrap(), "営業時間外です");
        assert_eq!(handler.calendar().unwrap().appointments.len(), 2);
        grab(&mut handler, 1);
        handler.process_gesture(&Gesture::Tap, &state(), None);
        let same = form_query("kept", &format_iso(base), 101, "10:00", "11:00");
        submit(&mut handler, &same);
        assert_eq!(handler.calendar().unwrap().appointments[0].title, "kept");
    }

    #[test]
    fn a_drag_before_the_open_hours_is_refused() {
        let (mut handler, _) = drag_fixture();
        for shift in &mut handler.calendar.as_mut().unwrap().shifts {
            shift.open = 660;
        }
        let before = handler.calendar().unwrap().appointments[0].clone();
        let (x, y) = grab(&mut handler, 1);
        drag_to(&mut handler, x, y + 2.0 * SLOT_PX);
        end(&mut handler);
        assert_eq!(handler.calendar().unwrap().appointments[0].start, before.start);
        assert!(!handler.dirty());
    }

    #[test]
    fn clicking_the_backdrop_closes_the_modal() {
        let (mut handler, _) = drag_fixture();
        tap_empty(&mut handler, 0.0, 0.0);
        let commands = press(&mut handler, modal());
        assert!(matches!(commands.as_slice(), [Command::CloseModal { .. }]));
        assert!(handler.editing.is_none());
    }

    #[test]
    fn month_button_shows_the_month_form_again() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        let commands = press(&mut handler, month_button());
        assert!(commands.iter().any(|c| matches!(c,
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. } if *id == edit_form())));
    }

    #[test]
    fn reload_button_discards_pending_edits() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        let (index, before) = drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        let commands = press(&mut handler, reload_button());
        assert_eq!(handler.calendar().unwrap().appointments[index].start, before);
        assert_eq!(store.pending_len(), 0);
        assert_eq!(save_disabled(&commands), [true]);
    }

    #[test]
    fn a_failed_save_reports_the_error_and_stays_dirty() {
        let (mut handler, store) = with_store();
        handler.process_fetched(&sample_response(1, 200));
        drag_first_card_by(&mut handler, 2.0 * SLOT_PX);
        store.failing(true);
        let commands = press(&mut handler, save_button());
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::FileStore(FileStoreError::QuotaExceeded(_)) }]
        ));
        assert!(handler.dirty());
        store.failing(false);
        press(&mut handler, save_button());
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
        store
            .0
            .borrow_mut()
            .committed
            .insert(crate::calendar::store::appointment_key(5001).unwrap(), b"{".to_vec());
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        handler.attach(Box::new(store.clone()));
        let (_, commands) = handler.initial_draw();
        assert!(commands.iter().any(|command| matches!(
            command,
            Command::Error { error: Error::Data(DataError::Parse(_)) }
        )));
        assert!(!commands.iter().any(|command| matches!(command, Command::Fetch { .. })));
    }

    #[test]
    fn save_and_discard_without_a_store_do_nothing() {
        let mut handler = Handler::new(VIEWPORT, today(), REM);
        assert!(press(&mut handler, save_button()).is_empty());
        assert!(handler.discard_commands().is_empty());
    }
}
