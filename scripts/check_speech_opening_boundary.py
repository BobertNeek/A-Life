#!/usr/bin/env python3
"""Static Rust source contracts plus a lightweight Python reference model.

This does not compile or execute Rust, drive Bevy, or prove live input dispatch.
The actual Rust regression tests must still be built and run for this source tree.
"""

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import sys
import unittest


MAX_TYPED_CHARS = 512
HELPER_CONTRACT = """
fn append_player_speech_key_text<'a>(
    input: &mut String,
    messages: impl Iterator<Item = &'a KeyboardInput>,
    opened_this_frame: bool,
) -> bool {
    let mut after_opening_enter = !opened_this_frame;
    let mut submit_enter = false;
    for message in messages {
        if message.state != ButtonState::Pressed {
            continue;
        }
        if !after_opening_enter {
            after_opening_enter = message.key_code == KeyCode::Enter && !message.repeat;
            continue;
        }
        if !opened_this_frame && message.key_code == KeyCode::Enter && !message.repeat {
            submit_enter = true;
        }
        if let Some(text) = &message.text {
            for character in text.chars().filter(|character| !character.is_control()) {
                if input.chars().count() < MAX_TYPED_CHARS {
                    input.push(character);
                }
            }
        }
    }
    submit_enter
}
"""


def normalized(text):
    return re.sub(r"\s+", " ", text).strip()


def source_contract(source, tests, lockfile):
    for crate in ("bevy_input", "bevy_ecs"):
        assert f'name = "{crate}"\nversion = "0.18.1"' in lockfile, crate
    helper_start = source.index("fn append_player_speech_key_text<'a>(")
    helper_end = source.index("#[cfg(test)]", helper_start)
    assert normalized(source[helper_start:helper_end]) == normalized(HELPER_CONTRACT), (
        "ordered reader or existing printable/Unicode/length policy changed"
    )
    start = source.index("pub(crate) fn handle_production_conversation_lineage_input(")
    end = source.index("// KeyboardInput retains order;", start)
    handler = source[start:end]
    assert "const MAX_TYPED_CHARS: usize = 512;" in source
    assert "mod speech_input_tests;" in source
    assert '#[path = "production_conversation_lineage_ui/speech_input_tests.rs"]' in source
    assert handler.count("append_player_speech_key_text(") == 2
    assert "append_player_speech_key_text(&mut state.input, key_messages.read(), true);" in handler
    assert "append_player_speech_key_text(&mut state.input, key_messages.read(), false);" in handler
    assert """if crate::production_voxel_renderer::menu::captures_input(menu) {
        key_messages.clear();
        return;
    }""" in handler
    assert """if dock.as_ref().is_some_and(|dock| dock.speech_opened) {
        key_messages.clear();
        return;
    }""" in handler
    assert """if !state.input_open && !state.lineage_open && keyboard.get_just_pressed().next().is_none() {
        key_messages.clear();
        return;
    }""" in handler
    assert """if state.lineage_open {
        key_messages.clear();""" in handler
    closed_start = handler.index("    if !state.input_open {")
    closed_end = handler.index("    if keyboard.just_pressed(KeyCode::Escape)", closed_start)
    closed = handler[closed_start:closed_end]
    assert closed.count("return;") == 1
    assert "send_player_speech(" not in closed
    assert closed.rstrip().endswith("key_messages.clear();\n        return;\n    }")
    assert """state.status = "Player speech cancelled".to_string();
        key_messages.clear();
        return;""" in handler
    assert handler.count("send_player_speech(") == 1
    assert "if submit_enter && keyboard.just_pressed(KeyCode::Enter) && !state.input.trim().is_empty() {" in handler
    for name in (
        "opening_frame_keeps_only_text_after_the_first_fresh_enter",
        "an_enter_release_or_repeat_is_not_the_opening_text_boundary",
        "a_missing_raw_opening_enter_discards_the_batch",
        "preopening_text_never_reappears_on_the_next_frame",
        "active_typing_preserves_unicode_printable_repeats_and_the_existing_limit",
        "held_enter_repeats_do_not_retrigger_the_bevy_submit_edge",
    ):
        assert f"fn {name}()" in tests, name
    assert "app.add_plugins(InputPlugin);" in tests
    assert ".reset(KeyCode::Enter);" in tests
    assert "MessageCursor::default()" in tests


@dataclass(frozen=True)
class Event:
    key: str = "C"
    state: str = "pressed"
    text: str = ""
    repeat: bool = False


def enter(repeat=False):
    return Event("Enter", text="\r", repeat=repeat)


def text(value, repeat=False):
    return Event(text=value, repeat=repeat)


def append_reference(input_text, events, opened_this_frame):
    after_opening_enter = not opened_this_frame
    for message in events:
        if message.state != "pressed":
            continue
        if not after_opening_enter:
            after_opening_enter = message.key == "Enter" and not message.repeat
            continue
        for character in message.text:
            # Rust char::is_control(): Unicode general category Cc (not all category C).
            if not (ord(character) <= 0x1F or 0x7F <= ord(character) <= 0x9F):
                if len(input_text) < MAX_TYPED_CHARS:
                    input_text += character
    return input_text


