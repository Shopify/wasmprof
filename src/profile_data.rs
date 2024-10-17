use crate::collapsed_stack::CollapsedStacks;
use crate::speedscope::SpeedscopeFile;
use crate::WeightUnit;
use wasmtime::FrameInfo;
use std::path::PathBuf;

#[derive(Clone)]
pub enum AddressType {
    ModuleOffset(usize),
    FuncOffset(usize),
}

#[derive(Clone)]
pub struct FrameData {
    pub name: String,
    pub module: Option<String>,
    pub func_index: u32,
    pub func_offset: Option<usize>,
    pub module_offset: Option<usize>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub address: Option<AddressType>,
}

/// Contains the data collected by the profiler.
/// It can be converted into collapsed stacks format or speedscope format.
pub struct ProfileData {
    frames: Vec<FrameData>,
    samples: Vec<Vec<usize>>,
    weights: Option<Vec<u128>>,
    weight_unit: WeightUnit,
    binary_path: Option<PathBuf>,
}

impl ProfileData {
    pub fn new(
        frames: Vec<FrameData>,
        samples: Vec<Vec<usize>>,
        weights: Option<Vec<u128>>,
        weight_unit: WeightUnit,
        binary_path: Option<PathBuf>,
    ) -> Self {
        Self {
            frames,
            samples,
            weights,
            weight_unit,
            binary_path,
        }
    }

    pub fn into_collapsed_stacks(self) -> CollapsedStacks {
        CollapsedStacks::new(self.frames.into_iter().map(|f| f.name).collect(), self.samples, self.weights)
    }

    pub fn to_speedscope(&self, name: Option<String>) -> SpeedscopeFile {
        SpeedscopeFile::new(self, name)
    }

    pub fn frames(&self) -> &[FrameData] {
        &self.frames
    }

    pub fn samples(&self) -> &[Vec<usize>] {
        &self.samples
    }

    pub fn weights(&self) -> &[u128] {
        self.weights.as_ref().map(|w| w.as_slice()).unwrap_or(&[])
    }

    pub fn weight_unit(&self) -> &WeightUnit {
        &self.weight_unit
    }

    pub fn binary_path(&self) -> Option<&PathBuf> {
        self.binary_path.as_ref()
    }

    pub fn frames_mut(&mut self) -> &mut [FrameData] {
        &mut self.frames
    }
}

impl From<(&FrameInfo, Option<&PathBuf>)> for FrameData {
    fn from((frame_info, _binary_path): (&FrameInfo, Option<&PathBuf>)) -> Self {
        let symbols = frame_info.symbols();
        let (file, line, column) = if let Some(symbol) = symbols.first() {
            (symbol.file().map(String::from), symbol.line(), symbol.column())
        } else {
            (None, None, None)
        };

        let address = frame_info.module_offset().map(AddressType::ModuleOffset)
            .or_else(|| frame_info.func_offset().map(AddressType::FuncOffset));

        FrameData {
            name: frame_info.func_name().unwrap_or("<unknown>").to_string(),
            module: frame_info.module().name().map(|s| s.to_string()),
            func_index: frame_info.func_index(),
            func_offset: frame_info.func_offset(),
            module_offset: frame_info.module_offset(),
            file,
            line,
            column,
            address,
        }
    }
}
