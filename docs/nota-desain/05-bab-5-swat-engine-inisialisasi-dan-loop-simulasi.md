## Bab 5  swat-engine: inisialisasi dan loop simulasi

Bab ini menjelaskan jantung *crate* `swat-engine`: bagaimana sebuah `Project` (hasil pembacaan berkas masukan pada bab-bab sebelumnya) diubah menjadi simulasi harian, dan bagaimana hasil harian itu dijumlahkan menjadi tabel bulanan, tahunan, dan rata-rata tahunan. File yang dicakup adalah `crates/swat-engine/src/lib.rs` (deklarasi modul), `run.rs` (loop simulasi, trait `OutputSink`, `RunOptions`, `RunReport`, `run`, `run_with`, `regions`, `lapse_rates`, dan gerbang cetak harian), `clock.rs` (jam simulasi), `fortran.rs` (rutin kecil yang meniru gfortran dan SWAT+ secara bit demi bit), `hyd.rs` (tipe hidrograf dan operatornya), serta `periods.rs` (akumulator periode).

Dalam SWAT+ Fortran, peran ini dipegang oleh `time_control.f90` (loop tahun dan hari), `command.f90` (loop objek per hari beserta panggilan rutin keluaran), dan rutin keluaran seperti `hru_output.f90`, `basin_output.f90`, `aquifer_output.f90`, `sd_channel_output.f90`. Port Rust memisahkan perhitungan dari penulisan berkas: engine hanya memanggil method trait `OutputSink`, sedangkan penulisan tabel teks dilakukan oleh *crate* lain (lihat bab tentang keluaran). Semua bab tentang HRU, iklim, routing, kanal, dan reservoir yang lain dipanggil dari loop yang dijelaskan di sini.

Prinsip yang dipegang di seluruh bab: aritmetika dikerjakan dalam presisi tunggal (`f32`) dengan urutan operasi persis seperti Fortran, sebab dalam presisi tunggal urutan penjumlahan menentukan hasil. Bila sebuah fungsi tampak janggal (misalnya membagi dua kali, atau mengembalikan nilai lama setelah dijumlah), hampir selalu itu karena SWAT+ melakukannya demikian dan port mempertahankannya agar keluaran identik.

### 5.1  Modul crate (`lib.rs`) dan urutan satu hari simulasi

`lib.rs` tidak berisi fungsi. Isinya: dokumentasi crate, atribut lint, dan deklarasi sebelas modul publik: `channel`, `climate`, `clock`, `fortran`, `hru`, `hyd`, `periods`, `res`, `route`, `run`, `waterbody`.

Dua atribut lint patut dicatat karena mencerminkan kebijakan port:

- `#![allow(clippy::needless_range_loop, clippy::approx_constant, clippy::manual_clamp)]`: loop berindeks dan konstanta SWAT+ sendiri (misalnya `3.1416` dan `6.283185` alih-alih `π`) sengaja dipertahankan seperti ditulis Fortran, agar aritmetikanya sama. Mengganti `6.283185` dengan `2π` presisi penuh akan mengubah bit terakhir hasil.
- `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, ...))]`: di luar pengujian, engine dilarang *panic*. Semua kegagalan dikembalikan sebagai `Result<_, String>` atau dicatat di `RunReport`.

#### 5.1.1  Urutan proses satu hari simulasi

Daftar bernomor berikut adalah diagram alir `run_with` (bagian 5.3), dengan padanan di `time_control.f90` dan `command.f90`. Indentasi menunjukkan sarang loop.

1. **Inisialisasi sebelum loop** (padanan: bagian awal `time_control` dan seluruh rantai `proc_*` sebelum itu).
   1. `Clock::new` membuat jam dari `time.sim` (tabel `ndays` mula-mula tabel tahun kabisat).
   2. `Weather::new` menyiapkan stasiun cuaca, generator cuaca (WGN), dan berkas pengukur (bab iklim).
   3. Untuk setiap HRU: cari stasiunnya dengan `fortran::search` (gagal = stasiun 1), lalu hitung koreksi *lapse rate* dengan `lapse_rates` (padanan `cli_lapse`).
   4. `init_hrus` membangun seluruh HRU (tanah, tanaman, CN, hidrologi). Bila gagal atau jumlahnya tidak cocok dengan `hru.con`, alasannya dicatat di `RunReport::hru_skipped` dan simulasi berlanjut hanya dengan cuaca.
   5. Bila HRU berhasil, `route::Network::new` membangun jaringan objek (unit routing, akuifer, kanal, reservoir, *recall*). Kegagalan dicatat di `routing_skipped`; kanal yang tidak dirouting dicatat di `channel_skipped`.
   6. Siapkan `HruBasin` (parameter cekungan, dengan `--fix-swat` bila diminta), status bersama `Saved`, pemetaan stasiun deposisi atmosfer (padanan `cli_read_atmodep`), dan indeks waktu awal deposisi.
   7. `clock.before_loop()` lalu `weather.precip_control(0, …)`: hujan hari pertama dibaca/dibangkitkan lebih dulu (padanan `call cli_precip_control (0)` sebelum `do curyr`).
   8. Buat `PrintGate` (jendela dan interval cetak harian), `PrintYears`, dan semua akumulator periode. Susun daftar tanaman cekungan (padanan `plant_all_init`).
2. **Untuk setiap tahun** `curyr = 1..nbyr` (padanan `do curyr = 1, time%nbyr`):
   1. `clock.start_year(curyr)`: tabel bulan tahun itu (kabisat Gregorian), hari pertama dan terakhir tahun itu.
   2. `prt.start_year(...)`: tambahkan hari tahun tercetak ke `yrs_prt` dan `days_prt` (atau set `yrs_prt = nbyr` pada tahun pemanasan).
   3. Bila ini tahun tercetak pertama (`yrs > nyskip`): `Hru::start_balance` untuk setiap HRU (padanan `basin_sw_init`).
   4. **Untuk setiap hari** `jday = day_start..day_end_yr` (padanan `do julian_day`):
      1. `clock.set_day(jday)`: bulan, tanggal, dan bendera akhir tahun (padanan `xmon` dan pemeriksaan `end_yr`).
      2. `out.progress(...)` untuk laporan kemajuan.
      3. `gate.update(&clock)`: perbarui status cetak harian (padanan blok `pco%day_print`).
      4. `weather.climate_control(&clock)`: cuaca hari ini di semua stasiun (padanan `climate_control`).
      5. Hitung bendera `output` (`yrs > nyskip`), `printing` (output dan gerbang membuka), `end_sim` (hari terakhir tahun terakhir; saat itu `prt.end_sim` membagi `yrs_prt`), dan isi `PeriodDay`.
      6. Majukan indeks deposisi atmosfer (padanan `cli_atmodep_time_control`).
      7. **Fase lahan, HRU demi HRU dalam urutan perintah SWAT+** (`Network::hru_order`, sehingga HRU yang menerima air HRU lain dihitung setelahnya; padanan cabang `case ("hru")` dalam `command`):
         1. Salin cuaca stasiun ke `HruWeather`, tambahkan deposisi atmosfer, lalu bila `lapse = 1` terapkan koreksi elevasi (padanan awal `hru_control`).
         2. Bila PET Penman-Monteith (`pet = 1`), naikkan angin minimum menjadi 0,01 m/s.
         3. Bentuk `DayCtx` (tanggal, data WGN, *runon* dari HRU hulu via `Network::hru_runon`) dan jalankan `Hru::day` (padanan `hru_control`).
         4. Catat panen ke hasil tanaman cekungan; bila `output`, `Hru::close_day` (bagian harian `hru_output`).
         5. Serahkan hidrograf HRU ke jaringan (`Network::set_hru`).
      8. **Keluaran HRU dalam urutan nomor HRU**: `out.hru_day` (bila mencetak), akumulasi periode `HruAcc::day` (bila `output`), baris untuk wilayah, dan pada hari terakhir simulasi `out.crop_yld_aa`.
      9. `regions(...)`: neraca cekungan dan unit lanskap (padanan `basin_output`, `lsu_output`).
      10. **Fase routing** `Network::day` (padanan sisa loop `command`: unit routing, akuifer, kanal SWAT-DEG, reservoir, *recall*, dalam urutan perintah).
      11. **Keluaran objek routing** bila `output`: jumlah kanal cekungan, jumlah akuifer cekungan (berbobot luas), lalu per kanal (subharian dan harian), per akuifer, per reservoir dan cekungan reservoir (setelah itu aliran masuk/keluar harian reservoir dinolkan, seperti `basin_reservoir_output`), per *recall* dan cekungan *recall*, terakhir per unit routing.
      12. Bila `day == 181`: `Hru::midyear` (reset unit panas dasar di belahan bumi selatan).
   5. **Akhir tahun** (setelah loop hari): tulis hasil tanaman cekungan tahunan dan, pada tahun terakhir, rata-rata tahunannya; `Hru::end_year` untuk setiap HRU (pencampuran tanah `cswat = 0`, umur tanaman tahunan, reset unit panas belahan utara); `yrc += 1`.
3. **Akhir simulasi**: satu panggilan `out.progress(total, total, …)` terakhir dan `Ok(report)`.

Perbedaan struktural dengan Fortran: `command.f90` menjalankan semua objek (HRU, unit routing, akuifer, kanal, …) dalam satu loop `do while (icmd /= 0)` dan baru setelah itu memanggil semua rutin keluaran. Port membagi loop itu menjadi fase HRU (di `run_with`) dan fase jaringan (`Network::day`). Hal ini setara karena HRU hanya menerima air dari HRU lain (dijamin oleh `hru_order`), bukan dari objek di hilir. Urutan *panggilan keluaran* juga sedikit berbeda (Fortran memanggil `hru_output` untuk semua HRU, lalu `aquifer_output`, `sd_channel_output`, `reservoir_output`, `ru_output`, `recall_output`, lalu `basin_output`, `lsu_output`, `basin_aquifer_output`, …), tetapi karena keluaran tidak mengubah keadaan model kecuali di tempat yang ditiru (reset `in_d`/`out_d` reservoir), angka yang dihasilkan sama. Fitur `time_control` yang belum diport (tabel keputusan bersyarat `conditions`/`actions` lewat `cond_up`, alokasi pupuk kandang `mallo_control`, alokasi air `wallo_*`, `calsoft`, HRU-lte, interval rata-rata tahunan `aa_numint`) tidak muncul di loop ini.

### 5.2  Tipe data `run.rs` dan trait `OutputSink`

`run.rs` mendefinisikan tipe-tipe yang menjadi antarmuka antara engine dan pemakainya (program `swatplus-rs`, modul WebAssembly, atau pengujian).

| Tipe | Jenis | Isi dan satuan |
| --- | --- | --- |
| `HruWeather` | struct `Copy` | Cuaca yang dilihat HRU pada satu hari (`w` di `hru_control`): `precip` (mm), `ts` (hujan per langkah waktu, mm, larik `MAX_STEP`, hanya `step` nilai pertama terpakai), `tmax`, `tmin`, `tave` (°C), `solrad`, `solradmx` (MJ/m²), `windsp` (m/s), `rhum` (fraksi), `phubase0` (unit panas dasar 0 °C), `station_pet` (PET Hargreaves stasiun, mm, minimal 0,01, dipakai bila `pet = 3`), `atmo` (deposisi atmosfer: NO3 hujan mg/L, NO3 kering kg/ha/hari, NH4 hujan mg/L, NH4 kering kg/ha/hari; `None` tanpa `atmo.cli`). |
| `Day` | struct `Copy` | Tanggal hari tercetak: `jday` (hari ke-n tahun), `mon`, `day` (tanggal), `yr` (tahun kalender). |
| `Region` | enum | `Basin` atau `Lsu`: wilayah tabel neraca tingkat cekungan. |
| `Period` | enum (diekspor ulang dari `periods`) | `Mon`, `Yr`, `Aa`. |
| `HruRows` | struct (diekspor ulang dari `periods`) | Satu baris periode HRU/wilayah: `wb` (32 kolom neraca air), `nb` (15 kolom neraca hara), `ls` (13 kolom kehilangan), `pw` (25 kolom tanaman dan cuaca). |
| `PrintGate` | struct privat | Saklar cetak harian SWAT+: `day_print`, `over` (`day_print_over`), `int_day_cur`, dan salinan `PrintSettings` yang sudah dilengkapi. |
| `RunReport` | struct | Apa yang dilakukan run selain keluaran: `hru_skipped`, `routing_skipped`, `channel_skipped` (masing-masing `Option<String>` berisi alasan bila bagian itu tidak disimulasikan). |
| `RunOptions` | struct `Copy` | Satu field `fixes: bool`: perbaiki bug SWAT+ (lihat `docs/swat-fixes.md`) alih-alih menirunya. Bawaan `false` = identik dengan SWAT+. |

