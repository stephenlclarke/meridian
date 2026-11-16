//! Host menu state is ephemeral. Only Profile's prepared commands can change the game.
use crate::{docked_screens as ui, profile::Profile, session::Action};
use meridian_core::{
    AmmunitionAction, ModuleAction, ModuleKind, ModuleSlot, World, equipment,
    navigation::{DockContext, STOCK_JUMP_REACH_LY, SystemId},
    station_services::{self, CommodityId},
};

const ROUTES: [SystemId; 2] = [SystemId::AVENOR, SystemId::RENAS];
pub const BINDINGS: &[ui::BindingHint<'static>] = &[
    ui::BindingHint {
        key: ui::Key::QuantityLess,
        label: "Left -1",
    },
    ui::BindingHint {
        key: ui::Key::QuantityMore,
        label: "Right +1",
    },
    ui::BindingHint {
        key: ui::Key::QuantityLessTen,
        label: "Shift+Left -10",
    },
    ui::BindingHint {
        key: ui::Key::QuantityMoreTen,
        label: "Shift+Right +10",
    },
    ui::BindingHint {
        key: ui::Key::Previous,
        label: "Up",
    },
    ui::BindingHint {
        key: ui::Key::Next,
        label: "Down",
    },
    ui::BindingHint {
        key: ui::Key::NextTab,
        label: "Tab",
    },
    ui::BindingHint {
        key: ui::Key::PreviousTab,
        label: "Shift+Tab",
    },
    ui::BindingHint {
        key: ui::Key::Confirm,
        label: "Enter",
    },
    ui::BindingHint {
        key: ui::Key::Back,
        label: "Esc",
    },
    ui::BindingHint {
        key: ui::Key::Buy,
        label: "G",
    },
    ui::BindingHint {
        key: ui::Key::Sell,
        label: "H",
    },
    ui::BindingHint {
        key: ui::Key::Fit,
        label: "Y",
    },
    ui::BindingHint {
        key: ui::Key::Store,
        label: "U",
    },
    ui::BindingHint {
        key: ui::Key::Search,
        label: "/",
    },
    ui::BindingHint {
        key: ui::Key::Route,
        label: "R",
    },
    ui::BindingHint {
        key: ui::Key::NextSlot,
        label: "]",
    },
];

fn paused(world: &World) -> bool {
    world.mission_snapshot().is_some_and(|m| m.paused)
}
fn hull(world: &World) -> Result<&'static str, String> {
    world
        .equipment_snapshot()
        .map(|s| s.definition.hull)
        .ok_or_else(|| "No owned hull".into())
}
fn context<'a>(
    world: &'a World,
    catalogue: &'a ui::Catalogue,
    panel: &'a ui::EquipmentPanel,
    review: Option<&'a ui::Review>,
    stamp: ui::Stamp,
    selected: SystemId,
) -> ui::Context<'a> {
    let navigation = world.snapshot().navigation;
    ui::Context {
        world,
        catalogue,
        stamp,
        current_hull: panel_hull(world),
        paused: paused(world),
        current_reach_ly: navigation
            .map(|n| (f64::from(n.fuel_units) / 10.0).min(STOCK_JUMP_REACH_LY)),
        full_reach_ly: Some(STOCK_JUMP_REACH_LY),
        route_targets: &ROUTES,
        models: &crate::docked_render::MODELS,
        planet_previews: navigation.is_some_and(|n| n.system == selected)
            && world.celestial_snapshot().is_some(),
        navigation_bodies: &[],
        exchanges: &[],
        equipment: Some(panel),
        review,
        bindings: BINDINGS,
    }
}
fn panel_hull(world: &World) -> &'static str {
    world.equipment_snapshot().map_or("", |s| s.definition.hull)
}

/// Actual material facts; elapsed docked time alone does not invalidate a human review.
#[derive(PartialEq)]
struct Anchor {
    dock: Option<DockContext>,
    credits: Option<u32>,
    equipment: Option<equipment::Snapshot>,
    ammunition: Option<meridian_core::AmmunitionSnapshot>,
    modules: Option<meridian_core::ModularEquipmentSnapshot>,
    services: Option<meridian_core::ServiceSnapshot>,
    quotes: Vec<Option<station_services::CommodityQuote>>,
    paused: bool,
}
impl Anchor {
    fn capture(world: &World) -> Result<Self, String> {
        let ids = CommodityId::display_order().map_err(crate::market::content_error)?;
        Ok(Self {
            dock: world.current_dock_context(),
            credits: world.credits_tenths(),
            equipment: world.equipment_snapshot(),
            ammunition: world.ammunition_snapshot(),
            modules: world.modular_equipment_snapshot(),
            services: world.service_snapshot(),
            quotes: ids
                .into_iter()
                .map(|id| world.commodity_quote(id))
                .collect(),
            paused: paused(world),
        })
    }
}
#[derive(Clone, Copy)]
enum Intent {
    Change(Action),
    Route(SystemId),
}
struct Pending {
    view: ui::Review,
    intent: Intent,
    anchor: Anchor,
}

