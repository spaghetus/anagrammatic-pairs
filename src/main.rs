use indicatif::{HumanDuration, ProgressBar};
use rand::prelude::*;
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::{
		atomic::{AtomicBool, AtomicUsize},
		Arc, Mutex, RwLock,
	},
	thread,
};

const WORD_LIST: &str = include_str!("../word_list.txt");
const WORD_COUNT_EACH_PAIR: usize = 2;
const MEMBERS_EACH_PAIR: usize = 2;

fn main() {
	let sp = Arc::new(Mutex::new(
		ProgressBar::new_spinner().with_message("Working..."),
	));
	let words = Arc::new(
		WORD_LIST
			.lines()
			.filter(|w| w.chars().all(|c| c.is_ascii_alphabetic()))
			.collect::<Vec<_>>(),
	);
	let counter = Arc::new(AtomicUsize::new(0));
	let found_counter = Arc::new(AtomicUsize::new(0));
	let pairs = Arc::new(RwLock::new(SetOfPairs::new()));
	let finish = Arc::new(AtomicBool::new(false));
	let mut threads = Vec::new();
	for _ in 0..num_cpus::get() {
		let words = words.clone();
		let sp = sp.clone();
		let counter = counter.clone();
		let found_counter = found_counter.clone();
		let pairs = pairs.clone();
		let finish = finish.clone();
		threads.push(thread::spawn(move || {
			while !finish.load(std::sync::atomic::Ordering::SeqCst) {
				let mut rng = thread_rng();
				let pair: Vec<&str> = words
					.iter()
					.choose_multiple(&mut rng, WORD_COUNT_EACH_PAIR)
					.iter()
					.map(|s| **s)
					.collect();
				let pair = [pair[0], pair[1]];
				let count = pairs.insert(pair);
				if let Some(count) = count {
					found_counter.fetch_add(1, Ordering::Relaxed);
				}
				let counter = counter.fetch_add(1, Ordering::Relaxed);
				if counter % 100 == 0 {
					let sp = sp.lock().unwrap();
					sp.set_message(format!(
						"Working... {}k pairs processed, {} pairs found, {} elapsed.",
						counter as f64 / 1000.0,
						found_counter.load(Ordering::Relaxed),
						HumanDuration(sp.elapsed())
					));
				}
			}
		}))
	}
	ctrlc::set_handler(move || {
		eprintln!("\n\nWriting to disk...");
		let finish = finish.store(true, Ordering::SeqCst);
		let pair_list = pairs
			.read()
			.unwrap()
			.0
			.iter()
			.filter(|(k, v)| v.len() >= MEMBERS_EACH_PAIR)
			.map(|(k, v)| (k.clone(), v.clone()))
			.collect::<BTreeMap<CharacterCount, BTreeSet<[&str; WORD_COUNT_EACH_PAIR]>>>();
		serde_json::to_writer(std::io::stdout().lock(), &pair_list).unwrap();
	})
	.unwrap();
	for i in threads {
		i.join().unwrap();
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
impl Serialize for CharacterCount {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0
			.iter()
			.enumerate()
			.map(|(i, &v)| {
				let mut s = String::new();
				for _ in 0..v {
					s.push(std::char::from_u32(i as u32 + b'a' as u32).unwrap());
				}
				s
			})
			.collect::<String>()
			.serialize(serializer)
	}
}

#[derive(Serialize)]
struct SetOfPairs<'a>(BTreeMap<CharacterCount, BTreeSet<[&'a str; WORD_COUNT_EACH_PAIR]>>);
impl SetOfPairs<'_> {
	fn new() -> Self {
		Self(BTreeMap::new())
	}
}
trait IngestWordPair<'a> {
	fn insert(&self, words: [&'a str; WORD_COUNT_EACH_PAIR]) -> Option<CharacterCount>;
}
impl<'a> IngestWordPair<'a> for Arc<RwLock<SetOfPairs<'a>>> {
	fn insert(&self, words: [&'a str; WORD_COUNT_EACH_PAIR]) -> Option<CharacterCount> {
		let mut count = CharacterCount::new();
		for word in words.iter() {
			count.ingest(word);
		}
		let mut lock = self.write().unwrap();
		let set = lock.0.entry(count.clone()).or_insert(BTreeSet::new());
		set.insert(words);
		if set.len() >= MEMBERS_EACH_PAIR {
			Some(count)
		} else {
			None
		}
	}
}
