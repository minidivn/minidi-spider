#!/usr/bin/env python3
"""Install repository git hooks (commit-msg validation) into .git/hooks/.

Usage:
    python3 scripts/install-git-hooks.py
"""

import os
import shutil
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SOURCE_HOOK = os.path.join(REPO_ROOT, "scripts", "hooks", "commit-msg")
GIT_DIR = os.path.join(REPO_ROOT, ".git")
DEST_HOOK = os.path.join(GIT_DIR, "hooks", "commit-msg")


def main():
    if not os.path.isdir(GIT_DIR):
        print(f"[install-git-hooks] no .git directory at {GIT_DIR}", file=sys.stderr)
        print("  Run this from the repository root (or a clone of it).", file=sys.stderr)
        return 1

    if not os.path.isfile(SOURCE_HOOK):
        print(f"[install-git-hooks] missing hook source: {SOURCE_HOOK}", file=sys.stderr)
        return 1

    hooks_dir = os.path.join(GIT_DIR, "hooks")
    os.makedirs(hooks_dir, exist_ok=True)
    shutil.copy2(SOURCE_HOOK, DEST_HOOK)

    if os.name != "nt":
        os.chmod(DEST_HOOK, 0o755)

    print(f"[install-git-hooks] installed commit-msg hook -> {DEST_HOOK}")
    print("  New commits are validated by scripts/check_commit_msg.py.")
    print("  Remove it with: rm .git/hooks/commit-msg")
    return 0


if __name__ == "__main__":
    sys.exit(main())
