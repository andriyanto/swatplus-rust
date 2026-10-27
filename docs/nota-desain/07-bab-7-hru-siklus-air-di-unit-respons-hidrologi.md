## Bab 7  HRU: siklus air di unit respons hidrologi

Bab ini membahas berkas terbesar engine, `crates/swat-engine/src/hru/mod.rs` (sekitar 3.100 baris). Berkas ini memuat dua hal: (1) penyiapan HRU sebagaimana SWAT+ melakukannya di `proc_hru` (gabungan `hrudb_init`, `topohyd_init`, `soils_init`, `soil_phys_init`, `plant_init`, `cn2_init`, `hydro_init`) dan `time_conc_init`, termasuk penerapan `calibration.cal` ke parameter HRU dan lapisan tanah; dan (2) neraca air harian HRU yang di Fortran tersebar di `hru_control.f90` beserta subrutin `sq_*` (limpasan), `swr_*` (air tanah-profil), `ero_*` (erosi), `et_pot`/`et_act` (evapotranspirasi), `stmp_solt` (suhu tanah), dan `hru_urban`/`hru_urbanhr` (beban perkotaan). Submodul `bmp`, `carbon`, `mgt`, `nutrient`, `plant`, `septic`, `subdaily`, dan `wetland` di direktori yang sama dibahas di bab masing-masing; di sini mereka hanya disebut sejauh dipanggil oleh `Hru::day`.

Dalam alur simulasi, `init_hrus` dipanggil sekali oleh runner setelah generator cuaca diinisialisasi; lalu setiap hari runner memanggil `Hru::day` untuk setiap HRU dengan cuaca HRU (`HruWeather`), konteks hari (`DayCtx`), dan pengaturan DAS (`HruBasin`). Hasilnya, `HruDay`, berisi empat tabel harian (`hru_wb`, `hru_pw`, `hru_nb`, `hru_ls`) dan hidrograf keluaran HRU (`hd`: total, imbuhan, permukaan, lateral, tile) yang kemudian dirutekan ke objek hilir. Urutan operasi dalam `Hru::day` mengikuti `hru_control.f90` secara ketat karena kesamaan bit-for-bit bergantung padanya.

Semua besaran disimpan dalam `f32` (padanan `real` Fortran) kecuali dinyatakan lain. Satuan air adalah mm di atas luas HRU; massa adalah kg/ha.

### 7.1  Struktur data HRU dan pengaturan DAS

Subbab ini mencakup tipe-tipe data yang dipakai seluruh berkas serta method kecilnya.

**`Layer`** (padanan `soil(j)%phys(ly)` dan `soil(j)%ly(ly)`): satu lapisan tanah.

| Field | Arti | Satuan |
| --- | --- | --- |
| `d`, `thick` | kedalaman dasar lapisan, tebal lapisan | mm |
| `bd`, `awc`, `k` | bulk density, kapasitas air tersedia, konduktivitas jenuh | g/cm3, mm/mm, mm/jam |
| `clay`, `silt`, `sand`, `rock` | fraksi tekstur dan batuan | % |
| `alb`, `usle_k`, `cbn`, `cal` | albedo tanah lembap, faktor K USLE, karbon organik, CaCO3 | –, –, %, % |
| `wp`, `up`, `por` | titik layu, kapasitas lapang, porositas | fraksi volume |
| `ul`, `fc`, `st`, `wpmm` | air pada jenuh, air kapasitas lapang di atas titik layu, air tersimpan di atas titik layu, air titik layu | mm |
| `hk` | waktu tempuh perkolasi | jam |
| `tmp` | suhu lapisan | °C |
| `prk`, `flat` | perkolasi keluar lapisan dan aliran lateral dari lapisan hari ini | mm |
| `no3`, `nh4`, `wsol`, `lab`, `act`, `sta` | N mineral dan pool P mineral | kg/ha |
| `hact`, `hsta`, `cp` | humus aktif/stabil (`OrgMass`) dan pool model karbon | kg/ha |
| `psp_store`, `ssp_store`, `a_days`, `b_days` | status model P dinamis (`sol_p_model` 1) | –, hari |
| `init_bmix`, `bmix`, `tillagef`, `tillagef_biomix`, `tillagef_tillmix` | efisiensi pencampuran biologis dan efek olah tanah pada dekomposisi | fraksi |

**`Tile`** (baris `tiledrain.str`): `depth` (mm), `time` (jam untuk menguras tanah ke kapasitas lapang), `drain_co` (koefisien drainase, mm/hari), `ldrain` (lapisan tempat drain, berbasis-1, 0 bila tidak ada), `lag` (jam) dan `ttime` (bagian aliran tile tertahan yang dilepas per hari).

**`Snow`** (baris `snow.sno`): `falltmp` (suhu hujan salju, °C), `melttmp` (suhu ambang leleh, °C), `meltmx`/`meltmn` (faktor leleh maksimum 21 Juni dan minimum 21 Desember, mm/°C/hari), `timp` (faktor lag suhu paket salju), `covmx` (kedalaman salju untuk tutupan 100 %, mm), `cov50` (fraksi `covmx` untuk tutupan 50 %), `init_mm` (salju awal, mm).

**`Hru`**: parameter dan status satu HRU. Kelompok field utama:

| Kelompok | Field | Keterangan |
| --- | --- | --- |
| Topografi | `area_ha`, `km`, `elev`, `slope`, `slope_len`, `lat_len`, `field_wid`, `dep_co` | luas (ha, km2), elevasi (m), lereng (m/m), panjang lereng dan lateral (m) |
| Hidrologi | `esco`, `epco`, `cn3_swf`, `perco`, `perco_lim`, `latq_co`, `pet_co`, `lat_ttime`, `brt`, `canmx`, `ovn`, `tconc`, `ffc`, `anion_excl`, `hyd_biomix` | dari `hydrology.hyd`, `landuse.lum`, dll. |
| Bilangan kurva | `cn2`, `smx`, `wrt[2]`, `hydgrp` | CN kondisi II, retensi maksimum (mm), koefisien kurva retensi, grup hidrologi tanah |
| Status air | `layers`, `sumul`, `sumfc`, `sw`, `sw_300`, `avbd`, `zmx`, `sno_mm`, `canstor`, `surf_bs`, `bss`, `bss_tile`, `qtile` | mm kecuali `avbd` (g/cm3) dan `zmx` (kedalaman profil, mm) |
| Salju | `sno`, `snocov1`, `snocov2` | parameter dan koefisien kurva tutupan salju |
| Erosi | `usle_mult`, `usle_p`, `usle_ls`, `usle_cfac`, `surf_bs_sed[7]`, `surf_bs_cls[5]`, `erorgn`, `erorgp` | faktor gabungan USLE dan simpanan lag sedimen beserta nutrien terbawa |
| Tanaman | `plants`, `lai_sum`, `laimx_sum`, `cht_mx`, `tot_com`, `ab_gr_com`, `rsd_tot`, `phubase` | komunitas tanaman dan residu |
| Manajemen | `mgt`, `till_switch`, `till_days`, `till_depth`, `irr_applied`, `irr_runoff`, `irr_yr`, `fertn`, `fertp`, `pl_yield`, `harvests`, `days_plant`, `days_harv`, `days_kill`, `days_irr`, `irr_hmax`, `irr_hmin` | status jadwal operasi |
| Objek lain | `urban`, `twash`, `wet`, `wet_hyd_row`, `water_evap`, `tile`, `bmps`, `septic`, `sep_opt`, `sub` | perkotaan, lahan basah, drain, BMP, septik, langkah sub-harian |
| Kalibrasi/keluaran | `texture`, `lum_name`, `cal_group`, `sw_init`, `sno_init`, `lat`, `wgn`, `atmo_prev` | atribut kondisi `calibration.cal` dan status cetak |

**`HruWb`, `HruPw`, `HruNb`, `HruLs`**: satu baris harian dari `hru_wb_day`, `hru_pw_day`, `hru_nb_day`, dan `hru_ls_day` (hanya kolom yang dihitung port). **`HruDay`** membungkus keempatnya, ditambah `nutrients` (salah bila model karbon yang belum di-port), `hd: [Hyd; 5]` (padanan `ob%hd(1..5)` dari `hru_hyds`), `harvests`, dan `hhsurfq` (limpasan permukaan per langkah waktu setelah lag; kosong bila harian). **`Runon`** menampung `ob%hin_sur`, `hin_lat`, `hin_til` per hektar dari routing lanskap. **`DayCtx`** membawa hari dalam tahun, bulan, suhu tahunan, `phutot`, panjang hari dan `dormhr`, `precip_half_hr` (fraksi hujan puncak setengah jam untuk MUSLE), hujan stasiun, akhir tahun, dan `runon`. **`HruBasin`** membawa kode `pet`, `cn_froz`, `cswat`, `Biomix`, `PlantBasin`, `NutBasin`, `CarbonBasin`, `SubBasin`, dan bendera `fixes`. **`Biomix`** membawa `eff`, `depth` (mm), dan parameter faktor suhu `tmpf`, `tn`, `top`, `tx` dari `carbon.bsn`.

#### 7.1.1  `HruBasin::new`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn new(project: &Project) -> HruBasin`
- **Padanan SWAT+:** pembacaan `codes.bsn` dan `parameters.bsn` (`basin_read_cc`, `basin_prm_read`); tidak ada satu subrutin padanan
- **Tujuan:** Mengumpulkan pengaturan tingkat DAS yang dibaca rutin HRU harian.
- **Masukan dan keluaran:** `project.codes.pet`, `cswat`; `params` `cn_froz` (bawaan 0,000862), `urb_init_abst`, `eros_spl`, `rill_mult`, `eros_expo`, `c_factor` (bawaan 0). Keluaran `HruBasin`.
- **Cara kerja:**
  1. Salin kode PET dan `cswat`.
  2. Bila `cswat` = 2, bangun `Biomix::new` dan `CarbonBasin::new`; selain itu `None`.
  3. Bangun `PlantBasin`, `NutBasin`, dan `SubBasin` (parameter Green-Ampt dan erosi sub-harian).
  4. `fixes` diawali `false`.
- **Kebiasaan SWAT+ / catatan port:** nilai bawaan `cn_froz` 0,000862 sama dengan bawaan Fortran saat parameter tidak ada.

#### 7.1.2  `HruBasin::with_fixes`

- **Lokasi:** `hru/mod.rs`, `fn with_fixes(self, fixes: bool) -> HruBasin`
- **Padanan SWAT+:** tidak ada; khusus port (opsi `--fix-swat`)
- **Tujuan:** Menyebarkan bendera koreksi bug ke `HruBasin`, `PlantBasin`, dan `NutBasin` sekaligus.
- **Cara kerja:** set `self.fixes`, `self.plant.fixes`, `self.nut.fixes` ke nilai yang sama dan kembalikan struct.

#### 7.1.3  `Biomix::new`

- **Lokasi:** `hru/mod.rs`, `fn new(project: &Project) -> Biomix`
- **Padanan SWAT+:** `till_parm_read.f90` (baris bernama `biomix`) dan `carbon_bsn_read.f90`
- **Tujuan:** Menentukan efisiensi dan kedalaman pencampuran biologis serta parameter faktor suhu.
- **Masukan dan keluaran:** tabel `tillage.til` (`tillnm`, `effmix`, `deptil`) dan baris pertama `carbon.bsn` (`tmpf`, `tn`, `top`, `tx`).
- **Cara kerja:**
  1. Tanpa berkas olah tanah: `eff` = 0, `depth` = 0.
  2. Dengan berkas: mulai dari bawaan 0,2 dan 50 mm; telusuri semua baris, dan setiap baris yang namanya sama (perbandingan gaya Fortran, `fortran_eq`) dengan `biomix` menimpa nilai. Jadi baris `biomix` terakhir yang menang.
  3. Baca `tmpf` (bawaan 2), `tn`, `top`, `tx` dari `carbon.bsn` atau baris bawaan skema.
- **Kebiasaan SWAT+ / catatan port:** "baris terakhir menang" meniru loop Fortran yang tidak berhenti saat menemukan nama.

#### 7.1.4  `Biomix::fcgd`

