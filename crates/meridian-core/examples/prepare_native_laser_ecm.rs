//! Explicit XYZZY acceptance preparation. Never native earning or a new commander/profile.
//! Reads an existing isolated current save; emits a distinct create-new staging file only.
use meridian_core::{
    FlightMode, LaserFacing, ModuleAction, ModuleKind, ModuleSlot, World, XyzzyAction,
    navigation::SystemId, save::MAX_SAVE_BYTES,
};
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const BASE_HEAD: &str = "3553ea5b5f99ca261ed472513c8b79afe28d1dcf";
const BASE_TREE: &str = "bda557c5e243b1e0a43eafd7297cbc7c9a64a250";

fn require(condition: bool, message: &'static str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn load(path: &Path) -> Result<World> {
    require(
        fs::symlink_metadata(path)?.file_type().is_file(),
        "Input must be an explicit regular isolated save, not a symlink",
    )?;
    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_SAVE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    require(
        bytes.len() <= MAX_SAVE_BYTES,
        "Input exceeds current save bound",
    )?;
    Ok(World::from_save_bytes(&bytes)?)
}
fn prepare(world: &mut World) -> Result<()> {
    require(world.snapshot().mode == FlightMode::Docked, "Dock first")?;
    require(
        world
            .current_dock_context()
            .is_some_and(|dock| dock.system() == SystemId::AVENOR),
        "An actual Avenor main berth is required for practice",
    )?;
    require(
        world
            .xyzzy_snapshot()
            .is_some_and(|snapshot| !snapshot.active),
        "Input must be an explicitly isolated normal commander, not an existing testing profile",
    )?;
    // Probe actual public admission before any mutation. No practice acceptance is injected.
    world.review_practice(world.snapshot().tick)?;
    let review = world.review_xyzzy(XyzzyAction::Enter, world.snapshot().tick)?;
    world.prepare_xyzzy(review, world.snapshot().tick)?.commit();
    require(
        world
            .xyzzy_snapshot()
            .is_some_and(|s| s.active && s.unlimited_fuel),
        "Public testing activation did not provide its documented mode",
    )?;
    let review = world.review_xyzzy(XyzzyAction::ToggleUnlimitedFuel, world.snapshot().tick)?;
    world.prepare_xyzzy(review, world.snapshot().tick)?.commit();
    require(
        world
            .xyzzy_snapshot()
            .is_some_and(|s| s.active && !s.unlimited_fuel),
        "Accounted fuel must remain enabled",
    )?;
    require(
        world
            .ecm_snapshot()
            .is_some_and(|s| s.fitted_item.is_none() && s.activations == 0),
        "Use a context without pre-fitted ECM or prior activations",
    )?;
    // Original XYZZY fills cargo + EMP + scoops: free one real utility mount for paid ECM.
    // Store only the granted scoops through the public owner, never edit a slot directly.
    let scoops = world
        .modular_equipment_snapshot()
        .and_then(|s| {
            s.items
                .into_iter()
                .find(|item| item.slot == Some(ModuleSlot::UtilityFuelScoops))
        })
        .ok_or("Expected original granted Fuel Scoops")?;
    let review = world.review_module(
        ModuleAction::Store { item: scoops.id },
        world.snapshot().tick,
    )?;
    world
        .prepare_module(review, world.snapshot().tick)?
        .commit();
    // Check real paid offers/reviews, but perform no paid purchase/refit, fire or activation.
    for action in [
        ModuleAction::PurchaseLaser {
            kind: ModuleKind::BeamLaser,
            facing: LaserFacing::Front,
        },
        ModuleAction::PurchaseLaser {
            kind: ModuleKind::PulseLaser,
            facing: LaserFacing::Rear,
        },
        ModuleAction::PurchaseLaser {
            kind: ModuleKind::PulseLaser,
            facing: LaserFacing::Left,
        },
        ModuleAction::PurchaseLaser {
            kind: ModuleKind::PulseLaser,
            facing: LaserFacing::Right,
        },
        ModuleAction::Purchase {
            kind: ModuleKind::EcmSystem,
        },
        ModuleAction::PurchaseMiningLaser,
    ] {
        world.review_module(action, world.snapshot().tick)?;
    }
    world.review_practice(world.snapshot().tick)?;
    Ok(())
}
fn run(input: &Path, output: &Path) -> Result<()> {
    let mut world = load(input)?;
    let opening = world.credits_tenths();
    prepare(&mut world)?;
    let bytes = world.save_bytes()?;
    require(
        bytes.len() <= MAX_SAVE_BYTES,
        "Output exceeds current save bound",
    )?;
    let continued = World::from_save_bytes(&bytes)?;
    require(
        continued.snapshot() == world.snapshot(),
        "Cold simulation differs",
    )?;
    require(
        continued.modular_equipment_snapshot() == world.modular_equipment_snapshot()
            && continued.xyzzy_snapshot() == world.xyzzy_snapshot()
            && continued.ecm_snapshot() == world.ecm_snapshot(),
        "Cold equipment/testing/ECM state differs",
    )?;
    continued.review_practice(continued.snapshot().tick)?;
    // No discovery/default profile path, overwrite, old content bypass, receipt or ammo edit.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    println!(
        "setup=EXPLICIT_PUBLIC_XYZZY_NOT_NATIVE_EARNING\nqualification_base_head={BASE_HEAD}\nqualification_base_tree={BASE_TREE}\ninput={}\nstaging_output={}\nbytes={}\nopening_credits_tenths={opening:?}\nxyzzy={:?}\nflight={:?}\nmodules={:?}\necm={:?}\npractice={:?}\npractice_review=ADMITTED_BUT_NOT_ACCEPTED\npaid_mounts_ecm_mining=NOT_PURCHASED_BY_HELPER\ngranted_scoops=PUBLICLY_STORED_TO_FREE_ONE_UTILITY_MOUNT\n",
        input.display(),
        output.display(),
        bytes.len(),
        continued.xyzzy_snapshot(),
        continued.snapshot(),
        continued.modular_equipment_snapshot(),
        continued.ecm_snapshot(),
        continued.practice_snapshot()
    );
    Ok(())
}
fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let usage = "Usage: prepare_native_laser_ecm ISOLATED_CURRENT_SAVE NEW_STAGING_FILE";
    let input = args.next().ok_or(usage)?;
    let output = args.next().ok_or(usage)?;
    require(args.next().is_none(), usage)?;
    run(Path::new(&input), Path::new(&output))
}
