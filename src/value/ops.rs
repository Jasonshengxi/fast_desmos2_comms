use std::ops::{Add, Div, Mul, Neg, Sub};

use glam::DVec2;

use super::{List, OneRef, TypeMismatch, Value, ValueRef};

pub enum CrossIterError {
    TypeMismatch(TypeMismatch),
    TooLong,
}

pub fn try_cross_iter_many<E>(
    values: Vec<ValueRef>,
    mut func: &mut impl FnMut(Vec<OneRef>) -> Result<Value, E>,
    error_handling: &impl Fn(CrossIterError) -> E,
) -> Result<Value, E> {
    let lengths: Vec<_> = values.iter().map(|val| val.len()).collect();

    if lengths.iter().any(Option::is_some) {
        const CROSS_LIMIT: usize = 1_000_000_000;

        let true_lengths: Vec<_> = lengths.into_iter().map(|x| x.unwrap_or(1)).collect();

        let mut acc = 1usize;
        let mut scanned = Vec::with_capacity(true_lengths.len());
        for &x in true_lengths.iter() {
            let new_acc = acc
                .checked_mul(x)
                .and_then(|x| (x < CROSS_LIMIT).then_some(x))
                .ok_or(CrossIterError::TooLong)
                .map_err(error_handling)?;
            scanned.push(acc);
            acc = new_acc;
        }
        let max = acc;
        let zipped: Vec<_> = true_lengths.into_iter().zip(scanned).collect();

        let mut result = Value::Number(List::empty());
        for index in 0..max {
            let new_values = zipped
                .iter()
                .zip(values.iter())
                .map(|(&(m, d), val)| val.get_at((index / d) % m))
                .collect();

            let value = try_cross_iter_many(new_values, func, error_handling)?;
            result
                .push(value)
                .map_err(CrossIterError::TypeMismatch)
                .map_err(error_handling)?;
        }

        Ok(result)
    } else {
        func(
            values
                .into_iter()
                .map(|x| x.try_one_elem().unwrap_or_else(|| unreachable!()))
                .collect(),
        )
    }
}

pub fn try_iter_many_known<const N: usize, E>(
    values: [ValueRef; N],
    mut func: &mut impl FnMut([OneRef; N]) -> Result<Value, E>,
    error_handling: &impl Fn(TypeMismatch) -> E,
) -> Result<Value, E> {
    let max_len = values.iter().filter_map(|val| val.len()).reduce(usize::max);

    if let Some(len) = max_len {
        let mut result = Value::Number(List::empty());

        for index in 0..len {
            let new_values = values.map(|val| val.get_at(index));
            let value = try_iter_many_known(new_values, func, error_handling)?;
            result.push(value).map_err(error_handling)?;
        }

        Ok(result)
    } else {
        func(values.map(|x| x.try_one_elem().unwrap_or_else(|| unreachable!())))
    }
}

pub fn try_iter_many<E>(
    values: Vec<ValueRef>,
    mut func: &mut impl FnMut(Vec<OneRef>) -> Result<Value, E>,
    error_handling: &impl Fn(TypeMismatch) -> E,
) -> Result<Value, E> {
    let max_len = values.iter().filter_map(|val| val.len()).reduce(usize::max);

    if let Some(len) = max_len {
        let mut result = Value::Number(List::empty());

        for index in 0..len {
            let new_values = values.iter().map(|val| val.get_at(index)).collect();
            let value = try_iter_many(new_values, func, error_handling)?;
            result.push(value).map_err(error_handling)?;
        }

        Ok(result)
    } else {
        func(
            values
                .into_iter()
                .map(|x| x.try_one_elem().unwrap_or_else(|| unreachable!()))
                .collect(),
        )
    }
}

pub fn try_iter_alone_left<A: Copy, B: Copy, O, E>(
    lhs: List<A>,
    rhs: B,
    func: &impl Fn(A, B) -> Result<O, E>,
) -> Result<List<O>, E> {
    match lhs {
        List::Term(x) => func(x, rhs).map(List::Term),
        List::Flat(xs) => xs
            .into_iter()
            .map(|x| func(x, rhs))
            .collect::<Result<_, _>>()
            .map(List::Flat),
        List::Staggered(xs) => xs
            .into_iter()
            .map(|x| try_iter_alone_left(x, rhs, func))
            .collect::<Result<_, _>>()
            .map(List::Staggered),
    }
}

pub fn try_iter_alone_right<A: Copy, B: Copy, O, E>(
    lhs: A,
    rhs: List<B>,
    func: &impl Fn(A, B) -> Result<O, E>,
) -> Result<List<O>, E> {
    match rhs {
        List::Term(y) => func(lhs, y).map(List::Term),
        List::Flat(ys) => ys
            .into_iter()
            .map(|y| func(lhs, y))
            .collect::<Result<_, _>>()
            .map(List::Flat),
        List::Staggered(ys) => ys
            .into_iter()
            .map(|y| try_iter_alone_right(lhs, y, func))
            .collect::<Result<_, _>>()
            .map(List::Staggered),
    }
}

