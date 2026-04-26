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
            let target = tokens.get(1).map(String::as_str);
            let lines = vfs.ls(target)?;
            Ok(ExecOutput { stdout: lines })
        }
        "cd" => {
            let target = tokens.get(1).ok_or_else(|| anyhow!("cd requires target"))?;
            vfs.cd(target)?;
            Ok(ExecOutput::single(format!("cwd => {}", vfs.pwd())))
        }
        "mkdir" => {
            let target = tokens
                .get(1)
                .ok_or_else(|| anyhow!("mkdir requires target"))?;
            vfs.mkdir(target)?;
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
            let target = tokens.get(1).ok_or_else(|| anyhow!("rm requires target"))?;
            vfs.rm(target)?;
            Ok(ExecOutput::single(format!("removed {}", target)))
        }
        "find" => {
            let needle = tokens
                .last()
                .ok_or_else(|| anyhow!("find requires target"))?;
            let name = needle.trim_matches('"');
            let name = name.trim_start_matches("./");
            Ok(ExecOutput {
                stdout: vfs.find_name(name),
            })
        }
        "grep" => {
            let needle = tokens
                .get(1)
                .ok_or_else(|| anyhow!("grep requires needle"))?;
            Ok(ExecOutput {
                stdout: vfs.grep(needle),
            })
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
