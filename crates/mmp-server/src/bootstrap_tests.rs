use super::*;

#[test]
fn the_connection_string_password_is_redacted() {
    assert_eq!(
        redact("postgres://mmp:secret@localhost:55432/mmp"),
        "postgres://***@localhost:55432/mmp"
    );
    assert_eq!(
        redact("postgres://localhost/mmp"),
        "postgres://localhost/mmp"
    );
}

#[test]
fn the_bundled_seed_data_parses() {
    let seeds = seed_ingredients().expect("seed data should parse");
    assert!(seeds.len() > 100, "expected a useful catalogue");
}

#[test]
fn seed_keys_are_unique() {
    let seeds = seed_ingredients().unwrap();
    let mut keys: Vec<&str> = seeds.iter().map(|s| s.seed_key.as_str()).collect();
    keys.sort_unstable();
    let before = keys.len();
    keys.dedup();
    assert_eq!(before, keys.len(), "seed keys must be unique");
}

#[test]
fn seed_names_are_unique_case_insensitively() {
    let seeds = seed_ingredients().unwrap();
    let mut names: Vec<String> = seeds.iter().map(|s| s.name.to_lowercase()).collect();
    names.sort();
    let before = names.len();
    names.dedup();
    assert_eq!(
        before,
        names.len(),
        "the unique name index would reject duplicates"
    );
}

#[test]
fn seed_entries_carry_no_nutrition() {
    let raw: serde_json::Value = serde_json::from_str(SEED_INGREDIENTS).unwrap();
    for entry in raw.as_array().unwrap() {
        assert!(
            entry.get("nutrition").is_none(),
            "nutrition belongs on products, not ingredients: {entry}"
        );
    }
}
