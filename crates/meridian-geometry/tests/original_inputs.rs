use meridian_geometry::{Budget, Failure, FailureKind, Limits, OriginalAsset};

#[test]
fn every_original_input_preserves_components_facets_and_exact_corner_fields() -> Result<(), Failure>
{
    for (asset, components, triangles) in [
        (OriginalAsset::Sh010, 31, 1012),
        (OriginalAsset::Sh011, 31, 1012),
        (OriginalAsset::Sh016, 33, 1100),
        (OriginalAsset::Sh017, 33, 1100),
        (OriginalAsset::Sh166, 29, 988),
    ] {
        let mut budget = Budget::new(Limits::default())?;
        let decoded = asset.decode(asset.pinned_bytes(), &mut budget)?;
        assert_eq!(decoded.len(), triangles);
        let mut expected_component = 0;
        let mut expected_facet = 0;
        for facet in decoded {
            if facet.id().component != expected_component {
                expected_component += 1;
                expected_facet = 0;
            }
            assert_eq!(facet.id().component, expected_component);
            assert_eq!(facet.id().facet, expected_facet);
            expected_facet += 1;
            for vertex in facet.vertices() {
                assert_eq!(facet.uv_at(&vertex.point, &mut budget)?, vertex.uv);
            }
        }
        assert_eq!(expected_component + 1, components);
    }
    Ok(())
}

#[test]
fn changed_original_bytes_refuse_before_source_reconstruction() -> Result<(), Failure> {
    let asset = OriginalAsset::Sh010;
    let mut changed = asset.pinned_bytes().to_vec();
    let last = changed.len() - 2;
    changed[last] ^= 1;
    let mut b = Budget::new(Limits::default())?;
    assert_eq!(
        asset.decode(&changed, &mut b).err().map(|e| e.kind),
        Some(FailureKind::InvalidInput)
    );
    Ok(())
}
