
pub struct Argument {
    pub name: String,
    pub value: Option<String>,
}

pub struct ArgumentsParser {
    arguments: Vec<Argument>,
}

impl ArgumentsParser {
    pub fn new(args: Vec<String>) -> ArgumentsParser {
        let mut parser = ArgumentsParser {
            arguments: Vec::new(),
        };

        parser.parse(args);

        parser
    }

    fn parse(&mut self, args: Vec<String>) {
        let mut current_argument: Option<Argument> = None;

        for arg in args {
            if arg.starts_with("--") {
                if let Some(argument) = current_argument {
                    self.arguments.push(argument);
                }

                current_argument = Some(Argument {
                    name: arg[2..].to_string(),
                    value: None,
                });
            } else {
                if let Some(argument) = &mut current_argument {
                    argument.value = Some(arg);
                }
            }
        }

        if let Some(argument) = current_argument {
            self.arguments.push(argument);
        }
    }

    pub fn has_argument(&self, name: &str) -> bool {
        for argument in &self.arguments {
            if argument.name == name {
                return true;
            }
        }
        false
    }

    pub fn get_value(&self, name: &str) -> Option<String> {
        for argument in &self.arguments {
            if argument.name == name {
                return argument.value.clone();
            }
        }
        None
    }

    pub fn for_each_argument<F>(&self, mut f: F)
    where
        F: FnMut(&Argument),
    {
        for argument in &self.arguments {
            f(argument);
        }
    }
}