#### 5.2.1  `HruWeather::default`

- **Lokasi:** `crates/swat-engine/src/run.rs`, `impl Default for HruWeather`, `fn default() -> Self`.
- **Padanan SWAT+:** tidak ada; khusus port (Fortran memakai variabel `w` bertipe `weather_daily` yang diisi ulang).
- **Tujuan:** Nilai awal semua nol dan `atmo = None`, dipakai untuk mengisi vektor `hru_ws` sebelum loop HRU (HRU yang stasiunnya tidak ada tetap punya baris cuaca nol).
- **Masukan dan keluaran:** tidak ada masukan; keluaran `HruWeather` nol.
- **Cara kerja:** mengisi setiap field dengan `0.0`, larik `ts` dengan nol, `atmo` dengan `None`. Implementasi manual (bukan `derive`) diperlukan karena `Default` tidak diturunkan otomatis untuk larik berukuran `MAX_STEP` yang besar.

#### 5.2.2  Trait `OutputSink`

`OutputSink` adalah satu-satunya jalur keluaran engine. Engine memanggil method-methodnya pada hari yang dicetak SWAT+ (atau pada akhir periode); implementasi di *crate* keluaran menulis tabel teks yang formatnya sama dengan SWAT+. Hanya `hru_day` yang wajib diimplementasikan; semua method lain punya implementasi bawaan kosong, sehingga pengujian cukup mengimplementasikan yang dibutuhkan. Semua method menerima `&mut self`. Argumen `Option<Period>` bernilai `None` berarti baris harian, `Some(p)` berarti baris periode. Padanan Fortran untuk setiap method adalah pernyataan `write` dalam rutin keluaran yang disebut.

#### 5.2.2.1  `progress`

- **Lokasi:** `run.rs`, `fn progress(&mut self, done: usize, total: usize, day: Day)`, bawaan kosong.
- **Padanan SWAT+:** baris `write (*,1234) cal_sim, time%mo, time%day_mo, ...` di `time_control` (tampilan kemajuan ke layar dan `9003`), secara fungsional.
- **Tujuan:** laporan kemajuan: simulasi akan memulai hari ke-`done + 1` dari `total`. Setelah selesai dipanggil sekali lagi dengan `done = total`.
- **Masukan dan keluaran:** `done`, `total` (jumlah hari `time.sim`), `day`. Tidak ada keluaran. Modul WebAssembly meneruskannya ke `env.swat_progress` pada setiap persen bulat (docs `runner.md`).
- **Kebiasaan SWAT+ / catatan port:** dipanggil untuk setiap hari, termasuk tahun pemanasan dan hari yang tidak dicetak.

#### 5.2.2.2  `hru_day`

- **Lokasi:** `run.rs`, `fn hru_day(&mut self, day: Day, hru: usize, gis_id: i32, name: &str, w: &HruWeather, d: Option<&HruDay>)`; **wajib** (tanpa bawaan).
- **Padanan SWAT+:** bagian harian `hru_output.f90` (tabel `hru_wb_day`, `hru_nb_day`, `hru_ls_day`, `hru_pw_day`) dan tabel cuaca harian.
- **Tujuan:** satu hari satu HRU: nomor HRU (mulai 1), id GIS dan nama dari `hru.con`, cuaca yang dilihatnya, dan neraca hariannya.
- **Masukan dan keluaran:** `d` bernilai `None` bila fase lahan tidak disimulasikan (lihat `RunReport::hru_skipped`); penerima lalu hanya bisa menulis cuaca.
- **Kebiasaan SWAT+ / catatan port:** dipanggil dalam urutan nomor HRU walaupun HRU dihitung dalam urutan perintah; hanya pada hari `printing`.

#### 5.2.2.3  `ru_day`

- **Lokasi:** `run.rs`, `fn ru_day(&mut self, day: Day, name: &str, hd: &Hyd)`.
- **Padanan SWAT+:** bagian harian `ru_output.f90` (`ru_day.txt`).
- **Tujuan:** hidrograf yang diteruskan sebuah unit routing (`ru_d`) pada hari tercetak.
- **Masukan dan keluaran:** `hd` dalam m³ (aliran), t (sedimen), kg (hara).

#### 5.2.2.4  `cha_day`

- **Lokasi:** `run.rs`, `fn cha_day(&mut self, day, ich: usize, gis_id: i32, name: &str, w: &ChWat, stor: &Hyd, hin: &Hyd, hout: &Hyd, wtemp: f32)`.
- **Padanan SWAT+:** bagian harian `sd_channel_output.f90` (`channel_sd_day.txt`).
- **Tujuan:** satu hari satu kanal SWAT-DEG: nomor (mulai 1), id GIS, nama, badan air (`ChWat`: luas ha, hujan, evaporasi, rembesan), simpanan, aliran masuk, aliran keluar, dan suhu air hari itu (°C).

#### 5.2.2.5  `cha_subday`

- **Lokasi:** `run.rs`, `fn cha_subday(&mut self, day, ich, gis_id, name, flo: &[f32])`.
- **Padanan SWAT+:** tabel `channel_sd_subday.txt` di `sd_channel_output.f90` (run subharian).
- **Tujuan:** aliran keluar kanal per langkah waktu.
- **Kebiasaan SWAT+ / catatan port:** nilainya dalam m³ per langkah walaupun tajuk tabel SWAT+ menulis `m^3/s` (komentar sumber). Dipanggil sebelum `cha_day` untuk kanal yang sama.

#### 5.2.2.6  `aqu_day`

- **Lokasi:** `run.rs`, `fn aqu_day(&mut self, day, iaq: usize, gis_id: i32, name: &str, d: &AquDay)`.
- **Padanan SWAT+:** bagian harian `aquifer_output.f90` (`aquifer_day.txt`).
- **Tujuan:** keadaan harian satu akuifer (aliran, kedalaman muka air, simpanan, nitrat, imbuhan, rembesan, *revap*, dsb.; rincian di bab akuifer).

#### 5.2.2.7  `hru_period`

- **Lokasi:** `run.rs`, `fn hru_period(&mut self, p: Period, day, hru, gis_id, name, rows: &HruRows, nutrients: bool)`.
- **Padanan SWAT+:** bagian bulanan/tahunan/rata-rata tahunan `hru_output.f90` (`hru_*_mon`, `_yr`, `_aa`).
- **Tujuan:** satu baris periode HRU pada hari terakhir periode itu, dihasilkan oleh `HruAcc::day`.
- **Masukan dan keluaran:** `nutrients` sama dengan untuk tabel harian (apakah tabel hara ditulis).

#### 5.2.2.8  `ru_period`

- **Lokasi:** `run.rs`, `fn ru_period(&mut self, p: Period, day, name, hd: &Hyd)`.
- **Padanan SWAT+:** `ru_output.f90` (`ru_mon`, `ru_yr`, `ru_aa`).
- **Tujuan:** jumlah periode unit routing (dari `RuAcc::day`).

#### 5.2.2.9  `cha_period`

- **Lokasi:** `run.rs`, `fn cha_period(&mut self, p: Period, day, ich, gis_id, name, w: &ChWat, stor: &Hyd, hin: &Hyd, hout: &Hyd, wtemp: f32)`.
- **Padanan SWAT+:** `sd_channel_output.f90` (`channel_sd_mon`, `_yr`, `_aa`).
- **Tujuan:** baris periode kanal (dari `ChaAcc::day`).
- **Kebiasaan SWAT+ / catatan port:** simpanan dan suhu air yang dicetak adalah nilai hari itu, bukan rata-rata periode.

#### 5.2.2.10  `aqu_period`

- **Lokasi:** `run.rs`, `fn aqu_period(&mut self, p: Period, day, iaq, gis_id, name, d: &AquDay)`.
- **Padanan SWAT+:** `aquifer_output.f90` (`aquifer_mon`, `_yr`, `_aa`).
- **Tujuan:** baris periode akuifer (dari `AquAcc::day`).

#### 5.2.2.11  `region`

- **Lokasi:** `run.rs`, `fn region(&mut self, r: Region, p: Option<Period>, day, unit: usize, gis_id: i32, name: &str, rows: &HruRows)`.
- **Padanan SWAT+:** `basin_output.f90` (`basin_wb_*`, `basin_nb_*`, `basin_ls_*`, `basin_pw_*`) dan `lsu_output.f90` (`lsunit_*`).
- **Tujuan:** neraca harian (`p = None`) atau periode cekungan maupun unit lanskap. Untuk cekungan `unit = 1`, `gis_id = 1`, nama = nama cekungan.

#### 5.2.2.12  `crop_yld_aa`

- **Lokasi:** `run.rs`, `fn crop_yld_aa(&mut self, day, hru: usize, plant: &str, y: [f32; 4])`.
- **Padanan SWAT+:** penulisan `crop_yld_aa.txt` (`bagian rata-rata tahunan hru_output.f90, unit 4008,` pada akhir simulasi).
- **Tujuan:** hasil rata-rata per panen satu tanaman HRU selama simulasi: massa, karbon, N, P (kg/ha), pada hari terakhir.
- **Kebiasaan SWAT+ / catatan port:** bila `harv_num > 0`, total hasil dibagi jumlah panen; bila belum pernah dipanen, yang dikirim adalah totalnya (docs `engine1.md`).

#### 5.2.2.13  `basin_crop_yld`

- **Lokasi:** `run.rs`, `fn basin_crop_yld(&mut self, p: Period, yrc: i32, iplt: usize, plant: &str, area: f32, yld: f32, t_ha: f32)`.
- **Padanan SWAT+:** blok `do iplt = 1, basin_plants` pada akhir tahun di `time_control.f90` (`basin_crop_yld_yr`, `_aa`).
- **Tujuan:** luas panen (ha), hasil (t), dan t/ha untuk setiap tanaman cekungan, per tahun dan rata-rata tahunan.

#### 5.2.2.14  `basin_aqu`

- **Lokasi:** `run.rs`, `fn basin_aqu(&mut self, p: Option<Period>, day, name, d: &AquDay)`.
- **Padanan SWAT+:** `basin_aquifer_output.f90` (`basin_aqu_*`).
- **Tujuan:** akuifer cekungan: setiap akuifer diberi bobot luasnya dibagi luas total cekungan.

#### 5.2.2.15  `basin_cha`

- **Lokasi:** `run.rs`, `fn basin_cha(&mut self, p: Option<Period>, day, name, w: &ChWat, stor: &Hyd, hin: &Hyd, hout: &Hyd)`.
- **Padanan SWAT+:** `basin_sdchannel_output.f90` (`basin_sd_cha_*`).
- **Tujuan:** jumlah semua kanal SWAT-DEG cekungan (tanpa bobot).

#### 5.2.2.16  `res`

- **Lokasi:** `run.rs`, `fn res(&mut self, p: Option<Period>, day, j: usize, gis_id, name, w: &ChWat, stor: &Hyd, hin: &Hyd, hout: &Hyd)`.
- **Padanan SWAT+:** `reservoir_output.f90` (`reservoir_*`).
- **Tujuan:** baris harian atau periode satu reservoir: nomor (mulai 1), id GIS, nama, badan air, simpanan, aliran masuk dan keluar.

#### 5.2.2.17  `basin_res`

- **Lokasi:** `run.rs`, `fn basin_res(&mut self, p: Option<Period>, day, unit: usize, name, w, stor, hin, hout)`.
- **Padanan SWAT+:** `basin_reservoir_output.f90` (`basin_res_*`).
- **Tujuan:** jumlah semua reservoir. `unit` adalah satu lebih dari nomor reservoir terakhir (meniru nomor yang dicetak SWAT+).

#### 5.2.2.18  `recall`

- **Lokasi:** `run.rs`, `fn recall(&mut self, p: Option<Period>, day, name, h: &Hyd)`.
- **Padanan SWAT+:** `recall_output.f90` (`recall_*`).
- **Tujuan:** beban harian atau periode satu objek *recall* (sumber titik).

#### 5.2.2.19  `basin_psc`

- **Lokasi:** `run.rs`, `fn basin_psc(&mut self, p: Option<Period>, day, name, h: &Hyd)`.
- **Padanan SWAT+:** `basin_recall_output.f90` (`basin_psc_*`).
- **Tujuan:** jumlah semua objek *recall* cekungan.

#### 5.2.3  `PrintGate::new`

