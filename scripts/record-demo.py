#!/usr/bin/env python3
"""Record a thunderbox program and write an asciinema cast file.

The script starts a program inside a pseudo-terminal, sends a list of keys to
it, and writes what the program draws. The output is a cast file in the
asciinema format, version 2. The `agg` program turns that file into a GIF.

The script needs no recorder program. It writes the cast file itself, because
the format holds a header line and then one line for each piece of output.

Run the script like this:

    python3 scripts/record-demo.py app demo.cast

The first argument names the scene. Each scene names its own program and its
own list of keys. The scenes are these:

    app             The whole program, over the layout of the dashboard that it
                    holds, with the data source testdata.
    panel_embed     The example that draws one panel.
    full_embed      The example that holds a whole dashboard.
    stack_none      Two charts in one column, under no rule.
    stack_column    The same two charts, under the rule `align = "column"`.
    too_small       A terminal that is too small for one panel of its dashboard.
    rows            The row panels, which collapse and expand.
    not_a_terminal  The program with its standard output sent to a file.
    unknown_option  The program with an option that it does not know.
    help            The program with the option `--help`.
    wrong_unit      A dashboard with a panel that states the unit "byte".
    zero_refresh    A dashboard that states `refresh = "0s"`.

A scene can also hold a dashboard of its own. The script writes that dashboard
to a file, and it gives the path of that file to the program.

A scene can hold the key `output` with the value `file`. The script then sends
the standard output of the program to a new temporary file. The program keeps
the pseudo-terminal for its input and for its error output. After the
recording, the script prints the size of that file and deletes the file. The
script returns 1 when the file is not empty, because the picture would then
show a fault of the program.

A scene can hold the key `args`, a list of arguments. The script gives these
arguments to the program before the path of the dashboard. A scene that holds no
`args` key gives the program the path of the dashboard alone.

A scene can hold the key `file_name`. The script then writes the dashboard into
a new temporary directory, under that name. It starts the program inside that
directory, and it gives the program the bare file name. A message of the program
then names that file name, and it names no temporary path. A scene that holds no
`file_name` key gets a temporary file with a random name.

The script accepts these options:

    --cols N       The width of the terminal.
    --rows N       The height of the terminal.
    --program P    A program to run in place of the one that the scene names.
    --no-load      Record the scene without the work that a scene asks for.

No picture shows data of the machine that records it. Each scene therefore reads
the data source testdata, or data that its program makes itself. The script
stops before it records a scene that can read this machine.

A scene can still ask the script to give the machine work to do. See the `load`
key of a scene, and the `start_load` function. No scene asks for it today.
"""

import argparse
import contextlib
import fcntl
import json
import os
import pathlib
import pty
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time

# The dashboard that the two scenes of the stack draw.
#
# The two panels hold the same columns of the grid, so they stand one above the
# other. The first panel shows a share of use, and it writes a label such as
# `84%`. The second panel shows a rate, and it writes a label such as `100B/s`.
# The second chart therefore starts further to the right, and the two axes of
# time start at different cells. The `align` field of the dashboard states the
# rule that answers that. The word `ALIGN` below holds the place of the line
# that states the rule.
STACK_DASHBOARD = """
title = "stack"
refresh = "1s"
ALIGN
[time]
from = "now-30s"
to = "now"

[source.machine]
kind = "testdata"

[[panel]]
title = "cpu"
kind = "timeseries"
unit = "percent"
grid = { x = 0, y = 0, w = 24, h = 8 }
[[panel.target]]
source = "machine"
expr = "random_walk"

[[panel]]
title = "network in"
kind = "timeseries"
unit = "bytes/s"
grid = { x = 0, y = 8, w = 24, h = 8 }
[[panel.target]]
source = "machine"
expr = "sine"
"""


def stack_dashboard(align):
    """Returns the dashboard of a stack scene, under this rule.

    The `align` argument holds the value of the `align` field, or `None` for a
    dashboard that states no rule.
    """
    line = "" if align is None else f'align = "{align}"'
    return STACK_DASHBOARD.replace("ALIGN", line)


