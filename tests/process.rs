use hornbill::inference::{
    GenerationSettings, StoryBackend,
    process::{ProcessBackend, acquire_lease},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn request() -> hornbill::inference::StoryRequest {
    hornbill::prompt_builder::build(
        &hornbill::state::GameState::initial(),
        "Look",
        &[],
        &[],
        GenerationSettings {
            max_tokens: 1536,
            context_tokens: 6144,
            temperature: 0.6,
            seed: 42,
            memory_limit_bytes: 10 * 1024_u64.pow(3),
        },
        None,
    )
}

fn backend(dir: &TempDir, code: &str, timeout: u64) -> ProcessBackend {
    ProcessBackend {
        executable: PathBuf::from("python3"),
        args: vec!["-c".into(), code.into()],
        timeout: Duration::from_millis(timeout),
        worker_lock: dir.path().join("worker.lock"),
        listen_for_ctrl_c: false,
    }
}

#[tokio::test]
async fn stdout_protocol_and_stderr_are_drained_and_worker_is_reaped() {
    let dir = TempDir::new().unwrap();
    let mut worker = backend(
        &dir,
        "import json,sys; req=json.load(sys.stdin); sys.stderr.write('x'*100000); print(json.dumps({'protocol_version':1,'raw_text':'{}','metadata':{'request_version':req['protocol_version']},'error':None}))",
        5000,
    );
    let result = worker.generate(&request()).await.unwrap();
    assert_eq!(result.metadata["request_version"], 1);
    assert_eq!(result.metadata["worker_exited"], true);
    #[cfg(unix)]
    assert_eq!(
        unsafe { libc::kill(result.metadata["worker_pid"].as_i64().unwrap() as i32, 0) },
        -1
    );
    assert!(acquire_lease(&dir.path().join("worker.lock")).is_ok());
}

#[tokio::test]
async fn timeout_kills_worker_and_releases_the_single_flight_lease() {
    let dir = TempDir::new().unwrap();
    let mut worker = backend(
        &dir,
        "import sys,time; sys.stdin.read(); time.sleep(30)",
        150,
    );
    let began = Instant::now();
    assert!(
        worker
            .generate(&request())
            .await
            .unwrap_err()
            .to_string()
            .contains("timed out")
    );
    assert!(began.elapsed() < Duration::from_secs(3));
    assert!(acquire_lease(&dir.path().join("worker.lock")).is_ok());
}

#[tokio::test]
async fn oversized_stdout_is_rejected_without_waiting_for_process_timeout() {
    let dir = TempDir::new().unwrap();
    let mut worker = backend(
        &dir,
        "import sys,time; sys.stdin.read(); sys.stdout.write('x'*600000); sys.stdout.flush(); time.sleep(30)",
        30000,
    );
    let began = Instant::now();
    assert!(
        worker
            .generate(&request())
            .await
            .unwrap_err()
            .to_string()
            .contains("exceeded")
    );
    assert!(began.elapsed() < Duration::from_secs(3));
}

#[tokio::test]
async fn nonzero_exit_is_not_an_accepted_generation() {
    let dir = TempDir::new().unwrap();
    let mut worker = backend(
        &dir,
        "import sys; sys.stdin.read(); sys.stderr.write('model failure'); sys.exit(7)",
        5000,
    );
    assert!(
        worker
            .generate(&request())
            .await
            .unwrap_err()
            .to_string()
            .contains("model failure")
    );
}

#[test]
fn two_process_leases_cannot_overlap() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("worker.lock");
    let lease = acquire_lease(&path).unwrap();
    assert!(acquire_lease(&path).is_err());
    drop(lease);
    assert!(acquire_lease(&path).is_ok());
}

#[tokio::test]
async fn worker_cannot_leave_an_inference_descendant_running() {
    let dir = TempDir::new().unwrap();
    let marker = dir.path().join("descendant-survived");
    let code = "import json,subprocess,sys; sys.stdin.read(); subprocess.Popen([sys.executable,'-c','import sys,time,pathlib; time.sleep(0.7); pathlib.Path(sys.argv[1]).write_text(\"survived\")',sys.argv[1]],stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL); print(json.dumps({'protocol_version':1,'raw_text':'{}','metadata':{},'error':None}))";
    let mut worker = backend(&dir, code, 5000);
    worker.args.push(marker.display().to_string());
    worker.generate(&request()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1000)).await;
    assert!(!marker.exists(), "worker left a model descendant running");
}

#[tokio::test]
async fn aborting_a_ui_job_kills_the_worker_group() {
    let dir = TempDir::new().unwrap();
    let marker = dir.path().join("orphan-marker");
    let mut worker = backend(
        &dir,
        "import subprocess,sys,time;sys.stdin.read();subprocess.Popen([sys.executable,'-c','import pathlib,sys,time;time.sleep(0.8);pathlib.Path(sys.argv[1]).write_text(\"alive\")',sys.argv[1]],stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);time.sleep(30)",
        30000,
    );
    worker.args.push(marker.display().to_string());
    let task = tokio::spawn(async move { worker.generate(&request()).await });
    tokio::time::sleep(Duration::from_millis(200)).await;
    task.abort();
    let _ = task.await;
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert!(!marker.exists());
    assert!(acquire_lease(&dir.path().join("worker.lock")).is_ok());
}
