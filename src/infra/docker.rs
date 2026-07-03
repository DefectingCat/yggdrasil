use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::time::timeout;
use futures::StreamExt;

use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions, StartContainerOptions, RemoveContainerOptions, LogOutput, WaitContainerOptions};
use bollard::models::{HostConfig, ResourcesUlimits};
use crate::infra::runner_config::{ResourceLimits, RUNNER_CONFIG};

pub static DOCKER_CLIENT: LazyLock<Docker> = LazyLock::new(|| {
    Docker::connect_with_unix(&RUNNER_CONFIG.docker_socket_path, 120, bollard::API_DEFAULT_VERSION)
        .expect("Failed to connect to Docker daemon via unix socket")
});

pub fn build_host_config(limits: &ResourceLimits) -> HostConfig {
    let mut tmpfs = HashMap::new();
    tmpfs.insert("/code".to_string(), "size=16m,uid=1000,gid=1000".to_string());
    tmpfs.insert("/tmp".to_string(), "size=64m,mode=1777".to_string());
    tmpfs.insert("/run".to_string(), "size=16m,mode=1777".to_string());

    let memory = (limits.memory_mb * 1024 * 1024) as i64;

    HostConfig {
        cpu_quota: Some((limits.cpu_cores * 100_000.0) as i64),
        cpu_period: Some(100_000),
        memory: Some(memory),
        memory_swap: Some(memory), // = memory, disable swap
        network_mode: Some(if limits.allow_network { "bridge".to_string() } else { "none".to_string() }),
        readonly_rootfs: Some(true),
        tmpfs: Some(tmpfs),
        pids_limit: Some(64),
        ulimits: Some(vec![
            ResourcesUlimits { name: Some("nofile".to_string()), soft: Some(64), hard: Some(64) },
            ResourcesUlimits { name: Some("nproc".to_string()), soft: Some(64), hard: Some(64) },
        ]),
        cap_drop: Some(vec!["ALL".to_string()]),
        security_opt: Some(vec!["no-new-privileges".to_string()]),
        auto_remove: Some(false), // must be false to avoid premature removal before getting logs
        ..Default::default()
    }
}

struct ContainerGuard {
    container_id: String,
    docker: Docker,
}

impl Drop for ContainerGuard {
    fn drop(&mut self) {
        let docker = self.docker.clone();
        let container_id = self.container_id.clone();
        tokio::spawn(async move {
            let remove_options = Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            });
            let _ = docker.remove_container(&container_id, remove_options).await;
        });
    }
}

