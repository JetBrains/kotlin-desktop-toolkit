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
    clipboard = display.get_clipboard()
    clipboard.set_content(Gdk.ContentProvider.new_union([
        Gdk.ContentProvider.new_for_bytes(
            "text/html", GLib.Bytes.new("<p>Text from <b>TestAppClipboardSource</b></p>".encode("utf-8"))
        ),
        # Gdk.FileList.new_from_list is supported only from GTK 4.8, so use the raw values instead
        Gdk.ContentProvider.new_for_bytes(
            "text/uri-list", GLib.Bytes.new((
                                                    "file:///some/path/With%20Spaces/&%20$p%E2%82%AC%C2%A2%C3%AF%C3%A5%C5%82%20%C3%A7%C4%A7%C4%81%C5%99%C3%9F\r\n" +
                                                    "file:///tmp/%5BScreenshot%20from%2012:04:42%5D.png\r\n"
                                            ).encode("utf-8"))
        ),
        Gdk.ContentProvider.new_for_bytes(
            "text/plain;charset=utf-8",
            GLib.Bytes.new("/some/path/With Spaces/& $p€¢ïåł çħāřß\n/tmp/[Screenshot from 12:04:42].png".encode("utf-8"))
        ),
    ]))
    eprint("set clipboard content")

def on_is_active_changed(w: Gtk.Window, *_args, **_kwargs):
    if w.is_active:
        eprint("ready")

class MyApplication(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="org.jetbrains.desktop.linux.tests.TestAppClipboardSource")
        GLib.set_application_name("Clipboard Source Test App")

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
app.run()
