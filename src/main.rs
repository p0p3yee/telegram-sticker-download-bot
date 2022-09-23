use teloxide::{prelude::*, utils::command::BotCommands, net::Download, types::InputFile};
use tokio::fs::{File, create_dir, remove_dir_all, remove_file};

use std::error::Error;

mod zip_dir;
use zip_dir::zip_dir;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let bot = Bot::from_env().auto_send();

    teloxide::commands_repl(bot, answer, Command::ty()).await;
}

#[derive(BotCommands, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "Download provided sticker set")]
    Download(String),
}

async fn answer(
    bot: AutoSend<Bot>,
    message: Message,
    command: Command,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Only accept private chat command
    if !message.chat.is_private() {
        return Ok(())
    }

    let chat_id = message.chat.id;
    match command {
        Command::Help => {
            bot.send_message(chat_id, Command::descriptions().to_string()).await?
        }
        Command::Download(sticker_set_name) => {
            let result = bot.get_sticker_set(sticker_set_name).await;
            if result.is_err() {
                println!("{}", result.err().unwrap().to_string());
                bot.send_message(
                    chat_id,
                    "Sticker not found"
                ).await?
            } else {
                let result = result.unwrap();
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

                bot.send_message(chat_id, "Done").await?
            }
        }
    };

    Ok(())
}