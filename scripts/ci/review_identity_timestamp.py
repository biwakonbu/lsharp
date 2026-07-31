"""Strict UTC timestamp validation shared by release identity boundaries."""

from __future__ import annotations

import re


UTC_TIMESTAMP_PATTERN = re.compile(
    r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$"
)


def is_valid_utc_timestamp(value: object) -> bool:
    """Match the Rust canonical UTC timestamp parser, including calendar ranges."""

    if not isinstance(value, str) or not UTC_TIMESTAMP_PATTERN.fullmatch(value):
        return False

    year = int(value[0:4])
    month = int(value[5:7])
    day = int(value[8:10])
    hour = int(value[11:13])
    minute = int(value[14:16])
    second = int(value[17:19])

    if year == 0 or not 1 <= month <= 12 or day == 0:
        return False
    if hour > 23 or minute > 59 or second > 59:
        return False

    if month == 2:
        days = 29 if _is_leap_year(year) else 28
    elif month in (4, 6, 9, 11):
        days = 30
    else:
        days = 31
    return day <= days


def _is_leap_year(year: int) -> bool:
    return year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)
