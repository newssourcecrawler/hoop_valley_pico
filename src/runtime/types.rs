use std::collections::VecDeque;

pub type TokenId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepStatus {
    #[default]
    Ok,
    Halted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepPhase {
    Prefill,
    #[default]
    Evaluate,
    Finalize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameState {
    pub frame_id: u64,
    pub step_index: u64,
    pub phase: StepPhase,
    pub active: bool,
    pub state_slots: Vec<i32>,
    pub emitted_events: u64,
}

impl FrameState {
    pub fn new(frame_id: u64, slot_count: usize) -> Self {
        Self {
            frame_id,
            step_index: 0,
            phase: StepPhase::Prefill,
            active: true,
            state_slots: vec![0; slot_count],
            emitted_events: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryView {
    pub max_items: usize,
    pub items: VecDeque<TokenId>,
}

impl MemoryView {
    pub fn new(max_items: usize) -> Self {
        Self {
            max_items,
            items: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryDelta {
    None,
    Append(TokenId),
    EvictAndAppend { evicted: TokenId, appended: TokenId },
    ReplaceAll(VecDeque<TokenId>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepBudgets {
    pub max_state_slots: usize,
    pub max_memory_items: usize,
    pub max_steps: u64,
}

impl Default for StepBudgets {
    fn default() -> Self {
        Self {
            max_state_slots: 16,
            max_memory_items: 8,
            max_steps: 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StepControl {
    pub policy_flags: u32,
    pub debug_flags: u32,
    pub force_halt: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepInput {
    pub frame_id: u64,
    pub tick: u64,
    pub input_token: Option<TokenId>,
    pub frame_state: FrameState,
    pub memory_view: MemoryView,
    pub budgets: StepBudgets,
    pub control: StepControl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeSurface {
    pub pressure_milli: u32,
    pub memory_load_milli: u32,
    pub drift_milli: u32,
}

impl Default for ProbeSurface {
    fn default() -> Self {
        Self {
            pressure_milli: 0,
            memory_load_milli: 0,
            drift_milli: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepResult {
    pub frame_id: u64,
    pub tick: u64,
    pub phase: StepPhase,
    pub next_state: FrameState,
    pub probes: ProbeSurface,
    pub memory_delta: MemoryDelta,
    pub status: StepStatus,
}
