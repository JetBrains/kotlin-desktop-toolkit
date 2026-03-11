import sys
import traceback

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import GLib, Gdk, Gtk

def eprint(msg):
    print(msg)
    sys.stdout.flush()

class MyApplication(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="org.jetbrains.desktop.linux.tests.TestAppDragSource")
        self.window: Gtk.ApplicationWindow | None = None
        GLib.set_application_name("Drag Source Test App")

    def _pressed(self, src: Gtk.GestureClick, _n_press: int, x: float, y: float):
        assert(self.window is not None)
        surface = self.window.get_surface()
        assert(surface is not None)
        device = src.get_current_event_device()
        assert(device is not None)
        content = Gdk.ContentProvider.new_for_value("Text from TestAppDragSource")
        actions = Gdk.DragAction.COPY | Gdk.DragAction.MOVE
        drag = Gdk.Drag.begin(surface, device, content, actions, x, y)
        assert(drag is not None)
        eprint(f"TestAppDragSource drag begin")

    def do_activate(self):
        try:
            self.window = Gtk.ApplicationWindow(application=self, title="Drag Source")
            click_gesture = Gtk.GestureClick.new()
            click_gesture.connect("pressed", self._pressed)
            self.window.add_controller(click_gesture)
            self.window.present()
            eprint("ready")
        except Exception:
            eprint(traceback.format_exc())
            exit(1)


app = MyApplication()
app.run()
