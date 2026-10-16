#!/usr/bin/env python3
"""Writes benchmark results from scripts/bench-table.sh into BENCHMARKS.md and
README.md: each table, a line saying where and when it was measured, and the
sentences that quote its figures. Only the text between these markers
changes:

  BENCHMARKS.md  <!-- bench:relaxng --> ... <!-- /bench:relaxng -->
                 <!-- bench:schematron --> ... <!-- /bench:schematron -->
  README.md      <!-- bench:readme --> ... <!-- /bench:readme -->

Usage: scripts/bench-docs.py --info TEXT --relaxng TABLE.md [--schematron TABLE.md]
  --info  where and when the tables were measured, one sentence
A table file holds a Markdown table as bench-table.sh prints it. The README
line is only rewritten when both tables are given.
"""
import argparse
import os
import re
import statistics
import sys
import textwrap

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
CELL = re.compile(r"^([\d.]+) ± ([\d.]+) ms$")


def parse(path):
    """The table's lines and, per row, (label, theirs, ours, sd theirs, sd ours,
    speed-up as the table shows it)."""
    lines = [line.rstrip("\n") for line in open(path, encoding="utf-8") if line.startswith("|")]
    rows = []
    for line in lines[2:]:
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        theirs, ours = CELL.match(cells[-3]), CELL.match(cells[-2])
        if not (theirs and ours):
            sys.exit(f"{path}: can't read timings in: {line}")
        rows.append((
            f"{cells[0]}: {cells[1]}" if cells[0] == "papyri.info" else cells[0],
            float(theirs[1]), float(ours[1]), float(theirs[2]), float(ours[2]),
            float(cells[-1].rstrip("×")),
        ))
    if not rows:
        sys.exit(f"{path}: no rows")
    return "\n".join(lines), rows


def listing(names):
    return ", ".join(names[:-1]) + " and " + names[-1] if len(names) > 1 else names[0]


def speed_ups(rows):
    return sorted(((row[5], row[0]) for row in rows), reverse=True)


def faster(speeds, tool):
    low, high = speeds[-1][0], speeds[0][0]
    median = statistics.median(speed for speed, _ in speeds)
    slower = sum(speed < 1 for speed, _ in speeds)
    if slower:
        lead = f"tei-rs is faster than {tool} on {len(speeds) - slower} of {len(speeds)} samples"
    else:
        lead = f"tei-rs is faster than {tool} on every sample"
    return f"{lead}: {low:.1f}–{high:.1f}×, {median:.1f}× at the median."


def relaxng_summary(rows):
    speeds = speed_ups(rows)
    text = faster(speeds, "Jing")
    text += (
        f" The largest speed-ups are for {listing([label for _, label in speeds[:3]])};"
        f" the smallest for {listing([label for _, label in speeds[-3:]])}."
    )
    ours = max(row[4] for row in rows)
    theirs = [row[3] for row in rows]
    if ours < max(theirs):
        text += (
            f" tei-rs's timings vary less from run to run: σ of at most {ours:.1f} ms,"
            f" against Jing's {min(theirs):.1f}–{max(theirs):.0f} ms."
        )
    return text


def schematron_summary(rows, relaxng_rows):
    speeds = speed_ups(rows)
    text = faster(speeds, "the reference pipeline").replace("tei-rs is", "With Schematron, tei-rs is", 1)
    plain = {label: (theirs, ours) for label, theirs, ours, *_ in relaxng_rows}
    ours = {label: rule_ours - plain[label][1] for label, _, rule_ours, *_ in rows if label in plain}
    theirs = {label: rule_theirs - plain[label][0] for label, rule_theirs, *_ in rows if label in plain}
    if ours:
        most = max(ours, key=ours.get)
        text += (
            f" Checking the rules adds a median of {statistics.median(ours.values()):.0f} ms to"
            f" tei-rs (most for {most}: {ours[most]:.0f} ms), against a median of"
            f" {statistics.median(theirs.values()) / 1000:.1f} s for the reference's second JVM"
            " and XSLT run."
        )
    return text


def wrap(text):
    return textwrap.fill(text, width=78, break_on_hyphens=False)


def replace_block(path, name, body):
    text = open(path, encoding="utf-8").read()
    pattern = re.compile(rf"(<!-- bench:{name} -->\n).*?(<!-- /bench:{name} -->)", re.S)
    if not pattern.search(text):
        sys.exit(f"{path}: no <!-- bench:{name} --> block")
    text = pattern.sub(lambda match: match[1] + body.rstrip("\n") + "\n" + match[2], text, count=1)
    open(path, "w", encoding="utf-8").write(text)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--info", required=True)
    parser.add_argument("--relaxng", required=True)
    parser.add_argument("--schematron")
    args = parser.parse_args()
    benchmarks = os.path.join(ROOT, "BENCHMARKS.md")

    table, relaxng_rows = parse(args.relaxng)
    info = wrap(f"_{args.info}_")
    replace_block(benchmarks, "relaxng", f"{info}\n\n{table}\n\n{wrap(relaxng_summary(relaxng_rows))}")
    if not args.schematron:
        return
    table, rows = parse(args.schematron)
    summary = wrap(schematron_summary(rows, relaxng_rows))
    replace_block(benchmarks, "schematron", f"{info}\n\n{table}\n\n{summary}")

    def span(speeds):
        return f"{speeds[-1][0]:.0f}–{speeds[0][0]:.0f}×"

    plain, rules = speed_ups(relaxng_rows), speed_ups(rows)
    if plain[-1][0] < 1 or rules[-1][0] < 1:
        return  # the README's claim would need rewording; leave it to a person
    median = statistics.median(speed for speed, _ in plain)
    replace_block(
        os.path.join(ROOT, "README.md"),
        "readme",
        f"- {span(plain)} faster than Jing end-to-end on every sample ({median:.0f}× at the\n"
        f"  median), and {span(rules)} faster than Jing with SchXslt when checking\n"
        "  Schematron too; see [BENCHMARKS.md](BENCHMARKS.md).",
    )


if __name__ == "__main__":
    main()
