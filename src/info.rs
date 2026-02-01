use alloc::{borrow::ToOwned, collections::BTreeMap, string::String, vec::Vec};

#[cfg(test)]
use alloc::format;
use core::fmt::Display;

use nom::{
    bytes::complete::{tag, take_while_m_n},
    character::complete::{alphanumeric1, multispace0},
    multi::many0,
    sequence::delimited,
    IResult, Parser,
};

use crate::{merge, parse_attr, Attr};

/// Macro to generate documentation attribute accessor methods for types that have an `attrs` field.
///
/// This macro generates a set of common documentation attribute accessor methods for any type
/// that contains an `attrs: Vec<Attr>` field. The generated methods include:
///
/// - `name()` - Returns the human-readable display name
/// - `doc()` - Returns the full documentation text  
/// - `brief()` - Returns a brief summary
/// - `deprecated()` - Returns deprecation message
/// - `llm_context()` - Returns LLM-readable context
/// - `llm_intent()` - Returns LLM-readable intent
/// - `category()` - Returns the category
/// - `since()` - Returns version when introduced
/// - `get_attr(name)` - Returns value of any attribute by name
///
/// All methods are feature-gated behind `#[cfg(feature = "doc-attrs")]`.
///
/// # Usage
///
/// ```rust,ignore
/// struct MyType {
///     attrs: Vec<Attr>,
///     // ... other fields
/// }
///
/// impl_doc_attrs!(MyType);
/// ```
///
/// This will generate an `impl` block with all the documentation accessor methods.
macro_rules! impl_doc_attrs {
    ($type:ty) => {
        #[cfg(feature = "doc-attrs")]
        impl $type {
            /// Returns the human-readable display name for this item, if set.
            pub fn name(&self) -> Option<&str> {
                self.attrs.iter().find_map(|a| a.as_name())
            }

            /// Returns the full documentation text for this item, if set.
            pub fn doc(&self) -> Option<&str> {
                self.attrs.iter().find_map(|a| a.as_doc())
            }

            /// Returns the brief summary for this item, if set.
            pub fn brief(&self) -> Option<&str> {
                self.attrs.iter().find_map(|a| a.as_brief())
            }

            /// Returns the deprecation message for this item, if set.
            pub fn deprecated(&self) -> Option<&str> {
                self.attrs.iter().find_map(|a| a.as_deprecated())
            }

            /// Returns the LLM context for this item, if set.
            pub fn llm_context(&self) -> Option<&str> {
                self.attrs.iter().find_map(|a| a.as_llm_context())
            }

            /// Returns the LLM intent for this item, if set.
            pub fn llm_intent(&self) -> Option<&str> {
                self.attrs.iter().find_map(|a| a.as_llm_intent())
            }

            /// Returns the category for this item, if set.
            pub fn category(&self) -> Option<&str> {
                self.attrs.iter().find_map(|a| a.as_category())
            }

            /// Returns the version when this item was introduced, if set.
            pub fn since(&self) -> Option<&str> {
                self.attrs.iter().find_map(|a| a.as_since())
            }

            /// Returns the value of an attribute by name, if set.
            pub fn get_attr(&self, name: &str) -> Option<&str> {
                self.attrs.iter().find_map(|a| a.as_attr(name))
            }
        }
    };
}

/// Stores attributes for a method parameter or return value.
#[derive(Default, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct ParamEntry {
    pub attrs: Vec<Attr>,
}

impl ParamEntry {
    /// Merges two ParamEntry structs, combining their attributes.
    pub fn merge(self, x: ParamEntry) -> ParamEntry {
        ParamEntry {
            attrs: merge(self.attrs, x.attrs),
        }
    }
}

// Generate documentation attribute accessor methods for ParamEntry
// This provides: name(), doc(), brief(), deprecated(), llm_context(), llm_intent(), 
// category(), since(), and get_attr() methods
impl_doc_attrs!(ParamEntry);
/// Stores interface information for the crate.
#[derive(Default, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct Info {
    pub interfaces: BTreeMap<[u8; 32], InfoEntry>,
}

