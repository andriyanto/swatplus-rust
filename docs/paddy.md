# Rice paddies (phase 4)

SWAT+ models a rice paddy as a wetland on the HRU (`surf_stor` in `hru-data.hru`) whose
hydrology (`hydrology.wet`) is named `paddy`. Its water is held by a weir (`weir.res`, raised and
lowered by decision tables), topped up by ponding irrigation, and spills over the weir; puddling
mixes the soil under the water. The port follows SWAT+ commit 6f51536 bit for bit; what it does
and its quirks are in [`swat-quirks.md`](swat-quirks.md) ("Rice paddies"), the two bugs
`--fix-swat` corrects in [`swat-fixes.md`](swat-fixes.md).

## Validation

| Scenario | What | Result |
|---|---|---|
| `Osu_1hru` | SWAT+'s own paddy example (one 10 ha HRU in Korea, 2010–2023, 3 years of warm-up): weir set at random in a window (`prob_unif`), ponding irrigation, puddling, rice transplanted and harvested by decision tables, manure (`manu`), hard calibration of 13 parameters | every table SWAT+ prints, daily to average annual, identical |
| `Ps_paddy` | Ps_coast (Pessel's coastal lowland, West Sumatra) with its rice HRUs as paddies, two seasons a year, 2015–2017 | every table identical, crop yields too |

## Ps_paddy: the paddy management

`scripts/make-pessel-variants.py` builds it from Ps_coast. Pessel's coastal cut has two rice
HRUs (25 ha); the same management can be put on all 248 rice HRUs of Pessel. The values are
typical of irrigated lowland rice in Indonesia, from the sources below; where no local source
gives a value, the choice is marked as an assumption.

| Setting | Value | Basis |
|---|---|---|
| Seasons | two a year (IP 200): the weir raised on a day drawn between 14 Feb and 10 Mar, and between 1 and 23 August | assumption: West Sumatra's two main planting seasons (no planting calendar for Pesisir Selatan was found) |
| Ponding | weir at 50 mm during a season; irrigated back to 50 mm when the water falls under 30 mm (at most 30 times a year); drained (weir 0) at 85 % of the crop's heat units, harvested at 100 % | shallow ponding of 2.5–5 cm keeps yields (Sulistyono & Hayati 2016); the weir-and-ponding scheme of SWAT+'s paddy module (Jeong) |
| Between seasons | 20 mm behind the bunds (weir.res's first row is every wetland's weir; 20 mm is Pessel's principal depth, so its other wetlands are unchanged) | SWAT+ reads one weir for all wetlands |
| Percolation through the plough pan | 0.15 mm/h (3.6 mm/day) | puddled paddies percolate a few mm a day; 5 mm/day used for paddies in a SWAT study of the Upper Citanduy |
| Puddling | once per season when the paddy is flooded: `med_eff` of SWAT+'s `puddle.ops` (sediment 10,000 ppm in the water), tillage of 98 % mixing to 150 mm | SWAT+ paddy example |
| Crop | `rice120`, 120-day variety, transplanted into a flooded, puddled paddy | common inbred varieties (Ciherang, Inpari) mature in 115–125 days |
| Nitrogen | urea 250 kg/ha ≈ 115 kg N/ha in three splits of 38 kg N: basal incorporated (`inject`), top dressings at 25 % and 50 % of the heat units broadcast on the water (`side_dress`) | general recommendation for West Sumatra of 150–250 kg urea/ha, top dressing guided by the leaf colour chart (Atman et al.) |
| Phosphorus | SP-36 100 kg/ha ≈ 15.7 kg P/ha, basal | 100 kg SP-36/ha for a 7 t/ha target (Atman et al.) |

The top dressings fall on standing water. With the original carbon model, SWAT+ loses fertilizer
spread on a flooded paddy (it reaches no pool); `--fix-swat` puts it in the water, from where it
seeps to the soil. On Ps_paddy that raises the grain yield from 4.11–4.60 to 4.12–4.73 t/ha a
season (SWAT+ itself: 4.11, 4.60 and 4.59 t/ha in 2015–2017, within the national mean of about
5 t/ha of milled dry grain). The water balance of a paddy over a year: 2,700–3,700 mm of rain,
140–270 mm of irrigation, 990–1,130 mm of evapotranspiration, 340–410 mm of percolation, the rest
over the weir.

## Sources

- Jeong, J. *SWAT+ paddy module* (SWAT conference, Aarhus).
  https://swat.tamu.edu/media/wjcfpwrd/swatpluspaddy-aarhus-jeong.pdf
- Sulistyono, E. & Hayati, N. (2016). Ponding depth and yield of lowland rice (Subang, West
  Java), *Agrovigor*: ponding at −2.5, 0 and +2.5 cm; −2.5 cm did not lower the yield.
- Atman, Yardha & Salwati. *Pengelolaan hara P dan K pada padi sawah dengan metode omission plot*
  (BPTP Sumatera Barat; trial in Solok, 2004).
  https://repository.pertanian.go.id/bitstreams/318a4969-6046-4074-9f58-b37794a49577/download
- Modified SWAT with a paddy module for the Upper Citanduy (West Java), *Jurnal Sumberdaya Alam
  dan Lingkungan* (IPB): ponding up to 150 mm, percolation 5 mm/day.
  https://journal.ipb.ac.id/jsil/article/download/62629/30557
- Modified SWAT with a paddy module for the Maros sub-watershed (IPB repository).
  https://repository.ipb.ac.id/handle/123456789/107226
