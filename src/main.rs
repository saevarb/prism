mod app;
mod bucket;
mod cli;
mod render;

use anyhow::Result;
use app::AppMessage;
use clap::CommandFactory;
use clap::Parser;
use cli::Config;
use log::{debug, info};
use mpsc::channel;
use nix::{sys::signal, unistd::Pid};
use signal::killpg;

use std::io;
use std::io::Read;
use std::process::Child;
use std::sync::mpsc::SendError;
use std::thread;
use std::{io::BufRead, io::BufReader, sync::mpsc};
use std::{process::Stdio, sync::mpsc::Receiver};
use tui::{backend::CrosstermBackend, Terminal};

use crate::{
  app::App,
  render::{setup_tui, teardown_tui},
};

fn spawn_reader_thread<S: Read + std::marker::Send + 'static>(stream: S) -> Receiver<String> {
  let (tx, rx) = mpsc::channel::<String>();
  thread::spawn(move || {
    let reader = BufReader::new(stream);
    reader
      .lines()
      .filter_map(|line| line.ok())
      .for_each(|line| {
        if let Err(e) = tx.send(line) {
          debug!("Error sending line: {}", e);
        }
      });
  });
  rx
}

fn spawn_monitor_thread(mut child: Child) -> Receiver<AppMessage> {
  let (tx, rx) = mpsc::channel::<AppMessage>();
  thread::spawn(move || -> Result<(), SendError<_>> {
    loop {
      match child.try_wait() {
        Ok(Some(code)) => {
          tx.send(AppMessage::Exit(code))?;
          break Ok(());
        }
        Ok(None) => (),
        Err(_) => (),
      }
      thread::sleep(std::time::Duration::from_millis(16));
    }
  });
  rx
}

fn main() -> Result<()> {
  env_logger::init();
  let config = Config::parse();
  let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout())).unwrap();

  let mut cmd = Config::command();
  if config.command.len() == 0 {
    cmd
      .error(clap::error::ErrorKind::InvalidValue, "No command provided")
      .exit();
  }

  let shell_command = config.command.join(" ");
  debug!("Running command: {}", shell_command);
  debug!("Using regex: {}", config.prefix);
  let args: Vec<String> = vec!["-c".to_string(), shell_command];
  let mut process = std::process::Command::new("bash")
    .args(&args)
    .stderr(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;
  let stdout = process.stdout.take().expect("Failed to open stdout");
  let stderr = process.stderr.take().expect("Failed to open stderr");
  let output = spawn_reader_thread(stdout);
  let errors = spawn_reader_thread(stderr);
  let monitor = spawn_monitor_thread(process);

  setup_tui()?;
  let mut app = App::new(&config);
  let _res = app.run(&mut terminal, output, errors, monitor)?;
  teardown_tui(&mut terminal)?;

  // NOTE: The below is my current attempt at ensuring that all child processes are killed when we exit.
  // This does not seem to be reliable, however, but I can't figure out whether that's because I'm doing something wrong
  // or whether because there is a bug in turborepo.
  // I have previously had issues with dangling turborepo processes, which is why I think it is a possibility.
  let (tx, rx) = channel::<()>();
  ctrlc::set_handler(move || {
    info!("Received SIGINT, sending SIGTERM to process group");
    tx.send(()).expect("Failed to send signal");
  })
  .expect("Error setting Ctrl-C handler");

  // This should send SIGINT to all our children and terminate them
  killpg(Pid::this(), signal::Signal::SIGINT).expect("Failed to kill parent process group");
  rx.recv().expect("Failed to receive signal");
  // process.wait().expect("Failed to wait for process");
  debug!("Process exited");

  Ok(())
}
