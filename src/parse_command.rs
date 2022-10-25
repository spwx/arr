use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take},
    combinator::{cut, fail, map, map_parser},
    multi::many0,
    sequence::delimited,
    IResult,
};

use std::{collections::HashMap, path::Path};

use crate::error::ArrError;

fn lookup_value<'i>(
    args: &'i HashMap<String, String>,
) -> impl FnMut(&'i str) -> IResult<&'i str, String> {
    move |input| match args.get(&input.to_string()) {
        Some(value) => Ok(("", value.clone())),
        None => cut(fail)(input),
    }
}

fn get_vars<'i>(
    args: &'i HashMap<String, String>,
) -> impl FnMut(&'i str) -> IResult<&'i str, String> {
    move |input| {
        let (tail, head) = many0(alt((
            // get anything before #
            map(is_not("#"), String::from),
            // get a variable and replace it with its value
            // variables are surrounded with "#{" and "}"
            map_parser(
                delimited(tag("#{"), is_not("}"), tag("}")),
                lookup_value(args),
            ),
            // if the '#' was a false alarm, eat it up
            map(take(1usize), String::from),
        )))(input)?;

        let head = head.join("");

        Ok((tail, head))
    }
}

fn update_path(art_path: &str) -> impl FnMut(&str) -> IResult<&str, String> + '_ {
    move |input| {
        let (tail, head) = many0(alt((
            map(is_not("P"), String::from),
            map(tag("PathToAtomicsFolder"), |_| art_path.to_string()),
            map(take(1usize), String::from),
        )))(input)?;

        let head = head.join("");

        Ok((tail, head))
    }
}

pub fn parse_command(
    command: &str,
    art_path: &Path,
    args: &HashMap<String, String>,
) -> Result<String, ArrError> {
    let art_path = art_path.to_str().unwrap_or("");

    let (_tail, parsed_command) =
        update_path(art_path)(command).map_err(|e| ArrError::OtherNomError(e.to_string()))?;

    let res = match get_vars(args)(&parsed_command) {
        Ok((_tail, c)) => Ok(c),
        Err(nom::Err::Failure(e)) => Err(ArrError::ArgValueNotFound(e.input.to_string())),
        Err(e) => Err(ArrError::OtherNomError(e.to_string())),
    };

    res
}

#[cfg(test)]
mod test {
    use super::*;
    // use std::path::Path;

    fn setup_args() -> HashMap<String, String> {
        let mut args = HashMap::new();
        args.insert("var1".to_string(), "1".to_string());
        args.insert("var2".to_string(), "2".to_string());
        args.insert("var3".to_string(), "3".to_string());

        args
    }

    // #[test]
    // fn test_parse_command() {
    //     assert_eq!(
    //         parse_command("abc_#{var1}_b_c", Path::new("art"), &setup_args()),
    //         Ok("abc_1_b_c".to_string()),
    //     );
    // }

    // #[test]
    // fn test_parse_command_failure() {
    //     assert_eq!(
    //         parse_command("abc_#{var9}_b_c", Path::new("art"), &setup_args()),
    //         Err(ArrError::ArgValueNotFound("var9".to_string()))
    //     );
    // }

    #[test]
    fn no_vars() {
        assert_eq!(
            get_vars(&setup_args())("abc_123_b_c"),
            Ok(("", "abc_123_b_c".to_string())),
        );
    }

    #[test]
    fn var_in_middle() {
        assert_eq!(
            get_vars(&setup_args())("abc_#{var1}_b_c"),
            Ok(("", "abc_1_b_c".to_string())),
        );
    }

    #[test]
    fn var_at_beginning() {
        assert_eq!(
            get_vars(&setup_args())("#{var1}_b_c"),
            Ok(("", "1_b_c".to_string())),
        );
    }

    #[test]
    fn just_var() {
        assert_eq!(
            get_vars(&setup_args())("#{var1}"),
            Ok(("", "1".to_string()))
        );
    }

    #[test]
    fn fake_out_var() {
        assert_eq!(
            get_vars(&setup_args())("#{var1"),
            Ok(("", "#{var1".to_string())),
        );
    }

    #[test]
    fn tricky_vars() {
        assert_eq!(
            get_vars(&setup_args())("abc_##{var1}#_#{var2}_#{var3}"),
            Ok(("", "abc_#1#_2_3".to_string())),
        );
    }

    #[test]
    fn multiple_vars() {
        assert_eq!(
            get_vars(&setup_args())("abc_#{var1}_#{var2}_#{var3}"),
            Ok(("", "abc_1_2_3".to_string())),
        );
    }

    #[test]
    fn value_not_found() {
        assert_eq!(
            get_vars(&setup_args())("abc_#{var1}_#{var9}_#{var3}"),
            Err(nom::Err::Failure(nom::error::Error {
                input: "var9",
                code: nom::error::ErrorKind::Fail,
            })),
        );
    }

    #[test]
    fn test_update_path() {
        assert_eq!(
            update_path("yolo")("___PathToAtomicsFolder/LOL/123"),
            Ok(("", "___yolo/LOL/123".to_string()))
        );
    }
}
