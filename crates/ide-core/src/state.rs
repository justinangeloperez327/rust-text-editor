//! Headless application lifecycle state.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Lifecycle {
    #[default]
    Starting,
    Running,
    ShuttingDown,
}

#[derive(Debug, Default)]
pub struct ApplicationState {
    lifecycle: Lifecycle,
}

impl ApplicationState {
    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle
    }

    pub fn started(&mut self) -> Result<(), &'static str> {
        if self.lifecycle != Lifecycle::Starting {
            return Err("application can only start once");
        }
        self.lifecycle = Lifecycle::Running;
        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.lifecycle = Lifecycle::ShuttingDown;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_transitions() {
        let mut state = ApplicationState::default();
        assert_eq!(state.lifecycle(), Lifecycle::Starting);
        state.started().unwrap();
        assert_eq!(state.lifecycle(), Lifecycle::Running);
        assert!(state.started().is_err());
        state.shutdown();
        assert_eq!(state.lifecycle(), Lifecycle::ShuttingDown);
    }
}