# The dashboard of the app scene. It is the dashboard inside the program, with
# the data source testdata in place of sysinfo. The source `cores` holds four
# series, so the panels of the cores show four rows. testdata names each series
# with the label `series`, and sysinfo names each core with the label `core`.
# The query `processes` needs no change, because testdata answers it with a list
# of invented processes in the shape of the list of sysinfo.
# The function `main` checks that the result reads no data of this machine.
DEFAULT_DASHBOARD = (
    pathlib.Path(__file__).resolve().parent.parent
    / "crates" / "thunderbox" / "src" / "default.toml"
).read_text(encoding="utf-8")
APP_DASHBOARD = (
    DEFAULT_DASHBOARD.replace(
        '[source.machine]\nkind = "sysinfo"',
        '[source.machine]\nkind = "testdata"\n\n'
        '[source.cores]\nkind = "testdata"\nseed = 7\nseries = 4',
    )
    .replace('source = "machine"\nexpr = "cpu.cores"', 'source = "cores"\nexpr = "random_walk"')
    .replace('expr = "cpu"', 'expr = "random_walk"')
    .replace('expr = "memory"', 'expr = "sine"')
    .replace('expr = "network.rx"', 'expr = "ramp"')
    .replace('expr = "network.tx"', 'expr = "sine"')
    .replace("{{core}}", "{{series}}")
)

# The dashboard of the variables scene. The repository ships this dashboard, so
# the scene draws the same screen that a person reads.
VARIABLES_DASHBOARD = (
    pathlib.Path(__file__).resolve().parent.parent / "dashboards" / "variables.toml"
).read_text(encoding="utf-8")

# The dashboard of the rows scene. The repository ships this dashboard, so the
# scene draws the same screen that a person reads.
ROWS_DASHBOARD = (
    pathlib.Path(__file__).resolve().parent.parent / "dashboards" / "rows.toml"
).read_text(encoding="utf-8")

# The steps that both scenes of the stack follow. The scenes send no key, so a
# reader compares two pictures that differ in one thing only.
STACK_STEPS = [
    # The window holds 30 seconds. The scene waits a little longer than that,
    # so the store of samples fills the window, and then both charts move.
    (34.0, ""),
    (0.4, "q"),  # Stop the program.
]

# The dashboard of the too_small scene.
#
# The three panels stand one above the other, and the first one holds 4 rows of
# the grid against 12 rows for each of the others. The scene runs on a terminal
# of 19 rows, so the panels hold 18 rows. thunderbox gives 2 cells to the first
# panel, and it draws no panel under 3 cells high. It therefore drops that
# panel, it asks the data source of that panel for nothing, and the status row
# counts the panel that it dropped.
#
# The data source is the test data of this project, so the scene needs no server
# and no work on this machine.
TOO_SMALL_DASHBOARD = """
title = "too small"
refresh = "1s"

[time]
from = "now-1m"
to = "now"

[source.test]
kind = "testdata"
series = 2

[[panel]]
title = "the panel that thunderbox drops"
kind = "timeseries"
unit = "percent"
grid = { x = 0, y = 0, w = 24, h = 4 }
[[panel.target]]
source = "test"
expr = "random_walk"
legend = "series {{series}}"

[[panel]]
title = "cpu"
kind = "timeseries"
unit = "percent"
grid = { x = 0, y = 4, w = 24, h = 12 }
[[panel.target]]
source = "test"
expr = "sine"
legend = "series {{series}}"

[[panel]]
title = "network in"
kind = "timeseries"
unit = "bytes/s"
grid = { x = 0, y = 16, w = 24, h = 12 }
[[panel.target]]
source = "test"
expr = "ramp"
legend = "series {{series}}"
"""