pub fn try_iter_vec_left<A: Copy, B: Copy, O, E>(
    lhs: List<A>,
    rhs: Vec<B>,
    func: &impl Fn(A, B) -> Result<O, E>,
) -> Result<List<O>, E> {
    match lhs {
        List::Term(x) => rhs
            .into_iter()
            .map(|y| func(x, y))
            .collect::<Result<_, _>>()
            .map(List::Flat),
        List::Flat(xs) => xs
            .into_iter()
            .zip(rhs)
            .map(|(x, y)| func(x, y))
            .collect::<Result<_, _>>()
            .map(List::Flat),
        List::Staggered(xs) => xs
            .into_iter()
            .zip(rhs)
            .map(|(x, y)| try_iter_alone_left(x, y, func))
            .collect::<Result<_, _>>()
            .map(List::Staggered),
    }
}

pub fn try_iter_vec_right<A: Copy, B: Copy, O, E>(
    lhs: Vec<A>,
    rhs: List<B>,
    func: &impl Fn(A, B) -> Result<O, E>,
) -> Result<List<O>, E> {
    match rhs {
        List::Term(y) => lhs
            .into_iter()
            .map(|x| func(x, y))
            .collect::<Result<_, _>>()
            .map(List::Flat),
        List::Flat(ys) => lhs
            .into_iter()
            .zip(ys)
            .map(|(x, y)| func(x, y))
            .collect::<Result<_, _>>()
            .map(List::Flat),
        List::Staggered(ys) => lhs
            .into_iter()
            .zip(ys)
            .map(|(x, y)| try_iter_alone_right(x, y, func))
            .collect::<Result<_, _>>()
            .map(List::Staggered),
    }
}

pub fn try_iter_full<A: Copy, B: Copy, O, E>(
    lhs: List<A>,
    rhs: List<B>,
    func: &impl Fn(A, B) -> Result<O, E>,
) -> Result<List<O>, E> {
    match (lhs, rhs) {
        (List::Term(x), y) => try_iter_alone_right(x, y, func),
        (x, List::Term(y)) => try_iter_alone_left(x, y, func),
        (List::Flat(x), y) => try_iter_vec_right(x, y, func),
        (x, List::Flat(y)) => try_iter_vec_left(x, y, func),
        (List::Staggered(x), List::Staggered(y)) => x
            .into_iter()
            .zip(y)
            .map(|(x, y)| try_iter_full(x, y, func))
            .collect::<Result<_, _>>()
            .map(List::Staggered),
    }
}
pub fn iter_alone_left<A: Copy, B: Copy, O>(
    lhs: List<A>,
    rhs: B,
    func: &impl Fn(A, B) -> O,
) -> List<O> {
    match lhs {
        List::Term(x) => List::Term(func(x, rhs)),
        List::Flat(xs) => List::Flat(xs.into_iter().map(|x| func(x, rhs)).collect()),
        List::Staggered(xs) => List::Staggered(
            xs.into_iter()
                .map(|x| iter_alone_left(x, rhs, func))
                .collect(),
        ),
    }
}

pub fn iter_alone_right<A: Copy, B: Copy, O>(
    lhs: A,
    rhs: List<B>,
    func: &impl Fn(A, B) -> O,
) -> List<O> {
    match rhs {
        List::Term(y) => List::Term(func(lhs, y)),
        List::Flat(ys) => List::Flat(ys.into_iter().map(|y| func(lhs, y)).collect()),
        List::Staggered(ys) => List::Staggered(
            ys.into_iter()
                .map(|y| iter_alone_right(lhs, y, func))
                .collect(),
        ),
    }
}

pub fn iter_vec_left<A: Copy, B: Copy, O>(
    lhs: List<A>,
    rhs: Vec<B>,
    func: &impl Fn(A, B) -> O,
) -> List<O> {
    match lhs {
        List::Term(x) => List::Flat(rhs.into_iter().map(|y| func(x, y)).collect()),
        List::Flat(xs) => List::Flat(xs.into_iter().zip(rhs).map(|(x, y)| func(x, y)).collect()),
        List::Staggered(xs) => List::Staggered(
            xs.into_iter()
                .zip(rhs)
                .map(|(x, y)| iter_alone_left(x, y, func))
                .collect(),
        ),
    }
}

pub fn iter_vec_right<A: Copy, B: Copy, O>(
    lhs: Vec<A>,
    rhs: List<B>,
    func: &impl Fn(A, B) -> O,
) -> List<O> {
    match rhs {
        List::Term(y) => List::Flat(lhs.into_iter().map(|x| func(x, y)).collect()),
        List::Flat(ys) => List::Flat(lhs.into_iter().zip(ys).map(|(x, y)| func(x, y)).collect()),
        List::Staggered(ys) => List::Staggered(
            lhs.into_iter()
                .zip(ys)
                .map(|(x, y)| iter_alone_right(x, y, func))
                .collect(),
        ),
    }
}

