// Copyright (c) 2024 Damir Jelić
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    str::FromStr,
};

use anyhow::{anyhow, Result};

pub fn parse_proc_mouns() -> Result<Vec<Mount>> {
    const PROC_LOCATION: &str = "/proc/mounts";

    let path = PathBuf::from(PROC_LOCATION);
    let file = File::open(path)?;

    let reader = BufReader::new(file);

    let mut mounts = Vec::new();

    for line in reader.lines() {
        let line = line?;

        match Mount::from_str(&line) {
            Ok(mount) => mounts.push(mount),
            Err(e) => eprintln!("Failed to parse a /proc/mounts line: {e:?}"),
        }
    }

    Ok(mounts)
}

#[derive(Debug, Clone)]
pub struct Mount {
    pub device: String,
    pub mount_point: PathBuf,
    pub file_system_type: FileSystemType,
    pub options: Vec<String>,
    pub file_system_frequency: u8,
    pub file_system_pass_number: u8,
}

#[derive(Debug, Clone)]
pub enum FileSystemType {
    TmpFs,
    Unknown(String),
}

impl From<&str> for FileSystemType {
    fn from(value: &str) -> Self {
        match value {
            "tmpfs" => Self::TmpFs,
            other => Self::Unknown(other.to_owned()),
        }
    }
}

impl Mount {
    fn parse(line: &str) -> Result<Mount> {
        let (_, mount) = parser::parse_line(line)
            .map_err(|err| anyhow!("Failed to parse a mounts line: {err:?}"))?;

        Ok(mount)
    }
}

impl FromStr for Mount {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse(s)
    }
}

mod parser {
    use std::path::PathBuf;

    use nom::{
        bytes::complete::{tag, take_till, take_while1},
        character::complete::{one_of, space0, space1},
        combinator::{all_consuming, map, map_parser, map_res},
        multi::separated_list1,
        sequence::tuple,
        IResult,
    };

    use super::{FileSystemType, Mount};

    fn is_digit(c: char) -> bool {
        c.is_digit(10)
    }

    fn from_digit(input: &str) -> Result<u8, std::num::ParseIntError> {
        u8::from_str_radix(input, 10)
    }

    fn escaped_space(i: &str) -> nom::IResult<&str, &str> {
        nom::combinator::value(" ", nom::bytes::complete::tag("040"))(i)
    }

    fn escaped_backslash(i: &str) -> nom::IResult<&str, &str> {
        nom::combinator::recognize(nom::character::complete::char('\\'))(i)
    }

    fn transform_escaped(i: &str) -> nom::IResult<&str, std::string::String> {
        nom::bytes::complete::escaped_transform(
            nom::bytes::complete::is_not("\\"),
            '\\',
            nom::branch::alt((escaped_backslash, escaped_space)),
        )(i)
    }

    fn string(input: &str) -> IResult<&str, String> {
        map_parser(take_till(char::is_whitespace), transform_escaped)(input)
    }

    fn string_without_comma(input: &str) -> IResult<&str, &str> {
        take_till(|c: char| c == ',' || c.is_whitespace())(input)
    }

    fn device(input: &str) -> IResult<&str, String> {
        map(tuple((space0, string, space1)), |(_, device, _)| String::from(device))(input)
    }

    fn mount_point(input: &str) -> IResult<&str, PathBuf> {
        map(tuple((space0, string, space1)), |(_, mount_point, _)| PathBuf::from(mount_point))(
            input,
        )
    }

    fn filesystem_type(input: &str) -> IResult<&str, FileSystemType> {
        map(tuple((space0, string, space1)), |(_, filesystem_type, _)| {
            FileSystemType::from(filesystem_type.as_str())
        })(input)
    }

    fn mount_options(input: &str) -> IResult<&str, Vec<String>> {
        map(
            tuple((space0, separated_list1(tag(","), string_without_comma), space1)),
            |(_, options, _)| options.into_iter().map(|s| s.to_owned()).collect(),
        )(input)
    }

    fn fs_frequency(input: &str) -> IResult<&str, u8> {
        map_res(tuple((space0, take_while1(is_digit), space1)), |(_, number, _)| from_digit(number))(
            input,
        )
    }

    fn fs_passno(input: &str) -> IResult<&str, u8> {
        map_res(tuple((space0, one_of("012"), space0)), |(_, number, _)| {
            from_digit(&number.to_string())
        })(input)
    }

    pub fn parse_line(line: &str) -> IResult<&str, Mount> {
        map(
            all_consuming(tuple((
                device,
                mount_point,
                filesystem_type,
                mount_options,
                fs_frequency,
                fs_passno,
            ))),
            |(
                device,
                mount_point,
                file_system_type,
                options,
                file_system_frequency,
                file_system_pass_number,
            )| Mount {
                device,
                mount_point,
                file_system_type,
                options,
                file_system_frequency,
                file_system_pass_number,
            },
        )(line)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_line() {
        const LINE: &str = "proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0";

        let mount =
            Mount::from_str(&LINE).expect("We should be able to parse the /proc mount line");

        println!("{mount:?}");
    }
}
