use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::app::paths::Paths;
use crate::config::store::Store;
use crate::model::config::{ClientConfig, Config, Role, SessionConfig, SyncPairConfig};
use crate::model::state::{State, SyncPairState};
use crate::platform::launchd::{write_plist, Definition};
use crate::process::runner::{Output, Runner};
use crate::tooling::brew::{install_cask_args, install_formula_args, tap_args};
use crate::tooling::dependencies::{required_formulae, MUTAGEN_TAP, TAILSCALE_CASK};
use crate::tooling::mutagen::{
    build_create_args_with_ignores, list_args as mutagen_list_args, parse_list_output,
    ListedSession, SYNC_MODE_TWO_WAY_RESOLVED,
};
use crate::tooling::ssh::{
    batch_login_check_args, etterminal_path_probe_args, interactive_authorize_key_args,
    managed_identity_paths, port_probe_args, render_managed_host_block, upsert_managed_host_block,
    validate_ssh_host, validate_ssh_user,
};
use crate::tooling::tailscale::{parse_status_json, status_args};

const DAEMON_PATH: &str = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRootInput {
    pub name: String,
    pub local: String,
    pub remote: String,
    pub ignore_paths: Vec<String>,
    pub kind: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSetupInput {
    pub paired_server: String,
    pub server_ssh_user: String,
    pub sync_roots: Vec<SyncRootInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSetupSummary {
    pub paired_server: String,
    pub sync_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientPreflight {
    tailscale_ok: bool,
}

fn run_checked<R: Runner>(runner: &R, program: &str, args: &[String]) -> Result<Output> {
    let output = runner.run(program, args)?;
    if output.success {
        return Ok(output);
    }

    let mut message = format!("command failed: {program} {}", args.join(" "));
    if !output.stderr.trim().is_empty() {
        message.push_str(&format!("; stderr: {}", output.stderr.trim()));
    }

    Err(anyhow!(message))
}

fn is_expected_unload_absence(output: &Output) -> bool {
    let combined = format!(
        "{}\n{}",
        output.stdout.to_lowercase(),
        output.stderr.to_lowercase()
    );

    combined.contains("could not find specified service")
        || combined.contains("could not find service")
        || combined.contains("not loaded")
        || combined.contains("no such process")
        || combined.contains("not found")
}

fn run_unload_checked<R: Runner>(runner: &R, plist_path: &std::path::Path) -> Result<()> {
    let args = vec![
        "unload".into(),
        "-w".into(),
        plist_path.display().to_string(),
    ];
    let output = runner.run("launchctl", &args)?;
    if output.success || is_expected_unload_absence(&output) {
        return Ok(());
    }

    let mut message = format!("command failed: launchctl {}", args.join(" "));
    if !output.stderr.trim().is_empty() {
        message.push_str(&format!("; stderr: {}", output.stderr.trim()));
    }
    if !output.stdout.trim().is_empty() {
        message.push_str(&format!("; stdout: {}", output.stdout.trim()));
    }

    Err(anyhow!(message))
}

fn persist_config_and_state(
    paths: &Paths,
    store: &Store,
    config: &Config,
    state: &State,
) -> Result<()> {
    let prior_config = match fs::read(&paths.config_file) {
        Ok(previous) => Some(previous),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            return Err(anyhow!(
                "failed to snapshot existing config before setup write: {error}"
            ));
        }
    };
    store.save_config(config)?;

    if let Err(state_error) = store.save_state(state) {
        let rollback = match prior_config {
            Some(previous) => fs::write(&paths.config_file, previous),
            None => match fs::remove_file(&paths.config_file) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            },
        };

        return match rollback {
            Ok(()) => Err(anyhow!(
                "failed to persist setup state after config write: {state_error}"
            )),
            Err(rollback_error) => Err(anyhow!(
                "failed to persist setup state after config write: {state_error}; config rollback failed: {rollback_error}"
            )),
        };
    }

    Ok(())
}

fn has_matching_existing_sync(
    existing_syncs: &[ListedSession],
    root: &SyncRootInput,
) -> Result<bool> {
    let matching_name = existing_syncs
        .iter()
        .filter(|session| session.name == root.name)
        .collect::<Vec<_>>();

    if matching_name.is_empty() {
        return Ok(false);
    }

    if matching_name.len() > 1 {
        return Err(anyhow!(
            "multiple mutagen syncs already use the name `{}`; remove the duplicates before rerunning `eternalMac setup client`",
            root.name
        ));
    }

    let existing = matching_name[0];
    if existing.alpha_url.as_deref() == Some(root.local.as_str())
        && existing.beta_url.as_deref() == Some(root.remote.as_str())
    {
        return Ok(true);
    }

    Err(anyhow!(
        "existing mutagen sync `{}` does not match requested endpoints; expected {} <-> {}, found {} <-> {}",
        root.name,
        root.local,
        root.remote,
        existing.alpha_url.as_deref().unwrap_or("unknown"),
        existing.beta_url.as_deref().unwrap_or("unknown")
    ))
}

fn current_executable_path() -> Result<String> {
    Ok(std::env::current_exe()
        .context("resolving current executable path for client launch agent")?
        .display()
        .to_string())
}

fn verify_server_ssh_port<R: Runner>(runner: &R, server_host: &str) -> Result<()> {
    let args = port_probe_args(server_host);
    let output = runner.run("nc", &args)?;
    if output.success {
        return Ok(());
    }

    let mut message = format!(
        "server SSH port 22 is unreachable on {server_host}; enable Remote Login on the Mac Mini and rerun `eternalMac setup client`"
    );
    if !output.stderr.trim().is_empty() {
        message.push_str(&format!("; stderr: {}", output.stderr.trim()));
    }
    if !output.stdout.trim().is_empty() {
        message.push_str(&format!("; stdout: {}", output.stdout.trim()));
    }

    Err(anyhow!(message))
}

fn verify_server_batch_ssh<R: Runner>(runner: &R, server_host: &str) -> Result<Output> {
    let args = batch_login_check_args(server_host);
    runner.run("ssh", &args)
}

fn ensure_ssh_directory(paths: &Paths) -> Result<()> {
    fs::create_dir_all(&paths.ssh_dir)?;
    let permissions = fs::Permissions::from_mode(0o700);
    fs::set_permissions(&paths.ssh_dir, permissions)?;
    Ok(())
}

fn ensure_managed_keypair(
    paths: &Paths,
    server_host: &str,
    server_user: &str,
) -> Result<crate::tooling::ssh::ManagedIdentityPaths> {
    let identity_paths = managed_identity_paths(&paths.ssh_dir, server_host, server_user);
    if identity_paths.private_key_path.exists() && identity_paths.public_key_path.exists() {
        return Ok(identity_paths);
    }

    ensure_ssh_directory(paths)?;

    if identity_paths.private_key_path.exists() && !identity_paths.public_key_path.exists() {
        let output = Command::new("ssh-keygen")
            .args([
                "-y",
                "-f",
                identity_paths
                    .private_key_path
                    .to_str()
                    .context("private key path is not valid unicode")?,
            ])
            .output()
            .context("running ssh-keygen to reconstruct managed public key")?;
        if !output.status.success() {
            bail!(
                "command failed: ssh-keygen -y -f {}; stderr: {}",
                identity_paths.private_key_path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        fs::write(
            &identity_paths.public_key_path,
            String::from_utf8_lossy(&output.stdout).trim(),
        )?;
        fs::set_permissions(
            &identity_paths.public_key_path,
            fs::Permissions::from_mode(0o644),
        )?;
        return Ok(identity_paths);
    }

    if !identity_paths.private_key_path.exists() && identity_paths.public_key_path.exists() {
        fs::remove_file(&identity_paths.public_key_path)?;
    }

    let comment = format!("eternalmac-client@{server_host}");
    let output = Command::new("ssh-keygen")
        .args([
            "-t",
            "ed25519",
            "-N",
            "",
            "-f",
            identity_paths
                .private_key_path
                .to_str()
                .context("managed private key path is not valid unicode")?,
            "-C",
            &comment,
        ])
        .output()
        .context("running ssh-keygen for eternalmac managed key")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "command failed: ssh-keygen -t ed25519 -N '' -f {}; stderr: {}",
            identity_paths.private_key_path.display(),
            stderr.trim()
        );
    }

    Ok(identity_paths)
}

fn upsert_managed_ssh_config(
    paths: &Paths,
    server_host: &str,
    server_user: &str,
    identity_file: &std::path::Path,
) -> Result<()> {
    ensure_ssh_directory(paths)?;

    let existing = match fs::read_to_string(&paths.ssh_config_file) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(anyhow!(
                "failed to read ssh config at {}: {error}",
                paths.ssh_config_file.display()
            ));
        }
    };
    let block = render_managed_host_block(
        server_host,
        server_user,
        &identity_file.display().to_string(),
    );
    let updated = upsert_managed_host_block(&existing, server_host, &block);
    if updated != existing {
        fs::write(&paths.ssh_config_file, updated)?;
        fs::set_permissions(&paths.ssh_config_file, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

fn authorize_server_public_key_interactive(
    server_user: &str,
    server_host: &str,
    public_key: &str,
) -> Result<()> {
    println!(
        "A one-time SSH password prompt will open to authorize the dedicated eternalMac key on {}@{}.",
        server_user, server_host
    );

    let args = interactive_authorize_key_args(server_user, server_host, public_key);
    let status = Command::new("ssh")
        .args(&args)
        .status()
        .context("running ssh to authorize eternalmac managed key")?;
    if status.success() {
        return Ok(());
    }

    Err(anyhow!(
        "unable to authorize the dedicated eternalMac SSH key on {}@{}; rerun `eternalMac setup client` and complete the password prompt",
        server_user,
        server_host
    ))
}

fn ensure_noninteractive_server_ssh_with<R, A>(
    paths: &Paths,
    runner: &R,
    server_host: &str,
    server_user: &str,
    authorize_public_key: A,
) -> Result<()>
where
    R: Runner,
    A: FnOnce(&str, &str, &str) -> Result<()>,
{
    let identity_paths = ensure_managed_keypair(paths, server_host, server_user)?;
    upsert_managed_ssh_config(
        paths,
        server_host,
        server_user,
        &identity_paths.private_key_path,
    )?;

    let batch_check = verify_server_batch_ssh(runner, server_host)?;
    if batch_check.success {
        return Ok(());
    }

    let public_key = fs::read_to_string(&identity_paths.public_key_path).with_context(|| {
        format!(
            "reading managed public key from {}",
            identity_paths.public_key_path.display()
        )
    })?;
    authorize_public_key(server_user, server_host, public_key.trim_end())?;

    let verification = verify_server_batch_ssh(runner, server_host)?;
    if verification.success {
        return Ok(());
    }

    let mut message = format!(
        "non-interactive SSH is still unavailable for {}@{} after key authorization; rerun `eternalMac setup client` once direct `ssh {}` works without a password prompt",
        server_user, server_host, server_host
    );
    if !verification.stderr.trim().is_empty() {
        message.push_str(&format!("; stderr: {}", verification.stderr.trim()));
    }
    if !verification.stdout.trim().is_empty() {
        message.push_str(&format!("; stdout: {}", verification.stdout.trim()));
    }

    Err(anyhow!(message))
}

fn ensure_noninteractive_server_ssh<R: Runner>(
    paths: &Paths,
    runner: &R,
    server_host: &str,
    server_user: &str,
) -> Result<()> {
    ensure_noninteractive_server_ssh_with(
        paths,
        runner,
        server_host,
        server_user,
        authorize_server_public_key_interactive,
    )
}

fn detect_remote_etterminal_path<R: Runner>(runner: &R, server_host: &str) -> Result<String> {
    let args = etterminal_path_probe_args(server_host);
    let output = runner.run("ssh", &args)?;
    if output.success {
        if let Some(path) = output
            .stdout
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
        {
            return Ok(path.to_string());
        }
    }

    let mut message = format!(
        "unable to find etterminal on {server_host}; install Eternal Terminal on the Mac Mini and rerun `eternalMac setup client`"
    );
    if !output.stderr.trim().is_empty() {
        message.push_str(&format!("; stderr: {}", output.stderr.trim()));
    }
    if !output.stdout.trim().is_empty() {
        message.push_str(&format!("; stdout: {}", output.stdout.trim()));
    }

    Err(anyhow!(message))
}

pub(crate) fn preflight_client_setup<R: Runner>(runner: &R) -> Result<ClientPreflight> {
    let tap_args = tap_args(MUTAGEN_TAP);
    run_checked(runner, "brew", &tap_args)?;

    let formulae = required_formulae();
    if let Some(args) = install_formula_args(&formulae) {
        run_checked(runner, "brew", &args)?;
    }

    let cask_args = install_cask_args(TAILSCALE_CASK);
    run_checked(runner, "brew", &cask_args)?;

    let tailscale_args = status_args();
    let tailscale_status = run_checked(runner, "tailscale", &tailscale_args)?;
    let parsed_status = parse_status_json(&tailscale_status.stdout)?;

    Ok(ClientPreflight {
        tailscale_ok: parsed_status.backend_state == "Running",
    })
}

pub(crate) fn apply_client_setup_with_preflight<R: Runner>(
    paths: &Paths,
    store: &Store,
    runner: &R,
    preflight: ClientPreflight,
    input: ClientSetupInput,
) -> Result<ClientSetupSummary> {
    validate_ssh_host(&input.paired_server)
        .map_err(|error| anyhow!("invalid paired server: {error}"))?;
    validate_ssh_user(&input.server_ssh_user)
        .map_err(|error| anyhow!("invalid server SSH username: {error}"))?;

    let tailscale_ok = preflight.tailscale_ok;
    verify_server_ssh_port(runner, &input.paired_server)?;
    ensure_noninteractive_server_ssh(paths, runner, &input.paired_server, &input.server_ssh_user)?;
    let server_etterminal_path = detect_remote_etterminal_path(runner, &input.paired_server)?;

    let sync_pairs = input
        .sync_roots
        .iter()
        .map(|root| SyncPairConfig {
            name: root.name.clone(),
            local: root.local.clone(),
            remote: root.remote.clone(),
            mode: SYNC_MODE_TWO_WAY_RESOLVED.into(),
            ignore_paths: root.ignore_paths.clone(),
            kind: root.kind.clone(),
            label: root.label.clone(),
        })
        .collect::<Vec<_>>();

    let config = Config {
        role: Role::Client,
        server: None,
        client: Some(ClientConfig {
            paired_server: input.paired_server.clone(),
            server_ssh_user: Some(input.server_ssh_user.clone()),
            server_etterminal_path: Some(server_etterminal_path),
            pinned: vec![],
            sync_pairs: sync_pairs.clone(),
        }),
        session: SessionConfig { auto_attach: true },
    };
    let mut sync_states = sync_pairs
        .iter()
        .map(|pair| SyncPairState {
            name: pair.name.clone(),
            local: pair.local.clone(),
            remote: pair.remote.clone(),
            mode: pair.mode.clone(),
            ignore_paths: pair.ignore_paths.clone(),
            kind: pair.kind.clone(),
            label: pair.label.clone(),
            status: "pending".into(),
        })
        .collect::<Vec<_>>();
    persist_config_and_state(
        paths,
        store,
        &config,
        &State {
            role: Role::Client,
            tailscale_ok,
            server_reachable: false,
            healthy: false,
            summary: "client setup in progress; finalizing sync prerequisites".into(),
            tailscale_dns: None,
            daemon_healthy: false,
            daemon_heartbeat_unix: 0,
            default_session_present: false,
            known_sessions: vec![],
            syncs: sync_states.clone(),
        },
    )?;

    let existing_syncs_output = run_checked(runner, "mutagen", &mutagen_list_args())?;
    let existing_syncs = parse_list_output(&existing_syncs_output.stdout);

    let mut created_sync_count = 0usize;
    for (index, root) in input.sync_roots.iter().enumerate() {
        if !has_matching_existing_sync(&existing_syncs, root)? {
            let args = build_create_args_with_ignores(
                &root.name,
                &root.local,
                &root.remote,
                &root.ignore_paths,
            );
            run_checked(runner, "mutagen", &args)?;
        }
        sync_states[index].status = "created".into();
        created_sync_count += 1;
        store.save_state(&State {
            role: Role::Client,
            tailscale_ok,
            server_reachable: created_sync_count > 0,
            healthy: false,
            summary: "client setup in progress; finalizing sync prerequisites".into(),
            tailscale_dns: None,
            daemon_healthy: false,
            daemon_heartbeat_unix: 0,
            default_session_present: false,
            known_sessions: vec![],
            syncs: sync_states.clone(),
        })?;
    }

    let sync_names = sync_pairs
        .iter()
        .map(|pair| pair.name.clone())
        .collect::<Vec<_>>();
    let executable_path = current_executable_path()?;
    let environment_variables = BTreeMap::from([(String::from("PATH"), String::from(DAEMON_PATH))]);

    write_plist(
        &paths.client_plist,
        &Definition {
            label: "com.eternalmac.client".into(),
            program_arguments: vec![executable_path, "daemon".into(), "client".into()],
            environment_variables,
            run_at_load: true,
            keep_alive: true,
        },
    )?;

    run_unload_checked(runner, &paths.server_plist)?;

    let launchctl_args = vec![
        "load".into(),
        "-w".into(),
        paths.client_plist.display().to_string(),
    ];
    run_checked(runner, "launchctl", &launchctl_args)?;

    let server_reachable = !sync_pairs.is_empty();
    let healthy = tailscale_ok && server_reachable;
    store.save_state(&State {
        role: Role::Client,
        tailscale_ok,
        server_reachable,
        healthy,
        summary: "client setup complete; runtime health pending".into(),
        tailscale_dns: None,
        daemon_healthy: false,
        daemon_heartbeat_unix: 0,
        default_session_present: false,
        known_sessions: vec![],
        syncs: sync_states,
    })?;

    Ok(ClientSetupSummary {
        paired_server: input.paired_server,
        sync_names,
    })
}

pub fn apply_client_setup<R: Runner>(
    paths: &Paths,
    store: &Store,
    runner: &R,
    input: ClientSetupInput,
) -> Result<ClientSetupSummary> {
    let preflight = preflight_client_setup(runner)?;
    apply_client_setup_with_preflight(paths, store, runner, preflight, input)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use anyhow::Result;

    use super::ensure_noninteractive_server_ssh_with;
    use crate::app::paths::Paths;
    use crate::process::runner::{Output, Runner};
    use crate::tooling::ssh::batch_login_check_args;

    #[derive(Debug, Clone)]
    struct Stub {
        program: String,
        args: Vec<String>,
        output: Output,
    }

    #[derive(Default)]
    struct FakeRunner {
        calls: RefCell<Vec<(String, Vec<String>)>>,
        stubs: RefCell<Vec<Stub>>,
    }

    impl FakeRunner {
        fn with_stubs(stubs: Vec<Stub>) -> Self {
            Self {
                calls: RefCell::new(vec![]),
                stubs: RefCell::new(stubs),
            }
        }
    }

    impl Runner for FakeRunner {
        fn run(&self, program: &str, args: &[String]) -> Result<Output> {
            self.calls
                .borrow_mut()
                .push((program.to_string(), args.to_vec()));

            let index = self
                .stubs
                .borrow()
                .iter()
                .position(|stub| stub.program == program && stub.args.as_slice() == args);
            if let Some(index) = index {
                return Ok(self.stubs.borrow_mut().remove(index).output);
            }

            Ok(Output {
                stdout: String::new(),
                stderr: String::new(),
                success: true,
            })
        }
    }

    #[test]
    fn ensure_noninteractive_server_ssh_skips_authorization_when_batch_access_already_works() {
        let tempdir = tempfile::tempdir().unwrap();
        let paths = Paths::new(tempdir.path().to_path_buf());
        let runner = FakeRunner::default();
        let authorize_calls = RefCell::new(0usize);

        ensure_noninteractive_server_ssh_with(
            &paths,
            &runner,
            "mac-mini.example.ts.net",
            "devuser",
            |_, _, _| {
                *authorize_calls.borrow_mut() += 1;
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(*authorize_calls.borrow(), 0);
        assert!(paths.ssh_config_file.exists());
        assert!(paths
            .ssh_dir
            .join("eternalmac_devuser_mac_mini_example_ts_net_ed25519")
            .exists());
        assert!(runner.calls.borrow().iter().any(|(program, args)| {
            program == "ssh" && args == &batch_login_check_args("mac-mini.example.ts.net")
        }));
    }

    #[test]
    fn ensure_noninteractive_server_ssh_authorizes_key_when_batch_access_fails_initially() {
        let tempdir = tempfile::tempdir().unwrap();
        let paths = Paths::new(tempdir.path().to_path_buf());
        let ssh_args = batch_login_check_args("mac-mini.example.ts.net");
        let runner = FakeRunner::with_stubs(vec![
            Stub {
                program: "ssh".into(),
                args: ssh_args.clone(),
                output: Output {
                    stdout: String::new(),
                    stderr: "Permission denied".into(),
                    success: false,
                },
            },
            Stub {
                program: "ssh".into(),
                args: ssh_args.clone(),
                output: Output {
                    stdout: String::new(),
                    stderr: String::new(),
                    success: true,
                },
            },
        ]);
        let authorize_calls = RefCell::new(Vec::new());

        ensure_noninteractive_server_ssh_with(
            &paths,
            &runner,
            "mac-mini.example.ts.net",
            "devuser",
            |server_user, server_host, public_key| {
                authorize_calls.borrow_mut().push((
                    server_user.to_string(),
                    server_host.to_string(),
                    public_key.to_string(),
                ));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(authorize_calls.borrow().len(), 1);
        assert_eq!(authorize_calls.borrow()[0].0, "devuser");
        assert_eq!(authorize_calls.borrow()[0].1, "mac-mini.example.ts.net");
        assert!(authorize_calls.borrow()[0].2.starts_with("ssh-ed25519 "));
        assert_eq!(
            runner
                .calls
                .borrow()
                .iter()
                .filter(|(program, args)| {
                    program == "ssh" && args == &batch_login_check_args("mac-mini.example.ts.net")
                })
                .count(),
            2
        );
    }
}
