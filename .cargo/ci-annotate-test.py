#!/usr/bin/env python3
"""TEMPORARY (CI diagnosis, remove with the runner line in config.toml).

Runs a test binary as cargo asks and, on GitHub Actions only, repeats the failures as
annotations. Elsewhere it simply runs the binary."""
import os
import subprocess
import sys


def esc(s):
    return s.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def main():
    args = sys.argv[1:]
    if not os.environ.get("GITHUB_ACTIONS"):
        os.execvp(args[0], args)
    p = subprocess.Popen(args, stdout=subprocess.PIPE, close_fds=False)
    out = []
    for raw in p.stdout:
        sys.stdout.buffer.write(raw)
        sys.stdout.buffer.flush()
        out.append(raw.decode("utf-8", "replace"))
    code = p.wait()
    if code != 0:
        text = "".join(out)
        at = text.find("\nfailures:\n")
        text = text[at:] if at >= 0 else text[-30000:]
        parts = [text[i:i + 3500] for i in range(0, len(text), 3500)][:10]
        name = os.path.basename(args[0])
        for n, part in enumerate(parts, 1):
            print(f"::error title=test {name} {n} of {len(parts)}::{esc(part)}", flush=True)
    sys.exit(code)


main()
