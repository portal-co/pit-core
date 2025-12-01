#![no_std]
extern crate alloc;
/// Core library for portal interface types, PIT
///
/// Provides generic utilities, parsing, and data structures for portal-co projects.
/// This crate is `no_std` and uses `alloc` for heap-allocated types.
use alloc::{
    borrow::ToOwned,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use base64::Engine;
use core::fmt::{self, Display};
use core::{convert::identity as tuple, fmt::Formatter};
use derive_more::Display;
use nom::{
    AsChar, IResult, Input, Parser,
    bytes::complete::{is_not, tag, take, take_while_m_n},
    character::complete::{alpha1, char, multispace0, none_of, space0},
    combinator::opt,
    error::Error,
    multi::{many0, separated_list0},
    sequence::delimited,
};
use sha3::{Digest, Sha3_256};
#[path = "generics.rs"]
mod _generics;
#[path = "pcode.rs"]
mod _pcode;

/// Unstable module for pcode-related functionality.
#[instability::unstable(feature = "pcode")]
pub mod pcode {
    pub use crate::_pcode::*;
}

/// Unstable module for generics-related functionality.
#[instability::unstable(feature = "generics")]
pub mod generics {
    pub use crate::_generics::*;
}

use crate::util::WriteUpdate;
/// Utility functions and types.
pub mod util;
/// Parses an identifier from a string slice.
///
/// Identifiers may contain alphanumeric characters, '_', '$', and '.'.
/// Returns a tuple of the remaining input and the parsed identifier.
pub fn ident(a: &str) -> IResult<&str, &str> {
    return a.split_at_position1_complete(
        |a| !a.is_alphanum() && !(['_', '$', '.'].into_iter().any(|x| x == a)),
        nom::error::ErrorKind::AlphaNumeric,
    );
}
/// Attribute key-value pair.
/// Represents a key-value attribute, used for metadata and annotations throughout the interface system.
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct Attr {
    /// The attribute name.
    pub name: String,
    /// The attribute value.
    pub value: String,
}

/// Represents the arity (number and structure of parameters) for generics.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Arity {
    pub to_fill: BTreeMap<String, Arity>,
}
/// Display implementation for Arity, formats as a generic parameter list.
impl Display for Arity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<")?;
        for (a, b) in &self.to_fill {
            write!(f, "{a} {b}")?;
        }
        write!(f, ">")?;
        Ok(())
    }
}
impl Arity {
    pub fn is_simple(&self, depth: usize) -> bool {
        if depth == 0 {
            false
        } else {
            self.to_fill.values().all(|a| a.is_simple(depth - 1))
        }
    }
    /// Parses a generic parameter arity from a string.
    ///
    /// Returns a tuple of the remaining input and the parsed `Arity`.
    pub fn parse(a: &str) -> IResult<&str, Self> {
        let (a, c) = space0
            .and_then(delimited(
                tag("<"),
                many0(space0.and_then((ident, space0.and_then(Arity::parse)))),
                tag(">"),
            ))
            .parse(a)?;
        return Ok((
            a,
            Arity {
                to_fill: c.into_iter().map(|(a, b)| (a.to_owned(), b)).collect(),
            },
        ));
    }
}

impl Attr {
    pub fn as_wasm_abi(&self) -> Option<usize> {
        self.as_ver("wasmAbiVer")
    }
    pub fn from_wasm_abi(ver: usize) -> Option<Self> {
        Self::from_ver(ver, "wasmAbiVer")
    }
    pub fn as_ver(&self, a: &str) -> Option<usize> {
        if self.name == a {
            Some(usize::from_str_radix(&self.value, 16).ok()? + 1)
        } else {
            None
        }
    }
    pub fn from_ver(ver: usize, a: &str) -> Option<Self> {
        if ver == 0 {
            None
        } else {
            let ver = ver - 1;
            Some(Self {
                name: a.to_owned(),
                value: format!("{ver:x}"),
            })
        }
    }

    // Documentation attribute accessors (feature-gated)

