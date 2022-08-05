use std::{fs::File, path::Path};

use gumdrop::Options;
use indicatif::{ProgressBar, ProgressStyle};
use markov::Brain;
use snap::write::FrameEncoder;

#[derive(Debug, gumdrop::Options)]
struct Config {
    #[options(help = "print help message")]
    help: bool,

    #[options(
        help = "raw input file to train from",
        short = "i",
        required,
        meta = "path"
    )]
    input: String,

    #[options(
        help = "output path",
        short = "o",
        default = "output.sdb",
        meta = "path"
    )]
    output: String,

    #[options(
        help = "name of the brain",
        short = "n",
        default = "corpora",
        meta = "string"
    )]
    name: String,

    #[options(help = "display a progress bar", short = "p", default = "false")]
    progress: bool,

    #[options(help = "ngram size", short = "d", default = "5", meta = "int")]
    depth: usize,
}

#[derive(Default)]
enum Progress {
    Show(ProgressBar),
    #[default]
    Hide,
}

impl Progress {
    fn show(max: u64) -> Self {
        let template = "{percent:>3}% {wide_bar:.red/white.dim} {pos}/{len}";
        let display = "══━";

        let bar = ProgressBar::new(max);
        let style = ProgressStyle::with_template(template)
            .unwrap()
            .progress_chars(display);
        bar.set_style(style);

        Self::Show(bar)
    }
    fn tick(&self) {
        if let Self::Show(bar) = self {
            bar.inc(1)
        }
    }
    fn done(&self) {
        if let Self::Show(bar) = self {
            bar.finish();
        }
    }
}

fn save(brain: &Brain, path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = path.as_ref();
    let writer = File::create(path)?;
    let mut writer = FrameEncoder::new(writer);
    bincode::serialize_into(&mut writer, brain)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let config = Config::parse_args_default_or_exit();
    let mut brain = Brain::new(&config.name, config.depth);

    let data = std::fs::read_to_string(&config.input)?;
    let max = data.lines().count();

    let bar = config
        .progress
        .then(|| Progress::show(max as _))
        .unwrap_or_default();

    for line in data.lines() {
        bar.tick();
        brain.train(line);
    }
    bar.done();

    save(&brain, &config.output)?;

    Ok(())
}
