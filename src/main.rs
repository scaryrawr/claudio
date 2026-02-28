use anyhow::{anyhow, Result};
use clap::builder::Command as ClapCommand;
use clap::builder::ValueParser;
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
const OLLAMA_BASE_URL: &str = "http://localhost:11434";
const OLLAMA_AUTH_TOKEN: &str = "ollama";
const NON_INTERACTIVE_MODEL_ERROR: &str =
    "claudio: no --model provided and non-interactive mode detected; please pass --model <modelKey>.";

#[derive(Deserialize)]
struct LmStudioModelsResponse {
    data: Vec<LmsModel>,
}

#[derive(Deserialize)]
struct LmsModel {
    id: String,
    #[serde(rename = "type")]
    model_type: String,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: Option<String>,
    model: Option<String>,
}

#[derive(Clone, Copy)]
enum Provider {
    LmStudio,
    Ollama,
}

impl Provider {
    fn name(self) -> &'static str {
        match self {
            Provider::LmStudio => "LM Studio",
            Provider::Ollama => "Ollama",
        }
    }

    fn base_url(self) -> &'static str {
        match self {
            Provider::LmStudio => DEFAULT_BASE_URL,
            Provider::Ollama => OLLAMA_BASE_URL,
        }
    }

    fn default_auth_token(self) -> &'static str {
        match self {
            Provider::LmStudio => DEFAULT_AUTH_TOKEN,
            Provider::Ollama => OLLAMA_AUTH_TOKEN,
        }
    }
}

#[derive(Clone)]
struct DiscoveredModel {
    id: String,
    provider: Provider,
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
    let scan = scan_args(&args)?;

