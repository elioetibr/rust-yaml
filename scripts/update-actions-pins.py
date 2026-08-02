#!/usr/bin/env -S uv run --script

# SPDX-FileCopyrightText: Rust Yaml contributors
#
# SPDX-License-Identifier: MIT OR Apache-2.0

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "httpx>=0.28.1,<1.0.0",
# ]
# ///

"""Update SHA pins for GitHub Actions to the latest release.

Scans every ``.github/workflows/*.yml`` and ``.github/actions/**/action.yml``
for ``uses:`` references to external actions and pins them to the commit SHA
of the latest public release.  Only updates within the same major version
(no automatic breaking-change upgrades): if the currently pinned version is
``v3.x`` and the latest is ``v4.y``, the script *reports* the available major
bump but does **not** rewrite it.  Branch-based refs (``@stable``,
``@nightly``) are resolved to the branch-tip SHA every time.

Uses async HTTP (httpx) and parallel file I/O for performance.

Requires ``GITHUB_TOKEN`` (or ``GH_TOKEN``) env var for authenticated API
requests --- unauthenticated clients are rate-limited to 60 requests/hour,
which is quickly exhausted by a multi-repo scan.

Usage
-----
    python3 scripts/update-actions-pins.py                        # check-only (dry-run)
    python3 scripts/update-actions-pins.py --write                # write changes
    python3 scripts/update-actions-pins.py -v                     # debug output
    python3 scripts/update-actions-pins.py -q                     # warnings / errors only
    python3 scripts/update-actions-pins.py --write --apply-major
    python3 scripts/update-actions-pins.py --config my-pins.toml  # custom config
    python3 scripts/update-actions-pins.py --print-config         # show effective config

Exit code
---------
0 if everything is up-to-date (or changes written).
1 if there are stale pins (dry-run), write failed, or API error.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import re
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import httpx

REPO_ROOT = Path(__file__).resolve().parent.parent

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

LOG_ERROR = 40
LOG_WARN = 30
LOG_INFO = 20
LOG_DEBUG = 10
_DEFAULT_LOG_LEVEL = LOG_INFO


@dataclass
class Logger:
    """Minimal leveled logger.  Use module-level ``log`` singleton."""

    level: int = field(default=_DEFAULT_LOG_LEVEL)
    _name: str = field(default="pins")

    def error(self, msg: str) -> None:
        if self.level <= LOG_ERROR:
            print(f"ERROR  {msg}", file=sys.stderr)

    def warn(self, msg: str) -> None:
        if self.level <= LOG_WARN:
            print(f"WARN {msg}", file=sys.stderr)

    def info(self, msg: str) -> None:
        if self.level <= LOG_INFO:
            print(msg)

    def debug(self, msg: str) -> None:
        if self.level <= LOG_DEBUG:
            print(f"DEBUG  {msg}", file=sys.stderr)


log: Logger = Logger()

# ---------------------------------------------------------------------------
# Config file loader
# ---------------------------------------------------------------------------

_CONFIG_DIRS = [
    REPO_ROOT / ".github",
    REPO_ROOT,
    ]

_CONFIG_FILES = ["action-pins.toml", "action-pins.yaml", "action-pins.yml", "action-pins.json"]

_DEFAULT_CONFIG_RAW: dict[str, Any] = {
    # Empty by design: the toolchain action is elioseverojunior/rust-toolchain,
    # which ships release tags. dtolnay/rust-toolchain used to live here because
    # it publishes no releases at all and is referenced by branch (@stable,
    # @nightly) instead.
    "branch_only_repos": {},
    "branch_overrides": {"nightly": "nightly"},
    "ref_overrides": {"github/codeql-action": "v4"},
    # Repos that MUST stay pinned to a tag, never a SHA. The SLSA generator
    # verifies its own provenance against its release tag and refuses anything
    # else: "Invalid ref: <sha>. Expected ref of the form refs/tags/vX.Y.Z". A SHA
    # pin there means no provenance at all, so the tag is the correct pin and this
    # script must not "fix" it.
    #
    # This replaces a `reusable_paths` key that was loaded, printed by
    # --print-config, and consulted by nothing.
    "tag_only_repos": [
        "slsa-framework/slsa-github-generator",
    ],
    "max_concurrent_api": 10,
}


def _try_load_toml(path: Path) -> dict | None:
    """Load TOML via ``tomllib`` (3.11+) or ``tomli``."""
    for mod_name in ("tomllib", "tomli"):
        try:
            mod = __import__(mod_name)
            with path.open("rb") as f:
                return mod.load(f)
        except ImportError:
            continue
        except Exception:
            log.debug(f"failed to parse {path}")
            return None
    return None


def _try_load_yaml(path: Path) -> dict | None:
    """Load YAML via ``yaml`` (pyyaml)."""
    try:
        import yaml  # type: ignore[import-untyped]  # noqa: PLC0415

        with path.open() as f:
            return yaml.safe_load(f)
    except ImportError:
        return None
    except Exception:
        log.debug(f"failed to parse {path}")
        return None


def _try_load_json(path: Path) -> dict | None:
    try:
        with path.open() as f:
            return json.load(f)
    except Exception:
        log.debug(f"failed to parse {path}")
        return None


_LOADERS = {
    ".toml": _try_load_toml,
    ".yaml": _try_load_yaml,
    ".yml": _try_load_yaml,
    ".json": _try_load_json,
}


def _find_config_file() -> Path | None:
    for d in _CONFIG_DIRS:
        for name in _CONFIG_FILES:
            p = d / name
            if p.is_file():
                return p
    return None


def _merge_config(defaults: dict, overrides: dict) -> dict:
    """Shallow merge --- user values replace defaults per key."""
    merged = dict(defaults)
    merged.update(overrides)
    return merged


@dataclass
class PinsConfig:
    """Effective configuration for the action-pins script."""

    branch_only_repos: dict[str, list[str]]
    branch_overrides: dict[str, str]
    ref_overrides: dict[str, str]
    tag_only_repos: list[str]
    max_concurrent_api: int


def load_config(path_hint: str | None = None) -> PinsConfig:
    """Load config from *path_hint* or auto-detect, merged with defaults."""
    raw = dict(_DEFAULT_CONFIG_RAW)
    source: str | None = None

    if path_hint:
        p = Path(path_hint).resolve()
        if not p.is_file():
            log.warn(f"config file not found: {p}")
        else:
            loader = _LOADERS.get(p.suffix)
            if loader:
                overrides = loader(p)
                if overrides is not None:
                    raw = _merge_config(raw, overrides)
                    source = str(p)
    else:
        p = _find_config_file()
        if p is not None:
            loader = _LOADERS.get(p.suffix)
            if loader:
                overrides = loader(p)
                if overrides is not None:
                    raw = _merge_config(raw, overrides)
                    source = str(p)

    cfg = PinsConfig(
        branch_only_repos=raw["branch_only_repos"],
        branch_overrides=raw["branch_overrides"],
        ref_overrides=raw["ref_overrides"],
        tag_only_repos=raw["tag_only_repos"],
        max_concurrent_api=raw["max_concurrent_api"],
    )

    if source:
        log.info(f"config: {source}")
    else:
        log.debug("no config file found, using defaults")

    return cfg


CFG: PinsConfig | None = None  # set in main() after arg parsing

# ---------------------------------------------------------------------------
# Data
# ---------------------------------------------------------------------------

@dataclass
class ActionRef:
    """A single parsed ``uses:`` line."""

    file: Path
    line_index: int
    raw: str
    repo: str
    path_suffix: str
    ref_type: str       # "tag", "branch", or "sha"
    current_ref: str    # value after @
    major: Optional[int] = None

    @property
    def display(self) -> str:
        s = self.repo
        if self.path_suffix:
            s = f"{s}/{self.path_suffix}"
        return s


def parse_uses(line: str, file: Path, line_index: int) -> Optional[ActionRef]:
    m = re.search(r'uses:\s*["\']?([^"\'#\s]+)', line)
    if not m:
        return None
    value = m.group(1)
    if value.startswith("./"):
        return None
    # Skip JavaScript template literals (e.g. inside actions/github-script steps).
    if "${" in value:
        return None
    if "@" not in value:
        return None
    prefix, ref = value.rsplit("@", 1)

    parts = prefix.split("/", 2)
    if len(parts) == 2:
        repo, path_suffix = f"{parts[0]}/{parts[1]}", ""
    elif len(parts) == 3 and prefix.count("/") > 1:
        repo, path_suffix = f"{parts[0]}/{parts[1]}", parts[2]
    else:
        return None

    if re.match(r"^[0-9a-f]{40}$", ref):
        return ActionRef(file, line_index, value, repo, path_suffix, "sha", ref)

    if ref.startswith("v") and ref[1:].replace(".", "").isdigit():
        major = int(ref[1:].split(".")[0]) if "." in ref else int(ref[1:])
        return ActionRef(file, line_index, value, repo, path_suffix, "tag", ref, major)

    return ActionRef(file, line_index, value, repo, path_suffix, "branch", ref)


async def collect_refs_async(root: Path) -> List[ActionRef]:
    """Parallel scan of all workflow/action YAML files."""
    files: List[Path] = []
    for dirpath in [root / ".github" / "workflows", root / ".github" / "actions"]:
        if not dirpath.is_dir():
            continue
        for f in sorted(dirpath.rglob("*.yml")):
            if f.is_file():
                files.append(f)

    if not files:
        return []

    log.debug(f"scanning {len(files)} YAML file(s)")

    # Read all files concurrently via thread pool.
    contents = await asyncio.gather(
        *[asyncio.to_thread(f.read_text) for f in files],
    )

    refs: List[ActionRef] = []
    for f, content in zip(files, contents):
        for i, line in enumerate(content.splitlines()):
            ar = parse_uses(line, f, i)
            if ar is not None:
                refs.append(ar)

    log.debug(f"found {len(refs)} external action reference(s)")
    return refs


# ---------------------------------------------------------------------------
# GitHub API (async, authenticated, rate-limited)
# ---------------------------------------------------------------------------

GH_API = "https://api.github.com"


def _gh_headers() -> dict:
    h = {"Accept": "application/vnd.github+json"}
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN") or ""
    if token:
        h["Authorization"] = f"Bearer {token}"
    return h


# Global async cache for resolved SHAs.
_sha_cache: Dict[str, str] = {}

# Semaphore to limit concurrent API requests (lazy, created in main_async).
_api_sem: asyncio.Semaphore | None = None


async def _gh_get(client: httpx.AsyncClient, url: str) -> dict:
    log.debug(f"GET {url}")
    sem = _api_sem if _api_sem is not None else asyncio.Semaphore(CFG.max_concurrent_api)
    for attempt in range(3):
        async with sem:
            try:
                resp = await client.get(url, timeout=30)
            except httpx.TimeoutException:
                if attempt < 2:
                    await asyncio.sleep(2**attempt)
                    continue
                raise RuntimeError(f"Timeout after retries: {url}")

        # Check rate limit before error handling.
        remaining = resp.headers.get("X-RateLimit-Remaining")
        if remaining is not None and int(remaining) == 0:
            reset = int(resp.headers.get("X-RateLimit-Reset", "0"))
            wait = max(reset - time.time() + 2, 0)
            log.warn(f"rate limit exhausted, sleeping {wait:.0f}s...")
            await asyncio.sleep(wait)
            continue

        try:
            resp.raise_for_status()
        except httpx.HTTPStatusError as exc:
            if exc.response.status_code == 403 and attempt < 2:
                await asyncio.sleep(2**attempt)
                continue
            body = exc.response.text[:500]
            raise RuntimeError(f"HTTP {exc.response.status_code}: {body}") from exc

        return resp.json()

    raise RuntimeError("max retries exceeded")


async def resolve_tag_sha(client: httpx.AsyncClient, repo: str, tag: str) -> str:
    key = f"{repo}@tag:{tag}"
    if key in _sha_cache:
        log.debug(f"cache hit: {key}")
        return _sha_cache[key]
    log.debug(f"fetch tag ref: {repo} @ {tag}")
    ref_data = await _gh_get(client, f"{GH_API}/repos/{repo}/git/ref/tags/{tag}")
    obj = ref_data["object"]
    if obj["type"] == "commit":
        _sha_cache[key] = obj["sha"]
        return obj["sha"]
    log.debug(f"tag points to annotated tag object, resolving...")
    tag_data = await _gh_get(client, f"{GH_API}/repos/{repo}/git/tags/{obj['sha']}")
    _sha_cache[key] = tag_data["object"]["sha"]
    return tag_data["object"]["sha"]


async def resolve_branch_sha(client: httpx.AsyncClient, repo: str, branch: str) -> str:
    key = f"{repo}@branch:{branch}"
    if key in _sha_cache:
        log.debug(f"cache hit: {key}")
        return _sha_cache[key]
    log.debug(f"fetch branch ref: {repo} @ {branch}")
    ref_data = await _gh_get(client, f"{GH_API}/repos/{repo}/git/ref/heads/{branch}")
    _sha_cache[key] = ref_data["object"]["sha"]
    return ref_data["object"]["sha"]


async def latest_release(client: httpx.AsyncClient, repo: str) -> Tuple[str, str]:
    """Return (tag_name, commit_sha) for the latest public release."""
    try:
        release = await _gh_get(client, f"{GH_API}/repos/{repo}/releases/latest")
        tag = release["tag_name"]
    except RuntimeError:
        releases = await _gh_get(client, f"{GH_API}/repos/{repo}/releases?per_page=1")
        if not releases:
            raise RuntimeError("No releases found")
        tag = releases[0]["tag_name"]
    sha = await resolve_tag_sha(client, repo, tag)
    return tag, sha


# ---------------------------------------------------------------------------
# Per-ref logic
# ---------------------------------------------------------------------------


@dataclass
class PinResult:
    ref: ActionRef
    status: str            # "uptodate", "updated", "major-bump", "error"
    new_sha: Optional[str] = None
    tag: Optional[str] = None   # display tag for the comment, e.g. "v7.0.0"
    message: str = ""
    # True when the ref must be written as `@<tag>` rather than `@<sha> # <tag>`.
    pin_as_tag: bool = False


def _updated(ar: ActionRef, sha: str, tag: str) -> PinResult:
    return PinResult(
        ar,
        "updated",
        new_sha=sha,
        tag=tag,
        message=f"{tag} ({sha[:12]})",
    )


async def _resolve_tag(client: httpx.AsyncClient, ar: ActionRef) -> Tuple[str, str]:
    """Return (display_tag, commit_sha) for *ar*'s repo at its current best ref."""
    # Branch-only repo --- display just the branch name (the repo is in the uses:).
    if ar.repo in CFG.branch_only_repos:
        for branch in CFG.branch_only_repos[ar.repo]:
            sha = await resolve_branch_sha(client, ar.repo, branch)
            if sha == ar.current_ref:
                return branch, sha
        # No match -> default to first branch.
        default = CFG.branch_only_repos[ar.repo][0]
        sha = await resolve_branch_sha(client, ar.repo, default)
        return default, sha

    # Override ref (e.g. codeql-action @v3).
    if ar.repo in CFG.ref_overrides:
        ref = CFG.ref_overrides[ar.repo]
        sha = await resolve_tag_sha(client, ar.repo, ref)
        return ref, sha

    # Normal versioned action.
    tag, sha = await latest_release(client, ar.repo)
    return tag, sha