- **Lokasi:** `run.rs`, `fn new(print: &PrintSettings, clock: &Clock) -> Self` (privat).
- **Padanan SWAT+:** bagian akhir `basin_print_codes_read.f90` yang melengkapi jendela cetak.
- **Tujuan:** menyalin pengaturan `print.prt` dan mengisi nilai nol dengan bawaan SWAT+.
- **Masukan dan keluaran:** `day_start`, `day_end`, `yrc_start`, `yrc_end`, `interval` dari `print.prt`; `clock.yrc`, `clock.nbyr`.
- **Cara kerja:**
  1. `day_start = 0` menjadi 1; `day_end = 0` menjadi 366.
  2. `yrc_start = 0` menjadi tahun pertama simulasi; `yrc_end = 0` menjadi `yrc + nbyr` (satu lewat tahun terakhir).
  3. `interval ≤ 0` menjadi 1.
  4. `day_print = false`, `over = false`, dan penghitung `int_day_cur` dimulai dari nilai `interval` (sehingga hari pertama yang memenuhi jendela langsung dicetak).
- **Kebiasaan SWAT+ / catatan port:** `yrc_end` bawaan yang satu lewat tahun terakhir berarti kondisi penutupan jendela di `update` tidak pernah terpenuhi kecuali pengguna mengisinya (docs `swat-quirks.md`, bagian `print.prt`).

#### 5.2.4  `PrintGate::update`

- **Lokasi:** `run.rs`, `fn update(&mut self, clock: &Clock)`.
- **Padanan SWAT+:** blok “check time interval for daily printing” di `time_control.f90` (`pco%day_print`, `pco%day_print_over`, `pco%int_day_cur`).
- **Tujuan:** memajukan status cetak harian satu hari.
- **Cara kerja:**
  1. Bila `over` sudah benar, tidak berbuat apa-apa.
  2. Bila belum mencetak: mulai mencetak (`day_print = true`) ketika `day ≥ day_start` dan `yrc ≥ yrc_start`. Penghitung interval tidak dimajukan pada hari ini.
  3. Bila sedang mencetak dan `day > day_end` serta `yrc == yrc_end`: berhenti dan tandai `over`.
  4. Selain itu: `int_day_cur += 1`, kembali ke 1 bila melampaui `interval`.
- **Kebiasaan SWAT+ / catatan port:** syarat mulai memakai `day ≥ day_start` dan `yrc ≥ yrc_start` secara terpisah, jadi pada tahun-tahun setelah `yrc_start` pencetakan dimulai juga pada hari pertama yang ≥ `day_start`. Urutan cabang persis seperti Fortran.

#### 5.2.5  `PrintGate::prints`

- **Lokasi:** `run.rs`, `fn prints(&self) -> bool`.
- **Padanan SWAT+:** syarat `pco%day_print == "y" .and. pco%int_day_cur == pco%int_day` yang dipakai rutin keluaran harian.
- **Tujuan:** apakah hari ini dicetak menurut jendela dan interval.
- **Kebiasaan SWAT+ / catatan port:** di `run_with` hasilnya masih di-AND-kan dengan `yrs > nyskip`, karena tahun pemanasan tidak pernah mencetak.

### 5.3  Fungsi simulasi `run.rs`: `run`, `run_with`, `regions`, `lapse_rates`

#### 5.3.1  `run`

- **Lokasi:** `crates/swat-engine/src/run.rs`, `pub fn run(project: &Project, out: &mut dyn OutputSink) -> Result<RunReport, String>`.
- **Padanan SWAT+:** `time_control.f90` (dipanggil dari program utama `main.f90` setelah semua pembacaan).
- **Tujuan:** menjalankan simulasi sepanjang `time.sim` dengan opsi bawaan (meniru SWAT+, tanpa koreksi).
- **Masukan dan keluaran:** proyek dan penerima keluaran; mengembalikan `RunReport` atau pesan galat.
- **Cara kerja:** satu baris: `run_with(project, out, RunOptions::default())`.

#### 5.3.2  `run_with`

- **Lokasi:** `run.rs`, `pub fn run_with(project: &Project, out: &mut dyn OutputSink, opts: RunOptions) -> Result<RunReport, String>`.
- **Padanan SWAT+:** `time_control.f90` seluruhnya, ditambah bagian `command.f90` yang memanggil `hru_control` dan rutin keluaran, `cli_lapse.f90`, `cli_read_atmodep.f90` (indeks awal), `cli_atmodep_time_control.f90`, `plant_all_init.f90` (urutan tanaman cekungan), serta awal `hru_control.f90` (penerapan *lapse rate*).
- **Tujuan:** loop tahun dan hari. Urutan lengkapnya ada di 5.1.1; berikut rincian yang penting bagi hasil.
- **Masukan dan keluaran:** `Project` (semua masukan yang sudah dibaca), `opts.fixes`. Keluaran lewat `out`; `Err(String)` hanya bila pembacaan/pembangkitan cuaca gagal (`precip_control` atau `climate_control`), sedangkan bagian yang belum diport tidak menghentikan run tetapi dicatat di `RunReport`.
- **Cara kerja:**
  1. **Jam dan cuaca.** `Clock::new(&project.time)`, `total_days = project.time.days().len()`, `Weather::new(project, &mut clock)` (pembaca suhu dapat mengubah `clock.ndays`, lihat 5.4).
  2. **Stasiun dan lapse HRU.** Daftar nama stasiun dibuat; untuk setiap objek `Hru` di `hru.con`, `ist = search(nama_stasiun, o.weather_station)` dengan bawaan 1 bila tidak ditemukan (SWAT+ membiarkan `ob%wst` bernilai awal 1), lalu `lapse_rates(project, &weather, ist, o.elev)`. Lapse aktif bila `codes.bsn` `lapse == 1`.
  3. **Generator per HRU.** `wgn_of[j]` = indeks WGN (berbasis 0) stasiun HRU itu, dipakai `init_hrus`.
  4. **Fase lahan.** `init_hrus(project, &wgn_of, &weather.wgn, clock.step, &clock.ndays)`. Bila jumlah HRU dari `hru-data.hru` berbeda dengan `hru.con`, alasan “*n* HRUs in hru-data.hru, *m* in hru.con” dicatat; bila `Err`, pesan galatnya dicatat; `land = None`.
  5. **Jaringan.** Hanya bila `land` ada: `Network::new(project, &station_names, &tconc, &ovn_km, opts.fixes)` dengan waktu konsentrasi dan (`ovn`, `km`) setiap HRU. `Ok(None)` berarti proyek tanpa objek routing.
  6. **Status lain.** `hru_wb[j] = [snomlt, surq_gen, latq, wateryld]` (mm, untuk unit routing yang membaca neraca HRU), `HruBasin::new(project).with_fixes(opts.fixes)`, `Saved` (benih acak kondisi dan opsi septik), pemetaan stasiun cuaca → stasiun deposisi atmosfer dengan `search`.
  7. **Indeks deposisi awal** (meniru `cli_read_atmodep`): untuk deret tahunan (`timestep == "yr"`) cari `iyr` dengan `yr_init + iyr − 1 = yrc_start`; untuk bulanan (`"mo"`) jalankan bulan-tahun dari (`yr_init`, `mo_init`) sebanyak `num` langkah sampai cocok dengan tahun dan bulan awal simulasi, lalu `atmo_ts = 12·(yrc0 − yr_init) + imo`. Bila ditemukan, `atmo_first = false`.
  8. `clock.before_loop()`; `weather.precip_control(0, &clock)?`.
  9. **Akumulator.** `PrintGate::new`, `PrintYears::default()`, `HruAcc` per HRU; `RuAcc`, `ChaAcc`, `AquAcc`, `ResAcc`, `RecAcc` dibuat malas (diperbesar dengan `resize` saat objek ke-*k* pertama muncul). `basin_on = lsu_elems.len() > 1` (padanan `db_mx%lsu_elem > 0`; elemen indeks 0 tampaknya pengisi agar indeks berbasis 1). `RegionAcc::new(false)` untuk cekungan, `RegionAcc::new(true)` per unit lanskap.
  10. **Tanaman cekungan** (`plant_all_init`): menelusuri komunitas tanaman setiap HRU, nama tanaman yang belum ada ditambahkan ke `plts_bsn` dalam urutan kemunculan pertama; `bsn_num[j][ipl]` menyimpan posisinya.
  11. **Loop tahun.** Untuk `curyr = 1..=nbyr`: `clock.start_year`, `prt.start_year(yrs, nyskip, nbyr, is_leap(yrc), day_start, day_end_yr)`, lalu satu kali saja `start_balance` HRU pada tahun tercetak pertama.
  12. **Loop hari.** Untuk setiap `jday`: `set_day`, `progress`, `gate.update`, `climate_control`. Lalu:
      - `output = yrs > nyskip`; `printing = output && gate.prints()`; `end_sim = end_yr && yrs == nbyr` (memanggil `prt.end_sim`).
      - `PeriodDay`: `end_mo = (day == ndays[mo])`, `end_yr`, `end_sim`, `mon_days = ndays[mo] − ndays[mo−1]`, `day_end_yr`, `yrs_prt`, `days_prt`.
      - **Deposisi atmosfer harian.** Bila masih `atmo_first`, indeks menjadi 1 ketika tahun (dan bulan, untuk deret bulanan) mencapai awal deret. Selain itu indeks maju 1 pada akhir tahun (deret tahunan) atau akhir bulan (deret bulanan). Nilai hari ini: untuk `"aa"` elemen pertama; untuk `"mo"`/`"yr"` elemen `atmo_ts − 1` bila `1 ≤ atmo_ts ≤ num`, dengan deposisi kering dibagi jumlah hari bulan (bulanan) atau 365 (tahunan dan rata-rata tahunan) agar menjadi kg/ha/hari. Di luar jangkauan: `None`.
      - **HRU dalam urutan perintah.** `hru_order` dari jaringan bila panjangnya cocok, kalau tidak urutan 0..n. Untuk HRU *j*: salin cuaca stasiun, terapkan lapse (lihat persamaan di bawah), naikkan angin ke 0,01 m/s bila `basin.pet == 1`, isi `DayCtx` (antara lain `tmp_an`, `phutot`, `daylmn`, `dormhr = daylth` dari WGN HRU, `tave` dan panjang hari stasiun, hujan setengah jam maksimum, `end_yr = (day == day_end_yr)`, `runon`), panggil `h.day(&w, &ctx, &basin, &mut saved)`.
      - **Panen.** Untuk setiap `(ipl, y)` di `d.harvests`: `bsn_yld[k].area += area_ha`, `bsn_yld[k].yld += y·area_ha/1000` (kg/ha × ha / 1000 = t).
      - `h.close_day(&mut d.wb, printing)` bila `output`; `n.set_hru(j, &d.hd, &d.hhsurfq)`; simpan `hru_wb[j]`.
      - **Keluaran HRU** dalam urutan nomor: `hru_day` bila mencetak; bila `output`: `hru_acc[j].day(...)` dengan cuaca `[tmax, tmin, tave, solrad, windsp, rhum]`, `day_rows[j] = region_values(...)`. Pada `end_sim`: `crop_yld_aa` per tanaman (rata-rata per panen bila `harv_num > 0`; padanan penulisan di `hru_output.f90`).
      - `regions(...)` bila `output` dan ada fase lahan.
      - **Jaringan.** Bentuk `NetDay` (cuaca semua stasiun, `daylth` per stasiun dari WGN-nya, pengukur suhu per stasiun bila dari berkas, `hru_wb`, tanggal, `print = yrs > nyskip`) dan panggil `n.day(&nd)`. Bila `output`: lihat langkah 2.4.11 di 5.1.1.
      - Hari 181: `midyear` setiap HRU.
  13. **Akhir tahun.** Untuk setiap tanaman cekungan *k*: bila `yrs > nyskip`, kirim baris tahunan dengan `t_ha = yld/(area + 10⁻⁶)` lalu tambahkan ke `bsn_yld_aa`; nolkan `bsn_yld[k]` (selalu, juga di tahun pemanasan); pada tahun terakhir kirim baris rata-rata tahunan dengan `area/yrs_prt`, `yld/yrs_prt`, dan `t_ha` yang dihitung **sebelum** pembagian. Lalu `h.end_year(basin.cswat, opts.fixes)` dan `clock.yrc += 1`.
  14. `progress(days_done, total_days, …)` terakhir; `Ok(report)`.

Koreksi elevasi (padanan `hru_control`):

```latex
P_{hru} = \max\left(0,\; P_{st} + \Delta P\right), \qquad T_{max,hru} = T_{max,st} + \Delta T, \quad T_{min,hru} = T_{min,st} + \Delta T, \quad \bar T_{hru} = \bar T_{st} + \Delta T
```

