use std::{borrow::Cow, collections::HashMap, fmt::Write};

pub struct Preprocessor {
    defined: HashMap<String, Data>,
}

#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub enum Data {
    Bool(bool),
    I32(i32),
    U32(u32),
    F32(f32),
    F16(f32),
    Vec { n: usize, data: Vec<Data> },
    Null,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            defined: HashMap::new(),
        }
    }

    pub fn define(mut self, name: &str, data: Data) -> Self {
        self.defined.insert(name.to_string(), data);
        self
    }

    pub fn process(&self, input: &str) -> String {
        let mut out = String::new();

        for (name, value) in self.defined.iter().filter(|x| x.1 != &Data::Null) {
            out.write_fmt(format_args!(
                "const {name}: {} = {};\n",
                value.as_type(),
                value.as_value(),
                name = name
            ))
            .unwrap();
        }

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

                if self.defined.contains_key(expr) {
                    out.push_str(&block.join("\n"));
                    out.push('\n');
                }
            } else {
                out.push_str(line);
                out.push('\n');
            }

            i += 1;
        }

        out
    }
}

impl Data {
    fn as_type(&self) -> Cow<'static, str> {
        match self {
            Data::Bool(_) => Cow::Borrowed("bool"),
            Data::I32(_) => Cow::Borrowed("i32"),
            Data::U32(_) => Cow::Borrowed("u32"),
            Data::F32(_) => Cow::Borrowed("f32"),
            Data::F16(_) => Cow::Borrowed("f16"),
            Data::Null => Cow::Borrowed(""),
            Data::Vec { n, data } => Cow::Owned(format!("vec{n}<{}>", data[0].as_type())),
        }
    }

    fn as_value(&self) -> String {
        let mut out = String::new();

        match self {
            Data::Bool(x) => out.push_str(&x.to_string()),
            Data::I32(x) => out.push_str(&x.to_string()),
            Data::U32(x) => out.push_str(&x.to_string()),
            Data::F32(x) => out.push_str(&x.to_string()),
            Data::F16(x) => out.push_str(&x.to_string()),
            Data::Vec { n, data } => {
                let data = data
                    .iter()
                    .map(|x| x.as_value())
                    .collect::<Vec<_>>()
                    .join(", ");
                out.write_fmt(format_args!("vec{n}({data})")).unwrap();
            }
            _ => unreachable!(),
        }

        out
    }

    pub fn vec2(x: impl Into<Data>, y: impl Into<Data>) -> Data {
        Data::Vec {
            n: 2,
            data: vec![x.into(), y.into()],
        }
    }
}

impl From<bool> for Data {
    fn from(x: bool) -> Self {
        Data::Bool(x)
    }
}

impl From<i32> for Data {
    fn from(x: i32) -> Self {
        Data::I32(x)
    }
}

impl From<u32> for Data {
    fn from(x: u32) -> Self {
        Data::U32(x)
    }
}

impl From<f32> for Data {
    fn from(x: f32) -> Self {
        Data::F32(x)
    }
}
