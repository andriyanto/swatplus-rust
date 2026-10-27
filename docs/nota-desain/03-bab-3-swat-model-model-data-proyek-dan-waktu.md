## Bab 3  swat-model: model data proyek dan waktu

Bab ini membahas *crate* `swat-model`, yaitu lapisan data murni dari engine. Crate ini tidak membaca file dan tidak menghitung proses hidrologi apa pun: ia hanya mendefinisikan tipe-tipe yang menampung isi sebuah proyek SWAT+ (folder TxtInOut) setelah dibaca. Pembaca di crate `swat-io` mengisi tipe-tipe ini dari file teks, lalu `swat-engine` menjalankan simulasi di atasnya. Karena itu hampir setiap bab berikutnya merujuk ke struct yang dijelaskan di sini: `Project`, `TimeSim`/`SimDay`, `Table`, `BasinParams`, `SpatialObject`, `Soil`, `MgtSchedule`, `DecisionTable`, dan seterusnya.

File yang dicakup: `crates/swat-model/src/lib.rs`, `time.rs`, `table.rs`, `schema.rs`, `cio.rs`, `basin.rs`, `climate.rs`, `hru.rs`, `cal.rs`, `mgt.rs`, `objects.rs`, `routing.rs`, dan `print.rs`. Tiga prinsip desain berlaku di seluruh crate dan dinyatakan di komentar modul `lib.rs`: (1) nama field mengikuti nama SWAT+ di tempat pembaca kode Fortran akan mencarinya, dan setiap tipe menyebut file input serta rutin Fortran yang dicerminkannya; (2) bilangan real disimpan sebagai `f32`, karena `real` bawaan SWAT+ adalah presisi tunggal (keputusan K4 dalam dokumen desain RustGIS, `docs/swat/design.md`), sehingga nilai yang dibaca identik bit demi bit dengan nilai di memori program Fortran; (3) crate ini dikompilasi dengan larangan `unwrap`, `expect`, `panic`, `unimplemented`, dan `unreachable` di luar kode uji (atribut `deny(clippy::...)`), sehingga semua akses yang dapat gagal mengembalikan `Option`.

Dalam alur simulasi, crate ini berada di antara tahap pembacaan input (setara dengan `readcio_read`, `time_read`, `basin_read_*`, `hyd_read_connect`, `cli_*read`, `soil_db_read`, `mgt_read_*`, dan sebagainya di SWAT+) dan tahap inisialisasi objek di engine. Satu-satunya "logika" yang ada di sini adalah logika yang di SWAT+ dijalankan tepat setelah pembacaan: normalisasi periode simulasi (`time_read`), penurunan daftar hari simulasi (`time_control`), konversi hari Julian ke bulan dan tanggal (`xmon`), pengisian nilai bawaan parameter basin (`basin_prm_default`), ekspansi daftar elemen (`define_unit_elements`), dan pencarian baris tabel menurut nama dengan aturan perbandingan string Fortran. Jumlah fungsi yang didokumentasikan di bab ini: 42 fungsi non-uji ditambah 6 fungsi uji unit.

### 3.1  `lib.rs`: tipe akar `Project` dan tipe pendukung

`lib.rs` mendeklarasikan modul-modul crate, mengekspor ulang tipe-tipe utamanya (`pub use`), dan mendefinisikan tipe akar `Project` beserta lima tipe kecil yang tidak punya modul sendiri: `SepticSys`, `RecallDb`, `RecallRow`, `AtmoDep`, dan `AtmoStation`.

**Struct `Project`.** Satu nilai `Project` adalah seluruh proyek SWAT+ sejauh pembaca sudah mendukungnya. Semua field bersifat `pub`; `Project` menurunkan `Clone`, `Debug`, `Default`, dan `PartialEq`. Field dapat dikelompokkan sebagai berikut.

| Kelompok | Field | Isi dan file sumber |
| --- | --- | --- |
| Kendali | `cio: FileCio` | `file.cio`, peta slot ke nama file (subbab 3.5) |
| Kendali | `time: TimeSim` | `time.sim`, periode simulasi (subbab 3.2) |
| Kendali | `print: PrintSettings` | `print.prt` (subbab 3.13) |
| Basin | `codes: BasinCodes`, `params: BasinParams` | `codes.bsn`, `parameters.bsn` (subbab 3.6) |
| Basin | `carbon`, `carbon_lyr: Option<Table>` | `carbon.bsn` (satu baris) dan `carbon_lyr.bsn` (satu baris per jenis lapisan; namanya diturunkan dari nama `carbon.bsn`) |
| Basin | `basin_name: String`, `area_ls_ha`, `area_tot_ha: f32` | nama basin dan luas daratan serta luas total (ha) dari `object.cnt` (`bsn`) |
| Objek | `objects: Vec<SpatialObject>` | semua baris semua file `*.con` (subbab 3.11) |
| Iklim | `stations: Vec<WeatherStation>`, `wgn: Vec<Wgn>` | `weather-sta.cli`, `weather-wgn.cli` |
| Iklim | `station_lists: Vec<StationList>`, `series: Vec<StationSeries>` | daftar stasiun per variabel terukur (`pcp.cli`, `tmp.cli`, …) dan isi file stasiun terukur sesuai urutan daftar |
| Iklim | `atmodep: Option<AtmoDep>` | `atmo.cli` (deposisi atmosfer) |
| Iklim | `co2_yr_dat: bool` | apakah proyek memiliki `co2_yr.dat` (CO2 tahunan, dibaca SWAT+ oleh `co2_read`; menurut komentar kode belum ditangani) |
| HRU | `hru_data: Vec<HruData>` | `hru-data.hru`, diindeks dengan nomor HRU − 1 (SWAT+ menempatkan baris menurut nomornya) |
| HRU | `soils: Vec<Soil>`, `soil_plant_init: Vec<SoilPlantInit>`, `soil_test: Option<Table>` | `soils.sol`, `soil_plant.ini`, `nutrients.sol` |
| HRU | `soil_lyr_depths: Option<Vec<i32>>` | `soil_lyr_depths.sol` (kedalaman lapisan kustom, mm) bila ada |
| HRU | `topography`, `hydrology`, `field`, `snow: Option<Table>` | `topography.hyd`, `hydrology.hyd`, `field.fld`, `snow.sno` |
| Tutupan lahan | `landuse`, `cntable`, `ovn_table`, `cons_practice`, `urban`, `plants`, `tillage: Option<Table>` | `landuse.lum`, `cntable.lum`, `ovn_table.lum`, `cons_practice.lum`, `urban.urb`, `plants.plt`, `tillage.til` |
| Tutupan lahan | `plant_communities: Vec<PlantCommunity>` | `plant.ini` |
| Pengelolaan | `management: Vec<MgtSchedule>`, `lum_dtl: Vec<DecisionTable>` | `management.sch`, `lum.dtl` |
| Pengelolaan | `harv_ops`, `fertilizer`, `irr_ops`, `chem_app: Option<Table>` | `harv.ops`, `fertilizer.frt`, `irr.ops`, `chem_app.ops` |
| Pengelolaan | `puddle_ops`, `manure_db`, `manure_om`, `transplant_ops: Option<Table>` | dibaca dengan nama tetap ini apa pun isi `file.cio`: `puddle.ops`, `manure_db.frt`, `manure_om.frt`, dan keberadaan `transplant.ops` |
| Waduk/lahan basah | `wetland`, `wet_hyd`, `res_init`, `res_sed`, `res_nut`, `weir_res`, `res_dat`, `res_hyd: Option<Table>`; `res_dtl: Vec<DecisionTable>` | `wetland.wet`, `hydrology.wet`, `initial.res`, `sediment.res`, `nutrients.res`, `weir.res`, `reservoir.res`, `hydrology.res`, tabel rilis `res_rel.dtl` |
| Struktur | `tiledrain`, `fstrip`, `grassww`, `bmpuser`, `septic_sep: Option<Table>`; `septic: Vec<SepticSys>` | `tiledrain.str`, `filterstrip.str`, `grassedww.str`, `bmpuser.str`, `septic.sep`, `septic.str` (baris 0 adalah elemen kosong 0 milik SWAT+) |
| Recall | `recalls: Vec<RecallDb>` | `recall.rec` dan file-file yang disebutnya (`recall_read`) |
| Lanskap/routing | `lsu_defs: Vec<LsuDef>`, `lsu_elems: Vec<LsuElem>`, `aqu_elems: bool` | `ls_unit.def`, `ls_unit.ele` (indeks = nomor baris, elemen 0 tidak dipakai; kosong bila file tidak ada) dan apakah `aqu_catunit.ele` ada (diperlukan tabel akuifer basin) |
| Lanskap/routing | `ru_elements: Vec<RuElement>`, `ru_defs: Vec<RuDef>`, `ru_data: Vec<RuData>` | `rout_unit.ele` (elemen 0 tidak dipakai), `rout_unit.def`, `rout_unit.rtu` |
| Akuifer | `aquifer_db: Vec<AquiferDb>` | `aquifer.aqu` menurut nomor baris; elemen 0 menampung nilai bawaan |
| Kanal | `sd_channel_data: Vec<SdChannelData>`; `hyd_sed_lte`, `ch_nut`, `ch_init`, `om_water: Option<Table>`; `ch_temp_file: bool` | `channel-lte.cha` (elemen 0 tidak dipakai), `hyd-sed-lte.cha`, `nutrients.cha`, `initial.cha`, `om_water.ini`, dan apakah `temperature.cha` disebut |
| Kalibrasi | `cal_parms: Vec<CalParm>`, `cal_upd: Vec<CalUpdate>` | `cal_parms.cal`, `calibration.cal` (subbab 3.9) |

Pola umum: tabel basis data "satu tipe turunan per baris" disimpan sebagai `Option<Table>` (generik, skema di `schema.rs`); `None` berarti file tidak ada atau slotnya `null`. Struktur dengan bentuk khusus (jumlah lapisan bervariasi, daftar operasi, tabel keputusan) memiliki struct sendiri. Beberapa vektor sengaja memakai indeks berbasis 1 dengan elemen 0 kosong atau berisi nilai bawaan, meniru larik Fortran yang dialokasikan dari 0.

**Struct `SepticSys`** (baris `septic.str`, tipe Fortran `septic_system`, dibaca oleh `sep_read`):

| Field | Tipe | Arti |
| --- | --- | --- |
| `name` | `String` | nama sistem septik |
| `typ` | `i32` | jenis efluen: nomor baris di `septic.sep` |
| `yr` | `i32` | tahun pertama sistem beroperasi |
| `opt` | `i32` | status operasi: 0 tidak dioperasikan, 1 aktif, 2 gagal |
| `tfail` | `i32` | jumlah hari sistem yang gagal sampai diperbaiki |
| `r` | `[f32; 23]` | 23 bilangan real berurutan: `cap`, `area`, `z`, `thk`, `strm_dist`, `density`, `bd`, `bod_dc`, `bod_conv`, `fc1`, `fc2`, `fecal`, `plq`, `mrt`, `rsp`, `slg1`, `slg2`, `nitr`, `denitr`, `pdistrb`, `psorpmax`, `solpslp`, `solpintc` |

`SepticSys` juga mendefinisikan konstanta indeks untuk larik `r` agar kode engine tidak memakai angka ajaib: `CAP` = 0, `Z` = 2, `THK` = 3, `BD` = 6, `BOD_DC` = 7, `BOD_CONV` = 8, `FC1` = 9, `FC2` = 10, `FECAL` = 11, `PLQ` = 12, `MRT` = 13, `RSP` = 14, `SLG1` = 15, `SLG2` = 16, `NITR` = 17, `DENITR` = 18, `PDISTRB` = 19, `PSORPMAX` = 20, `SOLPSLP` = 21, `SOLPINTC` = 22. Satuan masing-masing besaran mengikuti `septic.str` SWAT+; proses biozona septik dibahas di bab tentang HRU.

**Struct `RecallDb`** (satu objek *recall*: beban terukur atau hasil model): `id: usize` (nomor baris di `recall.rec`), `name: String`, `typ: i32` (langkah waktu: 0 subharian, 1 harian, 2 bulanan, 3 tahunan, 4 koefisien ekspor), `file: String`, dan `rows: Vec<RecallRow>`.

**Struct `RecallRow`** (satu rekaman file recall): `jday` (hari dalam tahun), `mo` (bulan), `day_mo` (tanggal), `iyr` (tahun), semuanya `i32`, dan `vals: [f32; 18]`, yaitu 18 nilai sebuah hidrograf dalam urutan `hyd_output` SWAT+: debit `flo` (m3), sedimen `sed` (t), N organik `orgn`, P organik `sedp`, `no3`, `solp`, klorofil-a `chla`, `nh3`, `no2`, `cbod`, oksigen terlarut `dox` (kg), fraksi sedimen `san`, `sil`, `cla`, `sag`, `lag`, `grv` (t), dan suhu `temp` (°C).

**Struct `AtmoDep`** (`atmo.cli`, dibaca SWAT+ oleh `cli_read_atmodep`): `timestep: String` (`aa` rata-rata tahunan, `yr` tahunan, atau `mo` bulanan), `mo_init: i32` dan `yr_init: i32` (bulan dan tahun awal deret), `num: usize` (jumlah nilai per deret), dan `stations: Vec<AtmoStation>`.