- **Kebiasaan SWAT+ / catatan port:**
  - Hujan yang dikoreksi lapse dipotong pada 0 seperti Fortran, tetapi larik hujan subharian `ts` **tidak** dikoreksi (disalin dari stasiun apa adanya).
  - PET stasiun (`station_pet`) dihitung dari suhu stasiun yang belum dikoreksi lapse (docs `engine1.md`).
  - Angin minimum 0,01 m/s untuk Penman-Monteith diterapkan pada `w` yang juga dicetak, karena SWAT+ mencetak angin HRU setelah `et_pot` mengubahnya.
  - HRU dihitung dalam urutan perintah tetapi dicetak dan diakumulasi dalam urutan nomornya; ini meniru `command` (urutan objek) dan `hru_output` (loop `ihru = 1, sp_ob%hru`).
  - Indeks deposisi dimajukan setelah bendera akhir bulan/tahun hari ini ditetapkan, sehingga hari terakhir bulan atau tahun sudah memakai deposisi bulan/tahun berikutnya (komentar sumber; meniru `cli_atmodep_time_control` yang dipanggil setelah pemeriksaan `end_mo`/`end_yr`).
  - `basin_reservoir_output` menolkan `in_d` dan `out_d` reservoir setelah dicetak; port meniru ini hanya bila `output`. Ini sudah diperiksa dan setara: di `command.f90` semua pemanggilan keluaran, termasuk `reservoir_output` dan `basin_reservoir_output`, ada di dalam `if (time%yrs > pco%nyskip)`, dan `res_control` mengisi `res_in_d`/`res_out_d` hanya pada kondisi yang sama.
  - Baris rata-rata tahunan tanaman cekungan memakai `t_ha` dari jumlah sebelum dibagi `yrs_prt`, persis urutan `time_control`.
  - Pada `--fix-swat`, `opts.fixes` diteruskan ke `HruBasin`, `Network::new`, dan `Hru::end_year`; `run_with` sendiri tidak mengandung koreksi.

#### 5.3.3  `regions`

- **Lokasi:** `run.rs`, `fn regions(project, rows: &[Option<HruRows>], basin_on: bool, basin: &mut RegionAcc, lsus: &mut [RegionAcc], ru_gis: &[i32], bsn_name: &str, pday: &PeriodDay, printing: bool, day: Day, out: &mut dyn OutputSink)` (privat).
- **Padanan SWAT+:** `basin_output.f90` dan `lsu_output.f90`.
- **Tujuan:** neraca cekungan dan setiap unit lanskap (LSU) pada satu hari: hari HRU diberi bobot bagiannya dari cekungan (`bsn_frac`) dan dari unit lanskapnya (`ru_frac`), lalu dijumlah periode.
- **Masukan dan keluaran:** baris harian HRU (`day_rows`, dari `region_values`), elemen `ls_unit.ele` (`project.lsu_elems`), definisi `ls_unit.def` (`project.lsu_defs`). Keluaran lewat `out.region`.
- **Cara kerja:**
  1. **Cekungan** (bila `basin_on`): mulai dari baris kosong (`region_start`). Untuk `ihru = 1..n`: baca `iihru = lsu_elems[ihru].obtypno` (elemen ke-`ihru` menunjuk HRU mana), ambil elemen `iihru`; bila `bsn_frac > 10⁻¹²`, jenisnya `"hru"`, dan HRU `iihru` punya baris, tambahkan `region_add(r, h, bsn_frac)` serta jumlahkan `init = Σ (sw_init, sno_init)·bsn_frac`.
  2. `basin.start(init)` (hanya efektif pertama kali: padanan `basin_sw_init`). Bila mencetak, `daily_row` lalu `out.region(Basin, None, …)`. Lalu `basin.day(...)` mengirim periode yang berakhir.
  3. **Setiap LSU** `ilsu`: sama, tetapi elemen diambil dari daftar `def.elems` dan bobotnya `ru_frac` dengan ambang `10⁻⁹`. Id GIS dari unit routing ke-`ilsu`.

```latex
R_d = \sum_{i} f_i \cdot H_{i,d}, \qquad f_i = \texttt{bsn\_frac}_i \text{ (cekungan) atau } \texttt{ru\_frac}_i \text{ (LSU)}
```

- **Kebiasaan SWAT+ / catatan port:**
  - Cekungan membaca `obtypno` dari elemen ke-*i* untuk HRU ke-*i* lalu memakai bobot elemen yang ditunjuk itu, bukan elemen ke-*i* langsung; urutan penjumlahan mengikuti urutan itu (docs `outputs.md`).
  - Ambang bobot berbeda: `1e-12` untuk cekungan, `1e-9` untuk LSU, seperti di kedua rutin Fortran.
  - Jumlah dilakukan dalam `f32` dalam urutan elemen; mengubah urutan akan mengubah bit terakhir.

#### 5.3.4  `lapse_rates`

- **Lokasi:** `run.rs`, `fn lapse_rates(project: &Project, weather: &Weather, ist: usize, elev: f32) -> (f32, f32)` (privat).
- **Padanan SWAT+:** `cli_lapse.f90`.
- **Tujuan:** koreksi hujan dan suhu untuk sebuah objek pada elevasi `elev` (m) yang memakai stasiun `ist`.
- **Masukan dan keluaran:** `plaps` (mm/km) dan `tlaps` (°C/km) dari `parameters.bsn`; elevasi WGN stasiun atau elevasi pengukur hujan/suhu bila datanya dari berkas. Mengembalikan `(ΔP, ΔT)` dalam mm dan °C; `(0, 0)` bila stasiun tidak ada.
- **Cara kerja:**

```latex
\Delta P = \texttt{plaps}\cdot\frac{z_{obj} - z_{ref,P}}{1000}, \qquad \Delta T = \texttt{tlaps}\cdot\frac{z_{ref,T} - z_{obj}}{1000}
```

Di sini `z_obj` adalah elevasi objek, `z_ref,P` elevasi WGN bila hujan disimulasikan (`Source::Sim`) atau elevasi pengukur hujan (`Source::Gage(i)`), dan `z_ref,T` serupa untuk suhu.

- **Kebiasaan SWAT+ / catatan port:** tanda pada ΔT dibalik terhadap ΔP (referensi dikurangi objek), sama dengan Fortran; dengan `tlaps` positif, objek yang lebih tinggi menjadi lebih dingin. Koreksi dihitung sekali sebelum loop, hanya untuk HRU (objek lain tidak memerlukannya di port ini).

### 5.4  Jam simulasi (`clock.rs`)

`Clock` adalah padanan variabel global `time` (modul `time_module`) dan tabel global `ndays` SWAT+. Banyak rutin membaca `ndays` sebagai keadaan global, sehingga port menyimpannya di jam dan meneruskannya ke mana pun dibutuhkan.

| Field | Padanan | Arti |
| --- | --- | --- |
| `yrc` | `time%yrc` | tahun kalender |
| `yrs` | `time%yrs` | tahun simulasi ke-n (1 = pertama) |
| `nbyr` | `time%nbyr` | jumlah tahun disimulasikan |
| `day` | `time%day` | hari ke-n dalam tahun (Julian) |
| `mo`, `day_mo` | `time%mo`, `time%day_mo` | bulan dan tanggal |
| `day_start` | `time%day_start` | hari pertama loop tahun ini (1 setelah tahun pertama) |
| `day_end` | `time%day_end` | hari terakhir tahun terakhir seperti dibaca (0 = 31 Desember) |
| `day_end_yr` | `time%day_end_yr` | hari terakhir loop tahun ini |
| `step` | `time%step` | jumlah langkah waktu per hari |
| `end_yr` | `time%end_yr` | hari ini hari terakhir tahun |
| `ndays` | `ndays(1:13)` | hari kumulatif di akhir setiap bulan; `ndays[0] = 0`, `ndays[12] = 365` atau `366` |

Indeks Rust bergeser satu dari Fortran: `ndays[m]` Rust sama dengan `ndays(m+1)` Fortran, yaitu hari terakhir bulan ke-*m*.

#### 5.4.1  `Clock::new`

- **Lokasi:** `crates/swat-engine/src/clock.rs`, `pub fn new(time: &TimeSim) -> Self`.
- **Padanan SWAT+:** `time_read.f90` (pembacaan `time.sim`) dan nilai awal `ndays` di `time_module`.
- **Tujuan:** jam setelah `time.sim` dibaca: tahun awal, jumlah tahun, hari awal, bulan dan tanggal hari awal.
- **Cara kerja:**
  1. `t = time.normalized()` (normalisasi `step ≤ 0` → harian, `day_start ≤ 0` → 1; lihat docs `swat-quirks.md`, bagian `time.sim`).
  2. `ndays = NDAYS_LEAP` (tabel kabisat), lalu `(mo, day_mo) = xmon(&ndays, day_start)`.
  3. Field lain: `yrc = yrc_start`, `yrs = 1`, `nbyr = t.years()`, `day_end_yr = 0`, `end_yr = false`.
- **Kebiasaan SWAT+ / catatan port:** tabel `ndays` mula-mula selalu tabel tahun kabisat, apa pun tahunnya; pembaca cuaca kemudian dapat menggantinya (`set_ndays_mod4`) sebelum loop dimulai. Rutin yang membaca `ndays` sebelum loop (misalnya `cli_initwgn`) melihat tabel dalam keadaan itu (docs `swat-quirks.md`: “The month table is global”).

#### 5.4.2  `Clock::set_ndays_mod4`

- **Lokasi:** `clock.rs`, `pub fn set_ndays_mod4(&mut self, year: i32)`.
- **Padanan SWAT+:** pernyataan `if (mod(iyr,4) == 0) then ndays = ndays_leap else ndays = ndays_noleap` di pembaca berkas cuaca (dipanggil dari `climate.rs`, pembaca suhu).
- **Tujuan:** menyetel `ndays` dengan aturan kabisat sederhana “habis dibagi 4”.
- **Kebiasaan SWAT+ / catatan port:** hanya `time_control` yang memakai aturan abad (habis dibagi 100 bukan kabisat kecuali habis dibagi 400); pembaca memakai aturan mod 4 saja. Port mempertahankan kedua aturan di tempatnya masing-masing.

#### 5.4.3  `Clock::before_loop`

- **Lokasi:** `clock.rs`, `pub fn before_loop(&mut self)`.
- **Padanan SWAT+:** awal `time_control.f90` sebelum `do curyr`: `day_end_yr` dari `mod(yrc,4)`, `time%yrs = 1`, `time%day = time%day_start`, `call xmon`.
- **Tujuan:** keadaan yang dibutuhkan `cli_precip_control(0)` untuk membangkitkan hujan hari pertama.
- **Cara kerja:** `day_end_yr = 366` bila `yrc mod 4 = 0`, selain itu `365`; `yrs = 1`; `(mo, day_mo) = xmon(ndays, day)` dengan `ndays` dalam keadaan setelah pembaca.
- **Kebiasaan SWAT+ / catatan port:** di sini pun aturan mod 4, bukan Gregorian penuh.

#### 5.4.4  `Clock::start_year`

- **Lokasi:** `clock.rs`, `pub fn start_year(&mut self, curyr: i32)`.
- **Padanan SWAT+:** awal badan `do curyr` di `time_control.f90`.
- **Tujuan:** awal tahun simulasi `curyr` (berbasis 1): tabel bulan tahun itu, hari pertama dan terakhir loop.
- **Cara kerja:**
  1. `yrs = curyr`; `ndays` = tabel kabisat bila `is_leap(yrc)` (Gregorian: habis dibagi 4 dan bukan 100, atau habis dibagi 400), selain itu tabel biasa.
  2. Bila `yrs > 1`: `day_start = 1`.
  3. `day_end_yr = ndays[12]`; pada tahun terakhir dengan `day_end > 0`: `day_end_yr = min(day_end, ndays[12])` (padanan `amin0`, untuk pengguna yang mengisi 366 pada tahun bukan kabisat).
- **Kebiasaan SWAT+ / catatan port:** penghitungan `num_leap` yang di Fortran ada di blok yang sama dipindahkan ke `PrintYears::start_year` (5.7). Fortran juga menyetel `day_start = 1` bila `day_start == 0`; di port kasus itu sudah dinormalisasi oleh `TimeSim::normalized`.

#### 5.4.5  `Clock::set_day`

- **Lokasi:** `clock.rs`, `pub fn set_day(&mut self, jday: i32)`.
- **Padanan SWAT+:** awal badan `do julian_day`: `time%day = julian_day`, `call xmon`, pemeriksaan `time%day == time%day_end_yr`.
- **Tujuan:** pindah ke hari `jday` tahun berjalan.
- **Cara kerja:** `day = jday`; `(mo, day_mo) = xmon(ndays, jday)`; `end_yr = (day == day_end_yr)`.
- **Kebiasaan SWAT+ / catatan port:** `end_mo` dan `end_sim` tidak disimpan di jam; `run_with` menghitungnya ke dalam `PeriodDay`.

