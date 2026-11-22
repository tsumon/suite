use suite_contracts::ThemeKind;
use suite_platform::{PersonalizeDwords, PersonalizeRegistry, send_immersive_color_set};

pub trait PersonalizeStore: Send + Sync {
    fn try_read(&self) -> Result<(ThemeKind, ThemeKind), String>;
    fn try_write(&self, apps: ThemeKind, system: ThemeKind) -> Result<(), String>;
}

pub trait ThemeBroadcaster: Send + Sync {
    fn try_broadcast(&self) -> Result<(), String>;
}

pub struct RegistryPersonalizeStore;

impl PersonalizeStore for RegistryPersonalizeStore {
    fn try_read(&self) -> Result<(ThemeKind, ThemeKind), String> {
        let dwords = PersonalizeRegistry::try_read()?;
        let apps = if dwords.apps_use_light {
            ThemeKind::Light
        } else {
            ThemeKind::Dark
        };
        let sys = if dwords.system_uses_light {
            ThemeKind::Light
        } else {
            ThemeKind::Dark
        };
        Ok((apps, sys))
    }

    fn try_write(&self, apps: ThemeKind, system: ThemeKind) -> Result<(), String> {
        PersonalizeRegistry::try_write(PersonalizeDwords {
            apps_use_light: apps == ThemeKind::Light,
            system_uses_light: system == ThemeKind::Light,
        })
    }
}

pub struct NativeThemeBroadcaster;

impl ThemeBroadcaster for NativeThemeBroadcaster {
    fn try_broadcast(&self) -> Result<(), String> {
        send_immersive_color_set()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeToggleResult {
    pub succeeded: bool,
    pub broadcast_succeeded: bool,
    pub new_theme: ThemeKind,
    pub error: Option<String>,
}

pub struct ThemeToggleService<S: PersonalizeStore, B: ThemeBroadcaster> {
    store: S,
    broadcaster: B,
}

impl ThemeToggleService<RegistryPersonalizeStore, NativeThemeBroadcaster> {
    pub fn new_native() -> Self {
        Self::new(RegistryPersonalizeStore, NativeThemeBroadcaster)
    }
}

impl<S: PersonalizeStore, B: ThemeBroadcaster> ThemeToggleService<S, B> {
    pub fn new(store: S, broadcaster: B) -> Self {
        Self { store, broadcaster }
    }

    pub fn try_read(&self) -> Result<ThemeKind, String> {
        let (_, system) = self.store.try_read()?;
        Ok(system)
    }

    /// Flip AppsUseLightTheme + SystemUsesLightTheme together.
    /// Fires `on_changed` BEFORE `broadcaster.try_broadcast()` so local UI never waits on HWND_BROADCAST.
    pub fn toggle_with_hook<F>(&self, on_changed: F) -> ThemeToggleResult
    where
        F: FnOnce(ThemeKind),
    {
        let system = match self.store.try_read() {
            Ok((_, sys)) => sys,
            Err(e) => {
                return ThemeToggleResult {
                    succeeded: false,
                    broadcast_succeeded: false,
                    new_theme: ThemeKind::Light,
                    error: Some(e),
                };
            }
        };

        let next = if system == ThemeKind::Light {
            ThemeKind::Dark
        } else {
            ThemeKind::Light
        };

        if let Err(e) = self.store.try_write(next, next) {
            return ThemeToggleResult {
                succeeded: false,
                broadcast_succeeded: false,
                new_theme: system,
                error: Some(e),
            };
        }

        on_changed(next);

        match self.broadcaster.try_broadcast() {
            Ok(()) => ThemeToggleResult {
                succeeded: true,
                broadcast_succeeded: true,
                new_theme: next,
                error: None,
            },
            Err(e) => ThemeToggleResult {
                succeeded: true,
                broadcast_succeeded: false,
                new_theme: next,
                error: Some(e),
            },
        }
    }

    pub fn toggle(&self) -> ThemeToggleResult {
        self.toggle_with_hook(|_| {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct MockStore {
        apps: Mutex<ThemeKind>,
        sys: Mutex<ThemeKind>,
        events: Arc<Mutex<Vec<String>>>,
        fail_write: bool,
    }

    impl PersonalizeStore for MockStore {
        fn try_read(&self) -> Result<(ThemeKind, ThemeKind), String> {
            Ok((
                *self.apps.lock().unwrap(),
                *self.sys.lock().unwrap(),
            ))
        }

        fn try_write(&self, apps: ThemeKind, system: ThemeKind) -> Result<(), String> {
            if self.fail_write {
                return Err("policy locked".into());
            }
            *self.apps.lock().unwrap() = apps;
            *self.sys.lock().unwrap() = system;
            self.events.lock().unwrap().push("write".into());
            Ok(())
        }
    }

    struct MockBroadcaster {
        events: Arc<Mutex<Vec<String>>>,
        fail: bool,
    }

    impl ThemeBroadcaster for MockBroadcaster {
        fn try_broadcast(&self) -> Result<(), String> {
            self.events.lock().unwrap().push("broadcast".into());
            if self.fail {
                Err("broadcast err".into())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn local_hook_fires_before_broadcast() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let store = MockStore {
            apps: Mutex::new(ThemeKind::Light),
            sys: Mutex::new(ThemeKind::Light),
            events: Arc::clone(&events),
            fail_write: false,
        };
        let bc = MockBroadcaster {
            events: Arc::clone(&events),
            fail: false,
        };
        let svc = ThemeToggleService::new(store, bc);
        let res = svc.toggle_with_hook(|theme| {
            assert_eq!(theme, ThemeKind::Dark);
            events.lock().unwrap().push("local_hook".into());
        });
        assert!(res.succeeded);
        assert!(res.broadcast_succeeded);
        assert_eq!(res.new_theme, ThemeKind::Dark);
        assert_eq!(*events.lock().unwrap(), vec!["write", "local_hook", "broadcast"]);
    }

    #[test]
    fn write_failure_skips_broadcast() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let store = MockStore {
            apps: Mutex::new(ThemeKind::Dark),
            sys: Mutex::new(ThemeKind::Dark),
            events: Arc::clone(&events),
            fail_write: true,
        };
        let bc = MockBroadcaster {
            events: Arc::clone(&events),
            fail: false,
        };
        let svc = ThemeToggleService::new(store, bc);
        let res = svc.toggle();
        assert!(!res.succeeded);
        assert!(!res.broadcast_succeeded);
        assert_eq!(res.new_theme, ThemeKind::Dark);
        assert!(events.lock().unwrap().is_empty());
    }
}