def _is_tag_only(ar: ActionRef) -> bool:
    return ar.repo in CFG.tag_only_repos


async def check_ref(client: httpx.AsyncClient, ar: ActionRef) -> PinResult:
    try:
        tag, sha = await _resolve_tag(client, ar)

        # Tag-only: compare against the tag NAME and rewrite to the tag, so a
        # correctly tag-pinned ref is never reported stale and never SHA-pinned.
        if _is_tag_only(ar):
            if ar.ref_type == "tag" and ar.current_ref == tag:
                return PinResult(ar, "uptodate", tag=tag,
                                 message=f"already at {tag} (tag-pinned)")
            return PinResult(ar, "updated", new_sha=None, tag=tag, pin_as_tag=True,
                             message=f"{tag} (tag-pinned, not SHA)")

        if ar.ref_type == "branch":
            if sha == ar.current_ref:
                return PinResult(ar, "uptodate", tag=tag, message=f"already at {tag}")
            return _updated(ar, sha, tag)

        if ar.ref_type == "sha":
            if sha == ar.current_ref:
                return PinResult(ar, "uptodate", tag=tag, message=f"already at {tag}")
            latest_major = _major_of(tag)
            if latest_major is not None and ar.major is not None and latest_major > ar.major:
                return PinResult(
                    ar,
                    "major-bump",
                    new_sha=sha,
                    tag=tag,
                    message=f"major bump: v{ar.major} -> v{latest_major} ({tag})",
                )
            return _updated(ar, sha, tag)

        # Tag ref.
        if sha == ar.current_ref:
            return PinResult(ar, "uptodate", tag=tag, message=f"already at {tag}")
        if ar.major is not None:
            latest_major = _major_of(tag)
            if latest_major is not None and latest_major > ar.major:
                return PinResult(
                    ar,
                    "major-bump",
                    new_sha=sha,
                    tag=tag,
                    message=f"major bump: v{ar.major} -> v{latest_major} ({tag})",
                )
        return _updated(ar, sha, tag)

    except RuntimeError as exc:
        return PinResult(ar, "error", message=str(exc))


