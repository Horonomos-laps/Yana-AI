# Attribution and provenance

## Source

This skill adapts the design methodology of **evondevKit** (`skills/ui-ux`, version
0.8.0), https://github.com/evondev/evondevKit, reviewed at commit
`9c03553222592e433f88429a19f831f0707dbf8f` (2026-11-07). evondevKit is MIT licensed.

No evondevKit file is copied verbatim. Ideas, structures, and workflow shapes were
studied and rewritten for Yana AI. This notice is kept because the methodology is a
substantial influence on the structure of this skill.

## What was adapted

| evondevKit idea | Where it appears here |
|---|---|
| Do not build before the entry questions are answered (gate-first) | SKILL.md Steps 1 to 3, core rules |
| Route by what the request says; default is the designer path | SKILL.md Step 2 |
| Brief, per-screen main job, 2 to 3 wireframes, user chooses, then build (branch U) | `workflow.md` phases 1 to 4 |
| Audit the stack and existing components before designing (tiers) | `scripts/detect-stack.sh`, `workflow.md` phase 2 |
| Review mode with a findings table and no edits until rows are picked (branch V) | `workflow.md` review mode |
| Compare-with-original and keep-the-brand modes | `workflow.md` |
| Design system first, specimen page, `DESIGN.md` as the record (branch D) | `design-system.md` |
| Named visual "faces" and a tone axis | `aesthetic-directions.md` |
| State examples built statically, not by clicking | `workflow.md` phase 6 |
| Measured checks plus eye inspection, rendered probes at many widths | `visual-verification.md` |
| Locked-rules idea: decisions the owner already made are not reopened | SKILL.md rule 2 (reuse) and `DESIGN.md` precedence |

## What was deliberately not adopted

- Dashboard-only and Tailwind-only assumptions. This skill covers native macOS, web,
  and hybrid desktop.
- Vietnamese-only prose and the numbered rule scheme (M, I, N, P, T...).
- The specific component specs, color values, tokens, and layout files.
- The probe script itself; verification here uses whatever renderer is available.
- Auto-update check; Yana manages skills through its own lock mechanism.

## License text (evondevKit)

MIT License

Copyright (c) 2026 Tuấn Trần

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

Yana AI additions in this skill are Apache-2.0, matching this repository.
