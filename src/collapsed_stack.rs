use std::fmt;

pub struct CollapsedStack {
    stack: Vec<String>,
    weight: u128,
}
/// Contains the data collected by the profiler in collapsed stacks format.
/// The main use case is to call `write_to_file` on a `CollapsedStacks` instance.
/// The resulting file can be opened in a variety of profiling tools.
/// The format is explained here: https://github.com/jlfwong/speedscope/wiki/Importing-from-custom-sources#brendan-greggs-collapsed-stack-format
pub struct CollapsedStacks(Vec<CollapsedStack>);

impl CollapsedStacks {
    pub fn new(
        frames: Vec<String>,
        samples: Vec<Vec<usize>>,
        weights: Option<Vec<u128>>,
    ) -> CollapsedStacks {
        let mut stacks = Vec::new();
        for (i, sample) in samples.iter().enumerate() {
            let mut stack = Vec::new();
            for frame in sample {
                stack.push(frames[*frame].clone());
            }
            let weight = weights.as_ref().map(|w| w[i]).unwrap_or(1);
            stacks.push(CollapsedStack { stack, weight });
        }
        Self(stacks)
    }

    pub fn write_to_file(&self, path: &str) -> std::io::Result<()> {
        std::fs::write(path, self.to_string())
    }
}

impl fmt::Display for CollapsedStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for frame in self.stack.iter().rev() {
            if first {
                first = false;
            } else {
                write!(f, ";")?;
            }
            write!(f, "{}", frame)?;
        }
        write!(f, " {}", self.weight)
    }
}

impl fmt::Display for CollapsedStacks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stack in self.0.iter() {
            writeln!(f, "{}", stack)?;
        }
        Ok(())
    }
}
