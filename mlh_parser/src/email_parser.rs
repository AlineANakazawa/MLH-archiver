//! Top-level email parsing: decodes raw bytes into a [`ParsedEmail`].

use crate::ParsedEmail;
use crate::address_parser::{AddressScore, addr_to_string, score_email_address};
use crate::date_parser;
use crate::email_reader::{
    self, header_value_date, header_value_to_string, header_value_to_string_list,
};
use crate::errors::ParseError;
use crate::extractors::{self};

use crate::address_parser::normalize_address;
use chrono::{DateTime, FixedOffset, Utc};
use mail_parser::Message;
use parquet::errors::Result;

/// Parses a raw RFC 822 email byte slice into a [`ParsedEmail`].
///
/// Extracts headers, body text, trailers, and code patches. Dates are
/// normalized by [`process_date`](crate::date_parser::process_date). Missing
/// single-valued columns are populated with empty strings.
pub fn parse_email(
    email_data: &[u8],
    now: DateTime<FixedOffset>,
) -> Result<ParsedEmail, ParseError> {
    let msg = email_reader::decode_mail(email_data)
        .ok_or_else(|| ParseError::DecodeError("Failed to parse email bytes".to_string()))?;

    let mut email = ParsedEmail::default();
    collect_header_data(&msg, &mut email, now);

    let raw_body = email_reader::get_body(&msg);

    email.trailers = extractors::extract_attributions(&raw_body);
    email.code = extractors::extract_patches(&raw_body);
    email.raw_body = raw_body;

    Ok(email)
}

pub fn read_raw_offset(raw_content: &[u8], start_offset: u32, end_offset: u32) -> String {
    if start_offset >= end_offset {
        return String::new();
    }

    let start = (start_offset as usize).min(raw_content.len());
    let end = (end_offset as usize).min(raw_content.len());

    let sub_slice = &raw_content[start..end];
    String::from_utf8_lossy(sub_slice).into_owned()
}

/// Extracts all headers from a parsed message
///
/// Also evaluates headers using body information to better guide `From` selection.
/// The `From` header is chosen by scoring candidates (name presence, valid
/// email address). Obfuscated addresses are normalized.
fn collect_header_data(msg: &Message<'_>, email: &mut ParsedEmail, now: DateTime<FixedOffset>) {
    let mut from_candidates: Vec<String> = Vec::new();
    let mut date_options = vec![];
    let mut client_dates = vec![];

    for header in msg.headers() {
        let key = header.name().to_lowercase();

        if key == "message-id" {
            email.message_id = header_value_to_string(header.value()).unwrap_or_default();
        } else if key == "from" {
            if let Some(val_str) = header_value_to_string(header.value()) {
                from_candidates.push(val_str);
            }
        } else if key == "to" {
            if let Some(mut val_vec) = header_value_to_string_list(header.value()) {
                email.to.append(&mut val_vec);
            }
        } else if key == "cc" {
            if let Some(mut val_vec) = header_value_to_string_list(header.value()) {
                email.cc.append(&mut val_vec);
            }
        } else if key == "subject" {
            if let Some(val_str) = header_value_to_string(header.value()) {
                email.subject = val_str;
            }
        } else if key == "date" {
            // Date header is used in the client_date and possibly in the "date" column
            let raw_date = read_raw_offset(
                msg.raw_message(),
                header.offset_start(),
                header.offset_end(),
            );
            if let Some(val_date) = header_value_date(header.value()) {
                date_options.push(val_date);
            } else {
                if let Some(dt) = date_parser::parse_date_string(&raw_date) {
                    date_options.push(dt);
                }
            }
            client_dates.push(raw_date);

            // depends on type
        } else if key == "received" || key == "x-received" {
            // these in the other hand are only elegible to the "date" column
            if let Some(val_date) = header_value_date(header.value()) {
                date_options.push(val_date);
            } else {
                let raw_date = read_raw_offset(
                    msg.raw_message(),
                    header.offset_start(),
                    header.offset_end(),
                );
                if let Some(val_date) = header_value_date(header.value()) {
                    date_options.push(val_date);
                } else {
                    if let Some(dt) = date_parser::parse_date_string(&raw_date) {
                        date_options.push(dt);
                    }
                }
            }
        } else if key == "in-reply-to" {
            email.in_reply_to = header_value_to_string(header.value());
        } else if key == "references" {
            if let Some(mut val_vec) = header_value_to_string_list(header.value()) {
                email.references.append(&mut val_vec);
            }
        } else if key == "x-mailing-list" {
            email.x_mailing_list = header_value_to_string(header.value());
        }
    }

    // select date
    email.date = select_date(date_options, now);
    email.client_date = client_dates.join(", ");

    if from_candidates.is_empty()
        && let Some(from) = msg.from()
    {
        for addr in from.iter() {
            from_candidates.push(addr_to_string(addr));
        }
    }

    // some malformed messages put their "FROM" header in the body.
    let body_from = email_reader::extract_all_from_from_body(&msg.raw_message);

    from_candidates.extend(body_from);

    if !from_candidates.is_empty() {
        email.from = select_best_from_header(&from_candidates);
    };
}

fn select_best_from_header(values: &[String]) -> String {
    if values.is_empty() {
        return String::new();
    }
    if values.len() == 1 {
        return normalize_address(&values[0]);
    }

    let mut scored: Vec<(AddressScore, &String)> =
        values.iter().map(|v| (score_email_address(v), v)).collect();
    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    normalize_address(scored[0].1)
}

/// Processes the `date` and `client-date` entries in an email header map.
///
/// Selects the best date from the available options, applying millennium
/// correction and `Received`-header fallback in that order. The result is
/// stored back into `email_dict["date"]` as RFC 3339 and the raw client
/// dates in `email_dict["client-date"]` as `||`-delimited strings.
pub fn select_date(
    date_options: Vec<DateTime<FixedOffset>>,
    now: DateTime<FixedOffset>,
) -> Option<DateTime<Utc>> {
    // Filter out future dates
    let mut safe_options: Vec<DateTime<FixedOffset>> = date_options
        .into_iter()
        .filter(|d| date_parser::check_date_issues(d, now))
        .collect();

    // TODO: add a warning if the istance between dates is too large
    // this could feed a "date_confidence" field

    if !safe_options.is_empty() {
        safe_options.sort();
        Some(safe_options[0].into())
    } else {
        None
    }
}
