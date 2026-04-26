use std::collections::BTreeSet;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub name: String,
    pub image: String,
    pub running: bool,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerSim {
    images: BTreeSet<String>,
    containers: Vec<Container>,
}

impl Default for DockerSim {
    fn default() -> Self {
        Self {
            images: BTreeSet::from(["alpine:latest".to_string(), "nginx:latest".to_string()]),
            containers: vec![Container {
                name: "web".to_string(),
                image: "nginx:latest".to_string(),
                running: true,
                logs: vec!["nginx started".to_string(), "serving traffic".to_string()],
            }],
        }
    }
}

impl DockerSim {
    pub fn images(&self) -> Vec<String> {
        self.images.iter().cloned().collect()
    }

    pub fn pull(&mut self, image: &str) -> String {
        self.images.insert(normalize_image(image));
        format!("pulled {}", normalize_image(image))
    }

    pub fn run(&mut self, image: &str, name: Option<&str>) -> Result<String> {
        let image = normalize_image(image);
        if !self.images.contains(&image) {
            return Err(anyhow!("image not found: {}", image));
        }

        let name = name
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("ctr-{}", self.containers.len() + 1));
        self.containers.push(Container {
            name: name.clone(),
            image: image.clone(),
            running: true,
            logs: vec![format!("container {} started from {}", name, image)],
        });
        Ok(format!("started {}", name))
    }

    pub fn ps(&self, all: bool) -> Vec<String> {
        self.containers
            .iter()
            .filter(|c| all || c.running)
            .map(|c| {
                format!(
                    "{}\t{}\t{}",
                    c.name,
                    c.image,
                    if c.running { "running" } else { "stopped" }
                )
            })
            .collect()
    }

    pub fn stop(&mut self, name: &str) -> Result<String> {
        let container = self
            .containers
            .iter_mut()
            .find(|c| c.name == name)
            .ok_or_else(|| anyhow!("container not found: {}", name))?;
        container.running = false;
        container.logs.push("container stopped".to_string());
        Ok(format!("stopped {}", name))
    }

    pub fn rm(&mut self, name: &str) -> Result<String> {
        let index = self
            .containers
            .iter()
            .position(|c| c.name == name)
            .ok_or_else(|| anyhow!("container not found: {}", name))?;
        self.containers.remove(index);
        Ok(format!("removed {}", name))
    }

    pub fn logs(&self, name: &str) -> Result<Vec<String>> {
        let container = self
            .containers
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| anyhow!("container not found: {}", name))?;
        Ok(container.logs.clone())
    }

    pub fn exec(&mut self, name: &str, command: &[String]) -> Result<String> {
        let container = self
            .containers
            .iter_mut()
            .find(|c| c.name == name)
            .ok_or_else(|| anyhow!("container not found: {}", name))?;
        let rendered = command.join(" ");
        container.logs.push(format!("exec {}", rendered));
        Ok(format!("executed in {}: {}", name, rendered))
    }

    pub fn completions(&self, prefix: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .images
            .iter()
            .filter(|image| image.starts_with(prefix))
            .cloned()
            .collect();
        out.extend(
            self.containers
                .iter()
                .map(|c| c.name.clone())
                .filter(|name| name.starts_with(prefix)),
        );
        out.sort();
        out.dedup();
        out
    }
}

fn normalize_image(image: &str) -> String {
    if image.contains(':') {
        image.to_string()
    } else {
        format!("{}:latest", image)
    }
}

#[cfg(test)]
mod tests {
    use super::DockerSim;

    #[test]
    fn docker_run_requires_existing_image() {
        let mut sim = DockerSim::default();
        assert!(sim.run("missing", None).is_err());
        sim.pull("missing");
        assert!(sim.run("missing", Some("test")).is_ok());
    }
}
