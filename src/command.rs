use anyhow::{Result, anyhow};

use crate::{docker::DockerSim, model::LearningMode, vfs::VirtualFs};

#[derive(Debug, Clone)]
pub struct ExecOutput {
    pub stdout: Vec<String>,
}

impl ExecOutput {
    fn single(line: impl Into<String>) -> Self {
        Self {
            stdout: vec![line.into()],
        }
    }
}

pub fn execute(
    learning_mode: LearningMode,
    line: &str,
    vfs: &mut VirtualFs,
    docker: &mut DockerSim,
) -> Result<ExecOutput> {
    let tokens: Vec<String> = line.split_whitespace().map(ToString::to_string).collect();
    if tokens.is_empty() {
        return Ok(ExecOutput { stdout: vec![] });
    }

    if tokens[0] == "docker" {
        return execute_docker(&tokens, docker);
    }

    match learning_mode {
        LearningMode::Docker | LearningMode::Linux | LearningMode::Macos => {
            execute_shell(&tokens, vfs)
        }
    }
}

fn execute_shell(tokens: &[String], vfs: &mut VirtualFs) -> Result<ExecOutput> {
    match tokens[0].as_str() {
        "pwd" => Ok(ExecOutput::single(vfs.pwd())),
        "ls" => {
            let mut show_all = false;
            let mut target: Option<&str> = None;
            for token in &tokens[1..] {
                match token.as_str() {
                    "-a" | "--all" => show_all = true,
                    value if value.starts_with('-') => {}
                    value => target = Some(value),
                }
            }
            let lines = vfs.ls(target, show_all)?;
            Ok(ExecOutput { stdout: lines })
        }
        "cd" => {
            let target = tokens.get(1).ok_or_else(|| anyhow!("cd requires target"))?;
            vfs.cd(target)?;
            Ok(ExecOutput::single(format!("cwd => {}", vfs.pwd())))
        }
        "mkdir" => {
            let mut parents = false;
            let mut target: Option<&str> = None;
            for token in &tokens[1..] {
                match token.as_str() {
                    "-p" | "--parents" => parents = true,
                    value if value.starts_with('-') => {}
                    value => target = Some(value),
                }
            }
            let target = target.ok_or_else(|| anyhow!("mkdir requires target"))?;
            if parents {
                vfs.mkdir_p(target)?;
            } else {
                vfs.mkdir(target)?;
            }
            Ok(ExecOutput::single(format!("created {}", target)))
        }
        "touch" => {
            let target = tokens
                .get(1)
                .ok_or_else(|| anyhow!("touch requires target"))?;
            vfs.touch(target)?;
            Ok(ExecOutput::single(format!("touched {}", target)))
        }
        "cat" => {
            let target = tokens
                .get(1)
                .ok_or_else(|| anyhow!("cat requires target"))?;
            Ok(ExecOutput::single(vfs.cat(target)?))
        }
        "cp" => {
            let from = tokens.get(1).ok_or_else(|| anyhow!("cp requires source"))?;
            let to = tokens
                .get(2)
                .ok_or_else(|| anyhow!("cp requires destination"))?;
            vfs.cp(from, to)?;
            Ok(ExecOutput::single(format!("copied {} -> {}", from, to)))
        }
        "mv" => {
            let from = tokens.get(1).ok_or_else(|| anyhow!("mv requires source"))?;
            let to = tokens
                .get(2)
                .ok_or_else(|| anyhow!("mv requires destination"))?;
            vfs.mv(from, to)?;
            Ok(ExecOutput::single(format!("moved {} -> {}", from, to)))
        }
        "rm" => {
            let mut recursive = false;
            let mut target: Option<&str> = None;
            for token in &tokens[1..] {
                match token.as_str() {
                    "-r" | "-R" | "--recursive" => recursive = true,
                    value if value.starts_with('-') => {}
                    value => target = Some(value),
                }
            }
            let target = target.ok_or_else(|| anyhow!("rm requires target"))?;
            if recursive {
                vfs.rm_recursive(target)?;
            } else {
                vfs.rm(target)?;
            }
            Ok(ExecOutput::single(format!("removed {}", target)))
        }
        "find" => {
            let (prefix, name) = parse_find_args(tokens, vfs)?;
            let mut matches = vfs.find_name(&name);
            if let Some(prefix) = prefix {
                matches.retain(|path| path == &prefix || path.starts_with(&format!("{}/", prefix)));
            }
            Ok(ExecOutput { stdout: matches })
        }
        "grep" => {
            let needle = tokens
                .get(1)
                .ok_or_else(|| anyhow!("grep requires needle"))?;
            let file = tokens
                .iter()
                .skip(2)
                .find(|token| !token.starts_with('-'))
                .map(String::as_str);
            let stdout = match file {
                Some(file) => vfs.grep_in_file(needle, file)?,
                None => vfs.grep(needle),
            };
            Ok(ExecOutput { stdout })
        }
        "echo" => Ok(ExecOutput::single(tokens[1..].join(" "))),
        other => Err(anyhow!("unsupported command: {}", other)),
    }
}