# Each scene names a program, a size of terminal, and a list of steps. Each step
# holds a pause in seconds, and then the keys to send. An empty text means that
# the script only waits, and lets the program draw by itself.
#
# A scene can hold the key `output` with the value `file`. The recorder then
# sends the standard output of the program to a temporary file, and the program
# keeps the pseudo-terminal for its input and for its error output.
#
# A scene can hold the key `args`, a list of arguments. The script gives these
# arguments to the program before the path of the dashboard.
#
# A scene can hold the key `file_name`. The script then writes the dashboard into
# a new temporary directory, under that name. It starts the program inside that
# directory, and it gives the program the bare file name. A message of the
# program then names that file name, and it names no temporary path.
#
# No scene changes the marker. The octant marker needs a font that holds the
# characters of Unicode 16, and few fonts hold them. A recorder that has no such
# font draws a box in place of each octant. Every scene therefore keeps the
# braille marker, which almost every font holds.
SCENES = {
    "app": {
        "program": "target/debug/thunderbox",
        "dashboard": APP_DASHBOARD,
        "cols": 110,
        "rows": 32,
        "steps": [
            # testdata builds the points of the whole window at once, so each
            # chart has a shape after the first answer.
            (1.5, ""),
            # Each pause is short, so the picture stays under its limit of size.
            # The charts move while the selection moves, so a reader still sees
            # the live data.
            (1.0, "\t"),  # Select the stat panel.
            (1.0, "\t"),  # Select the gauge panel.
            (1.0, "\t"),  # Select the bar gauge panel.
            (1.0, "\t"),  # Select the second time series panel.
            (1.0, "\t"),  # Select the table panel.
            (1.0, "\t"),  # Select the top panel, which lists the processes.
            (1.2, "\t"),  # Return to the first panel.
            (1.6, "-"),  # Make the time range twice as wide.
            # Move the range back by half of its width. The status row names the
            # window in the past.
            (2.0, "\x1b[D"),
            (1.5, "0"),  # Return to the range that the dashboard states.
            # Stop the program. The pause shows the range that the key returned.
            (1.0, "q"),
        ],
    },
    "panel_embed": {
        "program": "target/debug/examples/panel_embed",
        "cols": 84,
        "rows": 18,
        "steps": [
            (7.0, ""),  # The program draws its own data, and thunderbox draws it.
            (0.4, "q"),
        ],
    },
    "full_embed": {
        "program": "target/debug/examples/full_embed",
        "cols": 110,
        "rows": 32,
        "steps": [
            (3.0, ""),  # Let the first queries answer.
            (1.6, "\t"),  # thunderbox uses this key, and it moves the selection.
            (1.6, "x"),  # thunderbox does not use this key. The program keeps it.
            (1.6, "\t"),
            (3.0, ""),  # Let the chart move.
            (0.4, "q"),
        ],
    },
    # The two scenes below differ in one line of one dashboard. Record both, and
    # place the two pictures one above the other. A reader then sees what the
    # rule does.
    # This scene reads the dashboard that `dashboards/variables.toml` holds.
    # The testdata data source draws it, so the scene needs no server and no
    # measurement of this machine.
    "variables": {
        "program": "target/debug/thunderbox",
        "dashboard": VARIABLES_DASHBOARD,
        "cols": 100,
        "rows": 26,
        "steps": [
            # The testdata data source builds the points of the whole window at
            # once, so the first chart is full. This scene therefore waits for
            # no store to fill, and it stays short.
            (5.0, ""),
            # Take the choice that follows. Every panel drops the answer that it
            # holds, because that answer belongs to another signal. The panels
            # then draw the signal that the row names.
            (6.0, "]"),
            (6.0, "]"),  # Take the choice that follows again.
            (6.0, ""),  # Let the last choice stand, and let the charts move.
            (0.4, "q"),  # Stop the program.
        ],
    },
    "stack_none": {
        "program": "target/debug/thunderbox",
        "dashboard": stack_dashboard(None),
        "cols": 90,
        "rows": 24,
        "steps": STACK_STEPS,
    },
    "stack_column": {
        "program": "target/debug/thunderbox",
        "dashboard": stack_dashboard("column"),
        "cols": 90,
        "rows": 24,
        "steps": STACK_STEPS,
    },
    # This scene reads the dashboard that `dashboards/rows.toml` holds. The
    # selection starts on the first row panel, and the key Tab moves it. The key
    # Enter collapses the row panel that the selection holds, or expands it.
    "rows": {
        "program": "target/debug/thunderbox",
        "dashboard": ROWS_DASHBOARD,
        "cols": 100,
        "rows": 30,
        "steps": [
            # The row panel `processes` starts collapsed, so its panel is hidden.
            (3.0, ""),
            (0.5, "\t"),  # Select the chart of the processor.
            (0.5, "\t"),  # Select the stat panel.
            (0.5, "\t"),  # Select the row panel `network`.
            # Collapse it. Its chart disappears, and the line below moves up.
            (1.2, "\r"),
            # Select the row panel `processes`. The key skips the hidden chart.
            (2.5, "\t"),
            (1.2, "\r"),  # Expand it. The list of processes appears.
            (3.5, ""),  # Let the list answer and move.
            (0.5, "\x1b[Z"),  # Select the row panel `network` again.
            (1.2, "\r"),  # Expand it. Its chart returns.
            (2.5, ""),
            (0.4, "q"),  # Stop the program.
        ],
    },
    # This scene records a terminal that is too small for one panel of its
    # dashboard. The status row names the count of the panels that thunderbox
    # dropped, and thunderbox asks the data source of such a panel for nothing.
    "too_small": {
        "program": "target/debug/thunderbox",
        "dashboard": TOO_SMALL_DASHBOARD,
        "cols": 90,
        "rows": 19,
        "steps": [
            # The testdata data source builds the points of the whole window at
            # once, so both charts are full from the first frame.
            (6.0, ""),
            (0.4, "q"),  # Stop the program.
        ],
    },
    # This scene sends the standard output of thunderbox to a file. The program
    # loads the dashboard, finds that its standard output is not a terminal, and
    # writes one message to the error output. It draws no dashboard, and it
    # stops by itself. The dashboard is a dashboard that the script already
    # holds, and it reads the data source testdata.
    "not_a_terminal": {
        "program": "target/debug/thunderbox",
        "dashboard": stack_dashboard(None),
        "output": "file",
        "cols": 120,
        "rows": 4,
        "steps": [
            (2.0, ""),  # The program stops by itself.
        ],
    },
    # This scene gives thunderbox an option that it does not know. The program
    # writes one message to the error output, and it stops with the code 1. The
    # dashboard is a dashboard that the script already holds. The program stops
    # before it reads that dashboard.
    "unknown_option": {
        "program": "target/debug/thunderbox",
        "args": ["--colour"],
        "dashboard": stack_dashboard(None),
        "cols": 110,
        "rows": 4,
        "steps": [
            (2.0, ""),  # The program stops by itself.
        ],
    },
    # This scene gives thunderbox the option `--help`. The program writes the
    # help text and stops. The help text holds 37 lines, so the terminal holds
    # 39 rows.
    "help": {
        "program": "target/debug/thunderbox",
        "args": ["--help"],
        "dashboard": stack_dashboard(None),
        "cols": 82,
        "rows": 39,
        "steps": [
            (2.0, ""),  # The program stops by itself.
        ],
    },
    # This scene gives thunderbox a dashboard with a panel that states the unit
    # "byte". No rule knows that unit, so the program writes one message to the
    # error output, and it stops with the code 1. The scene names the file, so
    # the message holds no temporary path. The longest line of the message holds
    # 156 characters, and the message holds 4 lines.
    "wrong_unit": {
        "program": "target/debug/thunderbox",
        "dashboard": stack_dashboard(None).replace('unit = "percent"', 'unit = "byte"'),
        "file_name": "wrong-unit.toml",
        "cols": 158,
        "rows": 6,
        "steps": [
            (2.0, ""),  # The program stops by itself.
        ],
    },
    # This scene gives thunderbox a dashboard with the field `refresh = "0s"`.
    # The program writes one message to the error output, and it stops with the
    # code 1. The scene names the file, so the message holds no temporary path.
    # The longest line of the message holds 157 characters, and the message holds
    # 4 lines.
    "zero_refresh": {
        "program": "target/debug/thunderbox",
        "dashboard": stack_dashboard(None).replace('refresh = "1s"', 'refresh = "0s"'),
        "file_name": "zero-refresh.toml",
        "cols": 158,
        "rows": 6,
        "steps": [
            (2.0, ""),  # The program stops by itself.
        ],
    },
}


