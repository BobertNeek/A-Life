# Wordless speech composer — 2026-10-18

Implementation source: [a694cc3b](https://github.com/BobertNeek/A-Life/commit/a694cc3b30bb4e7bd962c0eb11d45b1ecef15f51),
based on main `a9ac8bf8`. [PR #89](https://github.com/BobertNeek/A-Life/pull/89)
contains subsequent CI/report changes and their current hosted results.

The composer now shares the care dock's teal bevels and cream pictograms. Four
48px controls address nearby listeners/the selected creature, send, cancel, and
show keyboard help. Recipient names typed into the draft retain precedence.
Pending, rejected, and unavailable cues describe dispatch without claiming that
a creature heard, understood, or learned. Typed teaching content and the ASCII
end caret are the ordinary text. Optional help and accessibility labels explain
the controls.

The toolbar stays outside the scrolling draft viewport. Height budgeting rounds
up to whole pixels so the HUD exclusion budget covers Bevy's rounded geometry.
The draft retains its 512-character and Unicode input policy. Mouse/keyboard
controls share cancellation/address handling and the original send dispatcher.
Closing retains capture until held keys/buttons release. Focus enters the draft
and returns to its prior entity on close. AccessKit click, focus, and bounded
SetValue requests drain while closed, behind a menu, or unfocused, preventing
later replay. Production routing, translation workers, and completion ownership
are unchanged; PR #84's native critic qualification and the PC are untouched.

Local results on that implementation source:

- `bash scripts/check_speech_cpu.sh`: 25 speech/composer cases passed (7 state,
  6 worker, 6 opening-input, 6 composer UI/control). The CI follow-up adds the
  separately passed 15 care-dock cases to this same lane.
- The complete care-dock CPU suite: 15 passed, including composer exclusion
  bounds at 1280×720, 640×360, 320×480 and 320×240, short/error states, actual
  CPU UI focus hit tests, held-pointer capture, and preserved draft text.
- Static core boundaries, 77/77 documentation assertions, formatting and
  whitespace checks passed. The font cmap gate passed. The opening-boundary
  source contracts and all 15 Python reference-model cases passed.
- A browser replay of actual CPU Bevy container geometry and shipping images
  passed 24 fixtures: long 512-character drafts, optional help and broadcast
  drafts with pending/rejection cues across four viewports/error gutters. All
  images loaded, no browser errors occurred, and each displayed end caret was
  inside its clipped draft viewport. Narrow draft/help screenshots were visually
  inspected. The replay uses the embedded Bevy font with browser text layout.

Generated evidence remains under ignored
`target/artifacts/speech-composer/` in the cloud checkout. Reproduce the export
and replay with the commands in [Development](../DEVELOPMENT.md). SHA-256:

| Artifact | Digest |
| --- | --- |
| CPU geometry JSON | `9a669e1ca21a32d093b4d60275cf0ad999f7c672e8d28ee6e43fad44afd57914` |
| Browser checks JSON | `ea287be26ff21874d08628ab892640bc10e1e4eb016423ab595d083afa1da84c` |
| Composer source | `bc2beb92c4a78e62dca085954cbf34d709f85001f819021886e0179e35917541` |

The CPU fixtures call production helpers and Bevy UI hit-testing, with synthetic
state and UI-stack/visibility setup. They do not instantiate the GPU-only input
or completion handlers. The browser replay is rendered visual evidence of that
replay, not a native Bevy screenshot. Native glyph layout/compositing, OS
screen-reader operation, physical-GPU production input, asynchronous emission
through the live handlers, and causal creature speech acceptance remain
unperformed. No neural simulation or training was launched here.
