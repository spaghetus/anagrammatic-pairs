use indicatif::ProgressBar;
use rand::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

const WORD_LIST: &str = include_str!("../word_list.txt");
const WORD_COUNT_EACH_PAIR: usize = 2;
const MEMBERS_EACH_PAIR: usize = 2;

fn main() {
	let sp = ProgressBar::new_spinner().with_message("Working...");
	let mut rng = thread_rng();
	let words = WORD_LIST
		.lines()
		.filter(|w| w.chars().all(|c| c.is_ascii_alphabetic()))
		.collect::<Vec<_>>();
	let mut pairs = SetOfPairs::new();
	let mut counter: usize = 0;
	let mut found_counter: usize = 0;
	loop {
		let pair: Vec<&str> = words
			.iter()
			.choose_multiple(&mut rng, WORD_COUNT_EACH_PAIR)
			.iter()
			.map(|s| **s)
			.collect();
		let pair = [pair[0], pair[1]];
		let count = pairs.insert(pair);
		if let Some(count) = count {
			found_counter += 1;
			let tty = atty::is(atty::Stream::Stdout);
			if tty {
				println!("\r")
			}
			println!("{:?}", count.0);
			println!("{:?}", pairs.0.get(&count).unwrap());

			if tty {
				println!("\n\n")
			}
		}
		counter += 1;
		if counter % 100 == 0 {
			sp.set_message(format!(
				"Working... {}k pairs processed, {} pairs found",
				counter as f64 / 1000.0,
				found_counter
			));
		}
	}
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct CharacterCount(pub [u8; 26]);
impl CharacterCount {
	fn new() -> Self {
		Self([0; 26])
	}
	fn insert(&mut self, c: char) {
		self.0[(c.to_ascii_lowercase() as u8 - b'a') as usize] += 1;
	}
	fn ingest(&mut self, word: &str) {
		for c in word.chars() {
			self.insert(c);
		}
	}
}

struct SetOfPairs<'a>(BTreeMap<CharacterCount, BTreeSet<[&'a str; WORD_COUNT_EACH_PAIR]>>);
impl<'a> SetOfPairs<'a> {
	fn new() -> Self {
		Self(BTreeMap::new())
	}
	fn insert(&mut self, words: [&'a str; WORD_COUNT_EACH_PAIR]) -> Option<CharacterCount> {
		let mut count = CharacterCount::new();
		for word in words.iter() {
			count.ingest(word);
		}
		let set = self.0.entry(count.clone()).or_insert(BTreeSet::new());
		set.insert(words);
		if set.len() >= MEMBERS_EACH_PAIR {
			Some(count)
		} else {
			None
		}
	}
}
