use std::sync::Mutex;

use crate::{
    domain::appearance_preferences::AppearancePreferences,
    ports::appearance_preferences_repository::AppearancePreferencesRepository,
};

pub struct AppearancePreferencesService<R> {
    repository: R,
    current: Mutex<AppearancePreferences>,
}

impl<R: AppearancePreferencesRepository> AppearancePreferencesService<R> {
    pub fn bootstrap(repository: R) -> Result<Self, String> {
        let current = repository.load_preferences()?;
        Ok(Self {
            repository,
            current: Mutex::new(current),
        })
    }

    pub fn get(&self) -> Result<AppearancePreferences, String> {
        self.current
            .lock()
            .map(|value| *value)
            .map_err(|_| "Appearance preferences state is unavailable.".into())
    }

    pub fn set_font_size_step(&self, value: i8) -> Result<AppearancePreferences, String> {
        self.transition(|_| Ok(AppearancePreferences::with_font_size_step(value)))
    }

    pub fn adjust_font_size_step(&self, delta: i8) -> Result<AppearancePreferences, String> {
        self.transition(|current| {
            Ok(AppearancePreferences {
                font_size_step: current.font_size_step.adjust(delta)?,
            })
        })
    }

    fn transition(
        &self,
        next: impl FnOnce(AppearancePreferences) -> Result<AppearancePreferences, String>,
    ) -> Result<AppearancePreferences, String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "Appearance preferences state is unavailable.".to_string())?;
        let next = next(*current)?;
        if next == *current {
            return Ok(next);
        }
        self.repository.save_preferences(&next)?;
        *current = next;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };

    use super::*;

    #[derive(Clone, Default)]
    struct FakeRepository {
        value: Arc<Mutex<AppearancePreferences>>,
        fail_save: Arc<AtomicBool>,
    }

    impl AppearancePreferencesRepository for FakeRepository {
        fn load_preferences(&self) -> Result<AppearancePreferences, String> {
            Ok(*self.value.lock().unwrap())
        }

        fn save_preferences(&self, preferences: &AppearancePreferences) -> Result<(), String> {
            if self.fail_save.load(Ordering::SeqCst) {
                return Err("save failed".into());
            }
            *self.value.lock().unwrap() = *preferences;
            Ok(())
        }
    }

    #[test]
    fn bootstraps_gets_sets_adjusts_and_keeps_boundaries_idempotent() {
        let repository = FakeRepository::default();
        *repository.value.lock().unwrap() = AppearancePreferences::with_font_size_step(1);
        let service = AppearancePreferencesService::bootstrap(repository.clone()).unwrap();

        assert_eq!(service.get().unwrap().font_size_step.value(), 1);
        assert_eq!(
            service
                .adjust_font_size_step(1)
                .unwrap()
                .font_size_step
                .value(),
            2
        );
        assert_eq!(
            service
                .adjust_font_size_step(1)
                .unwrap()
                .font_size_step
                .value(),
            2
        );
        assert_eq!(
            service.set_font_size_step(99).unwrap(),
            AppearancePreferences::default()
        );
        assert!(service.adjust_font_size_step(0).is_err());
    }

    #[test]
    fn persists_before_replacing_state_and_rolls_back_on_failure() {
        let repository = FakeRepository::default();
        let service = AppearancePreferencesService::bootstrap(repository.clone()).unwrap();
        repository.fail_save.store(true, Ordering::SeqCst);

        assert!(service.set_font_size_step(2).is_err());
        assert_eq!(service.get().unwrap(), AppearancePreferences::default());
        assert_eq!(
            *repository.value.lock().unwrap(),
            AppearancePreferences::default()
        );
    }

    #[test]
    fn serializes_concurrent_adjustments() {
        let service =
            Arc::new(AppearancePreferencesService::bootstrap(FakeRepository::default()).unwrap());
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let service = service.clone();
                std::thread::spawn(move || service.adjust_font_size_step(1).unwrap())
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(service.get().unwrap().font_size_step.value(), 2);
    }
}
