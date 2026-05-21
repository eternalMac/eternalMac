use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedIdentityPaths {
    pub private_key_path: PathBuf,
    pub public_key_path: PathBuf,
}

pub fn build_sync_destination(user: &str, host: &str, path: &str) -> String {
    format!("{user}@{host}:{path}")
}

pub fn port_probe_args(host: &str) -> Vec<String> {
    vec![
        "-G".into(),
        "5".into(),
        "-z".into(),
        host.into(),
        "22".into(),
    ]
}

pub fn managed_identity_paths(
    ssh_dir: &Path,
    server_host: &str,
    server_user: &str,
) -> ManagedIdentityPaths {
    let stem = format!(
        "eternalmac_{}_{}_ed25519",
        sanitize_for_filename(server_user),
        sanitize_for_filename(server_host)
    );
    let private_key_path = ssh_dir.join(stem);
    let public_key_path = ssh_dir.join(format!(
        "{}.pub",
        private_key_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("eternalmac_ed25519")
    ));

    ManagedIdentityPaths {
        private_key_path,
        public_key_path,
    }
}

pub fn render_managed_host_block(
    server_host: &str,
    server_user: &str,
    identity_file: &str,
) -> String {
    format!(
        "# >>> eternalmac {server_host} >>>\n\
Host {server_host}\n\
  HostName {server_host}\n\
  User {server_user}\n\
  IdentityFile {identity_file}\n\
  IdentitiesOnly yes\n\
  PreferredAuthentications publickey\n\
# <<< eternalmac {server_host} <<<\n"
    )
}

pub fn upsert_managed_host_block(existing: &str, server_host: &str, block: &str) -> String {
    let start_marker = format!("# >>> eternalmac {server_host} >>>");
    let end_marker = format!("# <<< eternalmac {server_host} <<<");

    if let Some(start) = existing.find(&start_marker) {
        if let Some(end_relative) = existing[start..].find(&end_marker) {
            let end = start + end_relative + end_marker.len();
            let trailing_newline = existing[end..]
                .strip_prefix('\n')
                .unwrap_or(&existing[end..]);
            let suffix = trailing_newline.trim_start_matches('\n');
            return if suffix.is_empty() {
                block.to_string()
            } else {
                format!("{block}\n{suffix}")
            };
        }
    }

    if existing.trim().is_empty() {
        block.to_string()
    } else {
        format!("{block}\n{}", existing.trim_start_matches('\n'))
    }
}

pub fn batch_login_check_args(host: &str) -> Vec<String> {
    vec![
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "StrictHostKeyChecking=accept-new".into(),
        "-o".into(),
        "ConnectTimeout=5".into(),
        host.into(),
        "true".into(),
    ]
}

pub fn etterminal_path_probe_args(host: &str) -> Vec<String> {
    vec![
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "StrictHostKeyChecking=accept-new".into(),
        "-o".into(),
        "ConnectTimeout=5".into(),
        host.into(),
        "if [ -x /opt/homebrew/bin/etterminal ]; then printf '%s\\n' /opt/homebrew/bin/etterminal; elif [ -x /usr/local/bin/etterminal ]; then printf '%s\\n' /usr/local/bin/etterminal; else command -v etterminal; fi".into(),
    ]
}

pub fn interactive_authorize_key_args(
    server_user: &str,
    server_host: &str,
    public_key: &str,
) -> Vec<String> {
    let target = format!("{server_user}@{server_host}");
    let quoted_key = quote_shell_arg(public_key);
    let remote_command = format!(
        "umask 077; mkdir -p ~/.ssh; touch ~/.ssh/authorized_keys; grep -qxF {quoted_key} ~/.ssh/authorized_keys || printf '%s\\n' {quoted_key} >> ~/.ssh/authorized_keys"
    );

    vec![
        "-o".into(),
        "StrictHostKeyChecking=accept-new".into(),
        "-o".into(),
        "PreferredAuthentications=password,keyboard-interactive".into(),
        "-o".into(),
        "PubkeyAuthentication=no".into(),
        target,
        remote_command,
    ]
}

fn sanitize_for_filename(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();

    sanitized.trim_matches('_').to_string()
}

fn quote_shell_arg(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
