use crate::{ChainEntry, Datestamp, MarkovChain, TextSource};
use indexmap::IndexSet;
use rand::{seq::SliceRandom, Rng};

const MAX_TRIES: usize = 20;

pub trait ChainGenerate {
    fn generate<'a, R: Rng, I: IntoIterator<Item = &'a TextSource>>(
        &self,
        rng: &mut R,
        sources: I,
        min_words: usize,
        max_words: usize,
    ) -> Option<String>;

    fn generate_in_date_range<'a, R: Rng, I: IntoIterator<Item = &'a TextSource>>(
        &self,
        rng: &mut R,
        sources: I,
        date_range: (Datestamp, Datestamp),
        min_words: usize,
        max_words: usize,
    ) -> Option<String>;
}

impl ChainGenerate for MarkovChain {
    fn generate<'a, R: Rng, I: IntoIterator<Item = &'a TextSource>>(
        &self,
        rng: &mut R,
        sources: I,
        min_words: usize,
        max_words: usize,
    ) -> Option<String> {
        let edges = sources
            .into_iter()
            .flat_map(|s| &s.entries)
            .collect::<Vec<_>>();
        if !edges.is_empty() {
            generate_sequence(rng, &edges, min_words, max_words)
                .map(|s| seq_to_text(s, &self.words))
        } else {
            None
        }
    }

    fn generate_in_date_range<'a, R: Rng, I: IntoIterator<Item = &'a TextSource>>(
        &self,
        rng: &mut R,
        sources: I,
        date_range: (Datestamp, Datestamp),
        min_words: usize,
        max_words: usize,
    ) -> Option<String> {
        let edges = sources
            .into_iter()
            .flat_map(|s| &s.entries)
            .filter(|e| e.datestamp >= date_range.0 && e.datestamp <= date_range.1)
            .collect::<Vec<_>>();
        if !edges.is_empty() {
            generate_sequence(rng, &edges, min_words, max_words)
                .map(|s| seq_to_text(s, &self.words))
        } else {
            None
        }
    }
}

fn seq_to_text(seq: Vec<u32>, words: &IndexSet<String>) -> String {
    seq.into_iter()
        .filter_map(|word_idx| words.get_index(word_idx as usize).map(|w| w.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_sequence<R: Rng>(
    rng: &mut R,
    edges: &[&ChainEntry],
    min_words: usize,
    max_words: usize,
) -> Option<Vec<u32>> {
    let mut tries = 0;
    let mut generated: Vec<u32> = Vec::with_capacity(min_words as usize);
    while tries < MAX_TRIES {
        let mut edge = edges.choose(rng).unwrap();
        loop {
            generated.extend_from_slice(&edge.prefix);
            if generated.len() >= min_words && edge.suffix.is_terminal() {
                generated.push(edge.suffix.word_idx());
                return Some(generated);
            } else if generated.len() >= max_words {
                break;
            }
            let next_edges = edges
                .iter()
                .filter(|e| e.prefix[0] == edge.suffix.word_idx())
                .collect::<Vec<_>>();
            edge = match next_edges.choose(rng) {
                Some(e) => e,
                None => break,
            }
        }
        generated.clear();
        tries += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChainAppend, ChainSuffix, Datestamp, TextSource};
    use indexmap::indexset;
    use rand::{rngs::SmallRng, SeedableRng};

    #[test]
    fn test_determined_generation() {
        let mut chain: MarkovChain = Default::default();
        chain.words.insert("сегодня".into());
        chain.words.insert("у".into());
        chain.words.insert("меня".into());
        chain.words.insert("депрессия".into());
        chain.words.insert("с".into());
        chain.words.insert("собаками".into());

        chain.sources.push(TextSource {
            names: indexset!["дана".into()],
            entries: vec![
                ChainEntry {
                    prefix: [0, 1],
                    suffix: ChainSuffix::nonterminal(2),
                    datestamp: Datestamp {
                        year: 2070,
                        day: 360,
                    },
                },
                ChainEntry {
                    prefix: [4, 5],
                    suffix: ChainSuffix::terminal(6),
                    datestamp: Datestamp {
                        year: 2070,
                        day: 360,
                    },
                },
            ],
        });
        chain.sources.push(TextSource {
            names: indexset!["джилл".into()],
            entries: vec![ChainEntry {
                prefix: [2, 3],
                suffix: ChainSuffix::nonterminal(4),
                datestamp: Datestamp {
                    year: 2070,
                    day: 360,
                },
            }],
        });

        let mut rng = SmallRng::from_seed([1; 16]);
        let generated = chain.generate(&mut rng, chain.sources.iter(), 5, 6);
        assert_eq!(
            generated,
            Some("сегодня у меня депрессия с собаками".into())
        );
    }

    #[test]
    fn test_random_generation() {
        let mut chain = MarkovChain::new();
        chain.append_message_dump("tests/fixtures/messages.html");
        let mut rng = SmallRng::from_seed([1; 16]);
        let generated = chain.generate(&mut rng, chain.sources.iter(), 3, 5);
        assert_eq!(generated, Some("тоже пью жасминовый чай? 🤔🤔🤔".into()));
    }

    #[test]
    fn test_date_range_generation() {
        let mut chain = MarkovChain::new();
        chain.append_message_dump("tests/fixtures/messages.html");
        let mut rng = SmallRng::from_seed([1; 16]);
        let generated = chain.generate_in_date_range(
            &mut rng,
            chain.sources.iter(),
            (
                Datestamp {
                    year: 2018,
                    day: 10,
                },
                Datestamp {
                    year: 2018,
                    day: 21,
                },
            ),
            3,
            6,
        );
        assert_eq!(generated, Some("Denko Пью жасминовый чай (´･ω･`)".into()));
    }
}
