"""Builds Waypoint's built-in place lists (data/*.tsv).

Run with the raw downloads in one folder:
  airports.dat and airlines.dat from https://github.com/jpatokal/openflights (data/),
  package/data/cityMap.json from the city-timezones npm package (npm pack city-timezones@1.3.1).
  python3 build_places.py RAW_FOLDER OUT_FOLDER
Needs /usr/share/zoneinfo/tzdata.zi for old zone names.
"""
import csv, json, sys
raw, out = sys.argv[1], sys.argv[2]
# Old zone names (links) to their current ones, so every list uses names that
# every time zone database has.
LINKS = {}
with open("/usr/share/zoneinfo/tzdata.zi", encoding="utf-8") as f:
    for line in f:
        if line.startswith("L "):
            _, target, link = line.split()
            LINKS[link] = target
canon = lambda z: LINKS.get(z, z)
clean = lambda s: str(s if s is not None else "").replace("\t", " ").replace("\\N", "").strip()
air = []
with open(f"{raw}/airports.dat", encoding="utf-8") as f:
    for r in csv.reader(f):
        iata, tz = clean(r[4]), clean(r[11])
        if len(iata) == 3 and iata.isalpha() and "/" in tz and r[12] == "airport":
            air.append((iata.upper(), clean(r[1]), clean(r[2]), clean(r[3]), canon(tz)))
# Where the airport's city is in the city list with the same clock all year,
# use the city's zone name (OpenFlights gives Melbourne airport Australia/Hobart).
from zoneinfo import ZoneInfo
from datetime import datetime
def same_clock(a, b):
    try:
        za, zb = ZoneInfo(a), ZoneInfo(b)
    except Exception:
        return False
    for m in range(1, 13):
        for d in (1, 15):
            t = datetime(2027, m, d, 12)
            if t.replace(tzinfo=za).utcoffset() != t.replace(tzinfo=zb).utcoffset():
                return False
    return True
cities_raw = json.load(open(f"{raw}/package/data/cityMap.json", encoding="utf-8"))
norm = lambda c: {"United States of America": "United States"}.get(c, c)
best = {}
for c in cities_raw:
    k = (str(c["city"]).lower(), norm(str(c.get("country"))))
    if k not in best or (c.get("pop") or 0) > (best[k].get("pop") or 0):
        best[k] = c
fixed = 0
for n, a in enumerate(air):
    c = best.get((a[2].lower(), a[3]))
    if c and c.get("timezone") and canon(c["timezone"]) != a[4] and same_clock(canon(c["timezone"]), a[4]):
        air[n] = a[:4] + (canon(c["timezone"]),)
        fixed += 1
print("airport zones renamed from their city:", fixed)
air.sort()
with open(f"{out}/airports.tsv", "w", encoding="utf-8") as f:
    f.write("# OpenFlights airports (ODbL, openflights.org/data): code, name, city, country, time zone\n")
    for a in air: f.write("\t".join(a) + "\n")
lines = {}
with open(f"{raw}/airlines.dat", encoding="utf-8") as f:
    for r in csv.reader(f):
        code, name, active = clean(r[3]), clean(r[1]), r[7]
        if len(code) == 2 and code.isalnum() and active == "Y" and name and code.upper() not in lines:
            lines[code.upper()] = name
with open(f"{out}/airlines.tsv", "w", encoding="utf-8") as f:
    f.write("# OpenFlights airlines (ODbL, openflights.org/data), active ones: code, name\n")
    for c in sorted(lines): f.write(f"{c}\t{lines[c]}\n")
cities = json.load(open(f"{raw}/package/data/cityMap.json", encoding="utf-8"))
rows = []
for c in cities:
    tz = canon(clean(c.get("timezone")))
    if "/" not in tz: continue
    rows.append((clean(c["city"]), clean(c.get("city_ascii")), clean(c.get("country")), clean(c.get("iso2")), clean(c.get("province")), str(int(c.get("pop") or 0)), tz))
rows.sort(key=lambda r: (r[0].lower(), -int(r[5])))
with open(f"{out}/cities.tsv", "w", encoding="utf-8") as f:
    f.write("# Natural Earth populated places (public domain) via city-timezones 1.3.1 (MIT): city, ascii name, country, country code, province, population, time zone\n")
    for r in rows: f.write("\t".join(r) + "\n")
print(len(air), "airports;", len(lines), "airlines;", len(rows), "cities")
