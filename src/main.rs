#![feature(iter_intersperse)]

use std::time::{SystemTime, UNIX_EPOCH};
use discord_rich_presence::{activity, new_client, DiscordIpc};
use urlencoding::encode;


struct Mpris<'a> {
	source: mpris::Player<'a>,
	title: String,
	album: String,
	artists: String,
	track_url: String,
}

impl Mpris<'_> {
	fn init() -> Self {
		let source = mpris::PlayerFinder::new()
			.expect("D-Bus connection failed")
			.find_active()
			.expect("No player found in D-Bus");
		
		let metadata = source.get_metadata()
							 .expect("Failed to get player metadata");

		let title = metadata.title().unwrap().to_string();
		let album = metadata.album_name().unwrap().to_string();
		let artists = metadata.artists().unwrap()
			.into_iter()
			.intersperse(&", ")
			.map(String::from)
			.collect::<std::string::String>();
		let track_url = metadata.url().unwrap()
			.trim()
			.split(" ")
			.map(String::from)
			.collect::<std::string::String>();

		Self {
			source,
			title,
			album,
			artists,
			track_url,
		}
	}

	fn update(&mut self) {
		let metadata = self.source.get_metadata()
								  .expect("Failed to get player metadata");

		self.title = metadata.title().unwrap().to_string();
		self.album = metadata.album_name().unwrap().to_string();
		self.artists = metadata.artists().unwrap()
			.into_iter()
			.intersperse(&", ")
			.map(String::from)
			.collect::<std::string::String>();
		self.track_url = metadata.url().unwrap()
			.trim()
			.split(" ")
			.map(String::from)
			.collect::<std::string::String>();
	}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut state = Mpris::init();
	
	let mut client = new_client("946585878741024789")?;
	client.connect()?;
	println!("RPC connected");

	let mut url_old: Vec<String> = "".trim().split(' ').map(String::from).collect();
	loop {

		let line_1: &str = &(state.artists.as_str().to_owned() + ": " + &state.album);
		let line_2: &str = &state.title;
		let button_search_url = &(state.artists.to_owned() + " - " + &state.title);
		let button_search_url = &("https://www.last.fm/search/tracks?q=".to_owned() + &encode(button_search_url));


		let timestamps = activity::Timestamps::new().start(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64);

		if state.track_url != url_old[0] {
			let payload = activity::Activity::new()
				.details(line_1)
				.state(line_2)
				//.party(Party::new().size([1, 10]))
				.timestamps(timestamps)
				.assets(
					activity::Assets::new()
						.large_image("cat1")
						.large_text("rich presence api bad. no album art. here is cat instead.")
				)
				.buttons(vec![
					activity::Button::new(
						"Find on Last.fm",
						button_search_url,
					),
				]);
			client.set_activity(payload)?;
		}

		url_old = state.track_url.trim().split(" ").map(String::from).collect();
		std::thread::sleep(std::time::Duration::from_secs(1));
		state.update();
	}

	client.close()?;


	Ok(())
}
