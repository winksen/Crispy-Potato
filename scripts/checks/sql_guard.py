"""Prove clippy rejects dynamic application SQL; never execute the fixture."""
from pathlib import Path
import subprocess


def main():
    root = Path(__file__).resolve().parents[2]
    examples = root / "crates" / "engine" / "examples"
    examples.mkdir(exist_ok=True)
    fixture = examples / "__sql_guard_check.rs"
    # Exclusive creation preserves any pre-existing developer file.
    with fixture.open("x", encoding="utf-8") as out:
        out.write('fn main() { let table = "observations"; '
                  'let sql = format!("SELECT * FROM {table}"); '
                  'let _ = sqlx::query::<sqlx::Postgres>(sqlx::AssertSqlSafe(sql.as_str())); }\n')
    try:
        result = subprocess.run(
            ["cargo", "clippy", "--workspace", "--example", "__sql_guard_check",
             "--locked", "--", "-D", "warnings"],
            cwd=root, capture_output=True, text=True, encoding="utf-8", errors="replace",
        )
        if result.returncode == 0 or "disallowed method" not in result.stderr:
            print(result.stderr)
            raise SystemExit("SQL guard regression: dynamic query was not rejected by the expected lint")
        print("SQL guard rejects a format!-built application query")
    finally:
        fixture.unlink()


if __name__ == "__main__":
    main()
