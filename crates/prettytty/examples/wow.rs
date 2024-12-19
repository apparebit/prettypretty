use prettytty::{
    cmd::{Format, ResetStyle, SetForeground8},
    sgr, Sgr,
};

fn main() {
    println!(
        "\n    {}┏━━━━━━┓\n    ┃ Wow! ┃\n    ┗━━━━━━┛{}\n",
        sgr!(Format::Bold, SetForeground8(202)),
        ResetStyle
    );
}
