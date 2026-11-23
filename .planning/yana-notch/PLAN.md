# Yana Notch: plan (drafted 2026-11-22)

Decision (user, 2026-11-22): take the ideas of notchd widely, write Yana's own code, credit the source properly.
Native Swift inside the Yana app (SwiftUI + AppKit), switched on in Settings. Study: .claude/assistant/repo-analyses/notchd.md

## Attribution rules

1. No notchd source is pasted. Every file is written fresh; the idea is credited, not the text.
2. yana-app/CREDITS.md lists each borrowed idea with the notchd file it came from, and the Apache 2.0 notice of that project.
3. A short credit line in Settings > About that links to the notchd repo.
4. If any piece of code ever is copied, it keeps its license header and is listed in CREDITS.md as copied.

## Phases (each ends with tests, a snapshot, and one honest note on what is not verified)

1. Shell: borderless panel over the notch, measured size (safeAreaInsets), shape, spring open/close, hover and click,
   click-through outside the shape, handle on a screen without a notch, off by default, Settings switch.
2. Yana live activity: running / waiting for approval / denied / done, a glance line, Allow and Deny on the notch
   (from "authority pending-approvals"), jump back to the app.
3. Coding agents: Claude Code and Codex sessions through a spool folder of JSON events (idea from notchd Agents).
4. Widgets worth having for a work tool: timer and Pomodoro, notes, to-do linked to Yana Tasks, tray for files and
   images dropped onto the notch (to attach to a chat), clipboard history.
5. Polish: priorities between activities, per-activity switches, fullscreen behaviour, haptics, multi-screen.

## Not decided

- Media with lyrics, calendar, weather, bluetooth, battery: possible later, not core to Yana.
- Needs a Mac with a notch to verify phases 1 and 5.
