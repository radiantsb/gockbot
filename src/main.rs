use dotenv::dotenv;
use poise::{Framework, serenity_prelude as serenity};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::RwLock;
use std::{collections::HashMap, env};
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Serialize, Deserialize, Copy, Clone)]
struct UserChanceConfig {
    im_chance: f32,
    math_chance: f32,
}
#[derive(Serialize, Deserialize)]
struct Data {
    chances: RwLock<HashMap<String, UserChanceConfig>>,
} // User data, which is stored and accessible in all command invocations
impl Data {
    ///writes chances to the specified path
    fn write_to_file(&self, file_name: &str) -> Result<(), Error> {
        let guard = self.chances.read().expect("shits poisoned bro"); //fucking
        //trait bounds mean i cant ? this shit
        let json = serde_json::to_string(&*guard)?;
        fs::write(file_name, json)?;
        Ok(())
    }
    fn get_chances_for_user(&self, user_id: serenity::UserId) -> Result<UserChanceConfig, Error> {
        let guard = self.chances.read().expect("shits poisoned bro"); //trait
        //bounds go brrr
        let chance = guard
            .get(&user_id.to_string())
            .unwrap_or(&UserChanceConfig {
                im_chance: 1.0,
                math_chance: 1.0,
            });
        Ok(chance.clone())
    }
    ///updates the chances for a user and then writes the new hashmap to the specified file, return
    ///type indicates if writing to file was successful
    fn update_chances_for_user(
        &self,
        user_id: serenity::UserId,
        new_chances: UserChanceConfig,
        file_name: &str,
    ) -> Result<(), Error> {
        {
            let mut guard = self.chances.write().expect("shits poisoned bro"); //traits
            guard.insert(user_id.to_string(), new_chances);
        }
        self.write_to_file(file_name)
    }
}
#[poise::command(slash_command)]
async fn setchances(
    ctx: Context<'_>,
    #[description = "new im chance"] imchance: f32,
    #[description = "new math chance"] mathchance: f32,
) -> Result<(), Error> {
    if imchance < 0.0 {
        return Err("chances must be between 0 and 1 (0.5=50%)".into());
    }
    if imchance > 1.0 {
        return Err("chances must be between 0 and 1 (0.5=50%)".into());
    }
    if mathchance < 0.0 {
        return Err("chances must be between 0 and 1 (0.5=50%)".into());
    }
    if mathchance > 1.0 {
        return Err("chances must be between 0 and 1 (0.5=50%)".into());
    }
    let new_chances = UserChanceConfig {
        im_chance: imchance,
        math_chance: mathchance,
    };
    ctx.data()
        .update_chances_for_user(ctx.author().id, new_chances, "config.json")
}
#[tokio::main]
async fn main() {
    dotenv().expect("dotenv failed yo");
    let token = env::var("DISCORD_TOKEN").expect("environment variable DISCORD_TOKEN must be set");

    let config = fs::read_to_string("config.json").unwrap_or("{}".into());
    let data: Data = serde_json::from_str(config.as_str()).expect("json failed yo");

    let intents = serenity::GatewayIntents::non_privileged();
    let framework: Framework<Data, Error> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(data)
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