    /// Returns the value if this is a `name` attribute (human-readable display name).
    #[cfg(feature = "doc-attrs")]
    pub fn as_name(&self) -> Option<&str> {
        if self.name == "name" {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Creates a `name` attribute with the given display name.
    #[cfg(feature = "doc-attrs")]
    pub fn from_name(name: impl Into<String>) -> Self {
        Self {
            name: "name".to_owned(),
            value: name.into(),
        }
    }

    /// Returns the value if this is a `doc` attribute (full documentation text).
    #[cfg(feature = "doc-attrs")]
    pub fn as_doc(&self) -> Option<&str> {
        if self.name == "doc" {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Creates a `doc` attribute with the given documentation text.
    #[cfg(feature = "doc-attrs")]
    pub fn from_doc(doc: impl Into<String>) -> Self {
        Self {
            name: "doc".to_owned(),
            value: doc.into(),
        }
    }

    /// Returns the value if this is a `brief` attribute (short one-line summary).
    #[cfg(feature = "doc-attrs")]
    pub fn as_brief(&self) -> Option<&str> {
        if self.name == "brief" {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Creates a `brief` attribute with the given summary text.
    #[cfg(feature = "doc-attrs")]
    pub fn from_brief(brief: impl Into<String>) -> Self {
        Self {
            name: "brief".to_owned(),
            value: brief.into(),
        }
    }

    /// Returns the value if this is a `deprecated` attribute.
    #[cfg(feature = "doc-attrs")]
    pub fn as_deprecated(&self) -> Option<&str> {
        if self.name == "deprecated" {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Creates a `deprecated` attribute with the given deprecation message.
    #[cfg(feature = "doc-attrs")]
    pub fn from_deprecated(msg: impl Into<String>) -> Self {
        Self {
            name: "deprecated".to_owned(),
            value: msg.into(),
        }
    }

    /// Returns the value if this is an `llm.context` attribute (LLM-readable context).
    #[cfg(feature = "doc-attrs")]
    pub fn as_llm_context(&self) -> Option<&str> {
        if self.name == "llm.context" {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Creates an `llm.context` attribute with the given context text.
    #[cfg(feature = "doc-attrs")]
    pub fn from_llm_context(ctx: impl Into<String>) -> Self {
        Self {
            name: "llm.context".to_owned(),
            value: ctx.into(),
        }
    }

    /// Returns the value if this is an `llm.intent` attribute (LLM-readable intent).
    #[cfg(feature = "doc-attrs")]
    pub fn as_llm_intent(&self) -> Option<&str> {
        if self.name == "llm.intent" {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Creates an `llm.intent` attribute with the given intent description.
    #[cfg(feature = "doc-attrs")]
    pub fn from_llm_intent(intent: impl Into<String>) -> Self {
        Self {
            name: "llm.intent".to_owned(),
            value: intent.into(),
        }
    }

    /// Returns the value if this is a `category` attribute.
    #[cfg(feature = "doc-attrs")]
    pub fn as_category(&self) -> Option<&str> {
        if self.name == "category" {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Creates a `category` attribute with the given category name.
    #[cfg(feature = "doc-attrs")]
    pub fn from_category(cat: impl Into<String>) -> Self {
        Self {
            name: "category".to_owned(),
            value: cat.into(),
        }
    }

    /// Returns the value if this is a `since` attribute (version introduced).
    #[cfg(feature = "doc-attrs")]
    pub fn as_since(&self) -> Option<&str> {
        if self.name == "since" {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Creates a `since` attribute with the given version string.
    #[cfg(feature = "doc-attrs")]
    pub fn from_since(version: impl Into<String>) -> Self {
        Self {
            name: "since".to_owned(),
            value: version.into(),
        }
    }

    /// Returns the value if this attribute matches the given name.
    #[cfg(feature = "doc-attrs")]
    pub fn as_attr(&self, attr_name: &str) -> Option<&str> {
        if self.name == attr_name {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Creates an attribute with the given name and value.
    #[cfg(feature = "doc-attrs")]
    pub fn from_attr(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

pub fn merge(a: Vec<Attr>, b: Vec<Attr>) -> Vec<Attr> {
    let mut m = BTreeMap::new();
    for x in a.into_iter().chain(b) {
        m.insert(x.name, x.value);
    }
    return m
        .into_iter()
        .map(|(a, b)| Attr { name: a, value: b })
        .collect();
}

/// Parses a balanced bracketed string, returning the content inside brackets.
///
/// Returns a tuple of the remaining input and the parsed string.
pub fn parse_balanced(mut a: &str) -> IResult<&str, String> {
    let mut v = Vec::default();
    let mut i = 0;
    loop {
        let (b, x) = nom::character::complete::anychar(a)?;
        match x {
            '[' => i += 1,
            ']' => {
                if i == 0 {
                    return Ok((a, v.into_iter().collect()));
                }
                i -= 1;
            }
            _ => {}
        }
        a = b;
        v.push(x)
    }
}

/// Parses an attribute from a string in the format `[name=value]`.
///
/// Returns a tuple of the remaining input and the parsed `Attr`.
pub fn parse_attr(a: &str) -> IResult<&str, Attr> {
    let (a, _) = multispace0(a)?;
    let (a, _) = char('[')(a)?;
    let (a, _) = multispace0(a)?;
    let (a, name) = many0(none_of("=")).parse(a)?;
    let (a, _) = multispace0(a)?;
    let (a, _) = char('=')(a)?;
    let (a, _) = multispace0(a)?;
    let (a, value) = parse_balanced(a)?;
    let (a, _) = char(']')(a)?;
    let (a, _) = multispace0(a)?;
    return Ok((
        a,
        Attr {
            name: name.into_iter().collect(),
            value,
        },
    ));
}

/// Parses a list of attributes from a string.
///
/// Returns a tuple of the remaining input and a sorted vector of `Attr`.
pub fn parse_attrs(a: &str) -> IResult<&str, Vec<Attr>> {
    let (a, mut b) = many0(parse_attr).parse(a)?;
    b.sort_by_key(|a| a.name.clone());
    Ok((a, b))
}

impl Display for Attr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{}={}]", self.name, self.value)
    }
}

/// Represents a resource type, which may be absent, a specific resource, or a reference to "this".
#[non_exhaustive]
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub enum ResTy {
    /// No resource.
    None,
    /// A resource identified by a 32-byte ID.
    Of([u8; 32]),
    /// The current resource ("this").
    This,
}
impl ResTy {
    /// Renders the resource type to a formatter.
    ///
    /// This is comparable to `Display`, but allows custom formatting based on attributes.
    fn render(
        &self,
        fmt: &mut Formatter,
        gattrs: &(dyn Fn(&str) -> Option<usize> + '_),
    ) -> core::fmt::Result {
        match self {
            ResTy::None => Ok(()),
            ResTy::Of(v) => {
                if gattrs("ridFmtVer").unwrap_or_default() >= 1 {
                    write!(
                        fmt,
                        "~b64{}~",
                        base64::engine::general_purpose::STANDARD_NO_PAD.encode(v)
                    )
                } else {
                    write!(fmt, "{}", hex::encode(v))
                }
            }
            ResTy::This => {
                write!(fmt, "this")
            }
        }
    }
}
/// Parses a resource type from a string.
///
/// Returns a tuple of the remaining input and the parsed `ResTy`.
pub fn parse_resty(a: &str) -> IResult<&str, ResTy> {
    if let Some(a) = a.strip_prefix("this") {
        // let (a, k) = opt(tag("n"))(a)?;
        return Ok((a, ResTy::This));
    }
    if let Some((be, a)) = a.strip_prefix("~b64").and_then(|a| a.split_once("~")) {
        let mut b = [0u8; 32];
        if let Ok(v) = base64::engine::general_purpose::STANDARD_NO_PAD.decode_slice(be, &mut b)
            && v == 32
        {
            return Ok((a, ResTy::Of(b)));
        }
    }
    let (a, d) = opt(take_while_m_n(64, 64, |a: char| a.is_digit(16))).parse(a)?;
    return Ok((
        a,
        match d {
            Some(d) => {
                let mut b = [0u8; 32];
                hex::decode_to_slice(d, &mut b).unwrap();
                ResTy::Of(b)
            }
            None => ResTy::None,
        },
    ));
}
/// Represents an argument type for methods, including primitives and resources.
#[non_exhaustive]
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub enum Arg {
    /// 32-bit integer argument.
    I32,
    /// 64-bit integer argument.
    I64,
    /// 32-bit float argument.
    F32,
    /// 64-bit float argument.
    F64,
    /// Resource argument, with type, nullability, ownership, and annotations.
    Resource {
        /// Resource type.
        ty: ResTy,
        /// Whether the resource is nullable.
        nullable: bool,
        /// Whether the resource is taken (ownership).
        take: bool,
        /// Annotations for the resource.
        ann: Vec<Attr>,
    },
    // Func(Sig),
}
impl Arg {
    /// Renders the argument type to a formatter.
    ///
    /// This is comparable to `Display`, but allows custom formatting based on attributes.
    fn render(
        &self,
        fmt: &mut Formatter,
        gattrs: &(dyn Fn(&str) -> Option<usize> + '_),
    ) -> core::fmt::Result {
        match self {
            Arg::I32 => write!(fmt, "I32"),
            Arg::I64 => write!(fmt, "I64"),
            Arg::F32 => write!(fmt, "F32"),
            Arg::F64 => write!(fmt, "F64"),
            Arg::Resource {
                ty,
                nullable,
                take,
                ann,
            } => {
                write!(
                    fmt,
                    "{}R",
                    ann.iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(""),
                )?;
                ty.render(fmt, gattrs)?;
                write!(
                    fmt,
                    "{}{}",
                    if *nullable { "n" } else { "" },
                    if *take { "" } else { "&" }
                )
            }
        }
    }
}
/// Parses an argument type from a string, including annotations and resource details.
///
/// Returns a tuple of the remaining input and the parsed `Arg`.
pub fn parse_arg(a: &str) -> IResult<&str, Arg> {
    let (a, ann) = parse_attrs(a)?;
    let (a, _) = multispace0(a)?;
    // let (c,b) = take(1usize)(a)?;
    match a.strip_prefix("R") {
        Some(b) => {
            // if let Some(a) = b.strip_prefix("this"){
            //     let (a, k) = opt(tag("n"))(a)?;
            //     return Ok((
            //         a,
            //         Arg::Resource {
            //             ty: ResTy::This,
            //             nullable: k.is_some(),
            //         },
            //     ));
            // }
            let (a, d) = parse_resty(b)?;
            let (a, k) = opt(tag("n")).parse(a)?;
            let (a, take) = opt(tag("&")).parse(a)?;
            return Ok((
                a,
                Arg::Resource {
                    ty: d,
                    nullable: k.is_some(),
                    take: take.is_none(),
                    ann,
                },
            ));
        }
        // "(" => {
        //     let (a, x) = parse_sig(a)?;
        //     return Ok((a, Arg::Func(x)));
        // }
        None => {
            let (a, c) = take(3usize)(a)?;
            match c {
                "I32" => return Ok((a, Arg::I32)),
                "I64" => return Ok((a, Arg::I64)),
                "F32" => return Ok((a, Arg::F32)),
                "F64" => return Ok((a, Arg::F64)),
                _ => return Err(nom::Err::Error(Error::new(a, nom::error::ErrorKind::Tag))),
            }
        }
    }
    todo!()
}

/// Represents a method signature, including annotations, parameters, and return values.
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct Sig {
    /// Annotations for the signature.
    pub ann: Vec<Attr>,
    /// Method parameters.
    pub params: Vec<Arg>,
    /// Method return values.
    pub rets: Vec<Arg>,
}
impl Sig {
    /// Renders the method signature to a formatter.
    ///
    /// This is comparable to `Display`, but allows custom formatting based on attributes.
    fn render(
        &self,
        fmt: &mut Formatter,
        gattrs: &(dyn Fn(&str) -> Option<usize> + '_),
    ) -> core::fmt::Result {
        for a in self.ann.iter() {
            write!(fmt, "{a}")?;
        }
        write!(fmt, "(")?;
        for (i, p) in self.params.iter().enumerate() {
            if i != 0 {
                write!(fmt, ",")?;
            }
            p.render(fmt, gattrs)?;
        }
        write!(fmt, ") -> (")?;
        for (i, p) in self.rets.iter().enumerate() {
            if i != 0 {
                write!(fmt, ",")?;
            }
            p.render(fmt, gattrs)?;
        }
        write!(fmt, ")")
    }
}
/// Parses a method signature from a string, including parameters, return values, and annotations.
///
/// Returns a tuple of the remaining input and the parsed `Sig`.
pub fn parse_sig(a: &str) -> IResult<&str, Sig> {
    let (a, b) = parse_attrs(a)?;
    let (a, _) = multispace0(a)?;
    let mut d = delimited(char('('), separated_list0(char(','), parse_arg), char(')'));
    let (a, params) = d.parse(a)?;
    let (a, _) = multispace0(a)?;
    let (a, _) = tag("->")(a)?;
    let (a, _) = multispace0(a)?;
    let (a, rets) = d.parse(a)?;
    return Ok((
        a,
        Sig {
            params,
            rets,
            ann: b,
        },
    ));
}
/// Represents an interface, containing methods and interface-level annotations.
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct Interface {
    /// Methods in the interface, keyed by name.
    pub methods: BTreeMap<String, Sig>,
    /// Interface-level annotations.
    pub ann: Vec<Attr>,
}
impl Interface {
    /// Renders the interface to a formatter.
    ///
    /// This is comparable to `Display`, but allows custom formatting based on attributes.
    fn render(
        &self,
        f: &mut Formatter,
        gattrs: &(dyn Fn(&str) -> Option<usize> + '_),
    ) -> core::fmt::Result {
        for a in self.ann.iter() {
            write!(f, "{a}")?;
        }
        write!(f, "{}", "{")?;
        let mut x = self.methods.iter().collect::<Vec<_>>();
        x.sort_by_key(|a| a.0);
        for (i, (a, b)) in x.into_iter().enumerate() {
            if i != 0 {
                write!(f, ";")?;
            }
            write!(f, "{}", a)?;
            b.render(f, gattrs)?;
        }
        return write!(f, "{}", "}");
    }
}
/// Parses an interface from a string, including methods and interface-level annotations.
///
/// Returns a tuple of the remaining input and the parsed `Interface`.
pub fn parse_interface(a: &str) -> IResult<&str, Interface> {
    pub fn go(a: &str) -> IResult<&str, Interface> {
        let (a, s) = separated_list0(char(';'), tuple((multispace0, ident, parse_sig))).parse(a)?;
        let (a, _) = multispace0(a)?;
        return Ok((
            a,
            Interface {
                methods: s.into_iter().map(|(_, a, b)| (a.to_owned(), b)).collect(),
                ann: vec![],
            },
        ));
    }
    let (a, _) = multispace0(a)?;
    let (a, b) = parse_attrs(a)?;
    let (a, mut c) = delimited(char('{'), go, char('}')).parse(a)?;
    c.ann = b;
    return Ok((a, c));
}
macro_rules! display {
    ($($t:ty),*) => {
        const _: () = {$(impl Display for $t{
            fn fmt(&self, f: &mut Formatter) -> core::fmt::Result{
                self.render(f,&|_|None)
            }
        })*};
    };
}
display!(Sig, ResTy, Arg);
impl Display for Interface {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.render(f, &|ab| self.ann.iter().find_map(|a| a.as_ver(ab)))
    }
}
impl Interface {
    pub fn rid(&self) -> [u8; 32] {
        // use core::io::Write;
        use core::fmt::Write;
        let mut s = Sha3_256::default();
        write!(WriteUpdate { wrapped: &mut s }, "{self}").unwrap();
        return s.finalize().try_into().unwrap();
    }
    pub fn rid_str(&self) -> String {
        return hex::encode(self.rid());
    }
}
pub mod info;
pub fn retuple(a: Vec<Arg>) -> Interface {
    Interface {
        methods: a
            .into_iter()
            .enumerate()
            .map(|(a, b)| {
                (
                    format!("v{a}"),
                    Sig {
                        ann: vec![],
                        params: vec![],
                        rets: vec![b],
                    },
                )
            })
            .collect(),
        ann: vec![],
    }
}
