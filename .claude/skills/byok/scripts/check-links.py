#!/usr/bin/env python3
"""Check that every keyUrl and docsUrl in providers.json still resolves.

A BYOK tab whose "Get a key" buttons 404 is worse than no buttons at all — the
user follows a dead link and concludes the app is broken. Console URLs get
reorganised often, so this runs in CI rather than being verified once by hand.

Exit codes:
  0  every link reachable (or soft-failed, unless --strict)
  1  at least one link is genuinely broken
  2  the registry itself could not be read

Bot-blocking (403/405/429) is reported separately from a real 404. Those are
usually a WAF refusing an automated client, not a dead page, so they do not
fail the build unless --strict is passed.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import sys
import urllib.error
import urllib.request
from pathlib import Path

REGISTRY = Path(__file__).resolve().parent.parent / "reference" / "providers.json"

# Plain urllib announces itself as Python and gets refused far more often.
UA = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/125.0 Safari/537.36"
)
TIMEOUT = 20

# A WAF turning away an automated client, rather than a missing page.
SOFT_STATUSES = {401, 403, 405, 429, 503}


def probe(url: str) -> tuple[int | None, str]:
    """Return (status, note). HEAD first; some hosts only answer GET."""
    for method in ("HEAD", "GET"):
        req = urllib.request.Request(url, method=method, headers={"User-Agent": UA})
        try:
            with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
                return resp.status, method
        except urllib.error.HTTPError as e:
            if method == "HEAD" and e.code in (403, 405, 501):
                continue  # retry as GET
            return e.code, method
        except urllib.error.URLError as e:
            return None, f"{type(e.reason).__name__}: {e.reason}"
        except Exception as e:  # noqa: BLE001 - a checker must never itself crash
            return None, f"{type(e).__name__}: {e}"
    return None, "unreachable"


REQUIRED = ("id", "name", "category", "keyUrl", "docsUrl", "capabilities")


def validate(registry: dict) -> list[str]:
    """Structural problems that would render as a broken row rather than an error."""
    problems: list[str] = []
    known_categories = {c["id"] for c in registry.get("categories", [])}
    seen: set[str] = set()
    cap_keys = set(registry.get("capabilityKeys", {}))

    for i, p in enumerate(registry.get("providers", [])):
        where = p.get("id") or f"providers[{i}]"
        for field in REQUIRED:
            if not p.get(field):
                problems.append(f"{where}: missing {field}")
        if p.get("id") in seen:
            problems.append(f"{where}: duplicate id")
        seen.add(p.get("id"))
        if p.get("category") and p["category"] not in known_categories:
            problems.append(f"{where}: unknown category {p['category']!r}")
        for k in p.get("capabilities", {}):
            if k not in cap_keys:
                problems.append(f"{where}: capability {k!r} is not declared in capabilityKeys")
    return problems


def collect(registry: dict) -> list[tuple[str, str, str]]:
    """(provider id, field name, url) for every external link."""
    out = []
    for p in registry["providers"]:
        for field in ("keyUrl", "docsUrl"):
            url = p.get(field)
            if url and url.startswith("http") and "localhost" not in url:
                out.append((p["id"], field, url))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--strict",
        action="store_true",
        help="treat bot-blocking (401/403/405/429/503) as failure too",
    )
    ap.add_argument("--registry", type=Path, default=REGISTRY)
    args = ap.parse_args()

    try:
        registry = json.loads(args.registry.read_text())
    except (OSError, json.JSONDecodeError) as e:
        print(f"cannot read {args.registry}: {e}", file=sys.stderr)
        return 2

    if problems := validate(registry):
        print(f"{len(problems)} structural problem(s) in {args.registry.name}:", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1
    print(f"registry structure OK ({len(registry['providers'])} providers)")

    targets = collect(registry)
    print(f"checking {len(targets)} links from {args.registry.name}\n")

    broken: list[str] = []
    soft: list[str] = []

    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        results = pool.map(lambda t: (t, probe(t[2])), targets)

        for (pid, field, url), (status, note) in results:
            if status and 200 <= status < 400:
                mark = "ok  "
            elif status in SOFT_STATUSES:
                mark = "soft"
                soft.append(f"{pid}.{field} -> {status} {url}")
            else:
                mark = "FAIL"
                broken.append(f"{pid}.{field} -> {status or note} {url}")
            print(f"  [{mark}] {status or note:<24} {pid}.{field}")

    print()
    if soft:
        print(f"{len(soft)} link(s) refused an automated client "
              f"(likely a bot filter, not a dead page):")
        for s in soft:
            print(f"  - {s}")
        print()

    if broken:
        print(f"{len(broken)} BROKEN link(s):")
        for b in broken:
            print(f"  - {b}")
        return 1

    if soft and args.strict:
        print("--strict: treating soft failures as errors")
        return 1

    print(f"all {len(targets)} links resolved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
