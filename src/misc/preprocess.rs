use std::{borrow::Cow, collections::HashSet};

pub struct Preprocessor {
    defined: HashSet<String>,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            defined: HashSet::new(),
        }
    }

    pub fn define(mut self, name: &str) -> Self {
        self.defined.insert(name.to_string());
        self
    }

    pub fn define_cond(self, name: &str, cond: bool) -> Self {
        if cond {
            self.define(name)
        } else {
            self
        }
    }

    pub fn process<'a>(&self, input: &'a str) -> Cow<'a, str> {
        let mut out = String::new();
        let mut dirty = false;

        let lines = input.lines().collect::<Vec<_>>();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            if line.trim_start().starts_with("// #if ") {
                let expr = line.trim().strip_prefix("// #if ").unwrap();

                let mut block = Vec::new();
                while i < lines.len() {
                    i += 1;
                    let line = lines[i];

                    if line.trim_start().starts_with("// #endif") {
                        break;
                    }

                    block.push(line);
                }

                if self.defined.contains(expr) {
                    out.push_str(&block.join("\n"));
                    out.push('\n');
                } else {
                    dirty = true;
                }
            } else {
                out.push_str(line);
                out.push('\n');
            }

            i += 1;
        }

        if dirty {
            Cow::Owned(out)
        } else {
            Cow::Borrowed(input)
        }
    }
}
