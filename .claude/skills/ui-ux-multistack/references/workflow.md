# Workflow: eight adaptive phases

Adapted from the evondevKit design process (brief, wireframes, user choice, build).
Run only the phases the task size needs (see SKILL.md Step 3).

## Phase 1: Discovery

Read before asking. Gather from the repo, existing screens, README, DESIGN.md, and
the request itself. Then write a brief of at most 15 lines:

```
Product:        what it is, in one sentence
Users:          who, how often, how skilled, on what device and window size
Main job:       the one thing each screen exists to do
Workflows:      the 2-4 paths that matter most
Constraints:    stack, deployment target, brand, performance, legal
Existing look:  tokens, components, conventions already in the code
Unknowns:       what you assumed, marked as assumptions
```

Ask at most one question, and only if the answer changes the design. Otherwise assume,
label the assumption, move on.

## Phase 2: Analysis

- Run `scripts/detect-stack.sh`, open one real file per surface.
- Inventory what already exists for the thing you are about to build (component,
  token, pattern). Reuse beats rebuild.
- If screens exist, capture them (see visual-verification.md) and list problems by
  category: product, UX, visual, technical, taste.
- Check for `DESIGN.md`. If present, its decisions win over this skill's defaults.

## Phase 3: Design direction

Pick one direction from `aesthetic-directions.md`, with two reasons tied to the brief.
State what the direction rules out. A direction chosen without reasons is decoration.

## Phase 4: Layout and wireframing

1. Rank content: primary action, primary content, secondary, tertiary, chrome.
2. Choose the structural pattern (`components-patterns.md`).
3. Produce 2 to 3 wireframes that differ in structure, not in color. Each gets one
   line: what it optimizes, what it costs. ASCII is fine:

```
A: Sidebar + list + inspector        B: Top tabs + full-width table
+----+-----------+--------+          +-------------------------------+
| nav| list      | detail |          | tab tab tab        [search]   |
|    |           |        |          +-------------------------------+
+----+-----------+--------+          | table ...                     |
```

4. State which you would pick. **Gate: user chooses**, unless they said "just build it".
   A user-supplied wireframe or mockup image counts as the chosen option; build from it.

## Phase 5: Design specification

Write the spec once for the whole set of screens (see design-system.md):
tokens, type scale, color roles for light and dark, spacing and radius scales,
component list with every state, motion durations and easings, density.
If a system exists, the spec is a diff against it, not a new document.

## Phase 6: Implementation

- Build in the stack the detector found, following that stack's conventions.
- Use the project's components first. Create new ones only for gaps, named after the
  existing naming scheme.
- Build every state as a static example next to the live one (preview or specimen
  page). Do not rely on clicking through to see an empty or error state.
- Leave handlers and data hooks as typed stubs. Do not invent application logic.

## Phase 7: Visual verification

See `visual-verification.md`. Capture, measure, compare against the spec and the chosen
wireframe, report the claim level honestly.

## Phase 8: Refinement

Fix defects Phase 7 actually found. Rank by user impact. Do not restyle things that
passed. One refinement loop per round; a second round only for defects the first
round exposed. Stop when the findings list is empty or the remaining rows are taste.

## Review mode (existing UI, no build)

1. Capture the real thing: every main screen, both appearances, narrow and wide window.
2. Produce a findings table, one row per problem:

| # | Where | Category | Problem | Evidence | Fix | Impact |
|---|---|---|---|---|---|---|

3. Rank by impact. Mark rows that are taste, not defect.
4. **Do not edit until the user picks rows.** Then fix only those rows.
5. After fixing, re-capture and show before and after for the changed rows.

## Compare-with-original mode

Capture both versions identically (same size, same data, same appearance). Table:
criterion, original, new, which is better. You must look for rows where the new
version loses, and propose a fix for each.

## Keep-the-brand mode

Keep the page frame, color roles, and typeface. Replace off-system controls with the
project's own components, normalize spacing and radius to the scale, change nothing
else. Say what you left alone.

## Stop conditions

Stop and report instead of continuing when: scope grows beyond what was approved;
the needed spec is missing and filling it means inventing product decisions; a
required renderer is unavailable and the user must choose to continue unverified.