pub struct Docked {
    catalogue: ui::Catalogue,
    screens: ui::DockedScreens,
    pub open: bool,
    slot: String,
    feedback: String,
    pending: Option<Pending>,
    painted: Option<ui::Frame>,
    serial: u64,
    token: u64,
    pub route: Option<SystemId>,
}
impl Docked {
    pub fn new(world: &World) -> Result<Self, String> {
        let current = world
            .snapshot()
            .navigation
            .ok_or("No commander navigation")?
            .system;
        Ok(Self {
            catalogue: ui::Catalogue::bundled().map_err(str::to_owned)?,
            screens: ui::DockedScreens::new(current, hull(world)?),
            open: false,
            slot: "front-laser".into(),
            feedback: String::new(),
            pending: None,
            painted: None,
            serial: 0,
            token: 0,
            route: None,
        })
    }
    pub fn open(&mut self, world: &World, screen: ui::Screen) -> Result<(), String> {
        if self.pending.is_some() {
            return Err("Confirm or cancel the current review first".into());
        }
        if world.current_dock_context().is_none() {
            return Err("Dock at a station to open this screen".into());
        }
        self.pending = None;
        self.painted = None;
        self.screens.open(screen);
        self.open = true;
        Ok(())
    }
    pub fn close(&mut self) {
        self.open = false;
        self.pending = None;
        self.painted = None;
    }
    pub fn active(&self, world: &World) -> bool {
        self.open && world.current_dock_context().is_some()
    }
    pub fn focus_lost(&mut self) {
        self.screens.open(self.screens.screen());
        self.painted = None;
    }
    pub fn screen(&self) -> ui::Screen {
        self.screens.screen()
    }
    pub fn feedback(&mut self, message: String) {
        self.feedback = message;
    }
    pub fn discard_frame(&mut self) {
        self.painted = None;
    }
    pub fn search_active(&self) -> bool {
        self.painted.as_ref().is_some_and(|f| f.search_active)
    }
    pub fn build_frame(
        &mut self,
        world: &World,
        viewport: ui::Viewport,
        seconds: f64,
    ) -> Result<ui::Frame, String> {
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or("Screen revision exhausted")?;
        self.screens.advance(seconds);
        let panel = equipment_panel(world, &self.slot, &self.feedback)?;
        let stamp = ui::Stamp {
            session: 1,
            revision: self.serial,
        };
        let c = context(
            world,
            &self.catalogue,
            &panel,
            self.pending.as_ref().map(|p| &p.view),
            stamp,
            self.screens.selected_system(),
        );
        Ok(self.screens.frame(&c, viewport))
    }
    /// Call only after the corresponding GPU frame was actually presented.
    pub fn presented(&mut self, frame: ui::Frame) {
        if frame.stamp.revision == self.serial {
            self.painted = Some(frame);
        }
    }
    pub fn input(&mut self, profile: &mut Profile, input: ui::Input) -> Result<(), String> {
        let result = self.input_inner(profile, input);
        if let Err(error) = &result {
            self.feedback = error.clone();
        }
        result
    }
    fn input_inner(&mut self, profile: &mut Profile, input: ui::Input) -> Result<(), String> {
        let painted = self
            .painted
            .as_ref()
            .ok_or("Wait for the current screen to draw")?;
        let panel = equipment_panel(&profile.world, &self.slot, &self.feedback)?;
        let c = context(
            &profile.world,
            &self.catalogue,
            &panel,
            self.pending.as_ref().map(|p| &p.view),
            ui::Stamp {
                session: 1,
                revision: self.serial,
            },
            self.screens.selected_system(),
        );
        let outcome = self.screens.handle(input, painted, &c);
        match outcome {
            ui::Outcome::Updated | ui::Outcome::Ignored => {}
            ui::Outcome::Refused(reason) => {
                self.feedback = reason;
            }
            ui::Outcome::Request(request) => self.request(profile, request)?,
        }
        Ok(())
    }
    fn request(&mut self, profile: &mut Profile, request: ui::Request) -> Result<(), String> {
        let world = &profile.world;
        match request {
            ui::Request::Close => self.close(),
            ui::Request::CancelReview(token) => {
                if self.pending.as_ref().is_none_or(|p| p.view.token != token) {
                    return Err("Review changed".into());
                }
                self.pending = None;
                self.feedback = "Cancelled; no purchase or fitting change".into();
            }
            ui::Request::ConfirmReview(token) => {
                let pending = self.pending.take().ok_or("No current review")?;
                if pending.view.token != token || pending.anchor != Anchor::capture(world)? {
                    self.feedback = "Values changed; open a fresh review".into();
                    return Ok(());
                }
                match pending.intent {
                    Intent::Route(target) => {
                        self.route = Some(target);
                        self.feedback = "Destination selected; launch and press J to jump".into();
                    }
                    Intent::Change(action) => self.feedback = profile.command(action)?,
                }
            }
            ui::Request::ReviewRoute(target) => {
                if !ROUTES.contains(&target)
                    || Some(target) == world.snapshot().navigation.map(|n| n.system)
                {
                    return Err("This destination is not available for travel".into());
                }
                let current = world.snapshot().navigation.ok_or("No navigation")?;
                let catalogue = meridian_core::navigation::OriginalNavigation::bundled()
                    .map_err(|e| e.to_string())?;
                let distance = catalogue
                    .distance_ly(current.system, target)
                    .map_err(|e| e.to_string())?;
                let fuel = meridian_core::navigation::stock_jump_fuel_cost(distance)
                    .map_err(|e| e.to_string())?;
                if distance > STOCK_JUMP_REACH_LY || fuel > current.fuel_units || paused(world) {
                    return Err("Destination exceeds available range or game is paused".into());
                }
                self.review(
                    world,
                    Intent::Route(target),
                    "Select destination",
                    vec![
                        format!(
                            "Distance {distance:.2} LY | Fuel {:.1}",
                            f64::from(fuel) / 10.0
                        ),
                        "No fuel is spent until the actual jump".into(),
                    ],
                )?;
            }
            ui::Request::ReviewCommodity {
                commodity,
                buy,
                quantity,
            } => {
                let quote = world
                    .commodity_quote(commodity)
                    .ok_or("No current commodity offer")?;
                let definition = commodity
                    .definition()
                    .map_err(crate::market::content_error)?;
                let price = if buy {
                    quote.ask_tenths
                } else {
                    quote.bid_tenths
                };
                let command = station_services::CommodityTrade {
                    commodity,
                    quantity,
                    side: if buy {
                        station_services::TradeSide::Buy
                    } else {
                        station_services::TradeSide::Sell
                    },
                };
                let total = u64::from(price)
                    .checked_mul(quantity)
                    .ok_or("Trade total is too large")?;
                let cash = world.credits_tenths().ok_or("No commander wallet")?;
                let credited = total.min(u64::from(u32::MAX - cash));
                let settlement = if buy {
                    format!("Pay {}.{} CR in total", total / 10, total % 10)
                } else {
                    format!(
                        "Gross {}.{} CR; actual credit {}.{} CR",
                        total / 10,
                        total % 10,
                        credited / 10,
                        credited % 10
                    )
                };
                self.review(
                    world,
                    Intent::Change(Action::CommodityTrade(command)),
                    if buy {
                        "Buy commodity"
                    } else {
                        "Sell commodity"
                    },
                    vec![
                        format!(
                            "{}: {quantity} {} | {}.{} CR each",
                            definition.name,
                            definition.unit.symbol(),
                            price / 10,
                            price % 10
                        ),
                        settlement,
                        "Confirm trades all requested units or none".into(),
                    ],
                )?;
            }
            ui::Request::InspectEquipmentSlot(slot) => {
                let panel = equipment_panel(world, &self.slot, &self.feedback)?;
                if !panel.slots.iter().any(|s| s.id == slot && s.available) {
                    return Err("Slot not available".into());
                }
                self.slot = slot;
                self.screens.reset_equipment_selection();
                self.pending = None;
            }
            ui::Request::ReviewEquipment {
                node,
                instance,
                slot,
                operation,
                option,
            } => {
                if option.is_some() {
                    return Err("This equipment has no configurable option".into());
                }
                let panel = equipment_panel(world, &self.slot, &self.feedback)?;
                if slot != self.slot {
                    return Err("Equipment slot changed".into());
                }
                let row = panel
                    .rows
                    .iter()
                    .find(|r| {
                        r.node == node
                            && if operation == ui::EquipmentOperation::Buy {
                                r.instance.is_none()
                            } else {
                                r.instance == instance
                            }
                    })
                    .ok_or("Equipment selection changed")?;
                let allowed = match operation {
                    ui::EquipmentOperation::Buy => row.can_buy,
                    ui::EquipmentOperation::Fit => row.can_fit,
                    ui::EquipmentOperation::Store => row.can_store,
                    _ => false,
                };
                if !allowed {
                    return Err(row
                        .refusal
                        .clone()
                        .unwrap_or_else(|| "This fitting action is unavailable".into()));
                }
                let action = equipment_action(world, &node, instance, &slot, operation)?;
                let details = vec![
                    row.name.clone(),
                    row.price_tenths.map_or_else(
                        || "Owned item; no charge for fitting or storing".into(),
                        |p| format!("Offer {}.{} CR", p / 10, p % 10),
                    ),
                    "Confirm publishes the complete equipment change".into(),
                ];
                self.review(world, Intent::Change(action), "Equipment review", details)?;
            }
            ui::Request::ReviewShipExchange(_) => {
                return Err("Ship exchange is not yet available for this hull".into());
            }
            ui::Request::ReviewBodyRoute(_) => {
                return Err("No current asteroid navigation provider is joined".into());
            }
        }
        Ok(())
    }
    fn review(
        &mut self,
        world: &World,
        intent: Intent,
        title: &str,
        details: Vec<String>,
    ) -> Result<(), String> {
        self.token = self
            .token
            .checked_add(1)
            .ok_or("Review sequence exhausted")?;
        self.pending = Some(Pending {
            view: ui::Review {
                token: self.token,
                title: title.into(),
                details,
                can_confirm: true,
                refusal: None,
            },
            intent,
            anchor: Anchor::capture(world)?,
        });
        Ok(())
    }
}

