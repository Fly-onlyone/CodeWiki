//! The child process that holds a model, seen from both sides.
//!
//! A model is run by a hidden child of the program, never by the program
//! itself: native model code can end its process (out of memory, a fault in
//! a graphics driver), and a backend runs for weeks. The two speak JSON
//! lines over the child's standard input and output:
//!
//! ```text
//! child:  {"protocol":1,"device":"cpu","dimensions":768}        once, when the model is loaded;
//!                                                                or {"error":"..."} and it ends
//! parent: {"id":1,"kind":"document","texts":["...","..."]}
//! child:  {"id":1,"vectors":[[...],[...]]}                      or {"id":1,"error":"..."}
//! ```
//!
//! The child ends when its input closes, so none outlives its parent.
//!
//! [`serve`] is the child's loop; the engine crate gives it the function
//! that embeds. [`Supervisor`] is the parent's side: it starts the child on
//! the first request, gives up on one that hangs or dies, and stops one that
//! is idle.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cw_core::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::recommend::Device;

/// Version of the lines above. A child that speaks another one is refused,
/// which can only happen while an update replaces the programs.
pub const PROTOCOL: u32 = 1;

/// How long a child may take to load its model and say hello.
const LOAD_TIMEOUT: Duration = Duration::from_secs(180);
/// How long a child may take over one request. A batch is a handful of
/// texts; a child that takes longer hangs.
const REPLY_TIMEOUT: Duration = Duration::from_secs(120);
/// How long a child stays without a request.
pub const IDLE: Duration = Duration::from_secs(10 * 60);
/// Failures in a row after which the graphics card is given up for the
/// processor.
const CUDA_FAILURES: u32 = 2;

/// Whether a text is what a user asks or what is searched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Query,
    Document,
}

/// The child's first line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: u32,
    pub device: Device,
    pub dimensions: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    id: u64,
    kind: Kind,
    texts: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Reply {
    id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vectors: Option<Vec<Vec<f32>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// What the child's embedding gives: one vector per text, or why not.
pub type Embedded = std::result::Result<Vec<Vec<f32>>, String>;

/// The child's loop: say hello, then answer requests until the input closes.
/// `embed` must put no text of its input into an error: the parent logs it.
pub fn serve(
    input: impl BufRead,
    mut output: impl Write,
    hello: Hello,
    embed: &mut dyn FnMut(Kind, &[String]) -> Embedded,
) -> std::io::Result<()> {
    let mut send = |line: String| -> std::io::Result<()> {
        output.write_all(line.as_bytes())?;
        output.write_all(b"\n")?;
        output.flush()
    };
    send(serde_json::to_string(&hello)?)?;
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Request>(&line) {
            Ok(request) => match embed(request.kind, &request.texts) {
                Ok(vectors) => Reply {
                    id: request.id,
                    vectors: Some(vectors),
                    error: None,
                },
                Err(error) => Reply {
                    id: request.id,
                    vectors: None,
                    error: Some(error),
                },
            },
            Err(error) => Reply {
                error: Some(format!("not a request: {error}")),
                ..Reply::default()
            },
        };
        send(serde_json::to_string(&reply)?)?;
    }
    Ok(())
}

/// The child's only line when it cannot load its model; it ends after it.
/// `reason` must hold nothing secret: the parent logs it.
pub fn refuse(mut output: impl Write, reason: &str) -> std::io::Result<()> {
    let line = serde_json::json!({ "error": reason }).to_string();
    output.write_all(line.as_bytes())?;
    output.write_all(b"\n")?;
    output.flush()
}

/// Which child is wanted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    /// Catalog id of the model.
    pub model: String,
    /// The folder that holds the installed models.
    pub models_dir: PathBuf,
    pub device: Device,
}

/// The two ends of a running child, and the way to end it.
pub struct Channel {
    pub input: Box<dyn Write + Send>,
    pub output: Box<dyn Read + Send>,
    /// Ends the child now, whatever it is doing. Called once.
    pub kill: Box<dyn FnOnce() + Send>,
}

/// Starts a child. The real one starts a program; a test starts a thread.
pub trait Launcher: Send + Sync {
    fn launch(&self, spec: &Spec) -> Result<Channel>;
}

