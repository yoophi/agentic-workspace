#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessState {
    Reserved,
    Spawning,
    Adopted,
    Published,
    Active,
    Aborting,
    TerminatingPublished,
    TerminatingActive,
    Terminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessTransition {
    BeginSpawn,
    Adopt,
    Publish,
    ActivateTransient,
    ClaimUnpublishedAbort,
    RequestTermination,
    Finish,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidTransition {
    pub state: ProcessState,
    pub transition: ProcessTransition,
}

pub fn reduce(
    state: ProcessState,
    transition: ProcessTransition,
) -> Result<ProcessState, InvalidTransition> {
    use ProcessState::{
        Aborting, Active, Adopted, Published, Reserved, Spawning, Terminal, TerminatingActive,
        TerminatingPublished,
    };
    use ProcessTransition::{
        ActivateTransient, Adopt, BeginSpawn, ClaimUnpublishedAbort, Finish, Publish,
        RequestTermination,
    };
    match (state, transition) {
        (Reserved, BeginSpawn) => Ok(Spawning),
        (Spawning, Adopt) => Ok(Adopted),
        (Adopted, Publish) => Ok(Published),
        (Adopted, ActivateTransient) => Ok(Active),
        (Reserved | Spawning | Adopted, ClaimUnpublishedAbort) => Ok(Aborting),
        (Published, RequestTermination) => Ok(TerminatingPublished),
        (Active, RequestTermination) => Ok(TerminatingActive),
        (Spawning | Aborting | TerminatingPublished | TerminatingActive, Finish) => Ok(Terminal),
        _ => Err(InvalidTransition { state, transition }),
    }
}

#[cfg(test)]
mod tests {
    use super::{reduce, ProcessState, ProcessTransition};

    #[test]
    fn durable_publication_and_transient_activation_are_distinct() {
        assert_eq!(
            reduce(ProcessState::Adopted, ProcessTransition::Publish),
            Ok(ProcessState::Published)
        );
        assert_eq!(
            reduce(ProcessState::Adopted, ProcessTransition::ActivateTransient),
            Ok(ProcessState::Active)
        );
    }

    #[test]
    fn publication_cannot_win_after_abort_claims_the_attempt() {
        let aborting = reduce(
            ProcessState::Adopted,
            ProcessTransition::ClaimUnpublishedAbort,
        )
        .unwrap();
        assert!(reduce(aborting, ProcessTransition::Publish).is_err());
        assert_eq!(
            reduce(aborting, ProcessTransition::Finish),
            Ok(ProcessState::Terminal)
        );
    }

    #[test]
    fn late_unpublished_resolver_cannot_reverse_publication_or_activation() {
        assert!(reduce(
            ProcessState::Published,
            ProcessTransition::ClaimUnpublishedAbort
        )
        .is_err());
        assert!(reduce(
            ProcessState::Active,
            ProcessTransition::ClaimUnpublishedAbort
        )
        .is_err());
    }

    #[test]
    fn user_termination_preserves_the_winning_publication_kind() {
        assert_eq!(
            reduce(
                ProcessState::Published,
                ProcessTransition::RequestTermination
            ),
            Ok(ProcessState::TerminatingPublished)
        );
        assert_eq!(
            reduce(ProcessState::Active, ProcessTransition::RequestTermination),
            Ok(ProcessState::TerminatingActive)
        );
    }

    #[test]
    fn started_states_require_spawn_and_adoption_order() {
        assert!(reduce(ProcessState::Reserved, ProcessTransition::Publish).is_err());
        let spawning = reduce(ProcessState::Reserved, ProcessTransition::BeginSpawn).unwrap();
        let adopted = reduce(spawning, ProcessTransition::Adopt).unwrap();
        assert_eq!(
            reduce(adopted, ProcessTransition::Publish),
            Ok(ProcessState::Published)
        );
    }
}