/// Merges two Info structs, combining their interfaces.
impl Info {
    pub fn merge(self, x: Info) -> Info {
        let mut m: BTreeMap<[u8; 32], InfoEntry> = BTreeMap::new();
        for (a, b) in self.interfaces.into_iter().chain(x.interfaces.into_iter()) {
            let c = m.remove(&a).unwrap_or_default().merge(b);
            m.insert(a, c);
        }
        Info { interfaces: m }
    }

    /// Parses info from a string.
    pub fn parse(input: &str) -> IResult<&str, Info> {
        fn parse_interface_entry(input: &str) -> IResult<&str, ([u8; 32], InfoEntry)> {
            let (input, _) = multispace0(input)?;
            let (input, hex_id) = take_while_m_n(64, 64, |c: char| c.is_digit(16))(input)?;
            let mut id = [0u8; 32];
            hex::decode_to_slice(hex_id, &mut id).unwrap();
            let (input, _) = multispace0(input)?;
            let (input, _) = tag(":")(input)?;
            let (input, _) = multispace0(input)?;
            let (input, entry) = delimited(tag("["), InfoEntry::parse, tag("]")).parse(input)?;
            Ok((input, (id, entry)))
        }

        let (input, entries) = many0(parse_interface_entry).parse(input)?;
        Ok((
            input,
            Info {
                interfaces: entries.into_iter().collect(),
            },
        ))
    }
}
/// Stores attributes and methods for an interface.
#[derive(Default, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct InfoEntry {
    pub attrs: Vec<Attr>,
    pub methods: BTreeMap<String, MethEntry>,
}

/// Merges two InfoEntry structs, combining their attributes and methods.
impl InfoEntry {
    pub fn merge(self, x: InfoEntry) -> InfoEntry {
        let mut m: BTreeMap<String, MethEntry> = BTreeMap::new();
        for (a, b) in self.methods.into_iter().chain(x.methods.into_iter()) {
            let c = m.remove(&a).unwrap_or_default().merge(b);
            m.insert(a, c);
        }
        InfoEntry {
            attrs: merge(self.attrs, x.attrs),
            methods: m,
        }
    }

