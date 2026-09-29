#!/usr/bin/env python3
"""Check every production Rust process-spawn source against the 045 inventory."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / "specs/045-process-supervisor/contracts/process-inventory.md"
COMMAND_IMPORT_ALIAS = re.compile(
    r"use\s+(?:std|tokio)(?:::[\w{} ,]+)?::process::Command\s+as\s+([A-Za-z_][A-Za-z0-9_]*)"
)

EXPECTED_DIRECT_COUNTS = {
    "crates/acp-agent-core/src/infrastructure/acp/runner.rs": 1,
    "crates/acp-agent-core/src/infrastructure/acp/terminal.rs": 1,
    "crates/acp-agent-core/src/infrastructure/agent_catalog.rs": 1,
    "crates/acp-agent-core/src/infrastructure/acp/util.rs": 1,
    "crates/git-core/src/git_cli.rs": 11,
    "crates/workbench-core/src/infrastructure/git/cli_branch_provider.rs": 1,
    "crates/workbench-core/src/infrastructure/git/cli_remote_provider.rs": 1,
    "crates/workbench-core/src/infrastructure/git/cli_worktree_provider.rs": 5,
    "crates/workbench-core/src/infrastructure/git/cli_worktree_change_provider.rs": 3,
    "crates/workbench-core/src/infrastructure/fs/worktree_watcher.rs": 1,
    "crates/workbench-core/src/infrastructure/orchestration/worktree_guard.rs": 1,
    "crates/workbench-host/src/lifecycle/ensure.rs": 1,
    "apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs": 3,
    "apps/agentic-workbench/src-tauri/build.rs": 2,
    "crates/process-supervisor/src/platform/windows/feasibility.rs": 1,
    "apps/git-explorer/src-tauri/src/adapters/outbound/git_cli.rs": 3,
    "apps/git-explorer/src-tauri/src/adapters/outbound/fs_repository_watcher.rs": 1,
    "apps/hushline/src-tauri/src/adapters/system.rs": 7,
    "apps/markdown-annotator/src-tauri/src/cli_launcher.rs": 4,
}

CLASSIFICATION = {
    "crates/acp-agent-core/src/infrastructure/acp/runner.rs": "ServerOwned",
    "crates/acp-agent-core/src/infrastructure/acp/terminal.rs": "ServerOwned",
    "crates/acp-agent-core/src/infrastructure/agent_catalog.rs": "ServerOwned",
    "crates/acp-agent-core/src/infrastructure/acp/util.rs": "ServerOwned",
    "crates/git-core/src/git_cli.rs": "ServerOwned",
    "crates/workbench-core/src/infrastructure/git/cli_branch_provider.rs": "ServerOwned",
    "crates/workbench-core/src/infrastructure/git/cli_remote_provider.rs": "ServerOwned",
    "crates/workbench-core/src/infrastructure/git/cli_worktree_provider.rs": "ServerOwned",
    "crates/workbench-core/src/infrastructure/git/cli_worktree_change_provider.rs": "ServerOwned",
    "crates/workbench-core/src/infrastructure/fs/worktree_watcher.rs": "ServerOwned",
    "crates/workbench-core/src/infrastructure/orchestration/worktree_guard.rs": "ServerOwned",
    "crates/workbench-host/src/lifecycle/ensure.rs": "DaemonBootstrap",
    "apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs": "DesktopNative",
    "apps/agentic-workbench/src-tauri/build.rs": "Build",
    "crates/process-supervisor/src/platform/windows/feasibility.rs": "Fixture",
    "apps/git-explorer/src-tauri/src/adapters/outbound/git_cli.rs": "OtherApp",
    "apps/git-explorer/src-tauri/src/adapters/outbound/fs_repository_watcher.rs": "OtherApp",
    "apps/hushline/src-tauri/src/adapters/system.rs": "OtherApp",
    "apps/markdown-annotator/src-tauri/src/cli_launcher.rs": "OtherApp",
}


def without_cfg_test_items(source: str) -> str:
    """Remove brace-delimited items carrying a direct `#[cfg(test)]` attribute."""
    masked = mask_rust_comments_and_literals(source)
    output: list[str] = []
    cursor = 0
    marker = re.compile(r"(?m)^\s*#\[cfg\(test\)\]\s*$")
    while match := marker.search(source, cursor):
        output.append(source[cursor : match.start()])
        opening = masked.find("{", match.end())
        semicolon = masked.find(";", match.end())
        if semicolon >= 0 and (opening < 0 or semicolon < opening):
            cursor = semicolon + 1
            continue
        if opening < 0:
            cursor = match.end()
            continue
        depth = 0
        closing = opening
        for closing in range(opening, len(source)):
            if source[closing] == "{":
                depth += 1
            elif source[closing] == "}":
                depth -= 1
                if depth == 0:
                    closing += 1
                    break
        cursor = closing
    output.append(source[cursor:])
    return "".join(output)


def mask_rust_comments_and_literals(source: str) -> str:
    """Preserve offsets while hiding delimiters that are not Rust syntax."""
    chars = list(source)
    i = 0
    while i < len(source):
        if source.startswith("//", i):
            end = source.find("\n", i + 2)
            end = len(source) if end < 0 else end
            chars[i:end] = " " * (end - i)
            i = end
        elif source.startswith("/*", i):
            start = i
            depth = 1
            i += 2
            while i < len(source) and depth:
                if source.startswith("/*", i):
                    depth += 1
                    i += 2
                elif source.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
            chars[start:i] = " " * (i - start)
        elif source[i] == '"':
            start = i
            i += 1
            while i < len(source):
                if source[i] == "\\":
                    i += 2
                elif source[i] == '"':
                    i += 1
                    break
                else:
                    i += 1
            chars[start:i] = " " * (i - start)
        else:
            raw = re.match(r"(?:br|r)(?P<hashes>#{0,255})\"", source[i:])
            if raw:
                start = i
                hashes = raw.group("hashes")
                i += raw.end()
                close = source.find('"' + hashes, i)
                i = len(source) if close < 0 else close + 1 + len(hashes)
                chars[start:i] = " " * (i - start)
            else:
                i += 1
    return "".join(chars)


def command_creation_count(source: str) -> int:
    aliases = {"Command", *COMMAND_IMPORT_ALIAS.findall(source)}
    constructor_ends: set[int] = set()
    for alias in aliases:
        for match in re.finditer(rf"\b{re.escape(alias)}::new\s*\(", source):
            constructor_ends.add(match.end())
    for match in re.finditer(
        r"\b(?:std::process|tokio::process|process)::Command::new\s*\(", source
    ):
        constructor_ends.add(match.end())
    return len(constructor_ends)


def discovered_sources() -> dict[str, int]:
    discovered: dict[str, int] = {}
    for base in (ROOT / "crates", ROOT / "apps"):
        for path in base.rglob("*.rs"):
            relative = path.relative_to(ROOT).as_posix()
            if "/tests/" in relative or path.name.endswith("_tests.rs"):
                continue
            count = command_creation_count(
                without_cfg_test_items(path.read_text(encoding="utf-8", errors="replace"))
            )
            if count:
                discovered[relative] = count
    return discovered


def documented_sources() -> set[str]:
    text = INVENTORY.read_text(encoding="utf-8")
    return {
        match.group(1)
        for match in re.finditer(r"^\| `([^`]+\.rs)`", text, flags=re.MULTILINE)
        if "*" not in match.group(1)
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    parser.add_argument(
        "--enforce-supervised",
        action="store_true",
        help="also fail while a ServerOwned source still directly creates a Command",
    )
    args = parser.parse_args()

    discovered_counts = discovered_sources()
    discovered = set(discovered_counts)
    classified = set(CLASSIFICATION)
    documented = documented_sources()
    unknown = sorted(discovered - classified)
    stale = sorted(classified - discovered)
    undocumented = sorted(discovered - documented)
    server_direct = sorted(
        path for path in discovered if CLASSIFICATION.get(path) == "ServerOwned"
    )
    count_drift = {
        path: {"expected": EXPECTED_DIRECT_COUNTS.get(path), "actual": count}
        for path, count in sorted(discovered_counts.items())
        if EXPECTED_DIRECT_COUNTS.get(path) != count
    }
    result = {
        "discovered": sorted(discovered),
        "unknown": unknown,
        "stale": stale,
        "undocumented": undocumented,
        "server_owned_direct": server_direct,
        "direct_counts": discovered_counts,
        "count_drift": count_drift,
        "enforce_supervised": args.enforce_supervised,
    }
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        for key, value in result.items():
            print(f"{key}: {value}")

    failed = bool(unknown or stale or undocumented or count_drift)
    if args.enforce_supervised and server_direct:
        failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
