use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    File { content: String },
    Directory { children: BTreeMap<String, Node> },
}

impl Node {
    fn directory() -> Self {
        Self::Directory {
            children: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualFs {
    root: Node,
    cwd: Vec<String>,
}

impl Default for VirtualFs {
    fn default() -> Self {
        let mut fs = Self {
            root: Node::directory(),
            cwd: Vec::new(),
        };
        fs.seed();
        fs
    }
}

impl VirtualFs {
    pub fn pwd(&self) -> String {
        if self.cwd.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", self.cwd.join("/"))
        }
    }

    pub fn ls(&self, target: Option<&str>, show_all: bool) -> Result<Vec<String>> {
        let path = self.resolve_path(target.unwrap_or("."));
        let node = self.node_at(&path)?;
        match node {
            Node::Directory { children } => {
                let mut entries: Vec<String> = children.keys().cloned().collect();
                if show_all {
                    entries.insert(0, "..".to_string());
                    entries.insert(0, ".".to_string());
                }
                Ok(entries)
            }
            Node::File { .. } => Err(anyhow!("not a directory: {}", display_path(&path))),
        }
    }

    pub fn cd(&mut self, target: &str) -> Result<()> {
        let path = self.resolve_path(target);
        match self.node_at(&path)? {
            Node::Directory { .. } => {
                self.cwd = path;
                Ok(())
            }
            Node::File { .. } => Err(anyhow!("not a directory: {}", target)),
        }
    }

    pub fn mkdir(&mut self, target: &str) -> Result<()> {
        let path = self.resolve_path(target);
        self.insert_node(path, Node::directory())
    }

    pub fn mkdir_p(&mut self, target: &str) -> Result<()> {
        let path = self.resolve_path(target);
        for len in 1..=path.len() {
            let partial = path[..len].to_vec();
            if self.node_at(&partial).is_err() {
                self.insert_node(partial, Node::directory())?;
            }
        }
        Ok(())
    }

    pub fn touch(&mut self, target: &str) -> Result<()> {
        let path = self.resolve_path(target);
        self.insert_node(
            path,
            Node::File {
                content: String::new(),
            },
        )
    }

    pub fn write_file(&mut self, target: &str, content: &str) -> Result<()> {
        let path = self.resolve_path(target);
        self.insert_node(
            path,
            Node::File {
                content: content.to_string(),
            },
        )
    }

    pub fn cat(&self, target: &str) -> Result<String> {
        let path = self.resolve_path(target);
        match self.node_at(&path)? {
            Node::File { content } => Ok(content.clone()),
            Node::Directory { .. } => Err(anyhow!("is a directory: {}", target)),
        }
    }

    pub fn rm(&mut self, target: &str) -> Result<()> {
        let path = self.resolve_path(target);
        if matches!(self.node_at(&path)?, Node::Directory { .. }) {
            return Err(anyhow!("is a directory: {} (use rm -r)", target));
        }
        self.remove_node(&path)
    }

    pub fn rm_recursive(&mut self, target: &str) -> Result<()> {
        let path = self.resolve_path(target);
        self.remove_node(&path)
    }

    pub fn cp(&mut self, from: &str, to: &str) -> Result<()> {
        let source_path = self.resolve_path(from);
        let target_path = self.resolve_path(to);
        let node = self.node_at(&source_path)?.clone();
        self.insert_node(target_path, node)
    }

    pub fn mv(&mut self, from: &str, to: &str) -> Result<()> {
        let source_path = self.resolve_path(from);
        let target_path = self.resolve_path(to);
        let node = self.node_at(&source_path)?.clone();
        self.remove_node(&source_path)?;
        self.insert_node(target_path, node)
    }

    pub fn find_paths_with_prefix(&self, prefix: &str) -> Vec<String> {
        let mut out = Vec::new();
        self.collect_paths("/", &self.root, &mut out);
        out.into_iter().filter(|p| p.starts_with(prefix)).collect()
    }

    pub fn find_name(&self, needle: &str) -> Vec<String> {
        self.find_name_in(".", needle)
    }

    pub fn find_name_in(&self, start: &str, needle: &str) -> Vec<String> {
        let start_path = self.resolve_path(start);
        let start_prefix = if start_path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", start_path.join("/"))
        };
        let mut out = Vec::new();
        self.collect_paths("/", &self.root, &mut out);
        out.into_iter()
            .filter(|path| {
                (path == &start_prefix
                    || path.starts_with(&format!("{}/", start_prefix)))
                    && path.rsplit('/').next().unwrap_or_default() == needle
            })
            .collect()
    }

    pub fn grep(&self, needle: &str) -> Vec<String> {
        let mut matches = Vec::new();
        self.collect_grep("/", &self.root, needle, &mut matches);
        matches
    }

    pub fn grep_in_file(&self, needle: &str, file: &str) -> Result<Vec<String>> {
        let path = self.resolve_path(file);
        match self.node_at(&path)? {
            Node::File { content } => Ok(content
                .lines()
                .filter(|line| line.contains(needle))
                .map(|line| format!("{}:{}", file, line))
                .collect()),
            Node::Directory { .. } => Err(anyhow!("is a directory: {}", file)),
        }
    }

    fn seed(&mut self) {
        let _ = self.mkdir("/home");
        let _ = self.mkdir("/home/player");
        let _ = self.mkdir("/tmp");
        let _ = self.mkdir("/var");
        let _ = self.mkdir("/var/log");
        let _ = self.write_file("/home/player/readme.txt", "practice shell commands");
        let _ = self.write_file("/var/log/app.log", "error: demo failure\ninfo: restarted");
        let _ = self.cd("/home/player");
    }

    fn resolve_path(&self, raw: &str) -> Vec<String> {
        let mut parts = if raw.starts_with('/') {
            Vec::new()
        } else {
            self.cwd.clone()
        };

        for part in raw.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                other => parts.push(other.to_string()),
            }
        }

