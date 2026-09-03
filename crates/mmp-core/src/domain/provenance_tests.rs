use super::*;

#[test]
fn a_clean_seeded_record_accepts_refresh() {
    assert!(Provenance::seeded("whole-milk").accepts_seed_refresh());
}

#[test]
fn a_locally_edited_seeded_record_refuses_refresh() {
    let mut provenance = Provenance::seeded("whole-milk");
    provenance.locally_modified = true;
    assert!(!provenance.accepts_seed_refresh());
}

#[test]
fn a_local_record_is_never_touched_by_refresh() {
    assert!(!Provenance::local().accepts_seed_refresh());
}

#[test]
fn serde_representation_matches_the_origin_code() {
    for origin in CatalogueOrigin::ALL {
        let json = serde_json::to_string(&origin).unwrap();
        assert_eq!(json, format!("\"{}\"", origin.code()), "{origin:?}");
    }
}

#[test]
fn origin_codes_round_trip() {
    for origin in CatalogueOrigin::ALL {
        assert_eq!(CatalogueOrigin::from_str(origin.code()).unwrap(), origin);
    }
}
