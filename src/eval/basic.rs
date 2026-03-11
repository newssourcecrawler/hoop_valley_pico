use crate::runtime::types::{
    FrameState, MemoryDelta, MemoryView, ProbeSurface, StepBudgets, TokenId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalOutput {
    pub next_slots: Vec<i32>,
    pub probes: ProbeSurface,
    pub memory_delta: MemoryDelta,
}

#[derive(Debug, Default)]
pub struct BasicEvaluator;

impl BasicEvaluator {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        frame_state: &FrameState,
        memory_view: &MemoryView,
        input_token: Option<TokenId>,
        budgets: &StepBudgets,
    ) -> EvalOutput {
        let mut next_slots = frame_state.state_slots.clone();

        let memory_load_milli = if budgets.max_memory_items == 0 {
            0
        } else {
            ((memory_view.len() as u32) * 1000 / (budgets.max_memory_items as u32)).min(1000)
        };

        let token_bias = input_token.map(|t| (t % 17) as i32).unwrap_or(0);
        let pressure_bump = if memory_load_milli >= 750 { 2 } else { 1 };
        let recovery_bump = if input_token.is_none() { 1 } else { 0 };

        if !next_slots.is_empty() {
            next_slots[0] = next_slots[0].saturating_add(pressure_bump + token_bias);
        }
        if next_slots.len() > 1 {
            next_slots[1] =
                next_slots[1].saturating_add((memory_view.len() as i32) - recovery_bump);
        }
        if next_slots.len() > 2 {
            let drift = next_slots[0].saturating_sub(next_slots[1]).abs();
            next_slots[2] = drift;
        }
        if next_slots.len() > 3 {
            next_slots[3] = if input_token.is_some() { 1 } else { 0 };
        }

        let memory_delta = self.choose_memory_delta(memory_view, input_token, budgets);

        let drift_milli = if next_slots.len() > 2 {
            (next_slots[2].unsigned_abs().min(1000_u32)).min(1000)
        } else {
            0
        };

        let pressure_milli = (next_slots
            .first()
            .copied()
            .unwrap_or_default()
            .unsigned_abs()
            .min(1000_u32))
        .min(1000);

        EvalOutput {
            next_slots,
            probes: ProbeSurface {
                pressure_milli,
                memory_load_milli,
                drift_milli,
            },
            memory_delta,
        }
    }

    fn choose_memory_delta(
        &self,
        memory_view: &MemoryView,
        input_token: Option<TokenId>,
        budgets: &StepBudgets,
    ) -> MemoryDelta {
        let Some(token) = input_token else {
            return MemoryDelta::None;
        };

        if budgets.max_memory_items == 0 {
            return MemoryDelta::None;
        }

        if memory_view.len() < budgets.max_memory_items {
            return MemoryDelta::Append(token);
        }

        match memory_view.items.front().copied() {
            Some(evicted) => MemoryDelta::EvictAndAppend {
                evicted,
                appended: token,
            },
            None => MemoryDelta::Append(token),
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

    fn make_memory(items: &[TokenId], max_items: usize) -> MemoryView {
        let mut memory = MemoryView::new(max_items);
        memory.items = VecDeque::from(items.to_vec());
        memory
    }

    fn make_budgets(max_memory_items: usize) -> StepBudgets {
        StepBudgets {
            max_state_slots: 8,
            max_memory_items,
            max_steps: 8,
        }
    }

    #[test]
    fn evaluate_is_deterministic_for_same_input() {
        let evaluator = BasicEvaluator::new();
        let frame = make_frame();
        let memory = make_memory(&[10, 11], 4);
        let budgets = make_budgets(4);

        let a = evaluator.evaluate(&frame, &memory, Some(5), &budgets);
        let b = evaluator.evaluate(&frame, &memory, Some(5), &budgets);

        assert_eq!(a, b);
    }

    #[test]
    fn appends_when_memory_has_room() {
        let evaluator = BasicEvaluator::new();
        let out = evaluator.evaluate(
            &make_frame(),
            &make_memory(&[10, 11], 4),
            Some(42),
            &make_budgets(4),
        );

        assert!(matches!(out.memory_delta, MemoryDelta::Append(42)));
    }

    #[test]
    fn evicts_when_memory_is_full() {
        let evaluator = BasicEvaluator::new();
        let out = evaluator.evaluate(
            &make_frame(),
            &make_memory(&[10, 11, 12, 13], 4),
            Some(99),
            &make_budgets(4),
        );

        assert_eq!(
            out.memory_delta,
            MemoryDelta::EvictAndAppend {
                evicted: 10,
                appended: 99,
            }
        );
    }

    #[test]
    fn none_input_does_not_mutate_memory() {
        let evaluator = BasicEvaluator::new();
        let out = evaluator.evaluate(
            &make_frame(),
            &make_memory(&[10, 11], 4),
            None,
            &make_budgets(4),
        );

        assert!(matches!(out.memory_delta, MemoryDelta::None));
    }

    #[test]
    fn probes_stay_bounded() {
        let evaluator = BasicEvaluator::new();
        let out = evaluator.evaluate(
            &make_frame(),
            &make_memory(&[1, 2, 3, 4], 4),
            Some(500),
            &make_budgets(4),
        );

        assert!(out.probes.pressure_milli <= 1000);
        assert!(out.probes.memory_load_milli <= 1000);
        assert!(out.probes.drift_milli <= 1000);
    }
}