**Struct `AtmoStation`**: `name: String` dan empat deret `Vec<f32>`: `nh4_rf` dan `no3_rf` (amonium dan nitrat dalam air hujan, mg/L), serta `nh4_dry` dan `no3_dry` (deposisi kering, kg/ha per tahun). Setiap deret berisi satu nilai per langkah waktu (satu nilai saja untuk `aa`).

#### 3.1.1  `Project::objects_of`

- **Lokasi:** `crates/swat-model/src/lib.rs`, `pub fn objects_of(&self, kind: ObjectKind) -> impl Iterator<Item = &SpatialObject>`
- **Padanan SWAT+:** tidak ada; khusus port. Di SWAT+ setiap jenis objek punya larik sendiri (`ob` dengan rentang per jenis), sedangkan port menyimpan semua objek dalam satu vektor.
- **Tujuan:** Mengiterasi objek spasial dari satu jenis saja (misalnya semua HRU atau semua kanal SWAT-DEG).
- **Masukan dan keluaran:** jenis objek `kind`; keluaran berupa iterator referensi `SpatialObject` dalam urutan file.
- **Cara kerja:**
  1. Mengambil iterator atas `self.objects`.
  2. Menyaring elemen dengan `o.kind == kind`.
- **Catatan port:** urutan dipertahankan sesuai urutan baris dalam file `*.con`, yang menentukan nomor objek di SWAT+; karena itu engine dapat bergantung pada urutan iterator ini.

### 3.2  `time.rs`: periode simulasi dan kalender

Modul ini mencerminkan `time.sim` dan bagian kalender dari `time_module`, `time_read`, `time_control`, dan `xmon` di SWAT+. Ia menentukan hari-hari mana yang disimulasikan, dalam urutan apa, dan bagaimana hari Julian dipetakan ke bulan dan tanggal. Seluruh loop harian engine (bab tentang *runner*) berjalan di atas daftar `SimDay` yang dihasilkan di sini.

**Konstanta.**

| Konstanta | Nilai | Padanan SWAT+ |
| --- | --- | --- |
| `NDAYS_LEAP: [i32; 13]` | 0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335, 366 | `ndays_leap` di `time_module` |
| `NDAYS_NOLEAP: [i32; 13]` | 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365 | `ndays_noleap` |

Kedua larik adalah hari kumulatif pada akhir setiap bulan: elemen ke-*m* (berbasis 0) adalah hari Julian terakhir bulan ke-*m* (dengan elemen 0 = 0 sebagai "akhir bulan ke-0"), dan elemen ke-12 adalah panjang tahun.

**Struct `TimeSim`** (isi `time.sim` apa adanya; `Copy`, `Eq`, `Default`):

| Field | Tipe | Arti |
| --- | --- | --- |
| `day_start` | `i32` | hari Julian pertama yang disimulasikan pada tahun pertama (0 atau kurang = 1 Januari) |
| `yrc_start` | `i32` | tahun kalender pertama |
| `day_end` | `i32` | hari Julian terakhir pada tahun terakhir (0 atau kurang = 31 Desember) |
| `yrc_end` | `i32` | tahun kalender terakhir |
| `step` | `i32` | jumlah langkah waktu per hari (0 atau kurang = harian, dinormalkan menjadi 1); nilai > 1 mengaktifkan mode subharian |

**Struct `SimDay`** (satu hari simulasi; `Copy`, `Eq`):

| Field | Tipe | Arti |
| --- | --- | --- |
| `year` | `i32` | tahun kalender |
| `jday` | `i32` | hari dalam tahun, 1 = 1 Januari |
| `month` | `u8` | bulan, 1–12 |
| `day` | `u8` | tanggal dalam bulan |
| `year_index` | `i32` | tahun simulasi ke-berapa, 1 = tahun pertama (SWAT+ `time%yrs`) |

#### 3.2.1  `TimeSim::normalized`

- **Lokasi:** `crates/swat-model/src/time.rs`, `pub fn normalized(self) -> Self`
- **Padanan SWAT+:** baris setelah pembacaan di `time_read` (`time_read.f90`).
- **Tujuan:** Menghasilkan periode sebagaimana SWAT+ menggunakannya setelah membaca `time.sim`.
- **Masukan dan keluaran:** `TimeSim` mentah; keluaran `TimeSim` dengan `step` dan `day_start` yang sudah dinormalkan, field lain tidak berubah.
- **Cara kerja:**
  1. Bila `step <= 0`, `step` menjadi 1 (harian).
  2. Bila `day_start <= 0`, `day_start` menjadi 1 (1 Januari).
  3. `day_end`, `yrc_start`, dan `yrc_end` disalin apa adanya; `day_end = 0` tetap 0 dan ditafsirkan oleh `days()` sebagai "sampai akhir tahun".
- **Kebiasaan SWAT+ / catatan port:** fungsi bersifat murni (mengambil dan mengembalikan nilai salinan), sehingga `TimeSim` yang tersimpan di `Project` tetap merepresentasikan isi file. Proyek Ames\_sub1 menulis `day_start` = 0 dan `day_end` = 0; normalisasi inilah yang membuatnya berarti "tahun penuh".

#### 3.2.2  `TimeSim::years`

- **Lokasi:** `crates/swat-model/src/time.rs`, `pub fn years(&self) -> i32`
- **Padanan SWAT+:** `time%nbyr = time%yrc_end - time%yrc_start + 1` di `time_read`.
- **Tujuan:** Jumlah tahun kalender yang disimulasikan (termasuk tahun *warm-up* yang tidak dicetak).
- **Masukan dan keluaran:** `yrc_start`, `yrc_end`; keluaran jumlah tahun (dapat 0 atau negatif bila `yrc_end < yrc_start`).
- **Cara kerja:**

```latex
n_{byr} = yrc_{end} - yrc_{start} + 1
```

- **Catatan port:** nilai negatif tidak dipotong di sini; pemanggil `days()` memotongnya dengan `max(0)`.

#### 3.2.3  `TimeSim::days`

- **Lokasi:** `crates/swat-model/src/time.rs`, `pub fn days(&self) -> Vec<SimDay>`
- **Padanan SWAT+:** penentuan hari awal dan akhir per tahun di loop `do curyr = 1, time%nbyr` dalam `time_control` (`time_control.f90`), ditambah `xmon` untuk bulan dan tanggal.
- **Tujuan:** Membentuk daftar lengkap hari simulasi, berurutan, persis seperti yang dikunjungi loop tahunan dan harian SWAT+.
- **Masukan dan keluaran:** `TimeSim` (dinormalkan secara internal); keluaran `Vec<SimDay>`.
- **Cara kerja:**
  1. `t = self.normalized()`; `years = max(t.years(), 0)`.
  2. Untuk `year_index` = 1 … `years`: `year = yrc_start + year_index − 1`; ambil tabel bulan kumulatif `ndays = month_table(year)` dan `year_len = ndays[12]` (365 atau 366).
  3. Hari pertama: `day_start` pada tahun pertama, 1 pada tahun-tahun berikutnya.
  4. Hari terakhir: pada tahun terakhir dan bila `day_end > 0`, `min(day_end, year_len)`; selain itu `year_len`.
  5. Untuk setiap `jday` dari hari pertama sampai terakhir, hitung `(month, day)` dengan `month_day_in(&ndays, jday)` dan tambahkan `SimDay { year, jday, month, day, year_index }`.
- **Kebiasaan SWAT+ / catatan port:**
  - Pemotongan `min(day_end, year_len)` meniru `amin0 (time%day_end_yr, ndays(13))` di SWAT+, yang melindungi dari masukan 366 pada tahun bukan kabisat.
  - Bila `day_start` melampaui panjang tahun pertama atau `day_end` lebih kecil dari hari pertama pada tahun tunggal, rentang menjadi kosong dan tidak ada hari yang dihasilkan untuk tahun itu.
  - Penentuan tahun kabisat di loop utama `time_control` mengikuti aturan Gregorian penuh (habis dibagi 4, kecuali abad yang tidak habis dibagi 400), dan itulah yang dipakai di sini. Perlu dicatat bahwa SWAT+ di awal `time_control` (sebelum loop, untuk membangkitkan presipitasi hari pertama) memakai uji `Mod(yrc,4) == 0` saja; perbedaan ini tidak berpengaruh untuk tahun 1901–2099 dan tidak dicerminkan di fungsi ini.
  - Daftar dibentuk di muka (bukan iterator malas) agar *runner* dapat mengetahui jumlah hari total dan hari terakhir setiap tahun/bulan sebelum loop dimulai.

#### 3.2.4  `is_leap`

- **Lokasi:** `crates/swat-model/src/time.rs`, `pub fn is_leap(year: i32) -> bool`
- **Padanan SWAT+:** uji `Mod` bertingkat di `time_control.f90`.
- **Tujuan:** Menentukan apakah `year` adalah tahun kabisat Gregorian.
- **Cara kerja:** benar bila `(year mod 4 = 0 dan year mod 100 ≠ 0)` atau `year mod 400 = 0`. Contoh dari uji: 2000 dan 2024 kabisat; 1900 dan 2023 bukan.

#### 3.2.5  `month_table`

- **Lokasi:** `crates/swat-model/src/time.rs`, `pub fn month_table(year: i32) -> [i32; 13]`
- **Padanan SWAT+:** pemilihan `ndays = ndays_leap` atau `ndays = ndays_noleap` di `time_control`.
- **Tujuan:** Mengembalikan tabel hari kumulatif akhir bulan yang berlaku untuk `year`.
- **Cara kerja:** `NDAYS_LEAP` bila `is_leap(year)`, selain itu `NDAYS_NOLEAP`.

#### 3.2.6  `month_day`

- **Lokasi:** `crates/swat-model/src/time.rs`, `pub fn month_day(year: i32, jday: i32) -> Option<(u8, u8)>`
- **Padanan SWAT+:** subrutin `xmon` (`xmon.f90`).
- **Tujuan:** Mengubah hari Julian `jday` pada tahun `year` menjadi (bulan, tanggal).
- **Masukan dan keluaran:** tahun dan hari Julian; keluaran `Some((bulan 1–12, tanggal))`, atau `None` bila `jday < 1` atau melewati panjang tahun.
- **Cara kerja:** memanggil `month_day_in(&month_table(year), jday)`.
- **Catatan port:** `xmon` di SWAT+ bekerja pada larik `ndays` global yang sedang aktif dan tidak punya keluaran "tidak valid"; versi Rust menerima tahun secara eksplisit dan mengembalikan `None` untuk masukan di luar rentang. Contoh dari uji unit: (2023, 59) → (2, 28); (2023, 60) → (3, 1); (2024, 60) → (2, 29); (2024, 366) → (12, 31); (2023, 366) → `None`; (2023, 0) → `None`.

#### 3.2.7  `month_day_in` (privat)

- **Lokasi:** `crates/swat-model/src/time.rs`, `fn month_day_in(ndays: &[i32; 13], jday: i32) -> Option<(u8, u8)>`
- **Padanan SWAT+:** isi loop `do i_mo = 1, 12` di `xmon.f90`.
- **Tujuan:** Inti konversi hari Julian ke bulan/tanggal terhadap tabel kumulatif tertentu.
- **Cara kerja:**
  1. Bila `jday < 1`, kembalikan `None`.
  2. Untuk `mo` = 1 … 12, cari bulan pertama dengan `jday <= ndays[mo]`.
  3. Tanggal dihitung sebagai

```latex
day = jday - ndays[mo-1]
```

4. Bila tidak ada bulan yang memenuhi (hari melampaui `ndays[12]`), kembalikan `None`.

- **Catatan port:** akses larik memakai `get` (bukan indeks langsung) agar tidak ada jalur *panic*, sesuai larangan clippy crate ini.

#### 3.2.8  Uji unit `time.rs`

| Fungsi uji | Lokasi | Yang diuji |
| --- | --- | --- |
| `leap_years` | `time.rs`, modul `tests` | `is_leap` untuk 2000, 2024, 1900, 2023 |
| `xmon` | `time.rs`, modul `tests` | `month_day` di batas bulan Februari/Maret, akhir tahun kabisat, dan masukan tidak valid |
| `whole_years` | `time.rs`, modul `tests` | 2010–2023 dengan `day_end` 365 menghasilkan 14 × 365 + 3 hari (tiga tahun kabisat), hari pertama (2010, 1), hari terakhir (2023, 365, 12, 31), dan `step` 0 dinormalkan menjadi 1 |
| `zeros_mean_whole_years` | `time.rs`, modul `tests` | pola Ames\_sub1: `day_start` = `day_end` = 0 pada 1975–1976 memberi 365 + 366 hari, hari terakhir (1976, 366) dengan `year_index` 2 |
| `partial_first_and_last_year` | `time.rs`, modul `tests` | tahun pertama mulai hari 100 dan tahun terakhir berakhir hari 10; `day_end` 366 pada tahun bukan kabisat dipotong menjadi 365; tahun akhir sebelum tahun awal memberi daftar kosong |

### 3.3  `table.rs`: tabel generik "satu tipe turunan per baris"