- **Lokasi:** `hru/mod.rs`, `fn fcgd(&self, xx: f32) -> f32`
- **Padanan SWAT+:** fungsi `fcgd` di `fcgd.f90`
- **Tujuan:** Faktor suhu berbentuk kurva beta untuk laju pencampuran biologis (dan dekomposisi pada model karbon).
- **Masukan dan keluaran:** suhu tanah `xx` (°C); parameter `tn` (minimum), `top` (optimum), `tx` (maksimum). Keluaran faktor 0–1.
- **Cara kerja:**
  1. Bila `xx` <= `tn` atau `xx` >= `tx`, kembalikan 0.
  2. Hitung eksponen dan faktor:

```latex
q = \frac{T_n - T_{op}}{T_{op} - T_x},\qquad
f = \frac{(T - T_n)^{q}\,(T_x - T)}{(T_{op} - T_n)^{q}\,(T_x - T_{op})}
```

3. Potong ke 0 bila negatif.

#### 7.1.5  `Biomix::temperature_factor`

- **Lokasi:** `hru/mod.rs`, `fn temperature_factor(&self, emix: f32, stemp: f32) -> f32`
- **Padanan SWAT+:** bagian awal `mgt_newtillmix_wet` / pencampuran biologis pada `cswat` 2 (pemilihan `tmpf`)
- **Tujuan:** Mengoreksi efisiensi pencampuran `emix` menurut suhu tanah.
- **Cara kerja:** bila `tmpf` = 3, gunakan fungsi sigmoid; selain itu kalikan dengan `fcgd`:

```latex
f_{mix} = \begin{cases} e_{mix}\left(0.9\,\dfrac{T}{T + \exp(9.93 - 0.312\,T)} + 0.1\right) & t_{mpf}=3 \\ e_{mix}\,\mathrm{fcgd}(T) & \text{lainnya} \end{cases}
```

#### 7.1.6  `Row::find_or_first`

- **Lokasi:** `hru/mod.rs`, `fn find_or_first(table: Option<&Table>, fields, name: &str) -> (Row, bool)`
- **Padanan SWAT+:** pencarian pointer database di `hru_read`/`hrudb_init` (pointer bawaan 1)
- **Tujuan:** Mencari baris tabel menurut nama untuk data yang dirujuk `hru-data.hru`; bila nama tidak ditemukan (termasuk `null` dari Editor), memakai baris pertama tabel.
- **Cara kerja:**
  1. Panggil `Row::find`; bila ditemukan, kembalikan.
  2. Bila tidak, ambil baris pertama tabel bila ada (`true`); bila tabel kosong atau hilang, kembalikan baris bawaan dengan `false`.
- **Kebiasaan SWAT+ / catatan port:** meniru perilaku Fortran: pointer yang tak ditemukan tetap bernilai awal 1, sehingga HRU diam-diam memakai baris pertama.

#### 7.1.7  `Row::find`

- **Lokasi:** `hru/mod.rs`, `fn find(table, fields, name) -> (Row, bool)`
- **Padanan SWAT+:** loop pencarian nama di rutin baca database
- **Tujuan:** Mencari baris bernama; bila tidak ada, mengembalikan baris bawaan skema (padanan elemen 0 array Fortran yang berisi nilai bawaan) dengan `false`.

#### 7.1.8  Fungsi pembantu trivial

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `HruPw::columns` | `hru/mod.rs` | 19 pasangan (nama kolom, nilai) untuk `hru_pw_day.txt` (`lai`, `bioms`, `yield`, …, `c_gro`) |
| `HruNb::columns` | `hru/mod.rs` | 15 pasangan untuk `hru_nb_day.txt` (`lab_min_p` … `puptake`) |
| `HruLs::columns` | `hru/mod.rs` | 13 pasangan untuk `hru_ls_day.txt`; perhatikan nama kolom `lat3no3` (ejaan SWAT+) untuk `latno3` |
| `HruWb::columns` | `hru/mod.rs` | 32 pasangan untuk `hru_wb_day.txt`; `sw` dicetak sebagai `sw_ave`, `delsw` sebagai `sw_change` |
| `Row::get` | `hru/mod.rs` | nilai field ke-`index` dengan menjumlahkan `count` field sebelumnya (field larik) |
| `Row::real` | `hru/mod.rs` | `get(field,0)` sebagai real, 0 bila tidak ada |
| `Row::text` | `hru/mod.rs` | `get(field,0)` sebagai teks, string kosong bila tidak ada |
| `is_null` | `hru/mod.rs` | benar bila teks (tanpa spasi kanan) adalah `null` |

### 7.2  Penyiapan HRU (`proc_hru`)

Penyiapan dibagi menjadi tiga tahap yang meniru urutan program utama SWAT+: (a) `init_hru` per HRU (padanan `hrudb_init`, `topohyd_init`, `soils_init`, `soil_phys_init`, `plant_init`, `cn2_init`, `hydro_init`, `structure_init`), lalu `init_bmix` dan `time_conc`; (b) `cal_conditions` (padanan `proc_cal`); (c) `nutcarb_stage` (padanan `soil_nutcarb_init`, dipanggil dari program utama setelah kalibrasi). Urutan (b) sebelum (c) penting: nutrien tanah awal dihitung dari tanah yang sudah dikalibrasi.

#### 7.2.1  `init_hrus`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `pub fn init_hrus(project, wgn: &[usize], wgns: &[WgnState], step: i32, ndays: &[i32; 13]) -> Result<Vec<Hru>, String>`
- **Padanan SWAT+:** `proc_hru.f90` (dengan `hrudb_init`, `topohyd_init`, `soils_init`, `soil_phys_init`, `plant_init`, `cn2_init`, `hydro_init`), `time_conc_init.f90`, `cal_conditions.f90`, dan `soil_nutcarb_init`
- **Tujuan:** Membangun seluruh vektor `Hru` dalam keadaan yang sama dengan keadaan SWAT+ sesaat sebelum hari simulasi pertama, atau menolak dengan alasan bila masukan memerlukan bagian SWAT+ yang belum di-port.
- **Masukan dan keluaran:** proyek, indeks generator cuaca per HRU (`wgn`, berbasis-0), status generator (`wgns`), jumlah langkah waktu per hari (`step`), dan jumlah hari kumulatif per bulan (`ndays`). Keluaran vektor HRU atau `Err`.
- **Cara kerja:**
  1. Validasi kode: `cswat` harus 0 atau 2; Green-Ampt (`gampt` != 0) menuntut langkah sub-harian (`step` > 1); `step` maksimum 96; `pet` 0–3; `crk` (aliran retakan) dan `gwflow` harus 0. Pelanggaran menghasilkan `Err` dengan alasan.
  2. Baca `ffcb` (bawaan 0) dan `surlag` (bawaan 4) dari `parameters.bsn`.
  3. Untuk setiap baris `hru-data.hru` (berpasangan dengan objek HRU ke-i di `hru.con`): panggil `init_hru`, simpan `wgn`, panggil `init_bmix` dan `time_conc`.
  4. Bila `step` > 1 (sub-harian), dengan Δt = 1440/step menit:
     - faktor lag limpasan per langkah (`time_conc_init`):

```latex
brt = 1 - \exp\!\left(-\frac{surlag}{t_{conc}/(\Delta t/60)}\right)
```

- pasir lapisan atas dihitung ulang sebagai 100 − liat − debu (`soil_phys_init`) dan potensial matrik muka basah `wfsh` dihitung dengan `subdaily::wfsh` (lihat bab sub-harian/Green-Ampt);
- rata-rata `cvm` (log faktor C minimum) komunitas tanaman disimpan sebagai `cvm_com`;
- struktur `SubDaily` dibuat dengan larik simpanan `bs_q`, `bs_sed` sepanjang `step`.

5. Koefisien evaporasi lahan basah untuk `et_act` (`evrsv_et`) diambil dari lahan basah HRU bernomor sama dengan baris `hydrology.wet` milik HRU ini; bila HRU itu tidak punya lahan basah, 0,7.
6. Panggil `cal_conditions` lalu, per HRU, `nutcarb_stage` dengan `psp_saved` yang dibawa dari HRU ke HRU.

- **Kebiasaan SWAT+ / catatan port:** butir 5 meniru `et_act` yang membaca `wet_hyd(ihyd)` (indeks salah); di `--fix-swat` koefisien lahan basah sendiri yang dipakai (lihat `docs/swat-fixes.md`). Variabel `psp_saved` dan `igrow_saved` meniru variabel Fortran yang bertahan antarpanggilan (`save`): `psp` lapisan tanpa liat mewarisi nilai lapisan terakhir yang dihitung, dan `igrow` (hari tanam) di `plant_init`.

#### 7.2.2  `init_hru`

- **Lokasi:** `hru/mod.rs`, `fn init_hru(project, hd: &HruData, ob: &SpatialObject, g: &WgnState, ffcb: f32, ndays, igrow: &mut i32) -> Result<Hru, String>`
- **Padanan SWAT+:** `hrudb_init.f90`, `wet_initial`, `topohyd_init.f90`, `structure_init`, `soils_init.f90`, `soil_phys_init.f90`, `plant_init.f90`, `cn2_init.f90`, `hydro_init.f90`, `structure_set_parms`
- **Tujuan:** Membangun satu HRU dari baris `hru-data.hru`, objek spasialnya, dan generator cuacanya.
- **Masukan dan keluaran:** nama tabel yang dirujuk (`land_use_mgt`, `topo`, `hyd`, `snow`, `soil`, `surf_stor`, `field`); luas (ha), elevasi, lintang dari `hru.con`; statistik bulanan generator. Keluaran satu `Hru`.
- **Cara kerja:**
  1. **Dasar.** Cari baris `landuse.lum` dengan `Row::find_or_first`. Isi luas (ha dan km2 = ha/100), elevasi, lintang, dan `days_plant`/`days_harv`/`days_kill`/`days_irr` = −366.
  2. **Lahan basah** (`wet_initial`): bila `surf_stor` bukan `null`, bangun `wetland::Wetland` (ditolak bila `cswat` = 2).
  3. **Perkotaan** (`plant_init`): bila `urb_lu` ditemukan di `urban.urb`, isi `Urban` (`fimp`, `fcimp`, `curbden`, `urbcoef`, `dirtmx`, `thalf`, `tnconc`, `tpconc`, `tno3conc`) dan metode `urb_ro`: `buildup_washoff`, `usgs_reg` (kategori hujan tahunan generator < 508, < 1016, atau lebih), atau tidak ada.
  4. **Topografi dan hidrologi** (`topohyd_init`): `slope`, `dep_co`, `slope_len`, `lat_len` (50 m bila < 1e-6), lebar lahan dari `field.fld`; dari `hydrology.hyd`: `lat_ttime`, `canmx`, `erorgn`, `erorgp`, `biomix`, `esco`, `epco`, `cn3_swf`, `perco`. Sedimen lateral (`lat_sed` > 0) ditolak.
  5. **Drain bawah tanah**: bila tata guna lahan menyebut `tiledrain`, cari barisnya di `tiledrain.str` (ditolak bila tidak ada, bila `tdrn` = 1 (DRAINMOD), atau `wtdn` = 1). Isi `Tile` dan paksa `cn3_swf` = 0,95 serta `perco` = 0,1.
  6. **BMP** (`structure_init`): `bmp::init` dengan nama `fstrip`, `grassww`, `bmpuser`.
  7. **Salju**: salin parameter `snow.sno`, hitung koefisien kurva tutupan salju dengan `ascrv(0.5, 0.95, cov50, 0.95)`, dan `sno_mm` = `init_mm`.
  8. **Batas perkolasi** (`topohyd_init`): untuk `perco` > 1e-9,

```latex
f_{ln} = 1.0052\,\ln\!\left(-\ln(perco - 10^{-6})\right) + 5.6862,\qquad perco_{lim} = \min\!\left(e^{-f_{ln}},\,1\right)
```

Selain itu `perco_lim` = 0. `latq_co` dibaca; `pet_co` = 1 bila < 1e-6, dan dibagi 0,0023 bila < 0,01 (konversi koefisien Hargreaves lama).

