use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub path: String,
    pub name: String,
    pub git_repo: Option<String>,
    pub remotes: Vec<Environment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    pub ssh_user: String,
    pub ssh_host: String,
    pub app_name: String,
    pub domain: Option<String>,
    pub app_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub name: String,
    pub state: String,
    pub status: String,
    pub image: String,
    pub ports: Vec<String>,
    pub health: Option<String>,
}