Banyak file basis data SWAT+ dibaca dengan pola yang sama: hitung jumlah baris, alokasikan larik tipe turunan, lalu `read (unit, *) db(i)` untuk setiap baris, yaitu satu pembacaan *list-directed* yang mengisi semua komponen tipe secara berurutan. Daripada membuat satu struct Rust untuk setiap tipe Fortran itu, port menyimpan tabel-tabel ini secara generik: sebuah *skema* (daftar `Field` yang disalin dari definisi tipe Fortran, termasuk nilai bawaannya) dan baris-baris berupa `Vec<Value>` yang komponennya diratakan (larik dalam tipe diekspansi menjadi beberapa nilai). Pencarian baris menurut nama mengikuti perilaku SWAT+: baris pertama yang namanya sama, dibandingkan seperti Fortran membandingkan string.

**Enum `Kind`** (jenis komponen): `Int` (integer Fortran, disimpan `i32`), `Real` (`real` presisi tunggal, `f32`), `Text` (`character`, `String`).

**Struct `Field`** (satu komponen tipe turunan; `Copy`): `name: &'static str` (nama komponen seperti di Fortran), `kind: Kind`, `default: &'static str` (nilai bawaan persis seperti tertulis di kode Fortran, misalnya `".02"` atau `"50."`), dan `count: usize` (panjang larik; 1 untuk skalar).

**Enum `Value`** (satu nilai sel): `Int(i32)`, `Real(f32)`, `Text(String)`.

**Struct `Table`**: `file: String` (nama file sumber, untuk pesan dan diagnosis), `fields: &'static [Field]` (skema; selalu salah satu konstanta di `schema.rs`), dan `rows: Vec<Vec<Value>>` (baris dalam urutan file; setiap baris memiliki panjang Σ `count`).

Pembaca di `swat-io` (`read_table`, `fill`) memulai setiap baris dari `default_row`, lalu menimpa nilai satu per satu sampai menemukan nilai yang tidak dapat dikonversi; komponen sesudahnya tetap bernilai bawaan. Ini meniru gfortran, di mana pembacaan *list-directed* yang gagal berhenti di tengah tipe dan komponen sisanya mempertahankan inisialisasi tipe.

#### 3.3.1  `f`

- **Lokasi:** `crates/swat-model/src/table.rs`, `pub const fn f(name: &'static str, kind: Kind, default: &'static str) -> Field`
- **Padanan SWAT+:** deklarasi komponen skalar dalam `type ... end type` (mis. `real :: slope = .02`).
- **Tujuan:** Konstruktor ringkas komponen skalar untuk menulis skema di `schema.rs`.
- **Cara kerja:** mengembalikan `Field { name, kind, default, count: 1 }`. Karena `const fn`, skema dapat dibangun sebagai konstanta waktu kompilasi.

#### 3.3.2  `a`

- **Lokasi:** `crates/swat-model/src/table.rs`, `pub const fn a(name, kind, default, count: usize) -> Field`
- **Padanan SWAT+:** komponen larik, mis. `real, dimension(4) :: cn` pada `curvenumber_table`.
- **Tujuan:** Konstruktor komponen larik dengan `count` nilai yang semuanya berbawaan sama.
- **Cara kerja:** mengembalikan `Field { name, kind, default, count }`.
- **Catatan port:** untuk `cntable.lum`, nilai bawaan Fortran adalah (30, 55, 70, 77) yang berbeda per elemen; karena `a` hanya mendukung satu nilai bawaan, skema menulis `0.` dan engine menangani nilai bawaan Fortran saat nama CN tidak ditemukan (lihat komentar `CNTABLE`).

#### 3.3.3  `default_row`

- **Lokasi:** `crates/swat-model/src/table.rs`, `pub fn default_row(fields: &[Field]) -> Vec<Value>`
- **Padanan SWAT+:** inisialisasi komponen tipe turunan saat larik dialokasikan.
- **Tujuan:** Membuat satu baris baru yang berisi nilai bawaan semua komponen, diratakan.
- **Masukan dan keluaran:** skema; keluaran vektor `Value` sepanjang Σ `count`.
- **Cara kerja:**
  1. Untuk setiap `Field`, ulangi `count` kali.
  2. `Int`: `default.trim()` diparse sebagai `i32`, gagal → 0.
  3. `Real`: diparse sebagai `f32`, gagal → 0.0. Penulisan gaya Fortran seperti `".02"` dan `"50."` diterima pengurai float Rust.
  4. `Text`: string bawaan disalin apa adanya (termasuk spasi, mis. `" "` untuk `fertnm`).
- **Catatan port:** parsing ke `f32` langsung dari teks desimal memberi nilai yang sama dengan inisialisasi konstanta `real` di gfortran (pembulatan ke nilai presisi tunggal terdekat).

#### 3.3.4  `Value::as_real`, `Value::as_int`, `Value::as_text`

- **Lokasi:** `crates/swat-model/src/table.rs`; `as_real(&self) -> Option<f32>`, `as_int(&self) -> Option<i32>`, `as_text(&self) -> Option<&str>`
- **Padanan SWAT+:** tidak ada; khusus port (akses bertipe ke sel generik).
- **Tujuan:** Mengambil isi sel dengan tipe yang diharapkan.
- **Cara kerja:**
  1. `as_real`: `Real(v)` → `Some(v)`; `Int(v)` → `Some(v as f32)` (konversi integer ke real, seperti penugasan campuran di Fortran); `Text` → `None`.
  2. `as_int`: hanya `Int(v)` → `Some(v)`; `Real` sengaja tidak dikonversi (menghindari pemotongan diam-diam).
  3. `as_text`: hanya `Text`.

#### 3.3.5  `Table::empty`

- **Lokasi:** `crates/swat-model/src/table.rs`, `pub fn empty(file: &str, fields: &'static [Field]) -> Self`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** Membuat tabel tanpa baris untuk file dan skema tertentu; titik awal pembaca.

#### 3.3.6  `Table::offset`

- **Lokasi:** `crates/swat-model/src/table.rs`, `pub fn offset(&self, field: &str) -> Option<usize>`
- **Padanan SWAT+:** tidak ada; setara dengan alamat komponen dalam tipe turunan.
- **Tujuan:** Posisi nilai pertama komponen `field` dalam baris yang sudah diratakan.
- **Cara kerja:** menjumlahkan `count` semua komponen sebelum komponen yang namanya sama persis dengan `field`; `None` bila nama tidak ada di skema.
- **Catatan port:** pencarian linear per akses; skema terpanjang (`PLANTS`, 56 nilai) masih kecil sehingga biaya ini tidak signifikan dan engine umumnya menyalin nilai ke struct bertipe saat inisialisasi.

#### 3.3.7  `Table::get`

- **Lokasi:** `crates/swat-model/src/table.rs`, `pub fn get(&self, row: usize, field: &str, index: usize) -> Option<&Value>`
- **Tujuan:** Nilai ke-`index` (berbasis 0) dari komponen `field` pada baris `row` (berbasis 0).
- **Cara kerja:** `offset(field) + index`, lalu akses aman `rows.get(row)?.get(at)`.
- **Catatan port:** `index` tidak dibatasi oleh `count`; indeks yang melewati larik akan membaca komponen berikutnya. Pemanggil bertanggung jawab memakai indeks yang benar (mis. 0–3 untuk `cn`).

#### 3.3.8  `Table::real`, `Table::int`, `Table::text`

- **Lokasi:** `crates/swat-model/src/table.rs`; `real(&self, row, field) -> Option<f32>`, `int(...) -> Option<i32>`, `text(...) -> Option<&str>`
- **Tujuan:** Jalan pintas untuk komponen skalar: `get(row, field, 0)` lalu `as_real`/`as_int`/`as_text`.

#### 3.3.9  `Table::find`

- **Lokasi:** `crates/swat-model/src/table.rs`, `pub fn find(&self, name: &str) -> Option<usize>`
- **Padanan SWAT+:** loop pencarian nama yang tersebar di rutin inisialisasi, berbentuk `do i = 1, db_mx%... ; if (nama == db(i)%name) then ... exit`.
- **Tujuan:** Indeks baris pertama yang komponen pertamanya (nama) sama dengan `name`.
- **Cara kerja:**
  1. Iterasi baris dari indeks 0.
  2. Ambil nilai pertama baris sebagai teks.
  3. Bandingkan dengan `fortran_eq`; kembalikan indeks pertama yang cocok.
- **Kebiasaan SWAT+ / catatan port:** bila nama ganda, baris pertama yang menang, seperti loop SWAT+ yang memakai `exit`. Di beberapa tempat SWAT+ tidak keluar dari loop sehingga yang menang adalah kecocokan terakhir; untuk kasus-kasus itu engine tidak memakai `find` tetapi pencarian mundur (`rev().find`/`rfind` di `swat-engine/src/hru/mgt.rs`). Untuk skema yang komponen pertamanya bukan nama (mis. `WETLAND` dan `RES_DAT` dimulai dengan `id` integer), `find` tidak menemukan apa pun; pemanggil memakai `text(row, "name")`.

#### 3.3.10  `fortran_eq`

- **Lokasi:** `crates/swat-model/src/table.rs`, `pub fn fortran_eq(a: &str, b: &str) -> bool`
- **Padanan SWAT+:** semantik operator `==` pada `character` Fortran: string yang lebih pendek dianggap diisi spasi di kanan, sehingga spasi di ujung tidak berpengaruh.
- **Tujuan:** Membandingkan nama seperti Fortran.
- **Cara kerja:** membuang karakter spasi (`' '`) di ujung kanan kedua string lalu membandingkan sisanya secara persis (peka huruf besar/kecil).
- **Catatan port:** hanya spasi yang dibuang, bukan tab; spasi di awal tetap signifikan, seperti di Fortran. Fungsi ini juga dipakai engine untuk mencocokkan nama tanaman dalam operasi pengelolaan (`plnt`, `hvkl`, `all`, dsb.).

### 3.4  `schema.rs`: skema tabel basis data

`schema.rs` tidak berisi fungsi. Isinya adalah 39 konstanta `&[Field]`, masing-masing salinan definisi tipe turunan Fortran (nama komponen, urutan, jenis, dan nilai bawaan) untuk satu file basis data. Urutan komponen sangat penting karena pembacaan *list-directed* mencocokkan nilai dengan komponen menurut posisi, bukan menurut header kolom file; komentar setiap konstanta menyebut tipe dan rutin pembacanya. Satuan di bawah diambil dari komentar kode Rust atau definisi tipe Fortran; bila tidak disebut, lihat bab yang memakai tabel itu.