9. **Tanah**: cari tanah bernama di `soils.sol` (wajib ada) dan panggil `soil_layers`.
10. **Septik**: bila tata guna lahan menyebut sistem septik yang dioperasikan (`opt` != 0), bentuk lapisan biozona (`septic::biozone`) dan baca efisiensi `septic.sep` (10 kolom: `q_rate`, `bod`, `tss`, `nh4_n`, `no3_n`, `no2_n`, `org_n`, `min_p`, `org_p`, `fcoli`). Ditolak bila `cswat` != 0.
11. **Tanaman** (`plant_init`): komunitas `plant_cov` dari `plant.ini` (maksimum 20 tanaman; ditolak dengan `cswat` 1 atau `co2_yr.dat`). Setiap tanaman dicari di `plants.plt` (bila tidak ada, baris pertama), dibangun dengan `PlantDb::new` dan `plant::plant_init` (Bab tanaman), residu dijumlahkan ke `rsd_tot` dan `blai` ke `laimx_sum`.
12. **Manajemen**: jadwal `mgt_ops` di `management.sch` dibangun dengan `mgt::Mgt::new` memakai tahun rotasi awal komunitas; tanpa tanaman, operasi pertama dikembalikan ke indeks 0 (padanan nilai bawaan 1 di Fortran).
13. **Faktor USLE** (`plant_init`): faktor P dari `cons_practice.lum` (`pfac`), dan faktor LS:

```latex
m = 0.6\left(1 - e^{-35.835\,S}\right),\quad \sin\theta = \frac{S}{\sqrt{S^2+1}},\quad LS = \left(\frac{L}{22.128}\right)^{m}\left(65.41\sin^2\theta + 4.56\sin\theta + 0.065\right)
```

Dengan S lereng (m/m) dan L panjang lereng (m). Simpan juga `lum_name` dan `cal_group`.

14. **Kekasaran Manning** `ovn` dari `ovn_table.lum`; 0,05 bila nama tidak ditemukan.
15. **Bilangan kurva** (`cn2_init`): CN2 dari `cntable.lum` menurut grup hidrologi A–D; nama tak ditemukan memberi bawaan tipe (30, 55, 70, 77); grup lain memberi 0. Lalu `curno`.
16. **`hydro_init`**: faktor USLE gabungan

```latex
usle_{mult} = e^{-0.053\,rock_1}\;K_1\;P\;LS\;11.8
```

Suhu tanah awal semua lapisan = rata-rata `tmpmx(12)` dengan dirinya sendiri; fraksi air awal kapasitas lapang:

```latex
sffc = \begin{cases} \dfrac{P_{an}}{P_{an} + \exp(9.043 - 0.002135\,P_{an})} & ffcb \le 0 \\ ffcb & \text{lainnya} \end{cases},\qquad st_l = sffc\cdot fc_l
```

Dengan P\_an hujan tahunan rata-rata generator (mm); `sw` dijumlahkan.

17. **Dormansi awal**: bila ada tanaman, panjang hari dihitung untuk hari 0 (deklinasi `asin(0.4 sin((0-82)/58.09))`, sudut jam dibatasi), dan tanaman yang sedang tumbuh menjadi dorman bila panjang hari − `daylth` < `daylmn`.
18. `zmx` = kedalaman dasar lapisan terakhir. Untuk drain: `ldrain` = lapisan pertama yang dasarnya lebih dalam dari drain (berbasis-1), dan `ttime` = 1 − exp(−24/lag) bila `lag` > 0,01.
19. **Waktu tempuh lateral**: bila `lat_ttime` <= 0 dihitung dari konduktivitas terbesar profil,

```latex
t_{lat} = \max\!\left(1,\ \frac{10.4\,L_{lat}}{K_{sat,max}}\right),\qquad lat\_ttime = 1 - e^{-1/t_{lat}}
```

Selain itu `lat_ttime` = 1 − exp(−1/lat\_ttime).

- **Kebiasaan SWAT+ / catatan port:** (a) suhu tanah awal memakai `tmpmx(12)` dua kali, bukan rata-rata maksimum dan minimum (`docs/swat-quirks.md`); (b) dormansi diuji untuk hari 0 karena `hydro_init` berjalan sebelum jam simulasi diset; (c) `sin(atan(S))` ditulis sebagai `S/sqrt(S^2+1)` karena gfortran melipatnya demikian dan pembulatannya berbeda dari `sinf(atanf(S))` pada kira-kira dua dari lima lereng; selisih satu bit faktor USLE sempat menggeser biomassa dua HRU Batanghari hingga 2 %; (d) `perco_lim` di sini memakai ambang 1e-9, sedangkan kalibrasi memakai 1e-6; (e) `soil_plant.ini` `sw_frac` dibaca tetapi tidak dipakai, sama seperti SWAT+.

#### 7.2.3  `nutcarb_stage`

- **Lokasi:** `hru/mod.rs`, `fn nutcarb_stage(project, hd: &HruData, h: &mut Hru, psp_saved: &mut f32)`
- **Padanan SWAT+:** `soil_nutcarb_init` (dipanggil dari program utama setelah `proc_cal`)
- **Tujuan:** Mengisi pool nutrien dan karbon awal tanah dari entri `soil_plant.ini` HRU dan uji tanah (`nutrients.sol`).
- **Cara kerja:**
  1. Cari entri `soil_plant.ini` bernama; bila tidak ada, entri pertama.
  2. Bila tabel uji tanah memuat `nutc` entri itu, isi `SoilTest`: `exp_co` (dipaksa 0,001 bila > 0,005), `lab_p`, `nitrate`, `fr_hum_act`, `hum_c_n`, `hum_c_p`; selain itu nilai bawaan (`SoilTest::default`, yang di Bab nutrien menghasilkan bawaan SWAT+ 7 ppm nitrat dan 5 ppm P labil di permukaan).
  3. Panggil `Hru::nutcarb_init` (Bab nutrien) dengan `psp` (bawaan 0,4), `cswat`, `sol_p_model`.
  4. Bila `cswat` = 2, panggil `Hru::carbon_init` (Bab model karbon).

### 7.3  Lapisan tanah, bilangan kurva, dan waktu konsentrasi

#### 7.3.1  `soil_layers`

- **Lokasi:** `hru/mod.rs`, `fn soil_layers(h: &mut Hru, soil: &Soil, depths: Option<&[i32]>) -> Result<(), String>`
- **Padanan SWAT+:** `soils_init.f90` dan `soil_phys_init.f90`
- **Tujuan:** Menyusun lapisan HRU dari tanah `soils.sol` dan menghitung sifat hidrolik turunan.
- **Masukan dan keluaran:** lapisan berkas (`z` mm, `bd`, `awc`, `k`, tekstur, `rock`, `alb`, `cbn`, `usle_k`, `cal`), grup hidrologi, tekstur, `anion_excl`; kedalaman khusus opsional. Mengisi `h.layers`, `hydgrp`, `texture`, `anion_excl`, `sumul`, `sumfc`, `avbd`.
- **Cara kerja:**
  1. **Susunan lapisan.** Bila ada `soil_lyr_depths.sol`, pakai `custom_layers`. Bila tidak dan lapisan pertama lebih dalam dari 19,5 mm, sisipkan lapisan atas 10 mm dengan sifat lapisan pertama, lalu semua lapisan berkas (lapisan j memakai sifat lapisan berkas j−1). Bila tidak, lapisan 1 menjadi 10 mm dengan sifat lapisan pertama dan lapisan berikutnya diambil mulai lapisan berkas kedua.
  2. `anion_excl` dibatasi: <= 1e-6 menjadi 0,5; >= 1 menjadi 0,99.
  3. **Batas nilai per lapisan:** `k` <= 0 diganti menurut grup (A 50, B 20, C 5, D 2, lainnya 10 mm/jam); `bd` <= 1e-6 menjadi 1,3 dan > 2 menjadi 2; `awc` dibatasi \[0,005; 0,8\]; `rock` maksimum 98; `cal` <= 1e-6 menjadi 2,8 % (rata-rata 20.000 tanah SSURGO).
  4. **Titik layu, kapasitas lapang, porositas** (fraksi volume):

```latex
wp = 0.4\,\frac{clay\cdot bd}{100},\qquad up = wp + awc,\qquad por = 1 - \frac{bd}{2.65}
```

`wp` <= 0 menjadi 0,005. Bila `up` >= `por`: `up` = `por` − 0,05, `wp` = `up` − `awc`; bila `wp` masih <= 0, `up` = 0,75 `por` dan `wp` = 0,25 `por`.

5. **Isi air per lapisan** (mm), dengan tebal `thick` = `d` − `d_prev`:

```latex
ul = (por - wp)\,thick,\quad fc = (up - wp)\,thick,\quad wpmm = wp\,thick,\quad hk = \max\!\left(1,\ \frac{ul - fc}{K_{sat}}\right)
```

`sumul` dan `sumfc` dijumlahkan; `hk` adalah waktu tempuh perkolasi (jam).

6. **Bulk density rata-rata profil:** `avbd` = 2,65 (1 − Σ(por·thick)/d\_max).

- **Kebiasaan SWAT+ / catatan port:** penyisipan lapisan 10 mm menggeser sifat satu lapisan ke bawah (lapisan j memakai sifat lapisan berkas j−1); ini perilaku SWAT+ yang dipertahankan.

#### 7.3.2  `custom_layers`

- **Lokasi:** `hru/mod.rs`, `fn custom_layers(soil: &Soil, depths: &[i32]) -> Vec<Layer>`
- **Padanan SWAT+:** cabang `soil_lyr_depths.sol` di `soils_init.f90`
- **Tujuan:** Membagi ulang profil tanah ke kedalaman yang ditentukan pengguna.
- **Cara kerja:**
  1. Potong profil (kedalaman total dibulatkan ke bawah ke mm bulat, `tot0`) menjadi irisan 1 mm; setiap irisan mencatat indeks lapisan berkas tempatnya berada.
  2. **Penghitungan:** telusuri kedalaman; kedalaman <= 10 mm dilewati; kedalaman pertama yang mencapai atau melewati dasar menjadi dasar dan menghentikan daftar; bila daftar habis sebelum dasar, profil terpotong di kedalaman terakhir. Jumlah lapisan ditambah satu untuk lapisan atas 10 mm.
  3. **Pembentukan:** lapisan pertama selalu 10 mm (kedalaman pertama dibaca ulang setelahnya); setiap lapisan merata-ratakan 11 sifat (`bd`, `awc`, `k`, `cbn`, `clay`, `silt`, `sand`, `rock`, `alb`, `usle_k`, `cal`) atas irisan 1 mm-nya, dengan penjumlahan satu per satu dalam `f32`.
- **Kebiasaan SWAT+ / catatan port:** penjumlahan berurutan dalam presisi tunggal membuat lapisan seragam (mis. `bd` 1,26) keluar sedikit berbeda dari nilai aslinya; ini dipertahankan. Berkas dibaca dengan nama tetap apa pun isi `file.cio`.

#### 7.3.3  `soil_awc_init`

- **Lokasi:** `hru/mod.rs`, `fn soil_awc_init(h: &mut Hru)`
- **Padanan SWAT+:** `soil_awc_init.f90`
- **Tujuan:** Menghitung ulang sifat air lapisan setelah kalibrasi `z`, `bd`, atau `awc`.
- **Cara kerja:** sama dengan langkah 3 (khusus `awc`), 4, 5, dan 6 dari `soil_layers`, ditambah: air setiap lapisan dikembalikan ke `st` = `fc`·`ffc` dan `sw` dijumlahkan ulang.
- **Kebiasaan SWAT+ / catatan port:** kalibrasi `awc` karenanya mengembalikan air tanah ke bagian awal kapasitas lapang.

#### 7.3.4  `init_bmix`

- **Lokasi:** `hru/mod.rs`, `fn init_bmix(h: &mut Hru, b: &Biomix)`
- **Padanan SWAT+:** bagian `soil_nutcarb_init` yang mengisi `ly%init_bmix`
- **Tujuan:** Membagi efisiensi pencampuran biologis per lapisan menurut bagian lapisan yang berada di atas kedalaman pencampuran.
- **Cara kerja:**

```latex
init\_bmix_l = \begin{cases} eff & D_{mix} \ge d_l \\ eff\,\dfrac{D_{mix} - d_{l-1}}{thick_l} & d_{l-1} < D_{mix} \le d_l \\ 0 & \text{lainnya} \end{cases}
```

