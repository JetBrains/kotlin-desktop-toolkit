#!/usr/bin/env python3

import logging
import os
import gi

gi.require_version("IBus", "1.0")

from gi.repository import IBus, GLib

def first_candidate() -> IBus.Text:
    text = "first"
    ibus_text = IBus.Text.new_from_string(text)
    ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.SINGLE, 0, len(text))
    return ibus_text

def second_candidate() -> IBus.Text:
    text = "second"
    ibus_text = IBus.Text.new_from_string(text)
    return ibus_text

def third_candidate() -> IBus.Text:
    attr_start = 0
    attr_end = 0
    error_str = "❌ error"
    highlighted_error_str = "🖍 highlighted error"
    important_str = "❗important❗"
    ibus_text = IBus.Text.new_from_string(f"{error_str}, {highlighted_error_str}, {important_str}")

    attr_end += len(error_str)
    ibus_text.append_attribute(IBus.AttrType.FOREGROUND, 0xff0000, attr_start, attr_end)
    ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.ERROR, attr_start, attr_end)

    attr_start = attr_end
    attr_end += 2

    ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.SINGLE, attr_start, attr_end)

    attr_start = attr_end
    attr_end += len(highlighted_error_str)

    ibus_text.append_attribute(IBus.AttrType.BACKGROUND, 0xff0000, attr_start, attr_end)
    ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.ERROR, attr_start, attr_end)

    attr_start = attr_end
    attr_end += 2

    ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.SINGLE, attr_start, attr_end)

    attr_start = attr_end
    attr_end += len(important_str)

    ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.DOUBLE, attr_start, attr_end)

    return ibus_text

