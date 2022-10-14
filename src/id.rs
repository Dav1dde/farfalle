use rand::distributions::{Alphanumeric, DistString};

pub trait IdGen {
    fn next_id(&self, attempts: u32) -> String;
}

pub struct RandomIdGen {
    min_len: u8,
}

impl RandomIdGen {
    pub fn new(min_len: u8) -> Self {
        Self { min_len }
    }
}

impl IdGen for RandomIdGen {
    fn next_id(&self, attempts: u32) -> String {
        Alphanumeric.sample_string(
            &mut rand::thread_rng(),
            self.min_len as usize + attempts as usize,
        )
    }
}
