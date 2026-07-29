#!/usr/bin/env python3
"""Fetch tomorrow's weather forecast for Seoul without an API key."""

from __future__ import annotations

import json
import sys
from datetime import datetime, timedelta
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import urlopen
from zoneinfo import ZoneInfo


SEOUL_LATITUDE = 37.5665
SEOUL_LONGITUDE = 126.9780
SEOUL_TIMEZONE = ZoneInfo("Asia/Seoul")
FORECAST_URL = "https://api.open-meteo.com/v1/forecast"
WEATHER_CODES = {
    0: "맑음",
    1: "대체로 맑음",
    2: "부분적으로 흐림",
    3: "흐림",
    45: "안개",
    48: "서리 안개",
    51: "약한 이슬비",
    53: "이슬비",
    55: "강한 이슬비",
    56: "약한 어는 이슬비",
    57: "강한 어는 이슬비",
    61: "약한 비",
    63: "비",
    65: "강한 비",
    66: "약한 어는 비",
    67: "강한 어는 비",
    71: "약한 눈",
    73: "눈",
    75: "강한 눈",
    77: "싸락눈",
    80: "약한 소나기",
    81: "소나기",
    82: "강한 소나기",
    85: "약한 눈 소나기",
    86: "강한 눈 소나기",
    95: "뇌우",
    96: "약한 우박을 동반한 뇌우",
    99: "강한 우박을 동반한 뇌우",
}


def tomorrow_in_seoul() -> str:
    """Return tomorrow's date in Seoul as an ISO date string."""
    return (datetime.now(SEOUL_TIMEZONE).date() + timedelta(days=1)).isoformat()


def fetch_forecast(target_date: str) -> dict[str, object]:
    """Fetch the daily forecast for the requested date from Open-Meteo."""
    query = urlencode(
        {
            "latitude": SEOUL_LATITUDE,
            "longitude": SEOUL_LONGITUDE,
            "daily": (
                "weather_code,temperature_2m_max,temperature_2m_min,"
                "precipitation_sum,precipitation_probability_max"
            ),
            "timezone": "Asia/Seoul",
            "start_date": target_date,
            "end_date": target_date,
        }
    )
    with urlopen(f"{FORECAST_URL}?{query}", timeout=10) as response:
        return json.load(response)


def first_daily_value(daily: dict[str, object], key: str) -> object:
    """Return the first value for a daily forecast field."""
    values = daily.get(key)
    if not isinstance(values, list) or not values:
        raise ValueError(f"예보 응답에 {key!r} 값이 없습니다.")
    return values[0]


def format_forecast(payload: dict[str, object]) -> str:
    """Format an Open-Meteo response for terminal output."""
    daily = payload.get("daily")
    if not isinstance(daily, dict):
        raise ValueError("예보 응답에 daily 데이터가 없습니다.")

    weather_code = int(first_daily_value(daily, "weather_code"))
    description = WEATHER_CODES.get(weather_code, f"알 수 없음(코드 {weather_code})")
    date = first_daily_value(daily, "time")
    minimum = float(first_daily_value(daily, "temperature_2m_min"))
    maximum = float(first_daily_value(daily, "temperature_2m_max"))
    precipitation = float(first_daily_value(daily, "precipitation_sum"))
    probability = int(first_daily_value(daily, "precipitation_probability_max"))

    return "\n".join(
        [
            f"서울 내일 날씨 ({date}, Asia/Seoul)",
            f"날씨: {description}",
            f"기온: 최저 {minimum:.1f}°C / 최고 {maximum:.1f}°C",
            f"강수: {precipitation:.1f} mm / 최대 확률 {probability}%",
        ]
    )


def main() -> int:
    target_date = tomorrow_in_seoul()
    try:
        payload = fetch_forecast(target_date)
        print(format_forecast(payload))
    except (HTTPError, URLError, TimeoutError, json.JSONDecodeError, ValueError) as error:
        print(f"날씨 조회 실패: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