struct Child {
    spec: Spec,
    hello: Hello,
    input: Box<dyn Write + Send>,
    /// One line of the child's output each, or the reason it ended.
    lines: Receiver<std::result::Result<String, String>>,
    kill: Option<Box<dyn FnOnce() + Send>>,
    last_used: Instant,
    next_id: u64,
}

impl Child {
    fn end(&mut self) {
        if let Some(kill) = self.kill.take() {
            kill();
        }
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        self.end();
    }
}

#[derive(Default)]
struct State {
    child: Option<Child>,
    /// Failures of the CUDA child in a row.
    cuda_failures: u32,
}

/// What a page shows about the child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Status {
    /// The model and device of the running child; `None` when none runs.
    pub running: Option<(String, Device)>,
    /// True once the graphics card was given up for the processor.
    pub cuda_given_up: bool,
}

/// The parent's side: one child at a time, started when needed.
pub struct Supervisor {
    launcher: Box<dyn Launcher>,
    /// Whether a game is running: the child then does not linger.
    game: Box<dyn Fn() -> bool + Send + Sync>,
    state: Mutex<State>,
    /// What [`Supervisor::status`] answers. Kept beside the state, not read
    /// from it: the state is locked for as long as a request runs, and a
    /// page that asks every second must not wait for a batch.
    shown: Mutex<Status>,
    idle: Duration,
    load_timeout: Duration,
    reply_timeout: Duration,
}

fn wait_line(
    lines: &Receiver<std::result::Result<String, String>>,
    timeout: Duration,
    doing: &str,
) -> Result<String> {
    match lines.recv_timeout(timeout) {
        Ok(Ok(line)) => Ok(line),
        Ok(Err(reason)) => Err(Error::external(
            format!("the model worker ended while {doing}"),
            reason,
        )),
        Err(RecvTimeoutError::Timeout) => Err(Error::invalid(format!(
            "the model worker did not answer within {} s while {doing}",
            timeout.as_secs()
        ))),
        Err(RecvTimeoutError::Disconnected) => Err(Error::invalid(format!(
            "the model worker ended while {doing}"
        ))),
    }
}