#### 7.3.5  `ascrv`

- **Lokasi:** `hru/mod.rs`, `pub fn ascrv(x1, x2, x3, x4: f32) -> (f32, f32)`
- **Padanan SWAT+:** `ascrv.f90`
- **Tujuan:** Menghitung dua koefisien kurva-S y = x/(x + exp(a − b x)) yang melalui titik (x3, x1) dan (x4, x2). Dipakai untuk kurva tutupan salju dan kurva retensi CN.
- **Cara kerja:**

```latex
xx = \ln\!\left(\frac{x_3}{x_1} - x_3\right),\quad b = \frac{xx - \ln(x_4/x_2 - x_4)}{x_4 - x_3},\quad a = xx + x_3\,b
```

- **Catatan:** satu uji unit (`ascrv_passes_through_its_points`, di modul `tests`) memeriksa bahwa kurva melalui kedua titik.

#### 7.3.6  `curno`

- **Lokasi:** `hru/mod.rs`, `fn curno(h: &mut Hru)`
- **Padanan SWAT+:** `curno.f90`
- **Tujuan:** Menurunkan parameter retensi maksimum `smx` dan koefisien kurva retensi `wrt` dari CN2 (metode kelembapan tanah SCS yang dimodifikasi, Neitsch et al. 2009 bagian 2:1.1).
- **Cara kerja:**
  1. CN kondisi I dan III:

```latex
CN_1 = \max\!\left(CN_2 - \frac{20\,(100 - CN_2)}{100 - CN_2 + \exp(2.533 - 0.0636\,(100 - CN_2))},\ 0.4\,CN_2\right),\quad CN_3 = CN_2\,\exp(0.006729\,(100 - CN_2))
```

2. Retensi: `smx` = 254 (100/CN1 − 1), S3 = 254 (100/CN3 − 1); rasio rto3 = 1 − S3/smx dan rtos = 1 − 2,54/smx.
3. Air pada titik kalibrasi CN3: `sumfc'` = `sumfc` + `cn3_swf` (`sumul` − `sumfc`), lalu: < 0 menjadi 10, minimum 0,05, maksimum `sumul` − 0,05.
4. `wrt` = `ascrv(rto3, rtos, sumfc', sumul)`, sehingga retensi S turun ke S3 ketika air tanah mencapai `sumfc'` dan ke 2,54 mm ketika mencapai `sumul`.

#### 7.3.7  `time_conc`

- **Lokasi:** `hru/mod.rs`, `fn time_conc(h: &mut Hru, surlag: f32)`
- **Padanan SWAT+:** `time_conc_init.f90` (bagian HRU)
- **Tujuan:** Waktu konsentrasi HRU dan bagian limpasan yang dilepas per hari.
- **Cara kerja:**

```latex
t_{ov} = \frac{0.0556\,(L\,n)^{0.6}}{(S + 0.0001)^{0.3}},\quad L_{ch} = 0.5\sqrt{A_{ha}/2},\quad t_{ch} = \frac{0.31\,L_{ch}\,n^{0.75}}{A_{km}^{0.125}\,(0.5S + 0.001)^{0.375}}
```

```latex
t_{conc} = t_{ov} + t_{ch},\qquad brt = 1 - \exp\!\left(-\frac{surlag}{t_{conc}}\right)
```

dengan L panjang lereng (m), n = `ovn` (juga dipakai sebagai n saluran), S lereng.

- **Catatan port:** untuk sub-harian `init_hrus` menimpa `brt` dengan versi per langkah (7.2.1).

### 7.4  Kalibrasi keras (`calibration.cal`) pada HRU dan tanah

Kalibrasi keras diterapkan sekali, setelah semua HRU disiapkan (padanan `proc_cal` → `cal_conditions`). Akibatnya, perubahan hanya menjangkau besaran yang dihitung dari parameter itu *sesudahnya*: misalnya perubahan `slope` tidak memperbarui waktu konsentrasi dan LS USLE yang sudah dihitung. Fungsi `chg_par`, `cal_parm`, `cal_cond_met`, dan `cal_elems` bersifat `pub(crate)` karena juga dipakai modul akuifer.

#### 7.4.1  `chg_par`

- **Lokasi:** `hru/mod.rs`, `pub(crate) fn chg_par(u: &CalUpdate, p: &CalParm, cur: f32) -> f32`
- **Padanan SWAT+:** fungsi `chg_par` (`chg_par.f90`)
- **Tujuan:** Menerapkan satu perubahan kalibrasi pada nilai parameter saat ini dan membatasinya ke rentang `cal_parms.cal`.
- **Cara kerja:**

```latex
v = \begin{cases} val & \text{absval} \\ cur + val & \text{abschg} \\ (1 + val/100)\,cur & \text{pctchg} \\ (1 + val)\,cur & \text{lainnya (relchg)} \end{cases},\qquad v' = \min(\max(v, absmin), absmax)
```

#### 7.4.2  `cal_parm`

- **Lokasi:** `hru/mod.rs`, `pub(crate) fn cal_parm(project, u: &CalUpdate) -> CalParm`
- **Padanan SWAT+:** pencocokan nama di `cal_parmchg_read.f90` terhadap `cal_parms.cal`
- **Tujuan:** Mengambil baris `cal_parms.cal` (jenis objek, batas absolut) untuk nama parameter pembaruan; nilai bawaan bila tidak ada. Perbandingan nama gaya Fortran (`fortran_eq`).

#### 7.4.3  `cal_cond_met`

- **Lokasi:** `hru/mod.rs`, `pub(crate) fn cal_cond_met(u: &CalUpdate, hrus: &[Hru], e: usize) -> Result<bool, String>`
- **Padanan SWAT+:** bagian pengujian kondisi di `cal_conditions.f90`
- **Tujuan:** Menentukan apakah HRU nomor `e` (berbasis-1) memenuhi semua kondisi pembaruan.
- **Cara kerja:**
  1. Tanpa kondisi: benar.
  2. Perbandingan teks meniru string Fortran berpanjang tetap: target dipotong ke 25 karakter, spasi kanan diabaikan.
  3. Kondisi yang diuji: `hsg` (grup hidrologi), `texture`, `plant` (hanya tanaman pertama komunitas; HRU tanpa tanaman lolos), `landuse` (kegagalan langsung menghentikan loop), `cal_group`. Kondisi `res_pvol`, `res_typ`, `pl_class` ditolak sebagai belum di-port; variabel lain tidak menjadi kondisi.
- **Kebiasaan SWAT+ / catatan port:** kondisi selalu diuji pada HRU nomor `e`, apa pun jenis objek parameternya (untuk parameter `bsn`, HRU 1).

#### 7.4.4  `cal_elems`

- **Lokasi:** `hru/mod.rs`, `pub(crate) fn cal_elems(u: &CalUpdate, n: usize) -> Vec<usize>`
- **Padanan SWAT+:** `define_unit_elements`
- **Tujuan:** Daftar objek (berbasis-1) yang diubah: daftar eksplisit pembaruan, atau semua 1..n bila kosong (jumlah 0 di berkas). Rentang dengan bilangan negatif sudah diurai saat membaca berkas.

#### 7.4.5  `cal_conditions`

- **Lokasi:** `hru/mod.rs`, `fn cal_conditions(project, hrus: &mut [Hru]) -> Result<(), String>`
- **Padanan SWAT+:** `cal_conditions.f90` (dipanggil dari `proc_cal`)
- **Tujuan:** Menerapkan setiap baris `calibration.cal` ke parameter HRU, tanah, dan DAS.
- **Cara kerja:**
  1. Untuk setiap pembaruan, ambil `cal_parm`. Jenis objek selain `hru`, `sol`, `bsn`, `aqu` dan jenis perubahan selain `absval`, `abschg`, `pctchg`, `relchg` ditolak.
  2. `aqu` dilewati (diterapkan saat akuifer disiapkan).
  3. `bsn`: hanya `surlag` yang didukung; bila HRU 1 memenuhi kondisi, `surlag` diubah sekali lalu `brt` = 1 − exp(−surlag/tconc) untuk semua HRU.
  4. Untuk objek `hru`/`sol`: untuk setiap HRU di `cal_elems` (di luar 1..n ditolak) yang memenuhi kondisi: `sol` diterapkan per lapisan `lyr1`..`lyr2` (lyr1 minimal 1; lyr2 <= 0 berarti lapisan terakhir) lewat `cal_soil`; `hru` lewat `cal_hru`.
- **Kebiasaan SWAT+ / catatan port:** untuk `surlag` sub-harian, rumus harian yang dipakai (sesuai `cal_parm_select`).

#### 7.4.6  `cal_hru`

- **Lokasi:** `hru/mod.rs`, `fn cal_hru(h: &mut Hru, name: &str, chg: &dyn Fn(f32) -> f32) -> Result<(), String>`
- **Padanan SWAT+:** cabang HRU di `cal_parm_select.f90`
- **Tujuan:** Mengubah satu parameter HRU dan menghitung ulang turunannya.
- **Cara kerja (per nama parameter):**
  - `perco`, `cn3_swf` pada HRU ber-drain: tidak diubah.
  - `perco`: ubah, lalu `perco_lim` dihitung ulang dengan rumus 7.2.2 langkah 8 tetapi ambang 1e-6 (bukan 1e-9).
  - `cn2`, `cn3_swf`: ubah lalu `curno`.
  - `petco`, `latq_co`, `canmx`, `esco`, `lat_len`, `erorgn`, `erorgp`, `biomix` (`hyd_biomix`), `usle_p`, `elev`, `slope`, `field_wid`: ubah langsung.
  - `lat_ttime`: nilai yang tersimpan sudah berupa 1 − exp(−1/t); perubahan diterapkan pada nilai itu, lalu dibentuk ulang 1 − exp(−1/nilai baru).
  - `epco`: ubah dan salin ke setiap tanaman.
  - `snofall_tmp`, `snomelt_tmp`, `snomelt_max`, `snomelt_min`, `snomelt_lag`: ubah parameter salju.
  - `ovn`, `slope_len`, `field_len`, `field_ang`, `dis_stream`: diterima tanpa efek (SWAT+ hanya memakainya saat penyiapan atau tidak sama sekali).
  - Nama lain: `Err`.
- **Kebiasaan SWAT+ / catatan port:** perlakuan `lat_ttime` (perubahan atas nilai yang sudah ditransformasi) dan ambang `perco` yang berbeda meniru SWAT+ apa adanya. Perubahan `usle_p` tidak memperbarui `usle_mult` di sini (hanya kalibrasi `rock` atau `usle_k` yang melakukannya, dengan `usle_p` baru).

#### 7.4.7  `cal_soil`

- **Lokasi:** `hru/mod.rs`, `fn cal_soil(h: &mut Hru, ly: usize, name: &str, chg) -> Result<(), String>`
- **Padanan SWAT+:** cabang tanah di `cal_parm_select.f90`
- **Tujuan:** Mengubah satu parameter lapisan tanah (`ly` berbasis-0).
- **Cara kerja:**
  - `z`, `bd`, `awc`: ubah, lalu `soil_awc_init` dan `curno` (air tanah kembali ke bagian awal kapasitas lapang).
  - `k`: ubah, lalu `hk` = max(1, (ul − fc)/k).
  - `rock`: ubah; bila lapisan pertama, `usle_mult` dihitung ulang.
  - `usle_k`: ubah lalu `usle_mult` dihitung ulang: e^(−0,053 rock\_1) K\_1 P LS 11,8 (memakai lapisan pertama).
  - `alb`, `cal`, `anion_excl` (tingkat HRU): ubah langsung.
  - `ec`, `ph`: diterima tanpa efek.
  - Nama lain: `Err`.

### 7.5  Hari HRU: `Hru::day` dan rutin periode

#### 7.5.1  `Hru::day`

