// `Map` is sealed: a custom map whose lookups lie cannot mint evidence.
struct Liar;

impl gdp::justified::Map for Liar {
    type Key = u32;
    type Value = &'static str;

    fn entries(&self) -> impl Iterator<Item = (&u32, &&'static str)> {
        [(&999, &"not in the map")].into_iter()
    }

    fn size(&self) -> usize {
        0
    }
}

fn main() {}
