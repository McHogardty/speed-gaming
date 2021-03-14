

use std::env;
use std::time::Duration;

use serenity::{
    async_trait,
    model::{prelude::*, channel::Message, gateway::Ready},
    prelude::*
};

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
                tokio::time::sleep(Duration::from_secs(60 * 30)).await;
                match message.delete(ctx.http).await {
                    Ok(_) => println!("Successfully deleted message {}!", message.id),
                    Err(why) => println!("Error deleting message {}: {}", message.id, why)
                }
            });
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
