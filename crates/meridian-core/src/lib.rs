//! Deterministic simulation primitives without window, GPU, filesystem or wall-clock access.

#![forbid(unsafe_code)]

pub mod alpaca;
pub mod antelope;
pub mod aplomado_interceptor;
pub mod arna_cutter;
pub mod badger;
pub mod banteng;
pub mod barbary;
pub mod baza_fighter;
pub mod baza_mk_ii;
pub mod carabao_freighter;
pub mod carabao_highliner;
pub mod caracal_scout;
pub mod celestial;
pub mod condor;
pub mod docking;
pub mod dzo;
pub mod dzo_mk_ii;
pub mod eagle_cruiser;
pub mod elk;
pub mod equipment;
pub mod erne_corvette;
pub mod falconet;
mod flight;
pub mod freight;
pub mod gopher;
pub mod goshawk;
pub mod goshawk_custom;
pub mod griffon;
pub mod griffon_mk_ii;
pub mod griffon_spectre;
pub mod hare_explorer;
pub mod harpy_assault;
pub mod harpy_dropship;
pub mod harpy_gunship;
pub mod hobby_fighter;
pub mod hog;
pub mod hyena;
pub mod jackal;
pub mod jaguar;
pub mod laggar;
pub mod lanner;
pub mod llama;
pub mod mining;
pub mod moose_clipper;
pub mod mule;
pub mod muskox;
pub mod navigation;
pub mod oryx_voyager;
pub mod rabbit_explorer;
pub mod rhinoceros_trader;
pub mod saiga;
pub mod saker;
pub mod save;
pub mod seriema_courier;
pub mod springbok_starliner;
pub mod station;
pub mod station_services;
pub mod tamaraw;
pub mod traffic;
pub mod vulture_raider;
pub mod wombat;
pub mod yak;

pub use flight::GoshawkCustomApproachSnapshot;
pub use flight::PreparedNavigationRoute;
pub use flight::ProfitDestination;
pub use flight::{GoshawkApproachSnapshot, GoshawkApproachStatus};
pub use flight::{PreparedXyzzy, XyzzyAction, XyzzyError, XyzzyReview, XyzzySnapshot};

pub mod stag_liner;

pub use flight::{
    ENERGY_CAPACITY_HUNDREDTHS, FIXED_DELTA_SECONDS, FireError, FlightInput, MAX_SPEED,
    MissionError, MissionSnapshot, PULSE_COOLDOWN_TICKS, PULSE_RANGE, PulseContact,
    PulseLaserSnapshot, PulseShot, SHIELD_CAPACITY, ServiceSnapshot, Snapshot, StationNpc,
    StepError, World,
};
pub use station::{FlightMode, FlightStart, MotionRefusal, OriginalStationData, StationError};

pub use flight::{
    PRACTICE_HALF_SIDE, PRACTICE_MAX_OFFERS, PRACTICE_QUIET_TICKS, PracticeAttemptId,
    PracticeAttemptSnapshot, PracticeBeaconId, PracticeBeaconSnapshot, PracticeClosureReview,
    PracticeError, PracticeLedgerSnapshot, PracticeOutcome, PracticeReceptionReview,
    PracticeReceptionSnapshot, PracticeReview, PracticeSnapshot, PracticeStatus, PracticeTrace,
    PreparedPracticeConfirmation,
};

pub use flight::{
    CombatIdentity, CombatSnapshot, OrdinaryImpact, OrdinaryTarget, OrdinaryWeapon, PlayerCondition,
};
pub use flight::{TargetFilter, TargetIdentity, TargetMode, TargetingSnapshot, TargetingViewport};

pub use flight::{
    MISSILE_COOLDOWN_TICKS, MISSILE_FACES, MISSILE_LIFETIME_TICKS, MISSILE_MUZZLE_DISTANCE,
    MISSILE_RANGE, MISSILE_SPEED, MISSILE_VERTICES, MissileContact, MissileEnd, MissileError,
    MissileOrigin, MissileRound, MissileSnapshot, MissileTarget, ProjectileId, ProjectileSnapshot,
};

pub use flight::{
    FLIGHT_RECEPTION_MAX_BERTH_RECORDS, FlightReceptionAction, FlightReceptionAttemptSnapshot,
    FlightReceptionError, FlightReceptionId, FlightReceptionOutcome, FlightReceptionReview,
    FlightReceptionSnapshot, FlightReceptionTrace, FlightView, PreparedFlightReception,
};

pub use flight::{
    NavigationTonneId, NavigationTradeAction, NavigationTradeAttemptSnapshot, NavigationTradeError,
    NavigationTradeId, NavigationTradeMarketSnapshot, NavigationTradeOutcome,
    NavigationTradePlanSnapshot, NavigationTradeReview, NavigationTradeSnapshot,
    NavigationTradeTrace, PreparedNavigationTrade, PreparedStationService,
};

pub use flight::{
    AmmunitionAction, AmmunitionError, AmmunitionItem, AmmunitionReview, AmmunitionSnapshot,
    MissilePurchase, PreparedAmmunition,
};

pub use flight::{
    FittedFuelScoops, FittedMiningTool, FuelScoopsSnapshot, ModularEquipmentSnapshot, ModuleAction,
    ModuleError, ModuleKind, ModuleOffer, ModuleProvenance, ModuleReview, ModuleSlot, OwnedModule,
    PreparedModule,
};

pub use flight::{FlightResourcesSnapshot, FrontLaserSnapshot};

pub use flight::{EquipmentPreparationError, PreparedEquipment};

pub use flight::{
    EmpAxis, EmpChargePhase, EmpEquipmentAction, EmpEquipmentOffer, EmpEquipmentReview, EmpError,
    EmpGrades, EmpOrigin, EmpProvenance, EmpSnapshot, OwnedEmp, PreparedEmpEquipment,
};
