# Verify Evaluation

This workflow does not change backend logic. It is only for measuring verify quality after a run.

## The 3 metrics

- `Alive precision`
  - Of all rows the tool marked `Alive`, how many are truly alive in your audited sample.
  - Formula: `true_alive_predictions / all_alive_predictions_reviewed`

- `Dead precision`
  - Of all rows the tool marked `Dead`, how many are truly dead in your audited sample.
  - Formula: `true_dead_predictions / all_dead_predictions_reviewed`

- `Coverage`
  - Of all reviewed rows, how many the tool was willing to classify as `Alive` or `Dead` instead of `Unknown`.
  - Formula: `(tool_alive + tool_dead) / all_reviewed_rows`

## Input files

The tool already generates:

- `33_T4_FINAL_Detail.csv`

This file is the source for the review sheet.

## Step 1: Prepare a review sheet

```bash
python3 scripts/eval/verify_eval.py prepare \
  /path/to/33_T4_FINAL_Detail.csv \
  /path/to/verify_review.csv
```

The generated review CSV keeps all tool output columns and adds:

- `review_ground_truth`
- `review_confidence`
- `review_notes`

## Step 2: Fill the review columns

Use these values for `review_ground_truth`:

- `alive`
- `dead`
- `unknown`
- `skip`

Recommended use:

- `alive`: you have strong evidence the exact email is alive
- `dead`: you have strong evidence the exact email is dead
- `unknown`: you still cannot conclude manually
- `skip`: exclude this row from the scored sample

`review_confidence` is free text. Recommended values:

- `high`
- `medium`
- `low`

## Step 3: Score the sample

```bash
python3 scripts/eval/verify_eval.py score /path/to/verify_review.csv
```

Example output:

```text
Verify Evaluation
- Reviewed rows: 100
- Alive precision: 94.00% (47/50)
- Dead precision: 96.00% (24/25)
- Coverage: 75.00% (75/100)
- Unknown rows in reviewed sample: 25
```

## Interpretation

- High `Alive precision` means the tool is conservative enough when it says `Alive`.
- High `Dead precision` means the tool is not incorrectly killing good addresses too often.
- `Coverage` tells you how often the tool can make a hard decision instead of returning `Unknown`.

This is the intended decision model:

- `Alive` should be high-confidence only.
- `Dead` should be hard-failure only.
- everything uncertain should stay `Unknown`.