        parts
    }

    fn insert_node(&mut self, path: Vec<String>, node: Node) -> Result<()> {
        if path.is_empty() {
            return Err(anyhow!("cannot overwrite root"));
        }

        let (parents, name) = path.split_at(path.len() - 1);
        let dir = self.node_at_mut(parents)?;
        match dir {
            Node::Directory { children } => {
                children.insert(name[0].clone(), node);
                Ok(())
            }
            Node::File { .. } => Err(anyhow!("parent is not a directory")),
        }
    }

    fn remove_node(&mut self, path: &[String]) -> Result<()> {
        if path.is_empty() {
            return Err(anyhow!("cannot remove root"));
        }

        let (parents, name) = path.split_at(path.len() - 1);
        let dir = self.node_at_mut(parents)?;
        match dir {
            Node::Directory { children } => {
                children
                    .remove(&name[0])
                    .ok_or_else(|| anyhow!("no such file or directory"))?;
                Ok(())
            }
            Node::File { .. } => Err(anyhow!("parent is not a directory")),
        }
    }

    fn node_at(&self, path: &[String]) -> Result<&Node> {
        let mut current = &self.root;
        for part in path {
            current = match current {
                Node::Directory { children } => children
                    .get(part)
                    .ok_or_else(|| anyhow!("no such file or directory: {}", part))?,
                Node::File { .. } => return Err(anyhow!("path contains file segment")),
            };
        }
        Ok(current)
    }

    fn node_at_mut(&mut self, path: &[String]) -> Result<&mut Node> {
        let mut current = &mut self.root;
        for part in path {
            current = match current {
                Node::Directory { children } => children
                    .get_mut(part)
                    .ok_or_else(|| anyhow!("no such file or directory: {}", part))?,
                Node::File { .. } => return Err(anyhow!("path contains file segment")),
            };
        }
        Ok(current)
    }

    fn collect_paths(&self, prefix: &str, node: &Node, out: &mut Vec<String>) {
        let _ = &self.cwd;
        match node {
            Node::File { .. } => out.push(prefix.to_string()),
            Node::Directory { children } => {
                if prefix != "/" {
                    out.push(prefix.to_string());
                }
                for (name, child) in children {
                    let next = if prefix == "/" {
                        format!("/{}", name)
                    } else {
                        format!("{}/{}", prefix, name)
                    };
                    self.collect_paths(&next, child, out);
                }
            }
        }
    }

    fn collect_grep(&self, prefix: &str, node: &Node, needle: &str, out: &mut Vec<String>) {
        let _ = &self.cwd;
        match node {
            Node::File { content } => {
                if content.contains(needle) {
                    out.push(prefix.to_string());
                }
            }
            Node::Directory { children } => {
                for (name, child) in children {
                    let next = if prefix == "/" {
                        format!("/{}", name)
                    } else {
                        format!("{}/{}", prefix, name)
                    };
                    self.collect_grep(&next, child, needle, out);
                }
            }
        }
    }
}

fn display_path(path: &[String]) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::VirtualFs;

    #[test]
    fn vfs_moves_files() {
        let mut fs = VirtualFs::default();
        fs.touch("demo.txt").expect("touch");
        fs.mv("demo.txt", "archive.txt").expect("mv");
        assert!(fs.cat("archive.txt").is_ok());
        assert!(fs.cat("demo.txt").is_err());
    }

    #[test]
    fn ls_show_all_includes_dot_entries() {
        let fs = VirtualFs::default();
        let entries = fs.ls(None, true).expect("ls");
        assert_eq!(entries.first().map(String::as_str), Some("."));
        assert_eq!(entries.get(1).map(String::as_str), Some(".."));
        assert!(entries.iter().any(|entry| entry == "readme.txt"));
    }

    #[test]
    fn rm_rejects_directories_without_recursive_flag() {
        let mut fs = VirtualFs::default();
        let error = fs.rm("/var").expect_err("rm should reject directories");
        assert!(error.to_string().contains("use rm -r"));
    }

    #[test]
    fn rm_recursive_removes_directories() {
        let mut fs = VirtualFs::default();
        fs.rm_recursive("/var").expect("rm -r");
        assert!(fs.cd("/var").is_err());
    }

    #[test]
    fn mkdir_p_creates_missing_parents() {
        let mut fs = VirtualFs::default();
        fs.mkdir_p("/tmp/nested/path").expect("mkdir -p");
        fs.cd("/tmp/nested/path").expect("cd");
        assert_eq!(fs.pwd(), "/tmp/nested/path");
    }

    #[test]
    fn grep_in_file_returns_matching_lines_with_file_prefix() {
        let fs = VirtualFs::default();
        let matches = fs.grep_in_file("error", "/var/log/app.log").expect("grep");
        assert_eq!(matches, vec!["/var/log/app.log:error: demo failure"]);
    }

    #[test]
    fn grep_in_file_rejects_directories() {
        let fs = VirtualFs::default();
        let error = fs
            .grep_in_file("error", "/var/log")
            .expect_err("grep should reject directories");
        assert!(error.to_string().contains("is a directory"));
    }
}
