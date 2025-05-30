use anyhow::Result;
use clap::Parser as CliParser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use nanoid::nanoid;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, info};
use url::Url;

#[derive(CliParser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The URL to use for valid requests
    #[arg(long, default_value = "https://valid.rpki.isbgpsafeyet.com")]
    valid_url: String,

    /// The URL to use for invalid requests
    #[arg(long, default_value = "https://invalid.rpki.isbgpsafeyet.com")]
    invalid_url: String,

    /// Alphabet to use for generating the ID
    #[arg(long, default_value = "1234567890abcdef")]
    alphabet: String,

    /// Requests timeout
    #[arg(long, short, default_value = "3")]
    timeout: usize,

    /// Verbosity level
    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,
}

fn set_tracing(cli: &Cli) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_max_level(cli.verbose)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct IsBgpSafeYet {
    status: String,
    asn: u32,
    name: String,
    blackholed: bool,
}

async fn get_url(client: &Client, url: Url) -> Result<IsBgpSafeYet, Box<dyn std::error::Error>> {
    let response = client.get(url).send().await?;
    let isbgpsafeyet = response.json::<IsBgpSafeYet>().await?;
    Ok(isbgpsafeyet)
}

async fn check_success(client: &Client, url: Url) -> bool {
    match get_url(&client, url).await {
        Ok(response) => {
            debug!("Response: {:?}", response);
            true
        }

        Err(e) => {
            debug!("Error: {}", e);
            false
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    set_tracing(&cli)?;

    let alphabet = cli.alphabet.chars().collect::<Vec<char>>();
    let mut id = String::new();
    if alphabet.len() != 0 {
        id = nanoid!(10, &alphabet);
    }

    let valid_url = Url::parse(&cli.valid_url)?;
    let valid_url = valid_url.join(&id)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(cli.timeout as u64))
        .build()?;

    let valid_success = check_success(&client, valid_url).await;

    let invalid_url = Url::parse(&cli.invalid_url)?;
    let invalid_url = invalid_url.join(&id)?;

    let invalid_success = check_success(&client, invalid_url).await;

    if valid_success && !invalid_success {
        info!("OK");
    } else {
        info!("NOK");
    }

    Ok(())
}
