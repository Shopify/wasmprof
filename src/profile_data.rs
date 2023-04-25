use crate::collapsed_stack::CollapsedStacks;

/// Contains the data collected by the profiler.
/// It can be converted into collapsed stacks format.
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

    pub fn into_collapsed_stacks(self) -> CollapsedStacks {
        CollapsedStacks::new(self.frames, self.samples, self.weights)
    }
}
