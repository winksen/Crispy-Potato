"""Summarize local reports; maintain one bot-owned issue. Never interpolate shell code."""
import json
import html
import pathlib
import subprocess
import sys
import tempfile

TITLE = "Content health checks"
MARKER = "<!-- crispy-potato-content-health -->"


def summary(paths):
    problems = []
    skipped = []
    for path in paths:
        try:
            file = pathlib.Path(path)
            if file.stat().st_size > 1024 * 1024:
                raise ValueError("oversized report")
            report = json.loads(file.read_text(encoding="utf-8"))
            for key in ("problems", "skipped"):
                items = report.get(key, []) if key == "skipped" else report[key]
                if not isinstance(items, list) or len(items) > 1000:
                    raise ValueError("invalid report items")
                if any(not isinstance(item, dict) or any(
                    not isinstance(item.get(field), str)
                    for field in ("file", "field", "message")
                ) for item in items):
                    raise ValueError("invalid report item")
            problems.extend(report["problems"])
            skipped.extend(report.get("skipped", []))
        except (OSError, ValueError, KeyError, TypeError):
            problems.append({"file": path, "field": "check", "message": "Check failed before producing a valid report"})
    lines = [MARKER, "Automated content checks.", ""]
    for heading, items in [("Problems", problems), ("Skipped checks", skipped)]:
        lines.extend([f"### {heading}", ""])
        for item in items:
            safe = {key: html.escape(" ".join(value.split())[:1000]).replace("`", "&#96;")
                    for key, value in item.items() if isinstance(value, str)}
            lines.append(f"- `{safe['file']}` / `{safe['field']}`: {safe['message']}")
        if not items:
            lines.append("None.")
        lines.append("")
    return "\n".join(lines)[:60000], bool(problems or skipped)


def main(paths):
    body, needs_attention = summary(paths)
    issues = json.loads(subprocess.check_output(["gh", "issue", "list", "--state", "all", "--search", f'"{TITLE}" in:title', "--limit", "100", "--json", "number,title,author,body,state"], text=True))
    issue = next((i for i in issues if i["title"] == TITLE
                  and MARKER in (i.get("body") or "")
                  and i.get("author", {}).get("is_bot") is True), None)
    if issue is None and not needs_attention:
        return
    with tempfile.TemporaryDirectory() as directory:
        path = pathlib.Path(directory) / "body.md"
        path.write_text(body, encoding="utf-8")
        if issue is None:
            subprocess.run(["gh", "issue", "create", "--title", TITLE, "--body-file", str(path)], check=True)
        else:
            number = str(issue["number"])
            subprocess.run(["gh", "issue", "edit", number, "--body-file", str(path)], check=True)
            if needs_attention and issue["state"] == "CLOSED":
                subprocess.run(["gh", "issue", "reopen", number], check=True)
            elif not needs_attention and issue["state"] == "OPEN":
                subprocess.run(["gh", "issue", "close", number], check=True)


if __name__ == "__main__":
    main(sys.argv[1:])
