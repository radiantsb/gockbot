use chrono::Local;
use dotenv::dotenv;
use poise::serenity_prelude::{Mentionable, User};
use poise::{Framework, serenity_prelude as serenity};
use rand::random;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::sync::{LazyLock, RwLock};
use std::time::Duration;
use std::{collections::HashMap, env};
//match group 2 for the thing they said they are
static IM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[iI]([mM]|'[mM]| [aA][mM]) (.*)").unwrap());
static FACTORIAL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([0-9]+)!").unwrap());
static OTHER_MATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([0-9]+)([-+*x^])([0-9]+)").unwrap());
static QUESTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(is |will |am |was |were |are |do |does |did |has |had |have |can |could |will |would |should |\?)").unwrap()
});
//regex for responding :4 if someone says :3
static INCREMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r":([0-9]+)(?:\s|$)").unwrap());
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
        let guard = match self.chances.read() {
            Ok(ok) => ok,
            Err(_e) => {
                return Err("internal error accessing chances (RwLock poisoned)".into());
            }
        };
        let json = serde_json::to_string(&*guard)?;
        fs::write(file_name, json)?;
        Ok(())
    }
    fn get_chances_for_user(&self, user_id: serenity::UserId) -> Result<UserChanceConfig, Error> {
        let guard = match self.chances.read() {
            Ok(ok) => ok,
            Err(_e) => {
                return Err("internal error accessing chances (RwLock poisoned)".into());
            }
        };
        let chance = guard
            .get(&user_id.to_string())
            .unwrap_or(&UserChanceConfig {
                im_chance: 1.0,
                math_chance: 1.0,
            });
        Ok(*chance)
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
            let mut guard = match self.chances.write() {
                Ok(ok) => ok,
                Err(_e) => {
                    return Err("internal error accessing chances (RwLock poisoned)".into());
                }
            };
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
        let _ = ctx
            .reply(format!(
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
        let _ = ctx.reply(answer.to_string()).await;
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
    let _ = ctx.reply(answer.to_string()).await;
    Ok(())
}
#[poise::command(slash_command)]
async fn ban(ctx: Context<'_>, #[description = "the user to ban"] user: User) -> Result<(), Error> {
    let _ = ctx
        .reply(format!("successfully banned {}", user.mention()))
        .await;
    Ok(())
}
enum BigNumber {
    Small(u64, Duration),
    SmallSigned(i64, Duration),
    Large(f64, f64, Duration),
}
impl fmt::Display for BigNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BigNumber::Small(num, time) => {
                writeln!(f, "{}", num)?;
                write!(f, "-# calculated in {}s", time.as_secs_f64())
            }
            BigNumber::SmallSigned(num, time) => {
                writeln!(f, "{}", num)?;
                write!(f, "-# calculated in {}s", time.as_secs_f64())
            }
            BigNumber::Large(mantissa, exponent, time) => {
                writeln!(f, "{}x10^{}", mantissa, exponent)?;
                write!(f, "-# calculated in {}s", time.as_secs_f64())
            }
        }
    }
}
impl BigNumber {
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
async fn get_reply(text: &str, bot_mention: &str, chances: UserChanceConfig) -> Option<String> {
    if random::<f32>() < chances.im_chance
        && let Some(captures) = IM_RE.captures(text)
        && let Some(matched) = captures.get(2)
    {
        return Some(format!("hi {} im {}", matched.as_str(), bot_mention));
    }
    if random::<f32>() < chances.math_chance {
        if let Some(captures) = FACTORIAL_RE.captures(text)
            && let Some(matched) = captures.get(1)
            && let Ok(num) = matched.as_str().parse::<u64>()
            && let Some(factorial) = BigNumber::new_factorial(num)
        {
            return Some(format!("{}! = {}", num, factorial));
        }
        if let Some(captures) = OTHER_MATH_RE.captures(text)
            && let (Some(a), Some(op), Some(b)) =
                (captures.get(1), captures.get(2), captures.get(3))
            && let (Ok(a), op, Ok(b)) = (
                a.as_str().parse::<u64>(),
                op.as_str(),
                b.as_str().parse::<u64>(),
            )
            && let Some(mut result) = BigNumber::new_math(a, op, b)
        {
            //simulate integer underflow if message contains zwj
            if op == "-" && text.contains("‍") {
                //zero width joiner
                if let BigNumber::SmallSigned(res, dur) = result {
                    //doing it with shit over 0 will fuck stuff up differently methinks
                    if res < 0 {
                        result = BigNumber::Small(
                            //effectively calculates u64::MAX + result but requires multiple
                            //steps to convert stuff
                            ((i64::MAX + res) as u64) + (u64::MAX - (i64::MAX as u64)),
                            dur,
                        );
                    }
                    return Some(format!("{}{}{} = {}", a, op, b, result));
                }
            }
        } else if let Some(captures) = INCREMENT_RE.captures(text)
            && let Some(num) = captures.get(1)
            && let Ok(num) = num.as_str().parse::<u64>()
        {
            return Some(format!(":{}", num + 1));
        }
    }
    None
}
async fn get_reply_to_ping(
    text: &str,
    bot_mention: &str,
    chances: UserChanceConfig,
) -> Option<String> {
    if QUESTION_RE.is_match(text) {
        if text.contains("‍") {
            //zero width joiner
            return Some("nuh".to_string());
        }
        return Some("yeh".to_string());
    }
    get_reply(text, bot_mention, chances).await
}
#[tokio::main]
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
            commands: vec![setchances(), factorial(), power(), ban()],
            event_handler: |ctx, event, _framework, data| {
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
                                    &my_user_id.mention().to_string(),
                                    chances,
                                )
                                .await;
                                if let Some(reply) = reply {
                                    let _ = new_message.reply_ping(&ctx.http, reply).await;
                                }
                            }
                        } else {
                            if let Ok(chances) = data.get_chances_for_user(new_message.author.id) {
                                let reply = get_reply(
                                    &new_message.content,
                                    &my_user_id.mention().to_string(),
                                    chances,
                                )
                                .await;
                                if let Some(reply) = reply {
                                    let _ = new_message.reply_ping(&ctx.http, reply).await;
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