| Konstanta | File | Tipe / rutin SWAT+ | Nilai | Komponen (urutan) dan catatan |
| --- | --- | --- | --- | --- |
| `TOPOGRAPHY` | `topography.hyd` | `topography_db` / `topo_read` | 6 | `name`, `slope` (m/m, bawaan 0,02), `slope_len` (m, 50), `lat_len` (m, 50), `dis_stream` (m, 100), `dep_co` (1) |
| `HYDROLOGY` | `hydrology.hyd` | `hydrology_db` / `hydrol_read` | 15 | `name`, `lat_ttime`, `lat_sed`, `canmx`, `esco`, `epco`, `erorgn`, `erorgp`, `cn3_swf`, `biomix`, `perco`, `lat_orgn`, `lat_orgp`, `pet_co` (1,0), `latq_co` (0,3) |
| `WETLAND` | `wetland.wet` | `wet_read` | 7 | `id` (integer), `name`, lalu nama data `init`, `hyd`, `rel`, `sed`, `nut` |
| `WET_HYD` | `hydrology.wet` | `wetland_hyd_data` | 11 | `name`, `psa`, `pdep`, `esa`, `edep`, `k` (0,01), `evrsv` (0,7), `acoef`, `bcoef`, `ccoef` (1), `frac` (0,5) |
| `RES_INIT` | `initial.res` | `res_init_dat_c` | 6 | `name`, `org_min`, `pest`, `path`, `hmet`, `salt` (semuanya nama) |
| `RES_SED` | `sediment.res` | `reservoir_sed_data` | 7 | `name`, `nsed`, `d50`, `carbon`, `bd`, `sed_stlr`, `velsetlr` |
| `RES_NUT` | `nutrients.res` | `reservoir_nut_data` | 13 | `name`, `ires1`, `ires2` (integer, awal dan akhir "musim" pengendapan nutrien pertengahan tahun), `nsetlr1`, `nsetlr2`, `psetlr1`, `psetlr2`, `nsolr`, `psolr`, `theta_n`, `theta_p`, `conc_nmin`, `conc_pmin` |
| `URBAN` | `urban.urb` | `urban_db` | 11 | `name`, `fimp`, `fcimp`, `curbden`, `urbcoef`, `dirtmx`, `thalf`, `tnconc`, `tpconc`, `tno3conc`, `urbcn2` |
| `LANDUSE` | `landuse.lum` | `land_use_management` / `landuse_read` | 14 | `name`, `cal_group`, `plant_cov`, `mgt_ops`, `cn_lu`, `cons_prac`, `urb_lu`, `urb_ro`, `ovn`, `tiledrain`, `septic`, `fstrip`, `grassww`, `bmpuser` (semuanya nama rujukan ke tabel lain) |
| `CNTABLE` | `cntable.lum` | `curvenumber_table` / `cntbl_read` | 5 | `name`, `cn[4]` (CN kelompok hidrologi A–D; bawaan Fortran 30, 55, 70, 77, ditulis 0 di skema) |
| `OVN_TABLE` | `ovn_table.lum` | `overlandflow_n_table` / `overland_n_read` | 4 | `name`, `ovn`, `ovn_min`, `ovn_max` (koefisien Manning aliran permukaan, bawaan 0,5) |
| `CONS_PRACTICE` | `cons_practice.lum` | `conservation_practice_table` / `cons_prac_read` | 3 | `name`, `pfac` (faktor P USLE, 1,0), `sl_len_mx` (1,0) |
| `SNOW` | `snow.sno` | `snow_parameters` / `snowdb_read` | 9 | `name`, `falltmp`, `melttmp`, `meltmx`, `meltmn`, `timp`, `covmx`, `cov50`, `init_mm` |
| `FIELD` | `field.fld` | `field` / `field_read` | 4 | `name`, `length` (500), `wid` (100), `ang` (30) |
| `TILLAGE` | `tillage.til` | `tillage_db` / `till_parm_read` | 6 | `tillnm`, `effmix`, `deptil`, `ranrns`, `ridge_ht`, `ridge_sp`; baris bernama `biomix` menetapkan pencampuran biologis basin (`bmix_eff`, `bmix_depth`) |
| `CARBON_BSN` | `carbon.bsn` | skalar karbon basin / `carbon_bsn_read` (bila `cswat` = 2) | 28 | `frac_seq`, `frac_hum_microb`, `frac_hum_slow`, `frac_hum_passive`, `prmt_21`, `prmt_44`, `till_eff_days`, `rtof`, `bio_consf`, `till_consf`, `tmpf`, `watf`, `tn`, `top`, `tx`, `bmix_a`, `bmix_b`, `bmix_c`, `tillmix_a`, `tillmix_b`, `tillmix_c`, `photo_degrade_factor`, `n_act_frac`, `cnr_cap`, `cnr_ref`, `cpr_cap`, `cpr_ref`, `mathers_int`; bawaan = nilai awal variabel Fortran; dibaca sebagai satu rekaman |
| `PLANTS` | `plants.plt` | `plant_db` / `plant_parm_read` | 56 | `plantnm`, `typ`, `trig`, `nfix_co`, `days_mat`, `bio_e`, `hvsti`, `blai`, `frgrw1`, `laimx1`, `frgrw2`, `laimx2`, `dlai`, `dlai_rate`, `chtmx`, `rdmx`, `t_opt`, `t_base`, `cnyld`, `cpyld`, `pltnfr1`–`3`, `pltpfr1`–3, `wsyf`, `usle_c`, `gsi`, `vpdfr`, `gmaxfr`, `wavp`, `co2hi`, `bioehi`, `rsdco_pl`, `alai_min`, `laixco_tree`, `mat_yrs`, `bmx_peren`, `ext_coef`, `leaf_tov_min`, `leaf_tov_max`, `bm_dieoff`, `rsr1`, `rsr2`, `pop1`, `frlai1`, `pop2`, `frlai2`, `frsw_gro`, `aeration`, `rsd_pctcov`, `rsd_covfac`, `meta_frac`, `str_frac`, `lig_frac`; tiga terakhir adalah `res_part_fracs` (di file bernama `avg_lig_frac`, `ab_lig_frac`, `bg_lig_frac`); deskripsi di ujung baris dilewati |
| `SOILTEST` | `nutrients.sol` | `soiltest_db` / `solt_db_read` | 12 | `name`, `exp_co`, `lab_p`, `nitrate`, `fr_hum_act`, `hum_c_n`, `hum_c_p`, `inorgp`, `watersol_p`, `h3a_p`, `mehlich_p`, `bray_strong_p` (P uji tanah, ppm) |
| `CARBON_LYR` | `carbon_lyr.bsn` | laju per lapisan / `carbon_bsn_read` | 12 | `layer` (1 = lapisan atas, 2 = lainnya), `hp_rate`, `hs_rate`, `microb_rate`, `meta_rate`, `str_rate`, `microb_top_rate`, `hs_hp`, `a1co2`, `asco2`, `apco2`, `abco2` |
| `HARV_OPS` | `harv.ops` | `harvest_operation` / `mgt_read_harvops` | 5 | `name`, `typ`, `hi_ovr`, `eff`, `bm_min` |
| `FERTILIZER` | `fertilizer.frt` | `fertilizer_db` / `fert_parm_read` | 6 | `fertnm` (16 karakter di SWAT+), `fminn`, `fminp`, `forgn`, `forgp`, `fnh3n` (fraksi) |
| `IRR_OPS` | `irr.ops` | `irrigation_operation` / `mgt_read_irrops` | 8 | `name`, `amt_mm` (mm), `eff`, `surq`, `dep_mm` (mm), `salt`, `no3`, `po4` |
| `CHEM_APP` | `chem_app.ops` | `chemical_application_operation` / `mgt_read_chemapp` | 9 | `name`, `form`, `op_typ`, `app_eff`, `foliar_eff`, `inject_dep`, `surf_frac`, `drift_pot`, `aerial_unif` |
| `HYD_SED_LTE` | `hyd-sed-lte.cha` | `swatdeg_hydsed_data` / `sd_hydsed_read` | 23 | `name`, `order`, `chw`, `chd`, `chs`, `chl`, `chn`, `chk`, `bank_exp`, `cov`, `sinu`, `vcr_coef`, `d50`, `ch_clay`, `carbon`, `ch_bd`, `chss`, `bankfull_flo`, `fps`, `fpn`, `n_conc`, `p_conc`, `p_bio`; kolom file `erod_fact`, `cov_fact`, `eq_slp` = komponen `bank_exp`, `cov`, `vcr_coef`; deskripsi akhir baris dilewati |
| `CH_NUT` | `nutrients.cha` | `channel_nut_data` / `ch_read_nut` | 39 | `name`, `onco`, `opco`, `rs1`–`rs7`, `rk1`–`rk6`, `bc1`–`bc4`, `lao`, `igropt`, `ai0`–`ai6`, `mumax`, `rhoq`, `tfact`, `k_l`, `k_n`, `k_p`, `lambda0`–`lambda2`, `p_n` (laju dan koefisien QUAL2E) |
| `INIT_CHA` | `initial.cha` dan file `initial.*` lain | `ch_init` | 6 | `name`, `org_min`, `pest`, `path`, `hmet`, `salt` |
| `OM_WATER` | `om_water.ini` | `om_init_water` (sebuah `hyd_output` bernama) | 19 | `name`, lalu 18 komponen hidrograf `flo` … `temp` (lihat `RecallRow`) |
| `WEIR_RES` | `weir.res` | `reservoir_weir_outflow` | 5 | `name`, `c` (1,84), `k` (2,6), `w` (lebar, m, 2,5), `h` (tinggi, m) |
| `PUDDLE_OPS` | `puddle.ops` | `puddle_operation` | 9 | `name`, `wet_hc` (konduktivitas lapisan atas, mm/h), `sed`, `orgn`, `sedp`, `no3`, `solp`, `nh3`, `no2` (konsentrasi air setelah pelumpuran, ppm) |
| `MANURE_DB` | `manure_db.frt` | `manure_database` | 7 | `name`, `org_min`, `pests`, `paths`, `hmets`, `salts`, `constit` |
| `MANURE_OM` | `manure_om.frt` | `manure_attributes` | 8 | `name`, `frac_water`, `fcbn`, `fminn`, `fminp`, `forgn`, `forgp`, `fnh3n` |
| `TRANSPLANT` | `transplant.ops` | `transplant_operation` | 1 | hanya `name`; operasi transplantasi belum di-port |
| `RES_DAT` | `reservoir.res` | `reservoir_data_char_input` | 7 | `id`, `name`, `init`, `hyd`, `rel`, `sed`, `nut` |
| `RES_HYD` | `hydrology.res` | `reservoir_hyd_data` | 11 | `name`, `iyres`, `mores` (tahun dan bulan operasi), `psa` (ha), `pvol` (ha-m), `esa` (ha), `evol` (ha-m) pada pelimpah utama dan darurat, `k`, `evrsv`, `br1`, `br2` |
| `SEPTIC_SEP` | `septic.sep` | `septic_db` | 11 | `name`, `q_rate`, `bod`, `tss`, `nh4_n`, `no3_n`, `no2_n`, `org_n`, `min_p`, `org_p`, `fcoli` (efluen jenis tangki septik) |
| `FSTRIP` | `filterstrip.str` | `filtstrip_operation` | 5 | `name`, `flag`, `fld_vfs`, `con_vfs`, `cha_q` |
| `GRASSWW` | `grassedww.str` | `grwaterway_operation` | 8 | `name`, `flag`, `mann`, `sed_co`, `dp`, `wd`, `len`, `slp` |
| `BMPUSER` | `bmpuser.str` | `bmpuser_operation` | 8 | `name`, `flag`, `sed_eff`, `ptlp_eff`, `solp_eff`, `ptln_eff`, `soln_eff`, `bact_eff` (bagian yang disisihkan) |
| `TILEDRAIN` | `tiledrain.str` | `subsurface_drainage_parameters` | 9 | `name`, `depth`, `time`, `lag`, `radius`, `dist`, `drain_co`, `pumpcap`, `latksat` |

**Catatan port.** (1) Di kode sumber, komentar dokumen `landuse.lum` tertulis tepat di atas `WETLAND` (bersama komentar `wetland.wet`), sementara `LANDUSE` sendiri tidak berkomentar; ini hanya kosmetik dan tidak memengaruhi perilaku. (2) Untuk `plants.plt` dan `hyd-sed-lte.cha`, file memuat kolom deskripsi teks di ujung baris yang tidak menjadi bagian tipe; pembacaan seluruh tipe berhenti sebelum kolom itu, sehingga kolom itu terlewati seperti di SWAT+. (3) Skema yang dimulai dengan integer (`WETLAND`, `RES_DAT`) mencerminkan file yang menulis nomor baris sebelum nama.

### 3.5  `cio.rs`: file induk `file.cio`

`file.cio` adalah file induk proyek SWAT+: setiap barisnya memberi nama file input untuk sekelompok "slot". SWAT+ membacanya di `readcio_read` dengan satu `read (107,*) name, in_xxx` per baris, berurutan, sehingga baris dicocokkan menurut **posisi**; nama yang tertulis di awal baris diabaikan. Nama bawaan setiap slot berasal dari inisialisasi tipe di `input_file_module`; slot yang tidak pernah terisi (baris hilang di akhir file) mempertahankan nama bawaannya.

**Konstanta `SCHEMA: &[(&str, &[(&str, &str)])]`.** Daftar baris `file.cio` dalam urutan pembacaan SWAT+. Setiap entri adalah (kunci baris, daftar (slot, nama file bawaan)). Ringkasannya:

| No. | Kunci baris | Slot (bawaan) |
| --- | --- | --- |
| 1 | `simulation` | `time` (`time.sim`), `prt` (`print.prt`), `object_prt` (`object.prt`), `object_cnt` (`object.cnt`), `cs_db` (`constituents.cs`) |
| 2 | `basin` | `codes_bas` (`codes.bsn`), `parms_bas` (`parameters.bsn`), `carbon_bsn` (`carbon.bsn`) |
| 3 | `climate` | `weat_sta`, `weat_wgn`, `pet_cli`, `pcp_cli`, `tmp_cli`, `slr_cli`, `hmd_cli`, `wnd_cli`, `atmo_cli` (`weather-sta.cli` … `atmodep.cli`) |
| 4 | `connect` | `hru_con`, `hruez_con`, `ru_con`, `gwflow_con`, `aqu_con`, `aqu2d_con`, `chan_con`, `res_con`, `rec_con`, `exco_con`, `delr_con`, `out_con`, `chandeg_con` (file `*.con`) |
| 5 | `channel` | `init`, `dat`, `hyd`, `sed`, `nut`, `chan_ez`, `hyd_sed`, `temp` |
| 6 | `reservoir` | `init_res`, `res`, `hyd_res`, `sed_res`, `nut_res`, `weir_res`, `wet`, `hyd_wet` |
| 7 | `routing_unit` | `ru_def`, `ru_ele`, `ru`, `ru_dr` |
| 8 | `hru` | `hru_data` (`hru-data.hru`), `hru_ez` (`hru-lte.hru`) |
| 9 | `exco` | `exco`, `om`, `pest`, `path`, `hmet`, `salt` |
| 10 | `recall` | `recall_rec` |
| 11 | `dr` | `del_ratio`, `om`, `pest`, `path`, `hmet`, `salt` |
| 12 | `aquifer` | `init` (`initial.aqu`), `aqu` (`aquifer.aqu`) |
| 13 | `herd` | `animal`, `herd`, `ranch` |
| 14 | `water_rights` | `pou`, `pod` |
| 15 | `link` | `chan_surf`, `aqu_cha` |
| 16 | `hydrology` | `hydrol_hyd`, `topogr_hyd`, `field_fld` |
| 17 | `structural` | `tiledrain_str`, `septic_str`, `fstrip_str`, `grassww_str`, `bmpuser_str` |
| 18 | `hru_parm_db` | `plants_plt`, `fert_frt`, `till_til`, `pest`, `pathcom_db`, `hmetcom_db`, `saltcom_db`, `urban_urb`, `septic_sep`, `snow` |
| 19 | `ops` | `harv_ops`, `graze_ops`, `irr_ops`, `chem_ops`, `fire_ops`, `sweep_ops` |
| 20 | `lum` | `landuse_lum`, `management_sch`, `cntable_lum`, `cons_prac_lum`, `ovn_lum` |
| 21 | `chg` | `cal_parms`, `cal_upd`, `codes_sft`, `wb_parms_sft`, `water_balance_sft`, `ch_sed_budget_sft`, `ch_sed_parms_sft`, `plant_parms_sft`, `plant_gro_sft` |
| 22 | `init` | `plant`, `soil_plant_ini`, `om_water`, `pest_soil`, `pest_water`, `path_soil`, `path_water`, `hmet_soil`, `hmet_water`, `salt_soil`, `salt_water` |
| 23 | `soils` | `soils_sol`, `nut_sol`, `lte_sol` |
| 24 | `decision_table` | `dtbl_lum` (`lum.dtl`), `dtbl_res` (`res_rel.dtl`), `dtbl_scen` (`scen_lu.dtl`), `dtbl_flo` (`flo_con.dtl`) |
| 25 | `regions` | 17 slot `ele_*`/`def_*` untuk unit lanskap, region, dan unit tangkapan kanal, akuifer, waduk, dan recall |
| 26–30 | `pcp_path`, `tmp_path`, `slr_path`, `hmd_path`, `wnd_path` | satu slot masing-masing (`pcp`, `tmp`, `slr`, `hmd`, `wnd`), bawaan kosong = folder skenario |

