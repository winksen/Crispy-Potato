# Guidelines for AI assistants working on Crispy-Potato

## Where things are documented

- [docs/RELEASES_0.1.md](docs/RELEASES_0.1.md): the release index. One row per release of the 0.1 line, with its track, type, what it changes, when it is done and what it needs. The decisions it relies on (D1–D9) are agreed.
- [docs/MINDMAP.md](docs/MINDMAP.md): what the project is and what is built, kept current by every release.
- [CHANGELOG.md](CHANGELOG.md): what each release changed, newest first.
- [CONTRIBUTING.md](CONTRIBUTING.md): adding and editing entities, the sourcing rules, the fit-rating rubric and the review checklist. Content written for the catalog follows it.
- [docs/architecture.md](docs/architecture.md) and [docs/data-model.md](docs/data-model.md): how the system is meant to work and what each content field means.

## How each release is made

These are the rules from [the release index](docs/RELEASES_0.1.md#how-each-release-is-made). A change to them is made in both places.

1. **One row, one commit.** Work in version order. Before starting a row, read it, the rows in its *Needs* column, [architecture.md](docs/architecture.md) and [data-model.md](docs/data-model.md).
2. **Stay inside the row.** If a row needs a decision that isn't listed, stop and ask. Don't widen a row silently. If it turns out too big, ship its core, add a new row right after it for the rest, and renumber the planned rows that follow.
3. **Fixes.** A small bug found in shipped work rides along with the next release and is listed under *Fixed* in its changelog entry. A larger one gets its own 🐞 row, inserted next.
4. **Version.** Every release bumps `workspace.package.version` in the root `Cargo.toml` (and, from V0.1.96, the web package version) to its row's version. It is the only place the version is written; since V0.1.7 `cargo test` fails when it differs from the newest `CHANGELOG.md` entry.
5. **Changelog.** Every release adds an entry at the top of `CHANGELOG.md` in this format:
   ```markdown
   ## V0.1.N (YYYY-MM-DD)

   One sentence on what this release does.

   ### Added / ### Changed / ### Fixed / ### Security   (only the ones that apply)
   - **Bold lead.** What changed, in plain words.

   ### Not done
   - What the row asked for and this release didn't do, and why.

   ### Upgrading
   - What to run or change, or "Nothing to do."
   ```
6. **The index.** Every release sets its row to ✅ with the date, makes *What changes* describe what was built, and moves *Current* and *Next* in the header.
7. **Mindmap.** Every release updates [docs/MINDMAP.md](docs/MINDMAP.md): its scope line names the release, and the nodes and sections the release touched describe what is now built.
8. **Checks.** Run the checks for the side you touched. Back: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo validate`. Front: `pnpm lint`, `pnpm typecheck`, `pnpm test`, `pnpm build`. Also run anything the row's *Done when* names. Report what ran and what didn't; never claim a check that didn't run.
9. **Security rules that always apply:**
   - content files and source responses are untrusted input;
   - SQL only goes through sqlx bindings;
   - secrets never reach logs, URLs, error bodies or the browser bundle;
   - new dependencies must pass `cargo deny` and the web license check, and are named in the changelog.
10. **Commit message.** The title is `Update V0.1.N`, then one bullet per notable change in plain prose, without trailing periods. The second-to-last bullet says what was checked and what wasn't. The last bullet lists the documents updated.
    ```
    Update V0.1.N

    - <what changed>
    - <what changed>
    - <checks that ran, with counts; what was not run and why>
    - Updated changelog, mindmap and release index
    ```

**Content rows** (track E) are done when:
- the files validate;
- `cli verify-ids` and `cli check-links` pass for them;
- every claim a reader could dispute has a source, and no collected number is written into the file;
- the facts have been read against Wikidata and the project's own site;
- their ids are removed from `planned.yaml`.

## What may be run

`cargo` checks and tests, `pnpm` checks and tests, and local servers when a row's *Done when* needs them (decision D6). Everything run is reported in the commit.
