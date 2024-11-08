#[cfg(feature = "server")]
pub mod ops;
mod serde;

use glam::DVec2;
use std::fmt::Display;

pub use serde::Serde;

#[derive(Debug, Clone, Copy)]
pub enum ListRef<'a, T> {
    FlatElem(&'a T),
    FullList(&'a List<T>),
}

impl<'a, T> ListRef<'a, T> {
    fn try_one_elem(self) -> Option<&'a T> {
        match self {
            Self::FlatElem(x) => Some(x),
            Self::FullList(List::Term(x)) => Some(x),
            _ => None,
        }
    }

    fn len(&self) -> Option<usize> {
        match self {
            Self::FlatElem(_) => None,
            Self::FullList(list) => match list {
                List::Term(_) => None,
                List::Flat(xs) => Some(xs.len()),
                List::Staggered(xs) => Some(xs.len()),
            },
        }
    }

    fn get_at(&self, at: usize) -> Self {
        match self {
            Self::FlatElem(x) => Self::FlatElem(x),
            Self::FullList(list) => match list {
                List::Term(x) => Self::FlatElem(x),
                List::Flat(xs) => Self::FlatElem(&xs[at]),
                List::Staggered(xs) => Self::FullList(&xs[at]),
            },
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum List<T> {
    Term(T),
    Flat(Vec<T>),
    Staggered(Vec<List<T>>),
}

impl<T> List<T> {
    fn as_ref(&self) -> ListRef<T> {
        ListRef::FullList(self)
    }

    fn display(&self, ind: usize, displayer: &impl Fn(&T), use_newlines: &impl Fn(&[T]) -> bool) {
        let indent = "    ".repeat(ind);
        match self {
            Self::Term(x) => {
                print!("{indent}");
                displayer(x);
                println!();
            }
            Self::Flat(xs) => {
                if use_newlines(xs) {
                    println!("{indent}[");
                    for x in xs {
                        print!("{indent}    ");
                        displayer(x);
                        println!();
                    }
                    println!("{indent}]");
                } else {
                    print!("{indent}[");
                    if !xs.is_empty() {
                        for x in xs.iter().take(xs.len() - 1) {
                            displayer(x);
                            print!(" ");
                        }
                        displayer(&xs[xs.len() - 1]);
                    }
                    println!("]");
                }
            }
            Self::Staggered(xs) => {
                println!("{indent}[");
                for x in xs {
                    x.display(ind + 1, displayer, use_newlines);
                }
                println!("{indent}]");
            }
        }
    }

    pub fn empty() -> Self {
        Self::Flat(Vec::new())
    }

    pub fn try_term(self) -> Option<T> {
        match self {
            Self::Term(x) => Some(x),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Term(_) => false,
            Self::Flat(xs) => xs.is_empty(),
            Self::Staggered(xs) => xs.is_empty(),
        }
    }

    pub fn list(items: Vec<Self>) -> Self {
        if items.iter().all(|x| matches!(x, List::Term(_))) {
            Self::Flat(
                items
                    .into_iter()
                    .map(|x| {
                        let List::Term(x) = x else { unreachable!() };
                        x
                    })
                    .collect(),
            )
        } else {
            Self::Staggered(items)
        }
    }

    pub fn fix(item: Self) -> Self {
        match item {
            Self::Term(x) => Self::Flat(vec![x]),
            other => Self::Staggered(vec![other]),
        }
    }

    pub fn push(&mut self, item: Self) {
        match self {
            Self::Term(_) => todo!("decide what happens when pushing items to a terminal"),
            Self::Flat(xs) => match item {
                Self::Term(x) => xs.push(x),
                other => take_mut::take(self, |slf| {
                    let Self::Flat(xs) = slf else { unreachable!() };
                    let mut new_vals: Vec<_> = xs.into_iter().map(Self::Term).collect();
                    new_vals.push(other);
                    Self::Staggered(new_vals)
                }),
            },
            Self::Staggered(xs) => xs.push(item),
        }
    }
}

macro_rules! value_enum {
    (
        $($name: ident => $type: ident (one_name: $one_name: ident, try_name: $try_name: ident, str_name: $str_name: literal))*
    ) =>{
        #[non_exhaustive]
        #[derive(Debug, Clone, PartialEq)]
        pub enum Value {
            $($name(List<$type>)),*
        }

        #[non_exhaustive]
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub enum ValueKind {
            $($name),*
        }

        #[non_exhaustive]
        #[cfg(feature = "server")]
        #[derive(Debug, Clone, Copy)]
        pub enum ValueRef<'a> {
            $($name(ListRef<'a, $type>)),*
        }

        #[non_exhaustive]
        #[cfg(feature = "server")]
        #[derive(Debug, Clone, Copy)]
        pub enum OneRef<'a> {
            $($name(&'a $type)),*
        }

        impl Value {
            pub fn fix(self) -> Self {
                match self { $(Self::$name(xs) => Self::$name(List::fix(xs))),* }
            }
            pub fn is_empty(&self) -> bool {
                match self { $(Self::$name(xs) => xs.is_empty()),* }
            }

            pub fn push(&mut self, item: Self) -> Result<(), TypeMismatch> {
                if self.is_empty() {
                    *self = item.fix();
                } else {
                    match (self, item) {
                        $((Self::$name(xs), Self::$name(ys)) => xs.push(ys),)*
                        (left, right) => {
                            return Err(TypeMismatch {
                                expect: left.kind(),
                                got: right.kind(),
                            })
                        }
                    }
                }

                Ok(())
            }
            pub const fn kind(&self) -> ValueKind {
                match self { $(Self::$name(_) => ValueKind::$name),* }
            }
        }

        #[cfg(feature = "server")]
        impl Value {
            $(pub fn $one_name(value: $type) -> Self {
                Self::$name(List::Term(value))
            })*

            $(pub fn $try_name(self) -> Result<List<$type>, TypeMismatch> {
                match self {
                    Self::$name(x) => Ok(x),
                    other => Err(TypeMismatch {
                        expect: ValueKind::$name,
                        got: other.kind(),
                    }),
                }
            })*

            pub fn as_ref(&self) -> ValueRef {
                match self { $(Self::$name(x) => ValueRef::$name(x.as_ref())),* }
            }
        }


        impl ValueKind {
            $(pub fn $try_name(self) -> Result<(), TypeMismatch> {
                match self {
                    Self::$name => Ok(()),
                    other => Err(TypeMismatch {
                        expect: ValueKind::$name,
                        got: other,
                    }),
                }
            })*

            pub const fn name(&self) -> &'static str {
                match self {
                    $(Self::$name => $str_name),*
                }
            }
        }

        #[cfg(feature = "server")]
        impl<'a> ValueRef<'a> {
            $(pub fn $try_name(self) -> Result<ListRef<'a, $type>, TypeMismatch> {
                match self {
                    Self::$name(x) => Ok(x),
                    other => Err(TypeMismatch {
                        expect: ValueKind::$name,
                        got: other.kind(),
                    }),
                }
            })*

            pub const fn kind(&self) -> ValueKind {
                match self { $(Self::$name(_) => ValueKind::$name),* }
            }

            #[allow(clippy::len_without_is_empty)]
            pub fn len(&self) -> Option<usize> {
                match self {
                    $(Self::$name(x) => x.len()),*
                }
            }

            pub fn get_at(&self, at: usize) -> Self {
                match self {
                    $(Self::$name(x) => Self::$name(x.get_at(at))),*
                }
            }

            pub fn try_one_elem(self) -> Option<OneRef<'a>> {
                Some(match self {
                    $(Self::$name(x) => OneRef::$name(x.try_one_elem()?)),*
                })
            }
        }

        #[cfg(feature = "server")]
        impl<'a> OneRef<'a> {
            $(pub fn $try_name(self) -> Result<&'a $type, TypeMismatch> {
                match self {
                    Self::$name(x) => Ok(x),
                    other => Err(TypeMismatch {
                        expect: ValueKind::$name,
                        got: other.kind(),
                    }),
                }
            })*

            pub const fn kind(&self) -> ValueKind {
                match self { $(Self::$name(_) => ValueKind::$name),* }
            }

            pub fn to_value(self) -> Value {
                match self {
                    $(Self::$name(&x) => Value::$name(List::Term(x))),*
                }
            }
        }
    };
}

value_enum! {
    Number => f64 (
        one_name: one_number,
        try_name: try_number,
        str_name: "number"
    )
    Point => DVec2 (
        one_name: one_point,
        try_name: try_point,
        str_name: "point"
    )
}

impl Display for ValueKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name().fmt(f)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TypeMismatch {
    pub expect: ValueKind,
    pub got: ValueKind,
}

impl Value {
    pub fn empty() -> Self {
        Self::Number(List::empty())
    }

    pub fn display(&self) {
        match self {
            Self::Number(xs) => xs.display(0, &|x| print!("{x}"), &|xs| {
                let is_integral = xs.iter().all(|x| x.fract() == 0.0);
                match is_integral {
                    true => xs.len() > 20,
                    false => xs.len() > 5,
                }
            }),
            Self::Point(xs) => xs.display(0, &|p| print!("({} {})", p.x, p.y), &|xs| xs.len() > 8),
        }
    }

    pub fn list(items: Vec<Self>) -> Result<Self, TypeMismatch> {
        let first = items.first();
        Ok(match first {
            None | Some(Value::Number(_)) => Self::Number(List::list(
                items
                    .into_iter()
                    .map(|x| -> Result<_, TypeMismatch> {
                        let Value::Number(x) = x else {
                            return Err(TypeMismatch {
                                expect: ValueKind::Number,
                                got: x.kind(),
                            });
                        };
                        Ok(x)
                    })
                    .collect::<Result<Vec<_>, TypeMismatch>>()?,
            )),
            Some(Value::Point(_)) => Self::Point(List::list(
                items
                    .into_iter()
                    .map(|x| -> Result<_, TypeMismatch> {
                        let Value::Point(x) = x else {
                            return Err(TypeMismatch {
                                expect: ValueKind::Point,
                                got: x.kind(),
                            });
                        };
                        Ok(x)
                    })
                    .collect::<Result<Vec<_>, TypeMismatch>>()?,
            )),
        })
    }
}
