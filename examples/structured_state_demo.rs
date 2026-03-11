use hoop_valley_pico::runtime::engine::Engine;
use hoop_valley_pico::runtime::types::{FrameState, StepBudgets, StepControl};

fn main() {
    let engine = Engine::new();

    let initial_frame = FrameState::new(7, 4);
    let initial_memory = hoop_valley_pico::runtime::types::MemoryView::new(4);

    let budgets = StepBudgets {
        max_state_slots: 8,
        max_memory_items: 4,
        max_steps: 5,
    };

    let control = StepControl::default();

    let inputs = [Some(10), Some(21), Some(34), None, Some(55)];

    let record = engine
        .run(initial_frame, initial_memory, &inputs, budgets, control)
        .expect("engine run should succeed");

    println!("hoop_valley_pico :: structured state evolution demo");
    println!("frame_id={}", record.frame_id);
    println!("steps={}", record.len());
    println!();

    for step in &record.steps {
        println!("tick={}", step.tick);
        println!("  input_token={:?}", step.input_token);
        println!("  memory_before={:?}", step.memory_before.items);
        println!("  memory_delta={:?}", step.result.memory_delta);
        println!("  memory_after={:?}", step.memory_after.items);
        println!("  state_slots={:?}", step.result.next_state.state_slots);
        println!(
            "  probes=pressure:{} memory_load:{} drift:{}",
            step.result.probes.pressure_milli,
            step.result.probes.memory_load_milli,
            step.result.probes.drift_milli
        );
        println!(
            "  status={:?} phase={:?}",
            step.result.status, step.result.next_state.phase
        );
        println!();
    }

    println!("final_frame.step_index={}", record.final_frame.step_index);
    println!("final_frame.active={}", record.final_frame.active);
    println!("final_memory={:?}", record.final_memory.items);
}