- **Lokasi:** `hru/mod.rs`, `pub fn day(&mut self, w: &HruWeather, c: &DayCtx, b: &HruBasin, saved: &mut Saved) -> HruDay`
- **Padanan SWAT+:** `hru_control.f90` beserta `varinit.f90`, `stmp_solt`, `sq_canopyint`, `sq_snom`, `rls_routesurf`/`rls_routesoil`, `et_pot`, `et_act`, `mgt_operatn`, `surface` (`sq_dailycn`, `sq_volq`/`sq_daycn`, `sq_greenampt`, `ero_*`), `swr_percmain`, `nut_*`, `pl_community`/`pl_grow`, `hru_urban`/`hru_urbanhr`, `stor_surfstor`, `swr_substor`, `smp_*`, `sq_surfst`, `swr_subwq`, `hru_hyds`, dan bagian harian `hru_output`.
- **Tujuan:** Menjalankan satu hari siklus air, sedimen, nutrien, dan tanaman HRU, serta menyusun keluaran harian dan hidrograf HRU.
- **Masukan dan keluaran:** cuaca HRU `w` (hujan `precip` mm, `tmax`/`tmin`/`tave` °C, `solrad` dan `solradmx` MJ/m2, `rhum` fraksi, `windsp` m/s, `ts` hujan per langkah, `atmo` deposisi, `station_pet`, `phubase0`); konteks `c`; pengaturan `b`; status antarpanggilan `saved` (variabel modul Fortran). Keluaran `HruDay`.

**Cara kerja.** Langkah-langkah berikut dijalankan berurutan persis seperti `hru_control`.

**Langkah 1 – nol-kan harian (`time_control`, `varinit`).** `irr_applied`, `irr_runoff`, `fertn`, `fertp`, `water_evap` = 0. Bila model karbon aktif dan efek olah tanah masih berlaku (`till_switch`), `till_days` bertambah; setelah `till_eff_days` efek dimatikan dan `tillagef_tillmix` semua lapisan dinolkan.

**Langkah 2 – tabel keputusan.** `dtbl_day` (Bab manajemen) dijalankan dengan stres kemarin; lalu `days_plant`, `days_harv`, `days_kill`, `days_irr` bertambah satu. Air tanah awal hari `sw_volume_begin` = Σ st dicatat; `precip_eff` = hujan HRU. `prk` dan `flat` setiap lapisan dinolkan.

**Langkah 3 – satuan panas dasar dan stres.** Bila `tave` > 0 dan `phutot` > 0,01: `phubase` += tave/phutot. Semua stres tanaman (`strsw`, `strst`, `strsn`, `strsp`, `strsa`, `strss`) diset 1.

**Langkah 4 – albedo** (dengan LAI dan tutupan kemarin):

```latex
e_{aj} = \exp\!\left(-5\times10^{-5}(CV + 0.1)\right),\qquad \alpha = \begin{cases} 0.8 & sno > 0.5 \\ 0.23(1 - e_{aj}) + \alpha_{soil}\,e_{aj} & sno \le 0.5,\ LAI > 0 \\ \alpha_{soil} & \text{lainnya} \end{cases}
```

CV adalah biomassa di atas tanah ditambah residu (kg/ha), α\_soil albedo lapisan pertama.

**Langkah 5 – suhu tanah.** Opsi septik hari ini diambil dari `saved.septic_opt`, lalu `soil_temperature` (7.6.2).

**Langkah 6 – intersepsi kanopi (`sq_canopyint`).** Bila LAI dan LAI maksimum komunitas >= 0,001, kapasitas kanopi hari ini adalah

```latex
can_{mx,l} = canmx\,\frac{LAI}{LAI_{mx}}
```

Harian: bila `precip_eff` < can\_mx,l − canstor, semua hujan masuk kanopi; selain itu kanopi diisi penuh dan sisanya menjadi `precip_eff`. Sub-harian: hujan setiap langkah dikurangi ruang kosong kanopi secara berurutan; bila kanopi bertambah, pengurangan kedua dilakukan atas hujan stasiun (`w.ts`) mulai dari simpanan awal kanopi (meniru dua loop Fortran). Pada mode sub-harian hujan harian `precip_eff` tidak diubah oleh kanopi.

**Langkah 7 – salju (`sq_snom`).** Suhu paket salju tanpa memori:

```latex
T_{snow} = 0\cdot(1 - timp) + \bar T\,timp
```

Bila `tave` <= `falltmp`, semua `precip_eff` menjadi salju (`snofall`), dan hujan per langkah dinolkan. Bila `tmax` > `melttmp` dan ada salju:

```latex
b_{mlt} = \frac{m_{mx} + m_{mn}}{2} + \sin\!\left(\frac{day - 81}{58.09}\right)\frac{m_{mx} - m_{mn}}{2},\qquad SNO_{mlt} = b_{mlt}\,cov\left(\frac{T_{snow} + T_{max}}{2} - T_{mlt}\right)
```

```latex
cov = \begin{cases} \dfrac{r}{r + \exp(cov_1 - cov_2\,r)},\ r = \dfrac{SNO}{covmx} & SNO < covmx \\ 1 & \text{lainnya} \end{cases}
```

Lelehan dibatasi \[0, SNO\], dikurangkan dari salju, ditambahkan ke `precip_eff` dan dibagi rata ke setiap langkah.

**Langkah 8 – routing lanskap (`rls_routesurf`, `rls_routesoil`).** Bila ada `runon` dengan aliran permukaan > 1e-6: HRU berlahan basah menerima limpasan permukaan + tile langsung ke kolam (m3) dan mencatatnya sebagai `wet_runon`; HRU lain menerimanya sebagai hujan:

```latex
Q_{ls} = Q_{sur} + \frac{Q_{til}}{10\,A_{ha}}
```

(aliran tile per hektar dibagi lagi dengan 10·luas, perilaku SWAT+). Sedimen yang terbawa dibatasi kapasitas angkut

```latex
T_c = dep_{co}\;C_{usle}\;Q_{ls}\;S^{1.4}\;W_{field}^{1.4}
```

memakai faktor C terakhir yang dihitung. Aliran lateral masuk ke lapisan sebanding tebalnya (thick/d\_n).

**Langkah 9 – ET potensial (`et_pot`).** Tekanan uap jenuh e\_a = `ee(tave)`, aktual e\_d = e\_a·rhum, defisit VPD = e\_a − e\_d. Metode menurut `pet`: 0 Priestley-Taylor, 1 Penman-Monteith (juga menghasilkan transpirasi maksimum), 2 Hargreaves, 3 PET stasiun (yang oleh `climate_control` sudah diganti Hargreaves). PET HRU = `pet_co` × PET mentah.

**Langkah 10 – ET aktual (`et_act`).** `evaporation` (7.6.3) menghasilkan evaporasi kanopi, evaporasi tanah, dan transpirasi maksimum.

**Langkah 11 – operasi manajemen terjadwal.** `mgt_operatn` bila ada jadwal dan tahun tidak dilewati. Jadi operasi berjalan setelah ET dan sebelum limpasan.

**Langkah 12 – limpasan permukaan (`surface`).** Bukan untuk HRU lahan basah (CN-nya 0). CN harian dari `daily_cn` (7.6.5). Bila `precip_eff` > 0,1 mm: dengan Green-Ampt, `green_ampt` (bab sub-harian) menghasilkan limpasan, CN, dan limpasan per langkah; tanpa Green-Ampt dipakai persamaan SCS (`sq_volq`/`sq_daycn`):

```latex
S = \frac{25400}{CN} - 254,\qquad Q_{surf} = \frac{(R_{day} - 0.2S)^2}{R_{day} + 0.8S}\quad (R_{day} > 0.2S)
```

Pada HRU perkotaan bagian kedap air yang terhubung langsung (`fcimp`) memakai CN 98:

```latex
Q_{surf} = Q_{perv}\,(1 - f_{cimp}) + Q_{imp,CN=98}\,f_{cimp}
```

**Langkah 13 – lahan basah atau erosi.** HRU lahan basah: `wetland_control` (Bab lahan basah) memberi limpasan = pelepasan, infiltrasi = rembesan, dan limpasan per langkah dibagi rata. HRU lain: `surfq` += limpasan irigasi, lalu `erosion` (diimplementasikan di `hru/nutrient.rs`): debit puncak rasional (`ero_pkq`), energi hujan USLE (`ero_eiusle`), faktor C (`ero_cfactor`), dan MUSLE (`ero_ysed`):

```latex
q_{peak} = \frac{\alpha_{tc}\,Q_{surf}\,A_{km}}{3.6\,t_{conc}},\quad \alpha_{tc} = 1 - \exp\!\left(2\,t_{conc}\ln(1 - \alpha_{0.5})\right)
```

```latex
sed = \left(10\,Q_{surf}\,q_{peak}\,A_{ha}\right)^{0.56}\;C\;usle_{mult}
```

dengan usle\_mult = K·P·LS·CFRG·11,8 (7.2.2); `usle` (erosi USLE untuk keluaran) = 1,292·EI·C·usle\_mult/11,8. Sedimen run-on ditambahkan ke hasil sedimen, dan infiltrasi = max(0, `precip_eff` − `surfq`). Untuk Green-Ampt dengan limpasan dan debit puncak, `ero_ovrsed` menghitung sedimen per langkah.

**Langkah 14 – perkolasi (`swr_percmain`).** `percolate` (7.6.6) dengan masukan infiltrasi + irigasi (bukan untuk lahan basah, yang irigasinya masuk kolam) + run-on lahan basah per hektar; menghasilkan aliran lateral harian, perkolasi dasar profil, kelebihan jenuh ke `surfq`, dan aliran tile. Pelepasan lahan basah (`wet_oflo`) dikonversi ke mm.

**Langkah 15 – siklus nutrien tanah.** `cswat` 0: `rsd_decomp`, `nut_nminrl`. `cswat` 2: `biomix` (7.6.1) bila efisiensi > 1e-6, `cbn_surfrsd_decomp`, `cbn_rsd_transfer`, `cbn_zhang2` (denitrifikasi). Lalu `nut_nitvol` dan `nut_pminrl2` (`sol_p_model` 1) atau `nut_pminrl`. Biozona septik (`sep_biozone`) bila aktif, tahunnya tercapai, dan lapisannya tidak beku.

**Langkah 16 – tanaman.** `pl_community` dan `pl_grow` (Bab tanaman) dengan `ep_max`, PET, VPD, dan cuaca tanaman. Rata-rata (1 − stres) tanaman yang sedang tumbuh, tidak dorman, dan `phuacc` <= 1 dihitung untuk lima stres pertama; stres garam hanya dijumlahkan.

**Langkah 17 – air 0–300 mm.** `sw_300` = Σ st lapisan di atas 300 mm, ditambah bagian proporsional lapisan yang memotong 300 mm. ET hari ini = transpirasi + evaporasi tanah + evaporasi kanopi.

**Langkah 18 – nutrien terangkut.** Bila `surfq` > 0, debit puncak > 1e-6, dan `precip_eff` > 0: `sediment_nutrients` (rasio pengayaan, N organik, P sedimen) dan, pada `cswat` 2, `nut_orgnc2`. Deposisi atmosfer (`nut_nrain`): NO3 = 0,01·c\_rf·hujan + kering, NH4 serupa, ditambahkan ke lapisan pertama; di luar data deposisi nilai kemarin dicetak ulang tetapi tidak ditambahkan. Lalu `nut_nlch` dan `nut_solp`.

**Langkah 19 – perkotaan.** Sub-harian: `hru_urbanhr`; harian: `hru_urban` yang mengembalikan debu (`silyld`).

**Langkah 20 – lag sedimen (`stor_surfstor`).** Untuk tujuh konstituen (sedimen, N organik, P organik, NO3, P terlarut, P mineral aktif dan stabil):

```latex
S_k \leftarrow \max(S_k + x_k,\ 10^{-9}),\quad x_k' = brt\,S_k,\quad S_k \leftarrow S_k - x_k'
```

Sub-harian: sedimen masuk simpanan sekali per langkah dan hasilnya diganti `lag_sediment`. Lima kelas sedimen (pasir, debu, liat, agregat kecil dan besar) dimulai dari minimum 1e-6 (debu menerima `silyld`) dan melepas `brt` bagian.

**Langkah 21 – lag aliran lateral dan tile (`swr_substor`).** `bss` += aliran lateral baru, `bss_no3` += NO3 lateral; nilai < 1e-6 menjadi 0; lepaskan latq = bss·lat\_ttime (dan NO3) dengan ambang 1e-6 yang sama. Tile: `bss_tile` dan `bss_tno3` dengan faktor `ttime` drain.