class SampleEngine(IBus.Engine):
    def __init__(self):
        super().__init__()
        self.preedit_string_list: list[str] = []
        self.dead_key = None
        self.preedit_pos = 0
        self.candidate_pos = -1
        self.surrounding_ibus_text = None
        self.surrounding_cursor_pos = 0
        self.surrounding_anchor_pos = 0
        self.candidates: list[IBus.Text] = [
            first_candidate(),
            second_candidate(),
            third_candidate(),
        ]

    def _update_candidate(self):
        ibus_text = self.candidates[self.candidate_pos]
        self.preedit_pos = 0
        self.dead_key = None

        self.preedit_string_list = list(ibus_text.get_text())
        preedit_len = len(self.preedit_string_list)
        self.update_preedit_text(ibus_text, self.preedit_pos, preedit_len > 0)

    def _cursor_down(self):
        self.candidate_pos += 1
        if self.candidate_pos >= len(self.candidates):
            self.candidate_pos = 0
        self._update_candidate()

    def _cursor_up(self):
        self.candidate_pos -= 1
        if self.candidate_pos < 0:
            self.candidate_pos = len(self.candidates) - 1
        self._update_candidate()

    def _update(self):
        preedit_len = len(self.preedit_string_list)
        visible = preedit_len > 0

        text = ''.join(self.preedit_string_list)
        ibus_text = IBus.Text.new_from_string(text)
        if self.dead_key is not None and self.preedit_pos > 0:
            ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.SINGLE, 0, self.preedit_pos - 1)
            ibus_text.append_attribute(IBus.AttrType.BACKGROUND,0x00ff00, self.preedit_pos - 1, self.preedit_pos)
            ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.LOW, self.preedit_pos - 1, self.preedit_pos)
            if self.preedit_pos < preedit_len:
                ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.SINGLE, self.preedit_pos, preedit_len)
        else:
            ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.SINGLE if self.dead_key is None else IBus.AttrUnderline.LOW, 0, preedit_len)
        logging.debug(f"calling self.update_preedit_text with preedit_pos={self.preedit_pos}, visible={visible}, text={text}")
        self.update_preedit_text(ibus_text, self.preedit_pos, visible)

    def _commit_string(self):
        text = ''.join(self.preedit_string_list)
        logging.debug(f"calling self.commit_text for text={text}")
        self.commit_text(IBus.Text.new_from_string(text))
        self.dead_key = None
        self.preedit_string_list.clear()
        self.preedit_pos = 0
        self.candidate_pos = -1
        self._update()

    def do_process_key_event(self, keyval: int, key_code: int, state: int) -> bool:
        is_press = ((state & IBus.ModifierType.RELEASE_MASK) == 0)
        logging.debug(f"do_process_key_event: keyval={keyval}, key_code={key_code} (xkb {key_code + 8}), is_press={is_press}, state={state}")
        if not is_press:
            return False

        if self.preedit_string_list:
            if keyval in (IBus.Return, IBus.KP_Enter):
                self._commit_string()
                return True
            elif keyval == IBus.Escape:
                self.dead_key = None
                self.preedit_string_list.clear()
                self.preedit_pos = 0
                self.candidate_pos = -1
                self._update()
                return True
            elif keyval == IBus.BackSpace:
                self.dead_key = None
                self.candidate_pos = -1
                if self.preedit_pos > 0:
                    self.preedit_string_list.pop(self.preedit_pos - 1)
                    self.preedit_pos -= 1
                    self._update()
                return True
            elif keyval in (IBus.Left, IBus.KP_Left):
                self.dead_key = None
                self.candidate_pos = -1
                if self.preedit_pos > 0:
                    self.preedit_pos -= 1
                    self._update()
                return True
            elif keyval in (IBus.Right, IBus.KP_Right):
                self.dead_key = None
                self.candidate_pos = -1
                if self.preedit_pos < len(self.preedit_string_list):
                    self.preedit_pos += 1
                    self._update()
                return True
            elif keyval in (IBus.Down, IBus.KP_Down):
                self._cursor_down()
                return True
            elif keyval in (IBus.Up, IBus.KP_Up):
                self._cursor_up()
                return True

        self.candidate_pos = -1

        if state & IBus.ModifierType.MODIFIER_MASK == 0:
            if keyval == IBus.grave:
                self.dead_key = IBus.grave
                self.preedit_string_list.insert(self.preedit_pos, chr(keyval))
                self.preedit_pos += 1
                self._update()
                return True
            elif self.dead_key is not None:
                if keyval == IBus.e:
                    self.preedit_string_list[self.preedit_pos - 1] = 'è'
                    self.dead_key = None
                    if len(self.preedit_string_list) == 1:
                        self._commit_string()
                    else:
                        self._update()
                    return True
        else:
            if state & IBus.ModifierType.MOD1_MASK != 0:
                if ord(' ') <= keyval <= ord('~'):
                    self.dead_key = None
                    self.preedit_string_list.insert(self.preedit_pos, chr(keyval))
                    self.preedit_pos += 1
                    self._update()
                    return True
            elif state & IBus.ModifierType.CONTROL_MASK != 0 and keyval == IBus.u and self.surrounding_ibus_text is not None:
                self._do_uppercase()
                return True

        if keyval not in (
            IBus.Shift_L, IBus.Shift_R, IBus.Shift_Lock, IBus.Caps_Lock, IBus.Control_L, IBus.Control_R, IBus.Alt_L, IBus.Alt_R,
            IBus.Meta_L, IBus.Meta_R, IBus.Num_Lock, IBus.Super_L, IBus.Super_R, IBus.Hyper_L, IBus.Hyper_R, IBus.ISO_Level3_Shift,
            IBus.Mode_switch
        ):
            self.dead_key = None
            self.preedit_string_list.clear()
            self.preedit_pos = 0
            self._update()

        return False

    def _do_uppercase(self):
        ibus_text = self.surrounding_ibus_text
        cursor_pos = self.surrounding_cursor_pos
        anchor_pos = self.surrounding_anchor_pos
        text = ibus_text.get_text()
        logging.debug(f"_do_uppercase: surrounding_text: text={text}, cursor_pos={cursor_pos}, anchor_pos={anchor_pos}")
        if text:
            if cursor_pos == anchor_pos:
                if cursor_pos > 0:
                    # This doesn't properly work with some characters, e.g. "m̀", but it's good enough for testing.
                    substring = text[cursor_pos - 1]
                    logging.debug(f"_do_uppercase: substring={substring}")
                    self.delete_surrounding_text(-1, 1)
                    self.commit_text(IBus.Text.new_from_string(substring.upper()))
            else:
                offset_from_cursor_pos = 0 if cursor_pos < anchor_pos else anchor_pos - cursor_pos
                text_start_offset = min(cursor_pos, anchor_pos)
                nchars = abs(cursor_pos - anchor_pos)
                substring = text[text_start_offset: text_start_offset + nchars]
                logging.debug(
                    f"_do_uppercase: offset_from_cursor_pos={offset_from_cursor_pos}, nchars={nchars}, text_start_offset={text_start_offset}, substring={substring}")
                self.delete_surrounding_text(offset_from_cursor_pos, nchars)
                self.commit_text(IBus.Text.new_from_string(substring.upper()))

    def do_cursor_down(self) -> None:
        logging.debug("do_cursor_down")

    def do_cursor_up(self) -> None:
        logging.debug("do_cursor_up")

    def do_disable(self) -> None:
        logging.debug("do_disable")
        IBus.Engine.do_disable(self)

    def do_enable(self) -> None:
        logging.debug("do_enable")
        IBus.Engine.do_enable(self)
        # self.get_surrounding_text()

    def do_focus_in(self) -> None:
        logging.debug("do_focus_in")
        IBus.Engine.do_focus_in(self)
        self.get_surrounding_text()

    def do_focus_in_id(self, object_path: str, client: str) -> None:
        logging.debug(f"do_focus_in_id, object_path={object_path}, client={client}")

    def do_focus_out(self) -> None:
        logging.debug("do_focus_out")
        self.preedit_string_list.clear()
        self.dead_key = None
        self.preedit_pos = 0
        self.candidate_pos = -1
        IBus.Engine.do_focus_out(self)

    def do_focus_out_id(self, object_path: str) -> None:
        logging.debug(f"do_focus_out_id, object_path={object_path}")

    def do_page_down(self) -> None:
        logging.debug("do_page_down")

    def do_page_up(self) -> None:
        logging.debug("do_page_up")

    def do_property_activate(self, prop_name: str, prop_state: int) -> None:
        logging.debug("do_property_activate" + prop_name + " state: " + str(prop_state))

    def do_property_hide(self, prop_name: str) -> None:
        logging.debug("do_property_hide: " + prop_name)

    def do_property_show(self, prop_name: str) -> None:
        logging.debug("do_property_show: " + prop_name)

    def do_reset(self) -> None:
        logging.debug("do_reset")
        self.preedit_string_list.clear()
        self.dead_key = None
        self.candidate_pos = -1
        self.preedit_pos = 0
        self.surrounding_ibus_text = None
        self.surrounding_cursor_pos = 0
        self.surrounding_anchor_pos = 0

    def do_set_capabilities(self, caps: int) -> None:
        caps_list = []
        if caps & IBus.Capabilite.PREEDIT_TEXT:
            caps_list.append("PREEDIT_TEXT")
        if caps & IBus.Capabilite.AUXILIARY_TEXT:
            caps_list.append("AUXILIARY_TEXT")
        if caps & IBus.Capabilite.LOOKUP_TABLE:
            caps_list.append("LOOKUP_TABLE")
        if caps & IBus.Capabilite.FOCUS:
            caps_list.append("FOCUS")
        if caps & IBus.Capabilite.PROPERTY:
            caps_list.append("PROPERTY")
        if caps & IBus.Capabilite.SURROUNDING_TEXT:
            caps_list.append("SURROUNDING_TEXT")

        log_out = f"do_set_capabilities: {caps_list}"
        logging.debug(log_out)
        out_file_path = os.environ.get('TEST_IBUS_ENGINE_CAPS_OUT_FILE', None)
        if out_file_path is not None:
            with open(out_file_path, "a") as f:
                f.write(log_out + "\n")

    def do_set_content_type(self, purpose: int, hints: int) -> None:
        purpose_str = ""
        if purpose == IBus.InputPurpose.ALPHA:
            purpose_str = "ALPHA"
        elif purpose == IBus.InputPurpose.DIGITS:
            purpose_str = "DIGITS"
        elif purpose == IBus.InputPurpose.EMAIL:
            purpose_str = "EMAIL"
        elif purpose == IBus.InputPurpose.FREE_FORM:
            purpose_str = "FREE_FORM"
        elif purpose == IBus.InputPurpose.NAME:
            purpose_str = "NAME"
        elif purpose == IBus.InputPurpose.NUMBER:
            purpose_str = "NUMBER"
        elif purpose == IBus.InputPurpose.PASSWORD:
            purpose_str = "PASSWORD"
        elif purpose == IBus.InputPurpose.PHONE:
            purpose_str = "PHONE"
        elif purpose == IBus.InputPurpose.PIN:
            purpose_str = "PIN"
        elif purpose == IBus.InputPurpose.TERMINAL:
            purpose_str = "TERMINAL"
        elif purpose == IBus.InputPurpose.URL:
            purpose_str = "URL"

        hints_list = []
        if hints & IBus.InputHints.SPELLCHECK:
            hints_list.append("SPELLCHECK")
        if hints & IBus.InputHints.NO_SPELLCHECK:
            hints_list.append("NO_SPELLCHECK")
        if hints & IBus.InputHints.WORD_COMPLETION:
            hints_list.append("WORD_COMPLETION")
        if hints & IBus.InputHints.LOWERCASE:
            hints_list.append("LOWERCASE")
        if hints & IBus.InputHints.UPPERCASE_CHARS:
            hints_list.append("UPPERCASE_CHARS")
        if hints & IBus.InputHints.UPPERCASE_WORDS:
            hints_list.append("UPPERCASE_WORDS")
        if hints & IBus.InputHints.UPPERCASE_SENTENCES:
            hints_list.append("UPPERCASE_SENTENCES")
        if hints & IBus.InputHints.INHIBIT_OSK:
            hints_list.append("INHIBIT_OSK")
        if hints & IBus.InputHints.VERTICAL_WRITING:
            hints_list.append("VERTICAL_WRITING")
        if hints & IBus.InputHints.EMOJI:
            hints_list.append("EMOJI")
        if hints & IBus.InputHints.NO_EMOJI:
            hints_list.append("NO_EMOJI")
        if hints & IBus.InputHints.PRIVATE:
            hints_list.append("PRIVATE")

        log_out = f"do_set_content_type: purpose = {purpose_str}, hints = {hints_list}"
        logging.debug(log_out)
        out_file_path = os.environ.get('TEST_IBUS_ENGINE_CONTENT_TYPE_OUT_FILE', None)
        if out_file_path is not None:
            with open(out_file_path, "a") as f:
                f.write(log_out + "\n")

    def do_set_cursor_location(self, x: int, y: int, w: int, h: int) -> None:
        log_out = f"do_set_cursor_location: x={x}, y={y}, w={w}, h={h}"
        out_file_path = os.environ.get('TEST_IBUS_ENGINE_CURSOR_LOCATION_OUT_FILE', None)
        if out_file_path is not None:
            logging.debug(log_out)
            with open(out_file_path, "a") as f:
                f.write(log_out + "\n")

    def do_set_surrounding_text(self, ibus_text: IBus.Text, cursor_pos: int, anchor_pos: int) -> None:
        self.surrounding_ibus_text = ibus_text
        self.surrounding_cursor_pos = cursor_pos
        self.surrounding_anchor_pos = anchor_pos
        text_str = ibus_text.get_text()
        logging.debug(f"do_set_surrounding_text: text={text_str}, cursor_pos={cursor_pos}, anchor_pos={anchor_pos}")
        # We need to chain this function if we want get_surrounding_text to work
        IBus.Engine.do_set_surrounding_text(self, ibus_text, cursor_pos, anchor_pos)


