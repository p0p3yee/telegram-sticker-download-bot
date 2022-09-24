use lazy_static::lazy_static;
use regex::Regex;
use teloxide::{prelude::*, utils::command::BotCommands, net::Download, types::{InputFile, StickerSet}};
use tokio::fs::{File, create_dir, remove_dir_all, remove_file};

use std::{error::Error};

mod zip_dir;
use zip_dir::zip_dir;

lazy_static! {
    static ref ISLINK: Regex = Regex::new(r"^https://t\.me/(addemoji|addstickers)/(\S+)$").unwrap();
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let bot = Bot::from_env().auto_send();

    let handler = Update::filter_message()
    .branch(
        dptree::entry()
            .filter(|msg: Message| msg.chat.is_private())
            .filter_command::<Command>()
            .endpoint(answer),   
    )
    .branch(
        dptree::filter(|msg: Message| msg.chat.is_private())
            .endpoint(|msg: Message, bot: AutoSend<Bot>| async move {
                let mut sticker_set_name = "";
                if msg.text().is_some() {
                    let text = msg.text().unwrap();
                    if let Some(x) = ISLINK.captures(text) {
                        sticker_set_name = x.get(2).unwrap().as_str();
                    } else {
                        bot.send_message(msg.chat.id, "Incorrect url for emoji / sticker set").await?;
                    }
                    
                } else if msg.sticker().is_some() {
                    let sticker = msg.sticker().unwrap();
                    if sticker.set_name.is_some() {
                        sticker_set_name = &sticker.set_name.as_ref().unwrap();
                    } else {
                        bot.send_message(msg.chat.id, "Sticker set name not found").await?;
                    }
                }

                if !sticker_set_name.is_empty() {
                    handle_sticker_set_name(bot, msg.chat.id, sticker_set_name).await?;
                }

                Ok(())
            }),
    );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(BotCommands, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "Download provided sticker set")]
    Download(String),
}

async fn handle_sticker_set(bot: AutoSend<Bot>, chat_id: ChatId, result: StickerSet) -> Result<(), Box<dyn Error + Send + Sync>> {
    bot.send_message(
        chat_id,
        format!("Downloading {} stickers...", result.stickers.len())
    )
    .await?;

    let new_file_dir = format!("{}_{}", chat_id, result.name);
    create_dir(new_file_dir.clone()).await?;
    create_dir(format!("{}/stickers", new_file_dir)).await?;

    for sticker in result.stickers {
        let file = bot.get_file(sticker.file_id).await?;
        let target_path = file.file_path;
        let new_file_path = format!("{}/{}", new_file_dir, target_path);
        let mut new_file = File::create(new_file_path.clone()).await?;
        println!("New Path: {}", new_file_path);
        bot.download_file(&target_path, &mut new_file).await?;
    }

    let zip_path = format!("{}.zip", new_file_dir);
    zip_dir(&new_file_dir, &zip_path)?;
    bot.send_document(
        chat_id,
        InputFile::read(
            File::open(zip_path.clone()).await?
        ).file_name(zip_path.clone())
    ).await?;
    remove_dir_all(new_file_dir).await?;
    remove_file(zip_path).await?;

    Ok(())
}

async fn handle_sticker_set_name(bot: AutoSend<Bot>, chat_id: ChatId, sticker_set_name: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let result = bot.get_sticker_set(sticker_set_name).await;
    if result.is_err() {
        bot.send_message(
            chat_id,
            "Sticker not found"
        ).await?;
    } else {
        let result = result.unwrap();
        handle_sticker_set(bot.clone(), chat_id, result).await?;
        bot.send_message(chat_id, "Done").await?;
    }
    Ok(())
}

async fn answer(
    bot: AutoSend<Bot>,
    message: Message,
    command: Command,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chat_id = message.chat.id;
    match command {
        Command::Help => {
            bot.send_message(chat_id, Command::descriptions().to_string()).await?
        }
        Command::Download(sticker_set_name) => {
            handle_sticker_set_name(bot, chat_id, &sticker_set_name).await?;
            message
        }
    };

    Ok(())
}