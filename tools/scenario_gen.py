#!/usr/bin/env python3
"""Deterministic dated player actions in FC4HarnessLogic.h's script grammar.

Coordinates are always cells inside the selected tile, including edge tests.
Native placement failures (water, lots, slopes, crossings) are useful evidence;
parser errors are not. Saves affect only the harness's isolated region copy.
"""
import argparse
import calendar
from datetime import date, timedelta
import random
import sys

SIZES = {"small": 64, "medium": 128, "large": 256}
MIXES = ("default", "grow", "churn", "edge")


def add_months(day, months):
    """Calendar months, clamping the day at month end (never approximate days)."""
    year, month = divmod(day.year * 12 + day.month - 1 + months, 12)
    return date(year, month + 1, min(day.day, calendar.monthrange(year, month + 1)[1]))


def iso_date(value):
    try:
        parsed = date.fromisoformat(value)
        if parsed.isoformat() != value:
            raise ValueError()
        return parsed
    except ValueError:
        raise argparse.ArgumentTypeError("expected YYYY-MM-DD") from None


def generate(seed, size, start, months, mix="default"):
    """Return a complete script. No filesystem, environment or global RNG input."""
    if size not in SIZES or mix not in MIXES:
        raise ValueError("unknown size or mix")
    if not isinstance(months, int) or not 1 <= months <= 1200:
        raise ValueError("months must be 1..1200")
    if not isinstance(start, date):
        raise ValueError("start must be a date")
    try:
        end = add_months(start, months)
    except ValueError:
        raise ValueError("end date is outside years 0001..9999") from None
    rng = random.Random(seed)
    side = SIZES[size]
    last = side - 1
    # A 33-cell grid fits every tile. Its blocks are 7x7, with zones directly
    # beside a road/street, and spare cells for parallel utilities.
    ox, oz = (rng.randint(8, side - 41) for _ in range(2))
    zone_types = list(range(16))
    rng.shuffle(zone_types)
    lines = [
        "# scenario_gen v2: seed={} size={} start={} months={} mix={}".format(
            seed, size, start.isoformat(), months, mix),
        "# Cells stay inside the tile. Native refusals are expected in edge/churn.",
        "# pause/resume, speed 0/2 and turbo off/on share a date to keep time advancing.",
        "# Disaster starts run on the final date: native emergency mode can stop new days.",
    ]

    def emit(day, action):
        lines.append("{} {}".format(day.isoformat(), action))

    def rectangle():
        x, z = rng.randrange(side), rng.randrange(side)
        return x, z, min(last, x + rng.randint(1, 6)), min(last, z + rng.randint(1, 6))

    def zone(day, op, rect, kind=None):
        text = "{} {} {} {} {}".format(op, *rect)
        emit(day, text if kind is None else text + " " + str(kind))

    def network(day, ax, az, bx, bz, kind):
        # Reverse endpoints as well: network endpoints need not be ordered.
        if rng.randrange(2):
            ax, az, bx, bz = bx, bz, ax, az
        emit(day, "network {} {} {} {} {}".format(ax, az, bx, bz, kind))

    def grow(day, month):
        if month == 0:
            # Land above sea level, small enough to leave most of a tile alone.
            emit(day, "terrain {} {} {} {} 270 1".format(ox, oz, ox + 32, oz + 32))
            for offset in range(0, 33, 8):
                network(day, ox, oz + offset, ox + 32, oz + offset, 0)
                network(day, ox + offset, oz, ox + offset, oz + 32, 3)
            network(day, ox, oz + 1, ox + 32, oz + 1, 4)  # pipe
            network(day, ox + 1, oz, ox + 1, oz + 32, 5)  # power line
            network(day, ox, oz + 16, ox + 32, oz + 16, 1)  # rail crossing
            network(day, ox + 24, oz, ox + 24, oz + 32, 6)  # avenue
        # Every type, including all R/C/I densities, within four months (or
        # within the only available month for a one-month request).
        batches = min(months, 4)
        for index, kind in enumerate(zone_types):
            if index % batches != month % batches:
                continue
            col, row = index % 4, index // 4
            x, z = ox + col * 8 + 2, oz + row * 8 + 1
            zone(day, "zone", (x, z, x + 3, z + 3), kind)
        # Extend utilities and try new port sites over subsequent months.
        offset = (month % 4) * 8 + 1
        network(day, ox, oz + offset, ox + 32, oz + offset, 4)
        network(day, ox + offset, oz, ox + offset, oz + 32, 5)
        if month % 6 == 0:
            emit(day, "port {} {} {} {} 0".format(
                "airport" if month % 12 == 0 else "seaport",
                rng.randint(0, side - 24), rng.randint(0, side - 24), rng.randrange(4)))

    def churn(day, month):
        rect = rectangle()
        zone(day, "zone", rect, rng.randrange(16))
        zone(day, "zone", rect, rng.randrange(16))  # rezone same cells
        zone(day, "dezone", rect)
        x, z, right, bottom = rect
        network(day, x, z, min(last, right + 8), z, month % 13)
        network(day, right, z, right, min(last, bottom + 8), (month + 3) % 13)
        emit(day, "terrain {} {} {} {} {} {}".format(
            *rect, rng.choice((250, 270, 300, 450)), month % 7))
        emit(day, "disaster survey")
        emit(day, "save" if month % 2 == 0 else "save fast")
        emit(day, "speed {}".format(month % 3))
        emit(day, "pause")
        emit(day, "resume")
        emit(day, "speed 2")
        emit(day, "turbo off")
        emit(day, "turbo on")
        emit(day, "clockmode {}".format(month % 3))
        emit(day, "lightinghour {}".format((month + rng.randrange(24)) % 24))

    def edge(day, month):
        corner = rng.randrange(4)
        x, z = (0 if corner & 1 else last - 4), (0 if corner & 2 else last - 4)
        rect = (x, z, x + 4, z + 4)
        # Deterministically create water and a steep face before placement.
        emit(day, "terrain {} {} {} {} 0 {}".format(*rect, month % 7))
        zone(day, "zone", rect, 14)  # landfill on water
        zone(day, "zone", rect, 1)
        emit(day, "terrain {} {} {} {} 5000 {}".format(x, z, x, z + 4, month % 7))
        zone(day, "zone", rect, 3)  # steep terrain / previous zoning or lots
        # Across the tile to the map edge, never an invalid parser coordinate.
        network(day, 0, z, last, z, month % 13)
        network(day, x, 0, x, last, (month + 1) % 13)
        zone(day, "pricezone", (0, 0, last, last), month % 16)
        for bx, bz in ((0, 0), (last, 0), (0, last), (last, last)):
            zone(day, "pricezone", (bx, bz, bx, bz), rng.randrange(16))
        # The grow block is where existing lots may have developed.
        zone(day, "zone", (ox + 2, oz + 1, ox + 5, oz + 4), month % 16)
        zone(day, "pricezone", rectangle(), rng.randrange(16))

    emit(start, "mark seed={} mix={} size={} begin".format(seed, mix, size))
    emit(start, "speed 2")
    emit(start, "resume")
    emit(start, "turbo on")
    for month in range(months):
        day = add_months(start, month)
        stop = add_months(start, month + 1)
        emit(day, "mark seed={} mix={} month={}".format(seed, mix, month))
        phases = ("grow", "churn", "edge") if mix == "default" else (mix,)
        # A fixed topology-before-zoning order, while mixing all three families
        # within each month. Jitter dates without ever spilling out of a month.
        for index, phase in enumerate(phases):
            offset = index * max(1, (stop - day).days // 3) + rng.randrange(3)
            when = min(stop - timedelta(days=1), day + timedelta(days=offset))
            {"grow": grow, "churn": churn, "edge": edge}[phase](when, month)
        emit(stop - timedelta(days=1), "digest")
    if mix in ("churn", "default"):
        # Riot mode can hold EmergencyPause until player intervention. Exercise
        # all start forms only once the requested months have elapsed. At the
        # end-date new-day callback the harness still drains due actions and
        # deferred saves before quitting; no future SimNewDay is needed here.
        for disaster in ("fire", "riot"):
            for target in ("{} {}".format(rng.randrange(side), rng.randrange(side)), "auto"):
                emit(end, "disaster survey")
                emit(end, "disaster {} {}".format(disaster, target))
    emit(end, "mark seed={} mix={} complete".format(seed, mix))
    return "\n".join(lines) + "\n"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--size", choices=SIZES, required=True)
    parser.add_argument("--start", type=iso_date, required=True)
    parser.add_argument("--months", type=int, required=True)
    parser.add_argument("--mix", choices=MIXES, default="default")
    args = parser.parse_args(argv)
    try:
        text = generate(**vars(args))
    except ValueError as error:
        parser.error(str(error))
    sys.stdout.buffer.write(text.encode("utf-8"))


if __name__ == "__main__":
    main()
