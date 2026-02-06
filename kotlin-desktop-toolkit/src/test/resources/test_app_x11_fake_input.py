import argparse
import sys
from Xlib import X
from Xlib.ext import xtest
from Xlib.display import Display

def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)

def fake_input(operation, value):
    errors = []
    display = Display(":65")
    display.set_error_handler(lambda *args: errors.append(args))

    xtest.fake_input(display, operation, value)
    display.sync()
    if errors:
        raise Exception(errors)

def run():
    parser = argparse.ArgumentParser()
    parser.add_argument('--operation')
    parser.add_argument('--value')
    args = parser.parse_args()

    value = int(args.value)
    operation_str = args.operation

    operation = None
    if operation_str == "KeyPress":
        operation = X.KeyPress
    elif operation_str == "KeyRelease":
        operation = X.KeyRelease
    else:
        eprint("Error: Invalid operation " + operation_str)
        exit(1)

    fake_input(operation, value)

if __name__ == "__main__":
    run()
