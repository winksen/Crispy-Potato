# Security policy

## Reporting a vulnerability

Please report security problems privately, through GitHub's private vulnerability reporting: open the repository's **Security** tab and choose **Report a vulnerability**. Do not open a public issue, pull request or discussion about it.

A useful report says:

- what is affected: a crate, the content pipeline, the data collection, the API, the website or CI;
- how to reproduce it, with the smallest input that shows it;
- what an attacker could do with it.

The report stays private while it is assessed and fixed. The fix is released with a changelog entry under *Security*, and the reporter is credited unless they ask not to be.

## Supported versions

Crispy-Potato is in its 0.1 line and not deployed yet. Fixes go into the next release on `main`; earlier releases are not patched.

## What is in scope

[docs/security/threat-model.md](docs/security/threat-model.md) lists what the project protects, where untrusted input comes in and which release deals with each threat. A way around any of those mitigations is in scope, including in content files: they come from pull requests and are treated as untrusted.

Out of scope: the security of the external data sources themselves, network-level denial of service, and findings that need a compromised maintainer account or machine.
