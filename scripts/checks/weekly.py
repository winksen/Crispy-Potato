"""Summarize local reports; maintain one bot-owned issue. Never interpolate shell code."""
import json
import pathlib
import subprocess
import sys
import tempfile

TITLE = "Content health checks"


def summary(paths):
    problems = []
    skipped = []
    for path in paths:
        try:
            report = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
            problems.extend(report["problems"])
            skipped.extend(report.get("skipped", []))
        except (OSError, ValueError, KeyError, TypeError):
            problems.append({"file": path, "field": "check", "message": "Check failed before producing a valid report"})
    lines = ["<!-- crispy-potato-content-health -->", "Automated content checks.", ""]
    for heading, items in [("Problems", problems), ("Skipped checks", skipped)]:
        lines.extend([f"### {heading}", ""])
        lines.extend(f"- `{item['file']}` / `{item['field']}`: {item['message']}" for item in items)
        if not items:
            lines.append("None.")
        lines.append("")
    return "\n".join(lines)[:60000], bool(problems or skipped)


def main(paths):
    body, needs_attention = summary(paths)
    issues = json.loads(subprocess.check_output(["gh", "issue", "list", "--state", "all", "--search", f'"{TITLE}" in:title', "--limit", "100", "--json", "number,title,author,state"], text=True))
    issue = next((i for i in issues if i["title"] == TITLE and i["author"]["login"] == "github-actions[bot]"), None)
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
