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
        self.dnd_in_progress = False
        GLib.set_application_name("Drag Source Test App")

    def _on_is_active_changed(self, w: Gtk.Window, *_args, **_kwargs):
        if w.is_active and not self.dnd_in_progress:
            eprint("ready")

    def _on_dnd_finished(self, _drag: Gdk.Drag, *_args, **_kwargs):
        self.dnd_in_progress = False
        eprint("dnd-finished")

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
        self.dnd_in_progress = True
        drag.connect("dnd-finished", self._on_dnd_finished)

    def _on_motion_leave(self, *_args, **_kwargs):
        if self.dnd_in_progress:
            eprint("TestAppDragSource drag begin")


    def do_activate(self):
        try:
            self.window = Gtk.ApplicationWindow(application=self, title="Drag Source")
            self.window.connect("notify::is-active", self._on_is_active_changed)

            click_gesture = Gtk.GestureClick.new()
            click_gesture.connect("pressed", self._pressed)
            self.window.add_controller(click_gesture)

            motion_controller = Gtk.EventControllerMotion.new()
            motion_controller.connect("leave", self._on_motion_leave)
            self.window.add_controller(motion_controller)

            self.window.present()
        except Exception:
            eprint(traceback.format_exc())
            exit(1)


app = MyApplication()
app.run()
