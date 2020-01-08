use chrono::{Datelike, NaiveDateTime};
use indexmap::set::IndexSet;
use std::convert::TryInto;
use std::iter::FromIterator;
use vkopt_message_parser::reader::{fold_html, EventResult, MessageEvent};

const NGRAM_CNT: usize = 2; // Use a bigram markov chain model

pub type ChainPrefix = [u32; NGRAM_CNT]; // indexes into MarkovChain.words

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Datestamp {
    pub year: i16,
    pub day: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChainEntry {
    pub prefix: ChainPrefix,
    pub suffix_word_idx: u32,
    pub datestamp: Datestamp,
}

#[derive(Default, Debug)]
pub struct TextSource {
    pub names: IndexSet<String>,
    pub entries: Vec<ChainEntry>,
}

#[derive(Default, Debug)]
pub struct MarkovChain {
    pub words: IndexSet<String>,
    pub sources: Vec<TextSource>,
}

#[derive(Default)]
struct ExtractedMessage {
    names: Vec<String>,
    datestamp: Datestamp,
    body: String,
}

impl MarkovChain {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn append_text(
        &mut self,
        input_file: &str,
        source_names: Vec<String>,
        datestamp: Datestamp,
    ) {
        let text = std::fs::read_to_string(input_file).unwrap();
        let source = source_by_names(&mut self.sources, source_names);
        push_text_entries(&text, datestamp, &mut source.entries, &mut self.words);
    }

    pub fn append_message_dump(&mut self, input_file: &str) {
        let last_msg = fold_html(
            input_file,
            Default::default(),
            |mut msg: ExtractedMessage, event| match event {
                MessageEvent::Start(0) => {
                    if !msg.body.is_empty() {
                        append_message(self, msg);
                    }
                    EventResult::Consumed(Default::default())
                }
                MessageEvent::FullNameExtracted(full_name) => {
                    msg.names.push(full_name.to_owned());
                    EventResult::Consumed(msg)
                }
                MessageEvent::ShortNameExtracted(short_name) => {
                    msg.names.push(short_name.to_owned());
                    EventResult::Consumed(msg)
                }
                MessageEvent::DateExtracted(date) => {
                    let timestamp =
                        NaiveDateTime::parse_from_str(date, "%Y.%m.%d %H:%M:%S").unwrap();
                    msg.datestamp = Datestamp {
                        year: timestamp.year() as i16,
                        day: timestamp.ordinal() as u16,
                    };
                    EventResult::Consumed(msg)
                }
                MessageEvent::BodyPartExtracted(body) => {
                    msg.body.push_str(body);
                    EventResult::Consumed(msg)
                }
                _ => EventResult::Consumed(msg),
            },
        )
        .unwrap();
        if !last_msg.body.is_empty() {
            append_message(self, last_msg);
        }
    }
}

fn source_by_names(sources: &mut Vec<TextSource>, names: Vec<String>) -> &mut TextSource {
    let idx = sources
        .iter()
        .position(|s| names.iter().any(|n| s.names.contains(n)))
        .unwrap_or_else(|| {
            let new_source = TextSource {
                names: IndexSet::from_iter(names.into_iter()),
                ..Default::default()
            };
            sources.push(new_source);
            sources.len() - 1
        });
    sources.get_mut(idx).unwrap()
}

fn append_message(chain: &mut MarkovChain, message: ExtractedMessage) {
    let source = source_by_names(&mut chain.sources, message.names);
    push_text_entries(
        &message.body,
        message.datestamp,
        &mut source.entries,
        &mut chain.words,
    );
}

fn push_text_entries(
    text: &str,
    datestamp: Datestamp,
    entries: &mut Vec<ChainEntry>,
    words: &mut IndexSet<String>,
) {
    let word_indexes = text
        .split(&[' ', '\n'][..])
        .filter(|word| !word.is_empty())
        .map(|word| words.insert_full(word.to_owned()).0 as u32)
        .collect::<Vec<_>>();

    for ngram in word_indexes.windows(NGRAM_CNT + 1) {
        let (prefix_words, suffix) = ngram.split_at(NGRAM_CNT);
        let prefix: ChainPrefix = prefix_words.try_into().unwrap();
        entries.push(ChainEntry {
            prefix,
            suffix_word_idx: suffix[0],
            datestamp,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authors() {
        let mut chain = MarkovChain::new();
        chain.append_message_dump("tests/fixtures/messages.html");
        assert_eq!(
            chain.sources[0].names,
            vec!["Sota Sota".into(), "sota".into()]
                .into_iter()
                .collect::<IndexSet<_>>()
        );
        assert_eq!(
            chain.sources[1].names,
            vec!["Denko Denko".into(), "denko".into()]
                .into_iter()
                .collect::<IndexSet<_>>()
        );
        println!("{:#?}", chain);
    }

    #[test]
    fn test_word_nodes() {
        let mut chain = MarkovChain::new();
        chain.append_message_dump("tests/fixtures/messages.html");
        assert_eq!(chain.words.get_index(0), Some(&"Привет".into()));
        assert_eq!(chain.words.get_index(1), Some(&"Denko".into()));
        assert_eq!(chain.words.get_index(2), Some(&"Пью".into()));

        assert_eq!(
            chain.sources[0].entries[0],
            ChainEntry {
                prefix: [0, 1],
                suffix_word_idx: 2,
                datestamp: Datestamp {
                    year: 2018,
                    day: 21
                }
            }
        );
    }

    #[test]
    fn test_no_empty_words() {
        let mut chain = MarkovChain::new();
        chain.append_message_dump("tests/fixtures/messages.html");
        let enumerated_words = chain.words.iter().enumerate();
        let empty_words =
            enumerated_words.filter_map(|(i, w)| if w.is_empty() { Some(i) } else { None });
        assert_eq!(empty_words.collect::<Vec<_>>(), vec![0usize; 0]);
    }

    #[test]
    fn test_text() {
        let mut chain = MarkovChain::new();
        chain.append_text(
            "tests/fixtures/text",
            vec!["angus".into(), "sol onset".into()],
            Datestamp { year: 0, day: 0 },
        );
        assert_eq!(
            chain.words,
            vec![
                "useless",
                "unreliable",
                "heavily",
                "distorted",
                "probe",
                "and",
                "flashing",
                "red"
            ]
            .into_iter()
            .map(|s| s.to_owned())
            .collect::<IndexSet<_>>()
        );
        assert_eq!(
            chain.sources[0].names,
            vec!["angus".into(), "sol onset".into()]
                .into_iter()
                .collect::<IndexSet<_>>()
        );
        assert_eq!(
            chain.sources[0].entries[0],
            ChainEntry {
                prefix: [0, 1],
                suffix_word_idx: 2,
                datestamp: Datestamp { year: 0, day: 0 }
            }
        );
    }
}
