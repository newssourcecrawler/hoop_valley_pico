# hoop_valley_pico

A tiny Rust runtime demo for structured state evolution.

`hoop_valley_pico` is a small deterministic runtime that advances bounded state one step at a time, records replayable transitions, and applies explicit memory deltas.

It is not a chatbot, not an agent wrapper, and not a full LLM runtime.

It is a minimal public demo of:

- structured state evolution
- replayable step records
- explicit memory mutation
- bounded runtime progression

## Why

Many current AI systems emphasize generation first.

This repo explores a different center:

```text
state -> governed step -> future state
```

The runtime keeps each step explicit:

- input at tick `N`
- memory before
- state transition
- memory delta
- memory after
- resulting state

## What is in here

- `StepInput` / `StepResult`
- `FrameState`
- `MemoryView` / `MemoryDelta`
- `ReplayRecord`
- a tiny deterministic `Engine`
- a basic evaluator stub
- one runnable example

## Run

```bash
cargo run --example structured_state_demo
```

## Example output

```text
hoop_valley_pico :: structured state evolution demo
frame_id=7
steps=5

tick=0
  input_token=Some(10)
  memory_before=[]
  memory_delta=Append(10)
  memory_after=[10]
  state_slots=[11, 0, 11, 1]
  probes=pressure:11 memory_load:0 drift:11
  status=Ok phase=Evaluate

tick=1
  input_token=Some(21)
  memory_before=[10]
  memory_delta=Append(21)
  memory_after=[10, 21]
  state_slots=[16, 1, 15, 1]
  probes=pressure:16 memory_load:250 drift:15
  status=Ok phase=Evaluate

tick=2
  input_token=Some(34)
  memory_before=[10, 21]
  memory_delta=Append(34)
  memory_after=[10, 21, 34]
  state_slots=[17, 3, 14, 1]
  probes=pressure:17 memory_load:500 drift:14
  status=Ok phase=Evaluate

tick=3
  input_token=None
  memory_before=[10, 21, 34]
  memory_delta=None
  memory_after=[10, 21, 34]
  state_slots=[19, 5, 14, 0]
  probes=pressure:19 memory_load:750 drift:14
  status=Ok phase=Evaluate

tick=4
  input_token=Some(55)
  memory_before=[10, 21, 34]
  memory_delta=Append(55)
  memory_after=[10, 21, 34, 55]
  state_slots=[25, 8, 17, 1]
  probes=pressure:25 memory_load:750 drift:17
  status=Halted phase=Finalize

final_frame.step_index=5
final_frame.active=false
final_memory=[10, 21, 34, 55]
```

## Design notes

This demo keeps the runtime intentionally small:

- one step at a time
- deterministic behavior
- explicit replay
- explicit memory application
- no hidden async
- no concurrency claims

The evaluator included here is deliberately simple. It exists only to demonstrate the runtime law.

## Not yet included

- multi-agent scheduling
- multi-event circulation
- multiple evaluator families
- LLM-specific runtime integration
- advanced routing or zone activation

## Direction

This repo is a pico-scale public slice of a broader runtime direction:

- bounded evaluation
- structured state evolution
- replayable future-state steps
- multiple data types under one runtime law
