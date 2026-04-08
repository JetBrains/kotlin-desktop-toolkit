import sys
import traceback

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk

def eprint(msg):
    print(msg)
    sys.stdout.flush()

def _text_changed(text_buffer: Gtk.TextBuffer):
    start, end = text_buffer.get_bounds()
    text = text_buffer.get_text(start, end, include_hidden_chars=True)
    eprint(text)

def _motion(*_args):
    eprint("Received DRAG_MOTION event")

def on_is_active_changed(w: Gtk.Window, *_args, **_kwargs):
    if w.is_active:
        eprint("ready")

class MyApplication(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="org.jetbrains.desktop.linux.tests.TestAppDropTarget")
        GLib.set_application_name("Drop Target Test App")

    def do_activate(self):
        try:
            window = Gtk.ApplicationWindow(application=self, title="Drop Target")
            window.connect("notify::is-active", on_is_active_changed)

            text_view = Gtk.TextView.new()
            text_view.set_editable(True)

            motion_controller = Gtk.DropControllerMotion.new()
            motion_controller.connect("motion", _motion)
            text_view.add_controller(motion_controller)

            text_buffer = text_view.get_buffer()
            text_buffer.connect("changed", _text_changed)

            window.set_child(text_view)
            window.present()
        except Exception:
            eprint(traceback.format_exc())
            exit(1)


app = MyApplication()
app.run()
