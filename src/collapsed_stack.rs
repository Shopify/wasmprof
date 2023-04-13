use std::{fmt};

pub struct CollapsedStack {
    stack: Vec<String>,
    weight: u128,
}

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