# The program that one worker runs. It keeps a share of the processor busy, and
# that share rises and falls. Each worker starts at a different point in the
# wave, so the cores of the machine report different values. A panel that draws
# one bar for each core therefore shows a different bar for each core.
WORKER = """
import math, os, sys, time
index = int(sys.argv[1])
# Hold a block of memory, so that the memory panel reports more than the floor.
block = bytearray(int(sys.argv[2]))
for offset in range(0, len(block), 4096):
    block[offset] = 1
start = time.time()
while True:
    phase = (time.time() - start) / 7.0 + index * 0.31
    share = 0.12 + 0.88 * (0.5 + 0.5 * math.sin(phase * 2 * math.pi))
    slot = 0.10
    busy = slot * share
    end = time.time() + busy
    while time.time() < end:
        pass
    time.sleep(max(0.004, slot - busy))
"""

# The program that moves bytes across the loopback interface of this machine.
# The network panels then report a rate that rises and falls.
TALKER = """
import math, socket, threading, time
server = socket.socket()
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind(("127.0.0.1", 0))
server.listen(1)
def read():
    connection, _ = server.accept()
    while connection.recv(65536):
        pass
threading.Thread(target=read, daemon=True).start()
writer = socket.create_connection(server.getsockname())
chunk = b"x" * 65536
start = time.time()
while True:
    phase = (time.time() - start) / 9.0
    share = 0.15 + 0.85 * (0.5 + 0.5 * math.sin(phase * 2 * math.pi))
    for _ in range(1 + int(share * 12)):
        writer.sendall(chunk)
    time.sleep(0.05)
"""

