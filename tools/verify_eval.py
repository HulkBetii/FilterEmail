#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import sys
from pathlib import Path


DETAIL_COLUMNS = [
    "email",
    "final_status",
    "dns_status",
    "smtp_outcome",
    "smtp_basic_code",
    "smtp_enhanced_code",
    "smtp_reply_text",
    "mx_host",
    "catch_all",
    "smtp_cached",
    "tested_at",
]

REVIEW_COLUMNS = DETAIL_COLUMNS + [
    "review_ground_truth",
    "review_confidence",
    "review_notes",
]

VALID_GROUND_TRUTH = {"alive", "dead", "unknown", "skip", ""}


def prepare_review(detail_csv: Path, output_csv: Path) -> int:
    with detail_csv.open(newline="", encoding="utf-8") as src:
        reader = csv.DictReader(src)
        missing = [column for column in DETAIL_COLUMNS if column not in (reader.fieldnames or [])]
        if missing:
            raise SystemExit(
                f"Detail CSV is missing required columns: {', '.join(missing)}"
            )

        rows = []
        for row in reader:
            review_row = {column: row.get(column, "") for column in DETAIL_COLUMNS}
            review_row["review_ground_truth"] = ""
            review_row["review_confidence"] = ""
            review_row["review_notes"] = ""
            rows.append(review_row)

    with output_csv.open("w", newline="", encoding="utf-8") as dst:
        writer = csv.DictWriter(dst, fieldnames=REVIEW_COLUMNS)
        writer.writeheader()
        writer.writerows(rows)

    print(f"Wrote review template: {output_csv}")
    print(f"Rows: {len(rows)}")
    return 0


def format_percent(numerator: int, denominator: int) -> str:
    if denominator == 0:
        return "n/a"
    return f"{(numerator / denominator) * 100:.2f}%"


def score_review(review_csv: Path) -> int:
    with review_csv.open(newline="", encoding="utf-8") as src:
        reader = csv.DictReader(src)
        rows = list(reader)

    reviewed = []
    invalid_truth = []
    for index, row in enumerate(rows, start=2):
        truth = row.get("review_ground_truth", "").strip().lower()
        if truth not in VALID_GROUND_TRUTH:
            invalid_truth.append((index, truth))
            continue
        if truth in {"", "skip"}:
            continue
        reviewed.append(row)

    if invalid_truth:
        details = ", ".join(f"line {line}: {value!r}" for line, value in invalid_truth[:10])
        raise SystemExit(f"Invalid review_ground_truth values found: {details}")

    reviewed_count = len(reviewed)
    predicted_alive = [row for row in reviewed if row.get("final_status", "") == "Alive"]
    predicted_dead = [row for row in reviewed if row.get("final_status", "") == "Dead"]
    predicted_actionable = [
        row for row in reviewed if row.get("final_status", "") in {"Alive", "Dead"}
    ]

    correct_alive = [
        row for row in predicted_alive if row.get("review_ground_truth", "").strip().lower() == "alive"
    ]
    correct_dead = [
        row for row in predicted_dead if row.get("review_ground_truth", "").strip().lower() == "dead"
    ]

    print("Verify Evaluation")
    print(f"- Reviewed rows: {reviewed_count}")
    print(
        f"- Alive precision: {format_percent(len(correct_alive), len(predicted_alive))} "
        f"({len(correct_alive)}/{len(predicted_alive)})"
    )
    print(
        f"- Dead precision: {format_percent(len(correct_dead), len(predicted_dead))} "
        f"({len(correct_dead)}/{len(predicted_dead)})"
    )
    print(
        f"- Coverage: {format_percent(len(predicted_actionable), reviewed_count)} "
        f"({len(predicted_actionable)}/{reviewed_count})"
    )

    unknown_rows = [row for row in reviewed if row.get("final_status", "") == "Unknown"]
    print(f"- Unknown rows in reviewed sample: {len(unknown_rows)}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Prepare and score verification review sheets from 33_T4_FINAL_Detail.csv."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    prepare = subparsers.add_parser(
        "prepare", help="Create a review CSV template from 33_T4_FINAL_Detail.csv"
    )
    prepare.add_argument("detail_csv", type=Path)
    prepare.add_argument("output_csv", type=Path)

    score = subparsers.add_parser(
        "score", help="Compute Alive precision, Dead precision, and Coverage"
    )
    score.add_argument("review_csv", type=Path)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    if args.command == "prepare":
        return prepare_review(args.detail_csv, args.output_csv)
    if args.command == "score":
        return score_review(args.review_csv)

    parser.error("Unknown command")
    return 2


if __name__ == "__main__":
    sys.exit(main())
