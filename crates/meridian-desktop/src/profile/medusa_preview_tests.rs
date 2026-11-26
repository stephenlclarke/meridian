use super::*;
use meridian_core::{FlightInput, MedusaPreviewFit, PlayerHullKind};

#[test]
fn medusa_profile_fit_and_cold_continue_do_not_touch_ordinary_slot()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tests::TestDirectory::new();
    let (mut ordinary, _) = Profile::open_startup(dir.0.clone(), 64201)?;
    ordinary.save()?;
    let ordinary_path = dir.0.join("commander.meridian");
    let ordinary_bytes = std::fs::read(&ordinary_path)?;
    let mut preview = Profile::open_medusa_preview(dir.0.clone())?;
    let fit = MedusaPreviewFit {
        cargo_expansion: true,
        cabins: 4,
    };
    preview.medusa_preview_fit(fit)?;
    let saved = std::fs::read(dir.0.join("medusa-preview.meridian"))?;
    assert!(
        preview
            .medusa_preview_fit(MedusaPreviewFit { cabins: 5, ..fit })
            .is_err()
    );
    assert_eq!(std::fs::read(dir.0.join("medusa-preview.meridian"))?, saved);
    assert_eq!(preview.world.medusa_preview_fit(), Some(fit));
    preview.command(Action::Launch)?;
    for _ in 0..8 {
        preview.world.step(FlightInput {
            throttle: 1.0,
            ..FlightInput::default()
        })?;
    }
    preview.command(Action::SetFlightView(meridian_core::FlightView::Rear))?;
    let flight = preview.world.snapshot();
    preview.save()?;
    drop(preview);
    let mut restored = Profile::open_medusa_preview(dir.0.clone())?;
    assert_eq!(restored.world.player_hull_kind(), PlayerHullKind::Medusa);
    assert_eq!(restored.world.snapshot(), flight);
    assert_eq!(restored.world.medusa_preview_fit(), Some(fit));
    assert_eq!(
        restored.world.flight_view(),
        meridian_core::FlightView::Rear
    );
    assert!(restored.new_commander(64203).is_err());
    assert_eq!(std::fs::read(&ordinary_path)?, ordinary_bytes);
    ordinary.continue_saved()?;
    assert_eq!(ordinary.world.player_hull_kind(), PlayerHullKind::Peregrine);
    Ok(())
}
