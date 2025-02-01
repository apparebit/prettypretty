use prettytty::{
    cmd::{Format, ResetStyle, SetBackground8, SetDefaultBackground, SetForeground8},
    fuse_sgr, Sgr,
};

const WOW: u8 = 124;
const BOX: u8 = 202;
const BG: u8 = 230;

fn main() {
    println!(
        "\n    {}┏━━━━━━┓{}\n    {}┃{} Wow! {}┃{}\n    {}┗━━━━━━┛{}\n",
        fuse_sgr!(SetBackground8::<BG>, SetForeground8::<BOX>),
        SetDefaultBackground,
        SetBackground8::<BG>,
        fuse_sgr!(SetForeground8::<WOW>, Format::Bold),
        fuse_sgr!(SetForeground8::<BOX>, Format::Regular),
        SetDefaultBackground,
        SetBackground8::<BG>,
        ResetStyle
    );
}