    /// Parses an InfoEntry from a string.
    pub fn parse(input: &str) -> IResult<&str, InfoEntry> {
        let (input, _) = multispace0(input)?;

        // Parse any line and categorize it
        fn parse_info_line(input: &str) -> IResult<&str, InfoLine> {
            let (input, _) = multispace0(input)?;
            
            // Try to parse root attribute
            if let Ok((input, _)) = tag::<&str, &str, nom::error::Error<&str>>("root")(input) {
                let (input, _) = multispace0(input)?;
                let (input, attr) = parse_attr(input)?;
                return Ok((input, InfoLine::Root(attr)));
            }
            
            // Try to parse param attribute
            if let Ok((input, _)) = tag::<&str, &str, nom::error::Error<&str>>("param")(input) {
                let (input, _) = multispace0(input)?;
                let (input, method_name) = alphanumeric1(input)?;
                let (input, _) = multispace0(input)?;
                let (input, index_str) = alphanumeric1(input)?;
                let index = index_str.parse::<usize>().map_err(|_| {
                    nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
                })?;
                let (input, _) = multispace0(input)?;
                let (input, attr) = parse_attr(input)?;
                return Ok((input, InfoLine::Param(method_name.to_owned(), index, attr)));
            }
            
            // Try to parse return attribute
            if let Ok((input, _)) = tag::<&str, &str, nom::error::Error<&str>>("return")(input) {
                let (input, _) = multispace0(input)?;
                let (input, method_name) = alphanumeric1(input)?;
                let (input, _) = multispace0(input)?;
                let (input, index_str) = alphanumeric1(input)?;
                let index = index_str.parse::<usize>().map_err(|_| {
                    nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
                })?;
                let (input, _) = multispace0(input)?;
                let (input, attr) = parse_attr(input)?;
                return Ok((input, InfoLine::Return(method_name.to_owned(), index, attr)));
            }
            
            // Try to parse method attribute
            if let Ok((input, _)) = tag::<&str, &str, nom::error::Error<&str>>("method")(input) {
                let (input, _) = multispace0(input)?;
                let (input, method_name) = alphanumeric1(input)?;
                let (input, _) = multispace0(input)?;
                let (input, attr) = parse_attr(input)?;
                return Ok((input, InfoLine::Method(method_name.to_owned(), attr)));
            }
            
            // If none match, return an error
            Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
        }

        #[derive(Debug)]
        enum InfoLine {
            Root(Attr),
            Method(String, Attr),
            Param(String, usize, Attr),
            Return(String, usize, Attr),
        }

        // Parse all info lines
        let (input, lines) = many0(parse_info_line).parse(input)?;

        // Process lines to build InfoEntry
        let mut root_attrs = Vec::new();
        let mut methods: BTreeMap<String, MethEntry> = BTreeMap::new();

        for line in lines {
            match line {
                InfoLine::Root(attr) => {
                    root_attrs.push(attr);
                }
                InfoLine::Method(method_name, attr) => {
                    methods.entry(method_name)
                        .or_insert_with(Default::default)
                        .attrs
                        .push(attr);
                }
                InfoLine::Param(method_name, index, attr) => {
                    methods.entry(method_name)
                        .or_insert_with(Default::default)
                        .params
                        .entry(index)
                        .or_insert_with(Default::default)
                        .attrs
                        .push(attr);
                }
                InfoLine::Return(method_name, index, attr) => {
                    methods.entry(method_name)
                        .or_insert_with(Default::default)
                        .returns
                        .entry(index)
                        .or_insert_with(Default::default)
                        .attrs
                        .push(attr);
                }
            }
        }

        // Sort attributes
        root_attrs.sort_by_key(|k| k.name.clone());
        for method_entry in methods.values_mut() {
            method_entry.attrs.sort_by_key(|k| k.name.clone());
            for param_entry in method_entry.params.values_mut() {
                param_entry.attrs.sort_by_key(|k| k.name.clone());
            }
            for return_entry in method_entry.returns.values_mut() {
                return_entry.attrs.sort_by_key(|k| k.name.clone());
            }
        }

        let (input, _) = multispace0(input)?;
        Ok((
            input,
            InfoEntry {
                attrs: root_attrs,
                methods,
            },
        ))
    }
}

// Generate documentation attribute accessor methods for InfoEntry
// This provides: name(), doc(), brief(), deprecated(), llm_context(), llm_intent(),
// category(), since(), and get_attr() methods  
impl_doc_attrs!(InfoEntry);
/// Stores attributes for a method, including its parameters and return values.
#[derive(Default, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct MethEntry {
    pub attrs: Vec<Attr>,
    /// Parameters indexed by their position (0-based)
    pub params: BTreeMap<usize, ParamEntry>,
    /// Return values indexed by their position (0-based)
    pub returns: BTreeMap<usize, ParamEntry>,
}
/// Merges two MethEntry structs, combining their attributes, parameters, and returns.
impl MethEntry {
    pub fn merge(self, x: MethEntry) -> MethEntry {
        let mut params: BTreeMap<usize, ParamEntry> = BTreeMap::new();
        for (idx, param) in self.params.into_iter().chain(x.params.into_iter()) {
            let merged = params.remove(&idx).unwrap_or_default().merge(param);
            params.insert(idx, merged);
        }

        let mut returns: BTreeMap<usize, ParamEntry> = BTreeMap::new();
        for (idx, ret) in self.returns.into_iter().chain(x.returns.into_iter()) {
            let merged = returns.remove(&idx).unwrap_or_default().merge(ret);
            returns.insert(idx, merged);
        }

        MethEntry {
            attrs: merge(self.attrs, x.attrs),
            params,
            returns,
        }
    }

