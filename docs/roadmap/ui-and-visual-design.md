## 8. UI and visual design

### 8.1 Theme tokens

Dark (default):

| Token | Value |
| --- | --- |
| Background | `#141414` |
| Panel | `#232323` |
| Button | `#2e2e2e` |
| Button hover | `#3a3a3a` |
| Button active | `#4a4a4a` |
| Primary text | `#e6e6e6` |
| Muted text | `#9a9a9a` |
| Active call | `#f2f2f2` on `#303030` |
| Call-now prompt card | `#1e4a2e` (green fill, white text) |
| Upcoming-move prompt card | `#602424` (red fill, white text) |
| Borders | `#3a3a3a` |

Light theme uses the same neutral structure with inverted values. No accent
colours; the active call is distinguished by contrast, weight, and a border.
All colours live in a theme module; do not hard-code colours in widgets.

The one deliberate exception is the large prompt card: it is green while the
caller must speak the shown call ("CALL NOW") and red while it looks ahead to
the move after the one being danced ("UPCOMING MOVE"), so the state is
readable at a glance across the room. The label text still states the state,
so the fill is never the only cue, and both fills keep white text at 4.5:1 or
better in the dark theme and dark text at 4.5:1 or better in the light theme.

### 8.2 Responsive behaviour

- Breakpoint: approximately 640 px.
- Tabs remain horizontally usable on mobile (scroll if necessary); the phone
  layout uses short tab labels and drops the wordmark to keep one row.
- Session controls stack on narrow screens: tempo and volume sliders span the
  width, transport buttons split one row, and the remaining toggles wrap.
- Minimum 44 px touch targets.
- Avoid hover-only interactions.
- The dance area scrolls; the current call stays visible. On short windows
  (under about 800 px of session height, including a landscape phone) the whole
  session page scrolls so the card stays reachable.
- The pickers become full-screen sheets on mobile and dialogs on desktop.
- Respect safe-area insets on mobile.

### 8.3 Accessibility

- Keyboard: Space = play/pause, R = restart, Esc = back, 1–4 = switch tabs,
  Enter = submit a search or add an alias. Shortcuts are suspended while a text
  field has focus.
- Every control has a text label or accessibility label.
- Text contrast meets WCAG AA against the chosen background.
- TTS is optional and never the only way to receive a call.
- Do not flash or pulse faster than 3 Hz.

### 8.4 Error and loading states

- Network error: keep cached content, show a dismissible banner with retry.
- Parse error: show the source URL and a short reason; do not replace the
  current dance/tune.
- Audio render: "Preparing music…" with a cancel action.
- No soundfont: built-in synth remains active.
- No TTS: disable the toggle and explain why.
