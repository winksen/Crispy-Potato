# Contributing to Crispy-Potato

Crispy-Potato is a reference for programming languages, frameworks and runtimes. Most of it is hand-written content in `content/`, reviewed through pull requests; the numbers (stars, downloads, survey results, releases) are collected from public sources and never written by hand. This guide covers adding and editing entities, the rules for sources and tone, how domain fit is rated, and what reviewers check.

You can also help by [reporting a data problem](https://github.com/winksen/Crispy-Potato/issues/new?template=report-data-problem.yml) or [requesting an entity](https://github.com/winksen/Crispy-Potato/issues/new?template=request-entity.yml). Security problems go through [SECURITY.md](SECURITY.md), never a public issue.

## Before you start

- **Rust.** Install it with [rustup](https://rustup.rs); `rust-toolchain.toml` picks the version. Validation needs nothing else: no database, no network.
- **An editor with the YAML language server**, such as VS Code with the Red Hat YAML extension. Each entity file points at its JSON Schema, so fields and allowed values are completed and mistakes are underlined as you type.
- **Licenses.** Content in `content/` is under [CC BY-SA 4.0](content/LICENSE); code and documentation are under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE). By opening a pull request you agree that your contribution is licensed the same way.

| Command | What it does |
|---|---|
| `cargo new-entity <kind> <id>` | Starts an entity file from a template and takes its id off `planned.yaml` |
| `cargo validate` | Checks every content file and lists every problem with its file and field |
| `cargo schema` | Regenerates the editor schemas after a model type or a vocabulary changes |
| `cargo test` | Runs the tests, including the content checks |

## Adding an entity

1. **Check that it is in scope.** The first public version covers the 50 entities in [`content/planned.yaml`](content/planned.yaml). For anything else, open a *Request an entity* issue first and wait for it to be accepted.
2. **Start the file.** For example:
   ```bash
   cargo new-entity framework spring-boot
   ```
   This writes `content/frameworks/spring-boot.yaml` with every field, a comment explaining each one, the allowed values of each trait, and `TODO` where you need to write something. It takes the id off `planned.yaml`. The id is the page's address and never changes: lowercase letters, digits and single hyphens.
3. **Fill in every `TODO`.** Delete the optional lines that don't apply, such as an id the entity doesn't have. [The data model](docs/data-model.md) describes every field and the format of every external id. The rest of this guide covers what to write:
   - the **tagline** and **summary** describe what it is in neutral words ([Writing style](#writing-style));
   - every **strength** and **weakness** a reader could dispute has a source ([Sourcing rules](#sourcing-rules));
   - **domain fit** follows [the rubric](#rating-domain-fit);
   - **`choose_when`** describes situations, not strengths;
   - **relations** are written on one side only; the reverse direction is derived;
   - the **timeline** has milestones that aren't releases (releases are collected);
   - **notable users** each have a source in which the user says so themselves;
   - a **code sample** is short, runs as shown, and is your own work.
4. **Run `cargo validate`** until it reports nothing. While a `TODO` is left, it lists only the placeholders.
5. **Open a pull request.** The template has the checklist reviewers use; tick what you have done and put your sources for fit ratings in its description.

## Editing an entity

- **Change what you can source**, and say in the pull request why the old text was wrong or out of date.
- **Never rename an id.** It is the page's address and other entities refer to it.
- **Numbers are collected, not written.** If a number on the site is wrong, open a *Report a data problem* issue; the cause may be the source itself.
- **A collected fact that is wrong**, such as an inception date or a designer from Wikidata, is corrected under `overrides` with a `reason` that says why the source is wrong and links what shows the right value.

## Sourcing rules

- **A source for anything a reader could dispute**: performance, adoption, comparisons with other entities, history, who uses it, and any "most" or "first". Basic facts the project's own documentation states plainly don't need one, but a link never hurts.
- **Use good sources.** In order of preference: the project's own documentation, specifications and release notes; its foundation or steering body; standards and papers; established technical publications; a company's own engineering blog for how that company uses it. Don't use vendor marketing, comparison listicles, AI-generated pages, anonymous forum posts, or Wikipedia on its own (use the sources it cites).
- **Read the facts against Wikidata and the project's own site.** Dates, designers, licenses and links must agree with them, or carry an override explaining why not.
- **No collected numbers in content.** Stars, downloads, ranks and survey shares are collected with their source and date; the schema refuses fields such as `stars:`, and prose must not quote them either ("over 200,000 stars").
- **Keep a neutral tone.** Describe, don't sell. Leave out marketing words such as *blazing*, *revolutionary*, *best-in-class*, *powerful*, *seamless*, *easy*, *simply*, *just* and *loved by developers*, and write the concrete claim behind them, with a source. Weaknesses are real trade-offs, not strengths in disguise. Don't address the reader as "you", except in `choose_when`, and don't write as "we".
- **Write in your own words.** Don't copy sentences from documentation or other sites; their licenses rarely allow it and our content is CC BY-SA. A short quotation with its source is fine.
- **No unreviewed AI text.** You may use AI tools to draft, translate or check, but every sentence you submit must be checked against its sources, rewritten in plain words where needed, and yours to answer for. Say in the pull request if and how you used one. Pull requests with unchecked generated text are closed.
- **Use stable links.** Prefer `https`, canonical pages over search results, and pages that will still be there in a few years.

## Rating domain fit

Each entity rates the domains that matter for it (not all of them) on a four-step scale. Rate it against the alternatives for that domain, and compare with how other entities rate the same domain, so that "good for web backends" means the same thing across the catalog.

| Fit | Means | Example |
|---|---|---|
| `excellent` | One of the first choices for this kind of project: built for it or widely used for it in production, with mature libraries and tools and few compromises. | Rust for command-line tools; React for web frontends |
| `good` | A sound choice that is used in production for it, with a gap compared with the leaders: a smaller ecosystem, more setup, or a trade-off that matters to some projects. | Rust for web backends, with a note on the ecosystem |
| `possible` | It can be done and is done, but with extra work, third-party tools or clear trade-offs; most teams would choose something else. | React for mobile, through React Native |
| `poor` | Rarely a reasonable choice for the project itself, or used only indirectly. | Rust for data science, where it builds tools such as Polars rather than analyses |

- **A note is required** for `possible` and `poor` (what the extra work or the trade-off is, and what people use instead), and for any rating a reviewer questions.
- **Back each rating with evidence** you could cite, such as production use, official support or the state of the ecosystem. Put the sources in a strength or a weakness, or in the pull request's description.
- **When unsure between two ratings, choose the lower one** and explain in the note.

## Writing style

- **Tagline:** one sentence of at most 100 characters saying what it is, such as "Systems programming with memory safety and no garbage collector."
- **Summary:** two to four sentences: what it is, how it works at a high level and what it is used for.
- **Strengths and weaknesses:** two to six of each. The title is a short phrase; the detail is one or two sentences saying what and why.
- **`choose_when`:** two to four situations that each complete "Choose *name* when …", in lower case and at most 140 characters, such as "you need the performance and control of C or C++ and want the compiler to rule out memory errors and data races". They say when a reader should pick this entity over its alternatives, rather than repeating its strengths.
- **Language:** plain English, sentence case, no exclamation marks.

## Review checklist

Reviewers go through this list, and authors can run through it before asking for a review.

- [ ] CI passes: validation, the schema check and the tests.
- [ ] The entity is in the v1 scope, or an accepted issue added it.
- [ ] Every claim a reader could dispute has a source that opens and says what the text says.
- [ ] Dates, designers, licenses and links match Wikidata and the project's own site, or an override explains why not.
- [ ] Each external id points at the right thing: open the Wikidata item, the GitHub repository, the packages, the Stack Overflow tag, the endoflife.date page and the Simple Icons icon.
- [ ] The survey name matches each year's results (`so_survey` can give a name per year).
- [ ] The tone is neutral, with no marketing words, and the weaknesses are real trade-offs.
- [ ] Fit ratings follow the rubric and agree with how other entities rate the same domain; `possible` and `poor` have notes.
- [ ] `choose_when` describes situations and doesn't repeat the strengths.
- [ ] Relations are right and written once; alternatives are of the same kind.
- [ ] No collected number is written into the file, in a field or in prose.
- [ ] Code samples are short, run as shown, and are the author's own work.
- [ ] The text is in the author's own words, and any AI help was checked against the sources and disclosed.

## Contributing code

- **Open an issue first** for anything bigger than a fix, so the change fits the [release plan](docs/RELEASES_0.1.md). Each release is one row of that plan and one commit; the rules are in its *How each release is made* section.
- **Run the checks** before opening a pull request: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo validate`, `cargo schema --check` and `cargo deny check`.
- **New dependencies** must pass `cargo deny` (the licenses allowed are in `deny.toml`) and be named in the changelog.
- **Content and source responses are untrusted input.** The [threat model](docs/security/threat-model.md) lists what the code must defend against.
