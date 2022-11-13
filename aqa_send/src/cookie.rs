use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1, take_while_m_n};
use nom::character::complete::char;
use nom::combinator::{map, map_parser, opt, recognize, value};
use nom::error::{ErrorKind, ParseError};
use nom::multi::fold_many1;
use nom::sequence::{preceded, separated_pair, terminated};
use nom::{AsChar, IResult, InputTakeAtPosition};
use std::collections::HashMap;

#[derive(Default, Debug, Eq, PartialEq)]
pub struct Cookie {
	pub name: String,
	pub value: String,
	pub expires: Option<DateHeader>,
	pub max_age: Option<i64>,
	pub domain: Option<String>,
	pub path: Option<String>,
	pub secure: bool,
	pub http_only: bool,
	pub same_site: Option<SameSite>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SameSite {
	Strict,
	Lax,
	None,
}

pub fn parse_cookie(input: &str) -> IResult<&str, HashMap<String, String>> {
	fold_many1(
		terminated(
			separated_pair(cookie_ascii::<&str, _>, tag("="), cookie_ascii::<&str, _>),
			opt(tag("; ")),
		),
		HashMap::new,
		|mut acc: HashMap<_, _>, (name, value)| {
			acc.insert(name.to_string(), value.to_string());
			acc
		},
	)(input)
}

pub fn parse_set_cookie(input: &str) -> IResult<&str, Cookie> {
	let (input, mut cookie) = map(
		separated_pair(cookie_ascii, char('='), cookie_ascii),
		|(name, value): (&str, &str)| Cookie {
			name: name.to_string(),
			value: value.to_string(),
			..Default::default()
		},
	)(input)?;

	let mut loop_input = input;
	loop {
		let (input, attributes_parser) = opt(tag("; "))(loop_input)?;
		if attributes_parser.is_none() {
			break;
		}

		let (input, _): (&str, ()) = alt((
			map(parse_expires(), |date| {
				cookie.expires = Some(date);
			}),
			map(parse_max_age(), |max_age| {
				cookie.max_age = Some(max_age);
			}),
			map(parse_domain(), |domain: &str| {
				cookie.domain = Some(domain.to_string());
			}),
			map(parse_path(), |path: &str| {
				cookie.path = Some(path.to_string());
			}),
			map(tag("Secure"), |_| {
				cookie.secure = true;
			}),
			map(tag("HttpOnly"), |_| {
				cookie.http_only = true;
			}),
			map(parse_same_site(), |same_site| {
				cookie.same_site = Some(same_site);
			}),
		))(input)?;
		loop_input = input;
	}
	let input = loop_input;

	Ok((input, cookie))
}

fn parse_expires() -> impl FnMut(&str) -> IResult<&str, DateHeader> {
	move |input: &str| preceded(tag("Expires="), parse_date)(input)
}

fn parse_max_age() -> impl FnMut(&str) -> IResult<&str, i64> {
	use nom::character::complete::i64;
	move |input: &str| preceded(tag("Max-Age="), i64)(input)
}

fn parse_same_site() -> impl FnMut(&str) -> IResult<&str, SameSite> {
	move |input: &str| {
		preceded(
			tag("SameSite="),
			alt((
				map(tag("Strict"), |_| SameSite::Strict),
				map(tag("Lax"), |_| SameSite::Lax),
				map(tag("None"), |_| SameSite::None),
			)),
		)(input)
	}
}

fn parse_domain() -> impl FnMut(&str) -> IResult<&str, &str> {
	move |input: &str| {
		preceded(
			tag("Domain="),
			take_while1(|c: char| c.is_ascii_alphanumeric() || c == '.'),
		)(input)
	}
}

fn parse_path() -> impl FnMut(&str) -> IResult<&str, &str> {
	move |input: &str| {
		preceded(
			tag("Path="),
			take_while1(|c: char| c.is_ascii_alphanumeric() || c == '/'),
		)(input)
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DateHeader {
	pub day_name: DayName,
	pub day: u8,
	pub month: u8,
	pub year: u16,
	pub hour: u8,
	pub minute: u8,
	pub second: u8,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DayName {
	Mon,
	Tue,
	Wed,
	Thu,
	Fri,
	Sat,
	Sun,
}

fn parse_date(input: &str) -> IResult<&str, DateHeader> {
	use nom::character::complete::{u16, u8};

	let (input, day_name) = alt((
		value(DayName::Mon, tag("Mon")),
		value(DayName::Tue, tag("Tue")),
		value(DayName::Wed, tag("Wed")),
		value(DayName::Thu, tag("Thu")),
		value(DayName::Fri, tag("Fri")),
		value(DayName::Sat, tag("Sat")),
		value(DayName::Sun, tag("Sun")),
	))(input)?;

	let (input, _) = tag(", ")(input)?;

	let (input, day) = map_parser(recognize(take_while_m_n(2, 2, char::is_dec_digit)), u8)(input)?;
	let (input, _) = char(' ')(input)?;

	let (input, month) = alt((
		value(1_u8, tag("Jan")),
		value(2_u8, tag("Feb")),
		value(3_u8, tag("Mar")),
		value(4_u8, tag("Apr")),
		value(5_u8, tag("May")),
		value(6_u8, tag("Jun")),
		value(7_u8, tag("Jul")),
		value(8_u8, tag("Aug")),
		value(9_u8, tag("Sep")),
		value(10_u8, tag("Oct")),
		value(11_u8, tag("Nov")),
		value(12_u8, tag("Dec")),
	))(input)?;
	let (input, _) = char(' ')(input)?;

	let (input, year) =
		map_parser(recognize(take_while_m_n(4, 4, char::is_dec_digit)), u16)(input)?;
	let (input, _) = char(' ')(input)?;

	let (input, hour) = map_parser(recognize(take_while_m_n(2, 2, char::is_dec_digit)), u8)(input)?;
	let (input, _) = char(':')(input)?;
	let (input, minute) =
		map_parser(recognize(take_while_m_n(2, 2, char::is_dec_digit)), u8)(input)?;
	let (input, _) = char(':')(input)?;
	let (input, second) =
		map_parser(recognize(take_while_m_n(2, 2, char::is_dec_digit)), u8)(input)?;

	let (input, _) = tag(" GMT")(input)?;

	Ok((
		input,
		DateHeader {
			day_name,
			day,
			month,
			year,
			hour,
			minute,
			second,
		},
	))
}

pub fn cookie_ascii<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
	T: InputTakeAtPosition,
	<T as InputTakeAtPosition>::Item: AsChar,
{
	input.split_at_position1_complete(|item| !matches!(item.as_char(), 'A'..='Z' | 'a'..='z' | '0'..='9' | '!' | '#'..='\'' | '*'..='+' | '-'..='.' | '`' | '|' | '~'), ErrorKind::AlphaNumeric)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_date_parser() {
		let date = "Wed, 21 Oct 2015 07:28:01 GMT";
		let (_, date) = parse_date(date).unwrap();
		assert!(matches!(date.day_name, DayName::Wed));
		assert_eq!(date.day, 21);
		assert_eq!(date.month, 10);
		assert_eq!(date.year, 2015);
		assert_eq!(date.hour, 7);
		assert_eq!(date.minute, 28);
		assert_eq!(date.second, 1);
	}

	#[test]
	fn test_parse_cookie_with_expiry_date() {
		let cookie = "id=a3fWa; Expires=Wed, 21 Oct 2015 07:28:00 GMT";
		let (_, cookie) = parse_set_cookie(cookie).unwrap();

		let expected = Cookie {
			name: "id".to_string(),
			value: "a3fWa".to_string(),
			expires: Some(DateHeader {
				day_name: DayName::Wed,
				day: 21,
				month: 10,
				year: 2015,
				hour: 7,
				minute: 28,
				second: 0,
			}),
			max_age: None,
			domain: None,
			path: None,
			secure: false,
			http_only: false,
			same_site: None,
		};
		assert_eq!(cookie, expected);
	}

	#[test]
	fn test_parse_cookie() {
		let cookie = "id=a3fWa";
		let (_, cookie) = parse_set_cookie(cookie).unwrap();

		let expected = Cookie {
			name: "id".to_string(),
			value: "a3fWa".to_string(),
			expires: None,
			max_age: None,
			domain: None,
			path: None,
			secure: false,
			http_only: false,
			same_site: None,
		};
		assert_eq!(cookie, expected);
	}

	#[test]
	fn test_parse_cookie_with_all_attributes() {
		let cookie = "id=a3fWa; Expires=Wed, 21 Oct 2015 07:28:00 GMT; HttpOnly; Secure; SameSite=Strict; Domain=foo.example.com; Max-Age=12305; Path=/docs/Web/HTTP";
		let (_, cookie) = parse_set_cookie(cookie).unwrap();

		let expected = Cookie {
			name: "id".to_string(),
			value: "a3fWa".to_string(),
			expires: Some(DateHeader {
				day_name: DayName::Wed,
				day: 21,
				month: 10,
				year: 2015,
				hour: 7,
				minute: 28,
				second: 0,
			}),
			max_age: Some(12305),
			domain: Some("foo.example.com".to_string()),
			path: Some("/docs/Web/HTTP".to_string()),
			secure: true,
			http_only: true,
			same_site: Some(SameSite::Strict),
		};
		assert_eq!(cookie, expected);
	}
}
