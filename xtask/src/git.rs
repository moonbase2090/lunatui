// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn output(args: &[&str]) -> io::Result<String> {
    output_in(Path::new("."), args)
}

fn output_in(dir: &Path, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git").arg("-C").arg(dir).args(args).output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout).map_err(io::Error::other)
}

pub(crate) fn enter_toplevel() -> io::Result<()> {
    let toplevel = output(&["rev-parse", "--show-toplevel"])?;
    env::set_current_dir(toplevel.trim())
}

pub(crate) fn work_tree_files(dir: &Path, pattern: &str) -> io::Result<Vec<PathBuf>> {
    Ok(output_in(dir, &["ls-files", "--", pattern])?
        .lines()
        .map(PathBuf::from)
        .collect())
}

pub(crate) fn changed_files(base: &str, head: &str) -> io::Result<Vec<String>> {
    Ok(
        output(&["diff", "--name-only", &format!("{base}...{head}")])?
            .lines()
            .map(str::to_owned)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::process;

    use super::{output_in, work_tree_files};

    fn scratch_repository(name: &str) -> io::Result<PathBuf> {
        let dir = env::temp_dir().join(format!("lunatui-xtask-{name}-{}", process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(dir.join("src"))?;
        output_in(&dir, &["init", "--quiet"])?;
        fs::write(dir.join(".gitignore"), "ignored.rs\n")?;
        fs::write(dir.join("src/lib.rs"), "")?;
        fs::write(dir.join("ignored.rs"), "")?;
        fs::write(dir.join("notes.txt"), "")?;
        Ok(dir)
    }

    fn listed(dir: &Path) -> io::Result<Vec<PathBuf>> {
        let files = work_tree_files(dir, "*.rs");
        fs::remove_dir_all(dir)?;
        files
    }

    #[test]
    fn lists_unignored_sources_when_the_repository_has_no_index() -> io::Result<()> {
        let dir = scratch_repository("no-index")?;
        assert!(!dir.join(".git/index").exists());
        assert_eq!(listed(&dir)?, [PathBuf::from("src/lib.rs")]);
        Ok(())
    }

    #[test]
    fn skips_indexed_files_deleted_from_the_work_tree() -> io::Result<()> {
        let dir = scratch_repository("deleted")?;
        fs::write(dir.join("gone.rs"), "")?;
        output_in(&dir, &["add", "gone.rs", "src/lib.rs"])?;
        fs::remove_file(dir.join("gone.rs"))?;
        assert_eq!(listed(&dir)?, [PathBuf::from("src/lib.rs")]);
        Ok(())
    }
}