**Langkah 22 – BMP.** `bmp::filter` (jalur filter, memakai K lapisan pertama), `bmp::grass_wway`, `bmp::user` mengurangi konstituen yang sudah di-lag (Bab struktur).

**Langkah 23 – lag limpasan (`sq_surfst`).** Harian: `surf_bs` = max(surf\_bs + surfq, 1e-6), Q\_day = surf\_bs·brt, sisanya tetap di simpanan. Green-Ampt: `lag_runoff` per langkah. Hasil air `qdr` = Q\_day + latq + qtile (minimal 0).

**Langkah 24 – kualitas air limpasan (`swr_subwq`).** Bila qdr > 1e-4: suhu air T\_w = 5 + 0,75·tave (minimum 0,1 °C) dalam K; klorofil-a = 0,1·100(orgN + NO3)/qdr; CBOD

```latex
C_{org} = \frac{cbn_1}{100}\,ER\,sed\cdot1000,\qquad CBOD = \frac{2.7\,C_{org}}{qdr\,A_{km}\,10^4}
```

oksigen jenuh dari persamaan APHA

```latex
\ln O_{sat} = -139.3441 + \frac{1.575701\times10^5}{T_K} - \frac{6.642308\times10^7}{T_K^2} + \frac{1.2438\times10^{10}}{T_K^3} - \frac{8.621949\times10^{11}}{T_K^4}
```

dan DO = O\_sat·exp(−0,1·CBOD), dibatasi \[0, O\_sat\].

**Langkah 25 – hidrograf (`hru_hyds`).** Konversi mm → m3 dengan 10·A\_ha dan kg/ha → kg dengan A\_ha. Permukaan: suhu 5 + 0,75·tave, aliran, sedimen (t), orgN, sedP (organik + mineral aktif + stabil), NO3, solP (+ P labil tile), klorofil, kelas sedimen; CBOD dan DO dalam ppm bila aliran > 0,01 m3. Imbuhan: perkolasi dasar dan NO3 perkolasi. Lateral dan tile: aliran dan NO3. Total = permukaan + lateral + tile. Urutan larik: \[total, imbuhan, permukaan, lateral, tile\].

**Langkah 26 – tabel harian.** Isi `HruWb` (antara lain `surq_gen` = Q\_day setelah lag, `surq_cont` = limpasan sebelum lag, `wateryld`, `perc`, `et`, `cn`, `delsw` = sw − sw awal hari, `lagsurf`, `laglatq`, kolom lahan basah), `HruPw` (LAI, biomassa, hasil panen, residu, suhu lapisan kedua sebagai `sol_tmp`, stres, N/P tanaman, `phubase`), `HruNb`, `HruLs` (`sedyld` dalam t/ha). Hasil panen hari ini dinolkan; biomassa negatif dipotong ke 0. `harvests` dipindahkan ke keluaran.

- **Kebiasaan SWAT+ / catatan port:**
  - Sedimen dikurangkan dua kali dari simpanannya (`stor_surfstor`), sehingga simpanan menjadi negatif setelah hari bersedimen; di `--fix-swat` sekali (pada sub-harian selalu sekali).
  - Rasio pengayaan untuk CBOD adalah yang terakhir dihitung oleh HRU mana pun (`saved.enratio`, variabel lokal bernama sama di `varinit`); `--fix-swat` memakai rasio HRU sendiri.
  - Kelas sedimen tidak pernah nol (minimum 1e-6) sehingga setiap HRU melepas sedikit sedimen setiap hari.
  - Uji salju di MUSLE terbalik (salju hanya menolkan hasil < 1e-6); dikoreksi di `--fix-swat`.
  - Limpasan kedap air memakai CN 98, bukan `urb_cn`.
  - Kolom `rsd_nitorg_n` dan `rsd_laborg_p` selalu 0; `lai_max` dicetak 0 dan `bm_max` sama dengan biomassa.
  - `nutrients` hanya benar bila `cswat` 0 atau 2.

#### 7.5.2  `Hru::start_balance`

- **Lokasi:** `hru/mod.rs`, `pub fn start_balance(&mut self)`
- **Padanan SWAT+:** `basin_sw_init`
- **Tujuan:** Menetapkan air tanah dan salju awal periode cetak (`sw_init` = `sw`, `sno_init` = `sno_mm`) pada awal tahun cetak pertama (setelah `nyskip`).

#### 7.5.3  `Hru::end_year`

- **Lokasi:** `hru/mod.rs`, `pub fn end_year(&mut self, cswat: i32, fixes: bool)`
- **Padanan SWAT+:** bagian akhir tahun `time_control.f90`
- **Tujuan:** Pekerjaan HRU pada hari terakhir tahun.
- **Cara kerja:**
  1. Dengan `fixes`, `irr_yr` dinolkan (di SWAT+ reset ini dikomentari).
  2. `cswat` 0 dan `hyd_biomix` > 1e-6: `year_end_mixing` (padanan `mgt_newtillmix_cswat0`, Bab nutrien).
  3. Tanaman tahunan: `curyr_mat` bertambah, maksimum `mat_yrs`.
  4. Belahan bumi utara (lintang >= 0): `phubase` = 0 dan `mgt_new_year`.

#### 7.5.4  `Hru::midyear`

- **Lokasi:** `hru/mod.rs`, `pub fn midyear(&mut self)`
- **Padanan SWAT+:** `time_control.f90` (hari 181)
- **Tujuan:** Di belahan selatan (lintang < 0), `phubase` = 0 dan `mgt_new_year` pada hari 181. Proyek Indonesia karenanya mereset satuan panas dasar di bulan Juli.

#### 7.5.5  `Hru::close_day`

- **Lokasi:** `hru/mod.rs`, `pub fn close_day(&mut self, wb: &mut HruWb, printed: bool)`
- **Padanan SWAT+:** bagian harian `hru_output.f90`
- **Tujuan:** Melengkapi kolom awal/akhir periode dan rata-rata air tanah serta salju.
- **Cara kerja:** `sw_init`, `sno_init` dari HRU; `sw_final` = sw, `sw_ave` = (sw\_init + sw\_final)/2; `sno_final`, `snopack` = (sno\_init + sno\_final)/2. Hanya bila hari dicetak, `sw_init`/`sno_init` HRU digeser ke nilai akhir.
- **Kebiasaan SWAT+ / catatan port:** dengan interval cetak > 1, `sw_init` dan `sw_ave` mencakup seluruh interval.

### 7.6  Air tanah: pencampuran biologis, suhu tanah, evaporasi, CN harian, perkolasi, tile, dan kelebihan jenuh

#### 7.6.1  `Hru::biomix`

- **Lokasi:** `hru/mod.rs`, `fn biomix(&mut self, b: &Biomix, cb: &CarbonBasin)`
- **Padanan SWAT+:** `mgt_biomix` (tanpa olah tanah), bagian air tanah; pool diteruskan ke `biomix_pools` (Bab model karbon)
- **Tujuan:** Pada `cswat` 2, setiap hari lapisan atas yang tidak beku di atas kedalaman pencampuran saling bertukar sebagian air tanah (dan pool, lewat `biomix_pools`).
- **Cara kerja:**
  1. Kedalaman campur D = min(dasar profil, `depth`); keluar bila <= 1e-6; bila < 10 mm menjadi 11 mm.
  2. Efisiensi lapisan: bila ada olah tanah (`till_days` > 0), `bmix` pulih per hari sebesar `bio_consf`·`init_bmix` (dikalikan st/fc bila tanah di bawah kapasitas lapang), maksimum `init_bmix`; pada hari olah tanah (`till_switch` dan `till_days` = 0) `bmix` = 0; selain itu `bmix` = `init_bmix`.
  3. Telusuri lapisan: lapisan beku (tmp <= 1e-6) menghentikan pencampuran dan D menjadi dasar lapisan di atasnya. Lapisan seluruhnya di atas D menyumbang `temperature_factor(bmix, T_l)`·thick; lapisan yang terpotong D menyumbang dengan suhu lapisan di atasnya × (D − d\_prev).
  4. Efisiensi rata-rata ē = Σ/D. Massa tanah lapisan (kg/ha) memakai bulk density lapisan pertama:

```latex
M_l = \frac{thick_l}{1000}\cdot10^4\cdot bd_1\cdot1000\cdot\left(1 - \frac{rock_l}{100}\right)
```

Bagian tercampur M\_mix = ē M\_l (lapisan terpotong: dikali (D − d\_prev)/thick); fraksi distribusi frac\_dep = thick/D atau (D − d\_prev)/D. Air tercampur dan distribusi baliknya:

```latex
W_{mix} = \sum_l \frac{M_{mix,l}}{M_l}\,st_l,\qquad st_l \leftarrow \frac{M_l - M_{mix,l}}{M_l}\,st_l + frac_{dep,l}\,W_{mix}
```

5. Panggil `biomix_pools` untuk tekstur, nutrien, dan pool.

- **Kebiasaan SWAT+ / catatan port:** massa semua lapisan memakai `bd` lapisan pertama; lapisan terpotong memakai suhu lapisan di atasnya; total `sw` tidak dijumlahkan ulang setelah pencampuran.

#### 7.6.2  `Hru::soil_temperature`

- **Lokasi:** `hru/mod.rs`, `fn soil_temperature(&mut self, w: &HruWeather, albday: f32, tmp_an: f32, yrc: i32)`
- **Padanan SWAT+:** `stmp_solt.f90`
- **Tujuan:** Suhu harian setiap lapisan dari suhu permukaan, suhu tahunan rata-rata, dan kedalaman redaman (Neitsch et al. 2009 bagian 1:1.3).
- **Masukan dan keluaran:** `avbd` (g/cm3), `sw` (mm), tutupan, salju, radiasi, suhu; mengubah `tmp` setiap lapisan (°C).
- **Cara kerja:**
  1. Kedalaman redaman maksimum dan aktual (mm):

```latex
DD_{max} = 1000 + \frac{2500\,\rho_b}{\rho_b + 686\,e^{-5.63\rho_b}},\quad sc = \frac{SW}{(0.356 - 0.144\rho_b)\,z_{tot}},\quad DD = DD_{max}\exp\!\left(\ln\frac{500}{DD_{max}}\left(\frac{1 - sc}{1 + sc}\right)^2\right)
```

2. Faktor penutup tanah dari tutupan CV (kg/ha) dan salju:

```latex
bcv = \max\!\left(\frac{CV}{CV + e^{7.563 - 1.297\times10^{-4}CV}},\ \frac{SNO}{SNO + e^{6.055 - 0.3002\,SNO}}\right)
```

(suku salju = 1 bila SNO > 120 mm; hanya dihitung bila ada salju). 3. Suhu permukaan tanah gundul, berpenutup, dan rata-rata:

```latex
T_{bare} = \bar T + \frac{T_{max} - T_{min}}{2}\cdot\frac{H(1 - \alpha) - 14}{20},\quad T_{cov} = bcv\,T_2 + (1 - bcv)T_{bare},\quad T_{srf} = \frac{T_{bare} + T_{cov}}{2}
```

dengan T\_2 suhu lapisan kedua kemarin. 4. Per lapisan, dengan z\_d kedalaman tengah lapisan (mm) dan ℓ = 0,8:

```latex
zd = \frac{z_d}{DD},\quad df = \frac{zd}{zd + \exp(-0.8669 - 2.0775\,zd)},\quad T_l \leftarrow \ell\,T_l + (1 - \ell)\left(df\,(\bar T_{AA} - T_{srf}) + T_{srf}\right)
```

5. Septik aktif (tahunnya tercapai): mulai lapisan biozona ke bawah, suhu < 10 °C diangkat menjadi 10 − 0,1(10 − T).

#### 7.6.3  `Hru::evaporation`

