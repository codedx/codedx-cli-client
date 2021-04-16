/*
 * Copyright 2021 Code Dx, Inc
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std;
use std::str;
use std::str::FromStr;

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! str_vec {
        ($($x:expr),*) => (vec![$($x.to_string()),*]);
    }
}

/// Wrapper for a Vec<String> representing some command-line arguments.
///
/// To get an instance of CmdArgs you can either construct one with `CmdArgs::from`,
/// or parse one from a `&str` e.g. `"hello world".parse()`.
#[derive(Debug)]
pub struct CmdArgs(pub Vec<String>);

impl FromStr for CmdArgs {
    type Err = ();

    fn from_str(s: &str) -> Result<CmdArgs, ()> {
        arg_list(s.as_ref()).to_result()
            .map(|v| CmdArgs(v))
            .map_err(|_| ())
    }
}

impl IntoIterator for CmdArgs {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Convenience function to convert some raw bytes (from a Vec) into a String.
fn vec_to_s(i:Vec<u8>) -> String {
    String::from_utf8_lossy(&i).into_owned()
}

/// Convenience function to convert some raw bytes (from a slice) into a String.
fn to_s(i: &[u8]) -> String {
    String::from_utf8_lossy(i).into_owned()
}

/// Function to parse a string surrounded by double-quotes.
///
/// Includes spaces, and allows for escape characters `\"`, `\'`, and `\\`.
named!(dq_string <String>,
    delimited!(
        char!('"'),
        map!(
            escaped_transform!(is_not!("\\\""), '\\',
                alt!(
                    tag!("\\")       => { |_| &b"\\"[..] }
                  | tag!("\"")       => { |_| &b"\""[..] }
                  | tag!("\'")       => { |_| &b"\'"[..] }
                )
            ),
            vec_to_s
        ),
        char!('"')
    )
);

/// Function to parse a string surrounded by single-quotes.
///
/// Includes spaces, and allows for escape characters `\"`, `\'`, and `\\`.
named!(sq_string <String>,
    delimited!(
        char!('\''),
        map!(
            escaped_transform!(is_not!("\\\'"), '\\',
                alt!(
                    tag!("\\")       => { |_| &b"\\"[..] }
                  | tag!("\"")       => { |_| &b"\""[..] }
                  | tag!("\'")       => { |_| &b"\'"[..] }
                )
            ),
            vec_to_s
        ),
        char!('\'')
    )
);

/// Function to parse a string from a series of consecutive non-whitespace characters.
named!(consecutive_string <String>,
    map!(
        is_not!(" \t"),
        to_s
    )
);

named!(one_arg <String>, alt!(dq_string | sq_string | consecutive_string));

named!(arg_list<Vec<String>>, separated_list!(is_a!(" \t"), one_arg));

#[cfg(test)]
fn test_parse(s: &str) -> Result<Vec<String>, ::nom::Err<&[u8]>> {
    arg_list(s.as_ref()).to_result()
}

#[test]
fn test_plain_arg(){
    let args = test_parse("hello").unwrap();
    assert!(args == str_vec!["hello"]);
}

#[test]
fn test_single_quoted(){
    let args = test_parse("'hello'").unwrap();
    assert!(args == str_vec!["hello"]);
}

#[test]
fn test_single_quoted_escape(){
    let args = test_parse("'hello \\'world\\''").unwrap();
    assert!(args == str_vec!["hello 'world'"]);
}

#[test]
fn test_double_quoted(){
    let args = test_parse("\"hello\"").unwrap();
    assert!(args == str_vec!["hello"]);
}

#[test]
fn test_double_quoted_escape(){
    let args = test_parse("\"hello \\\"world\\\"\"").unwrap();
    assert!(args == str_vec!["hello \"world\""]);
}

#[test]
fn test_multi_plain(){
    let args = test_parse("hello world").unwrap();
    assert!(args == str_vec!["hello", "world"]);
}

#[test]
fn test_multi_mixed(){
    let args = test_parse("hello 'world' \"how are you\"").unwrap();
    assert!(args == str_vec!["hello", "world", "how are you"]);
}

#[test]
fn test_multi_mixed_escape(){
    let args = test_parse("'ahoy \\'matey\\'' hello \"\\\"hello\\\"\"").unwrap();
    assert!(args == str_vec!["ahoy 'matey'", "hello", "\"hello\""]);
}

#[test]
fn test_cmdargs_parse(){
    let args: CmdArgs = "hello world".parse().unwrap();
    assert!(args.0 == str_vec!["hello", "world"]);
}