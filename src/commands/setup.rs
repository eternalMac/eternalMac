use anyhow::Result;
use clap::Subcommand;

use crate::app::context::AppContext;
use crate::setup::client::{
    apply_client_setup_with_preflight, preflight_client_setup, ClientSetupInput, SyncRootInput,
};
use crate::setup::prompts::{prompt_server_dns, prompt_server_ssh_user, prompt_sync_roots};
use crate::setup::server::apply_server_setup;

const DEFAULT_SERVER_HOST_LABEL: &str = "mac-mini";

#[derive(Debug, Clone, Subcommand)]
pub enum SetupCommand {
    #[command(about = "Configure this machine as the setup server")]
    Server,
    #[command(about = "Configure this machine as a setup client")]
    Client {
        #[arg(
            long,
            help = "Override the server DNS name to pair with",
            value_name = "SERVER"
        )]
        server: Option<String>,
    },
}

pub fn run_server() -> Result<()> {
    let context = AppContext::from_env()?;
    let summary = apply_server_setup(
        &context.paths,
        &context.store,
        &context.runner,
        DEFAULT_SERVER_HOST_LABEL.into(),
    )?;

    println!("Server setup complete.");
    println!("Server DNS: {}", summary.dns_name);
    println!("Default session: {}", summary.default_session);
    println!(
        "Next step: run `eternalMac setup client --server {}` on your client machine.",
        summary.dns_name
    );
    Ok(())
}

pub fn run_client(server_override: Option<String>) -> Result<()> {
    let context = AppContext::from_env()?;
    let (preflight, input) = collect_client_setup_request(
        server_override,
        std::env::var("USER").ok(),
        || preflight_client_setup(&context.runner),
        prompt_server_dns,
        prompt_server_ssh_user,
        prompt_sync_roots,
    )?;
    let summary = apply_client_setup_with_preflight(
        &context.paths,
        &context.store,
        &context.runner,
        preflight,
        input,
    )?;

    println!("Client setup complete.");
    println!("Paired server: {}", summary.paired_server);
    println!("Sync roots: {}", summary.sync_names.join(", "));
    println!("Next step: run `eternalMac attach` to start working.");
    Ok(())
}

fn collect_client_setup_request<PF, PD, PR, T>(
    server_override: Option<String>,
    ssh_user_prefill: Option<String>,
    preflight: PF,
    prompt_dns: PD,
    prompt_user: impl FnOnce(Option<String>) -> Result<String>,
    prompt_roots: PR,
) -> Result<(T, ClientSetupInput)>
where
    PF: FnOnce() -> Result<T>,
    PD: FnOnce(Option<String>) -> Result<String>,
    PR: FnOnce(&str, &str) -> Result<Vec<SyncRootInput>>,
{
    let preflight = preflight()?;
    let paired_server = prompt_dns(server_override)?;
    let server_ssh_user = prompt_user(ssh_user_prefill)?;
    let sync_roots = prompt_roots(&server_ssh_user, &paired_server)?;

    Ok((
        preflight,
        ClientSetupInput {
            paired_server,
            server_ssh_user,
            sync_roots,
        },
    ))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use anyhow::{anyhow, Result};

    use super::collect_client_setup_request;
    use crate::setup::client::SyncRootInput;

    #[test]
    fn client_setup_request_runs_preflight_before_prompts() {
        let calls = RefCell::new(Vec::new());

        let (preflight, input) = collect_client_setup_request(
            Some("override.ts.net".into()),
            Some("devuser".into()),
            || {
                calls.borrow_mut().push("preflight");
                Ok("ready")
            },
            |server_override| {
                calls.borrow_mut().push("prompt-dns");
                assert_eq!(server_override.as_deref(), Some("override.ts.net"));
                Ok("mac-mini.example.ts.net".into())
            },
            |user_override| {
                calls.borrow_mut().push("prompt-user");
                assert_eq!(user_override.as_deref(), Some("devuser"));
                Ok("devuser".into())
            },
            |server_user, server_dns| {
                calls.borrow_mut().push("prompt-syncs");
                assert_eq!(server_user, "devuser");
                assert_eq!(server_dns, "mac-mini.example.ts.net");
                Ok(vec![SyncRootInput {
                    name: "project".into(),
                    local: "/Users/me/project".into(),
                    remote: "devuser@mac-mini.example.ts.net:~/project".into(),
                    ignore_paths: vec![],
                    kind: None,
                    label: None,
                }])
            },
        )
        .unwrap();

        assert_eq!(preflight, "ready");
        assert_eq!(
            calls.into_inner(),
            vec!["preflight", "prompt-dns", "prompt-user", "prompt-syncs"]
        );
        assert_eq!(input.paired_server, "mac-mini.example.ts.net");
        assert_eq!(input.server_ssh_user, "devuser");
        assert_eq!(input.sync_roots.len(), 1);
        assert_eq!(input.sync_roots[0].name, "project");
    }

    #[test]
    fn client_setup_request_stops_before_prompts_when_preflight_fails() {
        let calls = RefCell::new(Vec::new());

        let error = collect_client_setup_request(
            None,
            Some("devuser".into()),
            || -> Result<()> {
                calls.borrow_mut().push("preflight");
                Err(anyhow!("tailscale missing"))
            },
            |_server_override| {
                calls.borrow_mut().push("prompt-dns");
                Ok("mac-mini.example.ts.net".into())
            },
            |_user_override| {
                calls.borrow_mut().push("prompt-user");
                Ok("devuser".into())
            },
            |_server_user, _server_dns| {
                calls.borrow_mut().push("prompt-syncs");
                Ok(vec![])
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("tailscale missing"));
        assert_eq!(calls.into_inner(), vec!["preflight"]);
    }
}
