use anyhow::{anyhow, Result};
use dialoguer::console::Style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use is_terminal::IsTerminal;
use serde::Deserialize;
use std::env;
use std::ffi::OsString;
use std::io;
use std::process::{Command, ExitCode};

const DEFAULT_BASE_URL: &str = "http://localhost:1234";
const DEFAULT_AUTH_TOKEN: &str = "lmstudio";
const NON_INTERACTIVE_MODEL_ERROR: &str =
    "claudio: no --model provided and non-interactive mode detected; please pass --model <modelKey>.";

#[derive(Deserialize)]
struct LmsModel {
    #[serde(rename = "modelKey")]
    model_key: Option<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    ensure_env_defaults();

    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let scan = scan_args(&args);

    if scan.starts_session && !scan.model_specified {
        if scan.print_mode || !stdin_stdout_are_tty() {
            eprintln!("{NON_INTERACTIVE_MODEL_ERROR}");
            return Ok(ExitCode::from(2));
        }

        let models = list_models()?;
        let selected = pick_model(&models).ok_or_else(|| anyhow!("claudio: no model selected"))?;
        return exec_claude(&args, Some(selected));
    }

    exec_claude(&args, None)
}

fn ensure_env_defaults() {
    if env::var_os("ANTHROPIC_BASE_URL").map_or(true, |value| value.is_empty()) {
        env::set_var("ANTHROPIC_BASE_URL", DEFAULT_BASE_URL);
    }
    if env::var_os("ANTHROPIC_AUTH_TOKEN").map_or(true, |value| value.is_empty()) {
        env::set_var("ANTHROPIC_AUTH_TOKEN", DEFAULT_AUTH_TOKEN);
    }
}

struct ScanResult {
    model_specified: bool,
    print_mode: bool,
    starts_session: bool,
}

fn scan_args(args: &[OsString]) -> ScanResult {
    let mut model_specified = false;
    let mut print_mode = false;
    let mut help_mode = false;
    let mut positional: Option<OsString> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(arg_str) = arg.to_str() {
            match arg_str {
                "--model" => {
                    model_specified = true;
                    i += 1;
                }
                _ if arg_str.starts_with("--model=") => {
                    model_specified = true;
                }
                "-p" | "--print" => {
                    print_mode = true;
                }
                "-h" | "--help" | "-v" | "--version" => {
                    help_mode = true;
                }
                "--" => {
                    if positional.is_none() && i + 1 < args.len() {
                        positional = Some(args[i + 1].clone());
                    }
                }
                _ if arg_str.starts_with('-') => {}
                _ => {
                    if positional.is_none() {
                        positional = Some(arg.clone());
                    }
                }
            }
        } else if positional.is_none() {
            positional = Some(arg.clone());
        }
        i += 1;
    }

    let mut starts_session = !help_mode;
    if let Some(pos) = positional.as_ref().and_then(|p| p.to_str()) {
        if matches!(
            pos,
            "doctor" | "install" | "mcp" | "plugin" | "setup-token" | "update" | "help"
        ) {
            starts_session = false;
        }
    }

    ScanResult {
        model_specified,
        print_mode,
        starts_session,
    }
}

fn stdin_stdout_are_tty() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

fn list_models() -> Result<Vec<String>> {
    let output = Command::new("lms")
        .args(["ls", "--llm", "--json"])
        .output()
        .map_err(|_| anyhow!("claudio: missing dependency: lms"))?;

    if !output.status.success() {
        return Err(anyhow!("claudio: no models found (try: lms ls --llm)"));
    }

    let models: Vec<LmsModel> = serde_json::from_slice(&output.stdout)
        .map_err(|_| anyhow!("claudio: no models found (try: lms ls --llm)"))?;

    let mut keys = Vec::new();
    for model in models {
        if let Some(key) = model.model_key {
            if !key.trim().is_empty() {
                keys.push(key);
            }
        }
    }

    if keys.is_empty() {
        return Err(anyhow!("claudio: no models found (try: lms ls --llm)"));
    }

    Ok(keys)
}

fn pick_model(models: &[String]) -> Option<String> {
    let mut theme = ColorfulTheme::default();
    theme.active_item_style = Style::new().green().bold();
    let selection = Select::with_theme(&theme)
        .with_prompt("Select a model for Claude Code")
        .items(models)
        .default(0)
        .interact_opt()
        .ok()?;

    selection.map(|index| models[index].clone())
}

fn exec_claude(args: &[OsString], selected_model: Option<String>) -> Result<ExitCode> {
    let mut command = Command::new("claude");
    command.args(args);

    if let Some(model) = selected_model {
        command.arg("--model").arg(model);
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = command.exec();
        Err(exec_error(err))
    }

    #[cfg(not(unix))]
    {
        let status = command.status().map_err(exec_error)?;
        Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
    }
}

fn exec_error(err: io::Error) -> anyhow::Error {
    if err.kind() == io::ErrorKind::NotFound {
        anyhow!("claudio: missing dependency: claude")
    } else {
        anyhow!("claudio: failed to exec claude")
    }
}
