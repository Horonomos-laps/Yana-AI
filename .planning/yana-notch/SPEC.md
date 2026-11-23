# Yana Notch: rewrite blueprint from reading notchd (2026-11-22)

Read in full: NotchController, NotchModel, NotchView (shape and root view), LiveActivities, Agents, plus a skim of the
rest. Not built, not run. This file describes mechanisms and numbers in our own words; Yana's code is written fresh
and credited in yana-app/CREDITS.md.

## 1. Window

- One panel per screen. Borderless + non-activating. Clear, no shadow of its own, not movable.
- Window level a little above the menu bar (mainMenu + 3). collectionBehavior: all Spaces, stationary,
  full-screen auxiliary, ignores cycle. May become key (for text fields), never main.
- Frame: full screen width, 280 pt tall, pinned to the top of the screen. The SwiftUI shape is drawn at top-center;
  the empty rest of the window must not eat clicks.
- Click pass-through: ignoresMouseEvents is true by default and becomes false only while the pointer is inside the
  hit rectangle (or a drag is in progress). The hit rectangle is the current shape plus 10 pt padding when closed,
  4 pt when open.
- Hosting view accepts the first mouse click (so a click works without activating the app).
- No Accessibility permission: hover comes from global and local NSEvent monitors (mouse moved, dragged, down, up),
  scroll from a local monitor, keys from a local monitor limited to events whose window is the panel.

## 2. Geometry

- Notch size: safeAreaInsets.top > 0 with both auxiliaryTopLeftArea and auxiliaryTopRightArea;
  width = screen width - left width - right width; height = safeAreaInsets.top. Otherwise no notch.
- No notch: draw a small handle instead (width and height are settings; defaults small, about 3-4 pt corner radii).
- Sizes (body between the two flared "ears"):
  - closed: the notch size
  - hover, notch screen: +16 wide, +4 high; no notch: +30 wide, 22 high
  - activity: notch width + 2 x 46 side wings; row height max(notch height, 30); +8 wide and +3 high when hovered
  - glance (a line slides out below the activity): +70 wide, +26 high
  - open: set by its content
- Corner radii (top, bottom): closed notch (6, 10), closed no notch (3, 4), hover or activity (8, 13),
  glance (10, 18), open (14, 26).
- Shape: top corners are concave quads that flare outward into the menu bar; bottom corners are convex quads.
  Total width = body + 2 x top radius. Radii are animatable so the shape morphs.
- Fullscreen: a private CoreGraphics call reads each display's current Space type (4 means fullscreen). If it
  disappears, report not fullscreen. User chooses: all monitors, notched screens only, or never.

## 3. Motion

- Open: spring response 0.42, damping 0.68 (overshoots). Close: 0.36 / 0.88. Hover: 0.3 / 0.55.
- Content enters blurred (radius 10), scaled to 0.82 anchored at the top, opacity 0, then springs in
  (0.45 / 0.7, delayed 0.06 s); leaves in 0.14 s ease-out.
- Buttons squish to 0.86 on press and bounce back (0.25 / 0.5). A light haptic on open.
- Closed notch is solid black to hide the camera; open may be frosted.

## 4. Interaction

- Hover: set when the pointer enters the hit rectangle. Closing waits 0.15 s and is cancelled if the pointer comes
  back; never closes while locked, dragging, arranging, or typing (when "don't close while typing" is on).
- Click opens. Optional open on hover.
- Trackpad: two-finger scroll tracked per gesture (reset on phase began, one action per gesture). Down by more than
  20 opens, up by more than 20 closes, or hides the current activity if closed. Sideways by more than 40 and more
  than twice the vertical skips a track. Scrolling a list inside the open notch must not close it.
- Files dragged over a 400 x 90 zone at the top center open the tray. Detected by comparing the drag pasteboard's
  changeCount at mouse down and again while dragging; accepted types: file URL, URL, string, tiff, png, file promises.
- A drag that ends without a mouse-up we saw (dropped on another app) must not leave the notch stuck open.
- Keys only when the open tab wants them: tray (select, copy, paste, delete, rename, quick look), clipboard (arrows,
  return pastes, delete, escape closes and returns to the previous app).

## 5. Live activities (the part worth reusing as a concept)

- An activity is an enum case with a kind. Long-lived ones come from their own managers; short ones are "flashes"
  shown for 4-5 s.
- The user orders the kinds in Settings; the first enabled kind that is currently active wins. A HUD (volume,
  brightness) always wins; a launch greeting ("hello") shows once.
- Swipe up dismisses: flashes end at once; long-lived ones are remembered with a signature (what they showed:
  song, count, event). A dismissed activity comes back when its signature changes ("unhide automatically"), or when
  it ended and starts again.
- Per-kind switch for "also show over fullscreen apps".
- Glance: hovering or an auto-peek (3-4 s on news) slides a line of detail under the activity.

## 6. Coding agents

- Events travel by files: a tiny hook writes the event JSON to a hidden temp name in a spool folder, then renames it
  to name.json (atomic). The app watches the folder (DispatchSource on a file descriptor opened with O_EVTONLY),
  reads each .json once, deletes it. Files older than 10 minutes while running, or 60 s when found at startup, are
  history, not news.
- The hook only writes while the app is running (a pgrep guard), so the folder never piles up.
- Claude Code events: UserPromptSubmit (working, remember the frontmost app), PostToolUse and PreToolUse (still
  working, only for a session already known), Notification (waiting, but only for permission or question types;
  the "waiting for your input" reminder after finishing is not a question), Stop (done), StopFailure and
  SessionEnd (remove).
- Codex: a notify command receives the event JSON as the last argument; only "agent-turn-complete" is reported.
- Interruptions send no event: a working session is checked every 3 s; if its transcript's last line contains the
  interruption marker, or nothing was written for 10 minutes, it is dropped; waiting/done sessions expire after 2 h.
- Connecting: the app edits Claude Code's settings.json (adds one hook group per event, marked so it can be removed)
  and Codex's config.toml (a top-level notify line; an existing notify is wrapped so it still runs, and kept as a
  comment so Disconnect restores it exactly). Backups before every change. Symlinked dotfiles are resolved and kept.
- Click on an activity brings the session's terminal or editor forward (the app captured at prompt time).
- Done flash plays an optional sound.

## 7. What Yana does differently

- Yana's own runs come from yana-rt (authority pending-approvals) and the chat engine, not from hooks.
- Approval buttons on the notch (Allow / Deny), which notchd does not need.
- Same activity model, so Yana, Claude Code and Codex sessions share one priority list.
- Keep v1 small: shell, activity model with Yana + agents, approvals. Tray, clipboard, timers, notes later.

## 8. Test plan (pure logic first)

Geometry (sizes and radii per state, notch detection from insets), activity priority and dismissal signature,
event parsing (each Claude Code event, Codex, needs-you rule), spool drain (age limits, atomic rename, deletion),
hook install/uninstall on temp files (idempotent, backup, restore the original notify), pass-through hit rectangle.
Window behaviour (level, pass-through, fullscreen, multi-screen) needs a Mac with a notch.