fn execute_docker(tokens: &[String], docker: &mut DockerSim) -> Result<ExecOutput> {
    let sub = tokens
        .get(1)
        .ok_or_else(|| anyhow!("docker requires subcommand"))?;
    match sub.as_str() {
        "images" => Ok(ExecOutput {
            stdout: docker.images(),
        }),
        "pull" => {
            let image = tokens
                .get(2)
                .ok_or_else(|| anyhow!("docker pull requires image"))?;
            Ok(ExecOutput::single(docker.pull(image)))
        }
        "run" => {
            let mut name = None;
            let mut image = None;
            let mut cursor = 2usize;
            while cursor < tokens.len() {
                match tokens[cursor].as_str() {
                    "--name" => {
                        name = tokens.get(cursor + 1).map(String::as_str);
                        cursor += 2;
                    }
                    "-d" => {
                        cursor += 1;
                    }
                    "-p" | "-e" | "-v" => {
                        if tokens.get(cursor + 1).is_none() {
                            return Err(anyhow!("docker run {} requires value", tokens[cursor]));
                        }
                        cursor += 2;
                    }
                    value if !value.starts_with('-') => {
                        image = Some(value);
                        cursor += 1;
                    }
                    _ => cursor += 1,
                }
            }
            let image = image.ok_or_else(|| anyhow!("docker run requires image"))?;
            Ok(ExecOutput::single(docker.run(image, name)?))
        }
        "ps" => {
            let all = tokens.iter().any(|t| t == "-a" || t == "--all");
            Ok(ExecOutput {
                stdout: docker.ps(all),
            })
        }
        "stop" => {
            let name = tokens
                .get(2)
                .ok_or_else(|| anyhow!("docker stop requires name"))?;
            Ok(ExecOutput::single(docker.stop(name)?))
        }
        "rm" => {
            let name = tokens
                .get(2)
                .ok_or_else(|| anyhow!("docker rm requires name"))?;
            Ok(ExecOutput::single(docker.rm(name)?))
        }
        "logs" => {
            let name = tokens
                .get(2)
                .ok_or_else(|| anyhow!("docker logs requires name"))?;
            Ok(ExecOutput {
                stdout: docker.logs(name)?,
            })
        }
        "exec" => {
            let name = tokens
                .get(2)
                .ok_or_else(|| anyhow!("docker exec requires name"))?;
            let cmd = tokens
                .get(3..)
                .ok_or_else(|| anyhow!("docker exec requires command"))?
                .to_vec();
            Ok(ExecOutput::single(docker.exec(name, &cmd)?))
        }
        other => Err(anyhow!("unsupported docker subcommand: {}", other)),
    }
}

