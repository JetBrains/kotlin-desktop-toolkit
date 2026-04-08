import sys
import traceback

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import GLib, Gdk, Gtk

def eprint(msg):
    print(msg)
    sys.stdout.flush()

# This is done for Wayland compatibility: even though it's enough to get the keyboard focus to set the clipboard,
# GTK uses only key (and pointer) down events:
# https://github.com/GNOME/gtk/blob/5301a91f1c74764facb4d60f40ab8621dd7af198/gdk/wayland/gdkseat-wayland.c#L4602
def on_key_pressed(*_args, **_kwargs):
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

def on_is_active_changed(w: Gtk.Window, *_args, **_kwargs):
    if w.is_active:
        eprint("ready")

class MyApplication(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="org.jetbrains.desktop.linux.tests.TestAppPrimarySelectionSource")
        GLib.set_application_name("Primary Selection Source Test App")

    def do_activate(self):
        try:
            window = Gtk.ApplicationWindow(application=self, title="Clipboard Source")
            window.connect("notify::is-active", on_is_active_changed)

            event_controller_key = Gtk.EventControllerKey.new()
            event_controller_key.connect("key-pressed", on_key_pressed)
            window.add_controller(event_controller_key)

            window.present()
        except Exception:
            eprint(traceback.format_exc())
            exit(1)

app = MyApplication()
app.hold()
app.run()
