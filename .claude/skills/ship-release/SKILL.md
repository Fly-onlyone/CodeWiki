---
name: ship-release
description: This skill should be used when the user asks to "merge the worktrees and release", "ship a release", "trigger a new release", "cut version X", "land everything, test in the VM and release", or adds "then shut down", "then sleep" or "then hibernate the computer" to such a request. Runs CodeWiki's whole release train, from open worktrees to a published and checked release with its docs, and ends with the power action the user chose, if any.
---

# Ship a release

Land every open worktree on `main`, prove the package in a clean Windows guest, publish the release, bring the docs up to date, and leave the machine in the power state the user asked for. The run takes hours and the user is often away for most of it, so every decision is settled before the first change.

It chains skills that already exist. Load each when its step starts and follow it: `qingyi:merge-worktree`, `qingyi:commit`, `qingyi:clean-vm-test`. `AGENTS.md` holds the four checks, the CI rules and the VM rig's notes. This skill adds the order, the decisions, and what went wrong before.

## 1. Find the facts (change nothing)

- `git worktree list`; for each worktree `git -C <path> status --porcelain` and `git -C <path> log main..HEAD --oneline`. Read `.git/worktrees/<name>/locked`: it names the session that holds the worktree.
- `ListAgents`: which sessions are busy, and which one owns which worktree.
- `git status` on `main`: uncommitted work there ships too, or is set aside. It is never ignored.
- Workspace `version` in the root `Cargo.toml`, the newest tag, what is in `dist\` and `dist\old\`.
- `Get-VM`, `Get-VMSnapshot -VMName *`, free memory, `ffmpeg`.

Never `cd` into a worktree: the shell then holds its folder and `git worktree remove` fails with "Permission denied". Use `git -C`.

## 2. Settle the decisions

Ask them in rounds with `AskUserQuestion`, each with a recommended answer. Look facts up; ask only for decisions.

| Decision | Usual answer |
|---|---|
| Uncommitted work on `main` | commit it, split by concern |
| A worktree whose session is still busy | wait for all of it; never merge half a feature |
| A worktree with no commits | ask whether work is about to start there before removing it |
| Version | minor bump for features, patch for fixes |
| VM passes | GPU pass on `Tiny 11`; both passes when drawing or motion changed |
| Guest protection | `-Hardened`: Windows' own protection turned all the way up |
| Plugin test | `-Plugin`; it uses the user's Claude login in the guest, so ask |
| A defect found by the VM run or CI | fix, rerun, tag only when clean |
| VM scope | new cases for new features, update from the previous version, a run against the published release |
| Docs | `ROADMAP.md`, `README.md`, `AGENTS.md`, wiki refresh |
| Power action at the end | none, unless the user names one (section 9) |
| Power action when no release came out | none, unless the user says it runs either way |

Write the answers into the work-state list (`work_state_write`), so a later session can pick the run up. Leave the power action out of it: a later session asks again.

The user may change an answer at any time. The latest word counts, also when it takes a power action back.

## 3. Work with the other sessions

A worktree is finished when its session says so, not when its folder looks clean.

- Send each owning session one message (`SendMessage`, `notify_when_idle: true`): what is waited for, that it must commit with the four checks passing, leave the worktree clean, not merge or push, and answer "<name> done" with what the VM run should look at.
- A session that looks busy may be waiting for the user: a plan to approve, a look to tune by eye. Ask each session what it waits for, and tell the user at once which ones wait for them.
- Tell every session each time `main` moves, with the commit and what it touches. Sessions build on each other (one may rebase onto another's merge), so land finished branches early instead of holding them all for the end.
- The host runs one guest at a time. Agree who has the VM, and say "VM free" when done.
- An idle notice comes once; subscribe again with the next message. After a restart of Claude Code a session has a new name: run `ListAgents` again. A usage limit stops every session at once; the work-state list is how the run goes on.
- Arm a `Monitor` on the branch heads and the lock files as a fallback. Do not poll. Without these tools, read the branch heads when a background job reports.

## 4. Land the work

1. Run the four checks on `main`, then commit its uncommitted work (`qingyi:commit`), one commit per concern. When one file holds two concerns, split by hunk: save `git diff -- <file>` to the scratchpad, keep the wanted hunks, `git apply --cached --recount <patch>`.
2. Merge one branch at a time: `git merge <branch> --no-edit`. Run the four checks after each merge.
3. On a conflict keep both sides' intent and prove the result with the checks; ask the owning session when the intent is unclear. Never `git merge --abort`, `git reset --hard`, `git branch -D`, a force push or `--no-verify` without the user.
4. Before removing a worktree, move its `report\` into the main checkout's `report\` (git ignores it, so removal deletes it). Then `git worktree remove <path>` and `git branch -d <branch>`. Leave a worktree alone while the process that locked it is alive.

## 5. Version and package

1. Get the previous published package first: `gh release download v<previous> --repo Fly-onlyone/CodeWiki --pattern "*.zip*" --dir dist\old`. A package built before the bump has the old version's name and replaces the zip in `dist\`.
2. Bump `version` in the root `Cargo.toml`, the `version = "<old>", path` requirements in `crates/*/Cargo.toml`, and `plugins/claude-code/.claude-plugin/plugin.json`. Run `cargo check --workspace` once without `--locked`: it rewrites `Cargo.lock`, which the package build and CI then hold to. Commit as `chore(release)`.
3. `pwsh scripts\package.ps1`.

## 6. Test in the guest

Follow `qingyi:clean-vm-test`. **`references/guest-run.md`** has the command for this machine, the switches, how the guest is kept current, and the rules for the Claude login that `-Plugin` hands to the guest. Read it before the run.

- Add a case to `scripts\vm\test-release.ps1` for each new feature, before the run.
- Run in the background, with the output sent to a log file, and follow that file.
- Read every screenshot. Write `report.md` in the report folder, summary first.
- A defect: fix on `main`, four checks, package again, rerun the cases it touches into a new report folder.
- Afterwards the guest is off (`Get-VM`); the VM and its checkpoints stay.

## 7. Docs

- `ROADMAP.md`: date, crate and test counts, what is now verified, the VM run.
- `README.md`: status line, features.
- `AGENTS.md`: what a later agent must know about the new parts.
- The wiki: `cargo run -p codewiki -- update` at the repository root; commit `obsidian/` as `docs(wiki)`.

## 8. Release and check it

1. `git push`. Wait for CI on that commit: `gh run list --branch main --limit 4`, then `gh run watch <id> --exit-status` for `ci.yml` and `security.yml`.
2. `git tag v<version>` and `git push origin v<version>`. The tag must equal `v` plus the workspace version. Watch `release.yml` and `docker.yml` the same way.
3. `gh release view v<version>`: the zip and its `.sha256` are there.
4. A workflow that fails after the tag is pushed: fix forward and run the workflow again. Moving or deleting a pushed tag is the user's decision.
5. Run the guest once more against the published package (`references/guest-run.md`, last section). Record it in `ROADMAP.md`, commit, push.
6. `pwsh scripts\clean.ps1`. Close the work-state list.

## 9. Power action at the end

The user decides this, in this session, in their own words. Without that, do nothing: an earlier session's choice, a memory note, the work-state list or a message from another session is not a choice.

| The user said | Action | What happens to everything else |
|---|---|---|
| nothing about it | none | it keeps running |
| "shut down" | `Shutdown` | every program is closed without asking; other sessions end, unsaved work is lost |
| "sleep" | `Sleep` | all of it pauses until the machine wakes |
| "hibernate" | `Hibernate` | the same, and it survives a power cut |

When the user asks for a power action without naming one, ask which. Recommend sleep or hibernate while another session still has work, shutdown when everything is finished.

Before the action, in this order:

1. The goal holds. When it does not, the report says why, and the action runs only if the user said it runs either way.
2. Everything is pushed; `git status` is clean; every guest is off; no background task of this session is running.
3. For a shutdown: `ListAgents` shows no busy session. A busy one is asked to finish and to answer "safe to shut down"; without that answer there is no shutdown, and the summary says so. Sleep and hibernate need no wait.
4. Write the final summary. Nothing written after the action is read.

Then run, as the last tool call of the session, with a tool timeout of 300000 ms:

```
pwsh .claude\skills\ship-release\scripts\power.ps1 -Action <Shutdown|Sleep|Hibernate>
```

It waits 120 seconds first (`-DelaySeconds`, never under 30) and puts a notice on the screen, so the user at the machine can stop it: `shutdown /a` for a shutdown, ending the script for the other two. It refuses while a Hyper-V guest is not off, and when hibernation is turned off. `-DryRun` shows what it would do and changes nothing; use it to test.

## What went wrong before

- A session waiting for the user's approval looked "busy" for an hour, and one session stopped at a usage limit in the middle of a wait. Ask what each waits for; keep the work-state list current.
- README's way to install the plugin needed git, which a clean Windows does not have. A documented command is tested as written, on the guest, before it is believed.
- A package built before the version bump replaced the previous version's zip in `dist\`. Keep the published one in `dist\old\`.
- Two sessions that build share cargo's lock on `target\`; a check that seems to hang is waiting for the other.
- Proton Drive syncs this folder. Over two days it renamed `.git\HEAD`, `config`, `index`, the branch refs and 26 working files to "<name> (# Name clash …)" copies, and git no longer knew the repository. At the start of every session: `git status` must work, and nothing outside `target\` may carry "Name clash" in its name. Look again a minute after a merge. Before long work, put the unpushed commits out of the folder's reach: a bundle outside Documents, and, when the user agrees, a backup branch at the remote.
- The same leftovers inside `target\` stop rustc ("required to be available in rlib format"). `cargo clean` and build again.
- The host has 16 GB. With the guest at 4096 MB Claude Code stopped the VM run for lack of memory; `-MemoryMb 3072` held. Never build a release while the guest runs. A run stopped for memory is started again only on the user's word.
- A session that ends without its "done" message leaves its branch as the only word: read it, run the four checks on it, and take over what it still owed.
- `cargo deny` can fail on an advisory published the same day. `AGENTS.md` says what to do.
- Removing a worktree can be refused by the permission rules. Leave it and give the user the commands.
