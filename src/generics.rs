use core::{fmt::Formatter, num};

use nom::{
    multi::{count, many1},
    number,
};

use crate::*;
pub trait Mangle {
    fn demangle(a: &str) -> IResult<&str, Self>
    where
        Self: Sized;
    fn mangle(&self, f: &mut Formatter) -> core::fmt::Result;
}
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Mangled<'a>(pub &'a (dyn Mangle + 'a));
impl<'a> Display for Mangled<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.mangle(f)
    }
}
impl Mangle for Arity {
    fn demangle(a: &str) -> IResult<&str, Self>
    where
        Self: Sized,
    {
        let (a, b) = tag(";")
            .and_then(alphanumeric1.map_opt(|a: &str| a.parse::<usize>().ok()))
            .parse(a)?;
        let (a, m) = many1((
            tag("P").and_then(alphanumeric1),
            tag(";").and_then(alphanumeric1.map_opt(|a: &str| a.parse::<usize>().ok())),
        ))
        .parse(a)?;
        let mut stack = vec![];
        for (i, j) in m.into_iter().rev() {
            let m = Arity {
                to_fill: (0..j).filter_map(|a| stack.pop()).collect(),
            };
            stack.push((i.to_owned(), m));
        }
        let p = Arity {
            to_fill: (0..b).filter_map(|a| stack.pop()).collect(),
        };
        Ok((a, p))
    }

    fn mangle(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, ";{}", self.to_fill.len())?;
        for (a, b) in &self.to_fill {
            write!(f, "P{a}{}", Mangled(b))?;
        }
        Ok(())
    }
}
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum Param {
    Attr(Attr),
    Interface {
        rid: [u8; 32],
        params: BTreeMap<String, Param>,
    },
}
impl Mangle for Param {
    fn demangle(a: &str) -> IResult<&str, Self>
    where
        Self: Sized,
    {
        fn parse_nonattr(a: &str) -> IResult<&str, Param> {
            let (a, b) = (
                tag("R").and_then(take_while_m_n(64, 64, |a: char| a.is_digit(16)).map(|a| {
                    let mut b = [0u8; 32];
                    hex::decode_to_slice(a, &mut b).unwrap();
                    b
                })),
                tag(";").and_then(alphanumeric1.map_opt(|a: &str| a.parse::<usize>().ok())),
            )
                .parse(a)?;

            let (a, params) = count(
                (
                    tag(";").and_then(alphanumeric1),
                    tag(";").and_then(Param::demangle),
                ),
                b.1,
            )
            .parse(a)?;

            Ok((
                a,
                Param::Interface {
                    rid: b.0,
                    params: params.into_iter().map(|(a, b)| (a.to_owned(), b)).collect(),
                },
            ))
        }
        return parse_attr.map(Param::Attr).or(parse_nonattr).parse(a);
    }

    fn mangle(&self, f: &mut Formatter) -> core::fmt::Result {
        match self {
            Param::Attr(attr) => write!(f, "{attr}"),
            Param::Interface { rid, params } => {
                write!(f, "R{};{}", hex::encode(rid), params.len())?;
                for (a, b) in params.iter() {
                    write!(f, ";{a};{}",Mangled(b))?;
                }
                Ok(())
            }
        }
    }
}
