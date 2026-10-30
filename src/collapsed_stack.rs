use std::fmt;

pub struct CollapsedStack {
    stack: Vec<String>,
    weight: u128,
}
/// Contains the data collected by the profiler in collapsed stacks format.
/// The main use case is to call `write_to_file` on a `CollapsedStacks` instance.
/// The resulting file can be opened in a variety of profiling tools.
/// Frame names percent-encode `%`, `;`, carriage returns, and newlines so they
/// cannot be confused with frame or sample delimiters.
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
            for character in frame.chars() {
                match character {
                    '%' => write!(f, "%25")?,
                    ';' => write!(f, "%3B")?,
                    '\r' => write!(f, "%0D")?,
                    '\n' => write!(f, "%0A")?,
                    other => write!(f, "{other}")?,
                }
            }
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

#[cfg(test)]
mod tests {
    use super::CollapsedStacks;

    #[test]
    fn escapes_frame_delimiters_and_literal_percent_signs() {
        let stacks = CollapsedStacks::new(
            vec!["leaf;frame\nnext\r%3B".into(), "root".into()],
            vec![vec![0, 1]],
            Some(vec![7]),
        );
        assert_eq!(stacks.to_string(), "root;leaf%3Bframe%0Anext%0D%253B 7\n");
    }

    #[test]
    fn preserves_ordinary_frame_names() {
        let stacks = CollapsedStacks::new(
            vec!["space::foo(int, β)".into(), "root".into()],
            vec![vec![0, 1]],
            Some(vec![7]),
        );
        assert_eq!(stacks.to_string(), "root;space::foo(int, β) 7\n");
    }
}