impl Supervisor {
    pub fn new(launcher: Box<dyn Launcher>, game: Box<dyn Fn() -> bool + Send + Sync>) -> Self {
        Self {
            launcher,
            game,
            state: Mutex::new(State::default()),
            shown: Mutex::new(Status {
                running: None,
                cuda_given_up: false,
            }),
            idle: IDLE,
            load_timeout: LOAD_TIMEOUT,
            reply_timeout: REPLY_TIMEOUT,
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Bring what [`Supervisor::status`] answers in line with the state.
    /// Called wherever the child or the count of failures changes.
    fn publish(&self, state: &State) {
        let status = Status {
            running: state
                .child
                .as_ref()
                .map(|child| (child.spec.model.clone(), child.hello.device)),
            cuda_given_up: state.cuda_failures >= CUDA_FAILURES,
        };
        *self
            .shown
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = status;
    }

    /// The device a request for `wanted` really runs on: the processor once
    /// the graphics card failed too often.
    fn device(state: &State, wanted: Device) -> Device {
        if wanted == Device::Cuda && state.cuda_failures >= CUDA_FAILURES {
            Device::Cpu
        } else {
            wanted
        }
    }

    fn start(&self, spec: &Spec) -> Result<Child> {
        let Channel {
            input,
            output,
            kill,
        } = self.launcher.launch(spec)?;
        let (sender, lines) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let ended = line.is_err();
                if sender
                    .send(line.map_err(|error| error.to_string()))
                    .is_err()
                    || ended
                {
                    return;
                }
            }
            let _ = sender.send(Err("its output closed".to_string()));
        });
        let mut child = Child {
            spec: spec.clone(),
            hello: Hello {
                protocol: 0,
                device: spec.device,
                dimensions: 0,
            },
            input,
            lines,
            kill: Some(kill),
            last_used: Instant::now(),
            next_id: 1,
        };
        // On any failure below the child is dropped, which ends it.
        let line = wait_line(&child.lines, self.load_timeout, "loading the model")?;
        let first: serde_json::Value = serde_json::from_str(&line)
            .map_err(|error| Error::external("reading the model worker's first line", error))?;
        if let Some(reason) = first["error"].as_str() {
            return Err(Error::invalid(format!(
                "the model worker could not load the model: {reason}"
            )));
        }
        let hello: Hello = serde_json::from_value(first)
            .map_err(|error| Error::external("reading the model worker's first line", error))?;
        if hello.protocol != PROTOCOL {
            return Err(Error::invalid(format!(
                "the model worker speaks protocol {}, this program {PROTOCOL}",
                hello.protocol
            )));
        }
        child.hello = hello;
        log::info!(
            "model worker started: {} on {:?}, {} dimensions",
            spec.model,
            hello.device,
            hello.dimensions
        );
        Ok(child)
    }

    fn ask(&self, child: &mut Child, kind: Kind, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let id = child.next_id;
        child.next_id += 1;
        let request = serde_json::to_string(&Request {
            id,
            kind,
            texts: texts.to_vec(),
        })
        .map_err(|error| Error::external("writing a request to the model worker", error))?;
        child
            .input
            .write_all(request.as_bytes())
            .and_then(|()| child.input.write_all(b"\n"))
            .and_then(|()| child.input.flush())
            .map_err(|error| Error::external("the model worker took no request", error))?;
        let line = wait_line(&child.lines, self.reply_timeout, "embedding")?;
        let reply: Reply = serde_json::from_str(&line)
            .map_err(|error| Error::external("reading the model worker's answer", error))?;
        if reply.id != id {
            return Err(Error::invalid(
                "the model worker answered another request than the one sent",
            ));
        }
        match (reply.vectors, reply.error) {
            (Some(vectors), None) if vectors.len() == texts.len() => Ok(vectors),
            (Some(vectors), None) => Err(Error::invalid(format!(
                "the model worker returned {} vectors for {} texts",
                vectors.len(),
                texts.len()
            ))),
            (_, error) => Err(Error::invalid(format!(
                "the model could not embed: {}",
                error.unwrap_or_else(|| "no reason given".to_string())
            ))),
        }
    }

    /// Embed `texts` with the child `spec` names, starting it when it is not
    /// running. One request at a time: a caller waits for the one before it,
    /// so a search slips in between two batches of a long run.
    pub fn embed(&self, spec: &Spec, kind: Kind, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let mut state = self.lock();
        let mut spec = spec.clone();
        spec.device = Self::device(&state, spec.device);
        if state.child.as_ref().is_some_and(|child| child.spec != spec) {
            // Another model or device: one child at a time.
            state.child = None;
        }
        let result = (|| {
            if state.child.is_none() {
                state.child = Some(self.start(&spec)?);
                self.publish(&state);
            }
            let child = state.child.as_mut().expect("just started");
            let vectors = self.ask(child, kind, texts)?;
            child.last_used = Instant::now();
            Ok(vectors)
        })();
        match &result {
            Ok(_) => {
                if spec.device == Device::Cuda {
                    state.cuda_failures = 0;
                }
                // A game is running: the answer is given, the memory goes
                // back at once.
                if (self.game)() {
                    state.child = None;
                }
            }
            Err(error) => {
                // Whatever went wrong, this child is not trusted again.
                state.child = None;
                if spec.device == Device::Cuda {
                    state.cuda_failures += 1;
                    if state.cuda_failures == CUDA_FAILURES {
                        log::warn!(
                            "the graphics card failed {CUDA_FAILURES} times ({error}); embedding on the processor from now on"
                        );
                    }
                }
            }
        }
        self.publish(&state);
        result
    }

    /// Stop the child if it has been idle for long, or at once while a game
    /// runs. Never waits for a request under way: it is not idle then.
    /// Returns whether a child still runs, so the caller knows when to stop
    /// asking.
    pub fn tick(&self) -> bool {
        let Ok(mut state) = self.state.try_lock() else {
            return true;
        };
        let Some(child) = state.child.as_ref() else {
            return false;
        };
        if child.last_used.elapsed() >= self.idle || (self.game)() {
            log::info!("model worker stopped: {}", child.spec.model);
            state.child = None;
            self.publish(&state);
            return false;
        }
        true
    }

    /// Stop the child now. A request under way is let finish first.
    pub fn stop(&self) {
        let mut state = self.lock();
        state.child = None;
        self.publish(&state);
    }

    /// Trust the graphics card again, as after a change of the settings.
    pub fn forgive_cuda(&self) {
        let mut state = self.lock();
        state.cuda_failures = 0;
        self.publish(&state);
    }

    /// Which child runs, and whether the graphics card was given up. Never
    /// waits for a request under way.
    pub fn status(&self) -> Status {
        self.shown
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// The program that runs models on the graphics card: the same worker,
/// built with CUDA, in the folder `cuda` beside this program with the NVIDIA
/// libraries it needs. `None` when this installation has none.
pub fn cuda_program() -> Option<PathBuf> {
    let beside = std::env::current_exe().ok()?.parent()?.join("cuda");
    let program = beside.join(if cfg!(windows) {
        "codewiki-embed-cuda.exe"
    } else {
        "codewiki-embed-cuda"
    });
    program.is_file().then_some(program)
}

/// Whether a model can run on a graphics card here: this installation has
/// the worker for it, and the machine has an NVIDIA card. Every installed
/// package has the worker; most machines have no such card.
pub fn cuda_usable() -> bool {
    cuda_program().is_some() && crate::hardware::nvidia_card().is_some()
}

/// The program that runs models on the processor: `codewiki`, which is this
/// program when the backend or the command line asks, and otherwise the one
/// beside this program. A program that is neither (a test, above all) has
/// none, and must never start a copy of itself as if it were a worker.
fn cpu_program() -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "codewiki.exe"
    } else {
        "codewiki"
    };
    let current =
        std::env::current_exe().map_err(|error| Error::external("finding this program", error))?;
    if current.file_name().is_some_and(|file| file == name) {
        return Ok(current);
    }
    let beside = current.with_file_name(name);
    if beside.is_file() {
        Ok(beside)
    } else {
        Err(Error::invalid(format!(
            "found no {name} beside this program to run the model with"
        )))
    }
}

