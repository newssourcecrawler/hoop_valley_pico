use crate::eval::basic::BasicEvaluator;
use crate::runtime::replay::{ReplayRecord, apply_memory_delta_locally};
use crate::runtime::types::{
    FrameState, MemoryDelta, MemoryView, ProbeSurface, StepBudgets, StepControl, StepInput,
    StepPhase, StepResult, StepStatus, TokenId,
};

#[derive(Debug, Default)]
pub struct Engine {
    evaluator: BasicEvaluator,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            evaluator: BasicEvaluator::new(),
        }
    }

    pub fn step(&self, input: StepInput) -> Result<StepResult, &'static str> {
        Self::validate_input(&input)?;

        if input.control.force_halt || matches!(input.frame_state.phase, StepPhase::Finalize) {
            let mut next_state = input.frame_state.clone();
            next_state.active = false;
            next_state.phase = StepPhase::Finalize;
            next_state.step_index = next_state.step_index.saturating_add(1);

            return Ok(StepResult {
                frame_id: input.frame_id,
                tick: input.tick,
                phase: input.frame_state.phase,
                next_state,
                probes: ProbeSurface::default(),
                memory_delta: MemoryDelta::None,
                status: StepStatus::Halted,
            });
        }

        let mut next_state = input.frame_state.clone();

        let eval = self.evaluator.evaluate(
            &input.frame_state,
            &input.memory_view,
            input.input_token,
            &input.budgets,
        );

        next_state.state_slots = eval.next_slots;
        next_state.step_index = next_state.step_index.saturating_add(1);
        next_state.phase = Self::next_phase(input.frame_state.phase);

        if !matches!(eval.memory_delta, MemoryDelta::None) {
            next_state.emitted_events = next_state.emitted_events.saturating_add(1);
        }

        if input.budgets.max_steps > 0 && next_state.step_index >= input.budgets.max_steps {
            next_state.active = false;
            next_state.phase = StepPhase::Finalize;
        }

        let status = if next_state.active {
            StepStatus::Ok
        } else {
            StepStatus::Halted
        };

        Ok(StepResult {
            frame_id: input.frame_id,
            tick: input.tick,
            phase: input.frame_state.phase,
            next_state,
            probes: eval.probes,
            memory_delta: eval.memory_delta,
            status,
        })
    }

    pub fn run(
        &self,
        initial_frame: FrameState,
        initial_memory: MemoryView,
        inputs: &[Option<TokenId>],
        budgets: StepBudgets,
        control: StepControl,
    ) -> Result<ReplayRecord, &'static str> {
        let mut frame = initial_frame.clone();
        let mut memory = initial_memory.clone();
        let mut record = ReplayRecord::new(initial_frame, initial_memory, budgets, control);

        for (tick, input_token) in inputs.iter().copied().enumerate() {
            if !frame.active {
                break;
            }

            let memory_before = memory.clone();

            let step_input = StepInput {
                frame_id: frame.frame_id,
                tick: tick as u64,
                input_token,
                frame_state: frame.clone(),
                memory_view: memory.clone(),
                budgets,
                control,
            };

            let result = self.step(step_input)?;
            apply_memory_delta_locally(&mut memory, &result.memory_delta, &budgets)?;
            frame = result.next_state.clone();

            record.push_step(
                tick as u64,
                input_token,
                memory_before,
                result,
                memory.clone(),
            );
        }

        Ok(record)
    }

    fn validate_input(input: &StepInput) -> Result<(), &'static str> {
        if input.frame_id != input.frame_state.frame_id {
            return Err("frame_id does not match frame_state.frame_id");
        }
        if input.tick != input.frame_state.step_index {
            return Err("tick does not match frame_state.step_index");
        }
        if input.memory_view.max_items != input.budgets.max_memory_items {
            return Err("memory_view.max_items does not match budgets.max_memory_items");
        }
        if input.frame_state.state_slots.len() > input.budgets.max_state_slots {
            return Err("state_slots exceed budgets.max_state_slots");
        }
        if input.memory_view.len() > input.budgets.max_memory_items {
            return Err("memory items exceed budgets.max_memory_items");
        }
        if !input.frame_state.active {
            return Err("frame is not active");
        }
        Ok(())
    }

    fn next_phase(current: StepPhase) -> StepPhase {
        match current {
            StepPhase::Prefill => StepPhase::Evaluate,
            StepPhase::Evaluate => StepPhase::Evaluate,
            StepPhase::Finalize => StepPhase::Finalize,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn make_frame() -> FrameState {
        FrameState::new(7, 4)
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
            max_steps: 4,
        }
    }

    #[test]
    fn step_is_deterministic_for_same_input() {
        let engine = Engine::new();
        let input = StepInput {
            frame_id: 7,
            tick: 0,
            input_token: Some(5),
            frame_state: make_frame(),
            memory_view: make_memory(),
            budgets: make_budgets(),
            control: StepControl::default(),
        };

        let a = engine.step(input.clone()).unwrap();
        let b = engine.step(input).unwrap();

        assert_eq!(a, b);
    }

    #[test]
    fn force_halt_finalizes_immediately() {
        let engine = Engine::new();
        let out = engine
            .step(StepInput {
                frame_id: 7,
                tick: 0,
                input_token: Some(5),
                frame_state: make_frame(),
                memory_view: make_memory(),
                budgets: make_budgets(),
                control: StepControl {
                    force_halt: true,
                    ..StepControl::default()
                },
            })
            .unwrap();

        assert_eq!(out.status, StepStatus::Halted);
        assert_eq!(out.next_state.phase, StepPhase::Finalize);
        assert!(!out.next_state.active);
        assert!(matches!(out.memory_delta, MemoryDelta::None));
    }

    #[test]
    fn run_builds_replay_record() {
        let engine = Engine::new();
        let record = engine
            .run(
                make_frame(),
                make_memory(),
                &[Some(5), Some(9), None],
                make_budgets(),
                StepControl::default(),
            )
            .unwrap();

        assert!(!record.is_empty());
        assert_eq!(record.frame_id, 7);
        assert_eq!(record.steps[0].tick, 0);
        assert_eq!(
            record.steps[0].memory_before.items,
            VecDeque::from(vec![10, 11])
        );
    }
}