pub async fn run_in_container(
    image_name: &str,
    run_cmd: &str,
    source: &str,
    ext: &str,
    limits: ResourceLimits,
) -> Result<(Option<i64>, String, String, bool), bollard::errors::Error> {
    let docker = &*DOCKER_CLIENT;
    let host_config = build_host_config(&limits);

    // Source injection script: use sh -c to first receive stdin and write to file, then exec the actual command
    let setup_cmd = format!("cat > /code/main.{} && exec {}", ext, run_cmd);
    let cmd = vec!["sh".to_string(), "-c".to_string(), setup_cmd];

    let config = Config {
        image: Some(image_name.to_string()),
        cmd: Some(cmd),
        host_config: Some(host_config),
        attach_stdin: Some(true),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        open_stdin: Some(true),
        stdin_once: Some(true),
        user: Some("1000:1000".to_string()), // non-root user
        working_dir: Some("/code".to_string()),
        ..Default::default()
    };

    let container = docker.create_container(
        None::<CreateContainerOptions<String>>,
        config
    ).await?;

    let container_id = container.id;
    let _guard = ContainerGuard {
        container_id: container_id.clone(),
        docker: docker.clone(),
    };

    // Attach to container to stream stdin, stdout, and stderr
    let attach_res = docker.attach_container(
        &container_id,
        Some(bollard::container::AttachContainerOptions::<String> {
            stdin: Some(true),
            stdout: Some(true),
            stderr: Some(true),
            stream: Some(true),
            ..Default::default()
        })
    ).await;

    let (mut writer, mut stream) = match attach_res {
        Ok(res) => (res.input, res.output),
        Err(e) => return Err(e),
    };

    // Start container
    docker
        .start_container(&container_id, None::<StartContainerOptions<String>>)
        .await?;

    // Write source code to stdin and drop/close the writer
    use tokio::io::AsyncWriteExt;
    let write_fut = async {
        let _ = writer.write_all(source.as_bytes()).await;
        let _ = writer.flush().await;
        let _ = writer.shutdown().await;
    };

    if timeout(Duration::from_secs(5), write_fut).await.is_err() {
        return Err(bollard::errors::Error::IOError {
            err: std::io::Error::new(std::io::ErrorKind::TimedOut, "Writing to stdin timed out")
        });
    }
    drop(writer);

    // Wait for execution with timeout control
    let wait_future = async {
        let mut wait_stream = docker.wait_container(&container_id, None::<WaitContainerOptions<String>>);
        wait_stream.next().await
    };

    let wait_res = timeout(Duration::from_secs(limits.timeout_secs), wait_future).await;

    let mut timed_out = false;
    let mut exit_code = None;

    match wait_res {
        Ok(Some(Ok(exit_status))) => {
            exit_code = Some(exit_status.status_code);
        }
        Ok(_) => {} // wait error
        Err(_) => {
            // timeout, kill container
            timed_out = true;
            let _ = docker.kill_container::<String>(&container_id, None).await;
        }
    }

    // Collect logs
    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                match chunk {
                    LogOutput::StdOut { message } => {
                        let remaining = (limits.output_bytes as usize).saturating_sub(stdout_buf.len() + stderr_buf.len());
                        if remaining > 0 {
                            let to_add = message.len().min(remaining);
                            stdout_buf.extend_from_slice(&message[..to_add]);
                        }
                    }
                    LogOutput::StdErr { message } => {
                        let remaining = (limits.output_bytes as usize).saturating_sub(stdout_buf.len() + stderr_buf.len());
                        if remaining > 0 {
                            let to_add = message.len().min(remaining);
                            stderr_buf.extend_from_slice(&message[..to_add]);
                        }
                    }
                    _ => {}
                }
                if stdout_buf.len() + stderr_buf.len() >= limits.output_bytes as usize {
                    break;
                }
            }
            Err(e) => {
                tracing::error!("Error reading container log stream: {:?}", e);
                break;
            }
        }
    }

    // Check OOM status
    let inspect = docker.inspect_container(&container_id, None).await;
    let oom_killed = inspect.ok().and_then(|info| {
        info.state.and_then(|s| s.oom_killed)
    }).unwrap_or(false);

    // Truncate output to limits.output_bytes
    let limit_bytes = limits.output_bytes as usize;
    let stdout_len = stdout_buf.len().min(limit_bytes);
    let stderr_len = stderr_buf.len().min(limit_bytes);

    let stdout = String::from_utf8_lossy(&stdout_buf[..stdout_len]).into_owned();
    let stderr = String::from_utf8_lossy(&stderr_buf[..stderr_len]).into_owned();

    if timed_out {
        return Err(bollard::errors::Error::IOError {
            err: std::io::Error::new(std::io::ErrorKind::TimedOut, "Execution timed out")
        });
    }

    Ok((exit_code, stdout, stderr, oom_killed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::runner_config::ResourceLimits;

    #[test]
    fn test_host_config_generation() {
        let limits = ResourceLimits {
            cpu_cores: 1.5,
            memory_mb: 256,
            timeout_secs: 5,
            output_bytes: 1024,
            allow_network: false,
        };
        let host_config = build_host_config(&limits);
        assert_eq!(host_config.cpu_quota, Some(150_000));
        assert_eq!(host_config.memory, Some(256 * 1024 * 1024));
        assert_eq!(host_config.readonly_rootfs, Some(true));
        assert_eq!(host_config.network_mode.as_deref(), Some("none"));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_run_in_container_success() {
        let limits = ResourceLimits {
            cpu_cores: 1.0,
            memory_mb: 128,
            timeout_secs: 5,
            output_bytes: 1024,
            allow_network: false,
        };
        let (exit_code, stdout, stderr, oom_killed) = run_in_container(
            "alpine:latest",
            "cat /code/main.txt",
            "hello world",
            "txt",
            limits,
        )
        .await
        .unwrap();

        assert_eq!(exit_code, Some(0));
        assert_eq!(stdout, "hello world");
        assert!(stderr.is_empty());
        assert!(!oom_killed);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_run_in_container_output_truncation() {
        let limits = ResourceLimits {
            cpu_cores: 1.0,
            memory_mb: 128,
            timeout_secs: 5,
            output_bytes: 5,
            allow_network: false,
        };
        let (exit_code, stdout, stderr, oom_killed) = run_in_container(
            "alpine:latest",
            "cat /code/main.txt",
            "hello world",
            "txt",
            limits,
        )
        .await
        .unwrap();

        assert_eq!(exit_code, Some(0));
        assert_eq!(stdout, "hello");
        assert!(stderr.is_empty());
        assert!(!oom_killed);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_run_in_container_timeout() {
        let limits = ResourceLimits {
            cpu_cores: 1.0,
            memory_mb: 128,
            timeout_secs: 1,
            output_bytes: 1024,
            allow_network: false,
        };
        let res = run_in_container(
            "alpine:latest",
            "sleep 10",
            "",
            "txt",
            limits,
        )
        .await;

        assert!(res.is_err());
        let err = res.unwrap_err();
        match err {
            bollard::errors::Error::IOError { err } => {
                assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
            }
            _ => panic!("Expected IOError(TimedOut), got {:?}", err),
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_run_in_container_cancellation() {
        let docker = &*DOCKER_CLIENT;
        
        let before = docker.list_containers(Some(bollard::container::ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        })).await.unwrap();
        let before_ids: std::collections::HashSet<String> = before.into_iter().map(|c| c.id.unwrap()).collect();

        let limits = ResourceLimits {
            cpu_cores: 1.0,
            memory_mb: 128,
            timeout_secs: 10,
            output_bytes: 1024,
            allow_network: false,
        };

        let run_fut = run_in_container(
            "alpine:latest",
            "sleep 100",
            "",
            "txt",
            limits,
        );

        tokio::select! {
            _ = run_fut => {
                panic!("Should have been cancelled");
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                // Cancelled!
            }
        }

        tokio::time::sleep(Duration::from_secs(2)).await;

        let after = docker.list_containers(Some(bollard::container::ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        })).await.unwrap();

        let mut leaked = Vec::new();
        for c in after {
            let id = c.id.unwrap();
            if !before_ids.contains(&id) && c.image.as_deref() == Some("alpine:latest") {
                leaked.push(id);
            }
        }

        let leaked_count = leaked.len();
        for id in leaked {
            let _ = docker.remove_container(&id, Some(bollard::container::RemoveContainerOptions {
                force: true,
                ..Default::default()
            })).await;
        }

        assert_eq!(leaked_count, 0, "Found {} leaked containers", leaked_count);
    }
}
