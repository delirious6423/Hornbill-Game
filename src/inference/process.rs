use super::{BackendReply, ImageBackend, StoryBackend, StoryRequest};
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};

pub fn acquire_lease(path: &std::path::Path) -> Result<File> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    file.try_lock()
        .context("another Hornbill turn/worker is running; wait for it to finish")?;
    Ok(file)
}

pub struct ProcessBackend {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub timeout: Duration,
    pub worker_lock: PathBuf,
    pub listen_for_ctrl_c: bool,
}

async fn read_bounded(reader: impl AsyncRead + Unpin, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .await?;
    ensure!(bytes.len() <= limit, "worker output exceeded {limit} bytes");
    Ok(bytes)
}

struct ProcessGroup(Option<u32>);

impl ProcessGroup {
    fn terminate(&mut self) {
        if let Some(pid) = self.0.take() {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
            #[cfg(not(unix))]
            let _ = pid;
        }
    }
}

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        // Also runs if a UI job drops/aborts this future while the worker is alive.
        self.terminate();
    }
}

impl ProcessBackend {
    pub async fn invoke(&mut self, request: &impl serde::Serialize) -> Result<BackendReply> {
        let lease = acquire_lease(&self.worker_lock)?;
        let mut command = Command::new(&self.executable);
        command
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .env("HF_HUB_OFFLINE", "1")
            .env("TRANSFORMERS_OFFLINE", "1")
            .env("HF_HUB_DISABLE_TELEMETRY", "1")
            .env("DO_NOT_TRACK", "1")
            .env("TOKENIZERS_PARALLELISM", "false");
        #[cfg(unix)]
        {
            use std::os::{fd::AsRawFd, unix::process::CommandExt};
            command.as_std_mut().process_group(0);
            let fd = lease.as_raw_fd();
            // Inherit only this lock. If the parent is killed, its worker still owns
            // the lease, so a second launch cannot overlap the orphaned model.
            unsafe {
                command.pre_exec(move || {
                    if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        let started = Instant::now();
        let mut child = command
            .spawn()
            .with_context(|| format!("cannot start worker {}", self.executable.display()))?;
        let pid = child.id().context("worker has no process ID")?;
        let mut group = ProcessGroup(Some(pid));
        let mut stdin = child.stdin.take().context("missing worker stdin")?;
        let stdout = child.stdout.take().context("missing worker stdout")?;
        let stderr = child.stderr.take().context("missing worker stderr")?;
        let mut input = serde_json::to_vec(request)?;
        input.push(b'\n');
        let result: Result<_> = tokio::select! {
            outcome = tokio::time::timeout(self.timeout, async {
                tokio::try_join!(
                    async { stdin.write_all(&input).await?; stdin.shutdown().await?; drop(stdin); Ok::<_,anyhow::Error>(()) },
                    read_bounded(stdout, 512 * 1024),
                    read_bounded(stderr, 256 * 1024),
                    async { child.wait().await.map_err(anyhow::Error::from) }
                )
            }) => match outcome { Ok(result) => result, Err(_) => Err(anyhow::anyhow!("worker timed out after {} seconds", self.timeout.as_secs())) },
            signal = tokio::signal::ctrl_c(), if self.listen_for_ctrl_c => {
                signal.context("cannot listen for Ctrl-C")?;
                Err(anyhow::anyhow!("generation cancelled with Ctrl-C"))
            }
        };
        // Reap on every path, including protocol errors and cancelled I/O. The
        // process group includes a GGUF worker's llama.cpp child, if present.
        group.terminate();
        if result.is_err() {
            let _ = child.start_kill();
        }
        let _ = child.wait().await;
        drop(lease);
        let (_, stdout, stderr, status) = result?;
        let diagnostic = String::from_utf8_lossy(&stderr);
        if !status.success() {
            bail!(
                "worker exited {status}: {}",
                diagnostic.chars().take(6000).collect::<String>()
            );
        }
        let mut reply: BackendReply = serde_json::from_slice(&stdout).with_context(|| {
            format!(
                "invalid worker envelope; stderr: {}",
                diagnostic.chars().take(4000).collect::<String>()
            )
        })?;
        ensure!(reply.protocol_version == 1, "unsupported worker protocol");
        ensure!(
            reply.metadata.is_object(),
            "worker metadata must be an object"
        );
        reply.metadata["worker_pid"] = pid.into();
        reply.metadata["worker_exited"] = true.into();
        reply.metadata["worker_total_ms"] = (started.elapsed().as_secs_f64() * 1000.0).into();
        reply.metadata["stderr"] = diagnostic.into_owned().into();
        Ok(reply)
    }
}

impl StoryBackend for ProcessBackend {
    async fn generate(&mut self, request: &StoryRequest) -> Result<BackendReply> {
        self.invoke(request).await
    }
}

impl ImageBackend for ProcessBackend {
    async fn generate_scene(
        &mut self,
        request: &crate::image_prompt::ImageRequest,
    ) -> Result<BackendReply> {
        self.invoke(request).await
    }
}
