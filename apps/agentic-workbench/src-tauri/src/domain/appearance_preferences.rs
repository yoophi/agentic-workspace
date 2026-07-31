use serde::{Deserialize, Deserializer, Serialize};

pub const MIN_FONT_SIZE_STEP: i8 = -2;
pub const MAX_FONT_SIZE_STEP: i8 = 2;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct FontSizeStep(i8);

impl FontSizeStep {
    pub fn new(value: i8) -> Self {
        if (MIN_FONT_SIZE_STEP..=MAX_FONT_SIZE_STEP).contains(&value) {
            Self(value)
        } else {
            Self::default()
        }
    }

    pub fn value(self) -> i8 {
        self.0
    }

    pub fn adjust(self, delta: i8) -> Result<Self, String> {
        if !matches!(delta, -1 | 1) {
            return Err("Font size adjustment must be -1 or 1.".into());
        }
        Ok(Self(
            (self.0 + delta).clamp(MIN_FONT_SIZE_STEP, MAX_FONT_SIZE_STEP),
        ))
    }
}

impl<'de> Deserialize<'de> for FontSizeStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        i8::deserialize(deserializer).map(Self::new)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearancePreferences {
    #[serde(default)]
    pub font_size_step: FontSizeStep,
}

impl AppearancePreferences {
    pub fn with_font_size_step(value: i8) -> Self {
        Self {
            font_size_step: FontSizeStep::new(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_exactly_five_steps_and_normalizes_invalid_values() {
        for value in MIN_FONT_SIZE_STEP..=MAX_FONT_SIZE_STEP {
            assert_eq!(FontSizeStep::new(value).value(), value);
        }
        assert_eq!(FontSizeStep::new(-3), FontSizeStep::default());
        assert_eq!(FontSizeStep::new(3), FontSizeStep::default());
    }

    #[test]
    fn clamps_adjustments_and_rejects_invalid_delta() {
        assert_eq!(FontSizeStep::new(2).adjust(1).unwrap().value(), 2);
        assert_eq!(FontSizeStep::new(-2).adjust(-1).unwrap().value(), -2);
        assert_eq!(FontSizeStep::new(0).adjust(1).unwrap().value(), 1);
        assert!(FontSizeStep::default().adjust(0).is_err());
    }

    #[test]
    fn serde_uses_camel_case_and_defaults_invalid_or_missing_steps() {
        let invalid: AppearancePreferences =
            serde_json::from_str(r#"{"fontSizeStep":12}"#).unwrap();
        let missing: AppearancePreferences = serde_json::from_str("{}").unwrap();
        assert_eq!(invalid, AppearancePreferences::default());
        assert_eq!(missing, AppearancePreferences::default());
        assert_eq!(
            serde_json::to_string(&AppearancePreferences::with_font_size_step(1)).unwrap(),
            r#"{"fontSizeStep":1}"#
        );
    }
}
