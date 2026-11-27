//! The three integral Naval mounts belong to the first ordinary Fleet Medusa issue.
use super::{FlightView, FrontLaserSnapshot, OrdinaryWeapon, World};
use crate::docking::{Basis3, RigidPose, Vec3};

pub(super) const INTEGRAL: [FlightView; 3] = [FlightView::Left, FlightView::Right, FlightView::Rear];

impl FlightView {
    pub(super) fn laser_ray(self, body: RigidPose) -> RigidPose {
        let neg = |v:Vec3| Vec3::new(-v.x,-v.y,-v.z);
        let b=body.orientation;
        let (x,z)=match self {
            Self::Front => (b.x,b.z), Self::Left=>(b.z,neg(b.x)),
            Self::Right=>(neg(b.z),b.x), Self::Rear=>(neg(b.x),neg(b.z)),
        };
        RigidPose {position:body.position,orientation:Basis3{x,y:b.y,z}}
    }
}
impl World {
    pub fn selected_laser_view(&self) -> FlightView {
        if self.xyzzy_medusa_active() {self.flight_view()} else {FlightView::Front}
    }
    pub(super) fn integral_laser(&self, facing:FlightView) -> Option<u64> {
        let index=INTEGRAL.iter().position(|v|*v==facing)? as u64;
        let (dock,_,revision)=self.xyzzy_medusa_issue()?;
        // All older ship inventory, ammunition and EMP IDs use offsets through 400011.
        let offset=revision.checked_mul(3)?.checked_add(500_011+index)?;
        let owner=dock.owner().seed();
        owner.checked_add(offset).or_else(||owner.checked_sub(offset))
    }
    pub(super) fn selected_fit(&self)->Option<(u64,OrdinaryWeapon)> {
        let facing=self.selected_laser_view();
        if facing==FlightView::Front {self.fitted_front_laser()} else {Some((self.integral_laser(facing)?,OrdinaryWeapon::FrontNavalLaser))}
    }
    pub fn selected_laser_snapshot(&self)->Option<FrontLaserSnapshot> {
        let (fitted_item_id,weapon)=self.selected_fit()?;
        Some(FrontLaserSnapshot{fitted_item_id,weapon,damage_hundredths:weapon.damage_hundredths(),range:6000,cooldown_ticks_per_shot:weapon.cooldown_ticks()})
    }
    pub(super) fn integral_view_at(&self,item:u64,tick:u64)->Option<FlightView>{
        let (_,issue_tick,_)=self.xyzzy_medusa_issue()?;
        if tick<=issue_tick {return None;}
        INTEGRAL.into_iter().find(|v|self.integral_laser(*v)==Some(item))
    }
    pub(super) fn laser_weapon_at_tick(&self,item:u64,tick:u64,emission:u64)->Option<OrdinaryWeapon>{
        if self.integral_view_at(item,tick).is_some(){Some(OrdinaryWeapon::FrontNavalLaser)}else{self.front_weapon_at_tick(item,tick,emission)}
    }
}