def _major_of(tag: str) -> Optional[int]:
    m = re.match(r"^v?(\d+)", tag)
    return int(m.group(1)) if m else None


# ---------------------------------------------------------------------------
# File rewriting (async via thread pool)
# ---------------------------------------------------------------------------

_RE_USES = re.compile(r'(uses:\s*["\']?[^"\'#\s]+)@([^\s"\'#]+)')
_RE_TRAILING = re.compile(r"\s+#\s*\S.*$")  # strip existing trailing comment


def _uses_line(prefix: str, sha: str, tag: Optional[str]) -> str:
    """Build a ``uses: owner/repo@sha # tag`` line."""
    line = f"uses: {prefix}@{sha}"
    if tag:
        line = f"{line} # {tag}"
    return line


def _apply_pin_sync(file: Path, ar: ActionRef, new_sha: str, tag: Optional[str] = None) -> bool:
    lines = file.read_text().splitlines()
    old_line = lines[ar.line_index]
    # Strip any existing trailing comment before rewriting.
    bare = _RE_TRAILING.sub("", old_line).rstrip()
    prefix = ar.repo
    if ar.path_suffix:
        prefix = f"{prefix}/{ar.path_suffix}"
    target = _uses_line(prefix, new_sha, tag)
    new_line = _RE_USES.sub(target, bare)
    if new_line == old_line:
        return False
    lines[ar.line_index] = new_line
    file.write_text("\n".join(lines) + "\n")
    return True