/// Starts the worker as a hidden program: `codewiki` for the processor,
/// [`cuda_program`] for the graphics card.
pub struct ProgramLauncher;

impl Launcher for ProgramLauncher {
    fn launch(&self, spec: &Spec) -> Result<Channel> {
        use std::process::{Command, Stdio};

        let program = match spec.device {
            Device::Cpu => cpu_program()?,
            Device::Cuda => cuda_program().ok_or_else(|| {
                Error::invalid("this installation has no worker for the graphics card")
            })?,
        };
        let mut command = Command::new(&program);
        command
            .arg("embed-worker")
            .arg("--model")
            .arg(&spec.model)
            .arg("--models-dir")
            .arg(&spec.models_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Not connected: a pipe nobody reads would stall the worker. It
            // writes its own log file.
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Without this Windows opens a terminal window for the worker.
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            // A model takes every core it finds. Below the priority of what
            // the user works with, it takes them only when they are free.
            const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
            command.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
        }
        let mut child = command
            .spawn()
            .map_err(|error| Error::external(format!("starting {}", program.display()), error))?;
        let (Some(input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            return Err(Error::invalid("the model worker has no pipes"));
        };
        Ok(Channel {
            input: Box::new(input),
            output: Box::new(output),
            kill: Box::new(move || {
                let _ = child.kill();
                let _ = child.wait();
            }),
        })
    }
}

/// How often the idle child and the game are looked at.
const WATCH: Duration = Duration::from_secs(5);

/// The supervisor of this process. Every built-in embedder asks it, so the
/// process holds one child however many embedders were made.
pub fn global() -> &'static Supervisor {
    static GLOBAL: std::sync::OnceLock<Supervisor> = std::sync::OnceLock::new();
    static WATCHED: std::sync::Once = std::sync::Once::new();
    let supervisor = GLOBAL
        .get_or_init(|| Supervisor::new(Box::new(ProgramLauncher), Box::new(crate::game::busy)));
    WATCHED.call_once(|| {
        // Looks every few seconds; with no child running a look costs
        // nothing, and it asks about a game only while one runs.
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(WATCH);
                global().tick();
            }
        });
    });
    supervisor
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::*;

    /// How the stand-in child behaves.
    #[derive(Clone, Copy, PartialEq)]
    enum Behavior {
        /// Answers every request: the vector of a text is its length and
        /// whether it was a query.
        Works,
        /// Never says hello.
        NeverLoads,
        /// Says hello and then never answers.
        Hangs,
        /// Says hello and ends at the first request.
        Dies,
        /// Says hello with another protocol version.
        OtherProtocol,
        /// Says that it cannot load its model, and ends.
        CannotLoad,
        /// Answers with an error of its own.
        Refuses,
        /// Works on the processor, dies on the graphics card.
        DiesOnCuda,
    }

    /// Starts a thread that plays the child over two pipes.
    struct Fake {
        behavior: Behavior,
        launched: Arc<AtomicUsize>,
        killed: Arc<AtomicUsize>,
        specs: Arc<Mutex<Vec<Spec>>>,
    }

    impl Launcher for Fake {
        fn launch(&self, spec: &Spec) -> Result<Channel> {
            self.launched.fetch_add(1, Ordering::SeqCst);
            self.specs.lock().unwrap().push(spec.clone());
            let (request_reader, request_writer) = std::io::pipe().unwrap();
            let (reply_reader, reply_writer) = std::io::pipe().unwrap();
            let behavior = match self.behavior {
                Behavior::DiesOnCuda if spec.device == Device::Cuda => Behavior::Dies,
                Behavior::DiesOnCuda => Behavior::Works,
                other => other,
            };
            let device = spec.device;
            std::thread::spawn(move || {
                let hello = Hello {
                    protocol: if behavior == Behavior::OtherProtocol {
                        PROTOCOL + 1
                    } else {
                        PROTOCOL
                    },
                    device,
                    dimensions: 2,
                };
                match behavior {
                    Behavior::NeverLoads => {
                        // Holds its pipes open and says nothing.
                        let _keep = (request_reader, reply_writer);
                        std::thread::sleep(Duration::from_secs(5));
                    }
                    Behavior::CannotLoad => {
                        let _ = refuse(reply_writer, "model.safetensors is damaged");
                    }
                    Behavior::Hangs | Behavior::Dies => {
                        let mut output = reply_writer;
                        let _ = writeln!(output, "{}", serde_json::to_string(&hello).unwrap());
                        let mut first = String::new();
                        let _ = BufReader::new(request_reader).read_line(&mut first);
                        if behavior == Behavior::Hangs {
                            let _keep = output;
                            std::thread::sleep(Duration::from_secs(5));
                        }
                    }
                    _ => {
                        let _ = serve(
                            BufReader::new(request_reader),
                            reply_writer,
                            hello,
                            &mut |kind, texts| {
                                if behavior == Behavior::Refuses {
                                    return Err("out of memory".to_string());
                                }
                                Ok(texts
                                    .iter()
                                    .map(|text| {
                                        vec![text.len() as f32, f32::from(kind == Kind::Query)]
                                    })
                                    .collect())
                            },
                        );
                    }
                }
            });
            let killed = self.killed.clone();
            Ok(Channel {
                input: Box::new(request_writer),
                output: Box::new(reply_reader),
                kill: Box::new(move || {
                    killed.fetch_add(1, Ordering::SeqCst);
                }),
            })
        }
    }

    struct Rig {
        supervisor: Supervisor,
        launched: Arc<AtomicUsize>,
        killed: Arc<AtomicUsize>,
        specs: Arc<Mutex<Vec<Spec>>>,
        game: Arc<AtomicBool>,
    }

    fn rig(behavior: Behavior) -> Rig {
        let launched = Arc::new(AtomicUsize::new(0));
        let killed = Arc::new(AtomicUsize::new(0));
        let specs = Arc::new(Mutex::new(Vec::new()));
        let game = Arc::new(AtomicBool::new(false));
        let playing = game.clone();
        let mut supervisor = Supervisor::new(
            Box::new(Fake {
                behavior,
                launched: launched.clone(),
                killed: killed.clone(),
                specs: specs.clone(),
            }),
            Box::new(move || playing.load(Ordering::SeqCst)),
        );
        supervisor.load_timeout = Duration::from_millis(300);
        supervisor.reply_timeout = Duration::from_millis(300);
        Rig {
            supervisor,
            launched,
            killed,
            specs,
            game,
        }
    }

    fn spec(model: &str, device: Device) -> Spec {
        Spec {
            model: model.to_string(),
            models_dir: PathBuf::from("models"),
            device,
        }
    }

    fn texts(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn the_child_starts_on_the_first_request_and_is_kept() {
        let rig = rig(Behavior::Works);
        let cpu = spec("small", Device::Cpu);
        assert_eq!(rig.supervisor.status().running, None);
        assert_eq!(rig.launched.load(Ordering::SeqCst), 0);

        let vectors = rig
            .supervisor
            .embed(&cpu, Kind::Document, &texts(&["ab", "abcd"]))
            .unwrap();
        assert_eq!(vectors, [[2.0, 0.0], [4.0, 0.0]]);
        let vectors = rig
            .supervisor
            .embed(&cpu, Kind::Query, &texts(&["abc"]))
            .unwrap();
        assert_eq!(vectors, [[3.0, 1.0]]);
        // One child served both.
        assert_eq!(rig.launched.load(Ordering::SeqCst), 1);
        assert_eq!(
            rig.supervisor.status().running,
            Some(("small".to_string(), Device::Cpu))
        );
        // Nothing to embed starts nothing and asks nothing.
        assert!(
            rig.supervisor
                .embed(&cpu, Kind::Document, &[])
                .unwrap()
                .is_empty()
        );

        rig.supervisor.stop();
        assert_eq!(rig.killed.load(Ordering::SeqCst), 1);
        assert_eq!(rig.supervisor.status().running, None);
    }

    #[test]
    fn another_model_replaces_the_running_child() {
        let rig = rig(Behavior::Works);
        let one = texts(&["a"]);
        rig.supervisor
            .embed(&spec("small", Device::Cpu), Kind::Document, &one)
            .unwrap();
        rig.supervisor
            .embed(&spec("large", Device::Cpu), Kind::Document, &one)
            .unwrap();
        assert_eq!(rig.launched.load(Ordering::SeqCst), 2);
        assert_eq!(rig.killed.load(Ordering::SeqCst), 1);
        assert_eq!(
            rig.supervisor.status().running,
            Some(("large".to_string(), Device::Cpu))
        );
    }

    #[test]
    fn an_idle_child_is_stopped_and_the_next_request_starts_another() {
        let mut rig = rig(Behavior::Works);
        rig.supervisor.idle = Duration::from_millis(50);
        let cpu = spec("small", Device::Cpu);
        let one = texts(&["a"]);
        // Nothing runs: nothing to watch.
        assert!(!rig.supervisor.tick());
        rig.supervisor.embed(&cpu, Kind::Document, &one).unwrap();
        // Used just now.
        assert!(rig.supervisor.tick());
        assert_eq!(rig.killed.load(Ordering::SeqCst), 0);
        std::thread::sleep(Duration::from_millis(80));
        assert!(!rig.supervisor.tick());
        assert_eq!(rig.killed.load(Ordering::SeqCst), 1);
        assert_eq!(rig.supervisor.status().running, None);

        rig.supervisor.embed(&cpu, Kind::Document, &one).unwrap();
        assert_eq!(rig.launched.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn while_a_game_runs_the_child_answers_and_does_not_linger() {
        let rig = rig(Behavior::Works);
        let cpu = spec("small", Device::Cpu);
        let one = texts(&["a"]);
        rig.supervisor.embed(&cpu, Kind::Document, &one).unwrap();
        // A game starts while the child is idle: the next look stops it.
        rig.game.store(true, Ordering::SeqCst);
        assert!(!rig.supervisor.tick());
        assert_eq!(rig.killed.load(Ordering::SeqCst), 1);

        // A request is still answered, and the child goes right after.
        let vectors = rig.supervisor.embed(&cpu, Kind::Document, &one).unwrap();
        assert_eq!(vectors, [[1.0, 0.0]]);
        assert_eq!(rig.supervisor.status().running, None);
        assert_eq!(rig.killed.load(Ordering::SeqCst), 2);

        // The game ends: the child stays again.
        rig.game.store(false, Ordering::SeqCst);
        rig.supervisor.embed(&cpu, Kind::Document, &one).unwrap();
        assert!(rig.supervisor.status().running.is_some());
    }

    #[test]
    fn a_child_that_never_loads_hangs_or_dies_is_given_up() {
        for (behavior, expected) in [
            (
                Behavior::NeverLoads,
                "did not answer within 0 s while loading the model",
            ),
            (Behavior::Hangs, "did not answer within 0 s while embedding"),
            (Behavior::Dies, "ended while embedding"),
            (Behavior::OtherProtocol, "speaks protocol 2, this program 1"),
            (
                Behavior::CannotLoad,
                "could not load the model: model.safetensors is damaged",
            ),
            (
                Behavior::Refuses,
                "the model could not embed: out of memory",
            ),
        ] {
            let rig = rig(behavior);
            let error = rig
                .supervisor
                .embed(&spec("small", Device::Cpu), Kind::Document, &texts(&["a"]))
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{error}");
            // The child was ended, and nothing is left running.
            assert_eq!(rig.killed.load(Ordering::SeqCst), 1, "{expected}");
            assert_eq!(rig.supervisor.status().running, None);
            // The next request tries a new child.
            let _ =
                rig.supervisor
                    .embed(&spec("small", Device::Cpu), Kind::Document, &texts(&["a"]));
            assert_eq!(rig.launched.load(Ordering::SeqCst), 2, "{expected}");
        }
    }

    #[test]
    fn the_graphics_card_is_given_up_after_two_failures_in_a_row() {
        let rig = rig(Behavior::DiesOnCuda);
        let cuda = spec("large", Device::Cuda);
        let one = texts(&["a"]);
        assert!(rig.supervisor.embed(&cuda, Kind::Document, &one).is_err());
        assert!(!rig.supervisor.status().cuda_given_up);
        assert!(rig.supervisor.embed(&cuda, Kind::Document, &one).is_err());
        assert!(rig.supervisor.status().cuda_given_up);

        // The same request now runs on the processor, and works.
        let vectors = rig.supervisor.embed(&cuda, Kind::Document, &one).unwrap();
        assert_eq!(vectors, [[1.0, 0.0]]);
        assert_eq!(
            rig.supervisor.status().running,
            Some(("large".to_string(), Device::Cpu))
        );
        let devices: Vec<Device> = rig.specs.lock().unwrap().iter().map(|s| s.device).collect();
        assert_eq!(devices, [Device::Cuda, Device::Cuda, Device::Cpu]);

        // A change of the settings gives the card another chance.
        rig.supervisor.forgive_cuda();
        assert!(!rig.supervisor.status().cuda_given_up);
        assert!(rig.supervisor.embed(&cuda, Kind::Document, &one).is_err());
    }

    #[test]
    fn the_childs_loop_answers_a_line_that_is_no_request() {
        let input = b"this is not json\n\n{\"id\":7,\"kind\":\"query\",\"texts\":[\"abc\"]}\n";
        let mut output = Vec::new();
        serve(
            &input[..],
            &mut output,
            Hello {
                protocol: PROTOCOL,
                device: Device::Cpu,
                dimensions: 1,
            },
            &mut |_, texts| Ok(texts.iter().map(|text| vec![text.len() as f32]).collect()),
        )
        .unwrap();
        let lines: Vec<&str> = std::str::from_utf8(&output).unwrap().lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], r#"{"protocol":1,"device":"cpu","dimensions":1}"#);
        assert!(lines[1].contains("not a request"), "{}", lines[1]);
        assert_eq!(lines[2], r#"{"id":7,"vectors":[[3.0]]}"#);
    }
}
