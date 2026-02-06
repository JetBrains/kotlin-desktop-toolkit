import sys
import traceback

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import GLib, Gio, Gdk, Gtk

def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)

class MyApplication(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="org.jetbrains.desktop.linux.tests.TestAppClipboardSource")
        GLib.set_application_name("Clipboard Source Test App")

    def do_activate(self):
        try:
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

            print("ready")
            sys.stdout.flush()
        except Exception:
            eprint(traceback.format_exc())
            exit(1)


app = MyApplication()
app.hold()
app.run()
