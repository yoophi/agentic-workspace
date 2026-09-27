//! 창 제목 요청의 검증(040 US3: AW `domain/mcp_title_control.rs`에서 이동). 문구는 오늘과 같다.

pub const MAX_WINDOW_TITLE_CHARS: usize = 80;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedWindowTitle(String);

impl ValidatedWindowTitle {
    pub fn parse(raw: &str) -> Result<Self, TitleValidationError> {
        let title = raw.trim();
        if title.is_empty() {
            return Err(TitleValidationError::Blank);
        }
        if title.chars().any(char::is_control) {
            return Err(TitleValidationError::ControlCharacter);
        }
        if title.chars().count() > MAX_WINDOW_TITLE_CHARS {
            return Err(TitleValidationError::TooLong {
                max: MAX_WINDOW_TITLE_CHARS,
            });
        }
        Ok(Self(title.to_string()))
    }

    pub fn into_string(self) -> String {
        self.0
    }

    #[cfg(test)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TitleValidationError {
    Blank,
    ControlCharacter,
    TooLong { max: usize },
}

impl TitleValidationError {
    pub fn reason(&self) -> String {
        match self {
            Self::Blank => "Window title must be non-empty.".to_string(),
            Self::ControlCharacter => {
                "Window title must not contain control characters.".to_string()
            }
            Self::TooLong { max } => {
                format!("Window title must be at most {max} characters.")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TitleValidationError, ValidatedWindowTitle, MAX_WINDOW_TITLE_CHARS};

    #[test]
    fn title_validation_trims_readable_title() {
        let title = ValidatedWindowTitle::parse("  Fix login retry state  ").unwrap();
        assert_eq!(title.as_str(), "Fix login retry state");
    }

    #[test]
    fn title_validation_rejects_blank_title() {
        assert_eq!(
            ValidatedWindowTitle::parse(" \t\n ").unwrap_err(),
            TitleValidationError::Blank
        );
    }

    #[test]
    fn title_validation_rejects_control_characters() {
        assert_eq!(
            ValidatedWindowTitle::parse("bad\u{0007}title").unwrap_err(),
            TitleValidationError::ControlCharacter
        );
    }

    #[test]
    fn title_validation_rejects_titles_over_maximum() {
        let title = "a".repeat(MAX_WINDOW_TITLE_CHARS + 1);
        assert_eq!(
            ValidatedWindowTitle::parse(&title).unwrap_err(),
            TitleValidationError::TooLong {
                max: MAX_WINDOW_TITLE_CHARS
            }
        );
    }
}