**Struct `CioSection`** (satu baris): `key: String` (kunci baris di `SCHEMA`), `name_in_file: Option<String>` (nama yang tertulis di awal baris, bila baris ada; hanya informatif), `files: Vec<(String, Option<String>)>` (setiap slot dan nama filenya; `None` bila file menulis `null`, artinya slot dimatikan).

**Struct `FileCio`**: `title: String` (baris judul), `sections: Vec<CioSection>`, dan `out_path: Option<String>` (folder keluaran dari baris setelah baris-baris folder cuaca, bila ada).

#### 3.5.1  `FileCio::defaults`

- **Lokasi:** `crates/swat-model/src/cio.rs`, `pub fn defaults() -> Self`
- **Padanan SWAT+:** nilai awal variabel `in_sim`, `in_basin`, … di `input_file_module`, sebelum `readcio_read` dijalankan.
- **Tujuan:** Membuat `FileCio` yang setiap slotnya berisi nama file bawaan, sebagai titik awal pembaca.
- **Cara kerja:**
  1. Untuk setiap entri `SCHEMA`, buat `CioSection` dengan `key` dari skema, `name_in_file = None`, dan `files` = setiap (slot, `Some(nama bawaan)`).
  2. `title` kosong dan `out_path = None`.
- **Kebiasaan SWAT+ / catatan port:** pembaca `swat-io` lalu menimpa slot menurut posisi. Baris yang memuat lebih banyak nama daripada slot mengabaikan sisanya (Ames\_sub1 menulis 12 nama pada baris `init` yang punya 11 slot); baris yang hilang di akhir file mempertahankan bawaan; nilai kosong (`,,`) atau yang terpotong oleh `/` membiarkan bawaan (`docs/swat-quirks.md`).

#### 3.5.2  `FileCio::file`

- **Lokasi:** `crates/swat-model/src/cio.rs`, `pub fn file(&self, section: &str, slot: &str) -> Option<&str>`
- **Padanan SWAT+:** akses `in_xxx%slot` disertai uji `/= "null"` dan pemeriksaan `inquire (file=..., exist=...)` yang biasa mendahului pembacaan file.
- **Tujuan:** Nama file untuk sebuah slot, atau `None` bila slot tidak boleh dibaca.
- **Masukan dan keluaran:** kunci baris dan nama slot; keluaran nama file.
- **Cara kerja:**
  1. Cari `CioSection` dengan `key == section`; tidak ada → `None`.
  2. Cari slot dengan nama `slot`; tidak ada → `None`.
  3. Kembalikan nama file bila `Some` dan tidak kosong setelah `trim`; `None` untuk `null` atau kosong.
- **Catatan port:** nama slot tidak unik antar-baris (mis. `pest`, `om`, `init` muncul di beberapa baris), sehingga baris harus selalu disebut. Untuk slot folder cuaca (`pcp_path` dan sejenisnya) nilai kosong berarti folder skenario; fungsi ini mengembalikan `None` untuk kasus itu dan pemanggil menafsirkannya sebagai "folder kerja".

### 3.6  `basin.rs`: kode kendali dan parameter basin

Modul ini memuat dua file tingkat basin: `codes.bsn` (kode kendali, dibaca `basin_read_cc` ke tipe `basin_control_codes`) dan `parameters.bsn` (parameter real, dibaca `basin_read_prm` ke tipe `basin_parms`). Keduanya dibaca SWAT+ dengan satu pembacaan *list-directed* per file, sehingga nilai dicocokkan ke field menurut posisi, apa pun isi header file.

**Struct `BasinCodes`** (`codes.bsn`, urutan field = urutan `basin_control_codes`). Makna diambil dari komentar tipe Fortran di `basin_module.f90`.

| Field | Bawaan | Arti |
| --- | --- | --- |
| `petfile: String` | `"         pet.cli"` (diawali 9 spasi, seperti di Fortran) | nama file PET |
| `wwqfile: String` | kosong | file kualitas air aliran basin |
| `pet: i32` | 0 | metode PET: 0 Priestley-Taylor, 1 Penman-Monteith, 2 Hargreaves, 3 dibaca dari file |
| `nam1` | 0 | tidak dipakai |
| `crk` | 0 | 1 = hitung aliran retakan tanah |
| `swift_out` | 0 | 1 = tulis berkas masukan SWIFT |
| `sed_det` | 0 | metode laju puncak: 0 hidrograf tak berdimensi NRCS dengan PRF, 1 intensitas hujan setengah jam |
| `rte` | 0 | routing air: 0 *variable storage*, 1 Muskingum |
| `deg`, `wq`, `cn`, `cfac`, `sed_ch`, `smax` | 0 | tidak dipakai menurut komentar Fortran |
| `nostress` | 0 | 0 semua stres tanaman, 1 matikan semua stres, 2 matikan stres nutrien saja |
| `cswat` | 0 | model karbon: 0 statis, 1 C-FARM (dicadangkan), 2 CENTURY/SWAT-C dinamis |
| `lapse` | 0 | 1 = koreksi presipitasi dan suhu terhadap elevasi (laju *lapse*) |
| `uhyd` | 1 | hidrograf satuan: 0 segitiga, 1 fungsi gamma |
| `tdrn` | 0 | drainase tile: 0 persamaan *drawdown days*, 1 DRAINMOD |
| `wtdn` | 0 | muka air tanah dangkal: 0 rutin asli, 1 DRAINMOD |
| `sol_p_model` | 0 | 0 model P tanah asli, 1 Vadas dan White (2010) |
| `gampt` | 0 | 0 bilangan kurva, 1 Green-Ampt |
| `atmo: String` | `"a"` | tidak dipakai menurut komentar Fortran |
| `qual2e` | 0 | 0 QUAL2E, 1 QUAL2E dengan transformasi disederhanakan |
| `gwflow` | 0 | 1 = modul gwflow aktif |
| `idc_till` | 3 | metode pengolahan tanah bila `cswat` = 2: 1 DSSAT, 2 EPIC, 3 Kemanian, 4 DNDC |

**Konstanta `BASIN_PARAM_NAMES: [&str; 42]` dan `BASIN_PARAM_DEFAULTS: [f32; 42]`.** Nama dan bawaan ke-42 field real `basin_parms` dalam urutan Fortran. Satuan dari komentar Fortran bila ada:

| Indeks | Nama | Bawaan | Arti (satuan) |
| --- | --- | --- | --- |
| 0 | `evlai` | 3,0 | LAI saat evaporasi tanah berhenti |
| 1 | `ffcb` | 0 | kadar air tanah awal sebagai fraksi kapasitas lapang |
| 2 | `surlag` | 4,0 | koefisien jeda limpasan permukaan (hari) |
| 3 | `adj_pkr` | 1,0 | faktor penyesuaian laju puncak |
| 4 | `prf` | 484 | faktor laju puncak (persamaan laju puncak) |
| 5 | `pestgwfact` | 0 | faktor pencampuran pestisida air tanah |
| 6 | `temp_decay` | 0 | faktor peluruhan pestisida terhadap suhu |
| 7 | `cmn` | 0,003 | laju mineralisasi N organik aktif |
| 8, 9 | `n_updis`, `p_updis` | 20, 20 | parameter distribusi serapan N dan P |
| 10 | `nperco` | 0,10 | koefisien perkolasi nitrat |
| 11 | `pperco` | 10,0 | koefisien perkolasi P |
| 12 | `phoskd` | 175 | koefisien partisi P tanah |
| 13 | `psp` | 0,40 | indeks ketersediaan P |
| 14 | `rsdco` | 0,05 | koefisien dekomposisi residu |
| 15 | `percop` | 0,5 | koefisien perkolasi pestisida |
| 16, 17, 18 | `msk_co1`, `msk_co2`, `msk_x` | 0,75; 0,25; 0,20 | koefisien Muskingum |
| 19 | `nperco_lchtile` | 0,5 | koefisien konsentrasi N untuk aliran tile dan pelindian |
| 20 | `evrch` | 0,60 | faktor penyesuaian evaporasi sungai |
| 21 | `scoef` | 1,0 | koefisien tampungan kanal |
| 22 | `cdn` | 1,40 | koefisien laju eksponensial denitrifikasi |
| 23 | `sdnco` | 1,30 | ambang denitrifikasi (fraksi kapasitas lapang) |
| 24 | `bact_swf` | 0,15 | fraksi pupuk kandang berisi koloni aktif |
| 25 | `tb_adj` | 0 | penyesuaian waktu dasar hidrograf satuan subharian |
| 26 | `cn_froz` | 0,000862 | parameter tanah beku untuk infiltrasi/limpasan |
| 27 | `dorm_hr` | −1 | ambang waktu dormansi (jam) |
| 28 | `plaps` | 0 | laju *lapse* presipitasi (mm/km) |
| 29 | `tlaps` | 6,5 | laju *lapse* suhu (°C/km) |
| 30 | `nfixmx` | 20 | fiksasi N harian maksimum (kg/ha) |
| 31 | `decr_min` | 0,01 | peluruhan residu harian minimum |
| 32 | `rsd_covco` | 0,75 | faktor tutupan residu |
| 33 | `urb_init_abst` | 1,0 | abstraksi awal maksimum area urban (Green-Ampt) |
| 34 | `petco_pmpt` | 100 | penyesuaian PET (%) untuk Penman-Monteith dan Priestley-Taylor |
| 35 | `uhalpha` | 1,0 | koefisien alfa hidrograf satuan gamma |
| 36–39 | `eros_spl`, `rill_mult`, `eros_expo`, `c_factor` | 0 | koefisien erosi percik, alur, eksponen aliran permukaan, dan skala faktor C |
| 40 | `ch_d50` | 0 | diameter median partikel kanal utama (mm) |
| 41 | `co2` | 400 | konsentrasi CO2 awal simulasi (ppm) |

**Struct `BasinParams`** (`parameters.bsn`): `reals: [f32; 42]` (urutan `BASIN_PARAM_NAMES`), `day_lag_mx: i32` (jumlah hari maksimum penundaan hidrograf), dan `igen: i32` (kode pembangkit bilangan acak: 0 mempertahankan benih tetap SWAT+; nilai lain menggeser benih).

#### 3.6.1  `BasinCodes::default`

- **Lokasi:** `crates/swat-model/src/basin.rs`, `impl Default for BasinCodes`
- **Padanan SWAT+:** inisialisasi komponen `basin_control_codes` di `basin_module.f90`.
- **Tujuan:** Nilai bawaan `codes.bsn` sebelum dibaca (tabel di atas). Pembaca mulai dari nilai ini, sehingga field yang tidak terbaca mempertahankan bawaan Fortran.
- **Catatan port:** `petfile` meniru persis string bawaan Fortran yang diawali sembilan spasi; `uhyd` = 1 (gamma) dan `idc_till` = 3 adalah satu-satunya kode integer yang bawaannya bukan 0.

#### 3.6.2  `BasinParams::default`

- **Lokasi:** `crates/swat-model/src/basin.rs`, `impl Default for BasinParams`
- **Padanan SWAT+:** inisialisasi komponen `basin_parms`.
- **Tujuan:** `reals = BASIN_PARAM_DEFAULTS`, `day_lag_mx = 0`, `igen = 5`.
- **Kebiasaan SWAT+:** bawaan `igen` = 5 penting. SWAT+ hanya memeriksa akhir file (`iostat < 0`) saat membaca; nilai yang gagal dikonversi menghentikan pembacaan secara diam-diam. `parameters.bsn` Osu\_1hru yang ditulis SWAT+ Editor menulis `day_lag_mx` sebagai `0.00000`, yang tidak dapat dibaca gfortran sebagai integer, sehingga `igen` tetap 5 dan benih acak bergeser. File Ames\_sub1 menulis `0 0`, sehingga `igen` = 0 (`docs/swat-quirks.md`).

#### 3.6.3  `BasinParams::get`

