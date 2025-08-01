use std::str::FromStr;

use nom::{
    branch::alt, bytes::complete::*, character::complete::*, combinator::*, multi::*, sequence::*,
    IResult,
};

#[derive(Debug, Clone)]
pub enum Pattern {
    Uri(String),
    Seq(Box<Pattern>, Box<Pattern>),
    Alt(Box<Pattern>, Box<Pattern>),
    Star(Box<Pattern>),
    Plus(Box<Pattern>),
    Opt(Box<Pattern>),
}

#[derive(Debug, Clone)]
pub enum Vertex {
    Any,
    Con(String),
}

#[derive(Debug, Clone)]
pub struct Query {
    pub src: Vertex,
    pub pattern: Pattern,
    pub dest: Vertex,
}

fn parse_query(input: &str) -> IResult<&str, Query> {
    let (input, src) = parse_vertex(input)?;
    let (input, pattern) = parse_pattern(input)?;
    let (input, dest) = parse_vertex(input)?;
    Ok((input, Query { src, pattern, dest }))
}

fn parse_vertex(input: &str) -> IResult<&str, Vertex> {
    delimited(multispace0, alt((parse_any, parse_con)), multispace0)(input)
}

fn parse_con(input: &str) -> IResult<&str, Vertex> {
    delimited(
        char('<'),
        map(take_until(">"), |s: &str| Vertex::Con(s.to_string())),
        char('>'),
    )(input)
}

fn parse_any(input: &str) -> IResult<&str, Vertex> {
    preceded(
        char('?'),
        map(take_while(|c: char| c.is_alphanumeric()), |_| Vertex::Any),
    )(input)
}

fn uri(input: &str) -> IResult<&str, Pattern> {
    delimited(
        char('<'),
        map(take_until(">"), |s: &str| Pattern::Uri(s.to_string())),
        char('>'),
    )(input)
}

fn parse_pattern(input: &str) -> IResult<&str, Pattern> {
    parse_alt(input)
}

fn parens(input: &str) -> IResult<&str, Pattern> {
    delimited(char('('), parse_pattern, char(')'))(input)
}

fn atom(input: &str) -> IResult<&str, Pattern> {
    /*delimited(multispace0, */
    alt((uri, parens))/*, multispace0)*/(input)
}

fn factor(input: &str) -> IResult<&str, Pattern> {
    let (input, base) = atom(input)?;
    let (input, modifier) = opt(one_of("*+?"))(input)?;

    let res = match modifier {
        Some('*') => Pattern::Star(Box::new(base)),
        Some('+') => Pattern::Plus(Box::new(base)),
        Some('?') => Pattern::Opt(Box::new(base)),
        _ => base,
    };

    Ok((input, res))
}

fn parse_seq(input: &str) -> IResult<&str, Pattern> {
    let (input, first) = factor(input)?;
    let (input, rest) = many0(preceded(
        delimited(multispace0, char('/'), multispace0),
        factor,
    ))(input)?;

    Ok((
        input,
        rest.into_iter().fold(first, |acc, item| {
            Pattern::Seq(Box::new(acc), Box::new(item))
        }),
    ))
}

fn parse_alt(input: &str) -> IResult<&str, Pattern> {
    let (input, first) = parse_seq(input)?;
    let (input, rest) = many0(preceded(
        delimited(multispace0, char('|'), multispace0),
        parse_seq,
    ))(input)?;

    Ok((
        input,
        rest.into_iter().fold(first, |acc, item| {
            Pattern::Alt(Box::new(acc), Box::new(item))
        }),
    ))
}

impl FromStr for Query {
    type Err = nom::Err<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match parse_query(s) {
            Ok(("", name)) => Ok(name),
            Ok((remaining, _)) => Err(nom::Err::Error(format!(
                "unparsed input is remaining: {:?}",
                remaining
            ))),
            Err(err) => Err(err.map(|err| err.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn test_basic_seq() {
        expect![[r#"Query { src: Con("0"), pattern: Seq(Uri("a"), Uri("b")), dest: Con("1") }"#]]
            .assert_eq(format!("{:?}", "<0> <a>/<b> <1>".parse::<Query>().unwrap()).as_str());
    }
    #[test]
    fn test_basic_seq_star() {
        expect![[r#"Query { src: Con("0"), pattern: Star(Seq(Uri("a"), Uri("b"))), dest: Any }"#]]
            .assert_eq(format!("{:?}", "<0> (<a>/<b>)* ?x".parse::<Query>().unwrap()).as_str());
    }
    #[test]
    fn test_basic_seq_alt() {
        expect![[r#"Query { src: Any, pattern: Star(Alt(Uri("a"), Uri("b"))), dest: Con("0") }"#]]
            .assert_eq(format!("{:?}", "?x (<a>|<b>)* <0>".parse::<Query>().unwrap()).as_str());
    }
    #[test]
    fn test_basic_seq_alt_prec() {
        expect![[r#"Query { src: Any, pattern: Alt(Seq(Seq(Uri("a"), Uri("b")), Uri("c")), Uri("d")), dest: Any }"#]]
            .assert_eq(format!("{:?}", "?x <a>/<b>/<c>|<d> ?y".parse::<Query>().unwrap()).as_str());
    }
    #[test]
    fn test_basic_seq_star_prec() {
        expect![[r#"Query { src: Con("1"), pattern: Seq(Uri("a"), Star(Uri("b"))), dest: Any }"#]]
            .assert_eq(format!("{:?}", "<1> <a>/<b>* ?y".parse::<Query>().unwrap()).as_str());
    }
    #[test]
    fn test_basic_parens_star() {
        expect![[r#"Query { src: Any, pattern: Seq(Uri("a"), Star(Seq(Uri("b"), Uri("c")))), dest: Any }"#]]
            .assert_eq(format!("{:?}", "?x <a>/(<b>/<c>)* ?y".parse::<Query>().unwrap()).as_str());
    }
    #[test]
    fn test_basic_opt() {
        expect![[r#"Query { src: Any, pattern: Seq(Seq(Seq(Uri("a"), Opt(Uri("b"))), Opt(Uri("b"))), Opt(Uri("b"))), dest: Con("e") }"#]]
            .assert_eq(format!("{:?}", "?x <a>/<b>?/<b>?/<b>? <e>".parse::<Query>().unwrap()).as_str());
    }
}
