use clap::{Parser, Subcommand};

use openai_api_rs::v1::api::OpenAIClient;

use serde::Deserialize;

mod db;

#[derive(Deserialize, Debug)]
struct EnvVariables {
    openai_key: String,
}

#[derive(Parser, Debug)]
#[command(subcommand_negates_reqs = true)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(required = true)]
    description: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Seed,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let env_variables = envy::from_env::<EnvVariables>().unwrap();

    let openai_client = OpenAIClient::new(env_variables.openai_key);
    let db_client = db::Db::new().await?;

    let args = Args::parse();

    if let Some(Commands::Seed) = args.command {
        println!("-----");
        println!("Seeding start");

        db_client.seed(&openai_client).await?;

        println!("-----");
        println!("Seeding database finish");
    } else if let Some(_) = args.description {
        db_client
            .find_similar_app_and_action(&openai_client, "create_contact")
            .await?;
    }

    return Ok(());
}
