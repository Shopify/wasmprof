use crate::{collapsed_stack::CollapsedStacks, speedscope::SpeedscopeFile};

pub struct ProfileData {
    frames: Vec<String>,
    samples: Vec<Vec<usize>>,
    weights: Option<Vec<u128>>,
}

impl ProfileData {
    pub fn new(frames: Vec<String>, samples: Vec<Vec<usize>>, weights: Option<Vec<u128>>) -> Self {
        Self {
            frames,
            samples,
            weights,
        }
    }

    pub fn into_speedscope(self) -> SpeedscopeFile {
        SpeedscopeFile::new(self.frames, self.samples, self.weights)
    }

    pub fn into_collapsed_stacks(self) -> CollapsedStacks {
        CollapsedStacks::new(self.frames, self.samples, self.weights)
    }
}
