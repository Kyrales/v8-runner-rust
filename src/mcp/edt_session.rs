use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::sync::{oneshot, OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

use crate::config::model::AppConfig;
use crate::platform::interactive::{
    InteractiveCommandOutput, InteractiveProcessError, InteractiveProcessExecutor,
    InteractiveProcessRequest, ShutdownOutcome,
};
use crate::platform::locator::UtilityType;
use crate::platform::utilities::PlatformUtilities;

/// One raw prompt-delimited command submitted to the shared EDT actor.
#[derive(Debug, Clone)]
pub struct EdtSessionRequest {
    /// Raw command line sent into the interactive `1cedtcli` prompt.
    pub command: String,
    /// Absolute deadline covering both queue wait and execution time.
    pub deadline: Instant,
    /// Cooperative cancellation token observed while queued and by the caller while running.
    pub cancellation: CancellationToken,
}

impl EdtSessionRequest {
    /// Creates a request with an uncancelled token.
    pub fn new(command: impl Into<String>, deadline: Instant) -> Self {
        Self {
            command: command.into(),
            deadline,
            cancellation: CancellationToken::new(),
        }
    }

    /// Overrides the cancellation token carried by this request.
    pub fn with_cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = cancellation;
        self
    }
}

/// Successful output of a prompt-delimited interactive EDT command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdtSessionResponse {
    pub stdout: String,
    pub stderr: String,
}

/// Reason why the actor drained pending work without executing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdtSessionDrainReason {
    Restart,
    Shutdown,
}

/// Typed failure contract for the shared EDT actor.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum EdtSessionError {
    #[error("shared EDT queue is full")]
    QueueFull,

    #[error("shared EDT request was cancelled while queued")]
    QueuedCancelled,

    #[error("shared EDT request timed out while queued")]
    QueuedTimeout,

    #[error("shared EDT request was cancelled while running")]
    RunningCancelled,

    #[error("shared EDT request timed out while running")]
    RunningTimeout,

    #[error("failed to start shared EDT session: {message}")]
    StartupFailed { message: String },

    #[error("shared EDT session failed: {message}")]
    SessionFailed { message: String },

    #[error("shared EDT actor drained queued work because of {reason:?}")]
    DrainedByRestartOrShutdown { reason: EdtSessionDrainReason },

    #[error("shared EDT actor failed internally: {message}")]
    InternalFailure { message: String },
}

/// Shutdown failures for the shared EDT actor.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum EdtSessionShutdownError {
    #[error("shared EDT actor did not stop within {timeout_ms}ms")]
    TimedOut { timeout_ms: u64 },

    #[error("shared EDT actor worker thread panicked")]
    WorkerPanicked,

    #[error("shared EDT actor failed internally during shutdown: {message}")]
    Internal { message: String },
}

/// Shared single-session EDT actor reserved for MCP mode.
#[derive(Clone)]
pub struct EdtSessionManager {
    inner: Arc<EdtSessionManagerInner>,
}

impl EdtSessionManager {
    /// Creates the production manager using existing MCP concurrency/shutdown settings.
    pub fn for_config(config: &AppConfig) -> Result<Self, EdtSessionError> {
        let queue_capacity = config.mcp.execution.max_concurrent_calls.max(1);
        let shutdown_timeout =
            Duration::from_secs(config.mcp.execution.shutdown_grace_period_secs.max(1));
        Self::with_factory(
            Arc::new(DefaultSessionFactory::new(config.clone())),
            queue_capacity,
            shutdown_timeout,
        )
    }

    /// Submits one raw EDT command to the shared actor.
    pub async fn execute(
        &self,
        request: EdtSessionRequest,
    ) -> Result<EdtSessionResponse, EdtSessionError> {
        if self.inner.shutdown_started.load(Ordering::SeqCst) {
            return Err(EdtSessionError::DrainedByRestartOrShutdown {
                reason: EdtSessionDrainReason::Shutdown,
            });
        }

        let permit = Some(
            self.inner
                .admission
                .clone()
                .try_acquire_owned()
                .map_err(|_| EdtSessionError::QueueFull)?,
        );
        let (response_tx, mut response_rx) = oneshot::channel();
        let deadline = tokio::time::Instant::from_std(request.deadline);
        let cancellation = request.cancellation.clone();
        let queued = Arc::new(QueuedRequest {
            request,
            state: Arc::new(RequestState::queued(permit)),
            response_tx: Mutex::new(Some(response_tx)),
        });
        let state = queued.state.clone();
        {
            let mut queue =
                self.inner
                    .queue
                    .lock()
                    .map_err(|_| EdtSessionError::InternalFailure {
                        message: "shared EDT queue lock was poisoned".to_owned(),
                    })?;
            if self.inner.shutdown_started.load(Ordering::SeqCst) {
                return Err(EdtSessionError::DrainedByRestartOrShutdown {
                    reason: EdtSessionDrainReason::Shutdown,
                });
            }
            queue.push_back(queued.clone());
        }
        self.inner.queue_ready.notify_one();

        let remove_if_still_queued = |error: EdtSessionError| {
            if state.release_queued() {
                let _ = self.inner.remove_queued(&queued);
                return Some(error);
            }
            None
        };
        let queued_shutdown_error = EdtSessionError::DrainedByRestartOrShutdown {
            reason: EdtSessionDrainReason::Shutdown,
        };
        let remove_shutdown_if_queued = || {
            if state.release_queued() {
                let _ = self.inner.remove_queued(&queued);
                return Some(queued_shutdown_error.clone());
            }
            None
        };
        let mut shutdown_armed = true;

        loop {
            tokio::select! {
                biased;
                response = &mut response_rx => {
                    return response.unwrap_or_else(|_| {
                        Err(EdtSessionError::InternalFailure {
                            message: "shared EDT worker dropped response channel".to_owned(),
                        })
                    });
                }
                _ = cancellation.cancelled() => {
                    if let Some(error) = remove_if_still_queued(EdtSessionError::QueuedCancelled) {
                        return Err(error);
                    }
                    if state.is_running() {
                        return Err(EdtSessionError::RunningCancelled);
                    }
                    continue;
                }
                _ = tokio::time::sleep_until(deadline) => {
                    if let Some(error) = remove_if_still_queued(EdtSessionError::QueuedTimeout) {
                        return Err(error);
                    }
                    if state.is_running() {
                        return Err(EdtSessionError::RunningTimeout);
                    }
                    continue;
                }
                _ = self.inner.shutdown_token.cancelled(), if shutdown_armed => {
                    if let Some(error) = remove_shutdown_if_queued() {
                        return Err(error);
                    }
                    shutdown_armed = false;
                    continue;
                }
            }
        }
    }

