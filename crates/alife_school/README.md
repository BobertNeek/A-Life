# alife_school

External in-world teacher and curriculum contracts.

This crate models teacher roles, lesson APIs, verification, and curriculum scaffolding. Teacher systems must instruct through ordinary perception channels such as hearing, vision, writing, gesture, object highlighting, visible reward/punishment, and social feedback; they must not directly inject hidden motor commands, hidden vectors, private reward signals, or weight edits.

The P23 scaffold includes:

- a versioned teacher-school schema contract;
- perception-only teacher events and channel checks;
- a grounded object/food/poison curriculum sequence;
- lesson response metadata that can annotate action candidates without changing arbitration;
- a simple headless curriculum runner; and
- verifiers over sealed `ExperiencePatch` logs plus memory/topology summaries.
