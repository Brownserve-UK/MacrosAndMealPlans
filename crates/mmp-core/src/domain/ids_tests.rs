use super::*;

#[test]
fn seeded_identifiers_are_stable() {
    assert_eq!(
        IngredientId::seeded("whole-milk"),
        IngredientId::seeded("whole-milk")
    );
}

#[test]
fn seeded_identifiers_differ_per_key_and_resource() {
    assert_ne!(
        IngredientId::seeded("whole-milk"),
        IngredientId::seeded("skimmed-milk")
    );
    assert_ne!(
        IngredientId::seeded("whole-milk").as_uuid(),
        ProductId::seeded("whole-milk").as_uuid()
    );
}

#[test]
fn generated_identifiers_are_version_seven() {
    assert_eq!(IngredientId::new().as_uuid().get_version_num(), 7);
}

#[test]
fn revision_advances() {
    assert_eq!(Revision::INITIAL.next(), Revision::new(2));
}
