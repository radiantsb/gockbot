use chrono::Local;
use dotenv::dotenv;
use poise::serenity_prelude::{Mentionable, UserId};
use poise::{Framework, serenity_prelude as serenity};
use rand::random;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{LazyLock, RwLock};
use std::time::Duration;
use std::{collections::HashMap, env};
//match group 2 for the thing they said they are
static IM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[iI]([mM]|'[mM]| [aA][mM]) (.*)").unwrap());
static FACTORIAL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([0123456789]+)!").unwrap());
static OTHER_MATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([0123456789]+)([-+*x^])([0123456789]+)").unwrap());
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
    if ctx
        .data()
        .update_chances_for_user(ctx.author().id, new_chances, "config.json")
        .is_ok()
    {
        ctx.reply(format!(
            "new im chance:{}\nnew math chance:{}",
            imchance, mathchance
        ))
        .await;
        Ok(())
    } else {
        Err("internal error while updating chance".into())
    }
}
#[poise::command(slash_command)]
async fn factorial(
    ctx: Context<'_>,
    #[description = "the number to find the factorial of"] num: u64,
) -> Result<(), Error> {
    if let Some(answer) = BigNumber::new_factorial(num) {
        ctx.reply(answer.to_string()).await;
        Ok(())
    } else {
        Err("number must be between 0 and 1000000000".into())
    }
}
#[poise::command(slash_command)]
async fn power(
    ctx: Context<'_>,
    #[description = "base of the power"] base: u64,
    #[description = "exponent"] exponent: u64,
) -> Result<(), Error> {
    let answer = BigNumber::new_pow(base, exponent);
    ctx.reply(answer.to_string()).await;
    Ok(())
}
enum BigNumber {
    Small(u64, Duration),
    SmallSigned(i64, Duration),
    Large(f64, f64, Duration),
}
impl BigNumber {
    fn to_string(&self) -> String {
        match self {
            BigNumber::Small(num, time) => {
                format!("{}\n-# calculated in {}s", num, time.as_secs_f64())
            }
            BigNumber::SmallSigned(num, time) => {
                format!("{}\n-# calculated in {}s", num, time.as_secs_f64())
            }
            BigNumber::Large(mantissa, exponent, time) => format!(
                "{}x10^{}\n-# calculated in {}s",
                mantissa,
                exponent,
                time.as_secs_f64()
            ),
        }
    }
    ///calculates factorial by multiplying integers, returns None if num>20 as that would cause a
    ///u64 overflow, safe to unwrap if input <=20
    fn new_small_factorial(num: u64) -> Option<BigNumber> {
        if num == 0 {
            return Some(BigNumber::Small(1, Duration::from_secs(0))); //it didnt take time trust
        }
        if num > 20 {
            return None;
        }
        let start = Local::now();
        let mut result = 1u64;
        for i in 1..=num {
            result *= i;
        }
        let dur = Local::now() - start;
        Some(BigNumber::Small(
            result,
            Duration::from_secs_f64(dur.as_seconds_f64()), //fuck this shit
        ))
    }
    fn new_large_factorial(num: u64) -> BigNumber {
        let start = Local::now();
        let mut sum = 0f64;
        for i in 1..=num {
            sum += (i as f64).log10();
        }
        BigNumber::Large(
            10f64.powf(sum.fract()),
            sum.floor(),
            Duration::from_secs_f64((Local::now() - start).as_seconds_f64()), //fucking stupid
        )
    }
    fn new_factorial(num: u64) -> Option<BigNumber> {
        match num {
            0..=20 => BigNumber::new_small_factorial(num),
            21..=1_000_000_000 => Some(BigNumber::new_large_factorial(num)),
            _ => None,
        }
    }
    fn new_multiply(a: u64, b: u64) -> BigNumber {
        let start = Local::now();
        let log_sum = (a as f64).log10() + (b as f64).log10();
        if log_sum > 19f64 {
            BigNumber::Large(
                10f64.powf(log_sum.fract()),
                log_sum.floor(),
                Duration::from_secs_f64((Local::now() - start).as_seconds_f64()),
            )
        } else {
            BigNumber::Small(
                a * b,
                Duration::from_secs_f64((Local::now() - start).as_seconds_f64()),
            )
        }
    }
    fn new_pow(a: u64, b: u64) -> BigNumber {
        let start = Local::now();
        let log_mult = (a as f64).log10() * b as f64;
        if log_mult > 19f64 {
            BigNumber::Large(
                10f64.powf(log_mult.fract()),
                log_mult.floor(),
                Duration::from_secs_f64((Local::now() - start).as_seconds_f64()),
            )
        } else {
            BigNumber::Small(
                a.pow(b as u32),
                Duration::from_secs_f64((Local::now() - start).as_seconds_f64()),
            )
        }
    }
    fn new_math(a: u64, op: &str, b: u64) -> Option<BigNumber> {
        match op {
            "+" => Some(BigNumber::Small(a + b, Duration::from_secs_f64(0f64))),
            "-" => Some(BigNumber::SmallSigned(
                (a as i64) - (b as i64),
                Duration::from_secs_f64(0f64),
            )),
            "*" => Some(BigNumber::new_multiply(a, b)),
            "x" => Some(BigNumber::new_multiply(a, b)),
            "^" => Some(BigNumber::new_pow(a, b)),
            _ => None,
        }
    }
}
///processes a message and returns a reply assuming that the message does not ping the bot
#[allow(clippy::collapsible_if, unused)]
async fn get_reply(
    text: &str,
    user_id: UserId,
    bot_mention: &str,
    chances: UserChanceConfig,
) -> Option<String> {
    if random::<f32>() < chances.im_chance {
        if let Some(captures) = IM_RE.captures(text) {
            if let Some(matched) = captures.get(2) {
                return Some(format!("hi {} im {}", matched.as_str(), bot_mention));
            }
        }
    }
    if random::<f32>() < chances.math_chance {
        if let Some(captures) = FACTORIAL_RE.captures(text) {
            if let Some(matched) = captures.get(1) {
                if let Ok(num) = matched.as_str().parse::<u64>() {
                    if let Some(factorial) = BigNumber::new_factorial(num) {
                        return Some(format!("{}! = {}", num, factorial.to_string()));
                    }
                }
            }
        }
        if let Some(captures) = OTHER_MATH_RE.captures(text) {
            if let (Some(a), Some(op), Some(b)) =
                (captures.get(1), captures.get(2), captures.get(3))
            {
                if let (Ok(a), op, Ok(b)) = (
                    a.as_str().parse::<u64>(),
                    op.as_str(),
                    b.as_str().parse::<u64>(),
                ) {
                    if let Some(result) = BigNumber::new_math(a, op, b) {
                        return Some(format!("{}{}{} = {}", a, op, b, result.to_string()));
                    }
                }
            }
        }
    }
    None
}
#[allow(clippy::collapsible_if, unused)]
async fn get_reply_to_ping(
    text: &str,
    user_id: UserId,
    bot_mention: &str,
    chances: UserChanceConfig,
) -> Option<String> {
    if text.contains("is ") {
        if text.contains("‍") {
            //zero width joiner
            return Some("nuh".to_string());
        }
        return Some("yeh".to_string());
    }
    get_reply(text, user_id, bot_mention, chances).await
}
#[tokio::main]
#[allow(clippy::collapsible_if, unused)]
async fn main() {
    dotenv().expect("dotenv failed yo");
    let token = env::var("DISCORD_TOKEN").expect("environment variable DISCORD_TOKEN must be set");

    let config = fs::read_to_string("config.json").unwrap_or("{}".into());
    let data: Data = Data {
        chances: serde_json::from_str(config.as_str()).expect("json failed yo"),
    };

    let mut intents = serenity::GatewayIntents::non_privileged();
    intents.insert(serenity::GatewayIntents::MESSAGE_CONTENT);
    let framework: Framework<Data, Error> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![setchances(), factorial(), power()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(async move {
                    let my_user_id = ctx.http.get_current_user().await?.id;
                    if let serenity::FullEvent::Message { new_message } = event {
                        if new_message.author.id == my_user_id {
                            return Ok(());
                        }
                        if let Ok(true) = new_message.mentions_me(&ctx.http).await {
                            if let Ok(chances) = data.get_chances_for_user(new_message.author.id) {
                                let reply = get_reply_to_ping(
                                    &new_message.content,
                                    new_message.author.id,
                                    &my_user_id.mention().to_string(),
                                    chances,
                                )
                                .await;
                                if let Some(reply) = reply {
                                    new_message.reply_ping(&ctx.http, reply).await;
                                }
                            }
                        } else {
                            if let Ok(chances) = data.get_chances_for_user(new_message.author.id) {
                                let reply = get_reply(
                                    &new_message.content,
                                    new_message.author.id,
                                    &my_user_id.mention().to_string(),
                                    chances,
                                )
                                .await;
                                if let Some(reply) = reply {
                                    new_message.reply_ping(&ctx.http, reply).await;
                                }
                            }
                        }
                    }

                    Ok(())
                })
            },
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
