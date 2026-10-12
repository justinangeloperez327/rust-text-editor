//! UI-independent application primitives.

pub mod layout;
pub mod state;

pub const APPLICATION_NAME: &str = "Rust Text Editor";

#[cfg(test)]
mod tests {
    use super::APPLICATION_NAME;

    #[test]
    fn application_name_is_defined() {
        assert!(!APPLICATION_NAME.is_empty());
    }
}
