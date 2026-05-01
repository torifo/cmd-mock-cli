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
            let mut long = false;
            let mut target = None;
            for t in &tokens[1..] {
                if t.starts_with('-') {
                    if t.contains('a') {
                        show_all = true;
                    }
                    if t.contains('l') {
                        long = true;
                    }
                } else {
                    target = Some(t.as_str());
                }
            }
            let lines = vfs.ls(target, show_all)?;
            if long {
                Ok(ExecOutput {
                    stdout: lines.iter().map(|n| format!("  {}", n)).collect(),
                })
            } else {
                Ok(ExecOutput { stdout: lines })
            }
        }
        "cd" => {
            let target = tokens.get(1).ok_or_else(|| anyhow!("cd requires target"))?;
            vfs.cd(target)?;
            Ok(ExecOutput::single(format!("cwd => {}", vfs.pwd())))
        }
        "mkdir" => {
            let parents = tokens.iter().any(|t| t == "-p");
            let target = tokens
                .iter()
                .skip(1)
                .find(|t| !t.starts_with('-'))
                .ok_or_else(|| anyhow!("mkdir requires target"))?;
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
            let recursive = tokens
                .iter()
                .any(|t| matches!(t.as_str(), "-r" | "-rf" | "-fr" | "-R"));
            let target = tokens
                .iter()
                .skip(1)
                .find(|t| !t.starts_with('-'))
                .ok_or_else(|| anyhow!("rm requires target"))?;
            if recursive {
                vfs.rm_recursive(target)?;
            } else {
                vfs.rm(target)?;
            }
            Ok(ExecOutput::single(format!("removed {}", target)))
        }
        "find" => {
            let mut name_pattern: Option<String> = None;
            let mut start = ".";
            let mut i = 1;
            while i < tokens.len() {
                match tokens[i].as_str() {
                    "-name" => {
                        if let Some(next) = tokens.get(i + 1) {
                            name_pattern =
                                Some(next.trim_matches('"').trim_start_matches("./").to_string());
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "-type" => i += 2, // skip flag and its value
                    token if !token.starts_with('-') => {
                        start = token;
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            let name = match &name_pattern {
                Some(n) => n.as_str(),
                None => tokens
                    .last()
                    .map(|s| s.trim_matches('"').trim_start_matches("./"))
                    .unwrap_or(""),
            };
            Ok(ExecOutput {
                stdout: vfs.find_name_in(start, name),
            })
        }
        "grep" => {
            let args: Vec<&str> = tokens
                .iter()
                .skip(1)
                .filter(|t| !t.starts_with('-'))
                .map(|s| s.as_str())
                .collect();
            let needle = args
                .first()
                .ok_or_else(|| anyhow!("grep requires needle"))?;
            if let Some(file) = args.get(1) {
                Ok(ExecOutput {
                    stdout: vfs.grep_in_file(needle, file)?,
                })
            } else {
                Ok(ExecOutput {
                    stdout: vfs.grep(needle),
                })
            }
        }
        "echo" => Ok(ExecOutput::single(tokens[1..].join(" "))),
        "head" => {
            let n = tokens
                .iter()
                .find(|t| t.starts_with('-') && t[1..].chars().all(|c| c.is_ascii_digit()))
                .and_then(|t| t[1..].parse::<usize>().ok())
                .unwrap_or(10);
            let target = tokens
                .iter()
                .skip(1)
                .find(|t| !t.starts_with('-'))
                .ok_or_else(|| anyhow!("head requires file"))?;
            Ok(ExecOutput {
                stdout: vfs.head(target, n)?,
            })
        }
        "tail" => {
            let n = tokens
                .iter()
                .find(|t| t.starts_with('-') && t[1..].chars().all(|c| c.is_ascii_digit()))
                .and_then(|t| t[1..].parse::<usize>().ok())
                .unwrap_or(10);
            let target = tokens
                .iter()
                .skip(1)
                .find(|t| !t.starts_with('-'))
                .ok_or_else(|| anyhow!("tail requires file"))?;
            Ok(ExecOutput {
                stdout: vfs.tail(target, n)?,
            })
        }
        "wc" => {
            let target = tokens
                .iter()
                .skip(1)
                .find(|t| !t.starts_with('-'))
                .ok_or_else(|| anyhow!("wc requires file"))?;
            let (lines, words, bytes) = vfs.wc(target)?;
            Ok(ExecOutput::single(format!(
                "{:>8} {:>8} {:>8} {}",
                lines, words, bytes, target
            )))
        }
        "chmod" => {
            let target = tokens
                .iter()
                .skip(1)
                .find(|t| {
                    !t.starts_with('-')
                        && !t.chars().all(|c| c.is_ascii_digit())
                        && !t.contains('+')
                        && !t.contains('-')
                        && !t.contains('=')
                })
                .or_else(|| tokens.last())
                .ok_or_else(|| anyhow!("chmod requires target"))?;
            Ok(ExecOutput::single(format!("chmod: applied to {}", target)))
        }
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
            let mut image: Option<&str> = None;
            let mut detach = false;
            let mut cursor = 2usize;
            while cursor < tokens.len() {
                match tokens[cursor].as_str() {
                    "--name" => {
                        name = tokens.get(cursor + 1).map(String::as_str);
                        cursor += 2;
                    }
                    "-d" | "--detach" => {
                        detach = true;
                        cursor += 1;
                    }
                    "-p" | "-e" | "-v" => {
                        cursor += 2; // skip flag and its value
                    }
                    value if !value.starts_with('-') => {
                        image = Some(value);
                        cursor += 1;
                    }
                    _ => cursor += 1,
                }
            }
            let image = image.ok_or_else(|| anyhow!("docker run requires image"))?;
            Ok(ExecOutput::single(docker.run(image, name, detach)?))
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
        assert_eq!(out, vec!["started demo (detached)".to_string()]);
    }
}
