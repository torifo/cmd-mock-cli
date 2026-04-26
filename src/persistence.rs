use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{docker::DockerSim, model::SessionSnapshot, vfs::VirtualFs};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub snapshot: SessionSnapshot,
    pub vfs: VirtualFs,
    pub docker: DockerSim,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedData {
    pub session: Option<StoredSession>,
    pub recent_results: Vec<String>,
}

pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new() -> Result<Self> {
        let candidates = [dirs::data_local_dir(), Some(std::env::temp_dir())];

        for candidate in candidates.into_iter().flatten() {
            let mut path = candidate;
            path.push("cmd-mock-cli");
            if fs::create_dir_all(&path).is_ok() {
                path.push("session.json");
                return Ok(Self { path });
            }
        }

        Err(anyhow::anyhow!(
            "failed to create writable data directory for session store"
        ))
    }

    pub fn load(&self) -> Result<PersistedData> {
        if !self.path.exists() {
            return Ok(PersistedData::default());
        }
        let raw = fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read session store: {}", self.path.display()))?;
        let data = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse session store: {}", self.path.display()))?;
        Ok(data)
    }

    pub fn save(&self, data: &PersistedData) -> Result<()> {
        let raw = serde_json::to_string_pretty(data)?;
        fs::write(&self.path, raw)
            .with_context(|| format!("failed to write session store: {}", self.path.display()))
    }
}
