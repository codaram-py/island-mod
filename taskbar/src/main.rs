mod appbar;

fn main() {
    appbar::install_safety_net();
    appbar::recover_from_crash();
}