- **Lokasi:** `crates/swat-model/src/basin.rs`, `pub fn get(&self, name: &str) -> Option<f32>`
- **Padanan SWAT+:** akses `bsn_prm%<nama>`.
- **Tujuan:** Mengambil parameter real menurut namanya.
- **Cara kerja:** cari posisi `name` di `BASIN_PARAM_NAMES`, lalu kembalikan `reals[posisi]`; nama tidak dikenal → `None`.
- **Catatan port:** engine memanggilnya dengan nilai cadangan yang sama dengan bawaan SWAT+, misalnya `get("surlag").unwrap_or(4.0)`.

#### 3.6.4  `BasinParams::complete`

- **Lokasi:** `crates/swat-model/src/basin.rs`, `pub fn complete(&mut self)`
- **Padanan SWAT+:** subrutin `basin_prm_default` (`basin_prm_default.f90`), dipanggil setelah `basin_read_prm`. Di port dipanggil oleh `swat-io/src/project.rs` setelah membaca `parameters.bsn`.
- **Tujuan:** Mengganti parameter yang nol atau tidak diisi dengan nilai bawaan operasional.
- **Masukan dan keluaran:** `self.reals` diubah di tempat.
- **Cara kerja:**
  1. Untuk 34 parameter dalam daftar `FLOORED`, bila nilai < 10⁻⁶ maka diganti dengan nilai pengganti:

```latex
p \leftarrow \begin{cases} p_{def} & p < 10^{-6} \\ p & \text{lainnya} \end{cases}
```

Pengganti umumnya sama dengan bawaan tipe, kecuali tiga yang berbeda: `prf` = 1 (bawaan tipe 484), `cmn` = 0,0003 (bawaan tipe 0,003), dan `urb_init_abst` = 0 (bawaan tipe 1). Parameter yang tidak termasuk daftar: `pestgwfact`, `temp_decay`, `scoef`, `dorm_hr`, `plaps`, `tlaps`, `petco_pmpt`, `co2` (dua terakhir punya aturan sendiri). Untuk `dorm_hr`, baris yang bersangkutan dikomentari di kode Fortran.

2. `petco_pmpt`: bila 0 < nilai < 0,5, nilai menjadi 0.
3. `co2`: bila < 100, nilai menjadi 400 ppm.

- **Kebiasaan SWAT+ / catatan port:**
  - Nilai negatif juga lolos uji `< 10⁻⁶` dan diganti; perilaku ini sama dengan Fortran.
  - `basin_prm_default` juga mengubah `petco_pmpt` menjadi fraksi, `(100 + petco_pmpt)/100`, menghitung faktor normalisasi serapan (`uptake%water_norm`, `n_norm`, `p_norm`), dan menetapkan `day_lag_mx = 2`. Langkah-langkah itu tidak dilakukan di `complete`: faktor serapan dihitung di modul tanaman engine, penyimpanan hidrograf dua hari ditanam langsung di `swat-engine/src/route.rs` (HRU, unit routing, kanal, dan outlet menyimpan 2 hari, objek lain 1), dan pencarian di kode engine tidak menemukan pemakaian `petco_pmpt`.

#### 3.6.5  Akses langsung `BasinParams`

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `adj_pkr(&self) -> f32` | `basin.rs` | `reals[3]`, faktor penyesuaian laju puncak |
| `dorm_hr(&self) -> f32` | `basin.rs` | `reals[27]`, ambang dormansi (jam) |
| `plaps(&self) -> f32` | `basin.rs` | `reals[28]`, laju *lapse* presipitasi (mm/km) |
| `tlaps(&self) -> f32` | `basin.rs` | `reals[29]`, laju *lapse* suhu (°C/km) |

Keempatnya mengindeks larik secara langsung (indeks konstan dalam batas larik berukuran tetap 42, sehingga tidak dapat *panic*) dan dipakai di jalur panas modul iklim dan tanaman.

### 3.7  `climate.rs`: stasiun cuaca, pembangkit cuaca, dan deret terukur

Modul ini menampung masukan iklim: definisi stasiun (`weather-sta.cli`), parameter pembangkit cuaca per stasiun (`weather-wgn.cli`), daftar file stasiun per variabel (`pcp.cli`, `tmp.cli`, `slr.cli`, `hmd.cli`, `wnd.cli`, `pet.cli`), dan isi file stasiun terukur (`*.pcp`, `*.tmp`, `*.slr`, `*.hmd`, `*.wnd`). Pemakaian data ini (pembangkitan cuaca, penempatan rekaman ke larik harian, koreksi elevasi) dibahas di bab iklim engine.

**Struct `WeatherStation`** (`cli_staread`): `name: String` lalu tujuh `Option<String>`: `wgn` (nama pembangkit cuaca), `pcp`, `tmp`, `slr`, `hmd`, `wnd` (nama file terukur, atau `sim` yang berarti variabel dibangkitkan), `pet` (kolom kedelapan; header file menyebutnya `wnd_dir`, tetapi SWAT+ membacanya sebagai `petgage`), dan `atmo_dep` (stasiun deposisi atmosfer). `None` berarti file menulis `null`.

**Struct `Wgn`** (`cli_wgnread`, tipe `weather_generator_db`): `name`, `lat` dan `lon` (derajat), `elev` (m), `rain_yrs` (jumlah tahun rekaman di balik statistik hujan maksimum setengah jam; bawaan tipe Fortran 10), dan `months: [WgnMonth; 12]`.

**Struct `WgnMonth`** (satu bulan parameter pembangkit, urutan file; satuan dari `climate_module.f90`):

| Field Rust | Fortran | Arti (satuan) |
| --- | --- | --- |
| `tmp_max_ave` | `tmpmx` | rata-rata suhu maksimum bulanan (°C) |
| `tmp_min_ave` | `tmpmn` | rata-rata suhu minimum bulanan (°C) |
| `tmp_max_sd` | `tmpstdmx` | simpangan baku suhu maksimum (°C) |
| `tmp_min_sd` | `tmpstdmn` | simpangan baku suhu minimum (°C) |
| `pcp_ave` | `pcpmm` | jumlah presipitasi bulan (mm) |
| `pcp_sd` | `pcpstd` | simpangan baku presipitasi harian (mm/hari) |
| `pcp_skew` | `pcpskw` | koefisien kemencengan presipitasi harian |
| `wet_dry` | `pr_wd` | peluang hari basah setelah hari kering |
| `wet_wet` | `pr_ww` | peluang hari basah setelah hari basah |
| `pcp_days` | `pcpd` | rata-rata jumlah hari hujan dalam bulan (hari) |
| `pcp_hhr` | `rainhmx` | hujan maksimum setengah jam dalam bulan (mm) |
| `slr_ave` | `solarav` | radiasi matahari harian rata-rata (MJ/m²/hari) |
| `dew_ave` | `dewpt` | suhu titik embun rata-rata (°C) |
| `wnd_ave` | `windav` | kecepatan angin rata-rata (m/s) |

**Enum `ClimateVar`** (variabel terukur, satu file daftar stasiun masing-masing): `Pcp` (presipitasi, mm), `Tmp` (suhu maksimum dan minimum, °C), `Slr` (radiasi matahari, MJ/m²), `Hmd` (kelembapan relatif), `Wnd` (kecepatan angin, m/s), `Pet` (PET terukur, mm). Konstanta `ClimateVar::ALL` memuat keenamnya dalam urutan ini.

**Struct `StationList`**: `var: ClimateVar` dan `files: Vec<String>` (nama file stasiun apa adanya, dalam urutan file). Urutan dipertahankan karena SWAT+ mencari file stasiun di daftar ini dengan pencarian biner (`search`), yang hanya andal pada daftar terurut.

**Struct `StationSeries`** (satu file stasiun terukur, apa adanya): `var`, `file`, `nbyr: i32` (jumlah tahun menurut header), `tstep: i32` (langkah per hari; 0 = harian), `lat`, `lon` (derajat), `elev` (m), dan `records: Vec<ClimateRecord>`. Rekaman disimpan sebagaimana dibaca; SWAT+ menempatkannya ke larik harian saat simulasi dimulai, dan engine mereproduksi langkah itu.

**Struct `ClimateRecord`**: `year`, `jday` (`i32`), `hour: Option<i32>` (jam, hanya untuk presipitasi subharian), dan `values: [f32; 2]` (nilai variabel; elemen kedua hanya untuk suhu, yaitu suhu minimum). Nilai hilang ditandai −99 seperti di SWAT+.

#### 3.7.1  `ClimateVar::cio_slot`

- **Lokasi:** `crates/swat-model/src/climate.rs`, `pub fn cio_slot(self) -> &'static str`
- **Padanan SWAT+:** komponen `in_cli%pcp_cli`, `tmp_cli`, dan seterusnya di `input_file_module`.
- **Tujuan:** Nama slot daftar stasiun variabel ini di baris `climate` `file.cio`.
- **Cara kerja:** pemetaan tetap `Pcp` → `pcp_cli`, `Tmp` → `tmp_cli`, `Slr` → `slr_cli`, `Hmd` → `hmd_cli`, `Wnd` → `wnd_cli`, `Pet` → `pet_cli`.

#### 3.7.2  `ClimateVar::path_section`

- **Lokasi:** `crates/swat-model/src/climate.rs`, `pub fn path_section(self) -> Option<(&'static str, &'static str)>`
- **Padanan SWAT+:** baris folder data cuaca di akhir `file.cio` (`in_path_pcp`, `in_path_tmp`, dsb.).
- **Tujuan:** (baris, slot) `file.cio` yang memuat folder data variabel ini.
- **Cara kerja:** `Pcp` → (`pcp_path`, `pcp`), `Tmp` → (`tmp_path`, `tmp`), `Slr` → (`slr_path`, `slr`), `Hmd` → (`hmd_path`, `hmd`), `Wnd` → (`wnd_path`, `wnd`); `Pet` → `None` (tidak punya baris folder).

#### 3.7.3  `ClimateVar::values`

- **Lokasi:** `crates/swat-model/src/climate.rs`, `pub fn values(self) -> usize`
- **Padanan SWAT+:** bentuk pembacaan rekaman di `cli_tmeas` (dua nilai) dibanding `cli_pmeas`, `cli_smeas`, `cli_hmeas`, `cli_wmeas` (satu nilai).
- **Tujuan:** Jumlah nilai per rekaman harian: 2 untuk `Tmp` (maksimum, minimum), 1 untuk yang lain.

### 3.8  `hru.rs`: HRU, tanah, dan kondisi awal

Modul ini memuat baris `hru-data.hru`, profil tanah `soils.sol`, kondisi awal tanah-tanaman `soil_plant.ini`, dan komunitas tanaman awal `plant.ini`. Struct-struct ini terutama berisi *nama* yang menunjuk ke tabel lain; penggabungan dan penurunan sifat HRU dilakukan di engine (bab HRU).

**Struct `HruData`** (`hru_read`): `id: i32` (nomor HRU), `name`, dan nama-nama rujukan: `topo` (`topography.hyd`), `hyd` (`hydrology.hyd`), `soil` (`soils.sol`), `land_use_mgt` (`landuse.lum`), `soil_plant_init` (`soil_plant.ini`), `surf_stor` (tampungan permukaan: lahan basah), `snow` (`snow.sno`), dan `field` (`field.fld`). `Project.hru_data` diindeks dengan `id − 1`.

**Struct `Soil`** (`soil_db_read`, tipe `soil_profile_db` + larik lapisan):

| Field | Tipe | Arti (satuan) | Bawaan tipe Fortran |
| --- | --- | --- | --- |
| `name` | `String` | nama seri tanah | — |
| `nly` | `i32` | jumlah lapisan | 1 |
| `hydgrp` | `String` | kelompok hidrologi tanah A–D | `A` |
| `zmx` | `f32` | kedalaman perakaran maksimum (mm) | 1500 |
| `anion_excl` | `f32` | fraksi porositas tempat anion tereksklusi | 0,5 |
| `crk` | `f32` | potensi volume retakan tanah | 0,01 |
| `texture` | `String` | tekstur tanah | — |
| `layers` | `Vec<SoilLayer>` | lapisan dalam urutan file | — |

**Struct `SoilLayer`** (`soildb%ly`, tipe `soilayer_db`):

| Field | Arti (satuan) | Bawaan tipe Fortran |
| --- | --- | --- |
| `z` | kedalaman dari permukaan ke dasar lapisan (mm) | 1500 |
| `bd` | berat isi (Mg/m³ = g/cm³) | 1,3 |
| `awc` | kapasitas air tersedia (mm air/mm tanah) | 0,2 |
| `k` | konduktivitas hidraulik jenuh (mm/jam) | 10 |
| `cbn` | karbon organik (%) | 2,0 |
| `clay`, `silt`, `sand` | liat, debu, pasir (%) | 10, 60, 30 |
| `rock` | fragmen batuan (%) | 0 |
| `alb` | albedo tanah lembap | 0,1 |
| `usle_k` | faktor erodibilitas USLE K | 0,2 |
| `ec` | daya hantar listrik (dS/m) | 0 |
| `cal` | CaCO3 (%) | 0 |
| `ph` | pH tanah | 0 |

Catatan: `Soil` dan `SoilLayer` menurunkan `Default` Rust (semua nol/kosong), bukan bawaan tipe Fortran di atas. Pembaca mengisi semua field dari file; bawaan Fortran hanya relevan bila baris file terpotong, dan penanganannya ada di pembaca `swat-io`.

