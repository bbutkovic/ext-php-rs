use std::{path::PathBuf, process::Command};

use anyhow::{bail, Context, Error, Ok, Result};

use crate::{find_executable, path_from_env, PHPInfo, PHPProvider};

pub struct Provider<'a> {
    info: &'a PHPInfo,
}

impl<'a> Provider<'a> {
    /// Runs `php-config` with one argument, returning the stdout.
    fn php_config(&self, arg: &str) -> Result<String> {
        let cmd = Command::new(self.find_bin()?)
            .arg(arg)
            .output()
            .context("Failed to run `php-config`")?;
        let stdout = String::from_utf8_lossy(&cmd.stdout);
        if !cmd.status.success() {
            let stderr = String::from_utf8_lossy(&cmd.stderr);
            bail!("Failed to run `php-config`: {} {}", stdout, stderr);
        }
        Ok(stdout.to_string())
    }

    fn find_bin(&self) -> Result<PathBuf> {
        // If path is given via env, it takes priority.
        if let Some(path) = path_from_env("PHP_CONFIG") {
            if !path.try_exists()? {
                // If path was explicitly given and it can't be found, this is a hard error
                bail!("php-config executable not found at {:?}", path);
            }
            return Ok(path);
        }
        find_executable("php-config").with_context(|| {
            "Could not find `php-config` executable. \
            Please ensure `php-config` is in your PATH or the \
            `PHP_CONFIG` environment variable is set."
        })
    }
}

impl<'a> PHPProvider<'a> for Provider<'a> {
    fn from_info(info: &'a PHPInfo) -> Result<Self> {
        Ok(Self { info })
    }

    fn get_includes(&self) -> Result<Vec<PathBuf>> {
        match self.info {
            PHPInfo::FromCommand(_) => Ok(self
                .php_config("--includes")?
                .split(' ')
                .map(|s| s.trim_start_matches("-I"))
                .map(PathBuf::from)
                .collect()),
            PHPInfo::FromEnv => self
                .info
                .get_key("includes")
                .map(|includes| {
                    includes
                        .as_str()
                        .split(",")
                        .map(|include| PathBuf::from(include))
                        .collect()
                })
                .ok_or(Error::msg("could not find includes in env")),
        }
    }

    fn get_defines(&self) -> Result<Vec<(String, String)>> {
        Ok(match self.info {
            PHPInfo::FromEnv => self
                .info
                .get_key("defines")
                .map(|defines| {
                    defines
                        .as_str()
                        .split(",")
                        .map(|define| {
                            define
                                .split_once("=")
                                .map(|(define, value)| (define.to_string(), value.to_string()))
                                .unwrap_or_else(|| (define.to_string(), "1".to_string()))
                        })
                        .collect()
                })
                .unwrap_or(vec![]),
            _ => vec![],
        })
    }
}
