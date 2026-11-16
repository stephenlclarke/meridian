//! Host menu state is ephemeral. Only Profile's prepared commands can change the game.
use crate::{docked_screens as ui, profile::Profile, session::Action};
use meridian_core::{
    AmmunitionAction, ModuleAction, ModuleKind, ModuleSlot, World, equipment,
    navigation::{DockContext, STOCK_JUMP_REACH_LY, SystemId},
    station_services::{self, CommodityId},
};

const ROUTES: [SystemId; 2] = [SystemId::AVENOR, SystemId::RENAS];
const MODELS: [ui::AvailableModel<'static>; 1] = [ui::AvailableModel {
    object: "SH025",
    geometry_key: "SH025",
}];
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
        models: &MODELS,
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
        let ids = CommodityId::display_order().map_err(|e| e.to_string())?;
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
            ui::Request::ReviewCommodity { commodity, buy } => {
                let quote = world
                    .commodity_quote(commodity)
                    .ok_or("No current commodity offer")?;
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
                        format!("One native unit | {}.{} CR", price / 10, price % 10),
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
                            && (operation == ui::EquipmentOperation::Buy || r.instance == instance)
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