fn parse_find_args(tokens: &[String], vfs: &VirtualFs) -> Result<(Option<String>, String)> {
    let mut search_root = None;
    let mut name = None;
    let mut cursor = 1usize;

    while cursor < tokens.len() {
        match tokens[cursor].as_str() {
            "-name" => {
                let needle = tokens
                    .get(cursor + 1)
                    .ok_or_else(|| anyhow!("find -name requires pattern"))?;
                name = Some(normalize_find_name(needle));
                cursor += 2;
            }
            value if value.starts_with('-') => {
                cursor += 1;
                if tokens
                    .get(cursor)
                    .is_some_and(|next| !next.starts_with('-'))
                {
                    cursor += 1;
                }
            }
            value => {
                if search_root.is_none() {
                    search_root = Some(value.to_string());
                } else if name.is_none() {
                    name = Some(normalize_find_name(value));
                }
                cursor += 1;
            }
        }
    }

    let name = name.ok_or_else(|| anyhow!("find requires target"))?;
    let prefix = search_root.map(|root| {
        let pwd = vfs.pwd();
        normalize_find_prefix(&pwd, &root)
    });
    Ok((prefix, name))
}

fn normalize_find_name(raw: &str) -> String {
    raw.trim_matches('"')
        .trim_start_matches("./")
        .trim_matches('*')
        .to_string()
}

fn normalize_find_prefix(pwd: &str, raw: &str) -> String {
    let mut parts = if raw.starts_with('/') {
        Vec::new()
    } else {
        pwd.trim_start_matches('/')
            .split('/')
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>()
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

    if parts.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", parts.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::execute;
    use crate::{docker::DockerSim, model::LearningMode, vfs::VirtualFs};

    fn run_shell(line: &str, vfs: &mut VirtualFs) -> Vec<String> {
        execute(LearningMode::Linux, line, vfs, &mut DockerSim::default())
            .unwrap()
            .stdout
    }

    #[test]
    fn ls_a_uses_show_all_flag() {
        let mut vfs = VirtualFs::default();
        let out = run_shell("ls -a", &mut vfs);
        assert_eq!(out.first().map(String::as_str), Some("."));
        assert_eq!(out.get(1).map(String::as_str), Some(".."));
    }

    #[test]
    fn rm_r_removes_directory() {
        let mut vfs = VirtualFs::default();
        run_shell("mkdir -p nested/dir", &mut vfs);
        assert!(
            execute(
                LearningMode::Linux,
                "rm nested",
                &mut vfs,
                &mut DockerSim::default()
            )
            .is_err()
        );
        run_shell("rm -r nested", &mut vfs);
        assert!(
            run_shell("ls", &mut vfs)
                .into_iter()
                .all(|entry| entry != "nested")
        );
    }

    #[test]
    fn mkdir_p_creates_missing_parents() {
        let mut vfs = VirtualFs::default();
        run_shell("mkdir -p alpha/beta", &mut vfs);
        let out = run_shell("ls alpha", &mut vfs);
        assert_eq!(out, vec!["beta".to_string()]);
    }

    #[test]
    fn grep_with_file_argument_limits_scope() {
        let mut vfs = VirtualFs::default();
        let out = run_shell("grep error /var/log/app.log", &mut vfs);
        assert_eq!(
            out,
            vec!["/var/log/app.log:error: demo failure".to_string()]
        );
    }

    #[test]
    fn find_name_parses_flags_and_root() {
        let mut vfs = VirtualFs::default();
        run_shell("mkdir -p logs/archive", &mut vfs);
        run_shell("touch logs/archive/app.log", &mut vfs);
        run_shell("touch /tmp/app.log", &mut vfs);
        let out = run_shell("find ./logs -type f -name app.log", &mut vfs);
        assert_eq!(out, vec!["/home/player/logs/archive/app.log".to_string()]);
    }

    #[test]
    fn docker_run_ignores_supported_flags_while_finding_image() {
        let mut vfs = VirtualFs::default();
        let mut docker = DockerSim::default();
        let out = execute(
            LearningMode::Docker,
            "docker run -d -p 8080:80 -e APP_ENV=dev -v /tmp:/data --name demo nginx",
            &mut vfs,
            &mut docker,
        )
        .unwrap()
        .stdout;
        assert_eq!(out, vec!["started demo".to_string()]);
    }
}
