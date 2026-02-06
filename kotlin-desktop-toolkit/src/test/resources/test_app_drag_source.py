import sys
import traceback

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import GLib, Gdk, Gtk

class MyApplication(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="org.jetbrains.desktop.linux.tests.TestAppDragSource")
        GLib.set_application_name("Drag Source Test App")

    def do_activate(self):
        try:
            window = Gtk.ApplicationWindow(application=self, title="Drag Source")
            drag_source = Gtk.DragSource.new()
            drag_source.set_content(Gdk.ContentProvider.new_for_value("Text from TestAppDragSource"))
            drag_source.set_actions(Gdk.DragAction.COPY | Gdk.DragAction.MOVE)
            window.add_controller(drag_source)
            window.present()
            print("ready")
            sys.stdout.flush()
        except Exception:
            eprint(traceback.format_exc())
            exit(1)


app = MyApplication()
app.run()