fn equipment_action(
    world: &World,
    node: &str,
    instance: Option<u64>,
    slot: &str,
    operation: ui::EquipmentOperation,
) -> Result<Action, String> {
    use ui::EquipmentOperation::{Buy, Fit, Store};
    if node == "missile-standard" {
        let action = match operation {
            Buy => AmmunitionAction::Purchase,
            Store => AmmunitionAction::Store {
                item: instance.ok_or("Select an owned missile")?,
            },
            Fit => AmmunitionAction::Refit {
                item: instance.ok_or("Select an owned missile")?,
                pylon: slot
                    .strip_prefix("pylon-")
                    .and_then(|s| s.parse::<u8>().ok())
                    .filter(|n| *n < 9)
                    .ok_or("Select a pylon")?,
            },
            _ => return Err("Unsupported missile operation".into()),
        };
        return Ok(Action::Ammunition(action));
    }
    let equipment = world.equipment_snapshot().ok_or("No equipment")?;
    if node == equipment.definition.id {
        let item = equipment.owned.map(|o| o.id);
        let command = match operation {
            Buy => equipment::Command::PurchaseCargoBay,
            Fit => equipment::Command::FitCargoBay(item.ok_or("No owned cargo bay")?),
            Store => equipment::Command::UnfitCargoBay(item.ok_or("No owned cargo bay")?),
            _ => return Err("Unsupported cargo operation".into()),
        };
        return Ok(Action::CargoEquipment(command));
    }
    let modules = world
        .modular_equipment_snapshot()
        .ok_or("No modular equipment")?;
    let action = match operation {
        Buy if node == ModuleKind::MiningLaser.node() => ModuleAction::PurchaseMiningLaser,
        Store => ModuleAction::Store {
            item: instance.ok_or("Select an owned module")?,
        },
        Fit => ModuleAction::Install {
            item: instance.ok_or("Select an owned module")?,
            replacing: modules
                .items
                .iter()
                .find(|i| i.slot == Some(ModuleSlot::FrontLaser))
                .map(|i| i.id),
        },
        _ => return Err("Unsupported module operation".into()),
    };
    Ok(Action::Module(action))
}