async def apply_pin(file: Path, ar: ActionRef, new_sha: str, tag: Optional[str] = None) -> bool:
    return await asyncio.to_thread(_apply_pin_sync, file, ar, new_sha, tag)


def _apply_tag_sync(file: Path, ar: ActionRef, tag: str) -> bool:
    """Pin to ``@<tag>`` with no trailing comment -- the tag IS the version."""
    lines = file.read_text().splitlines()
    old_line = lines[ar.line_index]
    bare = _RE_TRAILING.sub("", old_line).rstrip()
    prefix = ar.repo
    if ar.path_suffix:
        prefix = f"{prefix}/{ar.path_suffix}"
    new_line = _RE_USES.sub(f"uses: {prefix}@{tag}", bare)
    if new_line == old_line:
        return False
    lines[ar.line_index] = new_line
    file.write_text("\n".join(lines) + "\n")
    return True


async def apply_tag(file: Path, ar: ActionRef, tag: str) -> bool:
    return await asyncio.to_thread(_apply_tag_sync, file, ar, tag)


def _fix_comment_sync(file: Path, ar: ActionRef, tag: str) -> bool:
    """Ensure the existing SHA-pinned line has the correct tag comment."""
    lines = file.read_text().splitlines()
    old_line = lines[ar.line_index]
    # Strip any existing trailing comment before re-writing.
    bare = _RE_TRAILING.sub("", old_line).rstrip()
    prefix = ar.repo
    if ar.path_suffix:
        prefix = f"{prefix}/{ar.path_suffix}"
    target = _uses_line(prefix, ar.current_ref, tag)
    new_line = _RE_USES.sub(target, bare)
    if new_line == old_line:
        return False
    lines[ar.line_index] = new_line
    file.write_text("\n".join(lines) + "\n")
    return True


