use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Ok};
use futures::{future::try_join_all, TryFutureExt};
use openai_api_rs::v1::{
    api::OpenAIClient,
    chat_completion::{self, ChatCompletionRequest},
    common::{GPT4_O_2024_08_06, TEXT_EMBEDDING_3_SMALL},
    embedding::EmbeddingRequest,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tokio::time::sleep;

#[derive(sqlx::FromRow, Debug, Serialize, Deserialize, Clone)]
struct App {
    name: String,
    description: String,
    actions: Vec<Action>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Action {
    name: String,
    description: String,
}

pub struct Db {
    pool: sqlx::PgPool,
}

impl Db {
    pub async fn new() -> anyhow::Result<Self> {
        let pool = sqlx::PgPool::connect("postgres://user:password@localhost:5432/db").await?;
        return anyhow::Ok(Self { pool });
    }

    pub async fn seed(&self, openai_client: &OpenAIClient) -> anyhow::Result<()> {
        let apps = generate_apps(openai_client).await?;

        let handles = apps.iter().map(|app| {
            let app_handle = insert_app(&self.pool, &app).and_then(move |app_id| {
                let mut action_handles = vec![];

                app.actions.iter().for_each(|action| {
                    let handle = insert_action(&self.pool, openai_client, app_id, app, action);
                    action_handles.push(handle);
                });

                return try_join_all(action_handles);
            });

            return app_handle;
        });

        try_join_all(handles).await?;

        return Ok(());
    }

    pub async fn find_similar_app_and_action(
        &self,
        openai_client: &OpenAIClient,
        query: &str,
    ) -> anyhow::Result<()> {
        let result = sqlx::query(
            format!(
                "
                    select actions.id, apps.id as app_id, actions.name, apps.name as app_name, actions.description, (embedding <=> $1::vector) as cos_distance
                    from actions
                    join apps on actions.app_id = app_id
                    order by cos_distance ASC
                    limit 1
                "
            )
            .as_str(),
        ).bind(generate_embedding(&openai_client, query).await?)
        .fetch_one(&self.pool)
        .await?;

        println!("result = {:?}", result);

        return Ok(());
    }
}

async fn insert_app(pool: &PgPool, app: &App) -> anyhow::Result<i32> {
    let mut rng = {
        let rng = rand::thread_rng();
        StdRng::from_rng(rng).unwrap()
    };

    let milis = rng.gen_range(300..1000);

    println!("Inserting app: {} with delay: {}", app.name, milis);

    sleep(Duration::from_millis(milis)).await;

    let app_id = sqlx::query("insert into apps (name, description) values ($1, $2) returning id")
        .bind(&app.name)
        .bind(&app.description)
        .fetch_one(pool)
        .await?
        .get::<i32, _>("id");

    println!("Inserted app: {} with delay: {}", app.name, milis);

    return Ok(app_id);
}

async fn insert_action(
    pool: &PgPool,
    openai_client: &OpenAIClient,
    app_id: i32,
    app: &App,
    action: &Action,
) -> anyhow::Result<()> {
    let mut rng = {
        let rng = rand::thread_rng();
        StdRng::from_rng(rng).unwrap()
    };

    let milis = rng.gen_range(300..1000);

    println!(
        "Inserting action: {}, related to app: {} with delay: {}",
        action.name, app.name, milis
    );

    sleep(Duration::from_millis(milis)).await;

    let embedding_text = format!(
        "App: {app_name}.\nApp description: {app_description}.\n\nAction: ${action_name}.\nAction description: ${action_description}.",
        app_name = app.name,
        app_description = app.description,
        action_name = action.name,
        action_description = action.description
    );

    let embeddings = generate_embedding(openai_client, &embedding_text).await?;

    sqlx::query(
        "insert into actions (app_id, name, description, embedding) values ($1, $2, $3, $4)",
    )
    .bind(app_id)
    .bind(&action.name)
    .bind(&action.description)
    .bind(&embeddings)
    .execute(pool)
    .await?;

    println!(
        "Inserted action: {}, related to app: {} with delay: {}",
        action.name, app.name, milis
    );

    return Ok(());
}

async fn generate_embedding(
    openai_client: &OpenAIClient,
    embedding_text: &str,
) -> anyhow::Result<Vec<f32>> {
    let embeddings = openai_client
        .embedding(EmbeddingRequest::new(
            TEXT_EMBEDDING_3_SMALL.to_string(),
            embedding_text.to_string(),
        ))
        .await?
        .data
        .get(0)
        .map(|data| {
            return data.embedding.clone();
        })
        .unwrap();

    return Ok(embeddings);
}

async fn generate_apps(openai_client: &OpenAIClient) -> anyhow::Result<Vec<App>> {
    let app_generation_prompt = format!(
        r#"You are a helpful assistant that generates synthetic data for a workflow automation platform. Generate 3 unique apps with their names, types (public or private), and descriptions. For each app, also generate at least 3 actions with their names and descriptions. Provide the output in plain JSON format without any formatting. You are not writing markdown, you are writing JSON data.
    Example:
    [
      {{
        "name": "gmail",
        "description": "Send emails with Google Mail.",
        "actions": [
          {{
            "name": "send_email",
            "description": "Send an email to a recipient."
    }}, {{
            "name": "forward_email",
            "description": "Forward an existing email."
    }}, {{
            "name": "apply_label",
            "description": "Apply a label to an email."
    }}
        ]
    }}
    ]"#
    );

    let result = openai_client
        .chat_completion(ChatCompletionRequest::new(
            GPT4_O_2024_08_06.to_string(),
            vec![chat_completion::ChatCompletionMessage {
                role: chat_completion::MessageRole::system,
                content: chat_completion::Content::Text(app_generation_prompt),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
        ))
        .await?;

    if let Some(content) = &result.choices[0].message.content {
        let apps: Vec<App> = serde_json::from_str(content.as_str())?;
        return Ok(apps);
    } else {
        return Err(anyhow!("Missing data"));
    }
}
