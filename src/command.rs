use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete;
use nom::character::complete::space0;
use nom::character::complete::space1;
use nom::combinator::map;
use nom::combinator::opt;
use nom::error::Error as NomError;
use nom::sequence::tuple;
use nom::Parser;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Stop {
        index: Option<u8>,
    },
    Up {
        index: u8,
    },
    Down {
        index: u8,
    },
    Limit {
        index: u8,
        up_limit: Option<u32>,
        down_limit: Option<u32>,
    },
    Help,
}

impl Command {
    pub fn parse(input: &[u8]) -> Result<Command, ()> {
        match parser().parse(input) {
            Ok((_, command)) => Ok(command),
            Err(_) => Err(()),
        }
    }

    fn stop(index: Option<u8>) -> Self {
        Command::Stop { index }
    }

    fn up(index: u8) -> Self {
        Command::Up { index }
    }

    fn down(index: u8) -> Self {
        Command::Down { index }
    }

    fn limit(index: u8, up_limit: Option<u32>, down_limit: Option<u32>) -> Self {
        Command::Limit {
            index,
            up_limit,
            down_limit,
        }
    }

    fn help() -> Self {
        Command::Help
    }
}

fn parse_stop<'a>() -> impl Parser<&'a [u8], Command, NomError<&'a [u8]>> {
    map(
        tuple((
            tag("stop"),
            opt(map(tuple((space1, complete::u8)), |(_, index)| index)),
        )),
        |(_, index)| Command::stop(index),
    )
}

fn parse_up<'a>() -> impl Parser<&'a [u8], Command, NomError<&'a [u8]>> {
    map(tuple((tag("up"), space1, complete::u8)), |(_, _, index)| {
        Command::up(index)
    })
}

fn parse_down<'a>() -> impl Parser<&'a [u8], Command, NomError<&'a [u8]>> {
    map(
        tuple((tag("down"), space1, complete::u8)),
        |(_, _, index)| Command::down(index),
    )
}

fn parse_limit<'a>() -> impl Parser<&'a [u8], Command, NomError<&'a [u8]>> {
    map(
        tuple((
            tag("limit"),
            space1,
            complete::u8,
            opt(map(
                tuple((space1, tag("up"), space1, complete::u32)),
                |(_, _, _, limit)| limit,
            )),
            opt(map(
                tuple((space1, tag("down"), space1, complete::u32)),
                |(_, _, _, limit)| limit,
            )),
        )),
        |(_, _, index, up_limit, down_limit)| Command::limit(index, up_limit, down_limit),
    )
}

fn parse_help<'a>() -> impl Parser<&'a [u8], Command, NomError<&'a [u8]>> {
    map(tag("help"), |_| Command::help())
}

fn parser<'a>() -> impl Parser<&'a [u8], Command, NomError<&'a [u8]>> {
    map(
        tuple((
            space0,
            alt((
                parse_stop(),
                parse_up(),
                parse_down(),
                parse_limit(),
                parse_help(),
            )),
            space0,
        )),
        |(_, command, _)| command,
    )
}
