---
tags: [moc, core, cw-update]
---

# Core / cw-update — Map of Content

> Notes for `cw-update`: 7 concepts.

## Notes

- [[cw-update]] — Self-update of an installed CodeWiki: release check, download, verification and file swap.
- [[Download Module]] — downloading a package and checking it against its `.sha256` file.
- [[Release]] — A release newer than the running program.
- [[Release Module]] — asking GitHub for the latest release.
- [[Stage Module]] — unpacking a verified package and checking that its programs start.
- [[Swap Module]] — replacing the files of an installation, and undoing it.
- [[Wire Tests Module]] — drives the check and the download against a stand-in for GitHub, so no network is needed.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