fn base_row(node: &str, name: &str, group: &str) -> ui::EquipmentRow {
    ui::EquipmentRow {
        node: node.into(),
        name: name.into(),
        group: group.into(),
        price_tenths: None,
        quantity: ui::OfferQuantity::NotTracked,
        instance: None,
        owned: 0,
        fitted: 0,
        compatibility: String::new(),
        refusal: None,
        readout: Vec::new(),
        options: Vec::new(),
        can_buy: false,
        can_fit: false,
        can_store: false,
    }
}
fn equipment_panel(
    world: &World,
    selected: &str,
    feedback: &str,
) -> Result<ui::EquipmentPanel, String> {
    let equipment = world.equipment_snapshot().ok_or("No equipment account")?;
    let modules = world
        .modular_equipment_snapshot()
        .ok_or("No module inventory")?;
    let ammo = world.ammunition_snapshot().ok_or("No missile inventory")?;
    let tick = world.snapshot().tick;
    let front = modules
        .items
        .iter()
        .find(|i| i.slot == Some(ModuleSlot::FrontLaser));
    let mut slots = vec![
        ui::EquipmentSlot {
            id: "front-laser".into(),
            label: "Front laser".into(),
            fitted: front.map_or("Empty", |i| i.kind.name()).into(),
            available: true,
        },
        ui::EquipmentSlot {
            id: "cargo-expansion".into(),
            label: "Cargo expansion".into(),
            fitted: if equipment.owned.is_some_and(|i| i.fitted) {
                equipment.definition.name
            } else {
                "Empty"
            }
            .into(),
            available: true,
        },
    ];
    for pylon in 0..ammo.capacity {
        let occupied = ammo
            .items
            .iter()
            .any(|i| !i.consumed && i.pylon == Some(pylon));
        slots.push(ui::EquipmentSlot {
            id: format!("pylon-{pylon}"),
            label: format!("Pylon {}", pylon + 1),
            fitted: if occupied { "Missile" } else { "Empty" }.into(),
            available: true,
        });
    }
    let mut rows = Vec::new();
    if selected == "front-laser" {
        for item in &modules.items {
            let mut row = base_row(item.kind.node(), item.kind.name(), "Front laser");
            row.instance = Some(item.id);
            row.owned = 1;
            row.fitted = u32::from(item.slot.is_some());
            row.compatibility =
                "Owned SH025 front laser; fitting preserves shared energy and heat".into();
            row.can_store = world
                .review_module(ModuleAction::Store { item: item.id }, tick)
                .is_ok();
            row.can_fit = world
                .review_module(
                    ModuleAction::Install {
                        item: item.id,
                        replacing: front.map(|i| i.id),
                    },
                    tick,
                )
                .is_ok();
            rows.push(row);
        }
        if let Some(offer) = modules.mining_offer {
            let mut row = base_row(
                ModuleKind::MiningLaser.node(),
                "Mining Laser (purchase)",
                "Front laser",
            );
            row.price_tenths = Some(u64::from(offer.price_tenths));
            row.compatibility = format!(
                "Requires technology {} | Current {}",
                offer.required_technology, offer.technology
            );
            let admission = world.review_module(ModuleAction::PurchaseMiningLaser, tick);
            row.can_buy = admission.is_ok();
            row.refusal = admission.err().map(|e| e.to_string());
            rows.push(row);
        }
    } else if selected == "cargo-expansion" {
        let mut row = base_row(
            equipment.definition.id,
            equipment.definition.name,
            "Cargo utility",
        );
        row.instance = equipment.owned.map(|i| u64::from(i.id.serial()));
        row.owned = u32::from(equipment.owned.is_some());
        row.fitted = u32::from(equipment.owned.is_some_and(|i| i.fitted));
        row.price_tenths = equipment
            .owned
            .is_none()
            .then_some(u64::from(equipment.definition.price_tenths));
        row.compatibility = format!(
            "Hold {}/{} tonnes; fitted expansion adds {} tonnes",
            equipment.used_tonnes,
            equipment.hold_capacity_tonnes,
            equipment.definition.extra_hold_tonnes
        );
        row.can_buy = equipment.owned.is_none()
            && equipment.service_available
            && equipment
                .current_offer
                .is_some_and(|o| o.purchase_available)
            && world
                .credits_tenths()
                .is_some_and(|c| c >= equipment.definition.price_tenths);
        row.can_store = equipment.service_available
            && equipment.owned.is_some_and(|i| i.fitted)
            && equipment.used_tonnes <= equipment.definition.base_hold_tonnes;
        row.can_fit = equipment.service_available && equipment.owned.is_some_and(|i| !i.fitted);
        rows.push(row);
    } else if let Some(pylon) = selected
        .strip_prefix("pylon-")
        .and_then(|s| s.parse::<u8>().ok())
        .filter(|n| *n < ammo.capacity)
    {
        for item in ammo
            .items
            .iter()
            .filter(|i| !i.consumed && (i.pylon == Some(pylon) || i.pylon.is_none()))
        {
            let mut row = base_row(ammo.node, ammo.name, "Missile pylon");
            row.instance = Some(item.item);
            row.owned = 1;
            row.fitted = u32::from(item.pylon.is_some());
            row.compatibility = format!("Owned missile | Pylon {}", pylon + 1);
            row.can_store = world
                .review_ammunition(AmmunitionAction::Store { item: item.item }, tick)
                .is_ok();
            row.can_fit = world
                .review_ammunition(
                    AmmunitionAction::Refit {
                        item: item.item,
                        pylon,
                    },
                    tick,
                )
                .is_ok();
            rows.push(row);
        }
        let mut row = base_row(ammo.node, "Missile (purchase)", "Missile pylon");
        row.price_tenths = Some(u64::from(ammo.price_tenths));
        let rounds = world.missile_rounds().ok_or("No pylon fit")?;
        let next = rounds.iter().position(Option::is_none);
        row.compatibility = next.map_or_else(
            || "All pylons occupied".into(),
            |n| format!("A purchase fits the first free pylon ({})", n + 1),
        );
        let admission = world.review_ammunition(AmmunitionAction::Purchase, tick);
        row.can_buy = next == Some(usize::from(pylon)) && admission.is_ok();
        row.refusal = admission.err().map(|e| e.to_string());
        rows.push(row);
    } else {
        return Err("Unknown hull fitting slot".into());
    }
    Ok(ui::EquipmentPanel {
        selected_slot: selected.into(),
        slots,
        rows,
        equipment_units: [
            u32::from(equipment.equipment_units_used),
            u32::from(equipment.definition.equipment_units_capacity),
        ],
        utility_mounts: [
            u32::from(equipment.utility_mounts_used),
            u32::from(equipment.definition.utility_mounts_capacity),
        ],
        assigned_crew: None,
        required_crew: None,
        feedback: feedback.into(),
    })
}