    /// Stops admission, drains queued work, and waits for the worker thread to exit.
    pub fn shutdown(&self) -> Result<(), EdtSessionShutdownError> {
        self.inner
            .begin_shutdown()
            .map_err(|message| EdtSessionShutdownError::Internal { message })?;
        self.inner.join_worker()
    }

    fn with_factory(
        factory: Arc<dyn SessionFactory>,
        queue_capacity: usize,
        shutdown_timeout: Duration,
    ) -> Result<Self, EdtSessionError> {
        let inner = Arc::new(EdtSessionManagerInner {
            queue: Mutex::new(VecDeque::new()),
            queue_ready: Condvar::new(),
            admission: Arc::new(Semaphore::new(queue_capacity.max(1))),
            shutdown_token: CancellationToken::new(),
            shutdown_started: AtomicBool::new(false),
            shutdown_timed_out: AtomicBool::new(false),
            shutdown_timeout,
            active_pid: Arc::new(AtomicU32::new(0)),
            worker: Mutex::new(None),
        });
        let worker_inner = inner.clone();
        let worker_factory = factory.clone();
        let worker = thread::Builder::new()
            .name("v8tr-edt-session".to_owned())
            .spawn(move || run_worker(worker_inner, worker_factory))
            .map_err(|error| EdtSessionError::InternalFailure {
                message: format!("failed to spawn shared EDT worker thread: {error}"),
            })?;
        inner
            .worker
            .lock()
            .map_err(|_| EdtSessionError::InternalFailure {
                message: "shared EDT worker lock was poisoned during startup".to_owned(),
            })?
            .replace(worker);

        Ok(Self { inner })
    }
}

impl Drop for EdtSessionManager {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) <= 2 {
            let inner = self.inner.clone();
            let _ = inner.begin_shutdown();
            let _ = thread::Builder::new()
                .name("v8tr-edt-drop".to_owned())
                .spawn(move || {
                    let _ = inner.join_worker();
                });
        }
    }
}

