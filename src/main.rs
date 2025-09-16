use chrono::prelude::*;
use clap::{Parser, Subcommand};
use jsonwatch::diff;
use std::{error::Error, process::Command, str, thread, time};

#[derive(Parser, Debug)]
#[command(
    name = "jsonwatch",
    about = "Track changes in JSON data",
    version = "0.9.0"
)]
struct Cli {
    /// Don't print date and time for each diff
    #[arg(short = 'D', long)]
    no_date: bool,

    /// Don't print initial JSON values
    #[arg(short = 'I', long)]
    no_initial_values: bool,

    /// Print raw data to standard error with a timestamp
    #[arg(short = 'd', long = "debug")]
    debug: bool,

    /// Polling interval in seconds
    #[arg(short = 'n', long, value_name = "seconds", default_value = "1")]
    interval: u32,

    /// Exit after a number of changes
    #[arg(short = 'm', long = "max-changes", value_name = "count")]
    max_changes: Option<u32>,

    /// Subcommands for different data sources
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Execute a command and track changes in the JSON output
    #[command(aliases(["c", "command"]))]
    Cmd {
        /// Command to execute
        #[arg(value_name = "command")]
        command: String,

        /// Arguments to the command
        #[arg(
            value_name = "arg",
            trailing_var_arg = true,
            allow_hyphen_values = true
        )]
        args: Vec<String>,
    },

    /// Fetch a URL and track changes in the JSON data
    #[command(aliases(["u"]))]
    Url {
        /// URL to fetch
        #[arg(value_name = "url")]
        url: String,

        /// Custom User-Agent string
        #[arg(
            short = 'A',
            long = "user-agent",
            value_name = "user-agent",
            default_value = "curl/7.58.0"
        )]
        user_agent: String,

        /// Custom headers in the format "X-Foo: bar"
        #[arg(
            short = 'H',
            long = "header",
            value_name = "header",
            action = clap::ArgAction::Append
        )]
        headers: Vec<String>,
    },
}

fn run_command(
    command: &String,
    args: &[String],
) -> Result<String, Box<dyn Error>> {
    if command.is_empty() {
        return Ok(String::new());
    }

    let output = Command::new(command).args(args).output()?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn fetch_url(
    url: &str,
    user_agent: &str,
    headers: &[String],
) -> Result<String, Box<dyn Error>> {
    let mut req = ureq::get(url).set("User-Agent", user_agent);

    for header in headers {
        if let Some((name, value)) = header.split_once(':') {
            req = req.set(name.trim(), value.trim());
        }
    }

    let result = req.call()?;
    Ok(result.into_string()?)
}

fn print_debug(raw_data: &str) {
    let local = Local::now();
    let timestamp = local.format("%Y-%m-%dT%H:%M:%S%z");
    eprint!("[DEBUG {}]\n{}", timestamp, raw_data);

    if !raw_data.is_empty() && !raw_data.ends_with("\n") {
        eprintln!()
    }
}

fn watch(
    interval: time::Duration,
    max_changes: Option<u32>,
    print_date: bool,
    print_initial: bool,
    debug: bool,
    lambda: impl Fn() -> Result<String, Box<dyn Error>>,
) {
    let mut change_count = 0;
    let raw_data = match lambda() {
        Ok(s) => s,
        Err(e) => {
            if debug {
                let local = Local::now();
                let timestamp = local.format("%Y-%m-%dT%H:%M:%S%z");
                eprintln!("[DEBUG {}] Error: {}", timestamp, e);
            }
            String::new()
        }
    };
    let mut data: Option<serde_json::Value> =
        serde_json::from_str(&raw_data).ok();

    if print_initial {
        if debug {
            print_debug(&raw_data);
        }

        if let Some(json) = &data {
            println!("{}", serde_json::to_string_pretty(&json).unwrap())
        }
    }

    loop {
        if let Some(max) = max_changes {
            if change_count >= max {
                break;
            }
        }

        thread::sleep(interval);

        let raw_data = match lambda() {
            Ok(s) => s,
            Err(e) => {
                if debug {
                    let local = Local::now();
                    let timestamp = local.format("%Y-%m-%dT%H:%M:%S%z");
                    eprintln!("[DEBUG {}] Error: {}", timestamp, e);
                }
                continue;
            }
        };
        if debug {
            print_debug(&raw_data);
        }

        let prev = data.clone();
        data = serde_json::from_str(&raw_data).ok();

        let diff = diff::diff(&prev, &data);

        let changed = diff.len();
        if changed == 0 {
            continue;
        }

        change_count += 1;

        if print_date {
            let local = Local::now();
            print!("{}", local.format("%Y-%m-%dT%H:%M:%S%z"));

            if changed == 1 {
                print!(" ");
            } else {
                println!();
            }
        }

        if changed == 1 {
            print!("{}", diff);
        } else {
            let s = format!("{}", diff)
                .lines()
                .collect::<Vec<_>>()
                .join("\n    ");
            println!("    {}", s);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let lambda: Box<dyn Fn() -> Result<String, Box<dyn Error>>> =
        match &cli.command {
            Commands::Cmd { args, command } => {
                let args = args.clone();
                let command = command.clone();
                Box::new(move || run_command(&command, &args))
            }

            Commands::Url {
                url,
                user_agent,
                headers,
            } => {
                let url = url.clone();
                let user_agent = user_agent.clone();
                let headers = headers.clone();
                Box::new(move || fetch_url(&url, &user_agent, &headers))
            }
        };

    watch(
        time::Duration::from_secs(cli.interval as u64),
        cli.max_changes,
        !cli.no_date,
        !cli.no_initial_values,
        cli.debug,
        lambda,
    );
}