#### 5.4.6  `Clock::month_index` dan `xmon`

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `Clock::month_index(&self) -> usize` | `clock.rs` | Indeks bulan 0–11 untuk tabel bulanan (WGN, dll.): `clamp(mo, 1, 12) − 1`. Khusus port (Fortran memakai `time%mo` langsung sebagai indeks berbasis 1). |

#### 5.4.7  `xmon`

- **Lokasi:** `clock.rs`, `pub fn xmon(ndays: &[i32; 13], jd: i32) -> (i32, i32)`.
- **Padanan SWAT+:** `xmon.f90`.
- **Tujuan:** bulan dan tanggal dari hari ke-`jd` menurut tabel `ndays`.
- **Cara kerja:** untuk `mo = 1..12`, bulan pertama dengan `jd ≤ ndays[mo]` dipilih, `day_mo = jd − ndays[mo−1]`. Bila tidak ada (hari melewati tabel), kembalikan `(0, 0)`.

```latex
mo = \min\{ m \in 1..12 : jd \le N_m \}, \qquad day\_mo = jd - N_{mo-1}
```

- **Kebiasaan SWAT+ / catatan port:** Fortran tidak mengisi argumen keluarannya bila hari melewati tabel; pemanggil lalu melihat nilai lama variabelnya (docs `swat-quirks.md`: hari-hari setelah 31 Desember dalam `plant_init` memakai suhu Desember karena `xmon` membiarkan bulan seperti sebelumnya). Fungsi port mengembalikan `(0, 0)`; pemanggil yang perlu meniru “nilai lama” harus menanganinya sendiri.

### 5.5  Rutin kecil peniru gfortran dan SWAT+ (`fortran.rs`)

Modul ini berisi rutin yang dipakai di banyak tempat dan yang hasilnya harus identik bit demi bit dengan biner SWAT+ yang dikompilasi gfortran: pemangkatan real, pembangkit bilangan acak beserta distribusinya, dan pencarian nama. Alasan modul ini ada:

- **Presisi tunggal dan urutan operasi.** SWAT+ memakai `real` (32 bit). Rust dengan `f32` dan urutan operasi yang sama memberikan bit yang sama untuk operasi dasar (+, −, ×, ÷, `sqrt`), karena keduanya mengikuti IEEE 754 dengan pembulatan ke terdekat.
- **Pustaka matematika.** `exp`, `ln`, `powf`, `cos` tidak wajib dibulatkan benar oleh IEEE 754. Biner SWAT+ di Linux memanggil glibc; port native di Linux juga memanggil glibc lewat `f32::exp` dsb., sehingga hasilnya sama. Di WebAssembly, macOS, dan Windows pustaka matematikanya lain dan bit terakhir sebagian hasil bisa berbeda (docs `runner.md`).
- **Optimisasi kompiler.** Di sinilah `rpow` diperlukan: LLVM dapat mengganti `x.powf(0.5)` dengan `sqrt`, yang hasilnya tidak selalu sama dengan `powf` glibc.
- **Semantik bahasa Fortran.** Perbandingan string Fortran mengisi spasi sampai panjang deklarasi; pencarian biner SWAT+ punya batas 25 iterasi. Keduanya ditiru.

Modul ini tidak berisi rutin format angka; penulisan angka bergaya Fortran (format `G`, `E`, dll.) ada di crate keluaran dan dibahas di bab tersebut.

#### 5.5.1  `rpow`

- **Lokasi:** `crates/swat-engine/src/fortran.rs`, `pub fn rpow(x: f32, y: f32) -> f32`.
- **Padanan SWAT+:** operator Fortran `x ** y` dengan eksponen `real` (gfortran memanggil `powf` dari libm).
- **Tujuan:** pemangkatan real yang selalu memanggil `powf` pustaka, tidak pernah diganti `sqrt` oleh optimisasi.
- **Cara kerja:** `x.powf(std::hint::black_box(y))`. `black_box` menyembunyikan nilai eksponen dari optimizer sehingga panggilan pustaka dipertahankan.
- **Kebiasaan SWAT+ / catatan port:** `sqrt` dibulatkan benar, `powf` glibc tidak selalu; keduanya berbeda di bit terakhir untuk kira-kira 1 dari 1600 nilai. Persamaan Hargreaves memuat `x ** 0.5`; satu bit PET cukup untuk menggeser air satu lapisan tanah satu bit beberapa hari kemudian dan satu digit tercetak beberapa minggu kemudian (docs `swat-quirks.md`).

#### 5.5.2  `aunif`

- **Lokasi:** `fortran.rs`, `pub fn aunif(seed: &mut i32) -> f32`.
- **Padanan SWAT+:** `aunif.f90`.
- **Tujuan:** bilangan acak seragam di (0, 1) dengan generator *minimal standard* Park–Miller (pengali 16807, modulus 2³¹ − 1), dihitung dengan metode Schrage agar tidak meluap dalam bilangan bulat 32 bit. Benih dimajukan.
- **Cara kerja:**

```latex
q = \lfloor s / 127773 \rfloor, \quad s' = 16807\,(s - 127773\,q) - 2836\,q, \quad s' \leftarrow s' + 2147483647 \text{ bila } s' < 0, \quad u = \mathrm{f32}\!\left(s' \times 4.656612875\times10^{-10}\right)
```

- **Kebiasaan SWAT+ / catatan port:** perkalian dikerjakan dalam presisi ganda lalu dibulatkan ke tunggal, karena Fortran menulis `x1 * 4.656612875d-10` (konstanta `d`) dan menyimpannya ke `real`. Diuji: benih 1 menghasilkan 16807 lalu 282475249, dan nilai ke-10.000 adalah 1043618065 (Park dan Miller 1988).

#### 5.5.3  `dstn1`

- **Lokasi:** `fortran.rs`, `pub fn dstn1(rn1: f32, rn2: f32) -> f32`.
- **Padanan SWAT+:** `cli_dstn1.f90`.
- **Tujuan:** deviasi normal baku dari dua bilangan seragam (transformasi Box–Muller), dipakai generator cuaca.

```latex
z = \sqrt{-2\,\ln r_1}\;\cos(6.283185\, r_2)
```

- **Kebiasaan SWAT+ / catatan port:** konstanta `6.283185` (bukan 2π presisi penuh) dipertahankan; semua operasi `f32`.

#### 5.5.4  `atri`

- **Lokasi:** `fortran.rs`, `pub fn atri(at1: f32, at2: f32, at3: f32, seed: &mut i32) -> f32`.
- **Padanan SWAT+:** `atri.f90`.
- **Tujuan:** nilai dari distribusi segitiga dengan batas `at1`, `at3` dan modus `at2`, diskalakan ke rata-rata dan dibatasi dalam (0, 1). Dipakai antara lain untuk fraksi hujan setengah jam maksimum.
- **Cara kerja:**
  1. `u3 = at2 − at1`, `rn = aunif(seed)`, `y = 2/(at3 − at1)`, `b2 = at3 − at2`, `b1 = rn/y`, `x1 = y·u3/2`.
  2. Bila `rn ≤ x1`: `xx = 2·b1·u3`, hasil `= at1 + √xx` (√ diambil 0 bila `xx ≤ 0`).
  3. Selain itu: `xx = b2² − 2·b2·(b1 − 0.5·u3)`, hasil `= at3 − √xx`.
  4. Skala ke rata-rata: `amn = (at3 + at2 + at1)/3`, `atri = atri·at2/amn`.
  5. Batasi: `≥ 1` menjadi `0.99`; `≤ 0` menjadi `0.001`.
- **Kebiasaan SWAT+ / catatan port:** urutan operasi dan konstanta pembatas persis seperti Fortran. Penskalaan dengan `at2/amn` membuat hasil bukan lagi distribusi segitiga murni; ini perilaku SWAT+.

#### 5.5.5  `fortran_cmp`

- **Lokasi:** `fortran.rs`, `pub fn fortran_cmp(a: &str, b: &str) -> Ordering`.
- **Padanan SWAT+:** perbandingan intrinsik Fortran antara dua `character(len=50)`.
- **Tujuan:** membandingkan dua nama seperti Fortran: dipotong 50 karakter, sisi pendek diisi spasi.
- **Cara kerja:** untuk `i = 0..50`, ambil byte ke-*i* masing-masing (spasi bila lewat ujung) dan kembalikan hasil perbandingan pertama yang tidak sama; bila semuanya sama, `Equal`.
- **Kebiasaan SWAT+ / catatan port:** `"abc"` sama dengan `"abc   "`; spasi (32) lebih kecil daripada `!` (33), sehingga `"ab" < "ab!"`. Perbandingan per byte (ASCII), bukan Unicode.

#### 5.5.6  `search`

- **Lokasi:** `fortran.rs`, `pub fn search(list: &[String], find: &str) -> Option<usize>`.
- **Padanan SWAT+:** `search.f90`.
- **Tujuan:** pencarian biner nama di daftar; mengembalikan posisi berbasis 1, `None` bila tidak ditemukan (SWAT+ lalu membiarkan nilai pemanggil seperti sebelumnya).
- **Cara kerja:**
  1. `nf = 1`, `nl = len`. Ulangi paling banyak 25 kali:
  2. `nn = (nl − nf)/2 + nf` (pembagian bulat).
  3. Bila `nl − nf = 1`: periksa `list[nl]` lalu `list[nf]`; bila sama, kembalikan.
  4. Bila `list[nn] = find`: kembalikan `nn`.
  5. Bila `list[nn] > find`: `nl = nn`; bila `list[nn] < find`: `nf = nn`.
- **Kebiasaan SWAT+ / catatan port:** fungsi ini menganggap daftar terurut. Pada daftar tak terurut, nama yang ada bisa tidak ditemukan (pengujian: di `["b","c","a"]`, `"a"` tidak ditemukan), dan nama bisa ditemukan hanya bila probe pertama kebetulan mengenainya. Dalam SWAT+ ini membuat stasiun yang berkas pengukurnya tidak ditemukan beralih ke data simulasi (docs `swat-quirks.md`: “Station files are found by binary search”; contoh Ames\_sub1 yang stasiunnya menyebut `ames.tem` sedangkan daftar berisi `ames.tmp`). Batas 25 iterasi cukup untuk daftar sampai sekitar 2²⁵ nama (komentar Fortran: “max for 100,000,000”).

#### 5.5.7  Pengujian dalam `fortran.rs`

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `park_miller` | `fortran.rs`, `mod tests` | Memeriksa urutan benih Park–Miller (16807, 282475249, nilai ke-10.000 = 1043618065). |
| `binary_search_quirks` | `fortran.rs`, `mod tests` | Daftar terurut ditemukan semua; nama tak ada → `None`; kasus Ames\_sub1; daftar tak terurut yang ditemukan dan yang terlewat. |
| `padded_comparison` | `fortran.rs`, `mod tests` | Kesetaraan dengan spasi pengisi dan urutan spasi < `!`. |
| `atri_stays_in_bounds` | `fortran.rs`, `mod tests` | 1000 nilai `atri(0.2, 0.6, 0.9)` semuanya dalam (0, 1). |

### 5.6  Hidrograf (`hyd.rs`)

`Hyd` adalah padanan tipe `hyd_output` di `hydrograph_module.f90`: air yang diteruskan sebuah objek dalam satu hari beserta muatannya. Semua field `f32`.

| Field | Satuan | Arti |
| --- | --- | --- |
| `flo` | m³ | volume air |
| `sed` | t | sedimen |
| `orgn`, `sedp`, `no3`, `solp`, `nh3`, `no2` | kg | N organik, P terikat sedimen, nitrat, P terlarut, amonium, nitrit |
| `chla`, `cbod`, `dox` | kg | klorofil-a, CBOD, oksigen terlarut |
| `san`, `sil`, `cla`, `sag`, `lag`, `grv` | t | kelas sedimen: pasir, debu, liat, agregat kecil, agregat besar, kerikil |
| `temp` | °C | suhu air |

Operator Fortran (`interface operator`) dipetakan ke method Rust. Ke-17 jumlah (semua kecuali `temp`) selalu diperlakukan sama; perlakuan `temp` berbeda per operator, mengikuti Fortran.

| Fungsi | Lokasi | Padanan SWAT+ | Kegunaan |
| --- | --- | --- | --- |
| `Hyd::amounts(&self) -> [f32; 17]` | `hyd.rs` (privat) | tidak ada; khusus port | 17 jumlah dalam urutan berkas, tanpa suhu. |
| `Hyd::from_amounts(a, temp) -> Hyd` | `hyd.rs` (privat) | tidak ada; khusus port | Kebalikan `amounts`, dengan suhu diberikan. |

