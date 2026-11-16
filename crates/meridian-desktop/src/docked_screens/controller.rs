use super::*;
use meridian_core::station_services::CommodityId;
#[derive(Debug)]
pub struct DockedScreens {
    pub(super) screen: Screen,
    pub(super) selected_world: SystemId,
    pub(super) galaxy: u8,
    pub(super) remembered: [u16; 8],
    pub(super) local: bool,
    pub(super) map_colour: u8,
    pub(super) ship: String,
    pub(super) baseline: Option<String>,
    pub(super) compare: bool,
    pub(super) commodity: CommodityId,
    pub(super) sell: bool,
    pub(super) body_index: usize,
    pub(super) equipment_index: usize,
    pub(super) option_index: usize,
    pub(super) options: bool,
    pub(super) detail: bool,
    pub(super) scroll: usize,
    pub(super) searching: bool,
    pub(super) query: String,
    pub(super) search_index: usize,
    pub(super) orientation: Orientation,
    pub(super) rotation_paused: bool,
    pub(super) axis: u8,
    pub(super) hovered: Option<Rect>,
    pub(super) ui_revision: u64,
}
impl DockedScreens {
    pub fn new(current: SystemId, hull: &str) -> Self {
        let mut remembered = [0; 8];
        if usize::from(current.galaxy) < 8 {
            remembered[usize::from(current.galaxy)] = current.system;
        }
        Self {
            screen: Screen::Galaxy,
            selected_world: current,
            galaxy: current.galaxy.min(7),
            remembered,
            local: false,
            map_colour: 0,
            ship: hull.into(),
            baseline: None,
            compare: true,
            commodity: CommodityId::Food,
            sell: false,
            body_index: 0,
            equipment_index: 0,
            option_index: 0,
            options: false,
            detail: false,
            scroll: 0,
            searching: false,
            query: String::new(),
            search_index: 0,
            orientation: Orientation::default(),
            rotation_paused: false,
            axis: 2,
            hovered: None,
            ui_revision: 0,
        }
    }
    pub fn screen(&self) -> Screen {
        self.screen
    }
    pub fn selected_system(&self) -> SystemId {
        self.selected_world
    }
    pub fn selected_ship(&self) -> &str {
        &self.ship
    }
    pub fn open(&mut self, screen: Screen) {
        self.screen = screen;
        self.searching = false;
        self.query.clear();
        self.detail = false;
        self.options = false;
        self.scroll = 0;
        self.bump();
    }
    fn bump(&mut self) {
        self.ui_revision = self.ui_revision.wrapping_add(1);
        self.hovered = None;
    }
    pub fn frame(&self, context: &Context<'_>, viewport: Viewport) -> Frame {
        presentation::project(self, context, viewport)
    }
    pub fn advance(&mut self, seconds: f64) {
        if self.screen != Screen::Showroom
            || self.rotation_paused
            || !seconds.is_finite()
            || seconds < 0.0
        {
            return;
        }
        self.turn(seconds * 0.12, seconds * 0.06, seconds * 0.4);
        // Animation changes no selection/authority and does not invalidate a painted click.
    }
    fn turn(&mut self, pitch: f64, roll: f64, yaw: f64) {
        let t = std::f64::consts::TAU;
        self.orientation.pitch = (self.orientation.pitch + pitch).rem_euclid(t);
        self.orientation.roll = (self.orientation.roll + roll).rem_euclid(t);
        self.orientation.yaw = (self.orientation.yaw + yaw).rem_euclid(t);
    }
    fn select_world(&mut self, id: SystemId, context: &Context<'_>) {
        if context.catalogue.world(id).is_some() {
            self.selected_world = id;
            self.body_index = 0;
            self.galaxy = id.galaxy;
            self.remembered[usize::from(id.galaxy)] = id.system;
            self.scroll = 0;
        }
    }
    pub fn handle(&mut self, input: Input, painted: &Frame, context: &Context<'_>) -> Outcome {
        if matches!(input, Input::FocusLost) {
            self.searching = false;
            self.rotation_paused = true;
            self.bump();
            return Outcome::Updated;
        }
        if context.stamp != painted.stamp
            || painted.ui_revision != self.ui_revision
            || painted.screen != self.screen
        {
            return Outcome::Refused("Screen changed; review the current values again".into());
        }
        if context.world.current_dock_context().is_none() {
            return Outcome::Refused("Dock at a station to use these screens".into());
        }
        if let Input::PointerMoved(p) = input {
            self.hovered = painted
                .hits
                .iter()
                .rev()
                .find(|h| h.bounds.contains(p))
                .map(|h| h.bounds);
            return Outcome::Updated;
        }
        let outcome = match input {
            Input::Click(p) => self.click_input(p, painted, context),
            Input::Key(key) => {
                if let Some(review) = context.review {
                    match key {
                        Key::Confirm => self.review_confirm(context),
                        Key::Back => Outcome::Request(Request::CancelReview(review.token)),
                        _ => Outcome::Ignored,
                    }
                } else {
                    self.key(key, context)
                }
            }
            Input::Text(text) => self.text_input(text, context),
            Input::Scroll { point, rows } => self.scroll_input(point, rows, painted, context),
            Input::Drag { from, to, roll } => self.drag_input(from, to, roll, painted, context),
            Input::PointerMoved(_) | Input::FocusLost => Outcome::Ignored,
        };
        if !matches!(outcome, Outcome::Ignored | Outcome::Refused(_)) {
            self.bump();
        }
        outcome
    }
    fn click_input(&mut self, p: [f64; 2], painted: &Frame, context: &Context<'_>) -> Outcome {
        if let Some(hit) = painted.hits.iter().rev().find(|h| h.bounds.contains(p)) {
            if !hit.enabled {
                Outcome::Refused(
                    hit.refusal
                        .clone()
                        .unwrap_or_else(|| "Action unavailable".into()),
                )
            } else {
                self.control(&hit.control, context)
            }
        } else {
            if self.searching {
                self.searching = false;
            } else if self.screen == Screen::Galaxy {
                let nav = context.world.snapshot().navigation;
                let local = nav
                    .and_then(|n| context.catalogue.world(n.system).map(|w| (w, n.fuel_units)))
                    .filter(|_| self.local);
                let layout =
                    ChartLayout::new(painted.viewport, context.catalogue, self.galaxy, local);
                if layout.plot.contains(p) {
                    if let Some(marker) = painted.markers.iter().min_by(|a, b| {
                        let d = |m: &Marker| (m.point[0] - p[0]).hypot(m.point[1] - p[1]);
                        d(a).total_cmp(&d(b))
                            .then(a.system.system.cmp(&b.system.system))
                    }) {
                        self.select_world(marker.system, context);
                    }
                }
            }
            Outcome::Updated
        }
    }
    fn text_input(&mut self, text: String, context: &Context<'_>) -> Outcome {
        if context.review.is_some() {
            Outcome::Ignored
        } else if self.searching {
            for c in text.chars().filter(|c| !c.is_control()) {
                if self.query.chars().count() < 64 {
                    self.query.push(c);
                }
            }
            self.search_index = 0;
            self.scroll = 0;
            if self.screen == Screen::Showroom {
                if let Some(first) = context.catalogue.search_ships(&self.query).first() {
                    self.ship = first.id.into();
                }
            }
            Outcome::Updated
        } else {
            Outcome::Ignored
        }
    }
    fn scroll_input(
        &mut self,
        point: [f64; 2],
        rows: i32,
        painted: &Frame,
        context: &Context<'_>,
    ) -> Outcome {
        if context.review.is_some() {
            Outcome::Ignored
        } else if !painted.viewport.body().contains(point) {
            Outcome::Ignored
        } else if self.screen == Screen::System
            || (self.detail && self.screen == Screen::Showroom)
            || (self.screen == Screen::Galaxy && {
                let nav = context.world.snapshot().navigation;
                let local = nav
                    .and_then(|n| context.catalogue.world(n.system).map(|w| (w, n.fuel_units)))
                    .filter(|_| self.local);
                ChartLayout::new(painted.viewport, context.catalogue, self.galaxy, local)
                    .details
                    .contains(point)
            })
            || (self.screen == Screen::Equipment
                && !self.options
                && painted.viewport.panes().1.contains(point))
        {
            self.scroll = moved(self.scroll, rows, context.catalogue.specifications.len());
            Outcome::Updated
        } else {
            self.move_selection(rows, context);
            Outcome::Updated
        }
    }
    fn drag_input(
        &mut self,
        from: [f64; 2],
        to: [f64; 2],
        roll: bool,
        painted: &Frame,
        context: &Context<'_>,
    ) -> Outcome {
        if context.review.is_some()
            || self.searching
            || self.screen != Screen::Showroom
            || !painted
                .model_bounds
                .is_some_and(|r| r.contains(from) && r.contains(to))
        {
            Outcome::Ignored
        } else {
            let dx = (to[0] - from[0]).clamp(-256.0, 256.0) * 0.01;
            let dy = (to[1] - from[1]).clamp(-256.0, 256.0) * 0.01;
            self.rotation_paused = true;
            if roll {
                self.turn(0.0, dx, 0.0);
            } else {
                self.turn(dy, 0.0, dx);
            }
            Outcome::Updated
        }
    }
    fn control(&mut self, control: &Control, c: &Context<'_>) -> Outcome {
        if c.review.is_some() && !matches!(control, Control::ReviewConfirm | Control::ReviewCancel)
        {
            return Outcome::Ignored;
        }
        match control {
            Control::Open(s) => {
                self.open(*s);
                Outcome::Updated
            }
            Control::SelectSystem(id) => {
                self.select_world(*id, c);
                self.searching = false;
                Outcome::Updated
            }
            Control::SelectShip(id) => {
                self.ship = id.clone();
                self.searching = false;
                self.detail = false;
                self.scroll = 0;
                Outcome::Updated
            }
            Control::SelectCommodity(id) => {
                self.commodity = *id;
                Outcome::Updated
            }
            Control::SelectEquipment(index) => {
                self.equipment_index = *index;
                self.option_index = 0;
                self.options = false;
                Outcome::Updated
            }
            Control::SelectSlot(index) => {
                if let Some(slot) = c.equipment.and_then(|e| e.slots.get(*index)) {
                    Outcome::Request(Request::InspectEquipmentSlot(slot.id.clone()))
                } else {
                    Outcome::Refused("Slot is unavailable".into())
                }
            }
            Control::SelectOption(index) => {
                self.option_index = *index;
                Outcome::Updated
            }
            Control::Key(key) => {
                if self.searching && *key != Key::ClearSearch {
                    self.searching = false;
                }
                self.key(*key, c)
            }
            Control::Galaxy(g) => {
                self.change_galaxy(*g, c);
                Outcome::Updated
            }
            Control::Search => {
                self.searching = true;
                self.search_index = 0;
                Outcome::Updated
            }
            Control::TradeMode(sell) => {
                self.key(if *sell { Key::SellMode } else { Key::BuyMode }, c)
            }
            Control::SelectBody(id) => self.body_route(*id, c),
        }
    }
    fn review_confirm(&self, c: &Context<'_>) -> Outcome {
        c.review.map_or(Outcome::Ignored, |r| {
            if r.can_confirm && !c.paused {
                Outcome::Request(Request::ConfirmReview(r.token))
            } else {
                Outcome::Refused(
                    r.refusal
                        .clone()
                        .unwrap_or_else(|| "Confirmation unavailable while paused".into()),
                )
            }
        })
    }
    fn change_galaxy(&mut self, galaxy: u8, c: &Context<'_>) {
        self.local = false;
        self.query.clear();
        self.search_index = 0;
        self.select_world(
            SystemId {
                galaxy: galaxy.min(7),
                system: self.remembered[usize::from(galaxy.min(7))],
            },
            c,
        );
    }
    fn move_selection(&mut self, amount: i32, c: &Context<'_>) {
        if self.searching && self.screen == Screen::Galaxy {
            let rows = c.catalogue.search_worlds(self.galaxy, &self.query);
            self.search_index = moved(self.search_index, amount, rows.len());
            return;
        }
        match self.screen {
            Screen::Galaxy | Screen::System => {
                let id = SystemId {
                    galaxy: self.galaxy,
                    system: moved(usize::from(self.selected_world.system), amount, 256) as u16,
                };
                self.select_world(id, c);
            }
            Screen::Showroom if self.detail => {
                self.scroll = moved(self.scroll, amount, c.catalogue.specifications.len());
            }
            Screen::Showroom => {
                let rows = c.catalogue.search_ships(&self.query);
                if let Some(row) = rows.get(moved(
                    rows.iter().position(|o| o.id == self.ship).unwrap_or(0),
                    amount,
                    rows.len(),
                )) {
                    self.ship = row.id.into();
                    self.scroll = 0;
                }
            }
            Screen::Trading => {
                if let Ok(order) = CommodityId::display_order() {
                    let index = order
                        .iter()
                        .position(|id| *id == self.commodity)
                        .unwrap_or(0);
                    self.commodity = order[moved(index, amount, order.len())];
                }
            }
            Screen::Equipment => {
                if self.detail && !self.options {
                    self.scroll = moved(self.scroll, amount, 128);
                    return;
                }
                if let Some(panel) = c.equipment {
                    if self.options {
                        let count = panel
                            .rows
                            .get(self.equipment_index)
                            .map_or(0, |r| r.options.len());
                        self.option_index = moved(self.option_index, amount, count);
                    } else {
                        self.equipment_index =
                            moved(self.equipment_index, amount, panel.rows.len());
                    }
                }
            }
        }
    }
    fn key(&mut self, key: Key, c: &Context<'_>) -> Outcome {
        if self.searching {
            return self.search_key(key, c);
        }
        match key {
            Key::Back => {
                if self.options {
                    self.options = false;
                } else if self.detail {
                    self.detail = false;
                } else {
                    return Outcome::Request(Request::Close);
                }
                Outcome::Updated
            }
            Key::NextTab | Key::PreviousTab => {
                let index = Screen::ALL
                    .iter()
                    .position(|s| *s == self.screen)
                    .unwrap_or(0);
                self.open(
                    Screen::ALL[(index
                        + if key == Key::NextTab {
                            1
                        } else {
                            Screen::ALL.len() - 1
                        })
                        % Screen::ALL.len()],
                );
                Outcome::Updated
            }
            Key::ClearSearch => {
                self.query.clear();
                self.search_index = 0;
                Outcome::Updated
            }
            _ => match self.screen {
                Screen::Galaxy | Screen::System => self.map_key(key, c),
                Screen::Showroom => self.ship_key(key, c),
                Screen::Trading => self.market_key(key, c),
                Screen::Equipment => self.equipment_key(key, c),
            },
        }
    }
    fn search_key(&mut self, key: Key, c: &Context<'_>) -> Outcome {
        match key {
            Key::Back => self.searching = false,
            Key::Confirm => {
                if self.screen == Screen::Galaxy {
                    if let Some(w) = c
                        .catalogue
                        .search_worlds(self.galaxy, &self.query)
                        .get(self.search_index)
                    {
                        self.select_world(w.id, c);
                    }
                }
                self.searching = false;
            }
            Key::Erase => {
                self.query.pop();
                self.search_index = 0;
            }
            Key::ClearSearch => {
                self.query.clear();
                self.search_index = 0;
            }
            Key::Previous | Key::Next | Key::PagePrevious | Key::PageNext => {
                self.move_selection(selection_step(key), c)
            }
            _ => return Outcome::Ignored,
        }
        Outcome::Updated
    }
    fn map_key(&mut self, key: Key, c: &Context<'_>) -> Outcome {
        match key {
            Key::Previous | Key::Next => self.move_selection(selection_step(key), c),
            Key::PagePrevious | Key::PageNext => {
                self.scroll = moved(self.scroll, selection_step(key), 128)
            }
            Key::Confirm => {
                if self.screen == Screen::Galaxy {
                    self.open(Screen::System);
                } else {
                    return self.route(c);
                }
            }
            Key::Search if self.screen == Screen::Galaxy => {
                self.searching = true;
                self.search_index = 0;
            }
            Key::MapColour if self.screen == Screen::Galaxy => {
                self.map_colour = (self.map_colour + 1) % 3
            }
            Key::LocalChart if self.screen == Screen::Galaxy => {
                let Some(nav) = c.world.snapshot().navigation else {
                    return Outcome::Refused("Commander navigation unavailable".into());
                };
                let local = !self.local;
                if local {
                    self.change_galaxy(nav.system.galaxy, c);
                }
                self.local = local;
            }
            Key::CurrentLocation => {
                if let Some(nav) = c.world.snapshot().navigation {
                    self.select_world(nav.system, c);
                    self.query.clear();
                }
            }
            Key::NextGalaxy => self.change_galaxy(self.galaxy.saturating_add(1).min(7), c),
            Key::PreviousGalaxy => self.change_galaxy(self.galaxy.saturating_sub(1), c),
            Key::Route => return self.route(c),
            Key::NextBody | Key::PreviousBody if self.screen == Screen::System => {
                let count = c
                    .navigation_bodies
                    .iter()
                    .filter(|b| b.id.system == self.selected_world)
                    .count();
                self.body_index = moved(
                    self.body_index,
                    if key == Key::NextBody { 1 } else { -1 },
                    count,
                );
            }
            Key::BodyRoute if self.screen == Screen::System => {
                return c
                    .navigation_bodies
                    .iter()
                    .filter(|b| b.id.system == self.selected_world)
                    .nth(self.body_index)
                    .map_or_else(
                        || Outcome::Refused("Select a local navigation body".into()),
                        |body| self.body_route(body.id, c),
                    );
            }
            _ => return Outcome::Ignored,
        }
        Outcome::Updated
    }
    fn ship_key(&mut self, key: Key, c: &Context<'_>) -> Outcome {
        match key {
            Key::Previous | Key::Next | Key::PagePrevious | Key::PageNext => {
                self.move_selection(selection_step(key), c)
            }
            Key::Confirm | Key::Details => {
                self.detail = !self.detail;
                self.scroll = 0;
            }
            Key::Exchange => return self.exchange(c),
            Key::Search => {
                self.searching = true;
                self.search_index = 0;
            }
            Key::Compare => self.compare = !self.compare,
            Key::PinBaseline => self.baseline = Some(self.ship.clone()),
            Key::CurrentShip => self.baseline = None,
            _ => return self.model_key(key),
        }
        Outcome::Updated
    }
    fn model_key(&mut self, key: Key) -> Outcome {
        match key {
            Key::Rotate => self.rotation_paused = !self.rotation_paused,
            Key::NextAxis => self.axis = (self.axis + 1) % 3,
            Key::TurnLeft | Key::TurnRight => {
                let step = if key == Key::TurnLeft {
                    -std::f64::consts::PI / 12.0
                } else {
                    std::f64::consts::PI / 12.0
                };
                self.rotation_paused = true;
                match self.axis {
                    0 => self.turn(step, 0.0, 0.0),
                    1 => self.turn(0.0, step, 0.0),
                    _ => self.turn(0.0, 0.0, step),
                }
            }
            Key::ResetView => {
                self.orientation = Orientation::default();
                self.rotation_paused = true;
            }
            _ => return Outcome::Ignored,
        }
        Outcome::Updated
    }
    fn market_key(&mut self, key: Key, c: &Context<'_>) -> Outcome {
        match key {
            Key::Previous | Key::Next | Key::PagePrevious | Key::PageNext => {
                self.move_selection(selection_step(key), c);
                Outcome::Updated
            }
            Key::BuyMode | Key::SellMode => {
                self.sell = key == Key::SellMode;
                Outcome::Updated
            }
            Key::Confirm => self.commerce(!self.sell, c),
            Key::Buy => self.commerce(true, c),
            Key::Sell => self.commerce(false, c),
            _ => Outcome::Ignored,
        }
    }
    fn equipment_key(&mut self, key: Key, c: &Context<'_>) -> Outcome {
        match key {
            Key::Previous | Key::Next | Key::PagePrevious | Key::PageNext => {
                self.move_selection(selection_step(key), c)
            }
            Key::Confirm => return self.equipment(EquipmentOperation::Buy, c),
            Key::Fit => return self.equipment(EquipmentOperation::Fit, c),
            Key::Store => return self.equipment(EquipmentOperation::Store, c),
            Key::Details => {
                self.detail = !self.detail;
                self.scroll = 0;
            }
            Key::Options => {
                self.options = !self.options;
                self.option_index = 0;
            }
            Key::NextSlot => return self.next_slot(c),
            _ => return Outcome::Ignored,
        }
        Outcome::Updated
    }
    fn next_slot(&self, c: &Context<'_>) -> Outcome {
        let Some(panel) = c.equipment else {
            return Outcome::Refused("Equipment service unavailable".into());
        };
        let current = panel
            .slots
            .iter()
            .position(|slot| slot.id == panel.selected_slot)
            .unwrap_or(0);
        for step in 1..=panel.slots.len() {
            if let Some(slot) = panel.slots.get((current + step) % panel.slots.len()) {
                if slot.available {
                    return Outcome::Request(Request::InspectEquipmentSlot(slot.id.clone()));
                }
            }
        }
        Outcome::Refused("No equipment slots available".into())
    }
    fn body_route(&self, id: NavigationBodyId, c: &Context<'_>) -> Outcome {
        if c.paused {
            return Outcome::Refused("Resume before setting a navigation target".into());
        }
        match c
            .navigation_bodies
            .iter()
            .find(|b| b.id == id && b.id.system == self.selected_world)
        {
            Some(body)
                if body.can_target && body.position_world.into_iter().all(f64::is_finite) =>
            {
                Outcome::Request(Request::ReviewBodyRoute(id))
            }
            Some(body) => Outcome::Refused(
                body.refusal
                    .clone()
                    .unwrap_or_else(|| "This body is not available for navigation".into()),
            ),
            None => Outcome::Refused("Navigation body is no longer available".into()),
        }
    }
    fn commerce(&self, buy: bool, c: &Context<'_>) -> Outcome {
        if c.paused {
            return Outcome::Refused("Resume before trading".into());
        }
        if c.world.commodity_quote(self.commodity).is_none() {
            return Outcome::Refused("Market service unavailable at this berth".into());
        }
        // Reserved cargo may be the actual selected T04 sale: only World decides admission.
        Outcome::Request(Request::ReviewCommodity {
            commodity: self.commodity,
            buy,
        })
    }
    fn route(&self, c: &Context<'_>) -> Outcome {
        if c.paused {
            return Outcome::Refused("Resume before reviewing travel".into());
        }
        if !c.route_targets.contains(&self.selected_world) {
            return Outcome::Refused(
                "Station scene and route admission are not available for this selection".into(),
            );
        }
        Outcome::Request(Request::ReviewRoute(self.selected_world))
    }
    fn exchange(&self, c: &Context<'_>) -> Outcome {
        if c.paused {
            return Outcome::Refused("Resume before exchanging ships".into());
        }
        if !c
            .catalogue
            .search_ships(&self.query)
            .iter()
            .any(|o| o.id == self.ship)
        {
            return Outcome::Refused("Select a visible matching ship".into());
        }
        let Some(offer) = c.exchanges.iter().find(|o| o.object == self.ship) else {
            return Outcome::Refused("No admitted ship exchange offer for this hull".into());
        };
        if let Some(reason) = offer.refusal {
            return Outcome::Refused(reason.into());
        }
        Outcome::Request(Request::ReviewShipExchange(self.ship.clone()))
    }
    fn equipment(&self, operation: EquipmentOperation, c: &Context<'_>) -> Outcome {
        if c.paused {
            return Outcome::Refused("Resume before changing equipment".into());
        }
        let Some(panel) = c.equipment else {
            return Outcome::Refused("Modular equipment state is unavailable".into());
        };
        let Some(row) = panel.rows.get(self.equipment_index) else {
            return Outcome::Refused("Select an equipment item".into());
        };
        if let Some(reason) = &row.refusal {
            return Outcome::Refused(reason.clone());
        }
        let option = if self.options {
            let Some(option) = row.options.get(self.option_index) else {
                return Outcome::Refused("Select a module option".into());
            };
            if let Some(reason) = &option.refusal {
                return Outcome::Refused(reason.clone());
            }
            Some(option.id.clone())
        } else {
            None
        };
        let operation = if option.is_some() {
            EquipmentOperation::Configure
        } else {
            operation
        };
        let allowed = match operation {
            EquipmentOperation::Buy => row.can_buy,
            EquipmentOperation::Fit => row.can_fit && row.instance.is_some(),
            EquipmentOperation::Store => row.can_store && row.instance.is_some(),
            EquipmentOperation::Configure => row.instance.is_some(),
        };
        if !allowed {
            return Outcome::Refused(
                "The current item, slot or offer does not permit this action".into(),
            );
        }
        Outcome::Request(Request::ReviewEquipment {
            node: row.node.clone(),
            instance: if operation == EquipmentOperation::Buy {
                None
            } else {
                row.instance
            },
            slot: panel.selected_slot.clone(),
            operation,
            option,
        })
    }
}

fn selection_step(key: Key) -> i32 {
    match key {
        Key::Previous => -1,
        Key::Next => 1,
        Key::PagePrevious => -8,
        _ => 8,
    }
}