    /// Returns the parameter entry at the given index, if it exists.
    pub fn param(&self, index: usize) -> Option<&ParamEntry> {
        self.params.get(&index)
    }

    /// Returns the return value entry at the given index, if it exists.
    pub fn return_value(&self, index: usize) -> Option<&ParamEntry> {
        self.returns.get(&index)
    }
    
    /// Adds a parameter entry at the given index.
    pub fn add_param(&mut self, index: usize, param: ParamEntry) {
        self.params.insert(index, param);
    }
    
    /// Adds a return value entry at the given index.
    pub fn add_return(&mut self, index: usize, ret: ParamEntry) {
        self.returns.insert(index, ret);
    }
    
    /// Adds an attribute to a parameter at the given index.
    pub fn add_param_attr(&mut self, index: usize, attr: Attr) {
        self.params.entry(index)
            .or_insert_with(Default::default)
            .attrs
            .push(attr);
    }
    
    /// Adds an attribute to a return value at the given index.
    pub fn add_return_attr(&mut self, index: usize, attr: Attr) {
        self.returns.entry(index)
            .or_insert_with(Default::default)
            .attrs
            .push(attr);
    }
}

// Generate documentation attribute accessor methods for MethEntry
// This provides: name(), doc(), brief(), deprecated(), llm_context(), llm_intent(),
// category(), since(), and get_attr() methods
impl_doc_attrs!(MethEntry);
/// Display implementation for InfoEntry, formats attributes as root entries.
impl Display for InfoEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for a in self.attrs.iter() {
            writeln!(f, "root {a}")?;
        }
        for (k, m) in self.methods.iter() {
            for a in m.attrs.iter() {
                writeln!(f, "method {k} {a}")?;
            }
            for (idx, param) in m.params.iter() {
                for a in param.attrs.iter() {
                    writeln!(f, "param {k} {idx} {a}")?;
                }
            }
            for (idx, ret) in m.returns.iter() {
                for a in ret.attrs.iter() {
                    writeln!(f, "return {k} {idx} {a}")?;
                }
            }
        }
        Ok(())
    }
}
impl Display for Info {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (i, j) in self.interfaces.iter() {
            write!(f, "{}: [{}]", hex::encode(i), j)?;
        }
        Ok(())
    }
}

// Legacy parsing functions for backward compatibility
pub fn parse_entry(input: &str) -> IResult<&str, InfoEntry> {
    InfoEntry::parse(input)
}