#### 5.6.1  `Hyd::plus`

- **Lokasi:** `hyd.rs`, `pub fn plus(self, o: Hyd) -> Hyd`.
- **Padanan SWAT+:** `hydout_add` (operator `+`), `hydrograph_module.f90`.
- **Tujuan:** menjumlahkan dua hidrograf; suhu menjadi rata-rata berbobot aliran.

```latex
h_3[k] = h_1[k] + h_2[k], \qquad T_3 = \begin{cases} \dfrac{Q_1 T_1 + Q_2 T_2}{Q_1 + Q_2} & Q_1 + Q_2 > 10^{-6} \\ 0 & \text{lainnya} \end{cases}
```

- **Kebiasaan SWAT+ / catatan port:** suhu menjadi 0 (bukan rata-rata biasa) bila aliran gabungan ≤ 10⁻⁶ m³. Karena akumulator periode memakai `plus`, suhu dalam jumlah bulanan adalah rata-rata berbobot aliran, bukan rata-rata harian. Pengujian `temperature_mixes_by_flow` memeriksa (1 m³, 10 °C) + (3 m³, 20 °C) = 17,5 °C.

#### 5.6.2  `Hyd::minus`

- **Lokasi:** `hyd.rs`, `pub fn minus(self, o: Hyd) -> Hyd`.
- **Padanan SWAT+:** `hydout_subtract` (operator `-`).
- **Tujuan:** selisih jumlah demi jumlah; suhu diambil dari operand pertama.

#### 5.6.3  `Hyd::scale`

- **Lokasi:** `hyd.rs`, `pub fn scale(k: f32, h: Hyd) -> Hyd`.
- **Padanan SWAT+:** `hydout_mult_const` (operator `*` dengan konstanta di kiri).
- **Tujuan:** `k · h` untuk setiap jumlah; suhu tetap. Dipakai untuk fraksi aliran masuk (`frac_in * ob(iob)%hd(ihyd)` di `command`).
- **Kebiasaan SWAT+ / catatan port:** urutan operand dipertahankan (`k * a[i]`); untuk perkalian IEEE urutan ini tidak mengubah hasil, tetapi dipertahankan demi keterbacaan padanan.

#### 5.6.4  `Hyd::times`

- **Lokasi:** `hyd.rs`, `pub fn times(self, o: Hyd) -> Hyd`.
- **Padanan SWAT+:** `hydout_mult` (operator `**` didefinisikan ulang untuk hidrograf).
- **Tujuan:** perkalian jumlah demi jumlah (misalnya hidrograf dengan hidrograf rasio pengantaran); suhu dari operand pertama.

#### 5.6.5  `Hyd::over`

- **Lokasi:** `hyd.rs`, `pub fn over(self, k: f32) -> Hyd`.
- **Padanan SWAT+:** `hydout_div_const` (operator `/` dengan konstanta).
- **Tujuan:** `h / k` untuk setiap jumlah; suhu tetap. Dipakai untuk rata-rata tahunan (`/ yrs_prt`).
- **Kebiasaan SWAT+ / catatan port:** pembagian, bukan perkalian dengan `1/k`, karena dalam `f32` keduanya dapat berbeda di bit terakhir.

#### 5.6.6  `Hyd::add_const`

- **Lokasi:** `hyd.rs`, `pub fn add_const(k: f32, h: Hyd) -> Hyd`.
- **Padanan SWAT+:** `hydout_add_const` (operator `.add.`).
- **Tujuan:** `k + h[i]` untuk setiap jumlah; suhu tetap.

#### 5.6.7  `Hyd::columns`

- **Lokasi:** `hyd.rs`, `pub fn columns(&self) -> [(&'static str, f32); 18]`.
- **Padanan SWAT+:** tajuk `hyd_hdr` di `hydrograph_module.f90`.
- **Tujuan:** pasangan (nama kolom, nilai) untuk tabel hidrograf: 17 jumlah dengan nama `flo`…`grv`, lalu suhu.
- **Kebiasaan SWAT+ / catatan port:** kolom suhu bertajuk `null`, seperti tajuk SWAT+.

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `temperature_mixes_by_flow` | `hyd.rs`, `mod tests` | Uji `plus` (pencampuran suhu, suhu 0 untuk aliran nol) dan `scale` (suhu tetap). |

### 5.7  Akumulasi periode, bagian 1: aturan kolom, tahun tercetak, dan HRU (`periods.rs`)

`periods.rs` menjumlahkan nilai harian menjadi baris bulanan (`mon`), tahunan (`yr`), dan rata-rata tahunan (`aa`), dalam urutan operasi rutin keluaran SWAT+ (`hru_output`, `ru_output`, `aquifer_output`, `sd_channel_output`, `reservoir_output`, `recall_output`, `basin_output`, `lsu_output`, dan versi cekungannya). Karena semuanya `f32`, urutan penjumlahan dan pembagian adalah hasilnya; fungsi-fungsi ini tidak boleh “dirapikan”.

SWAT+ mendefinisikan tiga operator pada tipe neraca HRU (`output_landscape_module`): `+` menjumlah hari dalam periode, `/` membagi dengan jumlah tahun tercetak, dan `//` membagi dengan jumlah hari. Setiap kolom tunduk pada salah satu dari tiga aturan, yang di port dikodekan sebagai enum privat `Rule`:

| Aturan | `+` | `/` (tahun) | `//` (hari) | Contoh kolom |
| --- | --- | --- | --- | --- |
| `Sum` (fluks) | dijumlah | dibagi | tidak | `precip`, `surq_gen`, `et`, semua kolom hara dan kehilangan |
| `Ave` (keadaan) | dijumlah | tidak | dibagi | `cn`, `sw_ave`, `sw_300`, `snopack`, `wet_stor`, `lai`, `bioms`, suhu, cuaca |
| `Keep` (awal/akhir) | operand pertama dipertahankan | menjadi 0 | dipertahankan | `sw_init`, `sw_final`, `sno_init`, `sno_final` |

Konstanta aturan per tabel: `WB` (32 kolom `hru_wb` dalam urutan `HruWb::columns`: `precip`, `snofall`, `snomlt`, `surq_gen`, `latq`, `wateryld`, `perc`, `et`, `ecanopy`, `eplant`, `esoil`, `surq_cont`, `cn`, `sw_init`, `sw_final`, `sw_ave`, `sw_300`, `sno_init`, `sno_final`, `snopack`, `pet`, `qtile`, `irr`, `satex`, `satex_chan`, `sw_change`, `lagsurf`, `laglatq`, `lagsatex`, `wet_evap`, `wet_oflo`, `wet_stor`), `PW` (25 kolom `hru_pw`, daftar namanya publik sebagai `PW_NAMES`: `lai`, `bioms`, `yield`, `residue`, `sol_tmp`, `strsw`, `strsa`, `strstmp`, `strsn`, `strsp`, `strss`, `nplt`, `percn`, `pplnt`, `tmx`, `tmn`, `tmpav`, `solarad`, `wndspd`, `rhum`, `phubas0`, `lai_max`, `bm_max`, `bm_grow`, `c_gro`), serta `ALL_SUM_15` (neraca hara `nb`) dan `ALL_SUM_13` (kehilangan `ls`), yang semua kolomnya `Sum`. Indeks bernama: `WB_SW_INIT = 13`, `WB_SW_FINAL = 14`, `WB_SW = 15`, `WB_SNO_INIT = 17`, `WB_SNO_FINAL = 18`, `WB_SNOPACK = 19`, `PW_BIOMS = 1`, `PW_NPLT = 11`, `PW_PPLNT = 13`, `PW_BMMAX = 22`.

Struct publik:

| Tipe | Field | Arti |
| --- | --- | --- |
| `Period` | `Mon`, `Yr`, `Aa` | periode baris |
| `HruRows` | `wb [32]`, `nb [15]`, `ls [13]`, `pw [25]` | satu baris periode (atau hari) HRU/wilayah; satuan mengikuti tabel SWAT+ (mm, kg/ha, t/ha, °C, …) |
| `PeriodDay` | `end_mo`, `end_yr`, `end_sim` (bool); `mon_days`, `day_end_yr`, `yrs_prt`, `days_prt` (f32) | fakta kalender hari ini yang dibutuhkan akumulator; `mon_days = ndays(mo+1) − ndays(mo)` |
| `PrintYears` | `yrs_prt`, `days_prt` (f32), `num_leap` (i32) | padanan `time%yrs_prt`, `time%days_prt`, `time%num_leap` |
| `HruAcc` | `wb`, `nb`, `ls`, `pw` masing-masing `[…; 3]` (indeks 0 = bulan, 1 = tahun, 2 = rata-rata tahunan), `started` | padanan `hwb_m/y/a`, `hnb_m/y/a`, `hls_m/y/a`, `hpw_m/y/a` |

#### 5.7.1  Fungsi kecil

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `Period::suffix(self) -> &'static str` | `periods.rs` | Akhiran nama berkas SWAT+: `"mon"`, `"yr"`, `"aa"`. |
| `HruAcc::default()` | `periods.rs`, `impl Default` | Semua jumlah nol, `started = false`. `Ditulis manual (alasannya tidak dinyatakan di sumber).` |

#### 5.7.2  `add`

- **Lokasi:** `crates/swat-engine/src/periods.rs`, `fn add(acc: &mut [f32], x: &[f32], rules: &[Rule])` (privat).
- **Padanan SWAT+:** operator `+` pada tipe keluaran lahan (`hruout_waterbal_add`, `hruout_nutbal_add`, `hruout_losses_add`, `hruout_plantweather_add` di `output_landscape_module.f90`).
- **Tujuan:** `acc = acc + x` kolom demi kolom.
- **Cara kerja:** kolom `Sum` dan `Ave`: `acc[k] += x[k]`; kolom `Keep`: tidak berubah (nilai operand pertama dipertahankan).

#### 5.7.3  `add_pw`

- **Lokasi:** `periods.rs`, `fn add_pw(acc: &mut [f32; 25], x: &[f32; 25])` (privat).
- **Padanan SWAT+:** `hruout_plantweather_add`.
- **Tujuan:** `add` untuk tabel tanaman dan cuaca, lalu `acc[pplnt] = 0`.
- **Kebiasaan SWAT+ / catatan port:** `hruout_plantweather_add` tidak menyertakan P tanaman, sehingga jumlahnya selalu 0. Pada tabel HRU nilai ini ditimpa dengan nilai hari terakhir sebelum dicetak; pada cekungan tetap 0 (docs `outputs.md`).

#### 5.7.4  `div`

- **Lokasi:** `periods.rs`, `fn div(acc: &mut [f32], c: f32, rules: &[Rule])` (privat).
- **Padanan SWAT+:** operator `/` (`hruout_waterbal_div`, dsb.).
- **Tujuan:** membagi fluks dengan jumlah tahun tercetak.
- **Cara kerja:** `Sum`: `acc[k] /= c`; `Keep`: `acc[k] = 0`; `Ave`: tidak berubah.

#### 5.7.5  `ave`

- **Lokasi:** `periods.rs`, `fn ave(acc: &mut [f32], c: f32, rules: &[Rule])` (privat).
- **Padanan SWAT+:** operator `//` (`hruout_waterbal_ave`, dsb.).
- **Tujuan:** membagi kolom keadaan dengan jumlah hari.
- **Cara kerja:** hanya kolom `Ave`: `acc[k] /= c`.

#### 5.7.6  `hru_values`

- **Lokasi:** `periods.rs`, `pub fn hru_values(d: &HruDay, weather: [f32; 6]) -> ([f32; 32], [f32; 15], [f32; 13], [f32; 25])`.
- **Padanan SWAT+:** penyusunan `hwb_d`, `hnb_d`, `hls_d`, `hpw_d` di `hru_output.f90` (dengan cuaca dari `output_plantweather`).
- **Tujuan:** mengubah satu hari HRU menjadi empat larik kolom.
- **Masukan dan keluaran:** `d.wb`, `d.nb`, `d.ls`, `d.pw` (masing-masing dengan `columns()`), dan cuaca `[tmax, tmin, tave, solrad, windsp, rhum]` (°C, MJ/m², m/s, fraksi).
- **Cara kerja:**
  1. `wb`, `nb`, `ls` langsung dari `columns()`.
  2. `pw`: 14 kolom tanaman pertama, lalu 6 kolom cuaca (posisi 14–19), lalu 5 kolom sisanya (`phubas0`, `lai_max`, `bm_max`, `bm_grow`, `c_gro`).
  3. `pw[bm_max] = pw[bioms]`: harian, biomassa maksimum adalah biomassa hari itu.

