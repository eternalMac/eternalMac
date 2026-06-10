use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Context, Result};

#[derive(Debug, Clone)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub exit_code: Option<i32>,
}

pub trait Runner {
    fn run(&self, program: &str, args: &[String]) -> Result<Output>;

    fn run_interactive(&self, program: &str, args: &[String]) -> Result<Output> {
        self.run(program, args)
    }
}

pub fn command_failed(program: &str, args: &[String], output: &Output) -> anyhow::Error {
    let mut message = format!("command failed: {program} {}", args.join(" "));
    let stdout = output.stdout.trim();
    let stderr = output.stderr.trim();

    if !stdout.is_empty() {
        message.push_str(&format!("; stdout: {stdout}"));
    }
    if !stderr.is_empty() {
        message.push_str(&format!("; stderr: {stderr}"));
    }

    anyhow!(message)
}

#[derive(Debug, Default, Clone)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<Output> {
        let candidates = candidate_program_paths(program);
        let mut last_error = None;

        for candidate in &candidates {
            match Command::new(candidate).args(args).output() {
                Ok(output) => {
                    return Ok(Output {
                        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                        success: output.status.success(),
                        exit_code: output.status.code(),
                    });
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    last_error = Some(error);
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("running {program}"));
                }
            }
        }

        let searched = candidates.join(", ");
        Err(last_error
            .unwrap_or_else(|| std::io::Error::new(ErrorKind::NotFound, "command not found")))
        .with_context(|| format!("running {program}; searched {searched}"))
    }

    fn run_interactive(&self, program: &str, args: &[String]) -> Result<Output> {
        let candidates = candidate_program_paths(program);
        let mut last_error = None;

        for candidate in &candidates {
            match Command::new(candidate).args(args).status() {
                Ok(status) => {
                    return Ok(Output {
                        stdout: String::new(),
                        stderr: String::new(),
                        success: status.success(),
                        exit_code: status.code(),
                    });
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    last_error = Some(error);
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("running {program}"));
                }
            }
        }

        let searched = candidates.join(", ");
        Err(last_error
            .unwrap_or_else(|| std::io::Error::new(ErrorKind::NotFound, "command not found")))
        .with_context(|| format!("running {program}; searched {searched}"))
    }
}

fn candidate_program_paths(program: &str) -> Vec<String> {
    if Path::new(program).components().count() > 1 {
        return vec![program.to_string()];
    }

    let mut candidates = vec![
        program.to_string(),
        format!("/opt/homebrew/bin/{program}"),
        format!("/usr/local/bin/{program}"),
    ];

    if program == "tailscale" {
        candidates.push("/Applications/Tailscale.app/Contents/MacOS/Tailscale".to_string());
    }

    if program == "ping" {
        candidates.push("/sbin/ping".to_string());
    }

    dedupe(candidates)
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::with_capacity(values.len());
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::candidate_program_paths;

    #[test]
    fn candidate_program_paths_cover_homebrew_locations_for_named_commands() {
        assert_eq!(
            candidate_program_paths("tmux"),
            vec![
                "tmux".to_string(),
                "/opt/homebrew/bin/tmux".to_string(),
                "/usr/local/bin/tmux".to_string(),
            ]
        );
    }

    #[test]
    fn candidate_program_paths_include_tailscale_app_binary() {
        assert_eq!(
            candidate_program_paths("tailscale"),
            vec![
                "tailscale".to_string(),
                "/opt/homebrew/bin/tailscale".to_string(),
                "/usr/local/bin/tailscale".to_string(),
                "/Applications/Tailscale.app/Contents/MacOS/Tailscale".to_string(),
            ]
        );
    }

    #[test]
    fn candidate_program_paths_include_sbin_ping() {
        assert_eq!(
            candidate_program_paths("ping"),
            vec![
                "ping".to_string(),
                "/opt/homebrew/bin/ping".to_string(),
                "/usr/local/bin/ping".to_string(),
                "/sbin/ping".to_string(),
            ]
        );
    }

    #[test]
    fn candidate_program_paths_do_not_expand_explicit_paths() {
        assert_eq!(
            candidate_program_paths("/Users/me/bin/custom"),
            vec!["/Users/me/bin/custom".to_string()]
        );
    }
}