- **Lokasi:** `hru/mod.rs`, `fn evaporation(&mut self, w, pet_day: f32, ep_in: f32, fixes: bool) -> (f32, f32, f32)`
- **Padanan SWAT+:** `et_act.f90`
- **Tujuan:** Membagi PET menjadi evaporasi kanopi, transpirasi maksimum, sublimasi, evaporasi kolam lahan basah, dan evaporasi tanah per lapisan; serta menaikkan NO3 dari lapisan kedua bersama air yang menguap.
- **Masukan dan keluaran:** PET HRU (mm), transpirasi maksimum dari Penman-Monteith (`ep_in`, 0 untuk metode lain). Keluaran (evaporasi kanopi, evaporasi tanah `es_day`, transpirasi maksimum `ep_max`), mm.
- **Cara kerja:**
  1. **Kanopi:** PET' = PET − canstor. Bila negatif: kanopi menyisakan −PET', evaporasi kanopi = PET, PET' = 0, `ep_max` = 0. Bila tidak: semua air kanopi menguap.
  2. Bila PET' > 1e-6, **transpirasi maksimum**: E\_t = LAI·PET'/3 untuk LAI <= 3, selain itu PET'.
  3. **Evaporasi tanah maksimum** dengan indeks tutupan (0,5 bila salju >= 0,5 mm):

```latex
cov_{sol} = \exp\!\left(-5\times10^{-5}(CV + 0.1)\right),\quad E_s = PET'\,cov_{sol},\quad E_s' = \min\!\left(E_s,\ \frac{E_s\,PET'}{E_s + E_t}\right)
```

4. **Bila kolam lahan basah berisi air:** E\_s = evrsv·(1 − LAI/4)·PET' untuk LAI <= 4, selain itu 0 (Sakaguchi et al. 2014); jumlah E\_s + E\_t boleh melebihi PET. **Selain itu**, bila PET < E\_s + E\_t: E\_s = PET − E\_t, lalu bila PET' < E\_s + E\_t, E\_s = PET' E\_s/(E\_s + E\_t) dan E\_t = PET' E\_t/(E\_s + E\_t) dengan E\_s yang *baru diubah*.
5. **Sublimasi:** bila `tave` > 0, salju menguap lebih dulu dari sisa E\_s.
6. **Kolam lahan basah** menguap sebelum tanah; `water_evap` dicatat (mm).
7. **Tanah** (hanya bila tidak ada air kolam >= 1e-6): untuk lapisan yang batas atasnya (lapisan pertama: dasarnya) < 500 mm, kebutuhan kumulatif hingga dasar lapisan dan koreksi `esco`:

```latex
E_z = E_s''\,\frac{z}{z + \exp(2.374 - 0.00713\,z)},\qquad E_l = E_z - E_{z,prev}(1 - esco)
```

Bila st < fc: E\_l × exp(2,5 (st − fc)/fc). Lalu E\_l dibatasi 0,8·st (`etco`), >= 0, <= sisa kebutuhan. Bila st > E\_l, st = max(st − E\_l, 1e-6); selain itu lapisan dikeringkan ke 0. 8. **NO3 naik** pada lapisan kedua: rasio E\_l/st\_2 (maks 1; 0 bila st\_2 <= 1e-3), NO3 naik = 0,05·rasio·NO3\_2, NO3 kedua lapisan dijaga >= 0,0001. 9. `es_day` = max(E\_s − sisa, 0); jumlahkan ulang `sw`.

- **Kebiasaan SWAT+ / catatan port:** (a) `es_max` diubah sebelum `ep_max` dihitung darinya (butir 4); (b) lapisan dengan sisa < 1e-6 mm dipertahankan 1e-6; (c) koefisien lahan basah diambil dari `evrsv_et` (indeks salah SWAT+), atau `evrsv` sendiri pada `--fix-swat`; (d) transpirasi Penman-Monteith ditimpa butir 2 kecuali pada hari tanpa PET tersisa setelah kanopi.

#### 7.6.4  `Hru::sum_sw`

- **Lokasi:** `hru/mod.rs`, `fn sum_sw(&mut self)`
- **Padanan SWAT+:** penjumlahan `soil(j)%sw` di beberapa rutin
- **Tujuan:** `sw` = Σ st lapisan (mm), dijumlahkan berurutan dari atas.

#### 7.6.5  `Hru::daily_cn`

- **Lokasi:** `hru/mod.rs`, `fn daily_cn(&self, cn_froz: f32) -> f32`
- **Padanan SWAT+:** `sq_dailycn.f90`
- **Tujuan:** CN hari ini dari air tanah profil, dengan koreksi tanah beku.
- **Cara kerja:**

```latex
f = \mathrm{clamp}(w_1 - w_2\,SW,\ -20,\ 20),\qquad S = smx\left(1 - \frac{SW}{SW + e^{f}}\right)
```

(S = smx bila SW + e^f <= 0,001). Bila suhu lapisan kedua <= 0 (beku):

```latex
S_{froz} = smx\left(1 - \exp(-cn_{froz}\,S)\right)
```

Lalu S = max(S, 3) dan CN = 25400/(S + 254).

#### 7.6.6  `Hru::percolate`

- **Lokasi:** `hru/mod.rs`, `fn percolate(&mut self, inflpcp: f32, surfq: &mut f32) -> (f32, f32)`
- **Padanan SWAT+:** `swr_percmain.f90` (dengan `swr_percmicro`, `swr_satexcess`, `swr_origtile`), tanpa retakan
- **Tujuan:** Mendistribusikan infiltrasi ke profil dan menghitung perkolasi (storage routing) serta aliran lateral setiap lapisan.
- **Masukan dan keluaran:** air masuk (mm). Keluaran (aliran lateral harian total, perkolasi keluar dasar profil `sepbtm`), mm; mengubah `st`, `prk`, `flat`, `qtile`, dan `surfq` (kelebihan jenuh).
- **Cara kerja:**
  1. Air masuk diproses dalam potongan (`slug`) 1000 mm.
  2. Untuk setiap lapisan dari atas: st += air dari atas; kelebihan = st − fc; bila > 1e-5, `percmicro` memberi perkolasi dan aliran lateral, yang dikurangkan dari st (minimum 1e-6). Perkolasi menjadi masukan lapisan berikutnya; dari lapisan terakhir menjadi `sepbtm`. `flat` = lateral lapisan, `prk` += perkolasi; nilai < 1e-6 menjadi 0.
  3. Setelah semua potongan: `sat_excess`, `sum_sw`, lalu `qtile` = `tile_flow`.
- **Kebiasaan SWAT+ / catatan port:** perkolasi berjalan setiap hari meskipun tanpa hujan (lapisan di atas kapasitas lapang selalu terkuras). Variabel `lyrtile` dipertahankan bernilai 0 (tile lapisan tidak dihitung di sini).

#### 7.6.7  `Hru::percmicro`

- **Lokasi:** `hru/mod.rs`, `fn percmicro(&mut self, ly1: usize, sw_excess: f32) -> (f32, f32)`
- **Padanan SWAT+:** `swr_percmicro.f90`
- **Tujuan:** Perkolasi dan aliran lateral satu lapisan dari air di atas kapasitas lapang (model penyimpanan kinematik, Sloan dan Moore 1984, untuk lateral).
- **Cara kerja:**
  1. Lapisan beku (tmp <= 0): tidak ada aliran.
  2. Tinggi muka air jenuh dan aliran lateral (mm/hari), kecuali lapisan pertama:

```latex
H_o = \frac{2\,SW_{exc}}{(ul - fc)/thick},\qquad Q_{lat} = latq_{co}\;H_o\;K_{sat}\;\frac{slope}{L_{lat}}\;0.024
```

dibatasi \[0, SW\_exc\]. 3. Waktu tempuh TT = (ul − fc)/K\_sat (jam). Biozona septik: opsi 1 (aktif) memakai K\_sep = K (st − fc)/(ul − fc) dibatasi \[1e-6, K\] dan TT = (TT − fc)/K\_sep; opsi 2 (gagal) TT = 1e10. TT minimal 2 jam. 4. Perkolasi:

```latex
w_{perc} = (st - fc)\left(1 - \exp\!\left(-\frac{24}{TT}\right)\right)
```

(>= 0; septik aktif: maksimum 24 K\_sep, dicatat `bz_perc`). Lapisan terakhir dikalikan `perco_lim`. 5. Bila w\_perc + Q\_lat > SW\_exc, keduanya diskalakan proporsional sehingga jumlahnya SW\_exc.

- **Kebiasaan SWAT+ / catatan port:** lapisan pertama tidak menghasilkan aliran lateral; rumus TT septik (`(hk − fc)/K_sep`) mengurangkan `fc` (mm) dari waktu tempuh, ditiru apa adanya dari Fortran.

#### 7.6.8  `Hru::tile_flow`

- **Lokasi:** `hru/mod.rs`, `fn tile_flow(&mut self) -> f32`
- **Padanan SWAT+:** akhir `swr_percmain` (muka air dangkal, `wtable` 0) dan `swr_origtile.f90`
- **Tujuan:** Aliran drain bawah tanah harian (mm) dari air di atas kapasitas lapang.
- **Cara kerja:**
  1. Tanpa drain: 0. Hanya bila lapisan kedua tidak beku.
  2. Muka air dangkal dari dasar profil: bila SW > sumfc,

```latex
y = \max(0.9\,sumul,\ 1.1\,sumfc),\qquad wt_{shall} = \min\!\left(\frac{SW - sumfc}{y - sumfc},\,1\right) z_{mx}
```

selain itu wt\_shall = z\_mx. 3. Dengan d = z\_mx − kedalaman drain: bila wt\_shall > d dan SW > sumfc,

```latex
SW_{exc} = \frac{wt_{shall} - d}{wt_{shall}}(SW - sumfc),\qquad Q_{tile} = \min\!\left(SW_{exc}\left(1 - e^{-24/t_{drain}}\right),\ drain_{co}\right)
```

(tanpa faktor eksponensial bila t\_drain < 1 jam). 4. Air diambil dari lapisan dari atas, hanya bagian di atas fc; bila tidak cukup, Q\_tile dikurangi sisanya. `sw` dijumlahkan ulang.

#### 7.6.9  `Hru::sat_excess`

- **Lokasi:** `hru/mod.rs`, `fn sat_excess(&mut self, surfq: &mut f32)`
- **Padanan SWAT+:** `swr_satexcess.f90` (tanpa simpanan permukaan)
- **Tujuan:** Mengembalikan air di atas kejenuhan ke atas dan, bila lapisan teratas pun penuh, menjadikannya limpasan.
- **Cara kerja:**
  1. Dari lapisan terbawah ke lapisan kedua: kelebihan st − ul dipindahkan ke lapisan di atasnya.
  2. Lapisan pertama: kelebihan dipotong ke ul lalu didorong ke bawah lapisan demi lapisan selama ada lapisan yang masih bisa menampung.
  3. Sisa kelebihan menjadi `surfq`, atau untuk HRU lahan basah diteruskan ke `wet_excess` (Bab lahan basah).

### 7.7  Evapotranspirasi potensial (`et_pot`)

Keempat metode PET berbagi besaran dasar: tekanan uap jenuh dari `climate::ee` (kPa)

```latex
e^{o}(\bar T) = \exp\!\left(\frac{16.78\,\bar T - 116.9}{\bar T + 237.3}\right),\qquad e = e^{o}\,RH,\qquad VPD = e^{o} - e
```

Semua fungsi di subbab ini mengembalikan PET sebelum dikalikan `pet_co` HRU (mm/hari).

#### 7.7.1  `Hru::pet_radiation`

- **Lokasi:** `hru/mod.rs`, `fn pet_radiation(&self, w: &HruWeather, ed: f32) -> (f32, f32)`
- **Padanan SWAT+:** bagian radiasi `et_pot.f90`
- **Tujuan:** Radiasi gelombang pendek bersih untuk tanaman acuan dan radiasi gelombang panjang bersih (MJ/m2/hari).
- **Cara kerja:**

```latex
H_{net,s} = H\,(1 - \alpha_{ref}),\ \alpha_{ref} = \begin{cases}0.23 & sno \le 0.5\\ 0.8 & \text{lainnya}\end{cases};\qquad H_{net,l} = -(0.34 - 0.139\sqrt{e})\left(0.9\frac{H}{H_{mx}} + 0.1\right)4.9\times10^{-9}\,T_K^4
```

(suku awan = 0 bila H\_mx < 1e-4).

