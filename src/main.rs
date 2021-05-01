

use std::env;
use std::time::Duration;

use chrono::prelude::*;
use serenity::{
    async_trait,
    model::{prelude::*, channel::Message, gateway::Ready},
    prelude::*
};

const MAX_MESSAGE_AGE: Duration = Duration::from_secs(60 * 30);

// Our implementation of the event handler for the Discord gateway.
// Stores the currently active Guild ID and Channel ID to ensure that it only
// deletes messages for a specific channel in a specific guild.
struct Handler {
    active_guild_id: GuildId,
    active_channel_id: ChannelId
}

#[async_trait]
impl EventHandler for Handler {
    // Handle a message.
    async fn message(&self, ctx: Context, message: Message) {
        // Only schedule the message for deletion if the message is from the active guild and channel.
        if message.guild_id.unwrap() == self.active_guild_id && message.channel_id == self.active_channel_id {
            println!("Scheduling message {} for deletion in 30 minutes.", message.id);

            // Spawn a background thread which sleeps for 30 minutes before waking and deleting the message.
            tokio::spawn(async move {
                tokio::time::sleep(MAX_MESSAGE_AGE).await;
                match message.delete(ctx.http).await {
                    Ok(_) => println!("Successfully deleted message {}!", message.id),
                    Err(why) => {
                        println!("Error deleting message {}: {}", message.id, why);
                    }
                }
            });
        }
    }

    // A handler which is called when data for a specific guild is sent to the bot. Called in one of two
    // circumstances:
    // 1. On startup, for each guild which has already had the bot added.
    // 2. Whenever a new guild is added and the bot is running.
    async fn guild_create(&self, ctx: Context, guild: Guild) {
        println!("{:?} is created!", guild.id);

        // Look for the active channel in the channel list.
        if let Some(channel) = guild.channels.get(&self.active_channel_id) {
            println!("Found channel {:?}", channel);

            // Check to see if the active channel has a last message.
            if let Some(mut last_message_id) = channel.last_message_id {
                // Retrieve all of the message history for the channel to delete the messages.
                // If the message is older than 30 minutes, then delete it immediately.
                let mut messages_to_delete: Vec<Message> = Vec::new();

                // Using "before" to get messages before a particular ID is NOT inclusive, which means it
                // skips the very last message in the channel. We first use "most recent" to make sure we
                // don't miss any messages.
                let mut messages_result = channel.messages(&ctx.http, |retriever| {
                    // Get the 50 most recent messages in the channel.
                    retriever
                }).await;

                let utc_now = Utc::now();

                loop {
                    println!("Loop started. Getting messages.");

                    println!("Matching result.");
                    match messages_result {
                        // messages is a Vec which means that to modify it (using pop)
                        // we must declare it as mutable.
                        Ok(messages) => {
                            println!("Got messages {:?}", messages);

                            if let Some(last_message) = messages.last() {
                                last_message_id = last_message.id;
                            } else {
                                println!("Got no last message.");
                                break;
                            }

                            messages_to_delete.extend(messages.into_iter().filter(|m| {
                                if let Ok(message_age) = utc_now.signed_duration_since(m.timestamp).to_std() {
                                    return !m.pinned && message_age > MAX_MESSAGE_AGE;
                                } else {
                                    // This branch will be reached if conversion to a standard library duration fails. This only
                                    // occurs for a negative duration, i.e. the timestamp occurs after the current time, which means
                                    // it is less than 30 minutes old, so we don't want to delete it.
                                    return false;
                                }
                            }));
                        },
                        Err(err) => {
                            println!("Error retrieving messages {:?}", err);
                            break;
                        }
                    }

                    messages_result = channel.messages(&ctx.http, |retriever| {
                            // Get the 50 messages before last_message_id (inclusive).
                            retriever.before(last_message_id)
                    }).await;
                }

                println!("Messages to delete is {:?}", messages_to_delete);

                for message_id in messages_to_delete {
                    match message_id.delete(&ctx.http).await {
                        Ok(_) => println!("Successfully deleted message {:?}!", message_id),
                        Err(why) => {
                            println!("Error deleting message {:?}: {}", message_id, why);
                        }
                    }
                }
            }
        }
    }

    // A simple ready event handler to print when the gateway is ready to start sending other events.
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}


#[tokio::main]
async fn main() {
    // The discord token is required to authenticate the bot to the discord API.
    let token = env::var("DISCORD_TOKEN").expect("token");

    // Get the active guild ID and channel ID from the environment.
    let guild_id_input = env::var("ACTIVE_GUILD_ID").expect("guild_id");
    let channel_id_input = env::var("ACTIVE_CHANNEL_ID").expect("channel_id");

    let guild_id = str::parse::<u64>(&guild_id_input).expect("guild_id is not a valid integer.");
    let channel_id = str::parse::<u64>(&channel_id_input).expect("channel_id is not a valid integer.");

    // Initialise the client and start connecting to the gateway.
    let mut client = Client::builder(&token)
        .event_handler(Handler{active_guild_id: GuildId(guild_id), active_channel_id: ChannelId(channel_id)})
        .await
        .expect("Error creating client.");

    if let Err(why) = client.start().await {
        println!("Client error: {}", why);
    }
}