pub fn parse_info(input: &str) -> IResult<&str, Info> {
    Info::parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_param_entry() {
        let mut param = ParamEntry::default();
        param.attrs.push(Attr {
            name: "name".to_owned(),
            value: "counter".to_owned(),
        });
        param.attrs.push(Attr {
            name: "doc".to_owned(),
            value: "A counter parameter".to_owned(),
        });

        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(param.name(), Some("counter"));
            assert_eq!(param.doc(), Some("A counter parameter"));
        }
    }

    #[test]
    fn test_method_with_params_and_returns() {
        let mut method = MethEntry::default();
        method.attrs.push(Attr {
            name: "name".to_owned(),
            value: "add".to_owned(),
        });

        // Add parameter at index 0
        let mut param0 = ParamEntry::default();
        param0.attrs.push(Attr {
            name: "name".to_owned(),
            value: "left".to_owned(),
        });
        method.params.insert(0, param0);

        // Add parameter at index 1
        let mut param1 = ParamEntry::default();
        param1.attrs.push(Attr {
            name: "name".to_owned(),
            value: "right".to_owned(),
        });
        method.params.insert(1, param1);

        // Add return value at index 0
        let mut ret0 = ParamEntry::default();
        ret0.attrs.push(Attr {
            name: "name".to_owned(),
            value: "result".to_owned(),
        });
        method.returns.insert(0, ret0);

        // Test accessors
        assert!(method.param(0).is_some());
        assert!(method.param(1).is_some());
        assert!(method.param(2).is_none());
        assert!(method.return_value(0).is_some());
        assert!(method.return_value(1).is_none());

        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(method.name(), Some("add"));
            assert_eq!(method.param(0).unwrap().name(), Some("left"));
            assert_eq!(method.param(1).unwrap().name(), Some("right"));
            assert_eq!(method.return_value(0).unwrap().name(), Some("result"));
        }
    }

    #[test]
    fn test_parsing_with_params_and_returns() {
        let info_str = r#"
        root [name=Calculator]
        root [doc=A simple calculator interface]
        method add [name=Addition]
        method add [doc=Adds two numbers]
        param add 0 [name=left]
        param add 0 [doc=The left operand]
        param add 1 [name=right]
        param add 1 [doc=The right operand]
        return add 0 [name=result]
        return add 0 [doc=The sum of the operands]
        method sub [name=Subtraction]
        param sub 0 [name=minuend]
        return sub 0 [name=difference]
        "#;

        let (remaining, entry) = InfoEntry::parse(info_str).unwrap();
        // println!("Remaining: '{}'", remaining);
        assert!(remaining.trim().is_empty(), "Remaining input should be empty");

        // Check root attributes
        assert_eq!(entry.attrs.len(), 2);
        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(entry.name(), Some("Calculator"));
            assert_eq!(entry.doc(), Some("A simple calculator interface"));
        }

        // Check methods
        assert_eq!(entry.methods.len(), 2);
        
        let add_method = entry.methods.get("add").unwrap();
        assert_eq!(add_method.attrs.len(), 2);
        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(add_method.name(), Some("Addition"));
            assert_eq!(add_method.doc(), Some("Adds two numbers"));
        }

        // Check parameters for add method
        assert_eq!(add_method.params.len(), 2);
        let _param0 = add_method.param(0).unwrap();
        let _param1 = add_method.param(1).unwrap();
        
        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(_param0.name(), Some("left"));
            assert_eq!(_param0.doc(), Some("The left operand"));
            assert_eq!(_param1.name(), Some("right"));
            assert_eq!(_param1.doc(), Some("The right operand"));
        }

        // Check return values for add method
        assert_eq!(add_method.returns.len(), 1);
        let _ret0 = add_method.return_value(0).unwrap();
        
        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(_ret0.name(), Some("result"));
            assert_eq!(_ret0.doc(), Some("The sum of the operands"));
        }

        // Check sub method
        let sub_method = entry.methods.get("sub").unwrap();
        assert_eq!(sub_method.params.len(), 1);
        assert_eq!(sub_method.returns.len(), 1);
        
        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(sub_method.param(0).unwrap().name(), Some("minuend"));
            assert_eq!(sub_method.return_value(0).unwrap().name(), Some("difference"));
        }
    }

    #[test]
    fn test_parsing_complete_info() {
        let info_str = r#"
        deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef: [
            root [name=TestInterface]
            method test [name=TestMethod]
            param test 0 [name=input]
            return test 0 [name=output]
        ]
        "#;

        let (remaining, info) = Info::parse(info_str).unwrap();
        assert!(remaining.trim().is_empty());

        assert_eq!(info.interfaces.len(), 1);
        
        let interface_id = hex::decode("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef").unwrap();
        let mut expected_id = [0u8; 32];
        expected_id.copy_from_slice(&interface_id);
        
        let entry = info.interfaces.get(&expected_id).unwrap();
        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(entry.name(), Some("TestInterface"));
        }
        
        let _method = entry.methods.get("test").unwrap();
        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(_method.name(), Some("TestMethod"));
            assert_eq!(_method.param(0).unwrap().name(), Some("input"));
            assert_eq!(_method.return_value(0).unwrap().name(), Some("output"));
        }
    }

    #[test]
    fn test_display_format() {
        let mut entry = InfoEntry::default();
        entry.attrs.push(Attr {
            name: "name".to_owned(),
            value: "TestInterface".to_owned(),
        });

        let mut method = MethEntry::default();
        method.attrs.push(Attr {
            name: "doc".to_owned(),
            value: "Test method".to_owned(),
        });

        let mut param = ParamEntry::default();
        param.attrs.push(Attr {
            name: "name".to_owned(),
            value: "input".to_owned(),
        });
        method.params.insert(0, param);

        let mut ret = ParamEntry::default();
        ret.attrs.push(Attr {
            name: "name".to_owned(),
            value: "output".to_owned(),
        });
        method.returns.insert(0, ret);

        entry.methods.insert("test".to_owned(), method);

        let display_str = format!("{}", entry);
        assert!(display_str.contains("root [name=TestInterface]"));
        assert!(display_str.contains("method test [doc=Test method]"));
        assert!(display_str.contains("param test 0 [name=input]"));
        assert!(display_str.contains("return test 0 [name=output]"));
    }

    #[test]
    fn test_merging_with_params_and_returns() {
        // Create first method entry
        let mut method1 = MethEntry::default();
        method1.attrs.push(Attr {
            name: "name".to_owned(),
            value: "test".to_owned(),
        });
        
        let mut param1 = ParamEntry::default();
        param1.attrs.push(Attr {
            name: "doc".to_owned(),
            value: "First param doc".to_owned(),
        });
        method1.params.insert(0, param1);

        // Create second method entry
        let mut method2 = MethEntry::default();
        method2.attrs.push(Attr {
            name: "doc".to_owned(),
            value: "test documentation".to_owned(),
        });
        
        let mut param2 = ParamEntry::default();
        param2.attrs.push(Attr {
            name: "name".to_owned(),
            value: "input".to_owned(),
        });
        method2.params.insert(0, param2);

        // Merge methods
        let merged = method1.merge(method2);
        
        // Check merged method attributes
        assert_eq!(merged.attrs.len(), 2);
        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(merged.name(), Some("test"));
            assert_eq!(merged.doc(), Some("test documentation"));
        }

        // Check merged parameter attributes
        let merged_param = merged.param(0).unwrap();
        assert_eq!(merged_param.attrs.len(), 2);
        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(merged_param.name(), Some("input"));
        }
    }

    #[test]
    fn test_doc_attrs_macro() {
        // Test that the macro-generated documentation methods work correctly
        
        // Test ParamEntry
        let mut param = ParamEntry::default();
        param.attrs.push(Attr {
            name: "name".to_owned(),
            value: "test_param".to_owned(),
        });
        param.attrs.push(Attr {
            name: "doc".to_owned(),
            value: "Test parameter".to_owned(),
        });
        param.attrs.push(Attr {
            name: "deprecated".to_owned(),
            value: "Use v2 instead".to_owned(),
        });

        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(param.name(), Some("test_param"));
            assert_eq!(param.doc(), Some("Test parameter"));
            assert_eq!(param.deprecated(), Some("Use v2 instead"));
            assert_eq!(param.brief(), None);
            assert_eq!(param.get_attr("name"), Some("test_param"));
            assert_eq!(param.get_attr("nonexistent"), None);
        }

        // Test MethEntry
        let mut method = MethEntry::default();
        method.attrs.push(Attr {
            name: "name".to_owned(),
            value: "test_method".to_owned(),
        });
        method.attrs.push(Attr {
            name: "llm.context".to_owned(),
            value: "AI helper context".to_owned(),
        });

        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(method.name(), Some("test_method"));
            assert_eq!(method.llm_context(), Some("AI helper context"));
            assert_eq!(method.doc(), None);
        }

        // Test InfoEntry  
        let mut info = InfoEntry::default();
        info.attrs.push(Attr {
            name: "category".to_owned(),
            value: "utility".to_owned(),
        });
        info.attrs.push(Attr {
            name: "since".to_owned(),
            value: "1.0.0".to_owned(),
        });

        #[cfg(feature = "doc-attrs")]
        {
            assert_eq!(info.category(), Some("utility"));
            assert_eq!(info.since(), Some("1.0.0"));
            assert_eq!(info.name(), None);
        }
    }
}
