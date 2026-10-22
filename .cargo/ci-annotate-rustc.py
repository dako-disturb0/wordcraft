#!/usr/bin/env python3
"""TEMPORARY (CI diagnosis, remove with the rustc-wrapper line in config.toml).

Runs rustc / clippy-driver as cargo asks and, on GitHub Actions only, repeats its errors as
annotations, which can be read through the API when the job logs can't. Elsewhere it simply
runs the compiler."""
import json
import os
import re
import subprocess
import sys

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def esc(s):
    return s.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def main():
    args = sys.argv[1:]
    if not os.environ.get("GITHUB_ACTIONS"):
        os.execvp(args[0], args)
    p = subprocess.Popen(args, stderr=subprocess.PIPE, close_fds=False)
    errors = []
    for raw in p.stderr:
        sys.stderr.buffer.write(raw)
        sys.stderr.buffer.flush()
        if not raw.startswith(b"{"):
            continue
        try:
            d = json.loads(raw)
        except ValueError:
            continue
        if d.get("$message_type") == "diagnostic" and d.get("level") in ("error", "error: internal compiler error"):
            errors.append(ANSI.sub("", d.get("rendered") or d.get("message") or ""))
    code = p.wait()
    if errors:
        name = args[args.index("--crate-name") + 1] if "--crate-name" in args else "?"
        text = "\n".join(errors)
        parts = [text[i:i + 3500] for i in range(0, len(text), 3500)][:10]
        for n, part in enumerate(parts, 1):
            sys.stderr.write(f"::error title=rustc {name} {n} of {len(parts)}::{esc(part)}\n")
        sys.stderr.flush()
    sys.exit(code)


main()