**Struct `SoilPlantInit`** (`soil_plant_init`): `name`, `sw_frac: f32` (air tanah awal sebagai fraksi kapasitas lapang), dan nama-nama rujukan inisialisasi: `nutc` (nutrien, `nutrients.sol`), `pestc` (pestisida), `pathc` (patogen), `saltc` (garam), `hmetc` (logam berat), `csc` (konstituen).

**Struct `PlantInit`** (satu tanaman dalam komunitas `plant.ini`, tipe `plant_init_db`):

| Field | Arti (satuan) | Bawaan |
| --- | --- | --- |
| `cpnm` | nama tanaman di `plants.plt` | `frsd` |
| `igro` | status tumbuh di awal: `y` tumbuh (`lc_status`) | `y` |
| `lai` | indeks luas daun (m²/m²) | 0 |
| `bioms` | biomassa tanaman (kg/ha) | 0 |
| `phuacc` | fraksi unit panas yang sudah terkumpul | 0 |
| `pop` | populasi tanaman | 0 |
| `fr_yrmat` | fraksi tahun menuju kematangan yang telah dicapai (`yrs_init`) | 1 |
| `rsdin` | residu permukaan awal (kg/ha) | 10000 |

**Struct `PlantCommunity`** (`plant_community_db`, dibaca `readpcom`): `name`, `rot_yr_ini: i32` (tahun rotasi awal), `plants: Vec<PlantInit>`.

#### 3.8.1  `PlantInit::default`

- **Lokasi:** `crates/swat-model/src/hru.rs`, `impl Default for PlantInit`
- **Padanan SWAT+:** inisialisasi komponen `plant_init_db` di `plant_data_module.f90`.
- **Tujuan:** Nilai bawaan satu tanaman komunitas sebelum dibaca: `cpnm = "frsd"`, `igro = "y"`, `lai = bioms = phuacc = pop = 0`, `fr_yrmat = 1`, `rsdin = 10000` kg/ha.
- **Catatan port:** berbeda dari `Soil`, struct ini sengaja memakai bawaan Fortran karena baris `plant.ini` boleh lebih pendek dari tipe (komponen sisanya mempertahankan bawaan, termasuk residu awal 10.000 kg/ha). Field Fortran `db_num` (indeks ke `plants.plt`) tidak disimpan di sini; engine mencarinya menurut nama.

### 3.9  `cal.rs`: kalibrasi keras

Modul ini memuat dua file kalibrasi SWAT+: `cal_parms.cal` (daftar parameter yang boleh diubah beserta rentang absolutnya, dibaca `cal_parm_read`) dan `calibration.cal` (daftar perubahan, dibaca `cal_parmchg_read`). Perubahan diterapkan satu kali setelah objek disiapkan (`cal_conditions`, dipanggil dari `proc_cal`); penerapannya dibahas di bab HRU. Modul ini juga berisi fungsi `define_unit_elements` yang dipakai bersama oleh pembaca kalibrasi dan pembaca unit routing.

**Struct `CalParm`** (tipe `calibration_parameters`): `name` (nama parameter, mis. `cn2`, `esco`, `awc`), `ob_typ` (jenis objek pemilik parameter: `hru`, `chan`, `res`, `bsn`, `plt`, …), `absmin` dan `absmax` (`f32`, batas absolut nilai hasil perubahan). Field Fortran `units` tidak disimpan.

**Struct `CalUpdate`** (tipe `update_parameters`, satu baris `calibration.cal`):

| Field | Tipe | Arti |
| --- | --- | --- |
| `name` | `String` | parameter yang diubah |
| `chg_typ` | `String` | jenis perubahan: `absval`, `abschg`, `pctchg`, `relchg` |
| `val` | `f32` | besar perubahan |
| `conds` | `Vec<CalCond>` | kondisi yang membatasi objek yang diubah |
| `val1`, `val2` | `f32` | batas bawah dan atas kondisi `range` (yang terakhir dibaca) |
| `lyr1`, `lyr2` | `i32` | lapisan tanah pertama dan terakhir (0 = semua / sampai lapisan terakhir) |
| `year1`, `year2` | `i32` | tahun pertama dan terakhir (untuk presipitasi dan suhu) |
| `day1`, `day2` | `i32` | hari pertama dan terakhir (untuk presipitasi dan suhu) |
| `elems` | `Vec<usize>` | nomor objek (berbasis 1) yang diubah; kosong = semua objek jenis parameter itu |

Untuk referensi, fungsi Fortran `chg_par` menghitung nilai baru dari nilai lama *x* sebagai berikut, lalu memotongnya ke \[`absmin`, `absmax`\]:

```latex
x' = \begin{cases} v & \text{absval} \\ x + v & \text{abschg} \\ (1 + v/100)\,x & \text{pctchg} \\ (1 + v)\,x & \text{relchg} \end{cases}, \qquad x'' = \min(\max(x', absmin), absmax)
```

**Struct `CalCond`** (tipe `calibration_conditions`): `var` (variabel yang diuji; `cal_conditions` mengenali antara lain `hsg`, `texture`, `plant`, `pl_class`, `landuse`, `cal_group`, `res_typ`, `sol`, dan `range`), `alt` (operator), `targ: f32` (angka; tidak dipakai oleh kondisi yang diterapkan SWAT+), dan `targc` (teks yang harus sama dengan variabel). Kondisi `range` hanya memakai `var`; batasnya adalah `val1`/`val2` milik `CalUpdate`.

#### 3.9.1  `CalParm::default`

- **Lokasi:** `crates/swat-model/src/cal.rs`, `impl Default for CalParm`
- **Padanan SWAT+:** inisialisasi `calibration_parameters`.
- **Tujuan:** `name = "default"`, `ob_typ = "plt"`, `absmin = 0`, `absmax = 1×10⁶`.

#### 3.9.2  `define_unit_elements`

- **Lokasi:** `crates/swat-model/src/cal.rs`, `pub fn define_unit_elements(cnt: &[i32]) -> Vec<usize>`
- **Padanan SWAT+:** subrutin `define_unit_elements` (`define_unit_elements.f90`), yang bekerja pada larik global `elem_cnt` dan mengisi `defunit_num`.
- **Tujuan:** Mengekspansi daftar nomor objek yang ditulis ringkas, di mana bilangan negatif menutup rentang yang dibuka bilangan sebelumnya. Contoh: `1 -1858` berarti objek 1 sampai 1858; `2 5 -7 9` berarti 2, 5, 6, 7, 9.
- **Masukan dan keluaran:** `cnt` = daftar integer seperti tertulis di file (tanpa hitungannya); keluaran daftar nomor objek berbasis 1.
- **Cara kerja:** dengan `n = len(cnt)` dan penunjuk `ii` berbasis 1:
  1. Bila `n = 1`, keluaran adalah `[max(cnt[1], 0)]`, selesai.
  2. Selama `ii <= n`:
     - Bila `ii = n` (elemen terakhir yang belum terpakai): ambil `ie1 = cnt[ii-1]`, `ie2 = cnt[ii]`. Bila `ie2 > 0`, tambahkan `ie2`; selain itu tambahkan `ie1, ie1+1, …, |ie2|`. Lalu `ii += 1`.
     - Selain itu: ambil `ie1 = cnt[ii]`, `ie2 = cnt[ii+1]`. Bila `ie2 > 0`, tambahkan `ie1` dan `ii += 1`; selain itu tambahkan rentang `ie1 … |ie2|` dan `ii += 2`.
  3. Setiap nilai yang ditambahkan dipotong ke `max(·, 0)` sebelum diubah ke `usize`.
- **Kebiasaan SWAT+ / catatan port:**
  - Algoritme Fortran berjalan dua kali (hitung dulu, lalu alokasikan dan isi); port melakukannya dalam satu lintasan dengan hasil yang sama.
  - Untuk daftar kosong (`n = 0`), hasilnya kosong. Pemanggil di `swat-io` menafsirkan hitungan 0 sebagai "semua objek" (`elems` kosong), sesuai `docs/swat-quirks.md`.
  - Rentang yang terbalik (mis. `5 -3`) menghasilkan rentang kosong, sama seperti `do ie = 5, 3` di Fortran.
  - Kasus patologis di mana bilangan negatif muncul sendiri di posisi terakhir setelah sebuah rentang (mis. `1 -3 -5`) memakai elemen sebelumnya yang negatif sebagai awal rentang; Fortran akan menghasilkan nomor negatif, sedangkan port memotongnya menjadi 0. Masukan seperti ini tidak dihasilkan oleh SWAT+ Editor.

#### 3.9.3  Uji unit `cal.rs`

| Fungsi uji | Lokasi | Yang diuji |
| --- | --- | --- |
| `unit_elements` | `cal.rs`, modul `tests` | `[1, -4]` → 1, 2, 3, 4; `[7]` → 7; `[2, 5, -7, 9]` → 2, 5, 6, 7, 9 |

### 3.10  `mgt.rs`: jadwal pengelolaan dan tabel keputusan

Modul ini tidak berisi fungsi; isinya struct untuk `management.sch` (dibaca `mgt_read_mgtops`/`read_mgtops`) dan tabel keputusan `lum.dtl` (serta `res_rel.dtl`, yang memakai struktur sama; dibaca `dtbl_lum_read`/`dtbl_res_read`). Eksekusi operasi dan evaluasi tabel keputusan dibahas di bab pengelolaan HRU.

**Struct `MgtOp`** (tipe `management_ops`, satu operasi terjadwal seperti tertulis di file):

| Field | Tipe | Arti |
| --- | --- | --- |
| `op` | `String` | kode operasi: `plnt` (tanam), `hvkl` (panen dan matikan), `till` (olah tanah), `irrm` (irigasi), `fert` (pemupukan), `skip` (lompat ke tahun berikutnya), dan lain-lain |
| `mon`, `day` | `i32` | bulan dan tanggal operasi (operasi bertanggal) |
| `husc` | `f32` | fraksi unit panas pemicu operasi; 0 untuk operasi bertanggal |
| `op_char` | `String` | nama pertama (`op_data1`): tanaman, alat olah, operasi irigasi, atau pupuk |
| `op_plant` | `String` | nama kedua (`op_data2`): operasi panen, atau aplikasi kimia |
| `op3` | `f32` | jumlah (`op_data3`), mis. dosis pupuk (kg/ha) |

**Struct `MgtSchedule`** (tipe `management_schedule`): `name`, `autos: Vec<String>` (nama tabel keputusan otomatis, `auto_name`), `auto_crops: Vec<String>` (tanaman untuk tabel tanam-panen generik `auto_crop`: satu untuk `pl_hv_summer1` dan `pl_hv_winter1`, dua untuk `pl_hv_summer2`, ditulis setelah nama tabelnya), dan `ops: Vec<MgtOp>`.

**Struct `DtblCond`** (tipe `conditions_var`, satu kondisi tabel keputusan): `var` (variabel yang diuji, mis. `soil_water`, `phu_plant`, `year_rot`, `jday`), `ob` (jenis objek), `ob_num: i32` (nomor objek; 0 = objek yang sedang diproses), `lim_var` (variabel pembatas), `lim_op` (operator pada batas: `*`, `+`, `-`, `/`), dan `lim_const: f32` (konstanta pembatas).

**Struct `DtblAct`** (tipe `actions_var`, satu aksi): `typ` (jenis aksi, mis. `plant`, `harvest_kill`, `fertilize`, `irrigate`, `till`), `ob`, `ob_num: i32`, `name` (nama aksi), `option` (opsi, mis. nama tanaman atau pupuk), `const_: f32` (konstanta pertama; dinamai `const_` karena `const` adalah kata kunci Rust), `const2: f32` (konstanta kedua; paling kecil 1, sebagaimana dipaksakan `dtbl_lum_read`), dan `file_pointer` (nama rujukan tambahan, mis. operasi aplikasi).

**Struct `DecisionTable`** (tipe `decision_table`):

| Field | Tipe | Arti |
| --- | --- | --- |
| `name` | `String` | nama tabel |
| `alts` | `usize` | jumlah alternatif (kolom) |
| `conds` | `Vec<DtblCond>` | kondisi (baris atas) |
| `alt` | `Vec<Vec<String>>` | `alt[kondisi][alternatif]`: operator perbandingan tiap sel (`<`, `>`, `=`, `-` untuk diabaikan, dsb.) |
| `acts` | `Vec<DtblAct>` | aksi (baris bawah) |
| `outcomes` | `Vec<Vec<String>>` | `outcomes[aksi][alternatif]`: `y` bila aksi dijalankan saat alternatif terpenuhi, `n` bila tidak |

Penamaan `alt(cond, alt)` dan `act_outcomes(act, alt)` di komentar Rust mengikuti nama larik dua dimensi di Fortran; di port larik itu disimpan sebagai vektor dari vektor dengan urutan indeks yang sama.

### 3.11  `objects.rs`: objek spasial dan sambungannya

SWAT+ memodelkan DAS sebagai graf objek spasial (HRU, unit routing, akuifer, kanal, waduk, recall, outlet, dan sebagainya). Setiap jenis objek punya file sambungan `*.con` yang dibaca `hyd_read_connect`: satu baris per objek, berisi identitas, lokasi, penunjuk data, stasiun cuaca, dan daftar ke mana airnya dialirkan. Modul ini mendefinisikan representasi baris-baris itu. Penyusunan urutan perhitungan dan routing hidrograf berdasarkan graf ini dibahas di bab routing.

