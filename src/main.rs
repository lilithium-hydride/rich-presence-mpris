#![feature(iter_intersperse)]

use std::{thread, sync::mpsc, time::{
	Duration,
	SystemTime,
	UNIX_EPOCH}, io}
;
use std::io::BufRead;
use urlencoding::encode;
use tap::tap::*;
use mpris::{
	Event, 
	EventError
};
use discord_rich_presence::{
	DiscordIpc, 
	DiscordIpcClient, 
	activity::{
		Activity, 
		Assets, 
		Button, 
		Timestamps
	}
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	
	let (update_tx, update_rx): (mpsc::Sender<mpris::Event>, mpsc::Receiver<mpris::Event>)
		= mpsc::channel();

	{
		let (update_tx, update_rx) = (update_tx.clone(), update_rx);
		thread::spawn(move || {
			let mut rpc_client = DiscordIpcClient::new("946585878741024789")
				.expect("Failed to initialize Discord RPC client");
			
			loop {
				match rpc_client.connect() {
					Ok(_) => { 
						eprintln!("RPC connected");
						break;
					}
					Err(err) => {
						eprintln!("Failed to connect to Discord. Is it running?\n\tError: {err}");
						
						// Empty the receiver buffer so it doesn't spam Discord when it opens.
						for _ in update_rx.try_recv().into_iter() {}
					}
				}
				thread::sleep(Duration::from_secs(5));
			}

			let player = mpris::PlayerFinder::new()
				.expect("D-Bus connection failed")
				.find_active()
				.expect("No player found in D-Bus");
			let mut metadata = player
				.get_metadata()
				.expect("Failed to get initial data from MPRIS");
			let search_engine_name = "Last.fm";
			let search_engine_url = "https://www.last.fm/search/tracks?q=";

			// Queue up one loop iteration to send initial RPC data
			update_tx.send(Event::TrackChanged(Default::default()))
				.expect("Failed to send initial RPC data");

			loop {
				for event in update_rx.recv().into_iter() {
					match event {
						Event::Playing => {
							// TODO: Toggle "Elapsed: " timer in status
						}
						Event::Paused => {
							// TODO: Toggle "Elapsed: " timer in status
						}
						Event::Seeked { .. } => {
							// TODO: Edit "Elapsed: " timer to reflect new time
							// Implementing this will require a music player that actually
							// reports the event, and not all do for some reason.
						}
						Event::TrackMetadataChanged { .. }
						| Event::TrackChanged(_) => {
							metadata = player
								.get_metadata()
								.expect("Failed to fetch new track metadata");
						}
						Event::PlayerShutDown
						| Event::Stopped => {
							// Do not quit, just wait around for a new player.
							// Do disconnect from RPC in the meantime.
						}
						_ => {}
					}

					// Need to verify that the intersperse works with a player
					// that actually uses MPRIS properly for multiple artists.
					let artists = match metadata.artists() {
						Some(x) => x,
						None => vec!["N/A"]
					}.into_iter()
					 .intersperse(&", ")
					 .map(String::from)
					 .collect::<std::string::String>();

					let title = match metadata.title() {
						Some(x) => x,
						None => "N/A"
					};
					
					let album =  match metadata.album_name() { 
						Some(x) => x,
						None => title
					};

					let line_1 = &(artists.as_str().to_owned() + ": " + &album.to_string());
					let line_2 = &title;
					let button_search_url = &format!("{}{}", 
					                                 search_engine_url, 
					                                 encode(&format!("{} - {}", artists, title)));
					let button_search_text = &format!("Find on {search_engine_name}");

					let payload = Activity::new()
						.details(line_1)
						.state(line_2)
						.timestamps(
							Timestamps::new().start(
								SystemTime::now()
									.duration_since(UNIX_EPOCH)
									.unwrap()
									.as_secs() as i64
							)
						)
						.assets(
							Assets::new()
								.large_image("cat1")
								.large_text("rich presence api bad. no album art. here is cat instead.")
						)
						.buttons(vec![
							Button::new(
								button_search_text,
								button_search_url,
							),
						]);
					
					match rpc_client.set_activity(payload.clone()) {
						Ok(_) => {}
						Err(err) => {
							eprintln!("Failed to update Discord status. Is it running?\n\tError: {err}");
							eprintln!("Attempting to reconnect…");
							match rpc_client.reconnect() {
								Ok(_) => {
									eprintln!("Reconnect succeeded, resending data…");
									match rpc_client.set_activity(payload) {
										Ok(_) => {
											eprintln!("Data successfully resent, connection is normal.")
										}
										Err(err) => eprintln!("Failed to resend.\n\tError: {err}")
									}
								}
								Err(_) => {
									eprintln!("Failed to reconnect.");
								}
							}
						}
					}
				}
			}
		});
	}
	
	thread::spawn( move || {
		let player = mpris::PlayerFinder::new()
			.expect("D-Bus connection failed")
			.find_active()
			.expect("No player found in D-Bus");
		let player_events = player
			.events()
			.expect("Could not start player event stream");
		
		for event in player_events {
			match event {
				Ok(event) => match event {
					Event::Playing
					| Event::Paused
					| Event::Stopped
					| Event::PlayerShutDown
					| Event::Seeked { .. }
					| Event::TrackMetadataChanged {..}
					| Event::TrackChanged(_) => {
						update_tx.send(event.tap_dbg(|x| eprintln!("{:#?}", x)))
						         .expect("Failed to send MPRIS event update across threads.");
					}
					_ => {}
				}
				Err(err) => {
					println!("D-Bus error: {:?}. Aborting.", err);
					// TODO: what do?
				}
			}
		}
	});

	let stdin = io::stdin();
	for line in stdin.lock().lines() {
		if line.unwrap() == "q" {
			break;
		}
	}
	
	//rpc_client.close()?;
	Ok(())
}
