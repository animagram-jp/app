# python3 -c "from dice import example_judge; example_judge(display=True)"

import random


def dice(level: int = 0):
    arg2 = random.randint(0, 9)
    offset = 1 if arg2 == 0 else 0
    rolls = [random.randint(0, 9) + offset for _ in range(1 + abs(level))]
    if level > 0:
        arg1 = min(rolls)
    elif level < 0:
        arg1 = max(rolls)
    else:
        arg1 = rolls[0]

    return arg1 * 10 + arg2

def judge(target: int = 50, level: int = 0) -> bool:
    if not isinstance(target, int):
        raise TypeError(f"target must be an integer, got {type(target).__name__}")
    if not isinstance(level, int):
        raise TypeError(f"level must be an integer, got {type(level).__name__}")
    if not (1 <= target <= 100):
        raise ValueError(f"target must be between 1 and 100, got {target}")

    result = dice(level)
    return result <= target


def example_judge(trials: int = 100000, display: bool = False):
    targets = [5, 10, 25, 40, 50, 60, 75, 99]
    levels  = [-3, -2, -1, 0, 1, 2, 3]

    matrix = []
    for target in targets:
        row = []
        for level in levels:
            hits = sum(judge(target, level) for _ in range(trials))
            row.append(round(hits / trials, 4))
        matrix.append(row)

    if display:
        RESET  = "\033[0m"
        BOLD   = "\033[1m"
        DIM    = "\033[2m"
        # 値に応じて緑(高)→黄→赤(低)でグラデーション
        def color(v: float) -> str:
            if v >= 0.75: return "\033[92m"    # 明緑
            if v >= 0.50: return "\033[32m"    # 緑
            if v >= 0.25: return "\033[33m"    # 黄
            return "\033[91m"                   # 赤

        col_w  = 8
        tgt_w  = 8
        h_cols = "".join(f"{BOLD}L{l:+d}{RESET}".center(col_w + 9) for l in levels)
        header = f"{DIM}{'target':>{tgt_w}}{RESET} │ {h_cols}"
        sep    = f"{'─' * tgt_w}─┼─{'─' * (col_w * len(levels) + len(levels) - 1 + 18 * len(levels))}"

        print()
        print(f"  {BOLD}judge() True 確率マトリクス{RESET}  ({trials:,} trials/cell)")
        print(f"  {DIM}level: 負=高い目(max), 0=フラット, 正=低い目(min){RESET}")
        print()
        print(f"  {header}")
        print(f"  {DIM}{sep}{RESET}")
        for target, row in zip(targets, matrix):
            cells = " ".join(
                f"{color(v)}{v * 100:5.1f}%{RESET}".rjust(col_w + 9)
                for v in row
            )
            print(f"  {BOLD}{target:>{tgt_w}}{RESET} │ {cells}")
        print()

    return matrix
