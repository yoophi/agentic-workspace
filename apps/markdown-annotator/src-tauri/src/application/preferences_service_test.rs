use crate::domain::global_preferences::GlobalPreferences;
#[test]
fn validates_exact_names_and_font_range() {
    let mut value = GlobalPreferences::default();
    value.excluded_directory_names = vec!["a/b".into()];
    assert!(value.validate().is_err());
    value.excluded_directory_names = vec!["dist".into()];
    value.font_size = 20;
    assert!(value.validate().is_ok());
}
