use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::time::timeout;
use futures::StreamExt;

use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions, StartContainerOptions, RemoveContainerOptions, LogOutput, LogsOptions, WaitContainerOptions};
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

    let container_id = &container.id;

    // Attach to container to stream stdin, stdout, and stderr
    let attach_res = docker.attach_container(
        container_id,
        Some(bollard::container::AttachContainerOptions::<String> {
            stdin: Some(true),
            stdout: Some(true),
            stderr: Some(true),
            stream: Some(true),
            ..Default::default()
        })
    ).await;

    let (write_half, read_half) = match attach_res {
        Ok(res) => (Some(res.input), Some(res.output)),
        Err(e) => {
            let _ = docker.remove_container(container_id, None::<RemoveContainerOptions>).await;
            return Err(e);
        }
    };

    // Start container
    if let Err(e) = docker.start_container(container_id, None::<StartContainerOptions<String>>).await {
        let _ = docker.remove_container(container_id, None::<RemoveContainerOptions>).await;
        return Err(e);
    }

    // Write source code to stdin and drop/close the writer
    if let Some(mut writer) = write_half {
        use tokio::io::AsyncWriteExt;
        let _ = writer.write_all(source.as_bytes()).await;
        let _ = writer.flush().await;
        let _ = writer.shutdown().await;
        drop(writer);
    }

    // Wait for execution with timeout control
    let wait_future = async {
        let mut wait_stream = docker.wait_container(container_id, None::<WaitContainerOptions<String>>);
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
            let _ = docker.kill_container::<String>(container_id, None).await;
        }
    }

    // Collect logs
    let log_options = Some(LogsOptions::<String> {
        stdout: true,
        stderr: true,
        ..Default::default()
    });

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    if let Some(mut stream) = read_half {
        while let Some(Ok(chunk)) = stream.next().await {
            match chunk {
                LogOutput::StdOut { message } => stdout_buf.extend_from_slice(&message),
                LogOutput::StdErr { message } => stderr_buf.extend_from_slice(&message),
                _ => {}
            }
        }
    } else {
        // if attach failed to stream, fall back to logs api
        let mut log_stream = docker.logs(container_id, log_options);
        while let Some(Ok(chunk)) = log_stream.next().await {
            match chunk {
                LogOutput::StdOut { message } => stdout_buf.extend_from_slice(&message),
                LogOutput::StdErr { message } => stderr_buf.extend_from_slice(&message),
                _ => {}
            }
        }
    }

    // Check OOM status
    let inspect = docker.inspect_container(container_id, None).await;
    let oom_killed = inspect.ok().and_then(|info| {
        info.state.and_then(|s| s.oom_killed)
    }).unwrap_or(false);

    // Remove container
    let remove_options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });
    let _ = docker.remove_container(container_id, remove_options).await;

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
}
