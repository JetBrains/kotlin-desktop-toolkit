import sys
import traceback

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import GLib, GObject, Gdk, Gtk

class MyApplication(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="org.jetbrains.desktop.linux.tests.TestAppDragTarget")
        GLib.set_application_name("Drag Target Test App")

    def do_activate(self):
        try:
            window = Gtk.ApplicationWindow(application=self, title="Drag Target")
            drop_target = Gtk.DropTarget.new(GObject.TYPE_NONE, Gdk.DragAction.COPY)
            drop_target.set_gtypes([GObject.TYPE_STRING])
            window.add_controller(drop_target)
            window.present()
            print("ready")
            sys.stdout.flush()
        except Exception:
            eprint(traceback.format_exc())
            exit(1)


app = MyApplication()
app.run()
