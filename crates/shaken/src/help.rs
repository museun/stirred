use crate::Callable;

// TODO track where the command came from
#[derive(Default)]
pub struct HelpRegistry {
    map: Vec<HelpCommand>,
}

impl HelpRegistry {
    pub fn create_from<A, B, C>(callable: &impl Callable<A, B, Out = C>) -> Self
    where
        A: Send + 'static,
        B: Send + 'static,
    {
        callable
            .all_usage_and_help()
            .into_iter()
            .fold(Self::default(), |help, (usage, desc)| {
                if let Some(head) = usage.split(' ').next() {
                    help.with(head, desc, usage)
                } else {
                    help.with(usage, desc, None)
                }
            })
    }

    pub fn with<'a>(
        mut self,
        cmd: &str,
        description: &str,
        usage: impl Into<Option<&'a str>>,
    ) -> Self {
        if cmd.is_empty() {
            return self;
        }

        let usage = usage.into();

        let cmd = HelpCommand {
            cmd: Box::from(cmd),
            usage: usage
                .map(<str>::trim)
                .filter(|c| !c.is_empty())
                .map(Box::from),
            description: Box::from(description),
        };

        self.map.push(cmd);
        self
    }

    pub fn add(&mut self, cmd: &str, description: &str, usage: &str) {
        *self = std::mem::take(self).with(cmd, description, usage)
    }

    pub fn lookup(&self, cmd: &str) -> Option<(&str, &str)> {
        self.map.iter().find_map(|help| {
            if &*help.cmd == cmd {
                return Some((&**help.usage.as_ref()?, &*help.description));
            }
            None
        })
    }

    pub fn get_all_commands(&self) -> impl Iterator<Item = &str> + ExactSizeIterator {
        self.map.iter().map(|HelpCommand { cmd, .. }| &**cmd)
    }
}

struct HelpCommand {
    cmd: Box<str>,
    usage: Option<Box<str>>,
    description: Box<str>,
}