**Enum `ObjectKind`** (13 jenis, satu per file sambungan). Konstanta `ObjectKind::ALL` memuatnya dalam urutan baris `connect` `file.cio`:

| Varian | Slot `file.cio` | File bawaan | Kode `obj_typ` penerima |
|---|---|---|---|
| `Hru` | `hru_con` | `hru.con` | `hru` |
| `HruLte` | `hruez_con` | `hru-lte.con` | `hlt` |
| `RoutingUnit` | `ru_con` | `rout_unit.con` | `ru` |
| `Gwflow` | `gwflow_con` | `gwflow.con` | `gwflow` |
| `Aquifer` | `aqu_con` | `aquifer.con` | `aqu` |
| `Aquifer2d` | `aqu2d_con` | `aquifer2d.con` | tidak ada (tidak menerima air) |
| `Channel` | `chan_con` | `channel.con` | `cha` |
| `Reservoir` | `res_con` | `reservoir.con` | `res` |
| `Recall` | `rec_con` | `recall.con` | tidak ada (hanya sumber) |
| `ExportCoefficient` | `exco_con` | `exco.con` | `exc` |
| `DeliveryRatio` | `delr_con` | `delratio.con` | `dr` |
| `Outlet` | `out_con` | `outlet.con` | `out` |
| `ChannelDeg` | `chandeg_con` | `chandeg.con` | `sdc` |

`ChannelDeg` adalah kanal SWAT-DEG, jenis kanal yang dirouting SWAT+ saat ini.

**Struct `SpatialObject`** (satu baris file sambungan):

| Field | Tipe | Arti (satuan) |
|---|---|---|
| `kind` | `ObjectKind` | jenis objek |
| `id` | `i32` | nomor dalam jenisnya |
| `name` | `String` | nama |
| `gis_id` | `i32` | identitas GIS |
| `area_ha` | `f32` | luas (ha) |
| `lat`, `lon` | `f32` | lintang dan bujur (derajat) |
| `elev` | `f32` | elevasi (m) |
| `props` | `i32` | baris data objek di file data jenisnya (kolom `hru`, `rtu`, `aqu`, `lcha`, …; SWAT+ `props`) |
| `weather_station` | `String` | nama stasiun cuaca (`wst`) |
| `constit` | `i32` | penunjuk data konstituen (`cst`) |
| `props2` | `i32` | penunjuk kedua, mis. *overbank* (`ovfl`; SWAT+ `props2`) |
| `ruleset` | `String` | set aturan aliran (`rule`) |
| `outflows` | `Vec<Outflow>` | daftar tujuan aliran keluar |

**Struct `Outflow`** (satu bagian air yang dikirim ke objek lain): `obj_type: String` (kode jenis penerima, lihat tabel di atas), `obj_id: i32` (nomor penerima dalam jenisnya), `hyd_type: String` (komponen air: `tot` total, `sur` limpasan permukaan, `lat` aliran lateral, `til` aliran tile, `rhg` imbuhan, `aqu`, …), dan `frac: f32` (fraksi komponen itu yang dikirim).

#### 3.11.1  `ObjectKind::cio_slot`

- **Lokasi:** `crates/swat-model/src/objects.rs`, `pub fn cio_slot(self) -> &'static str`
- **Padanan SWAT+:** komponen `in_con%hru_con`, `in_con%ru_con`, dan seterusnya.
- **Tujuan:** Nama slot file sambungan jenis ini di baris `connect` `file.cio` (kolom kedua tabel di atas). Dipakai pembaca untuk memperoleh nama file lewat `FileCio::file("connect", slot)`.

#### 3.11.2  `ObjectKind::code`

- **Lokasi:** `crates/swat-model/src/objects.rs`, `pub fn code(self) -> Option<&'static str>`
- **Padanan SWAT+:** nilai `obj_typ` yang dikenali `hyd_read_connect` saat memetakan tujuan aliran ke larik objek.
- **Tujuan:** Kode teks yang dipakai objek lain untuk mengirim air ke jenis ini.
- **Cara kerja:** pemetaan tetap (kolom terakhir tabel di atas); `Aquifer2d` dan `Recall` mengembalikan `None` karena tidak dapat menjadi penerima.
- **Catatan port:** engine memakai fungsi ini secara terbalik (dari `Outflow.obj_type` ke `ObjectKind`) saat membangun graf routing.

### 3.12  `routing.rs`: unit routing, unit lanskap, akuifer, dan kanal SWAT-DEG

Modul ini berisi struct untuk file definisi unit routing (`rout_unit.ele`, `rout_unit.def`, `rout_unit.rtu`; dibaca `ru_read_elements` dan `ru_read`), unit lanskap (`ls_unit.def`, `ls_unit.ele`; tipe `lsu_out`, `lsu_elem`), basis data akuifer (`aquifer.aqu`; `aqu_read`), dan nama data kanal SWAT-DEG (`channel-lte.cha`).

**Struct `RuElement`** (tipe `routing_unit_elements`, satu baris `rout_unit.ele`): `name`, `obtyp` (jenis objek elemen: `hru`, `ru`, `sdc`, …), `obtypno: i32` (nomornya), `frac: f32` (bagian objek itu di dalam unit), dan `dr_name` (nama rasio pengiriman: `0` atau `calc` = dihitung dari waktu konsentrasi, `1` atau `full` = tanpa pengurangan).

**Struct `LsuDef`** (`ls_unit.def`, `lsu_out`): `name`, `area_ha: f32` (ha), `elems: Vec<usize>` (nomor baris `ls_unit.ele`, yang kemudian dibaca SWAT+ sebagai nomor HRU).

**Struct `LsuElem`** (`ls_unit.ele`, `lsu_elem`): `name`, `obtyp`, `obtypno: i32`, `bsn_frac: f32` (bagian elemen terhadap basin), `ru_frac: f32` (bagian terhadap unit lanskapnya).

**Struct `RuDef`** (`rout_unit.def`): `name`, `elems: Vec<usize>` (nomor baris `rout_unit.ele`, berbasis 1, sudah diekspansi dengan `define_unit_elements`).

**Struct `RuData`** (`rout_unit.rtu`): `name`, lalu nama-nama rujukan `elem_def` (definisi), `elem_dr` (rasio pengiriman), `topo` (topografi), `field` (lahan).

**Struct `AquiferDb`** (tipe `aquifer_database`, satu baris `aquifer.aqu`; satuan dari `aquifer_module.f90`):

| Field | Arti (satuan) | Bawaan |
|---|---|---|
| `name`, `init` | nama akuifer dan nama data awal di `initial.aqu` | kosong |
| `flo` | aliran balik awal (mm) | 0,05 |
| `dep_bot` | kedalaman dari permukaan lereng tengah ke dasar akuifer (m) | 0 |
| `dep_wt` | kedalaman awal muka air tanah (m) | 0 |
| `no3` | konsentrasi awal NO3-N (ppm) | 0 |
| `minp` | konsentrasi awal P mineral (ppm) | 0 |
| `cbn` | karbon organik awal (%) | 0,5 |
| `flo_dist` | jarak aliran rata-rata ke sungai atau objek (m) | 50 |
| `bf_max` | aliran dasar harian maksimum (mm) | 0 |
| `alpha` | konstanta resesi aliran dasar (1/hari) | 0 |
| `revap_co` | koefisien *revap* (evaporasi = PET × `revap_co`) | 0 |
| `seep` | fraksi imbuhan yang merembes ke akuifer dalam | 0 |
| `spyld` | hasil jenis akuifer (m³/m³) | 0 |
| `hlife_n` | waktu paruh nitrat di air tanah (hari) | 30 |
| `flo_min` | kedalaman muka air tanah agar aliran balik terjadi (m) | 0 |
| `revap_min` | kedalaman muka air tanah agar *revap* terjadi (m) | 0 |

**Struct `SdChannelData`** (tipe `swatdeg_datafiles`, satu baris `channel-lte.cha`): `name`, lalu nama-nama rujukan `init` (`initial.cha`), `hyd` (`hyd-sed-lte.cha`), `sed`, `nut` (`nutrients.cha`).

#### 3.12.1  `AquiferDb::default`

- **Lokasi:** `crates/swat-model/src/routing.rs`, `impl Default for AquiferDb`
- **Padanan SWAT+:** inisialisasi komponen `aquifer_database` di `aquifer_module.f90`.
- **Tujuan:** Nilai bawaan akuifer (kolom terakhir tabel di atas): `flo` 0,05 mm, `cbn` 0,5 %, `flo_dist` 50 m, `hlife_n` 30 hari, lainnya 0.
- **Catatan port:** `Project.aquifer_db[0]` menyimpan nilai bawaan ini, meniru larik Fortran yang elemen 0-nya berisi bawaan tipe; baris file mulai dari indeks 1.

### 3.13  `print.rs`: pengaturan keluaran `print.prt`

`print.prt` menentukan keluaran apa yang ditulis SWAT+ dan kapan; dibaca `basin_print_codes_read`. Penulisan keluaran sendiri dibahas di bab keluaran.

**Struct `PrintSettings`**:

| Field | Tipe | Arti |
|---|---|---|
| `nyskip` | `i32` | jumlah tahun yang dilewati sebelum mencetak (*warm-up*) |
| `day_start`, `yrc_start`, `day_end`, `yrc_end` | `i32` | jendela cetak (hari Julian dan tahun) |
| `interval` | `i32` | cetak setiap *n* hari (SWAT+ `int_day`) |
| `aa_years` | `Vec<i32>` | tahun-tahun interval rata-rata tahunan; kosong = seluruh periode |
| `csvout` | `String` | `y` = tulis juga file CSV |
| `use_obj_labels` | `String` | bendera kedua baris itu (header file: `dbout`; SWAT+: `use_obj_labels`) |
| `cdfout` | `String` | bendera keluaran netCDF |
| `crop_yld` | `String` | bendera pertama baris berikutnya (header file: `soilout`; SWAT+: `crop_yld`) |
| `mgtout` | `String` | tulis keluaran pengelolaan |
| `hydcon` | `String` | tulis keluaran sambungan hidrograf |
| `fdcout` | `String` | tulis kurva durasi aliran |
| `objects` | `Vec<PrintObject>` | tabel objek keluaran, urutan file |

**Struct `PrintObject`**: `name` (nama keluaran, mis. `basin_wb`, `hru_wb`, `channel_sd`) dan empat bendera `daily`, `monthly`, `yearly`, `avann` (`y`, `n`, dan `l` untuk sebagian keluaran karbon).

**Kebiasaan SWAT+** (dari `docs/swat-quirks.md`): jumlah interval rata-rata tahunan dibaca lebih dulu, lalu rekamannya dibaca ulang beserta tahun-tahunnya (`backspace`). Setelah itu jendela cetak dilengkapi: hari pertama 0 menjadi 1, hari terakhir 0 menjadi 366, tahun pertama 0 menjadi tahun simulasi pertama, tahun terakhir 0 menjadi satu tahun setelah tahun simulasi terakhir, dan penghitung interval mulai dari nilai interval. Pelengkapan ini dilakukan di `swat-io/src/print.rs` dan `swat-engine/src/run.rs`, bukan di crate model; `PrintSettings` menyimpan nilai seperti tertulis di file.

#### 3.13.1  `PrintSettings::object`

- **Lokasi:** `crates/swat-model/src/print.rs`, `pub fn object(&self, name: &str) -> Option<&PrintObject>`
- **Padanan SWAT+:** tidak ada fungsi padanan langsung; SWAT+ membaca tabel objek ke variabel bernama tetap (`pco%...`) menurut posisi baris.
- **Tujuan:** Mencari baris tabel objek keluaran menurut nama.
- **Cara kerja:** iterasi `objects` dan kembalikan yang pertama dengan `name` sama persis (perbandingan string Rust biasa, bukan `fortran_eq`); `None` bila tidak tercantum.
- **Catatan port:** pencarian hanya cocok bila nama di file ditulis persis; tidak ada pemangkasan spasi atau penyamaan huruf.

### 3.14  Rekapitulasi fungsi

| Berkas | Fungsi non-uji | Fungsi uji |
|---|---|---|
| `lib.rs` | 1 (`Project::objects_of`) | 0 |
| `time.rs` | 7 (`normalized`, `years`, `days`, `is_leap`, `month_table`, `month_day`, `month_day_in`) | 5 |
| `table.rs` | 14 (`f`, `a`, `as_real`, `as_int`, `as_text`, `default_row`, `empty`, `offset`, `get`, `real`, `int`, `text`, `find`, `fortran_eq`) | 0 |
| `schema.rs` | 0 (39 konstanta skema) | 0 |
| `cio.rs` | 2 (`defaults`, `file`) | 0 |
| `basin.rs` | 8 (`BasinCodes::default`, `BasinParams::default`, `get`, `complete`, `adj_pkr`, `dorm_hr`, `plaps`, `tlaps`) | 0 |
| `climate.rs` | 3 (`cio_slot`, `path_section`, `values`) | 0 |
| `hru.rs` | 1 (`PlantInit::default`) | 0 |
| `cal.rs` | 2 (`CalParm::default`, `define_unit_elements`) | 1 |
| `mgt.rs` | 0 | 0 |
| `objects.rs` | 2 (`cio_slot`, `code`) | 0 |
| `routing.rs` | 1 (`AquiferDb::default`) | 0 |
| `print.rs` | 1 (`object`) | 0 |
| **Jumlah** | **42** | **6** |
