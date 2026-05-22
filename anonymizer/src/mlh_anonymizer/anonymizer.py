"""Anonymization functions for applying SHA-1 hashing to various data types."""

import logging
import re
from typing import Any, Union

from mlh_anonymizer.hasher import generate_sha1_hash

logger = logging.getLogger(__name__)

EMAIL_RE = re.compile(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+(?:\.[a-zA-Z]{2,})?")
BRACKET_IDENTITY_RE = re.compile(r"([^<]*?)\s*<([^>]+@[^>]+)>")


def extract_name_and_prefix(text_before: str) -> tuple[str, str]:
    if not text_before:
        return "", ""

    seps = list(re.finditer(r"[:,;]", text_before))
    if seps:
        last_sep = seps[-1]
        sep_end = last_sep.end()
        ws_end = sep_end
        while ws_end < len(text_before) and text_before[ws_end] in (" ", "\t"):
            ws_end += 1
        prefix = text_before[:ws_end]
        name = text_before[ws_end:].strip()
        return name, prefix

    name = text_before.strip()
    return name, ""


def replace_bracketed_identity(m: re.Match) -> str:
    before = m.group(1)
    email = m.group(2)

    name, prefix = extract_name_and_prefix(before)

    if name:
        return f"{prefix}{generate_sha1_hash(name)} <{generate_sha1_hash(email)}>"
    return f"{before}<{generate_sha1_hash(email)}>"


def anonymize_line(line: str, is_multiline: bool = False) -> str:
    line = BRACKET_IDENTITY_RE.sub(replace_bracketed_identity, line)

    matches = list(re.finditer(EMAIL_RE, line))
    if not matches:
        return line

    result = []
    last_end = 0
    for m in matches:
        email = m.group(0)
        start, end = m.start(), m.end()

        if start > 0 and line[start - 1] == "<":
            continue
        if end < len(line) and line[end] == ">":
            continue

        text_before = line[last_end:start]

        if is_multiline and not text_before.strip():
            result.append(text_before)
            result.append(email)
            last_end = end
            continue

        name, prefix = extract_name_and_prefix(text_before)

        if name:
            result.append(prefix)
            result.append(name)
            result.append(" <")
            result.append(generate_sha1_hash(email))
            result.append(">")
        else:
            result.append(text_before)
            result.append(generate_sha1_hash(email))

        last_end = end

    result.append(line[last_end:])
    return "".join(result)


def anonymize_string(row_val: Any) -> Union[str, list[str]]:
    """Apply SHA-1 anonymization to identities within a row value.

    Detects identity patterns within strings and hashes only the
    identified name and email components. Processes line by line
    to avoid matching identities split across multiple lines.

    Handles strings and lists of strings.

    Args:
        row_val: Value to anonymize (str or list[str])

    Returns:
        Anonymized value with identities hashed

    Raises:
        Exception: If type is not supported
    """
    if isinstance(row_val, str):
        lines = row_val.split("\n")
        is_multiline = len(lines) > 1
        result_lines = [anonymize_line(line, is_multiline) for line in lines]
        return "\n".join(result_lines)
    if hasattr(row_val, "__iter__"):
        return [anonymize_string(val) for val in row_val]
    raise Exception(f"Unmapped type for {type(row_val)}")


def anonymize_map(row_val: Any, map_key: str) -> Union[list[dict], dict]:
    """Anonymize a specific key within map/list structures.

    Used for nested structures like trailers.identification.

    Args:
        row_val: row value (list[dict] or dict)
        map_key: Key within the dict to anonymize

    Returns:
        row value with specified key anonymized

    Raises:
        Exception: If type is not supported
    """
    if hasattr(row_val, "__iter__") and not isinstance(row_val, dict):
        parts = len(row_val)
        newrow_val = [{}] * parts
        for part_i in range(parts):
            part = row_val[part_i]
            # Anonymize the specified key
            part[map_key] = anonymize_string(part[map_key])
            newrow_val[part_i] = part
        return newrow_val
    elif isinstance(row_val, dict):
        newrow_val = {}
        newrow_val[map_key] = anonymize_string(row_val[map_key])
        return newrow_val
    else:
        raise Exception(f"Unsupported type for anonymize_map: {type(row_val)}")