struct EdtSessionManagerInner {
    queue: Mutex<VecDeque<Arc<QueuedRequest>>>,
    queue_ready: Condvar,
    admission: Arc<Semaphore>,
    shutdown_token: CancellationToken,
    shutdown_started: AtomicBool,
    shutdown_timed_out: AtomicBool,
    shutdown_timeout: Duration,
    active_pid: Arc<AtomicU32>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl EdtSessionManagerInner {
    fn begin_shutdown(&self) -> Result<(), String> {
        if self.shutdown_started.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        self.shutdown_token.cancel();
        self.queue_ready.notify_all();
        Ok(())
    }

    fn remove_queued(&self, target: &Arc<QueuedRequest>) -> Result<bool, EdtSessionError> {
        let mut queue = self
            .queue
            .lock()
            .map_err(|_| EdtSessionError::InternalFailure {
                message: "shared EDT queue lock was poisoned".to_owned(),
            })?;
        let Some(position) = queue.iter().position(|queued| Arc::ptr_eq(queued, target)) else {
            return Ok(false);
        };
        queue.remove(position);
        Ok(true)
    }

    fn next_request(&self) -> Option<Arc<QueuedRequest>> {
        let mut queue = match self.queue.lock() {
            Ok(queue) => queue,
            Err(poisoned) => poisoned.into_inner(),
        };
        loop {
            if let Some(queued) = queue.pop_front() {
                return Some(queued);
            }
            if self.shutdown_started.load(Ordering::SeqCst) {
                return None;
            }
            queue = match self.queue_ready.wait(queue) {
                Ok(queue) => queue,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
    }

    fn drain_pending(&self, error: EdtSessionError) {
        let drained = {
            let mut queue = match self.queue.lock() {
                Ok(queue) => queue,
                Err(poisoned) => poisoned.into_inner(),
            };
            queue.drain(..).collect::<Vec<_>>()
        };
        for queued in drained {
            queued.state.release_queued();
            queued.reply(Err(error.clone()));
        }
    }

    fn take_worker(&self) -> Result<Option<JoinHandle<()>>, EdtSessionShutdownError> {
        self.worker
            .lock()
            .map(|mut guard| guard.take())
            .map_err(|_| EdtSessionShutdownError::Internal {
                message: "shared EDT worker lock was poisoned".to_owned(),
            })
    }

    fn join_worker(&self) -> Result<(), EdtSessionShutdownError> {
        if self.shutdown_timed_out.load(Ordering::SeqCst) {
            return Err(EdtSessionShutdownError::TimedOut {
                timeout_ms: self.shutdown_timeout.as_millis() as u64,
            });
        }

        let Some(worker) = self.take_worker()? else {
            return Ok(());
        };

        let timeout = self.shutdown_timeout;
        let pid = self.active_pid.clone();
        if wait_for_worker(&worker, timeout) {
            return match worker.join() {
                Ok(()) => Ok(()),
                Err(_) => Err(EdtSessionShutdownError::WorkerPanicked),
            };
        }

        let pid = pid.swap(0, Ordering::SeqCst);
        let _ = kill_process_group_by_pid(pid);
        if wait_for_worker(&worker, timeout) {
            return match worker.join() {
                Ok(()) => Ok(()),
                Err(_) => Err(EdtSessionShutdownError::WorkerPanicked),
            };
        }

        self.shutdown_timed_out.store(true, Ordering::SeqCst);
        Err(EdtSessionShutdownError::TimedOut {
            timeout_ms: timeout.as_millis() as u64,
        })
    }
}

struct RequestState {
    stage: AtomicU8,
    permit: Mutex<Option<OwnedSemaphorePermit>>,
}

impl RequestState {
    fn queued(permit: Option<OwnedSemaphorePermit>) -> Self {
        Self {
            stage: AtomicU8::new(REQUEST_QUEUED),
            permit: Mutex::new(permit),
        }
    }

    fn release_queued(&self) -> bool {
        if self
            .stage
            .compare_exchange(
                REQUEST_QUEUED,
                REQUEST_DONE,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
        {
            self.release_permit();
            return true;
        }
        false
    }

    fn try_mark_running(&self) -> bool {
        self.stage
            .compare_exchange(
                REQUEST_QUEUED,
                REQUEST_RUNNING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }

    fn is_running(&self) -> bool {
        self.stage.load(Ordering::SeqCst) == REQUEST_RUNNING
    }

    fn finish(&self) {
        self.stage.store(REQUEST_DONE, Ordering::SeqCst);
        self.release_permit();
    }

    fn release_permit(&self) {
        match self.permit.lock() {
            Ok(mut guard) => {
                guard.take();
            }
            Err(poisoned) => {
                poisoned.into_inner().take();
            }
        }
    }
}

struct QueuedRequest {
    request: EdtSessionRequest,
    state: Arc<RequestState>,
    response_tx: Mutex<Option<oneshot::Sender<Result<EdtSessionResponse, EdtSessionError>>>>,
}

impl QueuedRequest {
    fn reply(&self, result: Result<EdtSessionResponse, EdtSessionError>) {
        let response_tx = match self.response_tx.lock() {
            Ok(mut guard) => guard.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some(response_tx) = response_tx {
            let _ = response_tx.send(result);
        }
    }
}

trait ManagedSession: Send {
    fn pid(&self) -> Option<u32>;

    fn execute(
        &mut self,
        command: &str,
        timeout: Duration,
    ) -> Result<InteractiveCommandOutput, InteractiveProcessError>;

    fn shutdown(&mut self, timeout: Duration) -> Result<ShutdownOutcome, InteractiveProcessError>;

    fn kill(&mut self) -> Result<(), InteractiveProcessError>;
}

impl ManagedSession for InteractiveProcessExecutor {
    fn pid(&self) -> Option<u32> {
        Self::pid(self)
    }

    fn execute(
        &mut self,
        command: &str,
        timeout: Duration,
    ) -> Result<InteractiveCommandOutput, InteractiveProcessError> {
        Self::execute(self, command, timeout)
    }

    fn shutdown(&mut self, timeout: Duration) -> Result<ShutdownOutcome, InteractiveProcessError> {
        Self::shutdown(self, timeout)
    }

    fn kill(&mut self) -> Result<(), InteractiveProcessError> {
        Self::kill(self)
    }
}

trait SessionFactory: Send + Sync {
    fn spawn_session(&self) -> Result<Box<dyn ManagedSession>, EdtSessionError>;

    fn pre_dispatch(
        &self,
        _session: &mut dyn ManagedSession,
        _request: &EdtSessionRequest,
    ) -> Result<(), EdtSessionError> {
        Ok(())
    }
}

#[derive(Clone)]
struct DefaultSessionFactory {
    config: AppConfig,
}

impl DefaultSessionFactory {
    fn new(config: AppConfig) -> Self {
        Self { config }
    }
}

impl SessionFactory for DefaultSessionFactory {
    fn spawn_session(&self) -> Result<Box<dyn ManagedSession>, EdtSessionError> {
        let mut utilities = PlatformUtilities::from_config(&self.config);
        let location = utilities.locate(UtilityType::EdtCli).map_err(|error| {
            EdtSessionError::StartupFailed {
                message: error.to_string(),
            }
        })?;
        let request = InteractiveProcessRequest::new(location.path).with_args([
            "-data".to_owned(),
            self.config
                .work_path
                .join("edt-workspace")
                .display()
                .to_string(),
        ]);
        InteractiveProcessExecutor::spawn(
            request,
            Duration::from_millis(self.config.tools.edt_cli.startup_timeout_ms),
        )
        .map(|session| Box::new(session) as Box<dyn ManagedSession>)
        .map_err(|error| EdtSessionError::StartupFailed {
            message: error.to_string(),
        })
    }
}

fn run_worker(inner: Arc<EdtSessionManagerInner>, factory: Arc<dyn SessionFactory>) {
    let mut session: Option<Box<dyn ManagedSession>> = None;
    while let Some(queued) = inner.next_request() {
        if inner.shutdown_token.is_cancelled() {
            queued.state.release_queued();
            queued.reply(Err(EdtSessionError::DrainedByRestartOrShutdown {
                reason: EdtSessionDrainReason::Shutdown,
            }));
            inner.drain_pending(EdtSessionError::DrainedByRestartOrShutdown {
                reason: EdtSessionDrainReason::Shutdown,
            });
            break;
        }

        if queued.request.cancellation.is_cancelled() {
            queued.state.release_queued();
            queued.reply(Err(EdtSessionError::QueuedCancelled));
            continue;
        }
        if is_deadline_exhausted(queued.request.deadline) {
            queued.state.release_queued();
            queued.reply(Err(EdtSessionError::QueuedTimeout));
            continue;
        }

        if session.is_none() {
            match factory.spawn_session() {
                Ok(new_session) => {
                    inner
                        .active_pid
                        .store(new_session.pid().unwrap_or(0), Ordering::SeqCst);
                    session = Some(new_session);
                }
                Err(error) => {
                    queued.state.release_queued();
                    queued.reply(Err(error));
                    inner.drain_pending(EdtSessionError::DrainedByRestartOrShutdown {
                        reason: EdtSessionDrainReason::Restart,
                    });
                    inner.active_pid.store(0, Ordering::SeqCst);
                    continue;
                }
            }
        }

        let Some(active_session) = session.as_mut() else {
            queued.reply(Err(EdtSessionError::InternalFailure {
                message: "shared EDT worker lost session after startup".to_owned(),
            }));
            continue;
        };

        if let Err(error) = factory.pre_dispatch(active_session.as_mut(), &queued.request) {
            queued.state.release_queued();
            queued.reply(Err(error));
            kill_and_drop_session(&mut session, inner.active_pid.as_ref());
            inner.drain_pending(EdtSessionError::DrainedByRestartOrShutdown {
                reason: EdtSessionDrainReason::Restart,
            });
            continue;
        }

        if queued.request.cancellation.is_cancelled() {
            queued.state.release_queued();
            queued.reply(Err(EdtSessionError::QueuedCancelled));
            continue;
        }
        let remaining = queued
            .request
            .deadline
            .saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            queued.state.release_queued();
            queued.reply(Err(EdtSessionError::QueuedTimeout));
            continue;
        }
        if !queued.state.try_mark_running() {
            continue;
        }
        if queued.request.cancellation.is_cancelled() {
            queued.state.finish();
            queued.reply(Err(EdtSessionError::RunningCancelled));
            continue;
        }
        let execution = active_session.execute(&queued.request.command, remaining);
        match execution {
            Ok(output) => {
                queued.state.finish();
                queued.reply(Ok(EdtSessionResponse {
                    stdout: output.stdout,
                    stderr: output.stderr,
                }));
            }
            Err(InteractiveProcessError::CommandTimeout { .. }) => {
                queued.state.finish();
                queued.reply(Err(EdtSessionError::RunningTimeout));
                kill_and_drop_session(&mut session, inner.active_pid.as_ref());
                inner.drain_pending(EdtSessionError::DrainedByRestartOrShutdown {
                    reason: EdtSessionDrainReason::Restart,
                });
            }
            Err(error) => {
                queued.state.finish();
                queued.reply(Err(EdtSessionError::SessionFailed {
                    message: error.to_string(),
                }));
                kill_and_drop_session(&mut session, inner.active_pid.as_ref());
                inner.drain_pending(EdtSessionError::DrainedByRestartOrShutdown {
                    reason: EdtSessionDrainReason::Restart,
                });
            }
        }
    }

    shutdown_session(
        &mut session,
        inner.shutdown_timeout,
        inner.active_pid.as_ref(),
    );
    inner.active_pid.store(0, Ordering::SeqCst);
}

fn kill_and_drop_session(session: &mut Option<Box<dyn ManagedSession>>, active_pid: &AtomicU32) {
    if let Some(mut session) = session.take() {
        if session.kill().is_err() {
            let pid = active_pid.load(Ordering::SeqCst);
            let _ = kill_process_group_by_pid(pid);
        }
    } else {
        let pid = active_pid.load(Ordering::SeqCst);
        let _ = kill_process_group_by_pid(pid);
    }
    active_pid.store(0, Ordering::SeqCst);
}

fn shutdown_session(
    session: &mut Option<Box<dyn ManagedSession>>,
    timeout: Duration,
    active_pid: &AtomicU32,
) {
    if let Some(mut session) = session.take() {
        if session.shutdown(timeout).is_err() {
            if session.kill().is_err() {
                let pid = active_pid.load(Ordering::SeqCst);
                let _ = kill_process_group_by_pid(pid);
            }
        }
    } else {
        let pid = active_pid.load(Ordering::SeqCst);
        let _ = kill_process_group_by_pid(pid);
    }
    active_pid.store(0, Ordering::SeqCst);
}

fn is_deadline_exhausted(deadline: Instant) -> bool {
    deadline <= Instant::now()
}

fn wait_for_worker(worker: &JoinHandle<()>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if worker.is_finished() {
            return true;
        }
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        thread::sleep(JOIN_POLL_INTERVAL.min(deadline.saturating_duration_since(now)));
    }
}

const REQUEST_QUEUED: u8 = 0;
const REQUEST_RUNNING: u8 = 1;
const REQUEST_DONE: u8 = 2;
const JOIN_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[cfg(unix)]
fn kill_process_group_by_pid(pid: u32) -> std::io::Result<()> {
    if pid == 0 {
        return Ok(());
    }
    unsafe {
        let pgid = -(pid as i32);
        if libc::kill(pgid, libc::SIGKILL) != 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn kill_process_group_by_pid(_pid: u32) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        EdtSessionDrainReason, EdtSessionError, EdtSessionManager, EdtSessionRequest,
        EdtSessionShutdownError, ManagedSession, SessionFactory,
    };
    use crate::platform::interactive::{
        InteractiveCommandOutput, InteractiveProcessError, ShutdownOutcome,
    };
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};
    use tokio::time::sleep;
    use tokio_util::sync::CancellationToken;

    #[derive(Clone)]
    struct FakeSessionFactory {
        plans: Arc<Mutex<VecDeque<SessionPlan>>>,
        starts: Arc<AtomicUsize>,
        commands: Arc<Mutex<Vec<String>>>,
        pre_dispatches: Arc<AtomicUsize>,
        pre_dispatch_delay: Duration,
        shutdowns: Arc<AtomicUsize>,
        next_pid: Arc<AtomicU32>,
    }

    impl FakeSessionFactory {
        fn new(plans: Vec<SessionPlan>) -> Self {
            Self {
                plans: Arc::new(Mutex::new(plans.into())),
                starts: Arc::new(AtomicUsize::new(0)),
                commands: Arc::new(Mutex::new(Vec::new())),
                pre_dispatches: Arc::new(AtomicUsize::new(0)),
                pre_dispatch_delay: Duration::ZERO,
                shutdowns: Arc::new(AtomicUsize::new(0)),
                next_pid: Arc::new(AtomicU32::new(41)),
            }
        }

        fn with_pre_dispatch_delay(mut self, delay: Duration) -> Self {
            self.pre_dispatch_delay = delay;
            self
        }

        fn start_count(&self) -> usize {
            self.starts.load(Ordering::SeqCst)
        }

        fn commands(&self) -> Vec<String> {
            self.commands.lock().expect("commands lock").clone()
        }

        fn shutdown_count(&self) -> usize {
            self.shutdowns.load(Ordering::SeqCst)
        }
    }

    impl SessionFactory for FakeSessionFactory {
        fn spawn_session(&self) -> Result<Box<dyn ManagedSession>, EdtSessionError> {
            self.starts.fetch_add(1, Ordering::SeqCst);
            let plan = self
                .plans
                .lock()
                .expect("plans lock")
                .pop_front()
                .expect("session plan");
            match plan {
                SessionPlan::StartupFailure { message, delay } => {
                    thread::sleep(delay);
                    Err(EdtSessionError::StartupFailed { message })
                }
                SessionPlan::Session(behaviors) => Ok(Box::new(FakeSession::new(
                    self.next_pid.fetch_add(1, Ordering::SeqCst),
                    self.commands.clone(),
                    self.shutdowns.clone(),
                    behaviors,
                ))),
            }
        }

        fn pre_dispatch(
            &self,
            _session: &mut dyn ManagedSession,
            _request: &EdtSessionRequest,
        ) -> Result<(), EdtSessionError> {
            self.pre_dispatches.fetch_add(1, Ordering::SeqCst);
            if !self.pre_dispatch_delay.is_zero() {
                thread::sleep(self.pre_dispatch_delay);
            }
            Ok(())
        }
    }

    enum SessionPlan {
        StartupFailure { message: String, delay: Duration },
        Session(Vec<CommandBehavior>),
    }

    enum CommandBehavior {
        CompleteAfter {
            delay: Duration,
            stdout: String,
            stderr: String,
        },
        FatalProcessExitAfter {
            delay: Duration,
        },
    }

    struct FakeSession {
        pid: u32,
        commands: Arc<Mutex<Vec<String>>>,
        shutdowns: Arc<AtomicUsize>,
        behaviors: Mutex<VecDeque<CommandBehavior>>,
        killed: AtomicBool,
    }

    impl FakeSession {
        fn new(
            pid: u32,
            commands: Arc<Mutex<Vec<String>>>,
            shutdowns: Arc<AtomicUsize>,
            behaviors: Vec<CommandBehavior>,
        ) -> Self {
            Self {
                pid,
                commands,
                shutdowns,
                behaviors: Mutex::new(behaviors.into()),
                killed: AtomicBool::new(false),
            }
        }
    }

    impl ManagedSession for FakeSession {
        fn pid(&self) -> Option<u32> {
            Some(self.pid)
        }

        fn execute(
            &mut self,
            command: &str,
            timeout: Duration,
        ) -> Result<InteractiveCommandOutput, InteractiveProcessError> {
            self.commands
                .lock()
                .expect("commands lock")
                .push(command.to_owned());
            let behavior = self
                .behaviors
                .lock()
                .expect("behaviors lock")
                .pop_front()
                .expect("command behavior");
            match behavior {
                CommandBehavior::CompleteAfter {
                    delay,
                    stdout,
                    stderr,
                } => {
                    if delay > timeout {
                        thread::sleep(timeout + Duration::from_millis(5));
                        Err(InteractiveProcessError::CommandTimeout {
                            command: command.to_owned(),
                            timeout_ms: timeout.as_millis() as u64,
                            stdout: String::new(),
                            stderr: String::new(),
                        })
                    } else {
                        thread::sleep(delay);
                        Ok(InteractiveCommandOutput { stdout, stderr })
                    }
                }
                CommandBehavior::FatalProcessExitAfter { delay } => {
                    thread::sleep(delay);
                    Err(InteractiveProcessError::ProcessExited {
                        exit_code: 17,
                        stdout: String::new(),
                        stderr: String::new(),
                    })
                }
            }
        }

        fn shutdown(
            &mut self,
            _timeout: Duration,
        ) -> Result<ShutdownOutcome, InteractiveProcessError> {
            self.shutdowns.fetch_add(1, Ordering::SeqCst);
            Ok(ShutdownOutcome::Graceful { exit_code: 0 })
        }

        fn kill(&mut self) -> Result<(), InteractiveProcessError> {
            self.killed.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    fn manager(
        factory: FakeSessionFactory,
        capacity: usize,
        shutdown: Duration,
    ) -> EdtSessionManager {
        EdtSessionManager::with_factory(Arc::new(factory), capacity, shutdown)
            .expect("create edt session manager")
    }

    fn queued_len(manager: &EdtSessionManager) -> usize {
        manager.inner.queue.lock().expect("queue lock").len()
    }

    fn request(command: &str, after_ms: u64) -> EdtSessionRequest {
        EdtSessionRequest::new(command, Instant::now() + Duration::from_millis(after_ms))
    }

    async fn wait_for_commands(factory: &FakeSessionFactory, expected: usize) {
        for _ in 0..50 {
            if factory.commands().len() >= expected {
                return;
            }
            sleep(Duration::from_millis(5)).await;
        }
        panic!("timed out waiting for {expected} commands");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn single_flight_startup_reuses_one_session_for_multiple_calls() {
        let factory = FakeSessionFactory::new(vec![SessionPlan::Session(vec![
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(20),
                stdout: "one".to_owned(),
                stderr: String::new(),
            },
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "two".to_owned(),
                stderr: String::new(),
            },
        ])]);
        let manager = manager(factory.clone(), 2, Duration::from_millis(50));

        let first = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-1", 200)).await }
        });
        let second = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-2", 200)).await }
        });

        assert_eq!(
            first
                .await
                .expect("first join")
                .expect("first result")
                .stdout,
            "one"
        );
        assert_eq!(
            second
                .await
                .expect("second join")
                .expect("second result")
                .stdout,
            "two"
        );
        assert_eq!(factory.start_count(), 1);
        assert_eq!(
            factory.commands(),
            vec!["cmd-1".to_owned(), "cmd-2".to_owned()]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn queue_full_counts_running_and_queued_admission() {
        let factory = FakeSessionFactory::new(vec![SessionPlan::Session(vec![
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(40),
                stdout: "ok".to_owned(),
                stderr: String::new(),
            },
        ])]);
        let manager = manager(factory.clone(), 1, Duration::from_millis(50));

        let first = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-1", 200)).await }
        });
        wait_for_commands(&factory, 1).await;

