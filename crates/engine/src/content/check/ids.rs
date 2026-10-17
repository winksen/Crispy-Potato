//! The format of each kind of external id, so a typo or an empty value fails
//! validation instead of a lookup in a source later.

use super::is_slug;

/// What a valid id under `ids.<field>` looks like, if `id` isn't one.
pub(super) fn problem(field: &str, id: &str) -> Option<&'static str> {
    let (valid, expected) = match field {
        "wikidata" => (wikidata(id), "a Wikidata item id like Q575650"),
        "github" => (
            github(id),
            "a GitHub repository like rust-lang/rust: an owner of letters, digits and single \
             hyphens, and a name of letters, digits, -, _ and . that is not . or .. and does \
             not end in .git",
        ),
        "npm" => (
            npm(id),
            "an npm package name like react or @angular/core: lowercase letters, digits, -, ., _ \
             and ~, not starting with . or _",
        ),
        "pypi" => (
            pypi(id),
            "a PyPI project name like django: letters, digits, -, _ and ., starting and ending \
             with a letter or digit",
        ),
        "crates" => (
            crates_io(id),
            "a crates.io name like axum: up to 64 letters, digits, - and _, starting with a letter",
        ),
        "nuget" => (
            nuget(id),
            "a NuGet package id like Microsoft.Maui.Controls: words of letters, digits and _ \
             joined by single . or -",
        ),
        "packagist" => (
            packagist(id),
            "a Packagist name like laravel/framework: vendor/package in lowercase letters and \
             digits, with single -, _ or . between them",
        ),
        "rubygems" => (
            rubygems(id),
            "a gem name like rails: letters, digits, -, _ and ., starting with a letter or digit \
             and holding at least one letter",
        ),
        "pub" => (
            pub_dev(id),
            "a pub.dev package name like flutter: lowercase letters, digits and _, not starting \
             with a digit",
        ),
        "hex" => (
            hex(id),
            "a Hex package name like phoenix: lowercase letters, digits and _, starting with a \
             letter",
        ),
        "maven" => (
            maven(id),
            "Maven coordinates like org.springframework.boot:spring-boot: a dotted group id, a \
             colon and an artifact id",
        ),
        "stackoverflow_tag" => (
            stackoverflow_tag(id),
            "a Stack Overflow tag like rust or c#: up to 35 lowercase letters, digits and + # . -",
        ),
        "so_survey" => (
            survey_name(id),
            "the name the Stack Overflow survey uses, like Rust: up to 60 characters with no \
             leading or trailing spaces",
        ),
        "endoflife" => (
            is_slug(id),
            "an endoflife.date product like nodejs: lowercase letters and digits joined by \
             single hyphens",
        ),
        "simple_icons" => (
            simple_icons(id),
            "a Simple Icons slug like rust or nodedotjs: lowercase letters and digits, with \
             single _ between words",
        ),
        _ => return None,
    };
    (!valid).then_some(expected)
}

/// Non-empty words of `word` bytes joined by single separators.
fn words(s: &str, separators: &[char], word: fn(u8) -> bool) -> bool {
    s.split(separators)
        .all(|part| !part.is_empty() && part.bytes().all(word))
}

fn lower_or_digit(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit()
}

fn wikidata(s: &str) -> bool {
    s.strip_prefix('Q').is_some_and(|n| {
        !n.is_empty() && !n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit())
    })
}

fn github(s: &str) -> bool {
    let Some((owner, repo)) = s.split_once('/') else {
        return false;
    };
    let owner_ok = owner.len() <= 39 && words(owner, &['-'], |b| b.is_ascii_alphanumeric());
    let repo_ok = (1..=100).contains(&repo.len())
        && repo
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        && repo != "."
        && repo != ".."
        && !repo.to_ascii_lowercase().ends_with(".git");
    owner_ok && repo_ok
}

fn npm(s: &str) -> bool {
    let part = |p: &str| {
        !p.is_empty()
            && !p.starts_with(['.', '_'])
            && p.bytes()
                .all(|b| lower_or_digit(b) || matches!(b, b'-' | b'.' | b'_' | b'~'))
    };
    let valid = match s.strip_prefix('@') {
        Some(scoped) => scoped
            .split_once('/')
            .is_some_and(|(scope, name)| part(scope) && part(name)),
        None => part(s),
    };
    valid && s.len() <= 214
}

