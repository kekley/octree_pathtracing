#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitterSamplingStrategy {
    None {
        name: &'static str,
        description: &'static str,
    },
    One {
        name: &'static str,
        description: &'static str,
    },
    OneBlock {
        name: &'static str,
        description: &'static str,
    },
    All {
        name: &'static str,
        description: &'static str,
    },
}

impl Default for EmitterSamplingStrategy {
    fn default() -> Self {
        Self::NONE
    }
}

impl EmitterSamplingStrategy {
    pub fn get_description(&self) -> &str {
        match self {
            EmitterSamplingStrategy::None { description, .. } => description,
            EmitterSamplingStrategy::One { description, .. } => description,
            EmitterSamplingStrategy::OneBlock { description, .. } => description,
            EmitterSamplingStrategy::All { description, .. } => description,
        }
    }
    pub const NONE: EmitterSamplingStrategy = EmitterSamplingStrategy::None {
        name: "None",
        description: "No emitter sampling.",
    };

    pub const ONE: EmitterSamplingStrategy = EmitterSamplingStrategy::One {
        name: "One",
        description: "Sample a single face.",
    };

    pub const ONE_BLOCK: EmitterSamplingStrategy = EmitterSamplingStrategy::OneBlock {
        name: "One Block",
        description: "Sample all the faces on a single emitter block.",
    };

    pub const ALL: EmitterSamplingStrategy = EmitterSamplingStrategy::All {
        name: "All",
        description: "Sample all faces on all emitter blocks.",
    };
}

pub struct EmittersConfig {
    pub sampling_strategy: EmitterSamplingStrategy,
    pub emitters_enabled: bool,
    pub emmitter_intensity: f32,
    pub f_sub_surface: f32,
}
