import sys
import traceback

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import GLib, Gdk, Gtk

class MyApplication(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="org.jetbrains.desktop.linux.tests.TestAppPrimarySelectionSource")
        GLib.set_application_name("Primary Selection Source Test App")

    def do_activate(self):
        try:
            display = Gdk.DisplayManager.get().get_default_display()
            clipboard = display.get_primary_clipboard()
            clipboard.set_content(Gdk.ContentProvider.new_union([
                Gdk.ContentProvider.new_for_bytes(
                    "text/html", GLib.Bytes.new("<p>Text from <b>TestAppPrimarySelectionSource</b></p>".encode("utf-8"))
                ),
                Gdk.ContentProvider.new_for_bytes(
                    "text/plain;charset=utf-8", GLib.Bytes.new("Text from TestAppPrimarySelectionSource".encode("utf-8"))
                ),
            ]))

            print("ready")
            sys.stdout.flush()
        except Exception:
            eprint(traceback.format_exc())
            exit(1)

app = MyApplication()
app.hold()
app.run()
