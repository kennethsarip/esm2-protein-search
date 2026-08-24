#!/usr/bin/env python3
"""Retire a completed phase from the planning docs.

Run this the moment a phase is done, before opening the PR. It performs the
three bookkeeping edits that CLAUDE.md section 8.1 requires, so that a stale
instruction never reaches the other three sessions:

  1. Collapses the phase's row in CLAUDE.md section 5.3 to a pointer. The phase
     identifier and its "unblocks" column stay, because the dependency table in
     5.1 and consistency check 8.4 #1 resolve against them.
  2. Deletes the phase's task detail from docs/workstreams/WS-<X>.md and leaves
     a one-line pointer. This is where the token bulk actually is.
  3. Appends the summary to that workstream's section of MEMORY.md.

Usage:
    scripts/phase_done.py B1 "recall@10 1.0 vs brute force, 12 ms p95 on the
        10k dev subset. crates/esm2-search-index/src/brute.rs"
    scripts/phase_done.py B1 "..." --dry-run
"""

from __future__ import annotations

import argparse
import datetime as dt
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

MEMORY_SECTIONS = {
    "A": "## WS-A: Data and embedding",
    "B": "## WS-B: Index core",
    "C": "## WS-C: Search service",
    "D": "## WS-D: Infrastructure and evaluation",
}


class PhaseDoneError(RuntimeError):
    pass


def collapse_claude_row(text: str, phase: str) -> str:
    """Replace the deliverable cell in the section 5.3 row with a pointer."""
    pattern = re.compile(rf"^\| {phase} \| (?!done, see MEMORY)(.+?) \| (.*?) \|$", re.M)
    match = pattern.search(text)
    if not match:
        if re.search(rf"^\| {phase} \| done, see MEMORY", text, re.M):
            raise PhaseDoneError(f"{phase} is already retired in CLAUDE.md 5.3")
        raise PhaseDoneError(f"no row for {phase} in CLAUDE.md section 5.3")
    return pattern.sub(rf"| {phase} | done, see MEMORY.md | \2 |", text, count=1)


def strip_workstream_detail(text: str, phase: str) -> str:
    """Delete the phase block from a WS file, leaving a single pointer line."""
    start = re.compile(rf"^\*\*Phase {phase}[:.].*$", re.M).search(text)
    if not start:
        if re.search(rf"^\*\*Phase {phase}\*\* - done", text, re.M):
            raise PhaseDoneError(f"{phase} detail is already stripped")
        raise PhaseDoneError(f"no '**Phase {phase}: ...**' block in the workstream file")
    nxt = re.compile(r"^\*\*Phase [A-D]\d[:.]", re.M).search(text, start.end())
    end = nxt.start() if nxt else len(text)
    return text[: start.start()] + f"**Phase {phase}** - done, see MEMORY.md.\n\n" + text[end:]


def append_memory_entry(text: str, phase: str, date: str, summary: str) -> str:
    """Append the entry to the workstream's own section, per CLAUDE.md 9.3."""
    header = MEMORY_SECTIONS[phase[0]]
    if header not in text:
        raise PhaseDoneError(f"no '{header}' section in MEMORY.md")
    start = text.index(header) + len(header)
    nxt = text.find("\n## ", start)
    end = nxt if nxt != -1 else len(text)
    body = re.sub(r"^_No entries yet\.(?:[^_]|_(?!\n))*_\n", "", text[start:end].lstrip("\n"), flags=re.M)
    entry = f"- [WS-{phase[0]} / {phase}] {date} - {summary}\n"
    prior = body.rstrip("\n") + "\n" if body.strip() else ""
    return text[:start] + "\n\n" + prior + entry + text[end:]


def wrap(summary: str) -> str:
    """Keep MEMORY.md entries to the one-or-two-line format in CLAUDE.md 9.2."""
    words, lines, cur = summary.split(), [], ""
    for w in words:
        # First line carries the "- [WS-B / B1] 2026-08-24 - " prefix; the rest
        # are indented two spaces. Both budgets target an 80-column file.
        limit = 53 if not lines else 78
        if cur and len(cur) + 1 + len(w) > limit:
            lines.append(cur)
            cur = w
        else:
            cur = f"{cur} {w}".strip()
    lines.append(cur)
    return "\n  ".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser(description="Retire a completed phase from the planning docs.")
    ap.add_argument("phase", help="phase identifier, for example B1")
    ap.add_argument("summary", help="what happened, including the measured number and the path")
    ap.add_argument("--date", default=dt.date.today().isoformat(), help="ISO date, defaults to today")
    ap.add_argument("--dry-run", action="store_true", help="print the edits without writing")
    args = ap.parse_args()

    phase = args.phase.upper()
    if not re.fullmatch(r"[A-D]\d", phase):
        print(f"error: '{args.phase}' is not a phase identifier like B1", file=sys.stderr)
        return 2
    if not re.search(r"\d", args.summary):
        print(
            "error: the summary carries no number. CLAUDE.md 9.2 exists because "
            "'phase works' is worth nothing to a future session.",
            file=sys.stderr,
        )
        return 2

    edits = {
        REPO / "CLAUDE.md": collapse_claude_row,
        REPO / "docs" / "workstreams" / f"WS-{phase[0]}.md": strip_workstream_detail,
    }
    written: dict[Path, str] = {}
    try:
        for path, fn in edits.items():
            written[path] = fn(path.read_text(), phase)
        memory = REPO / "MEMORY.md"
        written[memory] = append_memory_entry(
            memory.read_text(), phase, args.date, wrap(" ".join(args.summary.split()))
        )
    except (PhaseDoneError, FileNotFoundError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    for path, text in written.items():
        rel = path.relative_to(REPO)
        if args.dry_run:
            print(f"would rewrite {rel}")
        else:
            path.write_text(text)
            print(f"rewrote {rel}")

    print(
        f"\n{phase} retired. Still yours to do before the PR:\n"
        "  - Run the consistency check in CLAUDE.md 8.4, all seven items.\n"
        "  - If this phase decided an open question, move it from 7.2 to 7.3 with the reasoning.\n"
        "  - If it produced a number that contradicts an estimate in section 6, strike the estimate.\n"
        "  - If it closed or raised a risk, update 7.1."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
