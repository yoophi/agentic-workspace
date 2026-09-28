//! Windows feasibility entry points are compiled and exercised on the Windows
//! quality job before production consumers may migrate.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsCapabilityReport {
    pub suspended_create: bool,
    pub assign_before_resume: bool,
    pub kill_on_close: bool,
    pub breakaway_denied: bool,
}

impl WindowsCapabilityReport {
    #[must_use]
    pub fn supports_required_containment(&self) -> bool {
        self.suspended_create
            && self.assign_before_resume
            && self.kill_on_close
            && self.breakaway_denied
    }
}
