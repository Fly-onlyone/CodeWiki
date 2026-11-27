//! The commands of a JavaScript or TypeScript package, written for its
//! package manager (`npm`, `pnpm`, `yarn` or `bun`; see `cw_parse::jsbuild`
//! for how the manager is found).
//!
//! Every command runs from the repository root. A package that a workspace
//! lists is named with the manager's own filter, `npm -w <name>`,
//! `pnpm --filter <name>`, `yarn workspace <name>`, `bun run --filter <name>`;
//! any other package below the root is entered with `cd <folder> &&`, except
//! where npm's `--prefix` has always served (the operations note).

use cw_parse::{Manager, PmInfo};

/// A name or path as one shell argument: in double quotes when it holds a
/// character a shell reads as syntax (`@scope/name` starts a splat in
/// PowerShell).
pub fn arg(text: &str) -> String {
    let plain = text
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'));
    if plain && !text.is_empty() {
        text.to_string()
    } else {
        format!("\"{text}\"")
    }
}

/// `command` run in the folder `dir`: npm takes `--prefix`, the others a
/// change of folder.
fn in_folder(manager: Manager, dir: &str, command: &str) -> String {
    if dir.is_empty() {
        return command.to_string();
    }
    match (manager, command.strip_prefix("npm ")) {
        (Manager::Npm, Some(rest)) if !rest.starts_with("exec") => {
            format!("npm --prefix {} {rest}", arg(dir))
        }
        _ => format!("cd {} && {command}", arg(dir)),
    }
}

/// The command that installs what the lock file pins, where the lock file is.
pub fn install(pm: &PmInfo) -> String {
    in_folder(pm.manager, &pm.root, pm.install())
}

/// The command that runs the script `script` of the package in `dir`.
pub fn run_script(pm: &PmInfo, dir: &str, script: &str) -> String {
    let name = pm.member.as_deref().map(arg);
    match (pm.manager, name) {
        (Manager::Npm, Some(name)) => format!("npm -w {name} run {script}"),
        (Manager::Pnpm, Some(name)) => format!("pnpm --filter {name} run {script}"),
        (Manager::Yarn, Some(name)) => format!("yarn workspace {name} run {script}"),
        (Manager::Bun, Some(name)) => format!("bun run --filter {name} {script}"),
        (manager, None) => in_folder(manager, dir, &format!("{} run {script}", manager.name())),
    }
}

/// The command that runs a program of the package's dependencies (`vitest run
/// a.test.ts`), with paths as the package's own folder sees them. `None` when
/// the manager has no way to do that for a workspace package.
pub fn exec(pm: &PmInfo, dir: &str, program: &str) -> Option<String> {
    let name = pm.member.as_deref().map(arg);
    Some(match (pm.manager, name) {
        (Manager::Npm, Some(name)) => format!("npm -w {name} exec -- {program}"),
        (Manager::Pnpm, Some(name)) => format!("pnpm --filter {name} exec {program}"),
        (Manager::Yarn, Some(name)) => format!("yarn workspace {name} {program}"),
        (Manager::Bun, Some(_)) => return None,
        (manager, None) => {
            let command = match manager {
                Manager::Npm => format!("npx {program}"),
                Manager::Pnpm => format!("pnpm exec {program}"),
                Manager::Yarn => format!("yarn {program}"),
                Manager::Bun => format!("bunx {program}"),
            };
            if dir.is_empty() {
                command
            } else {
                format!("cd {} && {command}", arg(dir))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pm(manager: Manager, member: Option<&str>, root: &str, lock: bool) -> PmInfo {
        PmInfo {
            manager,
            berry: false,
            root: root.into(),
            lock: lock.then(|| "lock".to_string()),
            member: member.map(str::to_string),
        }
    }

    #[test]
    fn a_script_runs_the_way_each_manager_runs_one() {
        let at_root = |m| run_script(&pm(m, None, "", false), "", "build");
        assert_eq!(at_root(Manager::Npm), "npm run build");
        assert_eq!(at_root(Manager::Pnpm), "pnpm run build");
        assert_eq!(at_root(Manager::Yarn), "yarn run build");
        assert_eq!(at_root(Manager::Bun), "bun run build");
        let member = |m| run_script(&pm(m, Some("@acme/ui"), "", true), "packages/ui", "test");
        assert_eq!(member(Manager::Npm), "npm -w \"@acme/ui\" run test");
        assert_eq!(member(Manager::Pnpm), "pnpm --filter \"@acme/ui\" run test");
        assert_eq!(
            member(Manager::Yarn),
            "yarn workspace \"@acme/ui\" run test"
        );
        assert_eq!(member(Manager::Bun), "bun run --filter \"@acme/ui\" test");
        // A folder that no workspace lists is entered.
        let nested = |m| run_script(&pm(m, None, "web", false), "web", "test");
        assert_eq!(nested(Manager::Npm), "npm --prefix web run test");
        assert_eq!(nested(Manager::Pnpm), "cd web && pnpm run test");
    }

    #[test]
    fn an_install_follows_the_lock_file_where_it_lies() {
        assert_eq!(install(&pm(Manager::Npm, None, "", false)), "npm install");
        let locked = |m, root: &str| {
            let mut p = pm(m, None, root, true);
            p.berry = m == Manager::Yarn;
            install(&p)
        };
        assert_eq!(locked(Manager::Npm, ""), "npm ci");
        assert_eq!(locked(Manager::Npm, "web"), "npm --prefix web ci");
        assert_eq!(locked(Manager::Pnpm, ""), "pnpm install --frozen-lockfile");
        assert_eq!(
            locked(Manager::Yarn, "app"),
            "cd app && yarn install --immutable"
        );
        assert_eq!(locked(Manager::Bun, ""), "bun install --frozen-lockfile");
    }

    #[test]
    fn a_program_runs_through_the_managers_exec() {
        let root = |m| exec(&pm(m, None, "", true), "", "vitest run a.test.ts").unwrap();
        assert_eq!(root(Manager::Npm), "npx vitest run a.test.ts");
        assert_eq!(root(Manager::Pnpm), "pnpm exec vitest run a.test.ts");
        assert_eq!(root(Manager::Yarn), "yarn vitest run a.test.ts");
        assert_eq!(root(Manager::Bun), "bunx vitest run a.test.ts");
        let member = |m| exec(&pm(m, Some("web"), "", true), "apps/web", "jest a.test.ts");
        assert_eq!(
            member(Manager::Npm).unwrap(),
            "npm -w web exec -- jest a.test.ts"
        );
        assert_eq!(
            member(Manager::Pnpm).unwrap(),
            "pnpm --filter web exec jest a.test.ts"
        );
        assert_eq!(
            member(Manager::Yarn).unwrap(),
            "yarn workspace web jest a.test.ts"
        );
        assert_eq!(member(Manager::Bun), None);
        assert_eq!(
            exec(&pm(Manager::Npm, None, "", false), "web", "jest a.test.ts").unwrap(),
            "cd web && npx jest a.test.ts"
        );
    }
}
