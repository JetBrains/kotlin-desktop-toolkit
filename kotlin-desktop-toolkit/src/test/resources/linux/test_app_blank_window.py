import sys
import traceback

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk

def eprint(msg):
    print(msg)
    sys.stdout.flush()

def _motion(*_args):
    eprint(f"Received DRAG_MOTION event")

class MyApplication(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="org.jetbrains.desktop.linux.tests.TestAppBlankWindow")
        GLib.set_application_name("Blank Window Test App")

    def do_activate(self):
        try:
            window = Gtk.ApplicationWindow(application=self, title="Blank Window")

            event_controller = Gtk.DropControllerMotion.new()
            event_controller.connect("motion", _motion)
            window.add_controller(event_controller)

            window.present()
            eprint("ready")
        except Exception:
            eprint(traceback.format_exc())
            exit(1)


app = MyApplication()
app.run()
