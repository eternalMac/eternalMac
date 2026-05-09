use std::process::Command;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub trait Runner {
    fn run(&self, program: &str, args: &[&str]) -> Result<Output>;
}

#[derive(Debug, Default, Clone)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<Output> {
        let output = Command::new(program)
            .args(args)
            .output()
            .with_context(|| format!("running {program}"))?;

        Ok(Output {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            success: output.status.success(),
        })
    }
}
