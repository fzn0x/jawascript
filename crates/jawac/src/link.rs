//! Links an object file with the Jawa runtime into an executable, using the
//! system C compiler as the linker driver.
//!
//! CMake bakes in the defaults when it builds `jawac` (`JAWA_DEFAULT_CC`,
//! `JAWA_RUNTIME_LIB`, `JAWA_LINK_ARGS`); the environment variables `JAWA_CC`
//! and `JAWA_RUNTIME_LIB` override them at run time.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

const RUNTIME_NAME: &str = "libjawa_rt.a";

pub fn link(object: &Path, output: &Path) -> Result<(), String> {
    let cc: OsString = env_override("JAWA_CC")
        .or_else(|| option_env!("JAWA_DEFAULT_CC").map(OsString::from))
        .unwrap_or_else(|| OsString::from("cc"));
    let runtime = runtime_library()?;

    let mut command = Command::new(&cc);
    command.arg(object).arg(&runtime);
    // The collector and system libraries, `|`-separated.
    if let Some(args) = option_env!("JAWA_LINK_ARGS") {
        command.args(args.split('|').filter(|arg| !arg.is_empty()));
    }
    command.arg("-o").arg(output);

    let result = command.output().map_err(|error| {
        format!(
            "cannot run the linker `{}`: {error}\n  = note: set JAWA_CC to a C compiler",
            cc.to_string_lossy()
        )
    })?;
    if !result.status.success() {
        return Err(format!(
            "linking failed:\n{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(())
}

/// Finds `libjawa_rt.a`: from `JAWA_RUNTIME_LIB`, next to the `jawac`
/// executable (`<prefix>/bin/jawac` and `<prefix>/lib/libjawa_rt.a`, which
/// is also the build tree's layout), or where CMake built it.
fn runtime_library() -> Result<PathBuf, String> {
    if let Some(path) = env_override("JAWA_RUNTIME_LIB") {
        return Ok(PathBuf::from(path));
    }
    if let Some(dir) = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        let candidate = dir.join("..").join("lib").join(RUNTIME_NAME);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    if let Some(path) = option_env!("JAWA_RUNTIME_LIB") {
        if Path::new(path).exists() {
            return Ok(PathBuf::from(path));
        }
    }
    Err(format!(
        "cannot find the Jawa runtime ({RUNTIME_NAME})\n  = note: set JAWA_RUNTIME_LIB to its path"
    ))
}

/// An environment variable that overrides a default; set but empty counts as
/// unset.
fn env_override(name: &str) -> Option<OsString> {
    env::var_os(name).filter(|value| !value.is_empty())
}
