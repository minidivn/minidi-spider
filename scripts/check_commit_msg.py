#!/usr/bin/env python3
"""Validate a git commit message against docs/tooling/GIT_BEST_PRACTICES.md.

Usage (as a commit-msg hook):
    check_commit_msg.py <commit-msg-file>

Exit codes: 0 = valid, 1 = invalid, 2 = usage/IO error.
"""

import re
import sys

SUBJECT_MAX = 50
BODY_MAX = 72

TYPE = r"(feat|fix|docs|chore|refactor|test|perf|build|ci|style|revert)"
SUBJECT_RE = re.compile(rf"^{TYPE}(\([a-z0-9._-]+\))?!?:\s+.+$")

# Messages git generates itself and must always allow.
MERGE_RE = re.compile(r"^Merge (branch|tag|remote-tracking branch|pull request)")
REVERT_RE = re.compile(r'^Revert ".*"$')
SQUASH_RE = re.compile(r"^Squashed commit of the following:")


def is_auto_generated(subject):
    return bool(
        MERGE_RE.match(subject) or REVERT_RE.match(subject) or SQUASH_RE.match(subject)
    )


def main():
    if len(sys.argv) != 2:
        print("usage: check_commit_msg.py <commit-msg-file>", file=sys.stderr)
        return 2

    try:
        with open(sys.argv[1], "r", encoding="utf-8") as f:
            raw = f.read()
    except OSError as e:
        print(f"[commit-msg] cannot read message file: {e}", file=sys.stderr)
        return 2

    lines = raw.strip("\n").split("\n")
    subject = lines[0].strip()
    errors = []

    # Subject checks.
    if not subject:
        errors.append("empty commit message")
    elif subject.startswith("\\") or subject.endswith("\\"):
        errors.append(f"subject has backslash artifacts: {subject!r}")
    elif not SUBJECT_RE.match(subject) and not is_auto_generated(subject):
        errors.append(
            f"subject must match '<type>(<scope>): <subject>' — got: {subject!r}"
        )

    if subject and len(subject) > SUBJECT_MAX and not is_auto_generated(subject):
        errors.append(f"subject is {len(subject)} chars (max {SUBJECT_MAX})")

    # Structure: line 2 must be blank when a body follows.
    if len(lines) > 1 and lines[1].strip() != "":
        errors.append("line 2 must be blank (blank line between subject and body)")

    # Body checks.
    for i, line in enumerate(lines[2:], start=3):
        if line.strip() == "":
            continue
        if len(line) > BODY_MAX:
            errors.append(f"body line {i} is {len(line)} chars (max {BODY_MAX})")
        if line != line.rstrip():
            errors.append(f"body line {i} has trailing whitespace")
        if "\t" in line:
            errors.append(f"body line {i} contains a tab")

    if errors:
        print("[commit-msg] commit message rejected:", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        print("  See docs/tooling/GIT_BEST_PRACTICES.md", file=sys.stderr)
        return 1

    print("[commit-msg] commit message OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
