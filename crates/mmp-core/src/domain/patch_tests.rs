use super::*;

#[derive(Debug, Deserialize, PartialEq)]
struct Body {
    #[serde(default)]
    name: Patch<String>,
}

#[test]
fn an_absent_field_is_unchanged() {
    let body: Body = serde_json::from_str("{}").unwrap();
    assert_eq!(body.name, Patch::Unchanged);
}

#[test]
fn an_explicit_null_clears() {
    let body: Body = serde_json::from_str(r#"{"name": null}"#).unwrap();
    assert_eq!(body.name, Patch::Clear);
}

#[test]
fn a_value_sets() {
    let body: Body = serde_json::from_str(r#"{"name": "milk"}"#).unwrap();
    assert_eq!(body.name, Patch::Set("milk".to_owned()));
}

#[test]
fn apply_respects_each_state() {
    let current = || Some(1);
    assert_eq!(Patch::Unchanged.apply(current()), Some(1));
    assert_eq!(Patch::Set(2).apply(current()), Some(2));
    assert_eq!(Patch::<i32>::Clear.apply(current()), None);
}
