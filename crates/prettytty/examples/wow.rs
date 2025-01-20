use prettytty::{
    cmd::{Format, ResetStyle, SetForeground8},
    fuse_sgr, Sgr,
};

fn main() {
    println!(
        "\n    {}┏━━━━━━┓\n    ┃ Wow! ┃\n    ┗━━━━━━┛{}\n",
        fuse_sgr!(Format::Bold, SetForeground8::<202>),
        ResetStyle
    );
}
