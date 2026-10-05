// Names cannot leave their scope.
fn main() {
    let _leaked = gdp::name(1u32, |n| n);
}