class ReferenceFrames:
    """Model only: same aggregate-key guard and reader boundaries as inspected source."""

    def __init__(self, opened=False, keep_open_on_submit=False):
        self.opened = opened
        self.keep_open_on_submit = keep_open_on_submit
        self.input = ""
        self.pressed = set()
        self.submits = 0
        self.unread = []

    def frame(self, events=(), mode="normal", synthetic_enter=False):
        self.unread.extend(events)
        just_pressed = set()
        for event in events:
            if event.state == "pressed":
                if event.key not in self.pressed:
                    just_pressed.add(event.key)
                self.pressed.add(event.key)
            else:
                self.pressed.discard(event.key)
        if synthetic_enter:
            just_pressed.add("Enter")
        if mode in ("menu", "lineage", "pointer_open"):
            if mode == "pointer_open":
                self.opened, self.input = True, ""
            self.unread.clear()
            return
        if not self.opened:
            if "Enter" in just_pressed:
                self.opened = True
                self.input = append_reference("", self.unread, True)
            self.unread.clear()
            return
        if "Escape" in just_pressed:
            self.opened, self.input = False, ""
            self.unread.clear()
            return
        self.input = append_reference(self.input, self.unread, False)
        fresh_raw_enter = any(event.key == "Enter" and event.state == "pressed" and not event.repeat for event in self.unread)
        self.unread.clear()
        if "Enter" in just_pressed and fresh_raw_enter and self.input.strip():
            self.submits += 1
            if not self.keep_open_on_submit:
                self.opened, self.input = False, ""


class ReferenceModelTests(unittest.TestCase):
    def test_c_then_enter_opens_empty(self):
        f = ReferenceFrames()
        f.frame([text("c"), enter()])
        self.assertTrue(f.opened)
        self.assertEqual(f.input, "")
        f.frame()
        self.assertEqual(f.input, "")

    def test_enter_then_c_preserves_c_without_replay(self):
        f = ReferenceFrames()
        f.frame([enter(), text("c")])
        self.assertEqual(f.input, "c")
        f.frame()
        self.assertEqual(f.input, "c")

    def test_before_and_after_text(self):
        f = ReferenceFrames()
        f.frame([text("before"), enter(), text("é猫")])
        self.assertEqual(f.input, "é猫")

    def test_repeat_and_release_do_not_open_text_boundary(self):
        events = [Event("Enter", "released"), text("before"), enter(True), text("old"), enter(), text("after")]
        self.assertEqual(append_reference("", events, True), "after")

    def test_missing_raw_boundary_is_empty(self):
        f = ReferenceFrames()
        f.frame([text("c"), enter(True)], synthetic_enter=True)
        self.assertEqual(f.input, "")
        self.assertEqual(f.unread, [])

    def test_previous_closed_frame_does_not_leak(self):
        f = ReferenceFrames()
        f.frame([text("c")])
        f.frame([enter()])
        self.assertEqual(f.input, "")

    def test_lineage_batch_is_cleared(self):
        f = ReferenceFrames()
        f.frame([text("old")], mode="lineage")
        f.frame([enter()])
        self.assertEqual(f.input, "")

    def test_menu_batch_is_cleared(self):
        f = ReferenceFrames()
        f.frame([text("old")], mode="menu")
        f.frame([enter()])
        self.assertEqual(f.input, "")

    def test_pointer_open_flushes_whole_keyboard_batch(self):
        f = ReferenceFrames()
        f.frame([enter(), text("c")], mode="pointer_open")
        f.frame()
        self.assertTrue(f.opened)
        self.assertEqual(f.input, "")

    def test_cancel_clears_sameframe_text(self):
        f = ReferenceFrames(opened=True)
        f.frame([text("old"), Event("Escape")])
        f.frame([enter()])
        self.assertEqual(f.input, "")

    def test_active_unicode_and_printable_repeats_are_preserved(self):
        events = [text("é猫\r\n\t\x7f"), Event("C", "released", "ignored"), text("c", True)]
        self.assertEqual(append_reference("", events, False), "é猫c")

    def test_limit_is_512_characters(self):
        value = append_reference("猫" * 511, [text("é猫")], False)
        self.assertEqual(len(value), 512)
        self.assertTrue(value.endswith("é"))

    def test_opening_frame_multiple_enters_never_submit(self):
        f = ReferenceFrames()
        f.frame([enter(), text("c"), enter(), enter(True)])
        f.frame([enter(True)])
        self.assertEqual(f.input, "c")
        self.assertEqual(f.submits, 0)

    def test_repeat_after_held_state_reset_does_not_submit(self):
        f = ReferenceFrames(opened=True, keep_open_on_submit=True)
        f.input = "c"
        f.frame([enter()])
        f.pressed.clear()
        f.frame([enter(True)])
        self.assertEqual(f.submits, 1)

    def test_submit_once_and_no_held_repeat_retry(self):
        f = ReferenceFrames(opened=True, keep_open_on_submit=True)
        f.input = "c"
        f.frame([enter(), enter(True), enter(True)])
        for _ in range(3):
            f.frame([enter(True), enter(True)])
        self.assertEqual(f.submits, 1)
        f.frame([Event("Enter", "released"), enter()])
        self.assertEqual(f.submits, 2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    root = Path(__file__).resolve().parents[1]
    parser.add_argument("--source", type=Path, default=root / "crates/alife_game_app/src/production_conversation_lineage_ui.rs")
    parser.add_argument("--tests", type=Path, default=root / "crates/alife_game_app/src/production_conversation_lineage_ui/speech_input_tests.rs")
    parser.add_argument("--lockfile", type=Path, default=root / "Cargo.lock")
    args = parser.parse_args()
    try:
        source_contract(args.source.read_text(), args.tests.read_text(), args.lockfile.read_text())
    except (AssertionError, ValueError, OSError) as error:
        print(f"Source-contract FAIL: {error}", file=sys.stderr)
        return 1
    print("Static Rust source contracts PASS; these are not Rust runtime tests")
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(ReferenceModelTests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if result.wasSuccessful():
        print(f"Python reference-model cases PASS: {result.testsRun}; Rust/Bevy/live execution remains untested")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
