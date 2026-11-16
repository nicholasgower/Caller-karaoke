# ABC editor roadmap

**Status:** 0.1.0 has a minimal editor (see the user guide at
`website/docs/guides/abc-editor.md`). **Inspiration:** Michael Eskin's
[abctools](https://github.com/seisiuneer/abctools); this is an independent
implementation, not a port.

The first version deliberately does one thing: edit the ABC text of a saved tune
and show the engraved result. The items below are the improvements worth
considering, roughly in order of user value and implementation cost.

## 1. In-editor validation and navigation

- A syntax-highlighting text area (ABC headers, notes, chords, comments, bars).
- A line-number gutter with an error marker on the line the parser reports.
- Clicking a validation message scrolls to and selects the offending text.
- Parse-error messages that include the ABC line and column, not only the
  parser's byte offset.
- A list of parser warnings (missing meter, unsupported key, approximate repeat
  endings) next to the errors, because the player already reports them.

## 2. Editing tools

- Transpose the draft by a chosen interval, rewriting `K:` and the accidentals
  consistently.
- Split or join parts (`P:` / `|:` / `:|`) and extract one part into a new tune.
- Convert between common ABC dialects (modal keys, `M:C|`, Unicode fractions)
  with a visible diff.
- Whole-tune operations: set meter, unit note length, key, and title without
  editing the raw header by hand.
- Formatting helpers: one bar per line, consistent bar spacing, and trimming
  blank lines.
- Search and replace within the editor, including a chord-symbol-only mode.
- A chord checker that flags symbols the accompaniment resolver cannot use.
- A comparison view against `original_abc` with restore-one-hunk support.

## 3. History and safety

- Undo/redo for the editor draft with a bounded history, separate from the
  application-wide undo stack.
- Autosave a draft per tune so leaving the screen by a tab, swipe, or accidental
  window close does not lose a long edit.
- A confirmation when a draft differs from the saved text and the user leaves.
- Import/export the draft as a file from inside the editor, and copy it to the
  clipboard.
- Record a small library-level audit trail (edited at, size delta) so a caller
  can see which saved tunes were changed locally.

## 4. Preview and playback

- A play button that renders only the draft at the session tempo without
  replacing the loaded tune.
- A beat ruler under the staff, linked to the call timeline, so a caller can see
  where the 64-beat loop will cut a long tune.
- A warning when the draft's body is shorter or longer than 64 dance beats, with
  the arranger's repeat/trim decision shown before saving.
- A/B playback between the draft and the saved text.
- Show the parsed title, key, meter, and rhythm beside the field, with an
  explicit **Use these headers** action. The current editor deliberately leaves
  metadata alone; a future version can offer the re-read instead of doing it
  silently.

## 5. Import and export

- Split a multi-tune ABC file into separate saved tunes instead of importing
  only the first one and reporting the rest.
- A batch import screen with a preview before saving.
- A JSON import that accepts an array of dances and lets the user choose one.
- Export the edited tune directly to the OS "open with" flow on Android and as
  an ABC file in a chosen folder on every platform.
- Import from a URL or clipboard, behind the same size and validation limits as
  a local file.

## 6. Keyboard and accessibility

- Shortcuts: `Ctrl/Cmd+S` save, `Ctrl/Cmd+Z` draft undo, `Esc` cancel with a
  confirm when the draft is dirty, and a shortcut to toggle the preview.
- Screen-reader announcements for parse errors and save results, not only
  visible labels.
- A larger-text/one-column editor at phone width and with the soft keyboard
  open; the first version stacks, but a docked toolbar would be better.
- Full keyboard focus order and a visible focus ring on every control.

## 7. Platform parity

- Android: share the edited ABC directly with `ACTION_SEND` from the editor and
  open a chosen file in the editor from the document picker.
- Web: `beforeunload` protection for an unsaved draft and a `File System Access
  API` path where the browser supports it.
- Desktop: remember the last export directory and offer recent files.

## Out of scope for 0.1.0

- Using an external engraver or a hosted parser.
- Copying abctools code or assets.
- A full notation editor with mouse note entry; the ABC text stays the source of
  truth.
