use anyhow::Result;
use std::collections::VecDeque;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self { program: program.into(), args: Vec::new(), env: Vec::new(), cwd: None }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }
}

pub trait Shell {
    fn run(&mut self, command: CommandSpec) -> Result<()>;
    fn read(&mut self, command: CommandSpec) -> Result<String>;
}

pub struct RealShell {
    shell: xshell::Shell,
}

impl RealShell {
    pub fn new() -> Result<Self> {
        Ok(Self { shell: xshell::Shell::new()? })
    }
}

impl Shell for RealShell {
    fn run(&mut self, command: CommandSpec) -> Result<()> {
        info!(command = %render_command(&command), "running command");
        let _cwd = command.cwd.as_ref().map(|cwd| self.shell.push_dir(cwd));
        let mut cmd = self.shell.cmd(&command.program);
        for arg in &command.args {
            cmd = cmd.arg(arg);
        }
        for (key, value) in &command.env {
            cmd = cmd.env(key, value);
        }
        cmd.run()?;
        Ok(())
    }

    fn read(&mut self, command: CommandSpec) -> Result<String> {
        info!(command = %render_command(&command), "reading command output");
        let _cwd = command.cwd.as_ref().map(|cwd| self.shell.push_dir(cwd));
        let mut cmd = self.shell.cmd(&command.program);
        for arg in &command.args {
            cmd = cmd.arg(arg);
        }
        for (key, value) in &command.env {
            cmd = cmd.env(key, value);
        }
        Ok(cmd.read()?)
    }
}

#[derive(Debug, Default)]
pub struct MockShell {
    echoes: Vec<String>,
    commands: Vec<CommandSpec>,
    run_results: VecDeque<Result<()>>,
    read_results: VecDeque<Result<String>>,
}

impl MockShell {
    pub fn echoes(&self) -> &[String] {
        &self.echoes
    }

    pub fn commands(&self) -> &[CommandSpec] {
        &self.commands
    }

    pub fn fail_next_run(&mut self, message: impl Into<String>) {
        self.run_results.push_back(Err(anyhow::anyhow!(message.into())));
    }

    pub fn fail_next_read(&mut self, message: impl Into<String>) {
        self.read_results.push_back(Err(anyhow::anyhow!(message.into())));
    }
}

impl Shell for MockShell {
    fn run(&mut self, command: CommandSpec) -> Result<()> {
        self.echoes.push(render_command(&command));
        self.commands.push(command);
        match self.run_results.pop_front() {
            Some(result) => result,
            None => Ok(()),
        }
    }

    fn read(&mut self, command: CommandSpec) -> Result<String> {
        self.echoes.push(render_command(&command));
        self.commands.push(command);
        match self.read_results.pop_front() {
            Some(result) => result,
            None => Ok(String::new()),
        }
    }
}

pub fn render_command(command: &CommandSpec) -> String {
    let mut rendered = String::new();
    if !command.env.is_empty() {
        let env = command
            .env
            .iter()
            .map(|(key, value)| format!("{key}={}", redact_value(key, value)))
            .collect::<Vec<_>>()
            .join(" ");
        rendered.push_str(&env);
        rendered.push(' ');
    }
    rendered.push_str(&shell_quote(&command.program));
    for arg in &command.args {
        rendered.push(' ');
        rendered.push_str(&shell_quote(arg));
    }
    if let Some(cwd) = &command.cwd {
        rendered.push_str(" [cwd=");
        rendered.push_str(&cwd.display().to_string());
        rendered.push(']');
    }
    rendered
}

fn redact_value(key: &str, value: &str) -> String {
    let upper = key.to_ascii_uppercase();
    if upper.contains("TOKEN") || upper.contains("SECRET") || upper.contains("KEY") {
        "[redacted]".to_owned()
    } else {
        value.to_owned()
    }
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_owned();
    }
    if value.chars().all(|ch| ch.is_ascii_alphanumeric() || "-._/".contains(ch)) {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}