pub fn iter_full<A: Copy, B: Copy, O>(
    lhs: List<A>,
    rhs: List<B>,
    func: &impl Fn(A, B) -> O,
) -> List<O> {
    match (lhs, rhs) {
        (List::Term(x), y) => iter_alone_right(x, y, func),
        (x, List::Term(y)) => iter_alone_left(x, y, func),
        (List::Flat(x), y) => iter_vec_right(x, y, func),
        (x, List::Flat(y)) => iter_vec_left(x, y, func),
        (List::Staggered(x), List::Staggered(y)) => List::Staggered(
            x.into_iter()
                .zip(y)
                .map(|(x, y)| iter_full(x, y, func))
                .collect(),
        ),
    }
}

macro_rules! binary_op_list {
    (
        $first: ident, $second: ident => $result: ident;
        $(impl ($tr: ident, $func: ident, $op: tt))*
    ) => {$(
        impl $tr<List<$second>> for List<$first> {
            type Output = List<$result>;
            fn $func(self, rhs: List<$second>) -> Self::Output {
                iter_full(self, rhs, &|x, y| x $op y)
            }
        }
    )*}
}

macro_rules! binary_op_value1{
    (
        $(impl ($tr: ident, $func: ident, $op: tt))*
    ) => {$(
        impl $tr for Value {
            type Output = Result<Self, TypeMismatch>;
            fn $func(self, rhs: Self) -> Self::Output {
                match (self, rhs) {
                    (Value::Number(x), Value::Number(y)) => Ok(Value::Number(x $op y)),
                    (Value::Point(x), Value::Point(y)) => Ok(Value::Point(x $op y)),
                    (left, right) => Err(TypeMismatch {
                        expect: left.kind(),
                        got: right.kind(),
                    }),
                }
            }
        }
    )*};
}

macro_rules! binary_op_value2{
    (
        $(impl ($tr: ident, $func: ident, $op: tt))*
    ) => {$(
        impl $tr for Value {
            type Output = Result<Self, TypeMismatch>;
            fn $func(self, rhs: Self) -> Self::Output {
                match (self, rhs) {
                    (Value::Number(x), Value::Number(y)) => Ok(Value::Number(x $op y)),
                    (Value::Point(x), Value::Number(y)) => Ok(Value::Point(x $op y)),
                    (Value::Number(x), Value::Point(y)) => Ok(Value::Point(x $op y)),
                    (left, right) => Err(TypeMismatch {
                        expect: left.kind(),
                        got: right.kind(),
                    }),
                }
            }
        }
    )*};
}

binary_op_list! {
    f64, f64 => f64;
    impl (Add, add, +)
    impl (Sub, sub, -)
    impl (Mul, mul, *)
    impl (Div, div, /)
}

binary_op_list! {
    DVec2, DVec2 => DVec2;
    impl (Add, add, +)
    impl (Sub, sub, -)
}

binary_op_list! {
    DVec2, f64 => DVec2;
    impl (Mul, mul, *)
    impl (Div, div, /)
}

binary_op_list! {
    f64, DVec2 => DVec2;
    impl (Mul, mul, *)
    impl (Div, div, /)
}

binary_op_value1! {
    impl (Add, add, +)
    impl (Sub, sub, -)
}

binary_op_value2! {
    impl (Mul, mul, *)
    impl (Div, div, /)
}

impl<T> List<T> {
    pub fn map<U>(self, func: &impl Fn(T) -> U) -> List<U> {
        match self {
            Self::Term(x) => List::Term(func(x)),
            Self::Flat(xs) => List::Flat(xs.into_iter().map(func).collect()),
            Self::Staggered(xs) => List::Staggered(xs.into_iter().map(|x| x.map(func)).collect()),
        }
    }

    pub fn try_map<O, E>(self, func: &impl Fn(T) -> Result<O, E>) -> Result<List<O>, E> {
        match self {
            Self::Term(x) => func(x).map(List::Term),
            Self::Flat(xs) => xs
                .into_iter()
                .map(func)
                .collect::<Result<_, _>>()
                .map(List::Flat),
            Self::Staggered(xs) => xs
                .into_iter()
                .map(|x| x.try_map(func))
                .collect::<Result<_, _>>()
                .map(List::Staggered),
        }
    }
}

impl<T> List<List<T>> {
    pub fn flatten(self) -> List<T> {
        match self {
            List::Term(x) => x,
            List::Flat(xs) => List::Staggered(xs),
            List::Staggered(xs) => List::Staggered(xs.into_iter().map(List::flatten).collect()),
        }
    }
}

impl List<Value> {
    pub fn flatten_value(self) -> Result<Value, TypeMismatch> {
        match self {
            List::Term(x) => Ok(x),
            List::Flat(xs) => Value::list(xs),
            List::Staggered(xs) => Value::list(
                xs.into_iter()
                    .map(List::flatten_value)
                    .collect::<Result<_, _>>()?,
            ),
        }
    }
}

impl<T: Neg<Output = U>, U> Neg for List<T> {
    type Output = List<U>;

    fn neg(self) -> Self::Output {
        self.map(&Neg::neg)
    }
}

impl Neg for Value {
    type Output = Option<Self>;
    fn neg(self) -> Self::Output {
        Some(match self {
            Self::Number(xs) => Self::Number(-xs),
            Self::Point(xs) => Self::Point(-xs),
        })
    }
}
