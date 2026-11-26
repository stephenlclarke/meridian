//! Host menu state is ephemeral. Only Profile's prepared commands can change the game.
use crate::{docked_screens as ui, profile::Profile, session::Action};
use meridian_core::{
    AmmunitionAction, ModuleAction, ModuleKind, ModuleSlot, PassageStatus, PassengerAction, World,
    equipment,
    navigation::{DockContext, SystemId},
    station_services::{self, CommodityId},
};

use meridian_core::navigation::ROUTE_SYSTEMS;
pub const BINDINGS: &[ui::BindingHint<'static>] = &[
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
        label: "Shift+G",
    },
    ui::BindingHint {
        key: ui::Key::Refuel,
        label: "F",
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
    let reach = world.jump_reach_ly();
    ui::Context {
        world,
        catalogue,
        stamp,
        current_hull: panel_hull(world),
        paused: paused(world),
        current_reach_ly: navigation
            .zip(reach)
            .map(|(n, reach)| (f64::from(n.fuel_units) / 10.0).min(reach)),
        full_reach_ly: navigation
            .zip(reach)
            .map(|(n, reach)| reach.min(f64::from(n.fuel_capacity_units) / 10.0)),
        route_targets: &ROUTE_SYSTEMS,
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
    passage: Option<meridian_core::PassengerJourneySnapshot>,
    emp: Option<meridian_core::EmpSnapshot>,
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
            passage: world.passenger_journey_snapshot(),
            emp: world.emp_snapshot(),
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
    pub(crate) fn review_pending(&self) -> bool {
        self.pending.is_some()
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
    pub fn help_panel(&self) -> crate::help::Panel {
        crate::help::Panel::station(
            self.screen(),
            self.pending
                .as_ref()
                .map(|pending| pending.view.title.as_str()),
            self.search_active(),
            self.screens.details_open(),
        )
    }
    pub fn feedback(&mut self, message: String) {
        self.feedback = message;
    }
    pub fn discard_frame(&mut self) {
        self.painted = None;
    }
    pub fn search_active(&self) -> bool {
        self.screens.search_active()
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
                        if world.active_route_progress().is_some() {
                            profile.navigation_route(None)?;
                        }
                        self.route = Some(target);
                        self.feedback = "Destination selected; launch and press J to jump".into();
                    }
                    Intent::Change(action) => self.feedback = profile.command(action)?,
                }
            }
            ui::Request::SetNavigationRoute(plan) => {
                self.feedback = profile.navigation_route(Some(&plan))?;
                self.route = None;
                self.screens.close_route_chooser();
            }
            ui::Request::ClearNavigationRoute => {
                self.feedback = profile.navigation_route(None)?;
                self.route = None;
                self.screens.close_route_chooser();
            }
            ui::Request::RefuelNavigationRoute => {
                if !world.active_route_progress().is_some_and(|route| {
                    route.status == meridian_core::navigation::RouteStatus::RefuelRequired
                }) {
                    return Err("The active route does not need fuel".into());
                }
                self.feedback = profile.command(Action::StationService(
                    station_services::Command::RefillFuel,
                ))?;
            }
            ui::Request::RefuelMarket => {
                if self.screen() != ui::Screen::Trading {
                    return Err("Open Market to refuel".into());
                }
                ui::market_refuel_admission(world)?;
                self.feedback = profile.command(Action::StationService(
                    station_services::Command::RefillFuel,
                ))?;
            }
            ui::Request::ReviewRoute(target) => {
                if !ROUTE_SYSTEMS.contains(&target)
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
                let reach = world.jump_reach_ly().ok_or("No fitted drive")?;
                if distance > reach.min(f64::from(current.fuel_units) / 10.0) || paused(world) {
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
            ui::Request::ReviewCommodity { commodity, buy } => {
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
                let command = if buy {
                    station_services::Command::BuyCommodity(commodity)
                } else {
                    station_services::Command::SellCommodity(commodity)
                };
                self.review(
                    world,
                    Intent::Change(Action::StationService(command)),
                    if buy {
                        "Buy commodity"
                    } else {
                        "Sell commodity"
                    },
                    vec![
                        format!(
                            "{}: 1 {} | {}.{} CR",
                            definition.name,
                            definition.unit.symbol(),
                            price / 10,
                            price % 10
                        ),
                        "Confirm changes cargo and credits together".into(),
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
                self.review_equipment(world, node, instance, slot, operation, option)?;
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
    fn review_equipment(
        &mut self,
        world: &World,
        node: String,
        instance: Option<u64>,
        slot: String,
        operation: ui::EquipmentOperation,
        option: Option<String>,
    ) -> Result<(), String> {
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
        let details = if slot == "passenger-service" {
            let mut details = vec![row.name.clone()];
            details.extend(row.readout.clone());
            details.push("Confirm publishes the complete passage action".into());
            details
        } else {
            vec![
                row.name.clone(),
                row.price_tenths.map_or_else(
                    || "Owned item; no charge for fitting or storing".into(),
                    |p| format!("Offer {}.{} CR", p / 10, p % 10),
                ),
                "Confirm publishes the complete equipment change".into(),
            ]
        };
        self.review(
            world,
            Intent::Change(action),
            if slot == "passenger-service" {
                "Passenger service review"
            } else {
                "Equipment review"
            },
            details,
        )?;
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
    if slot == "passenger-service" {
        if operation != Buy || instance.is_some() {
            return Err("Select a passenger service action".into());
        }
        let action = match node {
            "g04-request" => PassengerAction::RequestReplies,
            "g04-board" => PassengerAction::Board,
            "g04-receive" => PassengerAction::Receive,
            "g04-cancel" => PassengerAction::Cancel,
            _ => return Err("Unknown passenger service action".into()),
        };
        return Ok(Action::Module(ModuleAction::Passenger(action)));
    }
    if slot == ui::emp::SLOT {
        return ui::emp::action(world, node, instance, operation).map(Action::Emp);
    }
    if slot == ui::fuel_scoops::SLOT {
        return ui::fuel_scoops::action(world, node, instance, operation).map(Action::Module);
    }
    if node == ModuleKind::FuelScoops.node() {
        return Err("Select the Fuel Scoops utility slot".into());
    }
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
    let targeting_node = node.starts_with("Targeting: ");
    if targeting_node != (slot == "targeting") {
        return Err("Select the targeting computer slot".into());
    }
    let drive_node = node.ends_with(" Hyperspace Drive");
    if drive_node != (slot == "drive") {
        return Err("Select the hyperspace drive slot".into());
    }
    if drive_node
        && operation != Buy
        && !modules
            .items
            .iter()
            .any(|owned| Some(owned.id) == instance && owned.kind.node() == node)
    {
        return Err("Owned drive selection changed".into());
    }
    let shield_node = meridian_core::ShieldGrade::ALL
        .iter()
        .any(|grade| grade.node() == node);
    if shield_node != (slot == "shield") {
        return Err("Select the shield system slot".into());
    }
    let cabin_node = meridian_core::CabinGrade::ALL
        .iter()
        .any(|grade| grade.name() == node);
    if cabin_node != (slot == "passenger-cabins") {
        return Err("Select passenger cabins".into());
    }
    if cabin_node
        && operation != Buy
        && !modules
            .items
            .iter()
            .any(|owned| Some(owned.id) == instance && owned.kind.node() == node)
    {
        return Err("Owned cabin selection changed".into());
    }
    if shield_node
        && operation != Buy
        && !modules
            .items
            .iter()
            .any(|owned| Some(owned.id) == instance && owned.kind.node() == node)
    {
        return Err("Owned shield selection changed".into());
    }
    let tank_node = node == ModuleKind::FuelStorage.node();
    if tank_node != (slot == "fuel-storage") {
        return Err("Select the fuel storage slot".into());
    }
    if tank_node
        && operation != Buy
        && !modules
            .items
            .iter()
            .any(|owned| Some(owned.id) == instance && owned.kind == ModuleKind::FuelStorage)
    {
        return Err("Owned fuel storage selection changed".into());
    }
    if targeting_node
        && operation != Buy
        && !modules
            .items
            .iter()
            .any(|owned| Some(owned.id) == instance && owned.kind.node() == node)
    {
        return Err("Owned module selection changed".into());
    }
    let action = match operation {
        Buy => {
            let offer = modules
                .offers
                .iter()
                .find(|offer| offer.kind.node() == node)
                .ok_or("No ordinary offer for this module")?;
            if offer.kind == ModuleKind::MiningLaser {
                ModuleAction::PurchaseMiningLaser
            } else {
                ModuleAction::Purchase { kind: offer.kind }
            }
        }
        Store => ModuleAction::Store {
            item: instance.ok_or("Select an owned module")?,
        },
        Fit => {
            let item = instance.ok_or("Select an owned module")?;
            let kind = modules
                .items
                .iter()
                .find(|owned| owned.id == item && owned.kind.node() == node)
                .ok_or("Owned module selection changed")?
                .kind;
            ModuleAction::Install {
                item,
                replacing: if kind == ModuleKind::FuelStorage || cabin_node {
                    None
                } else {
                    modules
                        .items
                        .iter()
                        .find(|owned| owned.slot == Some(kind.slot()))
                        .map(|owned| owned.id)
                },
            }
        }
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
fn module_rows(
    world: &World,
    modules: &meridian_core::ModularEquipmentSnapshot,
    slot: ModuleSlot,
    tick: u64,
) -> Vec<ui::EquipmentRow> {
    let label = match slot {
        ModuleSlot::FrontLaser => "Front laser",
        ModuleSlot::Energy => "Energy system",
        ModuleSlot::Navigation => "Navigation computer",
        ModuleSlot::Drive => "Hyperspace drive",
        ModuleSlot::Shield => "Shield system",
        ModuleSlot::Targeting => "Targeting computer",
        ModuleSlot::MiningTargeting => "Mining targeting",
        ModuleSlot::SalvageTargeting => "Salvage targeting",
        ModuleSlot::UtilityFuelScoops => "Fuel Scoops",
        ModuleSlot::FuelStorage(_) => "Fuel storage",
        ModuleSlot::PassengerCabin => "Passenger cabins",
    };
    let fitted = if slot == ModuleSlot::PassengerCabin {
        None
    } else {
        modules.items.iter().find(|item| item.slot == Some(slot))
    };
    let mut rows = Vec::new();
    for item in modules.items.iter().filter(|item| item.kind.slot() == slot) {
        let mut row = base_row(item.kind.node(), item.kind.name(), label);
        row.instance = Some(item.id);
        row.owned = 1;
        row.fitted = u32::from(item.slot.is_some());
        row.readout
            .push(crate::xyzzy::provenance(item.provenance).into());
        if item.kind == ModuleKind::FuelStorage {
            row.compatibility =
                "Repeatable physical tank; 1 utility mount and 1 equipment unit".into();
            row.readout
                .push("Adds 7.0 LY of tank fuel; refuel separately".into());
            if let Some(ModuleSlot::FuelStorage(index)) = item.slot {
                row.readout
                    .push(format!("Fitted tank position {}", index + 1));
            }
        } else if let ModuleKind::Cabin(grade) = item.kind {
            row.compatibility = "Separate 5t accommodation module | 1 equipment unit".into();
            row.readout.push(format!(
                "{} places | {}",
                grade.places(),
                if grade.supports_awake() {
                    "Awake or stasis"
                } else {
                    "Stasis only"
                }
            ));
            if let Some(cabin) = world
                .passenger_cabins()
                .and_then(|cabins| cabins.into_iter().find(|cabin| cabin.item == item.id))
            {
                if cabin.fitted {
                    row.readout.push(format!(
                        "Life support: {} | Stasis: {}",
                        if cabin.life_support_working {
                            "working"
                        } else {
                            "unavailable"
                        },
                        if cabin.stasis_working {
                            "working"
                        } else {
                            "unavailable"
                        }
                    ));
                }
                row.readout.push(format!(
                    "Free awake / stasis places {}/{}",
                    cabin.free_awake_places, cabin.free_stasis_places
                ));
                if let Some(person) = cabin.passenger {
                    row.readout
                        .push(format!("Private reservation: {}", person.name));
                }
            }
        } else if let ModuleKind::Shield(grade) = item.kind {
            row.compatibility = "Owned SH025 shield module; one fitted shield required".into();
            row.readout.push(format!(
                "Capacity {} per bank | Recharge unchanged",
                grade.capacity() / 100
            ));
        } else if let ModuleKind::Drive(grade) = item.kind {
            row.compatibility = "Owned SH025 hyperspace drive; one fitted drive required".into();
            row.readout.push(format!(
                "Spool {}s | Station mass lock {}",
                grade.spool_ticks() / 60,
                grade.mass_lock_distance()
            ));
        } else if let Some(reach) = item.kind.navigation_reach_ly() {
            row.compatibility = "Owned SH025 navigation module".into();
            row.readout.push(format!("Module jump limit {reach} LY"));
        } else if matches!(item.kind, ModuleKind::Targeting(_)) {
            row.compatibility = "Owned SH025 computer module".into();
            row.readout.push("Equipment units 1".into());
        } else if matches!(
            item.kind,
            ModuleKind::MiningTargeting | ModuleKind::SalvageTargeting
        ) {
            row.compatibility =
                "Independent SH025 fitting; requires a fitted targeting core".into();
            row.readout.push(
                if item.kind == ModuleKind::MiningTargeting {
                    "Live rock hold and pending ore tracking"
                } else {
                    "Tracking verified local wrecks"
                }
                .into(),
            );
        } else {
            row.compatibility =
                "Owned SH025 module; fitting preserves shared energy and heat".into();
        }
        if let Some(profile) = item.kind.laser_profile() {
            row.readout.push(format!(
                "Damage {} | Range 6000 | Fire interval {:.1}s",
                profile.damage_hundredths() / 100,
                f64::from(profile.cooldown_ticks()) / 60.0
            ));
        }
        if let Some(multiplier) = item.kind.energy_recharge_multiplier() {
            row.readout.push(format!(
                "Recharge {multiplier}x basic | Energy capacity 380"
            ));
        }
        row.can_store = world
            .review_module(ModuleAction::Store { item: item.id }, tick)
            .is_ok();
        row.can_fit = world
            .review_module(
                ModuleAction::Install {
                    item: item.id,
                    replacing: if item.kind == ModuleKind::FuelStorage {
                        None
                    } else {
                        fitted.map(|item| item.id)
                    },
                },
                tick,
            )
            .is_ok();
        rows.push(row);
    }
    for offer in modules.offers.iter().filter(|offer| offer.slot == slot) {
        let mut row = base_row(
            offer.kind.node(),
            &format!("{} (purchase)", offer.kind.name()),
            label,
        );
        row.price_tenths = Some(u64::from(offer.price_tenths));
        row.compatibility = format!(
            "Requires technology {} | Current {} | Equipment units {}",
            offer.required_technology, offer.technology, offer.equipment_units
        );
        if let Some(required) = offer.prerequisite {
            row.readout.push(format!(
                "Requires {}{}",
                if offer.slot == ModuleSlot::Drive {
                    "fitted "
                } else {
                    "owned "
                },
                required.name()
            ));
        }
        if let ModuleKind::Drive(grade) = offer.kind {
            row.readout.push(format!(
                "Spool {}s | Station mass lock {}",
                grade.spool_ticks() / 60,
                grade.mass_lock_distance()
            ));
        }
        if let ModuleKind::Shield(grade) = offer.kind {
            row.readout.push(format!(
                "Capacity {} per bank | Recharge unchanged",
                grade.capacity() / 100
            ));
        }
        if offer.kind == ModuleKind::FuelStorage {
            row.readout
                .push("Adds 7.0 LY of tank fuel; fuel not included".into());
            row.readout
                .push("Normal SH025: 3 shared utility mounts; global ownership cap 4".into());
        } else if let ModuleKind::Cabin(grade) = offer.kind {
            row.readout.push(format!(
                "5t | {} places | {}",
                grade.places(),
                if grade.supports_awake() {
                    "Awake or stasis"
                } else {
                    "Stasis only"
                }
            ));
        }
        let action = if offer.kind == ModuleKind::MiningLaser {
            ModuleAction::PurchaseMiningLaser
        } else {
            ModuleAction::Purchase { kind: offer.kind }
        };
        let admission = world.review_module(action, tick);
        row.can_buy = admission.is_ok();
        row.refusal = admission.err().map(|error| error.to_string());
        rows.push(row);
    }
    rows
}
fn shield_acquisition_rows() -> Vec<ui::EquipmentRow> {
    use meridian_core::ShieldGrade;
    [
        (
            ShieldGrade::AdvancedII,
            "ELITE-rank earned reward or PLUGH-only paid stock",
            "PLUGH source quote 10125.0 CR; unlock and rank paths are not available here",
        ),
        (
            ShieldGrade::Elite,
            "Acquisition-only; explicit XYZZY maximum or test grant",
            "No ordinary shop offer; test access does not create an earned receipt",
        ),
    ]
    .into_iter()
    .map(|(grade, rule, limit)| {
        let mut row = base_row(grade.node(), grade.name(), "Shield source acquisition");
        row.compatibility = rule.into();
        row.readout.push(format!(
            "Capacity {} per bank | Recharge unchanged",
            grade.capacity() / 100
        ));
        row.refusal = Some(limit.into());
        row
    })
    .collect()
}
fn passenger_service_rows(world: &World) -> Result<Vec<ui::EquipmentRow>, String> {
    let journey = world
        .passenger_journey_snapshot()
        .ok_or("No passenger service")?;
    let mut rows = Vec::new();
    for (node, label, action, detail) in [
        (
            "g04-request",
            "Request replies",
            PassengerAction::RequestReplies,
            "Both people evaluate current separate awake cabins and the Renas route.",
        ),
        (
            "g04-board",
            "Board both people",
            PassengerAction::Board,
            "Board the same two consenting people, one per private cabin.",
        ),
        (
            "g04-receive",
            "Renas reception / receive both",
            PassengerAction::Receive,
            "Actual Renas reception awards 610 CR and the first trusted clue once.",
        ),
        (
            "g04-cancel",
            "Avenor custodian / release both",
            PassengerAction::Cancel,
            "Return both original people to Avenor; no payment or clue.",
        ),
    ] {
        let mut row = base_row(node, label, "G04 passenger service");
        row.compatibility = "Avenor to Renas | 3.365 LY | Untimed | No deposit".into();
        let status = match journey.status {
            PassageStatus::Waiting => "Awaiting replies",
            PassageStatus::Refused if journey.reply_is_current => "Refused current offer",
            PassageStatus::Accepted if journey.reply_is_current => "Accepted current offer",
            PassageStatus::Refused | PassageStatus::Accepted => "Reply stale; request again",
            PassageStatus::Aboard => "Aboard in separate private cabins",
            PassageStatus::Delivered => "Received at Renas",
            PassageStatus::Cancelled => "Released to Avenor",
        };
        for person in journey.people {
            row.readout.push(format!("{}: {status}", person.name));
        }
        row.readout.push(detail.into());
        if let Some(clue) = journey.clue.and_then(|receipt| receipt.clue_id()) {
            row.readout.push(format!("Trusted clue: {clue}"));
        }
        let review = world.review_module(ModuleAction::Passenger(action), world.snapshot().tick);
        row.can_buy = review.is_ok();
        row.refusal = review.err().map(|error| error.to_string());
        rows.push(row);
    }
    Ok(rows)
}
fn append_pylon_slots(
    slots: &mut Vec<ui::EquipmentSlot>,
    ammo: &meridian_core::AmmunitionSnapshot,
) {
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
    slots.push(ui::EquipmentSlot {
        id: "energy".into(),
        label: "Energy system".into(),
        fitted: modules
            .items
            .iter()
            .find(|item| item.slot == Some(ModuleSlot::Energy))
            .map_or("Empty", |item| item.kind.name())
            .into(),
        available: true,
    });
    slots.push(ui::EquipmentSlot {
        id: "navigation".into(),
        label: "Navigation computer".into(),
        fitted: modules
            .items
            .iter()
            .find(|item| item.slot == Some(ModuleSlot::Navigation))
            .map_or("Empty", |item| item.kind.name())
            .into(),
        available: true,
    });
    slots.push(ui::EquipmentSlot {
        id: "drive".into(),
        label: "Hyperspace drive".into(),
        fitted: modules
            .items
            .iter()
            .find(|item| item.slot == Some(ModuleSlot::Drive))
            .map_or("Empty", |item| item.kind.name())
            .into(),
        available: true,
    });
    slots.push(ui::EquipmentSlot {
        id: "shield".into(),
        label: "Shield system".into(),
        fitted: modules
            .items
            .iter()
            .find(|item| item.slot == Some(ModuleSlot::Shield))
            .map_or("Empty", |item| item.kind.name())
            .into(),
        available: true,
    });
    slots.push(ui::fuel_scoops::slot(world)?);
    slots.push(ui::emp::slot(world));
    slots.push(ui::EquipmentSlot {
        id: "passenger-cabins".into(),
        label: "Passenger cabins".into(),
        fitted: format!(
            "{} fitted",
            modules
                .items
                .iter()
                .filter(|item| item.slot == Some(ModuleSlot::PassengerCabin))
                .count()
        ),
        available: true,
    });
    slots.push(ui::EquipmentSlot {
        id: "passenger-service".into(),
        label: "G04 passenger service".into(),
        fitted: world
            .passenger_journey_snapshot()
            .map_or("Unavailable".into(), |snapshot| {
                format!("{:?}", snapshot.status)
            }),
        available: true,
    });
    append_pylon_slots(&mut slots, &ammo);
    for (id, label, slot) in [
        ("targeting", "Targeting computer", ModuleSlot::Targeting),
        (
            "targeting-mining",
            "Mining targeting",
            ModuleSlot::MiningTargeting,
        ),
        (
            "targeting-salvage",
            "Salvage targeting",
            ModuleSlot::SalvageTargeting,
        ),
    ] {
        slots.push(ui::EquipmentSlot {
            id: id.into(),
            label: label.into(),
            fitted: modules
                .items
                .iter()
                .find(|item| item.slot == Some(slot))
                .map_or("Empty", |item| item.kind.name())
                .into(),
            available: true,
        });
    }
    let fitted_tanks = modules
        .items
        .iter()
        .filter(|item| item.kind == ModuleKind::FuelStorage && item.slot.is_some())
        .count();
    let fuel = world.snapshot().navigation.ok_or("No fuel account")?;
    slots.push(ui::EquipmentSlot {
        id: "fuel-storage".into(),
        label: "Fuel storage".into(),
        fitted: format!(
            "{fitted_tanks} fitted | {:.1}/{:.1} LY fuel",
            f32::from(fuel.fuel_units) / 10.0,
            f32::from(fuel.fuel_capacity_units) / 10.0
        ),
        available: true,
    });
    let mut rows = Vec::new();
    let module_slot = match selected {
        "front-laser" => Some(ModuleSlot::FrontLaser),
        "energy" => Some(ModuleSlot::Energy),
        "navigation" => Some(ModuleSlot::Navigation),
        "drive" => Some(ModuleSlot::Drive),
        "shield" => Some(ModuleSlot::Shield),
        "fuel-storage" => Some(ModuleSlot::FuelStorage(0)),
        "passenger-cabins" => Some(ModuleSlot::PassengerCabin),
        "targeting" => Some(ModuleSlot::Targeting),
        "targeting-mining" => Some(ModuleSlot::MiningTargeting),
        "targeting-salvage" => Some(ModuleSlot::SalvageTargeting),
        _ => None,
    };
    if let Some(slot) = module_slot {
        rows = module_rows(world, &modules, slot, tick);
        if slot == ModuleSlot::Shield {
            rows.extend(shield_acquisition_rows());
        }
    } else if selected == "passenger-service" {
        rows = passenger_service_rows(world)?;
    } else if selected == ui::emp::SLOT {
        rows = ui::emp::rows(world);
    } else if selected == ui::fuel_scoops::SLOT {
        rows = ui::fuel_scoops::rows(world)?;
    } else if selected == "cargo-expansion" {
        let mut row = base_row(
            equipment.definition.id,
            equipment.definition.name,
            "Cargo utility",
        );
        row.instance = equipment.owned.map(|i| u64::from(i.id.serial()));
        row.owned = u32::from(equipment.owned.is_some());
        row.fitted = u32::from(equipment.owned.is_some_and(|i| i.fitted));
        if let Some(item) = equipment.owned {
            row.readout.push(
                if item.xyzzy {
                    "XYZZY grant; no paid purchase"
                } else {
                    "Paid cargo bay; original receipt retained"
                }
                .into(),
            );
        }
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
                .is_some_and(|c| c >= equipment.definition.price_tenths)
            && modules.utility_mounts_used < modules.utility_mounts_capacity
            && modules.equipment_units_used < modules.equipment_units_capacity;
        row.can_store = equipment.service_available
            && equipment.owned.is_some_and(|i| i.fitted)
            && equipment.used_tonnes <= equipment.definition.base_hold_tonnes;
        row.can_fit = equipment.service_available
            && equipment.owned.is_some_and(|i| !i.fitted)
            && modules.utility_mounts_used < modules.utility_mounts_capacity
            && modules.equipment_units_used < modules.equipment_units_capacity;
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
            row.readout.push(
                if item.xyzzy {
                    "XYZZY finite missile; no paid purchase"
                } else if item.purchase.is_some() {
                    "Paid finite missile; original receipt retained"
                } else {
                    "Born finite missile"
                }
                .into(),
            );
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
            u32::from(modules.equipment_units_used),
            u32::from(modules.equipment_units_capacity),
        ],
        utility_mounts: [
            u32::from(modules.utility_mounts_used),
            u32::from(modules.utility_mounts_capacity),
        ],
        assigned_crew: None,
        required_crew: None,
        feedback: feedback.into(),
    })
}

#[cfg(test)]
mod secret_ui_tests {
    use super::*;
    use crate::profile::tests::{TestDirectory, open};
    fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|error| panic!("{error:?}"))
    }
    #[test]
    fn fuel_storage_rows_show_actual_count_capacity_and_repeatable_identity() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(ok(profile.new_commander(99037)));
        let panel = ok(equipment_panel(&profile.world, "fuel-storage", ""));
        assert!(
            panel
                .rows
                .iter()
                .any(|row| row.price_tenths == Some(20_000) && !row.can_buy)
        );
        ok(profile.xyzzy(meridian_core::XyzzyAction::MaximumFuelStorage));
        let panel = ok(equipment_panel(&profile.world, "fuel-storage", ""));
        assert_eq!(panel.utility_mounts, [7, 3]);
        let slot = panel
            .slots
            .iter()
            .find(|slot| slot.id == "fuel-storage")
            .unwrap_or_else(|| panic!("tank slot"));
        assert!(slot.fitted.contains("4 fitted"));
        assert!(slot.fitted.contains("41.0/41.0"));
        let owned = panel
            .rows
            .iter()
            .filter(|row| row.instance.is_some())
            .collect::<Vec<_>>();
        assert_eq!(owned.len(), 4);
        assert!(owned.iter().all(|row| row.fitted == 1
            && !row.can_store
            && row.readout.iter().any(|line| line.contains("XYZZY grant"))));
        assert!(
            equipment_action(
                &profile.world,
                ModuleKind::FuelStorage.node(),
                None,
                "shield",
                ui::EquipmentOperation::Buy
            )
            .is_err()
        );
    }
    #[test]
    fn actual_equipment_panels_keep_born_paid_and_xyzzy_provenance_distinct() {
        let directory = TestDirectory::new();
        let mut profile = open(&directory.0);
        assert!(ok(profile.new_commander(99036)));
        ok(profile.xyzzy(meridian_core::XyzzyAction::Enter));
        for slot in [
            "front-laser",
            "energy",
            "navigation",
            "shield",
            "targeting",
            "utility-fuelScoops",
            "cargo-expansion",
            "pylon-8",
        ] {
            let panel = ok(equipment_panel(&profile.world, slot, ""));
            assert!(
                panel
                    .rows
                    .iter()
                    .any(|row| row.owned == 1
                        && row.readout.iter().any(|line| line.contains("XYZZY"))),
                "{slot}"
            );
        }
        ok(profile.command(crate::session::Action::Module(
            ModuleAction::PurchaseMiningLaser,
        )));
        let front = ok(equipment_panel(&profile.world, "front-laser", ""));
        assert!(
            front
                .rows
                .iter()
                .any(|row| row.node == ModuleKind::MiningLaser.node()
                    && row.readout.iter().any(|line| line.contains("Paid module")))
        );
        assert!(
            front
                .rows
                .iter()
                .any(|row| row.node == ModuleKind::PulseLaser.node()
                    && row.readout.iter().any(|line| line.contains("Born module")))
        );
        assert!(
            front
                .rows
                .iter()
                .any(|row| row.node == ModuleKind::NavalLaser.node()
                    && row.readout.iter().any(|line| line.contains("XYZZY grant")))
        );
    }
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
        C::ArrowUp => K::Previous,
        C::ArrowDown => K::Next,
        C::PageUp => K::PagePrevious,
        C::PageDown => K::PageNext,
        C::Slash => K::Search,
        C::Backspace => K::Erase,
        C::Delete => K::ClearSearch,
        C::KeyG if shift => K::Sell,
        C::KeyG => K::Buy,
        C::KeyF if screen == ui::Screen::Trading && !shift => K::Refuel,
        C::KeyY => K::Fit,
        C::KeyU => K::Store,
        C::KeyO => K::Options,
        C::KeyD => K::Details,
        C::BracketRight if screen == ui::Screen::Equipment => K::NextSlot,
        C::BracketRight => K::NextGalaxy,
        C::BracketLeft => K::PreviousGalaxy,
        C::KeyL => K::LocalChart,
        C::KeyP if screen == ui::Screen::Galaxy => K::ProfitScan,
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
    fn fit_paid_tank(profile: &mut Profile) {
        ok(profile.xyzzy(meridian_core::XyzzyAction::Enter));
        let scoop = profile
            .world
            .modular_equipment_snapshot()
            .and_then(|snapshot| {
                snapshot
                    .items
                    .into_iter()
                    .find(|item| item.kind == ModuleKind::FuelScoops && item.slot.is_some())
            })
            .unwrap_or_else(|| panic!("fitted scoop"));
        ok(profile.command(Action::Module(ModuleAction::Store { item: scoop.id })));
        ok(profile.command(Action::Module(ModuleAction::Purchase {
            kind: ModuleKind::FuelStorage,
        })));
    }
    #[test]
    fn market_refuel_key_pays_for_actual_tank_and_preserves_identity() {
        use winit::keyboard::KeyCode;
        let (directory, mut profile, mut menu) = start();
        fit_paid_tank(&mut profile);
        let before_modules = profile.world.modular_equipment_snapshot();
        let before_cash = profile.world.credits_tenths().unwrap_or_default();
        let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
        let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
        assert!(frame.labels.iter().any(|label| label.text == "Refuel (F)"));
        assert!(
            frame
                .labels
                .iter()
                .any(|label| label.text == "Fuel 13.0/20.0 LY | Refuel 14.0 CR")
        );
        assert!(
            frame
                .hits
                .iter()
                .any(|hit| hit.control == ui::Control::Key(ui::Key::Refuel) && hit.enabled)
        );
        menu.presented(frame);
        assert_eq!(key(KeyCode::KeyF, false, ui::Screen::Equipment), None);
        assert_eq!(key(KeyCode::KeyF, true, ui::Screen::Trading), None);
        let refuel = key(KeyCode::KeyF, false, ui::Screen::Trading)
            .unwrap_or_else(|| panic!("Market F binding"));

        let tick = profile.world.snapshot().tick;
        ok(profile.world.request_pause(true, tick));
        present(&mut menu, &profile);
        let paused = ok(profile.world.save_bytes());
        ok(menu.input(&mut profile, ui::Input::Key(refuel)));
        assert_visible_feedback(&mut menu, &profile, "Resume before refuelling");
        assert_eq!(ok(profile.world.save_bytes()), paused);
        ok(profile.world.request_pause(false, tick));
        present(&mut menu, &profile);
        let before = ok(profile.world.save_bytes());

        // A different pending review cannot be bypassed by the fuel shortcut.
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Buy)));
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(refuel)));
        assert_eq!(ok(profile.world.save_bytes()), before);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Back)));
        present(&mut menu, &profile);

        // Refuelling uses Profile's durable transaction, not a direct world mutation.
        let path = directory.0.join("commander.meridian");
        let backup = directory.0.join("previous-save");
        let disk_before = ok(std::fs::read(&path));
        ok(std::fs::rename(&path, &backup));
        ok(std::fs::create_dir(&path));
        assert!(menu.input(&mut profile, ui::Input::Key(refuel)).is_err());
        assert_eq!(ok(profile.world.save_bytes()), before);
        assert_eq!(ok(std::fs::read(&backup)), disk_before);
        ok(std::fs::remove_dir(&path));
        ok(std::fs::rename(&backup, &path));
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(refuel)));
        assert_eq!(profile.world.credits_tenths(), Some(before_cash - 140));
        let nav = profile
            .world
            .snapshot()
            .navigation
            .unwrap_or_else(|| panic!("navigation"));
        assert_eq!((nav.fuel_units, nav.fuel_capacity_units), (200, 200));
        assert_eq!(profile.world.modular_equipment_snapshot(), before_modules);
        let disk = ok(std::fs::read(&path));
        assert_eq!(disk, ok(profile.world.save_bytes()));
        let recovered = ok(World::from_save_bytes(&disk));
        // Continue advances the mission epoch and review sequence deliberately. Compare the
        // authoritative recovered game facts, not a new serialization of the invalidated session.
        assert_eq!(recovered.snapshot().navigation, Some(nav));
        assert_eq!(recovered.credits_tenths(), Some(before_cash - 140));
        assert_eq!(recovered.modular_equipment_snapshot(), before_modules);
        assert_eq!(recovered.commander_id(), profile.world.commander_id());
        assert_eq!(recovered.player_ship_id(), profile.world.player_ship_id());
        assert_eq!(
            recovered.current_dock_context(),
            profile.world.current_dock_context()
        );
        assert_eq!(
            recovered.service_snapshot(),
            profile.world.service_snapshot()
        );

        let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
        let hit = frame
            .hits
            .iter()
            .find(|hit| hit.control == ui::Control::Key(refuel))
            .unwrap_or_else(|| panic!("refuel control"));
        assert!(!hit.enabled);
        let point = [hit.bounds.x + 5.0, hit.bounds.y + 5.0];
        menu.presented(frame);
        let full = ok(profile.world.save_bytes());
        ok(menu.input(&mut profile, ui::Input::Click(point)));
        assert_visible_feedback(&mut menu, &profile, "Fuel tank is already full");
        ok(menu.input(&mut profile, ui::Input::Key(refuel)));
        assert_eq!(ok(profile.world.save_bytes()), full);
        assert_eq!(ok(std::fs::read(&path)), full);
    }
    #[test]
    fn market_pointer_refuel_uses_the_same_paid_command() {
        let (_directory, mut profile, mut menu) = start();
        fit_paid_tank(&mut profile);
        let before_modules = profile.world.modular_equipment_snapshot();
        let cash = profile.world.credits_tenths().unwrap_or_default();
        let viewport = ui::Viewport::new(1280.0, 720.0).unwrap_or_else(|| panic!("viewport"));
        let frame = ok(menu.build_frame(&profile.world, viewport, 0.0));
        let hit = frame
            .hits
            .iter()
            .find(|hit| hit.control == ui::Control::Key(ui::Key::Refuel) && hit.enabled)
            .unwrap_or_else(|| panic!("enabled refuel control"));
        let point = [hit.bounds.x + 5.0, hit.bounds.y + 5.0];
        menu.presented(frame);
        ok(menu.input(&mut profile, ui::Input::Click(point)));
        assert_eq!(profile.world.credits_tenths(), Some(cash - 140));
        assert_eq!(
            profile
                .world
                .snapshot()
                .navigation
                .map(|nav| nav.fuel_units),
            Some(200)
        );
        assert_eq!(profile.world.modular_equipment_snapshot(), before_modules);
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
    fn independent_work_target_slots_project_shop_and_secret_fits() {
        let (_directory, mut profile, _) = start();
        for (slot, node, price) in [
            ("targeting-mining", "Targeting: Mining Module", 20_000),
            ("targeting-salvage", "Targeting: Salvage Module", 40_000),
        ] {
            let panel = ok(equipment_panel(&profile.world, slot, ""));
            let offer = panel
                .rows
                .iter()
                .find(|row| row.node == node)
                .unwrap_or_else(|| panic!("missing {node}"));
            assert_eq!(offer.price_tenths, Some(price));
            assert!(!offer.can_buy);
        }
        ok(profile.xyzzy(meridian_core::XyzzyAction::Enter));
        for (slot, node) in [
            ("targeting-mining", "Targeting: Mining Module"),
            ("targeting-salvage", "Targeting: Salvage Module"),
        ] {
            let panel = ok(equipment_panel(&profile.world, slot, ""));
            assert!(
                panel
                    .rows
                    .iter()
                    .any(|row| row.node == node && row.fitted == 1)
            );
        }
    }

    #[test]
    fn cabin_offers_and_separate_g04_review_use_real_demands_and_persist_refusal() {
        let (directory, mut profile, mut menu) = start();
        let panel = ok(equipment_panel(&profile.world, "passenger-cabins", ""));
        assert_eq!(panel.rows.len(), 6);
        for grade in meridian_core::CabinGrade::ALL {
            let row = panel
                .rows
                .iter()
                .find(|row| row.node == grade.name())
                .unwrap_or_else(|| panic!("missing cabin"));
            assert_eq!(row.price_tenths, Some(u64::from(grade.price_tenths())));
            assert!(!row.can_buy);
            assert_eq!(row.fitted, 0);
        }
        let panel = ok(equipment_panel(&profile.world, "passenger-service", ""));
        assert!(
            panel
                .rows
                .iter()
                .any(|row| row.node == "g04-request" && row.can_buy)
        );
        assert!(
            panel
                .rows
                .iter()
                .any(|row| row.node == "g04-board" && !row.can_buy)
        );
        assert!(
            equipment_action(
                &profile.world,
                "g04-request",
                None,
                "passenger-cabins",
                ui::EquipmentOperation::Buy
            )
            .is_err()
        );
        ok(menu.open(&profile.world, ui::Screen::Equipment));
        menu.slot = "passenger-service".into();
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
        assert!(
            menu.pending
                .as_ref()
                .is_some_and(|pending| pending.view.title == "Passenger service review")
        );
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
        let snapshot = profile
            .world
            .passenger_journey_snapshot()
            .unwrap_or_else(|| panic!("passage"));
        assert_eq!(snapshot.status, PassageStatus::Refused);
        assert_eq!(profile.world.credits_tenths(), Some(7200));
        drop(profile);
        let mut continued = open(&directory.0);
        ok(continued.continue_saved());
        let restored = continued
            .world
            .passenger_journey_snapshot()
            .unwrap_or_else(|| panic!("passage"));
        assert_eq!(
            (restored.status, restored.people, restored.cabins),
            (snapshot.status, snapshot.people, None)
        );
        assert!(!restored.reply_is_current);
    }

    #[test]
    fn saved_route_requests_publish_before_commit_and_cold_continue_keeps_the_selected_course() {
        let (directory, mut profile, mut menu) = start();
        let plan = ok(profile.world.route_alternatives(SystemId::LARERIA))[0]
            .plan
            .clone();
        ok(profile.save());
        let path = directory.0.join("commander.meridian");
        let backup = directory.0.join("previous-save");
        let original = ok(std::fs::read(&path));
        ok(std::fs::rename(&path, &backup));
        ok(std::fs::create_dir(&path));
        assert!(
            menu.request(&mut profile, ui::Request::SetNavigationRoute(plan.clone()))
                .is_err()
        );
        assert!(profile.world.active_route_progress().is_none());
        assert_eq!(ok(std::fs::read(&backup)), original);
        ok(std::fs::remove_dir(&path));
        ok(std::fs::rename(&backup, &path));
        ok(menu.request(&mut profile, ui::Request::SetNavigationRoute(plan.clone())));
        let restored = ok(World::from_save_bytes(&ok(std::fs::read(&path))));
        assert_eq!(
            ok(restored.active_route_progress().ok_or("route")).plan,
            plan
        );
        ok(menu.request(&mut profile, ui::Request::ClearNavigationRoute));
        let restored = ok(World::from_save_bytes(&ok(std::fs::read(&path))));
        assert!(restored.active_route_progress().is_none());
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
    fn lareria_is_an_admitted_route_but_the_stock_fit_refuses_its_distance() {
        let (_directory, mut profile, mut menu) = start();
        assert!(ROUTE_SYSTEMS.contains(&SystemId::LARERIA));
        let before = ok(profile.world.save_bytes());
        let refusal = menu
            .request(&mut profile, ui::Request::ReviewRoute(SystemId::LARERIA))
            .err()
            .unwrap_or_else(|| panic!("stock navigation admitted Lareria"));
        assert!(refusal.contains("exceeds available range"));
        assert!(menu.pending.is_none());
        assert_eq!(ok(profile.world.save_bytes()), before);
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
    fn shield_panel_exposes_source_grades_and_distinguishes_unavailable_acquisition() {
        use meridian_core::ShieldGrade;
        let (_directory, profile, _menu) = start();
        let panel = ok(equipment_panel(&profile.world, "shield", ""));
        let slot = panel
            .slots
            .iter()
            .find(|slot| slot.id == "shield")
            .unwrap_or_else(|| panic!("shield slot"));
        assert_eq!(slot.fitted, "Basic Shield");
        for grade in ShieldGrade::ALL {
            let row = panel
                .rows
                .iter()
                .find(|row| row.node == grade.node())
                .unwrap_or_else(|| panic!("missing shield grade"));
            assert!(row.readout.iter().any(|line| {
                line.contains(&format!("Capacity {} per bank", grade.capacity() / 100))
            }));
            assert!(!row.can_buy);
            if let Some((price, _)) = grade.ordinary_offer() {
                assert_eq!(row.price_tenths, Some(u64::from(price)));
                assert_eq!(row.instance, None);
            } else if grade == ShieldGrade::Basic {
                assert_eq!(row.fitted, 1);
                assert!(!row.can_store);
                assert!(row.instance.is_some());
            } else {
                assert!(row.refusal.is_some());
                assert_eq!(row.price_tenths, None);
                assert_eq!(row.instance, None);
            }
        }
        let capital = panel
            .rows
            .iter()
            .find(|row| row.node == ShieldGrade::AdvancedII.node())
            .unwrap_or_else(|| panic!("capital row"));
        assert!(
            capital.compatibility.contains("ELITE-rank") && capital.compatibility.contains("PLUGH")
        );
        assert!(
            equipment_action(
                &profile.world,
                ShieldGrade::BasicII.node(),
                None,
                "energy",
                ui::EquipmentOperation::Buy
            )
            .is_err()
        );
    }

    #[test]
    fn hyperspace_drive_panel_uses_actual_fitted_item_and_paid_offer_rules() {
        use meridian_core::navigation::DriveGrade;
        let (_directory, profile, _menu) = start();
        let panel = ok(equipment_panel(&profile.world, "drive", ""));
        let slot = panel
            .slots
            .iter()
            .find(|slot| slot.id == "drive")
            .unwrap_or_else(|| panic!("missing drive slot"));
        assert_eq!(slot.fitted, "Basic Hyperspace Drive");
        let basic = panel
            .rows
            .iter()
            .find(|row| row.node == ModuleKind::Drive(DriveGrade::Basic).node())
            .unwrap_or_else(|| panic!("missing born drive"));
        assert_eq!(basic.fitted, 1);
        assert!(!basic.can_store);
        assert!(basic.readout.iter().any(|line| line.contains("Spool 10s")));
        for (grade, price, text) in [
            (DriveGrade::Intermediate, 50_000, "Spool 5s"),
            (DriveGrade::Advanced, 150_000, "Spool 1s"),
        ] {
            let kind = ModuleKind::Drive(grade);
            let row = panel
                .rows
                .iter()
                .find(|row| row.node == kind.node())
                .unwrap_or_else(|| panic!("missing drive offer"));
            assert_eq!(row.price_tenths, Some(price));
            assert!(!row.can_buy);
            assert!(row.readout.iter().any(|line| line.contains(text)));
            assert!(
                equipment_action(
                    &profile.world,
                    kind.node(),
                    None,
                    "energy",
                    ui::EquipmentOperation::Buy
                )
                .is_err()
            );
        }
    }

    #[test]
    fn navigation_computer_panel_uses_owned_slot_and_source_prices() {
        let (_directory, profile, _menu) = start();
        let panel = ok(equipment_panel(&profile.world, "navigation", ""));
        let slot = panel
            .slots
            .iter()
            .find(|slot| slot.id == "navigation")
            .unwrap_or_else(|| panic!("Navigation computer slot"));
        assert_eq!(slot.fitted, "Basic Navigation Computer");
        assert_eq!(profile.world.jump_reach_ly(), Some(9.0));
        assert!(
            panel
                .rows
                .iter()
                .any(|row| row.node == ModuleKind::BasicNavigation.node()
                    && row.instance.is_some()
                    && row.fitted == 1
                    && row.price_tenths.is_none())
        );
        for (kind, price_tenths) in [
            (ModuleKind::IntermediateNavigation, 30_000),
            (ModuleKind::AdvancedNavigation, 90_000),
        ] {
            assert!(panel.rows.iter().any(|row| row.node == kind.node()
                && row.price_tenths == Some(price_tenths)
                && !row.can_buy));
        }
    }

    #[test]
    fn front_module_rows_and_pylon_slots_use_real_owned_instances_and_admission() {
        let (_directory, profile, _menu) = start();
        let panel = ok(equipment_panel(&profile.world, "front-laser", ""));
        assert_eq!(panel.slots.len(), 23);
        let actual: Vec<_> = panel.slots.iter().map(|slot| slot.id.as_str()).collect();
        assert_eq!(
            actual,
            [
                "front-laser",
                "cargo-expansion",
                "energy",
                "navigation",
                "drive",
                "shield",
                ui::fuel_scoops::SLOT,
                ui::emp::SLOT,
                "passenger-cabins",
                "passenger-service",
                "pylon-0",
                "pylon-1",
                "pylon-2",
                "pylon-3",
                "pylon-4",
                "pylon-5",
                "pylon-6",
                "pylon-7",
                "pylon-8",
                "targeting",
                "targeting-mining",
                "targeting-salvage",
                "fuel-storage"
            ]
        );
        let emp = panel
            .slots
            .iter()
            .find(|slot| slot.id == ui::emp::SLOT)
            .unwrap_or_else(|| panic!("EMP utility slot"));
        assert_eq!(emp.fitted, "Empty");
        let utility = panel
            .slots
            .iter()
            .find(|slot| slot.id == ui::fuel_scoops::SLOT)
            .unwrap_or_else(|| panic!("Fuel Scoops utility slot"));
        assert_eq!(utility.fitted, "Empty");
        assert!(
            panel
                .rows
                .iter()
                .all(|row| row.node != ModuleKind::FuelScoops.node())
        );
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

    #[test]
    fn presented_module_store_and_refit_keep_the_same_item_and_publish_before_continue() {
        let (directory, mut profile, mut menu) = start();
        ok(menu.open(&profile.world, ui::Screen::Equipment));
        let original = profile
            .world
            .modular_equipment_snapshot()
            .unwrap_or_else(|| panic!("Missing admitted equipment state"));
        let item = original.items[0];
        let credits = profile.world.credits_tenths();
        let resources = profile.world.flight_resources_snapshot();
        for operation in [ui::EquipmentOperation::Store, ui::EquipmentOperation::Fit] {
            let before = ok(profile.world.save_bytes());
            ok(menu.request(
                &mut profile,
                ui::Request::ReviewEquipment {
                    node: item.kind.node().into(),
                    instance: Some(item.id),
                    slot: "front-laser".into(),
                    operation,
                    option: None,
                },
            ));
            assert_eq!(ok(profile.world.save_bytes()), before);
            let review = &menu
                .pending
                .as_ref()
                .unwrap_or_else(|| panic!("Missing admitted equipment state"))
                .view;
            assert_eq!(review.title, "Equipment review");
            assert!(review.details.iter().any(|s| s.contains("no charge")));
            present(&mut menu, &profile);
            ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
            let expected_slot =
                (operation == ui::EquipmentOperation::Fit).then_some(ModuleSlot::FrontLaser);
            let current = profile
                .world
                .modular_equipment_snapshot()
                .unwrap_or_else(|| panic!("Missing admitted equipment state"));
            assert_eq!(current.items.len(), original.items.len());
            assert_eq!(current.items[0].id, item.id);
            assert_eq!(current.items[0].kind, item.kind);
            assert_eq!(current.items[0].slot, expected_slot);
            assert_eq!(profile.world.credits_tenths(), credits);
            assert_eq!(profile.world.flight_resources_snapshot(), resources);
            let disk = ok(std::fs::read(directory.0.join("commander.meridian")));
            assert_eq!(disk, ok(profile.world.save_bytes()));
            let cold = ok(World::from_save_bytes(&disk));
            assert_eq!(cold.modular_equipment_snapshot(), Some(current));
            assert_eq!(cold.flight_resources_snapshot(), resources);
            assert!(menu.pending.is_none());
        }
    }

    #[test]
    fn missile_offer_review_charges_once_and_failed_writer_cannot_publish_a_fitting() {
        let (directory, mut profile, mut menu) = start();
        ok(profile.save());
        ok(menu.open(&profile.world, ui::Screen::Equipment));
        ok(menu.request(
            &mut profile,
            ui::Request::InspectEquipmentSlot("pylon-3".into()),
        ));
        let before = ok(profile.world.save_bytes());
        ok(menu.request(
            &mut profile,
            ui::Request::ReviewEquipment {
                node: "missile-standard".into(),
                instance: None,
                slot: "pylon-3".into(),
                operation: ui::EquipmentOperation::Buy,
                option: None,
            },
        ));
        assert_eq!(ok(profile.world.save_bytes()), before);
        assert!(
            menu.pending
                .as_ref()
                .unwrap_or_else(|| panic!("Missing admitted equipment state"))
                .view
                .details
                .iter()
                .any(|s| s == "Offer 180.0 CR")
        );
        present(&mut menu, &profile);
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
        assert_eq!(profile.world.credits_tenths(), Some(5400));
        let ammo = profile
            .world
            .ammunition_snapshot()
            .unwrap_or_else(|| panic!("Missing admitted equipment state"));
        assert_eq!(ammo.items.iter().filter(|i| !i.consumed).count(), 4);
        let saved = ok(profile.world.save_bytes());
        assert_eq!(
            ok(std::fs::read(directory.0.join("commander.meridian"))),
            saved
        );
        ok(menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm)));
        assert_eq!(ok(profile.world.save_bytes()), saved);

        let item = ammo
            .items
            .iter()
            .find(|i| i.pylon == Some(3) && !i.consumed)
            .unwrap_or_else(|| panic!("Missing admitted equipment state"));
        ok(menu.request(
            &mut profile,
            ui::Request::ReviewEquipment {
                node: "missile-standard".into(),
                instance: Some(item.item),
                slot: "pylon-3".into(),
                operation: ui::EquipmentOperation::Store,
                option: None,
            },
        ));
        present(&mut menu, &profile);
        let path = directory.0.join("commander.meridian");
        let backup = directory.0.join("previous-save");
        ok(std::fs::rename(&path, &backup));
        ok(std::fs::create_dir(&path));
        assert!(
            menu.input(&mut profile, ui::Input::Key(ui::Key::Confirm))
                .is_err()
        );
        assert_eq!(ok(profile.world.save_bytes()), saved);
        assert_eq!(ok(std::fs::read(&backup)), saved);
        assert!(menu.pending.is_none());
        ok(std::fs::remove_dir(&path));
        ok(std::fs::rename(&backup, &path));
    }

    #[test]
    fn invalid_equipment_reviews_preserve_game_and_never_create_confirmation_authority() {
        let (_directory, mut profile, mut menu) = start();
        ok(menu.open(&profile.world, ui::Screen::Equipment));
        let item = profile
            .world
            .modular_equipment_snapshot()
            .unwrap_or_else(|| panic!("Missing admitted equipment state"))
            .items[0];
        let before = ok(profile.world.save_bytes());
        for (node, instance, slot, operation, option, reason) in [
            (
                item.kind.node(),
                Some(item.id),
                "front-laser",
                ui::EquipmentOperation::Store,
                Some("invented".into()),
                "no configurable option",
            ),
            (
                item.kind.node(),
                Some(item.id),
                "cargo-expansion",
                ui::EquipmentOperation::Store,
                None,
                "slot changed",
            ),
            (
                "unknown-module",
                Some(item.id),
                "front-laser",
                ui::EquipmentOperation::Fit,
                None,
                "selection changed",
            ),
            (
                item.kind.node(),
                Some(u64::MAX),
                "front-laser",
                ui::EquipmentOperation::Fit,
                None,
                "selection changed",
            ),
            (
                item.kind.node(),
                Some(item.id),
                "front-laser",
                ui::EquipmentOperation::Configure,
                None,
                "unavailable",
            ),
            (
                ModuleKind::MiningLaser.node(),
                None,
                "front-laser",
                ui::EquipmentOperation::Buy,
                None,
                "InsufficientCredits",
            ),
        ] {
            let refusal = menu
                .request(
                    &mut profile,
                    ui::Request::ReviewEquipment {
                        node: node.into(),
                        instance,
                        slot: slot.into(),
                        operation,
                        option,
                    },
                )
                .err()
                .unwrap_or_else(|| panic!("Invalid equipment request was accepted"));
            assert!(refusal.contains(reason), "{refusal}");
            assert!(menu.pending.is_none());
            assert_eq!(ok(profile.world.save_bytes()), before);
        }
    }
    #[test]
    fn energy_rows_keep_the_real_born_item_out_of_front_laser_rows() {
        let (_directory, profile, _menu) = start();
        let front = ok(equipment_panel(&profile.world, "front-laser", ""));
        assert!(
            front
                .rows
                .iter()
                .all(|row| row.node != ModuleKind::BasicEnergySystem.node())
        );
        let energy = ok(equipment_panel(&profile.world, "energy", ""));
        let basic = energy
            .rows
            .iter()
            .find(|row| row.node == ModuleKind::BasicEnergySystem.node())
            .unwrap_or_else(|| panic!("born energy row"));
        assert_eq!((basic.owned, basic.fitted), (1, 1));
        assert_eq!(basic.instance, Some(89816));
        assert!(!basic.can_buy);
        assert!(basic.can_store);
        let extra = energy
            .rows
            .iter()
            .find(|row| row.node == ModuleKind::ExtraEnergyUnit.node())
            .unwrap_or_else(|| panic!("paid energy offer"));
        assert_eq!(extra.price_tenths, Some(72000));
        assert!(!extra.can_buy);
        assert_eq!(energy.equipment_units, [0, 24]);
    }
    #[test]
    fn energy_store_and_refit_publish_the_same_item_without_displacing_the_front_laser() {
        let (directory, mut profile, _menu) = start();
        let before_pulse = profile.world.pulse_laser_snapshot();
        let action = ok(equipment_action(
            &profile.world,
            ModuleKind::BasicEnergySystem.node(),
            Some(89816),
            "energy",
            ui::EquipmentOperation::Store,
        ));
        ok(profile.command(action));
        let action = ok(equipment_action(
            &profile.world,
            ModuleKind::BasicEnergySystem.node(),
            Some(89816),
            "energy",
            ui::EquipmentOperation::Fit,
        ));
        assert!(matches!(
            action,
            Action::Module(ModuleAction::Install {
                item: 89816,
                replacing: None
            })
        ));
        ok(profile.command(action));
        assert_eq!(profile.world.pulse_laser_snapshot(), before_pulse);
        assert_eq!(profile.world.credits_tenths(), Some(7200));
        let bytes = ok(profile.world.save_bytes());
        assert_eq!(
            ok(std::fs::read(directory.0.join("commander.meridian"))),
            bytes
        );
        let loaded = ok(World::from_save_bytes(&bytes));
        assert_eq!(
            loaded.modular_equipment_snapshot(),
            profile.world.modular_equipment_snapshot()
        );
    }
    #[test]
    fn ordinary_beam_intent_still_requires_its_paid_admission_and_naval_has_no_shop_alias() {
        let (_directory, mut profile, _menu) = start();
        let before = ok(profile.world.save_bytes());
        let action = ok(equipment_action(
            &profile.world,
            ModuleKind::BeamLaser.node(),
            None,
            "front-laser",
            ui::EquipmentOperation::Buy,
        ));
        assert!(matches!(
            action,
            Action::Module(ModuleAction::Purchase {
                kind: ModuleKind::BeamLaser
            })
        ));
        assert!(profile.command(action).is_err());
        assert_eq!(ok(profile.world.save_bytes()), before);
        assert!(
            equipment_action(
                &profile.world,
                ModuleKind::NavalLaser.node(),
                None,
                "front-laser",
                ui::EquipmentOperation::Buy
            )
            .is_err()
        );
        assert_eq!(ok(profile.world.save_bytes()), before);
    }
}
