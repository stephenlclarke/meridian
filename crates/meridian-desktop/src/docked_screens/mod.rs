//! Pure docked presentation. Root owns input bindings, World transactions and durable commit.
mod catalogue;
mod controller;
mod layout;
mod presentation;
#[cfg(test)]
mod tests;
pub use catalogue::{Catalogue, ObjectRecord, Specification, WorldRecord};
pub use controller::DockedScreens;
pub use layout::{ChartLayout, Rect, Viewport};
use meridian_core::{World, navigation::SystemId, station_services::CommodityId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Galaxy,
    System,
    Showroom,
    Trading,
    Equipment,
}
impl Screen {
    pub const ALL: [Self; 5] = [
        Self::Galaxy,
        Self::System,
        Self::Showroom,
        Self::Trading,
        Self::Equipment,
    ];
    pub const fn short_title(self) -> &'static str {
        match self {
            Self::Galaxy => "Galaxy",
            Self::System => "System",
            Self::Showroom => "Ships",
            Self::Trading => "Market",
            Self::Equipment => "Equip",
        }
    }
    pub const fn title(self) -> &'static str {
        match self {
            Self::Galaxy => "Galactic Chart",
            Self::System => "System Data",
            Self::Showroom => "Ship Showroom",
            Self::Trading => "Market",
            Self::Equipment => "Equip Ship",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Previous,
    Next,
    PagePrevious,
    PageNext,
    Confirm,
    Back,
    Search,
    Erase,
    ClearSearch,
    NextTab,
    PreviousTab,
    Buy,
    Sell,
    BuyMode,
    SellMode,
    LocalChart,
    MapColour,
    NextGalaxy,
    PreviousGalaxy,
    CurrentLocation,
    Compare,
    PinBaseline,
    CurrentShip,
    Rotate,
    NextAxis,
    TurnLeft,
    TurnRight,
    ResetView,
    NextSlot,
    Store,
    Fit,
    Options,
    Route,
    Exchange,
    Details,
    NextBody,
    PreviousBody,
    BodyRoute,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Key(Key),
    Text(String),
    PointerMoved([f64; 2]),
    Click([f64; 2]),
    Scroll {
        point: [f64; 2],
        rows: i32,
    },
    Drag {
        from: [f64; 2],
        to: [f64; 2],
        roll: bool,
    },
    FocusLost,
}
/// Offered keyboard text is supplied from the very same binding map used by Root's adapter.
#[derive(Debug, Clone, Copy)]
pub struct BindingHint<'a> {
    pub key: Key,
    pub label: &'a str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub session: u64,
    pub revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentOperation {
    Buy,
    Fit,
    Store,
    Configure,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Close,
    ReviewRoute(SystemId),
    ReviewBodyRoute(NavigationBodyId),
    ReviewCommodity {
        commodity: CommodityId,
        buy: bool,
    },
    ReviewEquipment {
        node: String,
        instance: Option<u64>,
        slot: String,
        operation: EquipmentOperation,
        option: Option<String>,
    },
    InspectEquipmentSlot(String),
    ReviewShipExchange(String),
    ConfirmReview(u64),
    CancelReview(u64),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Updated,
    Request(Request),
    Refused(String),
    Ignored,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferQuantity {
    NotTracked,
    Available(u64),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipmentOption {
    pub id: String,
    pub label: String,
    pub value: String,
    pub price_tenths: Option<u64>,
    pub refusal: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipmentRow {
    pub node: String,
    pub name: String,
    pub group: String,
    pub price_tenths: Option<u64>,
    pub quantity: OfferQuantity,
    pub instance: Option<u64>,
    pub owned: u32,
    pub fitted: u32,
    pub compatibility: String,
    pub refusal: Option<String>,
    pub readout: Vec<String>,
    pub options: Vec<EquipmentOption>,
    pub can_buy: bool,
    pub can_fit: bool,
    pub can_store: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipmentSlot {
    pub id: String,
    pub label: String,
    pub fitted: String,
    pub available: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipmentPanel {
    pub selected_slot: String,
    pub slots: Vec<EquipmentSlot>,
    pub rows: Vec<EquipmentRow>,
    pub equipment_units: [u32; 2],
    pub utility_mounts: [u32; 2],
    pub assigned_crew: u32,
    pub required_crew: u32,
    pub feedback: String,
}
#[derive(Debug, Clone, Copy)]
pub struct AvailableModel<'a> {
    pub object: &'a str,
    pub geometry_key: &'a str,
}
#[derive(Debug, Clone, Copy)]
pub struct ShipExchange<'a> {
    pub object: &'a str,
    pub purchase_tenths: u64,
    pub trade_in_tenths: u64,
    pub refusal: Option<&'a str>,
}
#[derive(Debug, Clone)]
pub struct Review {
    pub token: u64,
    pub title: String,
    pub details: Vec<String>,
    pub can_confirm: bool,
    pub refusal: Option<String>,
}
/// Borrowed host projections. No row, price, catalogue fact or Request is a purchase authority.
pub struct Context<'a> {
    pub world: &'a World,
    pub catalogue: &'a Catalogue,
    pub stamp: Stamp,
    pub current_hull: &'a str,
    pub paused: bool,
    pub current_reach_ly: Option<f64>,
    pub full_reach_ly: Option<f64>,
    pub route_targets: &'a [SystemId],
    pub models: &'a [AvailableModel<'a>],
    pub planet_previews: bool,
    pub navigation_bodies: &'a [NavigationBody],
    pub exchanges: &'a [ShipExchange<'a>],
    pub equipment: Option<&'a EquipmentPanel>,
    pub review: Option<&'a Review>,
    pub bindings: &'a [BindingHint<'a>],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NavigationBodyId {
    pub system: SystemId,
    pub ordinal: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationBody {
    pub id: NavigationBodyId,
    pub name: String,
    pub position_world: [f64; 3],
    pub distance_world: Option<f64>,
    pub can_target: bool,
    pub refusal: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    Black,
    White,
    Yellow,
    Red,
    Green,
    Cyan,
    Blue,
    RedOrange,
    Gold,
    Lime,
    Leaf,
    Orange,
}
impl Ink {
    pub const fn rgba(self) -> u32 {
        match self {
            Self::Black => 0x000000ff,
            Self::White => 0xffffffff,
            Self::Yellow => 0xffff00ff,
            Self::Red => 0xff0000ff,
            Self::Green => 0x00ff00ff,
            Self::Cyan => 0x00ffffff,
            Self::Blue => 0x4488ffff,
            Self::RedOrange => 0xff6622ff,
            Self::Gold => 0xffcc22ff,
            Self::Lime => 0xccee22ff,
            Self::Leaf => 0x66ee44ff,
            Self::Orange => 0xffaa44ff,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub text: String,
    pub at: [f64; 2],
    pub ink: Ink,
    pub clip: Rect,
    pub scale: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub ink: Ink,
    pub clip: Rect,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Marker {
    pub system: SystemId,
    pub point: [f64; 2],
    pub radius: f64,
    pub ink: Ink,
    pub current: bool,
    pub selected: bool,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeRing {
    pub centre: [f64; 2],
    pub radius: f64,
    pub ink: Ink,
    pub clip: Rect,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orientation {
    pub pitch: f64,
    pub roll: f64,
    pub yaw: f64,
}
impl Default for Orientation {
    fn default() -> Self {
        Self {
            pitch: 0.25,
            roll: 0.0,
            yaw: 0.0,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetPreview {
    pub system: SystemId,
    pub bounds: Rect,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPreview {
    pub object: String,
    pub geometry_key: String,
    pub bounds: Rect,
    pub orientation: Orientation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Control {
    Open(Screen),
    SelectSystem(SystemId),
    SelectShip(String),
    SelectCommodity(CommodityId),
    SelectEquipment(usize),
    SelectSlot(usize),
    SelectOption(usize),
    Key(Key),
    Galaxy(u8),
    Search,
    ReviewConfirm,
    ReviewCancel,
    TradeMode(bool),
    SelectBody(NavigationBodyId),
}
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Hit {
    pub bounds: Rect,
    pub control: Control,
    pub enabled: bool,
    pub refusal: Option<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub stamp: Stamp,
    pub ui_revision: u64,
    pub screen: Screen,
    pub viewport: Viewport,
    pub labels: Vec<Label>,
    pub lines: Vec<Line>,
    pub markers: Vec<Marker>,
    pub range_rings: Vec<RangeRing>,
    pub navigation_cues: Vec<NavigationBody>,
    pub models: Vec<ModelPreview>,
    pub planets: Vec<PlanetPreview>,
    pub hovered: Option<Rect>,
    pub search_active: bool,
    pub(super) hits: Vec<Hit>,
    pub(super) model_bounds: Option<Rect>,
}
impl Frame {
    pub fn actionable_regions(&self) -> impl Iterator<Item = (Rect, bool)> + '_ {
        self.hits.iter().map(|h| (h.bounds, h.enabled))
    }
}
pub(super) fn money(tenths: u64) -> String {
    format!("{}.{:01}", tenths / 10, tenths % 10)
}
pub(super) fn moved(index: usize, amount: i32, length: usize) -> usize {
    index
        .saturating_add_signed(amount as isize)
        .min(length.saturating_sub(1))
}
