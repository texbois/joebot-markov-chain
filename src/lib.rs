use pyo3::prelude::*;
use pyo3::types::PyDict;

const MARKOVIFY_SETUP: &'static str = r#"
model = markovify.Text.from_json(model_json)
def generate(sentence_len):
    text = None
    while text is None:
        text = model.make_short_sentence(sentence_len)
    return text
"#;

pub struct MarkovChain {
    globals: PyObject,
}

impl MarkovChain {
    pub fn create(model_path: &str) -> PyResult<Self> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        let model = std::fs::read_to_string(model_path)?;

        let globals_dict = PyDict::new(py);
        globals_dict.set_item("markovify", py.import("markovify")?)?;
        globals_dict.set_item("model_json", model)?;

        py.run(MARKOVIFY_SETUP, Some(&globals_dict), None)?;

        let globals = globals_dict.to_object(py);
        Ok(Self { globals })
    }

    pub fn make_sentence(&self, len: u32) -> PyResult<String> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let globals = Some(self.globals.extract(py)?);
        py.eval(&format!("generate({})", len), globals, None)?
            .extract()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_filename() {
        assert!(MarkovChain::create("modell.json").is_err());
    }

    #[test]
    fn test_make_sentence() {
        let text = MarkovChain::create("model.json").and_then(|chain| chain.make_sentence(40));
        match text {
            Ok(text) => assert!(text.len() > 0),
            Err(err) => {
                let gil = Python::acquire_gil();
                let py = gil.python();
                err.print(py);
                panic!()
            }
        }
    }
}