def main():
    logging.basicConfig(format='[%(asctime)s.%(msecs)03d %(levelname)s jb_kdt_ibus_test_engine] %(message)s', level=logging.DEBUG, datefmt='%Y%m%d %H:%M:%S')
    IBus.init()
    bus = IBus.Bus.new()
    c = bus.get_connection()
    assert c is not None, os.environ
    factory = IBus.Factory.new(c)
    factory.add_engine("jb_kdt_ibus_test_engine", SampleEngine)

    component_name = "com.jetbrains.kdt.IBusTestEngine"
    component = IBus.Component(
        name=component_name,
        description="An IBus engine for KDT testing",
        version="0.1.0",
        license="Proprietary",
        author="JetBrains",
        homepage="https://www.jetbrains.com/",
        textdomain="jb-kdt-ibus-test-engine"
    )
    component.add_engine(
        IBus.EngineDesc(
            name="jb_kdt_ibus_test_engine",
            longname="JetBrains KDT IBus test engine",
            description="An IBus engine for KDT testing",
            language="en",
            license="Proprietary",
            author="JetBrains",
            layout="us",
        )
    )
    bus.register_component(component)

    main_loop = GLib.MainLoop()
    main_loop.run()
    factory.do_destroy()


if __name__ == "__main__":
    main()
