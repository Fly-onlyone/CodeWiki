---
description: Run CodeWiki in your repository's CI, and keep the committed wiki in step with the code.
icon: circle-play
---

# GitHub Action

CodeWiki has a GitHub Action. It installs a released CodeWiki on the runner and can do two things in your CI:

* **Report findings.** It runs `codewiki check`, which runs every analysis, and writes a report that GitHub's code scanning can show.
* **Hold the wiki to the code.** It syncs the wiki in `obsidian/` and fails the job when the sync changes anything. That means code changed and the wiki was not synced with it.

## Use it

Add a job to a workflow of your repository:

{% code title=".github/workflows/codewiki.yml" %}
```yaml
name: CodeWiki
on:
  push:
    branches: [main]
  pull_request:

jobs:
  codewiki:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      security-events: write   # only for the upload below
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0       # hotspots, ownership and change coupling read the history
      - id: codewiki
        uses: Fly-onlyone/CodeWiki@action-v1
        with:
          wiki: check
      - if: always()
        uses: github/codeql-action/upload-sarif@v4
        with:
          sarif_file: ${{ steps.codewiki.outputs.report }}
          category: codewiki
```
{% endcode %}

Leave `wiki: check` out when the wiki is not committed to the repository, or when you only want the findings.

## Inputs

| Input | Default | What it does |
| --- | --- | --- |
| `version` | `latest` | The CodeWiki release to use, as in `0.6.1`. Name one to keep runs the same. |
| `repo` | `.` | The repository's root, from the workspace. |
| `check` | `true` | Runs `codewiki check` and writes the report. |
| `fail-on` | `never` | The lowest level of a finding that fails the job: `note`, `warning`, `error` or `never`. |
| `format` | `sarif` | The report's format: `sarif`, `junit` or `text`. |
| `report` | a file in the runner's temporary folder | Where the report is written. |
| `wiki` | `off` | `check` syncs the wiki and fails when the sync changes anything. `update` syncs it, or writes it the first time, and leaves the changes for a later step to commit. |

## Outputs

| Output | What it holds |
| --- | --- |
| `version` | The CodeWiki version that ran. |
| `report` | The path of the report. |
| `wiki-changed` | `true` when the sync changed the wiki, `false` when it did not. |

After the action, `codewiki` is on `PATH` for the steps that follow, so a later step can run any command of the [command line](command-line.md).

## Runners

* **Linux.** The action runs CodeWiki from the image `ghcr.io/fly-onlyone/codewiki`, so the runner needs Docker. GitHub's own Linux runners have it.
* **Windows.** The action takes `codewiki.exe` from the release's zip and checks its checksum.
* **macOS.** There is no CodeWiki for macOS runners.

## When the wiki check fails

The check fails when the wiki in `obsidian/` is not what the commit's code generates. To fix it, run the sync in your repository and commit the result:

```
codewiki update
```

Three things can make it fail when you did sync:

* **A file written into the checkout.** CodeWiki counts every file it finds in the repository's folder. A step that writes a file there before the action, such as a report, makes the wiki look out of date. Write such files elsewhere, or name them in `.codewikiignore`.
* **A bot that cannot sync.** A pull request from a bot such as Dependabot changes code and cannot run the sync. Leave the step out for it with an `if:`, and sync when you merge.
* **An older CodeWiki.** Up to version 0.6.0 a file saved with Windows line endings on one machine and Unix ones on another counted as changed. From 0.6.1 line endings make no difference. Sync the wiki once with 0.6.1 or newer.
