use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub workspace: Workspace,
    #[serde(default)]
    pub server: ServerConfig,
    pub upstreams: Upstreams,
    pub profile: ProfileConfig,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    #[serde(default = "default_state_dir")]
    pub state_dir: PathBuf,
    #[serde(default = "default_artifact_dir")]
    pub artifact_dir: PathBuf,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            state_dir: default_state_dir(),
            artifact_dir: default_artifact_dir(),
        }
    }
}

fn default_state_dir() -> PathBuf {
    PathBuf::from(".remote-mcp-devkit/sessions")
}

fn default_artifact_dir() -> PathBuf {
    PathBuf::from(".remote-mcp-devkit/artifacts")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_listen_host")]
    pub host: String,
    #[serde(default = "default_listen_port")]
    pub port: u16,
    #[serde(default = "default_scheme")]
    pub scheme: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_listen_host(),
            port: default_listen_port(),
            scheme: default_scheme(),
        }
    }
}

fn default_listen_host() -> String {
    "localhost".to_string()
}

fn default_listen_port() -> u16 {
    8443
}

fn default_scheme() -> String {
    "https".to_string()
}

impl ServerConfig {
    pub fn base_url(&self) -> String {
        if (self.scheme == "https" && self.port == 443)
            || (self.scheme == "http" && self.port == 80)
        {
            format!("{}://{}", self.scheme, self.host)
        } else {
            format!("{}://{}:{}", self.scheme, self.host, self.port)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Upstreams {
    pub mcp: Option<Upstream>,
    /// When set, `/oauth/*` and `/.well-known/oauth-authorization-server`
    /// are proxied to this URL instead of the built-in mock AS.
    /// PRM and the 401 challenge stay in devkit.
    pub oauth: Option<Upstream>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Upstream {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(default = "default_mcp_path")]
    pub mcp_path: String,
    #[serde(default = "default_forwarded_proto")]
    pub forwarded_proto: String,
    #[serde(default)]
    pub oauth: OAuthConfig,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            mcp_path: default_mcp_path(),
            forwarded_proto: default_forwarded_proto(),
            oauth: OAuthConfig::default(),
        }
    }
}

fn default_mcp_path() -> String {
    "/mcp".to_string()
}

fn default_forwarded_proto() -> String {
    "https".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    #[serde(default = "default_oauth_client")]
    pub client: String,
    #[serde(default)]
    pub client_id_metadata_document: Option<String>,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            client: default_oauth_client(),
            client_id_metadata_document: None,
            scopes: default_scopes(),
        }
    }
}

fn default_oauth_client() -> String {
    "local-fake-client".to_string()
}

fn default_scopes() -> Vec<String> {
    vec!["mcp:read".to_string()]
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("read config {}: {}", path.display(), e))?;
        let cfg: Config = serde_yaml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("parse config {}: {}", path.display(), e))?;
        Ok(cfg)
    }

    pub fn load_or_default(path: Option<&Path>) -> anyhow::Result<Self> {
        if let Some(p) = path {
            return Self::load(p);
        }
        let default_path = Path::new("remote-mcp-devkit.yaml");
        if default_path.exists() {
            return Self::load(default_path);
        }
        Ok(Self::sample())
    }

    pub fn sample() -> Self {
        Self {
            version: 1,
            workspace: Workspace::default(),
            server: ServerConfig::default(),
            upstreams: Upstreams {
                mcp: Some(Upstream {
                    url: "http://127.0.0.1:18080".to_string(),
                }),
                oauth: None,
            },
            profile: ProfileConfig::default(),
        }
    }
}