fn pypi(s: &str) -> bool {
    let edge = |b: Option<&u8>| b.is_some_and(u8::is_ascii_alphanumeric);
    edge(s.as_bytes().first())
        && edge(s.as_bytes().last())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn crates_io(s: &str) -> bool {
    s.len() <= 64
        && s.starts_with(|c: char| c.is_ascii_alphabetic())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

fn nuget(s: &str) -> bool {
    s.len() <= 100 && words(s, &['.', '-'], |b| b.is_ascii_alphanumeric() || b == b'_')
}

fn packagist(s: &str) -> bool {
    s.split_once('/').is_some_and(|(vendor, package)| {
        // Package names may also use a double hyphen.
        words(vendor, &['-', '_', '.'], lower_or_digit)
            && words(
                &package.replace("--", "-"),
                &['-', '_', '.'],
                lower_or_digit,
            )
    })
}

fn rubygems(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphanumeric())
        && s.bytes().any(|b| b.is_ascii_alphabetic())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn pub_dev(s: &str) -> bool {
    s.len() <= 64
        && s.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
        && s.bytes().all(|b| lower_or_digit(b) || b == b'_')
}

fn hex(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.bytes().all(|b| lower_or_digit(b) || b == b'_')
}

fn maven(s: &str) -> bool {
    let Some((group, artifact)) = s.split_once(':') else {
        return false;
    };
    words(group, &['.'], |b| {
        b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')
    }) && !artifact.is_empty()
        && !artifact.starts_with('.')
        && artifact
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn stackoverflow_tag(s: &str) -> bool {
    (1..=35).contains(&s.len())
        && s.bytes().any(lower_or_digit)
        && s.bytes()
            .all(|b| lower_or_digit(b) || matches!(b, b'+' | b'#' | b'.' | b'-'))
}

fn simple_icons(s: &str) -> bool {
    s.len() <= 64 && words(s, &['_'], lower_or_digit)
}

fn survey_name(s: &str) -> bool {
    !s.is_empty() && s.trim() == s && s.chars().count() <= 60 && !s.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::problem;

    #[test]
    fn each_id_kind_accepts_real_ids_and_refuses_malformed_ones() {
        let cases: &[(&str, &[&str], &[&str])] = &[
            (
                "wikidata",
                &["Q575650", "Q2005"],
                &["", "575650", "Q", "Q0123", "q575650", "Q57a"],
            ),
            (
                "github",
                &[
                    "rust-lang/rust",
                    "facebook/react",
                    "vercel/next.js",
                    "a/b_c",
                    "a/.github",
                ],
                &[
                    "",
                    "rust",
                    "rust-lang/",
                    "/rust",
                    "rust-lang/rust/extra",
                    "-owner/repo",
                    "owner-/repo",
                    "own--er/repo",
                    "own_er/repo",
                    "owner/.",
                    "owner/..",
                    "owner/repo.git",
                    "owner/repo.GIT",
                    "owner/re po",
                ],
            ),
            (
                "npm",
                &[
                    "react",
                    "@angular/core",
                    "next",
                    "vue",
                    "lodash.debounce",
                    "@types/node",
                ],
                &[
                    "", "React", ".hidden", "_private", "@scope", "@scope/", "a b", "a/b", "@/core",
                ],
            ),
            (
                "pypi",
                &[
                    "django",
                    "Flask",
                    "fastapi",
                    "zope.interface",
                    "typing_extensions",
                ],
                &["", "-django", "django-", ".django", "dj ango", "dj/ango"],
            ),
            (
                "crates",
                &["axum", "serde_json", "tokio-util"],
                &["", "1password", "_axum", "ax.um", "ax um", &"a".repeat(65)],
            ),
            (
                "nuget",
                &[
                    "Microsoft.Maui.Controls",
                    "Newtonsoft.Json",
                    "xunit",
                    "Foo-Bar_Baz",
                ],
                &["", ".Foo", "Foo.", "Foo..Bar", "Foo Bar", "Foo/Bar"],
            ),
            (
                "packagist",
                &[
                    "laravel/framework",
                    "symfony/http-kernel",
                    "vendor/pkg--name",
                ],
                &[
                    "",
                    "laravel",
                    "Laravel/framework",
                    "laravel/",
                    "/framework",
                    "a/b/c",
                    "a/-b",
                ],
            ),
            (
                "rubygems",
                &["rails", "activesupport", "net-http", "nokogiri", "a2"],
                &["", ".rails", "-rails", "_rails", "123", "ra ils", "ra/ils"],
            ),
            (
                "pub",
                &["flutter", "http", "flutter_bloc", "_internal"],
                &["", "Flutter", "1flutter", "flutter-bloc", "flutter.bloc"],
            ),
            (
                "hex",
                &["phoenix", "ecto_sql", "plug2"],
                &[
                    "",
                    "Phoenix",
                    "_phoenix",
                    "2phoenix",
                    "phoenix-live",
                    "phoenix.live",
                ],
            ),
            (
                "maven",
                &[
                    "org.springframework.boot:spring-boot",
                    "io.ktor:ktor-server-core",
                    "junit:junit",
                ],
                &[
                    "",
                    "spring-boot",
                    "org.springframework.boot:",
                    ":spring-boot",
                    "org..boot:x",
                    "a:b:c",
                    "a:.b",
                ],
            ),
            (
                "stackoverflow_tag",
                &[
                    "rust",
                    "reactjs",
                    "c#",
                    "c++",
                    ".net",
                    "asp.net-core",
                    "node.js",
                ],
                &["", "Rust", "react js", "++", &"a".repeat(36), "rust/lang"],
            ),
            (
                "so_survey",
                &["Rust", "React", "C#", "ASP.NET Core", "Node.js"],
                &["", " Rust", "Rust ", "Ru\nst", &"a".repeat(61)],
            ),
            (
                "simple_icons",
                &[
                    "rust",
                    "react",
                    "nodedotjs",
                    "dotnet",
                    "hive_blockchain",
                    "c",
                ],
                &[
                    "",
                    "Rust",
                    "node.js",
                    "node-js",
                    "_rust",
                    "rust_",
                    "hive__blockchain",
                ],
            ),
            (
                "endoflife",
                &["nodejs", "spring-boot", "rust", "dotnet"],
                &[
                    "",
                    "NodeJS",
                    "spring_boot",
                    "spring--boot",
                    "-rust",
                    "node.js",
                ],
            ),
        ];
        for (field, valid, invalid) in cases {
            for id in *valid {
                assert_eq!(
                    problem(field, id),
                    None,
                    "ids.{field}: {id:?} should be valid"
                );
            }
            for id in *invalid {
                assert!(
                    problem(field, id).is_some(),
                    "ids.{field}: {id:?} should be refused"
                );
            }
        }
    }
}