    if scan.starts_session && !scan.model_specified {
        if scan.print_mode || !stdin_stdout_are_tty() {
            eprintln!("{NON_INTERACTIVE_MODEL_ERROR}");
            return Ok(ExitCode::from(2));
        }

        let models = list_models()?;
        let selected = pick_model(&models).ok_or_else(|| anyhow!("claudio: no model selected"))?;
        apply_selected_provider(selected.provider);
        return exec_claude(&args, Some(selected.id));
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

fn scan_args(args: &[OsString]) -> Result<ScanResult> {
    let command = ClapCommand::new("claudio")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .allow_hyphen_values(true)
        .ignore_errors(true)
        .arg(
            clap::Arg::new("model")
                .long("model")
                .value_name("MODEL")
                .allow_hyphen_values(true)
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            clap::Arg::new("print")
                .short('p')
                .long("print")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("help")
                .short('h')
                .long("help")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("version")
                .short('v')
                .long("version")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("rest")
                .num_args(0..)
                .allow_hyphen_values(true)
                .trailing_var_arg(true)
                .value_parser(ValueParser::os_string()),
        );

    let mut scan_args: Vec<OsString> = Vec::with_capacity(args.len() + 1);
    scan_args.push(OsString::from("claudio"));
    let mut expect_model_value = false;
    for arg in args {
        if expect_model_value {
            scan_args.push(arg.clone());
            expect_model_value = false;
            continue;
        }
        if let Some(arg_str) = arg.to_str() {
            if arg_str == "--model" {
                expect_model_value = true;
                scan_args.push(arg.clone());
                continue;
            }
            if arg_str == "--" {
                continue;
            }
        }
        scan_args.push(arg.clone());
    }

    let matches = command.get_matches_from(scan_args);
    let raw_model_specified = args.iter().any(|arg| {
        arg.to_str()
            .map(|arg_str| arg_str == "--model" || arg_str.starts_with("--model="))
            .unwrap_or(false)
    });
    let model_specified = matches.contains_id("model") || raw_model_specified;
    let print_mode = matches.get_flag("print");
    let help_mode = matches.get_flag("help") || matches.get_flag("version");
    let positional = matches
        .get_many::<OsString>("rest")
        .and_then(|mut values| values.next())
        .cloned();

    let mut starts_session = !help_mode;
    if let Some(pos) = positional.as_ref().and_then(|p| p.to_str()) {
        if matches!(
            pos,
            "doctor" | "install" | "mcp" | "plugin" | "setup-token" | "update" | "help"
        ) {
            starts_session = false;
        }
    }

    Ok(ScanResult {
        model_specified,
        print_mode,
        starts_session,
    })
}

fn stdin_stdout_are_tty() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

fn list_lmstudio_models() -> Result<Vec<String>> {
    let url = format!("{}/api/v0/models", DEFAULT_BASE_URL);
    let response: LmStudioModelsResponse = ureq::get(&url)
        .header(
            "Authorization",
            format!("Bearer {}", DEFAULT_AUTH_TOKEN).as_str(),
        )
        .call()
        .map_err(|_| anyhow!("could not reach LM Studio at {}", DEFAULT_BASE_URL))?
        .body_mut()
        .read_json()
        .map_err(|_| anyhow!("failed to parse LM Studio models response from {}", url))?;

    let models: Vec<String> = response
        .data
        .into_iter()
        .filter(|m| m.model_type == "llm" || m.model_type == "vlm")
        .map(|m| m.id)
        .filter(|id| !id.trim().is_empty())
        .collect();

    if models.is_empty() {
        return Err(anyhow!("no LLM models found at {}", DEFAULT_BASE_URL));
    }

    Ok(models)
}

fn list_ollama_models() -> Result<Vec<String>> {
    let url = format!("{}/api/tags", OLLAMA_BASE_URL);
    let response: OllamaTagsResponse = ureq::get(&url)
        .call()
        .map_err(|_| anyhow!("could not reach Ollama at {}", OLLAMA_BASE_URL))?
        .body_mut()
        .read_json()
        .map_err(|_| anyhow!("failed to parse Ollama tags response from {}", url))?;

    let models: Vec<String> = response
        .models
        .into_iter()
        .filter_map(|model| model.name.or(model.model))
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();

    if models.is_empty() {
        return Err(anyhow!("no models found at {}", OLLAMA_BASE_URL));
    }

    Ok(models)
}

fn list_models() -> Result<Vec<DiscoveredModel>> {
    let mut discovered: Vec<DiscoveredModel> = Vec::new();
    let mut issues: Vec<String> = Vec::new();

    match list_lmstudio_models() {
        Ok(models) => discovered.extend(models.into_iter().map(|id| DiscoveredModel {
            id,
            provider: Provider::LmStudio,
        })),
        Err(err) => issues.push(format!("LM Studio: {}", err)),
    }

    match list_ollama_models() {
        Ok(models) => discovered.extend(models.into_iter().map(|id| DiscoveredModel {
            id,
            provider: Provider::Ollama,
        })),
        Err(err) => issues.push(format!("Ollama: {}", err)),
    }

    discovered.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then_with(|| a.provider.name().cmp(b.provider.name()))
    });

    if discovered.is_empty() {
        return Err(anyhow!(
            "claudio: no models discovered from LM Studio ({}) or Ollama ({}): {}",
            DEFAULT_BASE_URL,
            OLLAMA_BASE_URL,
            issues.join(" | ")
        ));
    }

    Ok(discovered)
}

fn pick_model(models: &[DiscoveredModel]) -> Option<DiscoveredModel> {
    let items: Vec<String> = models
        .iter()
        .map(|m| format!("{} ({})", m.id, m.provider.name()))
        .collect();
    let mut theme = ColorfulTheme::default();
    theme.active_item_style = Style::new().green().bold();
    let selection = Select::with_theme(&theme)
        .with_prompt("Select a model for Claude Code")
        .items(&items)
        .default(0)
        .interact_opt()
        .ok()?;

    selection.map(|index| models[index].clone())
}

fn apply_selected_provider(provider: Provider) {
    env::set_var("ANTHROPIC_BASE_URL", provider.base_url());

    let set_provider_token = env::var("ANTHROPIC_AUTH_TOKEN")
        .map(|token| {
            let token = token.trim();
            token.is_empty() || token == DEFAULT_AUTH_TOKEN || token == OLLAMA_AUTH_TOKEN
        })
        .unwrap_or(true);

    if set_provider_token {
        env::set_var("ANTHROPIC_AUTH_TOKEN", provider.default_auth_token());
    }
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