#### 5.7.7  `PrintYears::start_year`

- **Lokasi:** `periods.rs`, `pub fn start_year(&mut self, yrs, nyskip, nbyr, leap: bool, day_start, day_end_yr)`.
- **Padanan SWAT+:** awal tahun di `time_control.f90` (penghitungan `num_leap` dan blok “sum years of printing”).
- **Tujuan:** memperbarui jumlah hari tercetak.
- **Cara kerja:**
  1. Bila tahun kabisat: `num_leap += 1`.
  2. Bila `yrs > nyskip`: `d = day_end_yr − day_start + 1`; `yrs_prt += d`, `days_prt += d`.
  3. Selain itu (tahun pemanasan): `yrs_prt = nbyr`.
- **Kebiasaan SWAT+ / catatan port:** pada tahun pemanasan `yrs_prt` disetel ke jumlah tahun simulasi (komentar Fortran: “tell user they are skipping more years than simulating”), lalu tahun tercetak *menambahkan hari* ke nilai itu. Jadi dengan `nyskip > 0`, `yrs_prt` akhirnya = `nbyr` + hari tercetak (docs `outputs.md`), sehingga setelah dibagi 365 hasilnya sedikit lebih besar dari jumlah tahun tercetak. Ditiru apa adanya; `num_leap` dihitung dengan aturan Gregorian seperti blok yang sama di Fortran.

#### 5.7.8  `PrintYears::end_sim`

- **Lokasi:** `periods.rs`, `pub fn end_sim(&mut self, nbyr: i32)`.
- **Padanan SWAT+:** `time%yrs_prt = time%yrs_prt / (365. + (time%num_leap / time%nbyr))` di `time_control.f90` pada hari `end_sim`.
- **Tujuan:** mengubah hari tercetak menjadi tahun.

```latex
\texttt{yrs\_prt} \leftarrow \frac{\texttt{yrs\_prt}}{365 + \lfloor \texttt{num\_leap} / \texttt{nbyr} \rfloor}
```

- **Kebiasaan SWAT+ / catatan port:** `num_leap / nbyr` adalah pembagian bilangan bulat, jadi bernilai 0 kecuali semua tahun kabisat; penyebutnya praktis selalu 365. Dipanggil di awal hari terakhir, sebelum akumulator memakai `yrs_prt`.

#### 5.7.9  `HruAcc::day`

- **Lokasi:** `periods.rs`, `pub fn day(&mut self, d: &HruDay, weather: [f32; 6], c: &PeriodDay, emit: impl FnMut(Period, &HruRows))`.
- **Padanan SWAT+:** bagian periode `hru_output.f90`, ditambah `basin_sw_init` (nilai awal) dan reset tahunan di `time_control`.
- **Tujuan:** menambahkan satu hari HRU tercetak ke jumlah bulan; pada akhir bulan, tahun, dan simulasi menggulirkan jumlah ke periode berikutnya dan memanggil `emit` dengan baris periode yang selesai.
- **Masukan dan keluaran:** hari HRU, cuaca, `PeriodDay`. Keluaran lewat `emit(Period, &HruRows)` (di `run_with` diteruskan ke `out.hru_period`).
- **Cara kerja:**
  1. `(wb, nb, ls, pw) = hru_values(d, weather)`; lalu `wb[sw_ave] = wb[sw_final]` dan `wb[snopack] = wb[sno_final]`: jumlah memakai air tanah dan salju **akhir hari**, bukan rata-rata harian.
  2. Pertama kali saja (`started`): isi `sw_init` dan `sno_init` ketiga periode dengan nilai hari ini (padanan `basin_sw_init`).
  3. Simpan `bioms` hari ini, `bm_max` bulan, dan `bm_max` rata-rata tahunan sebelum diubah.
  4. `+` ke jumlah bulan (`wb`, `nb`, `ls`, `pw` lewat `add_pw`); kembalikan `bm_max` bulan ke nilai sebelum ditambah, lalu `bm_max` bulan, tahun, dan rata-rata tahunan masing-masing = `max(bioms, bm_max)`.
  5. **Akhir bulan:** tambahkan jumlah bulan ke tahun (`bm_max` tahun dipertahankan); `//` jumlah bulan dengan `mon_days` (kolom keadaan menjadi rata-rata); kembalikan `bm_max` bulan; `sw_final` dan `sno_final` = nilai hari ini; `emit(Mon)` dengan `nplt` dan `pplnt` diganti nilai hari ini. Lalu nolkan bulan, kecuali `sw_init`/`sno_init` bulan berikutnya = `sw_final`/`sno_final` bulan ini.
  6. **Akhir tahun:** tambahkan jumlah tahun ke rata-rata tahunan; `//` tahun dengan `day_end_yr`; kembalikan `bm_max` tahun; **kembalikan `bm_max` rata-rata tahunan ke nilai sebelum hari ini**; `sw_final`/`sno_final` = hari ini; `emit(Yr)`. Nolkan tahun dengan awal = akhir tahun ini.
  7. **Akhir simulasi:** salin jumlah rata-rata tahunan, `/` dengan `yrs_prt` lalu `//` dengan `days_prt` (`wb` dan `pw`; `nb` dan `ls` hanya `/`); `sw_init`/`sno_init` = awal periode tercetak pertama, `sw_final`/`sno_final` = hari ini; `bm_max` = nilai rata-rata tahunan sebelum hari ini; `emit(Aa)`.

```latex
\bar X_{mon} = \frac{1}{N_{mo}} \sum_{d \in mo} X_d \;(\text{keadaan}), \qquad X_{aa} = \frac{1}{Y_{prt}} \sum_{d} X_d \;(\text{fluks}), \qquad \bar X_{aa} = \frac{1}{D_{prt}} \sum_{d} X_d \;(\text{keadaan})
```

- **Kebiasaan SWAT+ / catatan port:**
  - Jumlah bulanan keadaan dibagi **jumlah hari bulan itu**, juga untuk bulan yang hanya sebagian disimulasikan; tahunan dibagi `day_end_yr` (bukan jumlah hari yang benar-benar dijalankan bila `day_start > 1`).
  - Air tanah dan salju bulanan adalah rata-rata nilai akhir hari, sedangkan tabel harian mencetak rata-rata awal dan akhir hari (docs `outputs.md`).
  - `bm_max` rata-rata tahunan tidak memperhitungkan hari terakhir setiap tahun (dikembalikan ke nilai hari sebelumnya), dan `bm_max` tidak dibagi jumlah hari pada akhir simulasi walaupun aturannya `Ave`.
  - `nplt` dan `pplnt` setiap periode HRU adalah nilai hari terakhir periode, bukan jumlah atau rata-rata.
  - Hanya dipanggil pada hari `output` (bukan tahun pemanasan), sesuai `hru_output` yang hanya menjumlah setelah `nyskip`.

### 5.8  Akumulasi periode, bagian 2: unit routing, akuifer, kanal, wilayah, reservoir, recall (`periods.rs`)

Semua akumulator di bagian ini punya pola yang sama dengan `HruAcc::day`: jumlah bulan (`m`), tahun (`y`), dan rata-rata tahunan (`a`); pada `end_mo` bulan digulir ke tahun dan dicetak, pada `end_yr` tahun digulir ke rata-rata tahunan dan dicetak, pada `end_sim` rata-rata tahunan dibagi dan dicetak. Perbedaannya ada pada **kolom mana yang dirata-rata, dengan pembagi apa, dan kapan** (sebelum atau sesudah digulir). Perbedaan kecil itu berasal dari rutin Fortran yang berbeda-beda dan dipertahankan.

| Struct | Field | Padanan SWAT+ |
| --- | --- | --- |
| `RuAcc` | `m`, `y`, `a`: `Hyd` | `ru_m`, `ru_y`, `ru_a` (`ru_output.f90`) |
| `AquAcc` | `m`, `y`, `a`: `AquDay` | `aqu_m`, `aqu_y`, `aqu_a` (`aquifer_output.f90`); juga dipakai untuk akuifer cekungan |
| `ChaAcc` | `hin`, `hout`: `[Hyd; 3]`; `wat`: `[ChWat; 3]` | `ch_in_m/y/a`, `ch_out_*`, `ch_wat_*` (`sd_channel_output.f90`) |
| `RegionAcc` | `d_init: [f32; 2]` (publik), `wb`, `nb`, `ls`, `pw` `[…; 3]`, `point`, `started` | `bwb_*`, `bnb_*`, `bls_*`, `bpw_*` (`basin_output.f90`) dan `ruwb_*` dst. (`lsu_output.f90`) |
| `BasinChaAcc` | seperti `ChaAcc` | `basin_sdchannel_output.f90` |
| `ResAcc` | seperti `ChaAcc` | `reservoir_output.f90`, `basin_reservoir_output.f90` |
| `RecAcc` | `h: [Hyd; 3]` | `recall_output.f90`, `basin_recall_output.f90` |

`ChWat` (badan air kanal/reservoir) berisi `area_ha` (ha), `precip`, `evap`, `seep` (m³); `AquDay` dijelaskan di bab akuifer.

#### 5.8.1  `RuAcc::day`

- **Lokasi:** `periods.rs`, `pub fn day(&mut self, d: &Hyd, c: &PeriodDay, emit: impl FnMut(Period, &Hyd))`.
- **Padanan SWAT+:** bagian periode `ru_output.f90`.
- **Tujuan:** jumlah periode hidrograf yang diteruskan sebuah unit routing.
- **Cara kerja:** `m = m + d` (`hydout_add`); akhir bulan: `y = y + m`, `emit(Mon, m)`, `m = 0`; akhir tahun: `a = a + y`, `emit(Yr, y)`, `y = 0`; akhir simulasi: `emit(Aa, a / yrs_prt)`.
- **Kebiasaan SWAT+ / catatan port:** tidak ada perataan harian; karena `plus` merata-rata suhu berbobot aliran, kolom suhu periode adalah suhu campuran.

#### 5.8.2  `aqu_add`

- **Lokasi:** `periods.rs`, `fn aqu_add(a: &mut AquDay, b: &AquDay)` (privat).
- **Padanan SWAT+:** operator `+` untuk `aquifer_dynamic` (`aqu_add` di `aquifer_module.f90`).
- **Tujuan:** menjumlah 16 field; `cbn` (karbon) **diganti** dengan nilai `b`, bukan dijumlah.

#### 5.8.3  `aqu_div`

- **Lokasi:** `periods.rs`, `fn aqu_div(a: &AquDay, c: f32) -> AquDay` (privat).
- **Padanan SWAT+:** operator `/` untuk akuifer (`aqu_div`).
- **Tujuan:** membagi semua field dengan `c` kecuali `cbn`.

#### 5.8.4  `AquAcc::day`

- **Lokasi:** `periods.rs`, `pub fn day(&mut self, d: &AquDay, c: &PeriodDay, emit: impl FnMut(Period, &AquDay))`.
- **Padanan SWAT+:** bagian periode `aquifer_output.f90` (dan `basin_aquifer_output.f90` untuk cekungan).
- **Tujuan:** jumlah periode satu akuifer.
- **Cara kerja:**
  1. `m = m + d` (`aqu_add`).
  2. Akhir bulan: `stor`, `dep_wt`, `no3_st` dibagi `mon_days`; `y = y + m`; `emit(Mon, m)`; `m = 0`.
  3. Akhir tahun: `stor`, `dep_wt`, `no3_st` tahun dibagi **12** (rata-rata dari rata-rata bulanan); `a = a + y`; `emit(Yr, y)`; `y = 0`.
  4. Akhir simulasi: `emit(Aa, aqu_div(a, yrs_prt))`.
- **Kebiasaan SWAT+ / catatan port:** simpanan tahunan adalah rata-rata 12 rata-rata bulanan (bulan pendek berbobot sama dengan bulan panjang; bila simulasi dimulai di tengah tahun pembaginya tetap 12). Karbon adalah nilai hari terakhir (docs `outputs.md`).

#### 5.8.5  `wat_add`

- **Lokasi:** `periods.rs`, `fn wat_add(a: &mut ChWat, b: &ChWat)` (privat).
- **Padanan SWAT+:** operator `+` untuk `water_body` (`watbod_add` di `water_body_module.f90`).
- **Tujuan:** menjumlah `area_ha`, `precip`, `evap`, `seep`.

#### 5.8.6  `ChaAcc::day`

