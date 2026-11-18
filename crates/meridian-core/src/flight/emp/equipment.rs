//! Actual berth transactions. Purchase/grade costs enter the same account and prefix proof.
use super::{
    Arc, DockContext, EmpAxis, EmpEquipmentAction, EmpEquipmentReview, EmpError, EmpGrades,
    EmpProvenance, EquipmentEvent, FlightMode, INVALID, MAX_EQUIPMENT_EVENTS, OwnedEmp, SaveError,
    State, World, policy,
};
use crate::freight::g02::CommanderAccount;
pub struct PreparedEmpEquipment<'a> {
    world: &'a mut World,
    state: State,
    account: CommanderAccount,
    bytes: Vec<u8>,
}
impl PreparedEmpEquipment<'_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn publish_with<E>(self, writer: impl FnOnce(&[u8]) -> Result<(), E>) -> Result<(), E> {
        writer(&self.bytes)?;
        self.commit();
        Ok(())
    }
    pub fn commit(self) {
        self.world.commander_account = Some(self.account);
        self.world.emp = self.state;
    }
}
impl State {
    pub(in crate::flight) fn account_events(
        &self,
    ) -> Result<Vec<super::super::AccountEvent>, SaveError> {
        let mut grades = EmpGrades::default();
        let mut events = Vec::new();
        for event in self.equipment.iter() {
            let price = match event.action {
                EmpEquipmentAction::Purchase => {
                    if event.secret {
                        0
                    } else {
                        policy::BASE_PRICE
                    }
                }
                EmpEquipmentAction::Upgrade { axis, .. } => {
                    let (price, _) = grades.upgrade_offer(axis).map_err(|_| INVALID)?;
                    grades = grades.advancing(axis).map_err(|_| INVALID)?;
                    price
                }
                _ => 0,
            };
            events.push(super::super::AccountEvent {
                revision: event.revision,
                effect: super::super::AccountEffect::Debit(price),
            });
        }
        Ok(events)
    }
}
impl World {
    pub fn owned_emp(&self) -> Option<OwnedEmp> {
        self.emp.owned().ok().flatten()
    }
    pub fn review_emp_equipment(
        &self,
        action: EmpEquipmentAction,
        tick: u64,
    ) -> Result<EmpEquipmentReview, EmpError> {
        if tick != self.state.tick {
            return Err(EmpError::StaleReview);
        }
        if self.is_paused() {
            return Err(EmpError::Paused);
        }
        if self.state.mode != FlightMode::Docked {
            return Err(EmpError::BerthRequired);
        }
        let account = self
            .commander_account
            .as_ref()
            .ok_or(EmpError::NoCommander)?;
        let dock = self.current_dock_context().ok_or(EmpError::BerthRequired)?;
        self.current_service_facts(tick)
            .map_err(|_| EmpError::BerthRequired)?;
        let current = self.emp.owned().map_err(EmpError::Save)?;
        let (price, technology) = match action {
            EmpEquipmentAction::Purchase => {
                if current.is_some() {
                    return Err(EmpError::AlreadyOwned);
                }
                (policy::BASE_PRICE, policy::BASE_TECHNOLOGY)
            }
            EmpEquipmentAction::Upgrade { item, axis } => {
                let current = current
                    .filter(|i| i.item == item)
                    .ok_or(EmpError::MissingItem)?;
                if current.provenance != EmpProvenance::Paid {
                    return Err(EmpError::PaidBaseRequired);
                }
                current.grades.upgrade_offer(axis)?
            }
            EmpEquipmentAction::Fit { item } => {
                if current
                    .filter(|i| i.item == item)
                    .ok_or(EmpError::MissingItem)?
                    .fitted
                {
                    return Err(EmpError::NoChange);
                }
                (0, 0)
            }
            EmpEquipmentAction::Store { item } => {
                if !current
                    .filter(|i| i.item == item)
                    .ok_or(EmpError::MissingItem)?
                    .fitted
                {
                    return Err(EmpError::NoChange);
                }
                (0, 0)
            }
        };
        let definition = crate::equipment::definition().map_err(|_| EmpError::BerthRequired)?;
        let site =
            super::super::services::service_site(dock.system()).ok_or(EmpError::BerthRequired)?;
        if definition.offer(site).technology < technology {
            return Err(EmpError::TechnologyUnavailable);
        }
        if account.credits_tenths() < price {
            return Err(EmpError::InsufficientFunds);
        }
        if matches!(
            action,
            EmpEquipmentAction::Purchase | EmpEquipmentAction::Fit { .. }
        ) {
            let used = self
                .fuel_scoops_snapshot()
                .ok_or(EmpError::BerthRequired)?
                .utility_mounts_used;
            if used >= definition.utility_mounts_capacity {
                return Err(EmpError::UtilityCapacity);
            }
        }
        if self.emp.equipment.len() >= MAX_EQUIPMENT_EVENTS {
            return Err(EmpError::Capacity);
        }
        Ok(EmpEquipmentReview {
            action,
            dock,
            tick,
            epoch: self
                .freight_session
                .as_ref()
                .ok_or(EmpError::NoCommander)?
                .epoch(),
            revision: account.revision(),
        })
    }
    pub fn prepare_emp_equipment(
        &mut self,
        review: EmpEquipmentReview,
        tick: u64,
    ) -> Result<PreparedEmpEquipment<'_>, EmpError> {
        if self.review_emp_equipment(review.action, tick)? != review {
            return Err(EmpError::StaleReview);
        }
        let mut state = self.emp.clone();
        let current = state.owned().map_err(EmpError::Save)?;
        if review.action == EmpEquipmentAction::Purchase {
            state.meter_clock = self.combat.physical_tick();
            state.bank_clock = state.meter_clock;
        }
        if matches!(review.action, EmpEquipmentAction::Upgrade { .. }) {
            state.bank_clock = self.combat.physical_tick();
        }
        let price = match review.action {
            EmpEquipmentAction::Purchase => policy::BASE_PRICE,
            EmpEquipmentAction::Upgrade { axis, .. } => {
                current
                    .ok_or(EmpError::MissingItem)?
                    .grades
                    .upgrade_offer(axis)?
                    .0
            }
            _ => 0,
        };
        let account = self
            .commander_account
            .as_ref()
            .ok_or(EmpError::NoCommander)?
            .prepare_emp_equipment(price)
            .map_err(|_| EmpError::Capacity)?;
        Arc::make_mut(&mut state.equipment).push(EquipmentEvent {
            secret: false,
            action: review.action,
            dock: review.dock,
            tick,
            revision: account.revision(),
        });
        state.owned().map_err(EmpError::Save)?;
        let bytes = self
            .save_bytes_with_emp(&account, &state)
            .map_err(EmpError::Save)?;
        Ok(PreparedEmpEquipment {
            world: self,
            state,
            account,
            bytes,
        })
    }
}

