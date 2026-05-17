use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Paths {
    pub home_dir: PathBuf,
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub ssh_dir: PathBuf,
    pub ssh_config_file: PathBuf,
    pub launch_agents_dir: PathBuf,
    pub server_plist: PathBuf,
    pub client_plist: PathBuf,
    pub state_dir: PathBuf,
    pub state_file: PathBuf,
    pub log_dir: PathBuf,
}

impl Paths {
    pub fn new(home: PathBuf) -> Self {
        let home_dir = home;
        let config_dir = home_dir.join(".config").join("eternalmac");
        let ssh_dir = home_dir.join(".ssh");
        let ssh_config_file = ssh_dir.join("config");
        let launch_agents_dir = home_dir.join("Library").join("LaunchAgents");
        let state_dir = home_dir
            .join("Library")
            .join("Application Support")
            .join("eternalmac");
        let log_dir = home_dir.join("Library").join("Logs").join("eternalmac");
        Self {
            home_dir,
            config_file: config_dir.join("config.toml"),
            config_dir,
            ssh_dir,
            ssh_config_file,
            server_plist: launch_agents_dir.join("com.eternalmac.server.plist"),
            client_plist: launch_agents_dir.join("com.eternalmac.client.plist"),
            launch_agents_dir,
            state_file: state_dir.join("state.json"),
            state_dir,
            log_dir,
        }
    }
}
