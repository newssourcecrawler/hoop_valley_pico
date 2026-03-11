use crate::runtime::types::{
    FrameState, MemoryDelta, MemoryView, StepBudgets, StepControl, StepResult, TokenId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayStep {
    pub tick: u64,
    pub input_token: Option<TokenId>,
    pub memory_before: MemoryView,
    pub result: StepResult,
    pub memory_after: MemoryView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRecord {
    pub frame_id: u64,
    pub initial_frame: FrameState,
    pub initial_memory: MemoryView,
    pub budgets: StepBudgets,
    pub control: StepControl,
    pub steps: Vec<ReplayStep>,
    pub final_frame: FrameState,
    pub final_memory: MemoryView,
}

impl ReplayRecord {
    pub fn new(
        initial_frame: FrameState,
        initial_memory: MemoryView,
        budgets: StepBudgets,
        control: StepControl,
    ) -> Self {
        Self {
            frame_id: initial_frame.frame_id,
            initial_frame: initial_frame.clone(),
            initial_memory: initial_memory.clone(),
            budgets,
            control,
            steps: Vec::new(),
            final_frame: initial_frame,
            final_memory: initial_memory,
        }
    }

    pub fn push_step(
        &mut self,
        tick: u64,
        input_token: Option<TokenId>,
        memory_before: MemoryView,
        result: StepResult,
        memory_after: MemoryView,
    ) {
        self.final_frame = result.next_state.clone();
        self.final_memory = memory_after.clone();
        self.steps.push(ReplayStep {
            tick,
            input_token,
            memory_before,
            result,
            memory_after,
        });
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn last_step(&self) -> Option<&ReplayStep> {
        self.steps.last()
    }
}

pub fn apply_memory_delta_locally(
    memory_view: &mut MemoryView,
    delta: &MemoryDelta,
    budgets: &StepBudgets,
) -> Result<(), &'static str> {
    match delta {
        MemoryDelta::None => {}
        MemoryDelta::Append(token) => {
            if budgets.max_memory_items == 0 {
                return Err("cannot append memory when max_memory_items is 0");
            }
            if memory_view.len() >= budgets.max_memory_items {
                return Err("append would exceed max_memory_items");
            }
            memory_view.items.push_back(*token);
        }
        MemoryDelta::EvictAndAppend { evicted, appended } => match memory_view.items.pop_front() {
            Some(found) if found == *evicted => {
                if budgets.max_memory_items == 0 {
                    return Err("cannot append memory when max_memory_items is 0");
                }
                memory_view.items.push_back(*appended);
            }
            Some(_) => {
                return Err("memory eviction did not match expected oldest token");
            }
            None => {
                return Err("cannot evict from empty memory view");
            }
        },
        MemoryDelta::ReplaceAll(next) => {
            if next.len() > budgets.max_memory_items {
                return Err("replacement memory exceeds max_memory_items");
            }
            memory_view.items = next.clone();
        }
    }

    memory_view.max_items = budgets.max_memory_items;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::types::{ProbeSurface, StepPhase, StepStatus};
    use std::collections::VecDeque;

    fn make_frame() -> FrameState {
        FrameState::new(7, 3)
    }

    fn make_memory() -> MemoryView {
        let mut memory = MemoryView::new(4);
        memory.items = VecDeque::from(vec![10, 11]);
        memory
    }

    fn make_budgets() -> StepBudgets {
        StepBudgets {
            max_state_slots: 8,
            max_memory_items: 4,
            max_steps: 8,
        }
    }

    fn make_result(next_state: FrameState, memory_delta: MemoryDelta) -> StepResult {
        StepResult {
            frame_id: 7,
            tick: 0,
            phase: StepPhase::Evaluate,
            next_state,
            probes: ProbeSurface::default(),
            memory_delta,
            status: StepStatus::Ok,
        }
    }

    #[test]
    fn replay_record_starts_with_initial_state() {
        let frame = make_frame();
        let memory = make_memory();
        let record = ReplayRecord::new(
            frame.clone(),
            memory.clone(),
            make_budgets(),
            StepControl::default(),
        );

        assert_eq!(record.frame_id, 7);
        assert_eq!(record.initial_frame, frame);
        assert_eq!(record.initial_memory, memory);
        assert_eq!(record.final_frame.frame_id, 7);
        assert_eq!(record.final_memory.items, VecDeque::from(vec![10, 11]));
        assert!(record.is_empty());
    }

    #[test]
    fn push_step_updates_final_frame_and_memory() {
        let frame = make_frame();
        let memory = make_memory();
        let budgets = make_budgets();
        let mut record = ReplayRecord::new(frame, memory.clone(), budgets, StepControl::default());

        let mut next_state = make_frame();
        next_state.step_index = 1;
        next_state.emitted_events = 1;
        next_state.phase = StepPhase::Evaluate;

        let result = make_result(next_state.clone(), MemoryDelta::Append(42));
        let mut memory_after = memory.clone();
        apply_memory_delta_locally(&mut memory_after, &result.memory_delta, &budgets).unwrap();

        record.push_step(0, Some(5), memory, result.clone(), memory_after.clone());

        assert_eq!(record.len(), 1);
        assert_eq!(record.final_frame, next_state);
        assert_eq!(record.final_memory, memory_after);
        assert_eq!(record.last_step().unwrap().result, result);
    }

    #[test]
    fn local_memory_apply_matches_append_and_evict_rules() {
        let budgets = make_budgets();
        let mut memory = make_memory();

        apply_memory_delta_locally(&mut memory, &MemoryDelta::Append(12), &budgets).unwrap();
        assert_eq!(memory.items, VecDeque::from(vec![10, 11, 12]));

        apply_memory_delta_locally(&mut memory, &MemoryDelta::Append(13), &budgets).unwrap();
        assert_eq!(memory.items, VecDeque::from(vec![10, 11, 12, 13]));

        apply_memory_delta_locally(
            &mut memory,
            &MemoryDelta::EvictAndAppend {
                evicted: 10,
                appended: 99,
            },
            &budgets,
        )
        .unwrap();
        assert_eq!(memory.items, VecDeque::from(vec![11, 12, 13, 99]));
    }

    #[test]
    fn local_memory_apply_rejects_wrong_eviction() {
        let budgets = make_budgets();
        let mut memory = make_memory();

        let err = apply_memory_delta_locally(
            &mut memory,
            &MemoryDelta::EvictAndAppend {
                evicted: 99,
                appended: 12,
            },
            &budgets,
        )
        .unwrap_err();

        assert_eq!(err, "memory eviction did not match expected oldest token");
    }

    #[test]
    fn replace_all_respects_budget() {
        let budgets = make_budgets();
        let mut memory = make_memory();

        apply_memory_delta_locally(
            &mut memory,
            &MemoryDelta::ReplaceAll(VecDeque::from(vec![1, 2, 3])),
            &budgets,
        )
        .unwrap();

        assert_eq!(memory.items, VecDeque::from(vec![1, 2, 3]));
        assert_eq!(memory.max_items, budgets.max_memory_items);
    }

    #[test]
    fn replay_record_keeps_control_metadata() {
        let frame = make_frame();
        let memory = make_memory();
        let control = StepControl {
            policy_flags: 3,
            debug_flags: 9,
            force_halt: false,
        };

        let record = ReplayRecord::new(frame, memory, make_budgets(), control);

        assert_eq!(record.control.policy_flags, 3);
        assert_eq!(record.control.debug_flags, 9);
        assert!(!record.control.force_halt);
    }
}