impl World {
    pub(in crate::flight) fn grant_xyzzy_emp(
        &mut self,
        dock: DockContext,
        tick: u64,
    ) -> Result<(), SaveError> {
        let current = self.emp.owned()?;
        if current.is_some_and(|item| item.fitted) {
            return Ok(());
        }
        if self.emp.equipment.len() >= MAX_EQUIPMENT_EVENTS {
            return Err(SaveError::TooLarge);
        }
        let account = self
            .commander_account
            .as_ref()
            .ok_or(INVALID)?
            .prepare_emp_equipment(0)
            .map_err(|_| INVALID)?;
        let action = current.map_or(EmpEquipmentAction::Purchase, |item| {
            EmpEquipmentAction::Fit { item: item.item }
        });
        let mut state = self.emp.clone();
        if current.is_none() {
            state.meter_clock = self.combat.physical_tick();
            state.bank_clock = state.meter_clock;
        }
        Arc::make_mut(&mut state.equipment).push(EquipmentEvent {
            secret: current.is_none(),
            action,
            dock,
            tick,
            revision: account.revision(),
        });
        state.owned()?;
        self.emp = state;
        self.commander_account = Some(account);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmpEquipmentOffer {
    pub action: EmpEquipmentAction,
    pub price_tenths: u32,
    pub required_technology: u8,
    pub technology: u8,
    pub refusal: Option<EmpError>,
}
impl World {
    /// Shop data is derived from the same grade policy and admission used by the transaction.
    pub fn emp_equipment_offers(&self) -> Vec<EmpEquipmentOffer> {
        let Some(dock) = self
            .current_dock_context()
            .filter(|_| self.state.mode == FlightMode::Docked)
        else {
            return Vec::new();
        };
        let Some(site) = super::super::services::service_site(dock.system()) else {
            return Vec::new();
        };
        let Ok(definition) = crate::equipment::definition() else {
            return Vec::new();
        };
        let technology = definition.offer(site).technology;
        let mut offers = Vec::new();
        let mut add = |action, price_tenths, required_technology| {
            offers.push(EmpEquipmentOffer {
                action,
                price_tenths,
                required_technology,
                technology,
                refusal: self.review_emp_equipment(action, self.state.tick).err(),
            })
        };
        if let Some(item) = self.owned_emp() {
            add(
                if item.fitted {
                    EmpEquipmentAction::Store { item: item.item }
                } else {
                    EmpEquipmentAction::Fit { item: item.item }
                },
                0,
                0,
            );
            for axis in [EmpAxis::Power, EmpAxis::Capacity, EmpAxis::Recharge] {
                if let Ok((price, required)) = item.grades.upgrade_offer(axis) {
                    add(
                        EmpEquipmentAction::Upgrade {
                            item: item.item,
                            axis,
                        },
                        price,
                        required,
                    );
                }
            }
        } else {
            add(
                EmpEquipmentAction::Purchase,
                policy::BASE_PRICE,
                policy::BASE_TECHNOLOGY,
            );
        }
        offers
    }
}