        let second = manager.execute(request("cmd-2", 200)).await;

        assert_eq!(second, Err(EdtSessionError::QueueFull));
        assert!(first.await.expect("first join").is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn queued_cancellation_returns_early_before_execution() {
        let factory = FakeSessionFactory::new(vec![SessionPlan::Session(vec![
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(50),
                stdout: "first".to_owned(),
                stderr: String::new(),
            },
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "third".to_owned(),
                stderr: String::new(),
            },
        ])]);
        let manager = manager(factory.clone(), 2, Duration::from_millis(100));

        let running = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-1", 300)).await }
        });
        wait_for_commands(&factory, 1).await;

        let cancellation = CancellationToken::new();
        let queued = tokio::spawn({
            let manager = manager.clone();
            let cancellation = cancellation.clone();
            async move {
                manager
                    .execute(request("cmd-2", 300).with_cancellation(cancellation))
                    .await
            }
        });
        sleep(Duration::from_millis(10)).await;
        cancellation.cancel();

        assert_eq!(
            queued.await.expect("queued join"),
            Err(EdtSessionError::QueuedCancelled)
        );
        assert_eq!(
            manager
                .execute(request("cmd-3", 300))
                .await
                .expect("third result")
                .stdout,
            "third"
        );
        assert!(running.await.expect("running join").is_ok());
        assert_eq!(
            factory.commands(),
            vec!["cmd-1".to_owned(), "cmd-3".to_owned()]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn queued_timeout_returns_early_before_execution() {
        let factory = FakeSessionFactory::new(vec![SessionPlan::Session(vec![
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(50),
                stdout: "first".to_owned(),
                stderr: String::new(),
            },
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "third".to_owned(),
                stderr: String::new(),
            },
        ])]);
        let manager = manager(factory.clone(), 2, Duration::from_millis(100));

        let running = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-1", 300)).await }
        });
        wait_for_commands(&factory, 1).await;

        let queued = manager.execute(request("cmd-2", 20)).await;

        assert_eq!(queued, Err(EdtSessionError::QueuedTimeout));
        assert_eq!(
            manager
                .execute(request("cmd-3", 300))
                .await
                .expect("third result")
                .stdout,
            "third"
        );
        assert!(running.await.expect("running join").is_ok());
        assert_eq!(
            factory.commands(),
            vec!["cmd-1".to_owned(), "cmd-3".to_owned()]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn queued_cancellation_removes_entries_from_internal_queue() {
        let factory = FakeSessionFactory::new(vec![SessionPlan::Session(vec![
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(60),
                stdout: "first".to_owned(),
                stderr: String::new(),
            },
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "second".to_owned(),
                stderr: String::new(),
            },
        ])]);
        let manager = manager(factory.clone(), 2, Duration::from_millis(100));

        let running = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-1", 300)).await }
        });
        wait_for_commands(&factory, 1).await;

        for _ in 0..20 {
            let cancellation = CancellationToken::new();
            cancellation.cancel();
            assert_eq!(
                manager
                    .execute(request("cancelled", 300).with_cancellation(cancellation))
                    .await,
                Err(EdtSessionError::QueuedCancelled)
            );
        }

        assert_eq!(queued_len(&manager), 0);
        assert_eq!(
            manager
                .execute(request("cmd-2", 300))
                .await
                .expect("second result")
                .stdout,
            "second"
        );
        assert!(running.await.expect("running join").is_ok());
        assert_eq!(
            factory.commands(),
            vec!["cmd-1".to_owned(), "cmd-2".to_owned()]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn deadline_expiring_during_pre_dispatch_stays_queued_timeout_without_restart() {
        let factory = FakeSessionFactory::new(vec![SessionPlan::Session(vec![
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "second".to_owned(),
                stderr: String::new(),
            },
        ])])
        .with_pre_dispatch_delay(Duration::from_millis(20));
        let manager = manager(factory.clone(), 2, Duration::from_millis(100));

        assert_eq!(
            manager.execute(request("cmd-1", 10)).await,
            Err(EdtSessionError::QueuedTimeout)
        );
        assert_eq!(
            manager
                .execute(request("cmd-2", 200))
                .await
                .expect("second result")
                .stdout,
            "second"
        );
        assert_eq!(factory.start_count(), 1);
        assert_eq!(factory.commands(), vec!["cmd-2".to_owned()]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn running_cancellation_is_cooperative_and_capacity_recovers_after_completion() {
        let factory = FakeSessionFactory::new(vec![SessionPlan::Session(vec![
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(60),
                stdout: "first".to_owned(),
                stderr: String::new(),
            },
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "second".to_owned(),
                stderr: String::new(),
            },
        ])]);
        let manager = manager(factory.clone(), 1, Duration::from_millis(100));
        let cancellation = CancellationToken::new();

        let running = tokio::spawn({
            let manager = manager.clone();
            let cancellation = cancellation.clone();
            async move {
                manager
                    .execute(request("cmd-1", 300).with_cancellation(cancellation))
                    .await
            }
        });
        wait_for_commands(&factory, 1).await;
        cancellation.cancel();

        assert_eq!(
            running.await.expect("running join"),
            Err(EdtSessionError::RunningCancelled)
        );
        assert_eq!(
            manager.execute(request("cmd-2", 300)).await,
            Err(EdtSessionError::QueueFull)
        );
        sleep(Duration::from_millis(70)).await;
        assert_eq!(
            manager
                .execute(request("cmd-2", 300))
                .await
                .expect("second result")
                .stdout,
            "second"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn running_timeout_forces_lazy_restart_and_drains_queued_calls() {
        let factory = FakeSessionFactory::new(vec![
            SessionPlan::Session(vec![CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(60),
                stdout: "late".to_owned(),
                stderr: String::new(),
            }]),
            SessionPlan::Session(vec![CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "fresh".to_owned(),
                stderr: String::new(),
            }]),
        ]);
        let manager = manager(factory.clone(), 2, Duration::from_millis(100));

        let first = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-1", 20)).await }
        });
        wait_for_commands(&factory, 1).await;

        let queued = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-2", 200)).await }
        });

        assert_eq!(
            first.await.expect("first join"),
            Err(EdtSessionError::RunningTimeout)
        );
        assert_eq!(
            queued.await.expect("queued join"),
            Err(EdtSessionError::DrainedByRestartOrShutdown {
                reason: EdtSessionDrainReason::Restart
            })
        );
        sleep(Duration::from_millis(40)).await;
        assert_eq!(
            manager
                .execute(request("cmd-3", 200))
                .await
                .expect("fresh result")
                .stdout,
            "fresh"
        );
        assert_eq!(factory.start_count(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn startup_failure_drains_queued_requests_without_retrying_failed_call() {
        let factory = FakeSessionFactory::new(vec![
            SessionPlan::StartupFailure {
                message: "boom".to_owned(),
                delay: Duration::from_millis(30),
            },
            SessionPlan::Session(vec![CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "ok".to_owned(),
                stderr: String::new(),
            }]),
        ]);
        let manager = manager(factory.clone(), 2, Duration::from_millis(100));

        let first = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-1", 200)).await }
        });
        let second = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-2", 200)).await }
        });

        let first_result = first.await.expect("first join");
        let second_result = second.await.expect("second join");
        let startup = Err(EdtSessionError::StartupFailed {
            message: "boom".to_owned(),
        });
        let drained = Err(EdtSessionError::DrainedByRestartOrShutdown {
            reason: EdtSessionDrainReason::Restart,
        });
        assert!(
            (first_result == startup && second_result == drained)
                || (first_result == drained && second_result == startup),
            "expected one startup failure and one drained request, got {first_result:?} and {second_result:?}"
        );
        assert_eq!(factory.commands(), Vec::<String>::new());
        assert_eq!(
            manager
                .execute(request("cmd-3", 200))
                .await
                .expect("third result")
                .stdout,
            "ok"
        );
        assert_eq!(factory.start_count(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fatal_session_error_drains_queue_and_restarts_lazily() {
        let factory = FakeSessionFactory::new(vec![
            SessionPlan::Session(vec![CommandBehavior::FatalProcessExitAfter {
                delay: Duration::from_millis(30),
            }]),
            SessionPlan::Session(vec![CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "ok".to_owned(),
                stderr: String::new(),
            }]),
        ]);
        let manager = manager(factory.clone(), 2, Duration::from_millis(100));

        let first = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-1", 200)).await }
        });
        wait_for_commands(&factory, 1).await;
        let second = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-2", 200)).await }
        });

        let first_result = first.await.expect("first join");
        assert!(matches!(
            first_result,
            Err(EdtSessionError::SessionFailed { .. })
        ));
        assert_eq!(
            second.await.expect("second join"),
            Err(EdtSessionError::DrainedByRestartOrShutdown {
                reason: EdtSessionDrainReason::Restart
            })
        );
        assert_eq!(factory.commands(), vec!["cmd-1".to_owned()]);
        assert_eq!(
            manager
                .execute(request("cmd-3", 200))
                .await
                .expect("third result")
                .stdout,
            "ok"
        );
        assert_eq!(factory.start_count(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shutdown_drains_queued_requests() {
        let factory = FakeSessionFactory::new(vec![SessionPlan::Session(vec![
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(40),
                stdout: "first".to_owned(),
                stderr: String::new(),
            },
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "second".to_owned(),
                stderr: String::new(),
            },
        ])]);
        let manager = manager(factory.clone(), 2, Duration::from_millis(200));

        let running = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-1", 300)).await }
        });
        wait_for_commands(&factory, 1).await;
        let queued = tokio::spawn({
            let manager = manager.clone();
            async move { manager.execute(request("cmd-2", 300)).await }
        });
        sleep(Duration::from_millis(10)).await;

        manager.shutdown().expect("shutdown");

        assert!(running.await.expect("running join").is_ok());
        assert_eq!(
            queued.await.expect("queued join"),
            Err(EdtSessionError::DrainedByRestartOrShutdown {
                reason: EdtSessionDrainReason::Shutdown
            })
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn drop_starts_background_shutdown_cleanup() {
        let factory = FakeSessionFactory::new(vec![SessionPlan::Session(vec![
            CommandBehavior::CompleteAfter {
                delay: Duration::from_millis(5),
                stdout: "ok".to_owned(),
                stderr: String::new(),
            },
        ])]);
        {
            let manager = manager(factory.clone(), 1, Duration::from_millis(50));
            assert_eq!(
                manager
                    .execute(request("cmd-1", 200))
                    .await
                    .expect("command result")
                    .stdout,
                "ok"
            );
        }

        for _ in 0..50 {
            if factory.shutdown_count() == 1 {
                return;
            }
            sleep(Duration::from_millis(5)).await;
        }
        panic!("timed out waiting for drop-driven shutdown cleanup");
    }

    #[test]
    fn repeated_shutdown_is_idempotent() {
        let factory = FakeSessionFactory::new(vec![SessionPlan::Session(vec![])]);
        let manager = manager(factory, 1, Duration::from_millis(20));

        manager.shutdown().expect("first shutdown");
        manager.shutdown().expect("second shutdown");
    }

    #[test]
    fn shutdown_timeout_surface_is_typed() {
        let error = EdtSessionShutdownError::TimedOut { timeout_ms: 10 };
        assert_eq!(
            error.to_string(),
            "shared EDT actor did not stop within 10ms"
        );
    }
}