# The number of bytes that each worker holds. Four workers therefore hold two
# thousand million bytes, which lifts the memory panel off its floor.
BLOCK_BYTES = 500_000_000


def start_load(workers):
    """Starts the programs that give this machine work to do.

    The function returns the processes that it started. The caller stops each
    one with `stop_load`.
    """
    running = [
        subprocess.Popen(
            [sys.executable, "-c", WORKER, str(index), str(BLOCK_BYTES)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        for index in range(workers)
    ]
    running.append(
        subprocess.Popen(
            [sys.executable, "-c", TALKER],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    )
    return running


def stop_load(running):
    """Stops each program that `start_load` started."""
    for process in running:
        process.kill()
    for process in running:
        process.wait()


def record(program, cols, rows, steps, out_path, workers=0, args=(), to_file=False, cwd=None):
    """Runs the program in a pseudo-terminal and writes a cast file.

    The `workers` argument states how many programs give this machine work
    while the recorder runs. A value of 0 records an idle machine.

    The `args` argument holds the arguments of the program. A scene that draws
    a dashboard of its own passes the path of that dashboard here.

    The `to_file` argument sends the standard output of the program to a new
    temporary file. The function then returns the count of events and the size
    of that file in bytes. Otherwise the size is `None`.

    The `cwd` argument names the directory where the program starts. The value
    `None` keeps the directory of the recorder.
    """
    events = []
    load = start_load(workers) if workers else []
    # The machine needs a moment to reach the load before the program starts.
    if load:
        time.sleep(1.5)
    start = time.time()

    # The recorder opens the file before the fork, so that it can read the size
    # of the file afterwards.
    sink = tempfile.NamedTemporaryFile("wb", suffix=".out") if to_file else None

    pid, fd = pty.fork()
    if pid == 0:
        if sink is not None:
            os.dup2(sink.fileno(), 1)
        if cwd is not None:
            os.chdir(cwd)
        os.environ["TERM"] = "xterm-256color"
        os.environ["COLUMNS"] = str(cols)
        os.environ["LINES"] = str(rows)
        os.execv(program, [os.path.basename(program), *args])

    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))

    def drain(seconds):
        """Reads what the program draws for a number of seconds."""
        end = time.time() + seconds
        while time.time() < end:
            ready, _, _ = select.select([fd], [], [], 0.05)
            if not ready:
                continue
            try:
                data = os.read(fd, 65536)
            except OSError:
                return False
            if not data:
                return False
            events.append([round(time.time() - start, 3), "o", data.decode("utf-8", "replace")])
        return True

    for pause, keys in steps:
        if not drain(pause):
            break
        if keys:
            os.write(fd, keys.encode())

    drain(0.5)
    try:
        os.kill(pid, signal.SIGKILL)
        os.waitpid(pid, 0)
    except (ProcessLookupError, ChildProcessError):
        pass
    os.close(fd)
    stop_load(load)

    size = None
    if sink is not None:
        size = os.path.getsize(sink.name)
        sink.close()

    header = {
        "version": 2,
        "width": cols,
        "height": rows,
        "timestamp": int(start),
        "env": {"TERM": "xterm-256color", "SHELL": "/bin/sh"},
    }
    with open(out_path, "w") as handle:
        handle.write(json.dumps(header) + "\n")
        for event in events:
            handle.write(json.dumps(event) + "\n")
    return len(events), size


def machine_fault(name, scene):
    """Returns the reason that a scene can show data of this machine, or `None`.

    The program `thunderbox` with no dashboard reads this machine, and so does
    each dashboard that declares the data source `sysinfo`.
    """
    dashboard = scene.get("dashboard")
    if scene["program"].endswith("/thunderbox") and dashboard is None:
        return f"The scene {name} runs thunderbox with no dashboard, so it reads this machine."
    if dashboard is not None and 'kind = "sysinfo"' in dashboard:
        return f"The dashboard of the scene {name} declares sysinfo, so it reads this machine."
    return None


def main():
    parser = argparse.ArgumentParser(description="Record a thunderbox program.")
    parser.add_argument("scene", choices=sorted(SCENES), help="The scene to record.")
    parser.add_argument("output", help="The cast file to write.")
    parser.add_argument("--cols", type=int, help="The width of the terminal.")
    parser.add_argument("--rows", type=int, help="The height of the terminal.")
    parser.add_argument("--program", help="A program to run in place of the default.")
    parser.add_argument(
        "--no-load",
        action="store_true",
        help="Record the scene without the work that the scene asks for.",
    )
    options = parser.parse_args()

    scene = SCENES[options.scene]
    program = options.program or scene["program"]
    cols = options.cols or scene["cols"]
    rows = options.rows or scene["rows"]

    if not os.path.exists(program):
        print(f"The program {program} is not there. Build it first.", file=sys.stderr)
        return 1
    fault = machine_fault(options.scene, scene)
    if fault:
        print(fault, file=sys.stderr)
        return 1
    workers = 0 if options.no_load else scene.get("load", 0)
    # A scene that holds a dashboard of its own needs a file for it. The
    # program reads a path, and it reads no text on the standard input.
    dashboard = scene.get("dashboard")
    file_name = scene.get("file_name")
    cwd = None
    with contextlib.ExitStack() as stack:
        args = list(scene.get("args", []))
        if dashboard is not None and file_name is not None:
            # The program path is relative to the root of the workspace, and the
            # program starts in another directory. The path must be absolute.
            program = os.path.abspath(program)
            cwd = stack.enter_context(tempfile.TemporaryDirectory())
            (pathlib.Path(cwd) / file_name).write_text(dashboard, encoding="utf-8")
            args.append(file_name)
        elif dashboard is not None:
            handle = stack.enter_context(
                tempfile.NamedTemporaryFile("w", suffix=".toml", delete=True)
            )
            handle.write(dashboard)
            handle.flush()
            args.append(handle.name)
        count, size = record(
            program,
            cols,
            rows,
            scene["steps"],
            options.output,
            workers,
            args,
            scene.get("output") == "file",
            cwd,
        )
    print(f"The script wrote {count} events to {options.output}.")
    if size is not None:
        print(f"The program wrote {size} bytes to its standard output.")
        if size != 0:
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
