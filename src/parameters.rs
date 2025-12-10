use std::collections::HashMap;

/// Parameter storage for the animator
#[derive(Debug, Default)]
pub struct Parameters {
    bools: HashMap<String, bool>,
    floats: HashMap<String, f32>,
    ints: HashMap<String, i32>,
    triggers: HashMap<String, bool>,
}

impl Parameters {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.bools.get(name).copied()
    }

    #[inline]
    pub fn get_float(&self, name: &str) -> Option<f32> {
        self.floats.get(name).copied()
    }

    #[inline]
    pub fn get_int(&self, name: &str) -> Option<i32> {
        self.ints.get(name).copied()
    }

    #[inline]
    pub fn get_trigger(&self, name: &str) -> bool {
        self.triggers.get(name).copied().unwrap_or(false)
    }

    #[inline]
    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.bools.insert(name.to_string(), value);
    }

    #[inline]
    pub fn set_float(&mut self, name: &str, value: f32) {
        self.floats.insert(name.to_string(), value);
    }

    #[inline]
    pub fn set_int(&mut self, name: &str, value: i32) {
        self.ints.insert(name.to_string(), value);
    }

    #[inline]
    pub fn set_trigger(&mut self, name: &str) {
        self.triggers.insert(name.to_string(), true);
    }

    #[inline]
    pub fn reset_triggers(&mut self) {
        self.triggers.clear();
    }
}