/// Menu shortcuts are scoped to the current screen and never become flight commands.
pub fn key(code: winit::keyboard::KeyCode, shift: bool, screen: ui::Screen) -> Option<ui::Key> {
    use ui::Key as K;
    use winit::keyboard::KeyCode as C;
    Some(match code {
        C::Tab if shift => K::PreviousTab,
        C::Tab => K::NextTab,
        C::Escape | C::KeyB if !shift => K::Back,
        C::Enter => K::Confirm,
        C::ArrowLeft if screen == ui::Screen::Trading && shift => K::QuantityLessTen,
        C::ArrowRight if screen == ui::Screen::Trading && shift => K::QuantityMoreTen,
        C::ArrowLeft if screen == ui::Screen::Trading => K::QuantityLess,
        C::ArrowRight if screen == ui::Screen::Trading => K::QuantityMore,
        C::ArrowUp => K::Previous,
        C::ArrowDown => K::Next,
        C::PageUp => K::PagePrevious,
        C::PageDown => K::PageNext,
        C::Slash => K::Search,
        C::Backspace => K::Erase,
        C::Delete => K::ClearSearch,
        C::KeyG => K::Buy,
        C::KeyH => K::Sell,
        C::KeyY => K::Fit,
        C::KeyU => K::Store,
        C::KeyO => K::Options,
        C::KeyD => K::Details,
        C::BracketRight if screen == ui::Screen::Equipment => K::NextSlot,
        C::BracketRight => K::NextGalaxy,
        C::BracketLeft => K::PreviousGalaxy,
        C::KeyL => K::LocalChart,
        C::KeyR if screen == ui::Screen::Showroom => K::Rotate,
        C::KeyR => K::Route,
        C::Space if screen == ui::Screen::Showroom => K::Rotate,
        C::KeyA if screen == ui::Screen::Showroom => K::NextAxis,
        C::ArrowLeft if screen == ui::Screen::Showroom => K::TurnLeft,
        C::ArrowRight if screen == ui::Screen::Showroom => K::TurnRight,
        C::Home if screen == ui::Screen::Showroom => K::ResetView,
        C::Home => K::CurrentLocation,
        C::KeyC => K::Compare,
        C::KeyB if shift => K::PinBaseline,
        C::KeyV => K::CurrentShip,
        C::KeyX => K::Exchange,
        C::KeyN => K::NextBody,
        C::KeyM => K::PreviousBody,
        C::KeyT => K::BodyRoute,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::tests::{TestDirectory, open};
    fn ok<T, E: core::fmt::Debug>(value: Result<T, E>) -> T {
        value.unwrap_or_else(|e| panic!("{e:?}"))
    }
    fn start() -> (TestDirectory, Profile, Docked) {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(ok(profile.new_commander(89811)));
        let mut menu = ok(Docked::new(&profile.world));
        ok(menu.open(&profile.world, ui::Screen::Trading));
        (directory, profile, menu)
    }
    fn present(menu: &mut Docked, profile: &Profile) {
        let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
        let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
        menu.presented(frame);
    }
    fn assert_visible_feedback(menu: &mut Docked, profile: &Profile, expected: &str) {
        let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
        let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
        let label = frame
            .labels
            .iter()
            .find(|l| l.text.contains(expected))
            .unwrap_or_else(|| panic!("Missing visible feedback: {expected}"));
        assert!(label.at[0] >= label.clip.x && label.at[1] >= label.clip.y);
        assert!(label.at[1] + 9.0 <= label.clip.y + label.clip.height);
        assert!(label.text.chars().count() <= (label.clip.width / 7.5).floor() as usize);
        menu.presented(frame);
    }
    #[test]
    fn trading_writer_failure_is_visible_and_preserves_live_and_disk_state() {
        let (directory, mut profile, mut menu) = start();
        ok(profile.save());
        let path = directory.0.join("commander.meridian");
        let backup = directory.0.join("previous-save");
        let before = ok(profile.world.save_bytes());
        let previous = ok(std::fs::read(&path));
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Buy)));
        present(&mut menu, &profile);
        ok(std::fs::rename(&path, &backup));
        ok(std::fs::create_dir(&path));
        assert!(
            menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm))
                .is_err()
        );
        assert!(menu.pending.is_none());
        let expected: String = menu.feedback.chars().take(24).collect();
        assert!(!expected.is_empty());
        assert_visible_feedback(&mut menu, &profile, &expected);
        assert_eq!(ok(profile.world.save_bytes()), before);
        assert_eq!(ok(std::fs::read(&backup)), previous);
        ok(std::fs::remove_dir(&path));
        ok(std::fs::rename(&backup, &path));
        assert_eq!(ok(std::fs::read(&path)), previous);
    }
    #[test]
    fn absent_cargo_sale_refusal_is_visible_on_trading() {
        let (_directory, mut profile, mut menu) = start();
        let before = ok(profile.world.save_bytes());
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Sell)));
        assert!(menu.pending.is_some());
        present(&mut menu, &profile);
        assert!(
            menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm))
                .is_err()
        );
        let expected: String = menu.feedback.chars().take(24).collect();
        assert!(!expected.is_empty());
        assert_visible_feedback(&mut menu, &profile, &expected);
        assert_eq!(ok(profile.world.save_bytes()), before);
    }
    #[test]
    fn route_selection_instruction_is_visible_on_system_and_galaxy() {
        let (_directory, mut profile, mut menu) = start();
        let before = ok(profile.world.save_bytes());
        for screen in [ui::Screen::System, ui::Screen::Galaxy] {
            ok(menu.open(&profile.world, screen));
            ok(menu.request(&mut profile, ui::Request::ReviewRoute(SystemId::RENAS)));
            present(&mut menu, &profile);
            ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
            assert_eq!(menu.route, Some(SystemId::RENAS));
            assert_visible_feedback(&mut menu, &profile, "launch and press J");
            assert_eq!(ok(profile.world.save_bytes()), before);
        }
    }
    #[test]
    fn real_market_review_confirm_publishes_one_native_tonne_once() {
        let (directory, mut profile, mut menu) = start();
        present(&mut menu, &profile);
        let before = ok(profile.world.save_bytes());
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Buy)));
        assert!(menu.pending.is_some());
        assert_eq!(ok(profile.world.save_bytes()), before);
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
        assert!(menu.pending.is_none());
        assert_eq!(profile.world.credits_tenths(), Some(6974));
        assert_eq!(profile.world.commodity_quantity(CommodityId::Food), Some(1));
        assert_eq!(
            ok(std::fs::read(directory.0.join("commander.meridian"))),
            ok(profile.world.save_bytes())
        );
        let after = ok(profile.world.save_bytes());
        // A repeated confirmation of the old painted modal has no purchase authority.
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
        assert_eq!(ok(profile.world.save_bytes()), after);
    }
    #[test]
    fn quantity_review_publishes_all_units_and_cold_continue_preserves_gameplay() {
        let (directory, mut profile, mut menu) = start();
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::QuantityMoreTen)));
        let before = ok(profile.world.save_bytes());
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Buy)));
        assert_eq!(ok(profile.world.save_bytes()), before);
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
        assert_eq!(profile.world.credits_tenths(), Some(4714));
        assert_eq!(
            profile.world.commodity_quantity(CommodityId::Food),
            Some(11)
        );
        let saved = ok(profile.world.save_bytes());
        assert_eq!(
            ok(std::fs::read(directory.0.join("commander.meridian"))),
            saved
        );
        assert_visible_feedback(&mut menu, &profile, "Bought 11 t Food for 248.6 CR");
        let flight = profile.world.snapshot();
        let mission = profile.world.mission_snapshot();
        let services = profile.world.service_snapshot();
        let modules = profile.world.modular_equipment_snapshot();
        let ammunition = profile.world.ammunition_snapshot();
        let resources = profile.world.flight_resources_snapshot();
        let old_review = ok(profile
            .world
            .review_ammunition(AmmunitionAction::Purchase, flight.tick));
        // The public loader rotates review authority without rewriting the saved file.
        let expected = ok(World::from_save_bytes(&saved));
        let expected_continued = ok(expected.save_bytes());
        drop(profile);
        let mut cold = open(&directory.0);
        ok(cold.continue_saved());
        assert_eq!(
            ok(std::fs::read(directory.0.join("commander.meridian"))),
            saved
        );
        assert_eq!(ok(cold.world.save_bytes()), expected_continued);
        assert_eq!(cold.world.snapshot(), flight);
        assert_eq!(cold.world.mission_snapshot(), mission);
        assert_eq!(cold.world.service_snapshot(), services);
        assert_eq!(cold.world.modular_equipment_snapshot(), modules);
        assert_eq!(cold.world.ammunition_snapshot(), ammunition);
        assert_eq!(cold.world.flight_resources_snapshot(), resources);
        assert_eq!(cold.world.credits_tenths(), Some(4714));
        assert_eq!(cold.world.commodity_quantity(CommodityId::Food), Some(11));
        assert!(matches!(
            cold.world.prepare_ammunition(old_review, flight.tick),
            Err(meridian_core::AmmunitionError::StaleReview)
        ));
        assert_eq!(ok(cold.world.save_bytes()), expected_continued);
    }

    #[test]
    fn quantity_publication_failure_leaves_entire_world_and_previous_save_unchanged() {
        let (directory, mut profile, mut menu) = start();
        ok(profile.save());
        let path = directory.0.join("commander.meridian");
        let backup = directory.0.join("previous-save");
        let before = ok(profile.world.save_bytes());
        let previous = ok(std::fs::read(&path));
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::QuantityMoreTen)));
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Buy)));
        present(&mut menu, &profile);
        ok(std::fs::rename(&path, &backup));
        ok(std::fs::create_dir(&path));
        assert!(
            menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm))
                .is_err()
        );
        assert_eq!(ok(profile.world.save_bytes()), before);
        assert_eq!(ok(std::fs::read(&backup)), previous);
        assert!(menu.pending.is_none());
        ok(std::fs::remove_dir(&path));
        ok(std::fs::rename(&backup, &path));
        assert_eq!(ok(std::fs::read(&path)), previous);
    }

    #[test]
    fn unaffordable_quantity_refuses_without_partial_purchase_or_save() {
        let (directory, mut profile, mut menu) = start();
        ok(profile.save());
        let before = ok(profile.world.save_bytes());
        let saved = ok(std::fs::read(directory.0.join("commander.meridian")));
        for _ in 0..4 {
            present(&mut menu, &profile);
            ok(menu.input(&mut profile, ui::Input::Key(ui::Key::QuantityMoreTen)));
        }
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Buy)));
        present(&mut menu, &profile);
        assert!(
            menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm))
                .is_err()
        );
        assert!(menu.pending.is_none());
        assert_eq!(profile.world.commodity_quantity(CommodityId::Food), Some(0));
        assert_eq!(profile.world.credits_tenths(), Some(7200));
        assert_eq!(ok(profile.world.save_bytes()), before);
        assert_eq!(
            ok(std::fs::read(directory.0.join("commander.meridian"))),
            saved
        );
    }

    #[test]
    fn skipped_frames_do_not_publish_actionable_menu_geometry() {
        let (_directory, mut profile, mut menu) = start();
        let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
        let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
        assert!(
            menu.input(&mut profile, ui::Input::Key(ui::Key::Buy))
                .is_err()
        );
        menu.presented(frame);
        let next = ok(menu.build_frame(&profile.world, viewport, 0.0));
        let before = ok(profile.world.save_bytes());
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Buy)));
        assert!(menu.pending.is_none());
        assert_eq!(ok(profile.world.save_bytes()), before);
        menu.presented(next);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Buy)));
        assert!(menu.pending.is_some());
        assert_eq!(ok(profile.world.save_bytes()), before);
    }
    #[test]
    fn changed_quote_or_wallet_requires_new_review_instead_of_double_purchase() {
        let (_directory, mut profile, mut menu) = start();
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Buy)));
        present(&mut menu, &profile);
        ok(profile.command(Action::StationService(
            station_services::Command::BuyCommodity(CommodityId::Food),
        )));
        let after_external_purchase = ok(profile.world.save_bytes());
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
        assert!(menu.pending.is_none());
        assert!(menu.feedback.contains("Values changed"));
        assert_visible_feedback(&mut menu, &profile, "Values changed");
        assert_eq!(ok(profile.world.save_bytes()), after_external_purchase);
    }
    #[test]
    fn front_module_rows_and_pylon_slots_use_real_owned_instances_and_admission() {
        let (_directory, profile, _menu) = start();
        let panel = ok(equipment_panel(&profile.world, "front-laser", ""));
        assert_eq!(panel.slots.len(), 11);
        assert_eq!(panel.assigned_crew, None);
        let pulse = panel
            .rows
            .iter()
            .find(|r| r.node == ModuleKind::PulseLaser.node())
            .unwrap_or_else(|| panic!("Pulse row"));
        assert_eq!((pulse.owned, pulse.fitted), (1, 1));
        assert!(pulse.instance.is_some());
        assert!(!pulse.can_buy);
        let mining = panel
            .rows
            .iter()
            .find(|r| r.node == ModuleKind::MiningLaser.node())
            .unwrap_or_else(|| panic!("Mining row"));
        assert!(!mining.can_buy);
        assert_eq!(mining.price_tenths, Some(33000));
        let pylon = ok(equipment_panel(&profile.world, "pylon-3", ""));
        let offer = pylon
            .rows
            .iter()
            .find(|r| r.instance.is_none())
            .unwrap_or_else(|| panic!("offer"));
        assert!(offer.can_buy);
        assert_eq!(offer.quantity, ui::OfferQuantity::NotTracked);
        assert_eq!(
            profile
                .world
                .missile_rounds()
                .unwrap_or_else(|| panic!("rounds"))
                .iter()
                .flatten()
                .count(),
            3
        );
    }
}
