# Design intelligence

How to judge before building. The aim is independent design judgment: understand the
product, name the real problem, and be willing to disagree with a weak decision.

## 1. Understand before designing

Answer these from evidence, not from taste:

- What is the product for, and who pays or depends on it?
- Who uses this screen, how often, under what pressure? (daily operator vs once-a-year admin)
- What is the one job of this screen? If you list three, rank them.
- What does the user hold in their head when they arrive? What must they see first?
- What existing visual identity and platform conventions apply?
- What are the technical limits (window sizes, data volume, offline, latency)?

## 2. Classify every problem

| Category | Question it answers | Typical fix owner |
|---|---|---|
| Product | Is this the right thing to build or show? | Scope, content, flow |
| UX | Can the user finish the task without friction? | Structure, labels, steps, feedback |
| Visual | Is hierarchy, spacing, type, color working? | Tokens, layout |
| Technical | Does it render and perform correctly? | Code |
| Taste | Is it a preference with no evidence either way? | Report as preference only |

Fix order: product, UX, visual, technical. Taste last, and only if asked.
A visual polish on a screen with a product problem is wasted work; say so.

## 3. Challenge protocol

When the request or the existing design is the weak point:

1. Name the decision in one sentence ("The settings are split across three modals").
2. Give the evidence or principle (task steps counted, a heuristic below, a measurement).
3. Offer a better alternative and its cost.
4. Let the user decide. If they keep the original, do it well and note the trade-off once.

Do not challenge to look clever. Challenge only when you can point at a concrete cost.

## 4. Usability heuristics (fast pass)

Score each as pass, weak, or fail with one line of evidence.

| Heuristic | Check |
|---|---|
| Visibility of status | Does every action show progress, success, or failure? |
| Match with the user's world | Labels use the user's words, not internal names |
| Control and freedom | Undo, cancel, escape, close on every transient state |
| Consistency | Same thing looks and behaves the same everywhere |
| Error prevention | Destructive actions confirmed or undoable; bad input blocked early |
| Recognition over recall | Options visible; recent and suggested items offered |
| Efficiency | Shortcuts, bulk actions, sensible defaults for repeat users |
| Minimal design | Every element earns its place; no dead chrome |
| Error recovery | Messages say what happened and the next step |
| Help in context | Hints near the problem, not in a manual |

For macOS also check: menu bar coverage of every command, keyboard-only reachability,
window restoration, standard shortcuts (Cmd-W, Cmd-,, Cmd-F, Cmd-Z) behaving normally.

## 5. Hierarchy test

Squint at a capture, or blur it. You should still identify, in order: where am I,
what is the main content, what is the main action. If two elements compete, one must
lose weight (size, contrast, or position), not both gain decoration.

## 6. Cut before you add

For every proposed element ask: can this be removed, merged, or made a default
instead? Complexity is a cost the user pays on every visit. State what complexity a
proposal adds. The simpler option wins unless there is evidence for the extra.

## 7. Authority is not evidence

A reference app, a trend, or a stakeholder opinion is input. Evidence (measured
task time, contrast ratio, user observation, platform guideline) outranks it. When
you copy a pattern from a reference, say why it fits this product, and learn from
its structure, rhythm, and state handling, not its pixels or assets.

## 8. Subjective vs objective

Objective (defects): contrast below threshold, clipped text, broken alignment,
inconsistent tokens, unreachable control, missing state, layout break at a size.
Subjective (preferences): "feels heavy", "prefer rounder", color taste. Present
subjective items as options with a recommendation, never as errors.

## 9. Product-fit questions for direction

- Is this a tool people sit in all day, or a place they visit to decide something?
- Is trust the main emotion (finance, security) or delight (creative, play)?
- Is information density the value (logs, tables) or focus (writing, reading)?
- Does the brand already own a color, a typeface, a shape language?

The answers feed `aesthetic-directions.md`.