- **Lokasi:** `periods.rs`, `pub fn day(&mut self, w: &ChWat, hin: &Hyd, hout: &Hyd, c: &PeriodDay, emit: impl FnMut(Period, &ChWat, &Hyd, &Hyd))`.
- **Padanan SWAT+:** bagian periode `sd_channel_output.f90`.
- **Tujuan:** jumlah periode satu kanal SWAT-DEG.
- **Cara kerja:**
  1. Bulan: `hin_m += hin`, `hout_m += hout` (`plus`), `wat_m += w`.
  2. Akhir bulan: tambahkan bulan ke tahun (**sebelum** perataan); lalu `hin_m.flo`, `hout_m.flo`, dan `wat_m.area_ha` dibagi `mon_days`; `emit(Mon)`; nolkan bulan.
  3. Akhir tahun: `hin_y.flo`, `hout_y.flo` dibagi `day_end_yr`; **lalu** tambahkan tahun ke rata-rata tahunan; tambahkan `wat_y` ke `wat_a`; `wat_y.area_ha /= day_end_yr`; `emit(Yr)`; nolkan tahun.
  4. Akhir simulasi: `k = yrs_prt`; `area_ha / k / k`, `precip / k`, `evap / k`, `seep / k`; `hin_a / k`, `hout_a / k`; `emit(Aa)`.
- **Kebiasaan SWAT+ / catatan port:**
  - Hanya **aliran** (`flo`) yang dirata-rata per hari, bukan muatannya (docs `outputs.md`).
  - Aliran rata-rata tahunan adalah jumlah dari aliran tahunan yang **sudah** dirata-rata harian, dibagi `yrs_prt`; sedangkan luas air tahunan dijumlahkan ke rata-rata tahunan **sebelum** dirata-rata. Akibatnya luas air rata-rata tahunan dibagi tahun dua kali (`/ k / k`), seperti SWAT+.
  - Simpanan dan suhu air yang dicetak adalah nilai hari terakhir (diberikan pemanggil, tidak diakumulasi).

#### 5.8.7  `region_values`

- **Lokasi:** `periods.rs`, `pub fn region_values(d: &HruDay, weather: [f32; 6]) -> HruRows`.
- **Padanan SWAT+:** nilai harian HRU (`hwb_d` dst.) seperti dibaca `basin_output`/`lsu_output` setelah `hru_output` berjalan.
- **Tujuan:** baris harian HRU untuk wilayah: `hru_values` apa adanya (air tanah dan salju sebagai rata-rata hari, biomassa sebagai maksimum), dibungkus `HruRows`.
- **Kebiasaan SWAT+ / catatan port:** berbeda dengan `HruAcc::day`, di sini `sw_ave` dan `snopack` **tidak** diganti nilai akhir hari, sehingga wilayah menjumlahkan rata-rata harian (docs `outputs.md`).

#### 5.8.8  `region_add`

- **Lokasi:** `periods.rs`, `pub fn region_add(r: &mut HruRows, h: &HruRows, c: f32)`.
- **Padanan SWAT+:** `bwb_d = bwb_d + hwb_d(ihru) * frac` dan seterusnya di `basin_output.f90`/`lsu_output.f90` (operator `*` lalu `+`; `hruout_waterbal_mult`, `hruout_nutbal_mult`, dsb.).
- **Tujuan:** menambahkan satu HRU berbobot `c` ke baris wilayah.
- **Cara kerja:**
  1. `wb`: semua kolom kecuali `Keep`: `r += h·c`; lalu `sw_final` dan `sno_final` juga `r += h·c` (kolom akhir dijumlah tersendiri).
  2. `nb`: `r += h·c`, kecuali kolom 9 dan 10 (`rsd_nitorg_n`, `rsd_laborg_p`, N dan P organik dari residu) yang dianggap 0 bila `h < 10⁻¹⁰`.
  3. `ls` dan `pw`: `r += h·c`; lalu `pplnt = 0`.
- **Kebiasaan SWAT+ / catatan port:** ambang `10⁻¹⁰` berasal dari `hruout_nutbal_mult`; perkalian dilakukan sebelum penjumlahan (`h·c` lalu `+`), sesuai dua operator Fortran yang terpisah.

#### 5.8.9  `region_start`

- **Lokasi:** `periods.rs`, `pub fn region_start(prev: &HruRows) -> HruRows`.
- **Padanan SWAT+:** `bwb_d = bwbz` (nol) di awal `basin_output`, dengan nilai awal dipertahankan.
- **Tujuan:** baris wilayah kosong yang hanya membawa `sw_init` dan `sno_init` dari `prev`. Di `regions` dipanggil dengan baris nol.

#### 5.8.10  `RegionAcc::new`

- **Lokasi:** `periods.rs`, `pub fn new(point: bool) -> Self`.
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** akumulator wilayah kosong. `point = true` untuk unit lanskap (N dan P tanaman periode = nilai hari terakhir), `false` untuk cekungan (rata-rata).

#### 5.8.11  `RegionAcc::start`

- **Lokasi:** `periods.rs`, `pub fn start(&mut self, init: [f32; 2])`.
- **Padanan SWAT+:** `basin_sw_init.f90` untuk cekungan dan LSU.
- **Tujuan:** pada pemanggilan pertama saja, menyetel air tanah dan salju awal wilayah (`init`, jumlah berbobot `sw_init`/`sno_init` HRU) ke `d_init` dan ketiga periode. Pemanggilan berikutnya diabaikan.

#### 5.8.12  `RegionAcc::daily_row`

- **Lokasi:** `periods.rs`, `pub fn daily_row(&mut self, d: &HruRows) -> HruRows`.
- **Padanan SWAT+:** bagian harian `basin_output`/`lsu_output` yang mencetak `bwb_d`.
- **Tujuan:** baris harian yang dicetak: awal hari dari `d_init`, air tanah dan salju sebagai rata-rata awal dan akhir hari, lalu awal hari berikutnya = akhir hari ini.

```latex
SW_{ave} = \frac{SW_{init} + SW_{final}}{2}, \qquad SNO_{pack} = \frac{SNO_{init} + SNO_{final}}{2}
```

- **Kebiasaan SWAT+ / catatan port:** `d_init` hanya maju ketika baris harian dicetak (dipanggil hanya bila `printing`). Dengan interval cetak > 1, `sw_init` dan `sw_ave` mencakup seluruh interval (docs `swat-quirks.md`: “`sw_init` moves only on printed days”).

#### 5.8.13  `RegionAcc::day`

- **Lokasi:** `periods.rs`, `pub fn day(&mut self, d: &HruRows, c: &PeriodDay, emit: impl FnMut(Period, &HruRows))`.
- **Padanan SWAT+:** bagian periode `basin_output.f90` dan `lsu_output.f90`.
- **Tujuan:** sama dengan `HruAcc::day` untuk wilayah.
- **Cara kerja:** identik dengan 5.7.9 langkah 4–7, dengan tiga perbedaan: (a) baris harian dijumlah apa adanya (rata-rata harian air tanah dan salju, bukan nilai akhir); (b) tidak ada penanganan khusus `bm_max` (kolom itu mengikuti aturan `Ave` biasa); (c) `nplt` dan `pplnt` diganti nilai hari terakhir hanya bila `point` (LSU).
- **Kebiasaan SWAT+ / catatan port:** `pplnt` cekungan selalu 0 karena `add_pw` dan `region_add` menolkannya; pada LSU juga 0 karena nilai hari terakhirnya sendiri sudah 0 setelah `region_add` (docs `outputs.md`).

#### 5.8.14  `aqu_scaled`

- **Lokasi:** `periods.rs`, `pub fn aqu_scaled(a: &AquDay, c: f32) -> AquDay`.
- **Padanan SWAT+:** `aqu_mult` (operator `*` untuk akuifer).
- **Tujuan:** mengalikan semua field dengan `c` kecuali `cbn`.

#### 5.8.15  `aqu_sum`

- **Lokasi:** `periods.rs`, `pub fn aqu_sum(acc: &mut AquDay, a: &AquDay, c: f32)`.
- **Padanan SWAT+:** jumlah harian di `basin_aquifer_output.f90`: `baqu_d = baqu_d + aqu_d(iaq) * frac`.
- **Tujuan:** menambahkan satu akuifer berbobot bagian luasnya (`area_ha / area_tot_ha`) ke jumlah cekungan; karbon menjadi milik akuifer terakhir (efek `aqu_add`).

#### 5.8.16  `BasinChaAcc::day`

- **Lokasi:** `periods.rs`, `pub fn day(&mut self, w: &ChWat, hin: &Hyd, hout: &Hyd, c: &PeriodDay, emit: …)`.
- **Padanan SWAT+:** bagian periode `basin_sdchannel_output.f90`.
- **Tujuan:** jumlah periode semua kanal cekungan.
- **Cara kerja:** seperti `ChaAcc::day`, tetapi aliran masuk/keluar **tidak** dirata-rata per hari (hanya luas air: bulan dibagi `mon_days` setelah digulir ke tahun; tahun dibagi `day_end_yr` setelah digulir ke rata-rata tahunan), dan pada akhir simulasi luas air dibagi `yrs_prt` sekali.

#### 5.8.17  `wat_sum`

- **Lokasi:** `periods.rs`, `pub fn wat_sum(acc: &mut ChWat, w: &ChWat)`.
- **Padanan SWAT+:** jumlah harian badan air di rutin keluaran cekungan.
- **Tujuan:** pembungkus publik `wat_add`, dipakai `run_with` untuk menjumlah badan air semua kanal atau reservoir dalam sehari.

#### 5.8.18  `ResAcc::day`

- **Lokasi:** `periods.rs`, `pub fn day(&mut self, w: &ChWat, hin: &Hyd, hout: &Hyd, c: &PeriodDay, emit: …)`.
- **Padanan SWAT+:** bagian periode `reservoir_output.f90` dan `basin_reservoir_output.f90`.
- **Tujuan:** jumlah periode satu reservoir atau semua reservoir cekungan.
- **Cara kerja:**
  1. Bulan: `hin_m`, `hout_m`, `wat_m` dijumlah.
  2. Akhir bulan: tambahkan aliran bulan ke tahun; `wat_m.area_ha /= mon_days` **lalu** tambahkan `wat_m` ke tahun; `emit(Mon)`; nolkan.
  3. Akhir tahun: tambahkan aliran tahun ke rata-rata tahunan; `wat_y.area_ha /= 12` **lalu** tambahkan ke rata-rata tahunan; `emit(Yr)`; nolkan.
  4. Akhir simulasi: semua dibagi `yrs_prt` (luas sekali).
- **Kebiasaan SWAT+ / catatan port:** luas air tahunan reservoir adalah rata-rata 12 rata-rata bulanan (berbeda dengan kanal yang membagi `day_end_yr`). Aliran tidak dirata-rata. Simpanan yang dicetak adalah nilai hari itu.

#### 5.8.19  `RecAcc::day`

- **Lokasi:** `periods.rs`, `pub fn day(&mut self, d: &Hyd, c: &PeriodDay, emit: impl FnMut(Period, &Hyd))`.
- **Padanan SWAT+:** bagian periode `recall_output.f90` dan `basin_recall_output.f90`.
- **Tujuan:** jumlah periode beban *recall* (sumber titik).
- **Cara kerja:** identik dengan `RuAcc::day` (jumlah dengan `plus`; rata-rata tahunan = jumlah / `yrs_prt`).

#### 5.8.20  Ringkasan pembagi per tabel

| Tabel | Bulanan | Tahunan | Rata-rata tahunan |
| --- | --- | --- | --- |
| HRU, cekungan, LSU | fluks: jumlah; keadaan: / hari bulan | fluks: jumlah; keadaan: / `day_end_yr` | fluks: / `yrs_prt`; keadaan: / `days_prt` |
| Unit routing, *recall* | jumlah | jumlah | / `yrs_prt` |
| Akuifer | simpanan, muka air, NO3 tersimpan: / hari bulan | idem: / 12 | / `yrs_prt` (kecuali karbon) |
| Kanal SWAT-DEG | `flo` dan luas: / hari bulan | `flo` dan luas: / `day_end_yr` | / `yrs_prt`; luas / `yrs_prt`² |
| Kanal cekungan | luas: / hari bulan | luas: / `day_end_yr` | / `yrs_prt` |
| Reservoir | luas: / hari bulan | luas: / 12 | / `yrs_prt` |

#### 5.8.21  Daftar periksa kelengkapan bab

Fungsi yang didokumentasikan dalam bab ini: `run.rs` 27 (`HruWeather::default`, 19 method `OutputSink`, 3 method `PrintGate`, `run`, `run_with`, `regions`, `lapse_rates`); `clock.rs` 7; `fortran.rs` 6 ditambah 4 uji; `hyd.rs` 9 ditambah 1 uji; `periods.rs` 29. Jumlah: 78 fungsi produksi dan 5 fungsi uji. `lib.rs` tidak berisi fungsi.