async def fix_comment(file: Path, ar: ActionRef, tag: str) -> bool:
    return await asyncio.to_thread(_fix_comment_sync, file, ar, tag)


# ---------------------------------------------------------------------------
# Main (async)
# ---------------------------------------------------------------------------


async def main_async(args: argparse.Namespace) -> int:
    if not (os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")):
        log.warn(
            "no GITHUB_TOKEN / GH_TOKEN set --- unauthenticated API is "
            "limited to 60 req/hr",
        )

    refs = await collect_refs_async(REPO_ROOT)
    if not refs:
        log.info("No external action references found.")
        return 0

    log.debug(
        f"checking {len(refs)} reference(s) across "
        f"{len({r.file for r in refs})} file(s)",
    )

    async with httpx.AsyncClient(headers=_gh_headers()) as client:
        # Run all ref checks concurrently.
        results: List[PinResult] = await asyncio.gather(
            *[check_ref(client, ar) for ar in refs],
        )

    changed = 0
    comments_fixed = 0
    major_bumps = 0
    errors = 0
    for r in results:
        tag_display = r.ref.display

        if r.status == "uptodate":
            log.info(f"  OK  {tag_display:50s}  {r.message}")
            # No comment on a tag-only ref: `@v2.1.0 # v2.1.0` is noise.
            if args.write and r.tag and not _is_tag_only(r.ref):
                if await fix_comment(r.ref.file, r.ref, r.tag):
                    comments_fixed += 1
                    log.info(
                        f"       -> comment: {r.ref.file.name}:"
                        f"{r.ref.line_index + 1}",
                    )
        elif r.status == "major-bump":
            major_bumps += 1
            note = " (use --apply-major)" if not args.apply_major else ""
            log.info(f" MAJOR {tag_display:50s}  {r.message}{note}")
            if args.apply_major and r.new_sha:
                if await apply_pin(r.ref.file, r.ref, r.new_sha, tag=r.tag):
                    changed += 1
                    log.info(
                        f"       -> {r.ref.file.name}:{r.ref.line_index + 1}",
                    )
        elif r.status == "updated":
            changed += 1
            log.info(f"  UP  {tag_display:50s}  {r.message}")
            if args.write and r.pin_as_tag and r.tag:
                if await apply_tag(r.ref.file, r.ref, r.tag):
                    log.info(f"       -> {r.ref.file.name}:{r.ref.line_index + 1}")
            elif args.write and r.new_sha:
                if await apply_pin(r.ref.file, r.ref, r.new_sha, tag=r.tag):
                    changed += 1
                    log.info(
                        f"       -> {r.ref.file.name}:{r.ref.line_index + 1}",
                    )
        elif r.status == "error":
            errors += 1
            log.error(f" ERROR  {tag_display:50s}  {r.message}")

    if errors:
        log.error(f"\n{errors} error(s)")

    if not args.write and (changed or comments_fixed):
        log.info(f"\n{changed} update(s) available. Run with --write to apply.")
        return 1

    if changed:
        log.info(f"\n{changed} file(s) updated.")
    if comments_fixed:
        log.info(f"{comments_fixed} comment(s) fixed.")

    if not changed and not comments_fixed and major_bumps and not args.apply_major:
        log.info(
            f"\n{major_bumps} major-bump(s) available. "
            "Review manually or use --apply-major.",
        )

    return 0 if not errors else 1


def _configure_logging(args: argparse.Namespace) -> None:
    """Set log level based on ``--verbose`` / ``--quiet`` counts."""
    level = _DEFAULT_LOG_LEVEL
    level -= args.verbose * 10
    level += args.quiet * 10
    log.level = max(LOG_DEBUG, min(LOG_ERROR, level))


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Update SHA pins for GitHub Actions",
        epilog="Set GITHUB_TOKEN or GH_TOKEN env var for authenticated API access.",
    )
    ap.add_argument("--write", action="store_true", help="write changes")
    ap.add_argument("--apply-major", action="store_true", help="apply major bumps")
    ap.add_argument(
        "-v",
        "--verbose",
        action="count",
        default=0,
        help="increase verbosity (repeat for debug)",
    )
    ap.add_argument(
        "-q",
        "--quiet",
        action="count",
        default=0,
        help="decrease verbosity (repeat for errors only)",
    )
    ap.add_argument(
        "--config",
        type=str,
        default=None,
        help="path to config file (JSON / YAML / TOML)",
    )
    ap.add_argument(
        "--print-config",
        action="store_true",
        help="print effective config and exit",
    )
    args = ap.parse_args()
    _configure_logging(args)

    global CFG
    CFG = load_config(args.config)

    if args.print_config:
        import pprint  # noqa: PLC0415

        pprint.pprint(
            {
                "branch_only_repos": CFG.branch_only_repos,
                "branch_overrides": CFG.branch_overrides,
                "ref_overrides": CFG.ref_overrides,
                "tag_only_repos": CFG.tag_only_repos,
                "max_concurrent_api": CFG.max_concurrent_api,
            },
        )
        return 0

    return asyncio.run(main_async(args))


if __name__ == "__main__":
    sys.exit(main())