- **Kebiasaan SWAT+ / catatan port:** T\_K^4 dihitung sebagai (T\_K²)² karena gfortran mengubah `tk**4` menjadi dua kali kuadrat; urutan perkalian ini menentukan pembulatan.

#### 7.7.2  `Hru::pet_constants`

- **Lokasi:** `hru/mod.rs`, `fn pet_constants(&self, w, ea: f32) -> (f32, f32, f32)`
- **Padanan SWAT+:** bagian awal `et_pot.f90`
- **Tujuan:** Panas laten penguapan λ (MJ/kg), konstanta psikrometrik γ dan kemiringan kurva tekanan uap Δ (kPa/°C).
- **Cara kerja:**

```latex
P = 101.3 - z\,(0.01152 - 0.544\times10^{-6}z),\quad \lambda = 2.501 - 2.361\times10^{-3}\bar T,\quad \gamma = \frac{1.013\times10^{-3}P}{0.622\lambda},\quad \Delta = \frac{4098\,e^{o}}{(\bar T + 237.3)^2}
```

dengan z elevasi HRU (m).

#### 7.7.3  `Hru::pet_priestley_taylor`

- **Lokasi:** `hru/mod.rs`, `fn pet_priestley_taylor(&self, w, ea, ed) -> f32`
- **Padanan SWAT+:** cabang `pet` 0 `et_pot.f90` (Priestley dan Taylor 1972)
- **Cara kerja:**

```latex
PET = \max\!\left(0,\ 1.28\,\frac{\Delta}{\Delta + \gamma}\,\frac{H_{net,s} + H_{net,l}}{\lambda}\right)
```

#### 7.7.4  `Hru::pet_penman_monteith`

- **Lokasi:** `hru/mod.rs`, `fn pet_penman_monteith(&self, w, albday, ea, ed, vpd, b: &HruBasin) -> (f32, f32)`
- **Padanan SWAT+:** cabang `pet` 1 `et_pot.f90` (Monteith 1965; tanaman acuan alfalfa 40 cm sesuai Neitsch et al. 2009)
- **Tujuan:** PET tanaman acuan dan transpirasi maksimum kanopi HRU.
- **Cara kerja:**
  1. Kerapatan-panas ρc\_p = 1710 − 6,85·T; tahanan aerodinamik acuan dan tahanan kanopi acuan dengan CO2 DAS:

```latex
r_a = \frac{114}{u\,(170/1000)^{0.2}},\quad r_c = \frac{49}{1.4 - 0.4\,CO_2/330},\quad PET = \max\!\left(0,\ \frac{\Delta H_{net} + \gamma\,\rho c_p\,VPD/r_a}{\lambda\left(\Delta + \gamma(1 + r_c/r_a)\right)}\right)
```

H\_net di sini memakai albedo acuan (7.7.1). 2. Bila tidak ada tanaman yang tumbuh, `ep_max` = 0. 3. Untuk kanopi nyata (tinggi h = `cht_mx`·100 cm, minimum 1 cm): ketinggian ukur z = 170 cm (h <= 1 m) atau h + 100; angin di z: u\_z = u (z/1000)^0,2; kekasaran z\_om = 0,123 h (h <= 200) atau 0,058 h^1,19; z\_ov = 0,1 z\_om; d = 0,667 h:

```latex
r_a = \frac{\ln\frac{z - d}{z_{om}}\;\ln\frac{z - d}{z_{ov}}}{0.41^2\,u_z}
```

4. Konduktansi stomata per tanaman dikoreksi VPD: g = gsi·max(1 − vpd2 (VPD − 1), 0,1) bila VPD > 1; rata-rata tertimbang LAI dijumlahkan. Tahanan kanopi:

```latex
r_c = \frac{1/(g + 0.01)}{0.5\,(LAI + 0.01)\,(1.4 - 0.4\,CO_2/330)}
```

5. `ep_max` = rumus Penman-Monteith dengan H\_net albedo hari ini (`albday`), r\_a dan r\_c kanopi; dibatasi \[0, PET\].

- **Kebiasaan SWAT+ / catatan port:** (a) r\_c memakai konduktansi tanaman *terakhir* (`gsi_adj` setelah loop), bukan rata-rata tertimbang yang baru dijumlahkan; `--fix-swat` memakai rata-rata tertimbang; (b) angin HRU sudah dinaikkan ke minimal 0,01 m/s oleh loop runner (SWAT+ mengubah dan mencetak angin itu); (c) pangkat `x^0.2` dan `h^1.19` memakai `fortran::rpow` (pemanggilan `powf` yang tidak dioptimasi menjadi fungsi lain) agar sama bit dengan gfortran; (d) CO2 adalah `co2` DAS (konstan tanpa `co2_yr.dat`); (e) `ep_max` ini kemudian ditimpa oleh `et_act` kecuali pada hari tanpa PET tersisa setelah kanopi.

#### 7.7.5  `pet_hargreaves`

- **Lokasi:** `hru/mod.rs`, `fn pet_hargreaves(w: &HruWeather) -> f32` (fungsi bebas, bukan method)
- **Padanan SWAT+:** cabang `pet` 2 `et_pot.f90` (Hargreaves dan Samani 1985)
- **Cara kerja:** radiasi ekstraterestrial setara mm dari radiasi maksimum, H\_0 = H\_mx·37,59/30:

```latex
PET = \max\!\left(0,\ 0.0023\,\frac{H_0}{\lambda}\,(\bar T + 17.8)\,(T_{max} - T_{min})^{0.5}\right)\quad (T_{max} > T_{min};\ \text{selain itu } 0)
```

- **Kebiasaan SWAT+ / catatan port:** `(Tmax − Tmin)^0.5` dihitung dengan `rpow` (`powf`), bukan `sqrt`, karena keduanya berbeda bit terakhir pada sekitar 1 dari 1600 nilai dan satu bit PET cukup untuk menggeser air lapisan tanah dan digit tercetak beberapa minggu kemudian.

### 7.8  HRU perkotaan (`hru_urban`, `hru_urbanhr`)

**`Urban`** (baris `urban.urb`, `urban_db`): `fimp` dan `fcimp` (fraksi kedap air total dan yang terhubung langsung), `curbden` (kerapatan panjang trotoar, km/ha), `urbcoef` (koefisien pencucian, 1/mm), `dirtmx` (padatan maksimum, kg/km trotoar), `thalf` (hari hingga setengah maksimum), `tnconc`, `tpconc`, `tno3conc` (N total, P total, NO3 dalam padatan, mg/kg), dan `ro`. **`UrbanRunoff`**: `BuildupWashoff`, `UsgsReg(kategori hujan)` (persamaan regresi USGS Driver dan Tasker, kategori 0: < 508 mm, 1: < 1016 mm, 2: lebih), dan `None` (tidak ada beban dari area kedap air).

#### 7.8.1  `Hru::hru_urban`

- **Lokasi:** `hru/mod.rs`, `fn hru_urban(&mut self, surfq: f32, phh: f32, precip: f32, nd: &mut NutDay) -> f32`
- **Padanan SWAT+:** `hru_urban.f90` (dengan fungsi `regres`)
- **Tujuan:** Beban padatan dan nutrien dari area kedap air, yang menggantikan bagian `fimp` dari sedimen dan nutrien hari ini. Mengembalikan debu tambahan (`silyld`).
- **Cara kerja (regresi USGS):** hanya bila hujan > 0,1 dan limpasan > 0,1 mm. Untuk konstituen k (padatan tersuspensi, N total, P total) dan kategori hujan:

```latex
Y_k = \frac{\beta_0\left(\frac{R}{25.4}\right)^{\beta_1}\left(\frac{A_{km}f_{imp}}{2.589}\right)^{\beta_2}(100f_{imp} + 1)^{\beta_3}\,\beta_4}{2.205}\quad\text{(kg)}
```

Sedimen = 0,001·Y\_ss·fimp + sed·(1 − fimp); N organik 0,7 dan NO3 0,3 dari Y\_TN/(100 A\_km); P organik 0,75 dan P terlarut 0,25 dari Y\_TP/(100 A\_km), masing-masing ditimbang fimp terhadap nilai lama. `silyld` = 0,001 Y\_ss fimp.

- **Cara kerja (build-up/wash-off):**
  1. Bila limpasan <= 0,1 mm: `twash` += 1 hari, tanpa beban.
  2. Padatan terakumulasi dan pencucian selama durasi limpasan:

```latex
D_0 = \frac{D_{mx}\,t_{wash}}{t_{half} + t_{wash}},\quad k_u = urbcoef\,\frac{3.6\,q_{peak}}{A_{km}},\quad t_{ro} = \min\!\left(\frac{4.605}{-2\ln(1 - \alpha_{0.5})} + t_{conc},\,24\right),\quad D = D_0\,e^{-\min(k_u t_{ro},\,24)}
```

(D < 1e-6 menjadi 0); `twash` = t\_half D/(D\_mx − D). 3. Padatan tercuci SS = max((D\_0 − D)·curbden, 0); TN, TP, NO3 = konsentrasi × SS / 1e6. 4. Sedimen = 0,001 SS fimp + sed (1 − fimp); NO3, N organik (TN − NO3), P organik (0,75 TP), P terlarut (0,25 TP) ditimbang serupa. 5. `silyld` = 0,001·SS·A\_ha·fimp.

- **Kebiasaan SWAT+ / catatan port:** debu perkotaan dalam ton (dikalikan luas), sedangkan sedimen dalam t/ha, persis seperti SWAT+. (Catatan: `docs/swat-quirks.md` masih menyebut `usgs_reg` belum di-port; kode saat ini sudah memuat cabangnya.)

#### 7.8.2  `Hru::hru_urbanhr`

- **Lokasi:** `hru/mod.rs`, `fn hru_urbanhr(&mut self, surfq: f32, ubnrunoff: &[f32], nd: &mut NutDay)`
- **Padanan SWAT+:** `hru_urbanhr.f90`
- **Tujuan:** Versi per langkah waktu dari build-up/wash-off, apa pun `urb_ro`.
- **Cara kerja:** untuk setiap langkah, laju limpasan kedap air q = ubnrunoff·60/Δt (mm/jam). Bila q > 0,025 dan limpasan hari > 0,1 mm: D\_0 seperti di atas, D = D\_0 exp(−urbcoef·q·Δt/60) (dalam f64), `twash` dan SS dihitung dalam f64, lalu NO3, N organik, P organik, P terlarut menggantikan bagian `fimp` (langkah terakhir menimpa yang sebelumnya). Bila tidak, `twash` += Δt/1440.
- **Kebiasaan SWAT+ / catatan port:** `dirt` bertipe double di Fortran sehingga port memakai `f64` untuk D, `twash`, dan SS; NO3 dipotong ke bilangan bulat kg (`tno3` integer); sedimen tidak diganti.

### 7.9  Pengujian unit

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `tests::ascrv_passes_through_its_points` | `hru/mod.rs` (`#[cfg(test)]`) | memeriksa bahwa kurva-S dari `ascrv(0.5, 0.95, 0.5, 0.95)` melalui (0,5; 0,5) dan (0,95; 0,95) dengan galat < 1e-5 |

**Rekap cakupan.** Bab ini mendokumentasikan seluruh 54 fungsi berkas `hru/mod.rs` (dihitung dengan `grep "fn "`): 4 method `columns`, `HruBasin::new`/`with_fixes`, 3 method `Biomix`, 5 method `Row` dan `is_null`, `init_hrus`, 7 fungsi kalibrasi, `soil_awc_init`, `init_hru`, `nutcarb_stage`, `custom_layers`, `soil_layers`, `init_bmix`, `ascrv`, `curno`, `time_conc`, 14 method `Hru` neraca harian (`start_balance`, `day`, `end_year`, `midyear`, `close_day`, `biomix`, `soil_temperature`, `evaporation`, `sum_sw`, `daily_cn`, `percolate`, `tile_flow`, `percmicro`, `sat_excess`), 4 method PET, `hru_urban`, `hru_urbanhr`, `pet_hargreaves`, dan satu uji unit. Rutin yang dipanggil `Hru::day` tetapi didefinisikan di submodul (`erosion`, `sediment_nutrients`, `green_ampt`, `wetland_control`, `pl_grow`, `mgt_operatn`, dll.) dibahas di bab submodulnya.
