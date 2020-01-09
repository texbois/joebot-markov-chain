use crate::{ChainEntry, Datestamp, MarkovChain};
use rand::{seq::SliceRandom, Rng};

const MAX_TRIES: usize = 20;

pub trait ChainGenerate {
    fn generate<R: Rng>(
        &self,
        rng: &mut R,
        source_names: &[&str],
        min_words: usize,
    ) -> Option<String>;
}

impl ChainGenerate for MarkovChain {
    fn generate<R: Rng>(
        &self,
        rng: &mut R,
        source_names: &[&str],
        min_words: usize,
    ) -> Option<String> {
        let edges = self
            .sources
            .iter()
            .filter(|s| source_names.iter().any(|&n| s.names.contains(n)))
            .map(|s| &s.entries)
            .collect::<Vec<_>>();

        if edges.is_empty() {
            return None;
        }

        generate_sequence(rng, &edges, min_words).map(|s| {
            s.into_iter()
                .filter_map(|word_idx| self.words.get_index(word_idx as usize).map(|w| w.as_str()))
                .collect::<Vec<_>>()
                .join(" ")
        })
    }
}

fn generate_sequence<R: Rng>(
    rng: &mut R,
    edges: &[&Vec<ChainEntry>],
    min_words: usize,
) -> Option<Vec<u32>> {
    let mut tries = 0;
    let mut generated: Vec<u32> = Vec::with_capacity(min_words as usize);
    while tries < MAX_TRIES {
        let mut edge = sample_2d(rng, edges);
        loop {
            for &word in edge.prefix.iter() {
                generated.push(word);
            }
            if generated.len() >= min_words {
                break;
            }
            let next_edges = edges
                .iter()
                .flat_map(|&es| es.iter().filter(|e| e.prefix[0] == edge.suffix_word_idx))
                .collect::<Vec<_>>();
            edge = match next_edges.choose(rng) {
                Some(e) => e,
                None => break,
            }
        }
        if generated.len() >= min_words {
            return Some(generated);
        }
        generated.clear();
        tries += 1;
    }
    None
}

fn sample_2d<'e, T, R: Rng>(rng: &mut R, slices: &'e [&Vec<T>]) -> &'e T {
    let lengths = slices.iter().map(|e| e.len());
    let total_len: usize = slices.iter().map(|e| e.len()).sum();
    let sampled_idx = if total_len <= (core::u32::MAX as usize) {
        rng.gen_range(0, total_len as u32) as usize
    } else {
        rng.gen_range(0, total_len)
    };

    let mut len_iterated = 0;
    for (slice_idx, len) in lengths.enumerate() {
        len_iterated += len;
        if sampled_idx < len_iterated {
            return &slices[slice_idx][sampled_idx - (len_iterated - len)];
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChainAppend, Datestamp, TextSource};
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
                    suffix_word_idx: 2,
                    datestamp: Datestamp {
                        year: 2070,
                        day: 360,
                    },
                },
                ChainEntry {
                    prefix: [4, 5],
                    suffix_word_idx: 6,
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
                suffix_word_idx: 4,
                datestamp: Datestamp {
                    year: 2070,
                    day: 360,
                },
            }],
        });

        let mut rng = SmallRng::from_seed([1; 16]);
        let generated = chain.generate(&mut rng, &["джилл", "дана"], 6);
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
        let generated = chain.generate(&mut rng, &["sota", "denko"], 4);
        assert_eq!(generated, Some("тоже пью жасминовый чай?".into()));
    }
}
