use crate::value::serde::Serde;
use crate::value::List;
use glam::DVec2;
use std::fmt::Debug;

fn test_serde<T: Serde + PartialEq + Debug>(name: &'static str, value: &T) {
    println!("Running test {name}");
    let mut buffer = Vec::new();
    value.serialize_to(&mut buffer);
    let mut start = 0;
    let new_value = T::deserialize_from(&mut start, &buffer);
    if start != buffer.len() {
        panic!("Didn't consume all in case {value:?}");
    }
    if &new_value != value {
        panic!("Wrong value: {new_value:?}, should be {value:?}");
    }
}

fn test_serde_all<T: Serde + PartialEq + Debug>(
    values: impl IntoIterator<Item = (&'static str, T)>,
) {
    println!("Starting tests.");
    for (name, value) in values {
        test_serde(name, &value);
    }
    println!();
}

#[derive(Debug)]
struct TotalF64(f64);
impl PartialEq for TotalF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 || (self.0.is_nan() && other.0.is_nan())
    }
}
impl Serde for TotalF64 {
    fn serialize_to(&self, data: &mut Vec<u8>) {
        self.0.serialize_to(data);
    }
    fn deserialize_from(at: &mut usize, data: &[u8]) -> Self {
        TotalF64(f64::deserialize_from(at, data))
    }
}

macro_rules! list_with_name {
    ($($expr: expr),* $(,)?) => {[$((stringify!($expr),$expr)),*]};
}

#[test]
fn number_serde() {
    test_serde_all(list_with_name![
        0.1,
        0.0,
        -0.1,
        -3.7,
        f64::INFINITY,
        f64::NEG_INFINITY
    ]);
}

#[test]
fn point_serde() {
    test_serde_all(list_with_name![
        DVec2::ZERO,
        DVec2::ONE,
        DVec2::X,
        DVec2::MAX,
        DVec2::INFINITY,
        DVec2::NEG_ONE,
    ]);
}

#[test]
fn list_serde() {
    test_serde_all(list_with_name![
        List::Term(0.0),
        List::Term(f64::INFINITY),
        List::Flat(Vec::new()),
        List::Flat(vec![1.0, 2.0]),
        List::Flat(vec![0.0; 1000]),
        List::Flat((0..2000).map(|x| x as f64).collect()),
        List::Staggered(vec![List::Term(1.0), List::Flat(vec![4.0, 2.0, 3.0])])
    ]);
}
