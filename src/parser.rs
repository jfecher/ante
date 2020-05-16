use nom::IResult;
use nom::bytes::complete::tag;

pub fn parse_string(input: &str) -> IResult<&str, &str> {
    tag("print \"Hello, World!\"")(input)
}