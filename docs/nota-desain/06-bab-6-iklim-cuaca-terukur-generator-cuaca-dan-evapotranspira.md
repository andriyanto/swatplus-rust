## Bab 6  Iklim: cuaca terukur, generator cuaca, dan evapotranspirasi potensial

Bab ini membahas bagaimana engine menyediakan cuaca harian untuk setiap stasiun cuaca: menempatkan data stasiun terukur (hujan, suhu, radiasi matahari, kelembapan, angin) ke dalam larik harian, membangkitkan nilai yang hilang dengan generator cuaca stokastik SWAT+ (rantai Markov hujan, distribusi skew-normal, deviat normal berkorelasi untuk suhu, distribusi segitiga untuk kelembapan dan fraksi hujan setengah jam, distribusi eksponensial untuk angin), lalu menghitung besaran turunan: radiasi maksimum dan panjang hari, PET Hargreaves stasiun, indeks kelembapan 30 hari, *lag* suhu udara, dan satuan panas basis nol. Bab ini juga mencakup tiga metode PET tingkat HRU (Priestley-Taylor, Penman-Monteith, Hargreaves), koreksi *lapse rate* elevasi, serta deposisi nitrogen atmosfer (`atmo.cli`).

File Rust utama adalah `crates/swat-engine/src/climate.rs` (seluruhnya, 34 fungsi/method dan konstanta). Fungsi pendukung dari file lain yang dibahas di sini karena merupakan bagian alur iklim: generator bilangan acak dan distribusinya di `crates/swat-engine/src/fortran.rs` (`aunif`, `dstn1`, `atri`, `rpow`, `search`, `fortran_cmp`), penanganan bulan di `crates/swat-engine/src/clock.rs` (`set_ndays_mod4`, `month_index`), PET HRU di `crates/swat-engine/src/hru/mod.rs` (`pet_constants`, `pet_radiation`, `pet_priestley_taylor`, `pet_penman_monteith`, `pet_hargreaves`), serta `lapse_rates` dan kontrol waktu deposisi atmosfer di `crates/swat-engine/src/run.rs`. Padanan Fortran adalah keluarga `cli_*.f90`, `climate_control.f90`, `gcycl.f90`, `aunif.f90`, `atri.f90`, `ee.f90`, `et_pot.f90` dan `nut_nrain.f90`.

Posisi dalam alur simulasi: `Weather::new` dipanggil sekali di awal `run_with` (setara pembacaan `cli_*meas`, `cli_wgnread`, `cli_staread`); sebelum loop tahun, `precip_control(0)` menyiapkan hujan hari pertama; setiap hari `climate_control` dijalankan sebelum objek mana pun disimulasikan, kemudian `run.rs` menyalin cuaca stasiun ke `HruWeather` setiap HRU (dengan koreksi *lapse rate* bila `codes.bsn` `lapse` = 1) dan HRU menghitung PET-nya sendiri di dalam fase lahan. Urutan setiap tarikan bilangan acak dipertahankan persis seperti SWAT+, karena nilai yang dibangkitkan bergantung padanya: satu tarikan ekstra menggeser seluruh deret berikutnya.

### 6.1  Data stasiun terukur: `Gage` dan penempatannya

Struct `Gage` mewakili satu file stasiun terukur (`*.pcp`, `*.tmp`, `*.slr`, `*.hmd`, `*.wnd`) setelah ditempatkan ke larik harian, sebagaimana pembaca `cli_pmeas`, `cli_tmeas`, `cli_smeas`, `cli_hmeas`, `cli_wmeas` meninggalkannya.

| Field | Arti | Satuan |
| --- | --- | --- |
| `start_day`, `start_yr` | hari Julian dan tahun rekaman pertama file | hari, tahun |
| `end_day`, `end_yr` | hari dan tahun rekaman terakhir | hari, tahun |
| `yrs_start` | selisih tahun antara tahun pertama simulasi dan tahun pertama file, bila file mulai lebih lambat (0 bila tidak) | tahun |
| `tstep` | langkah waktu file: 0 harian, >0 sub-harian | – |
| `elev` | elevasi stasiun (dipakai *lapse rate*) | m |
| `ts` | `ts(day, year)`: nilai harian (hujan mm, suhu maksimum °C, radiasi MJ/m², kelembapan relatif fraksi, angin m/s), indeks `[year-1][day-1]` | sesuai variabel |
| `ts2` | suhu minimum (`ts2`), hanya untuk suhu | °C |
| `tss` | hujan sub-harian `tss(step, day, year)`, rata per tahun, hari, langkah | mm per langkah |
| `step` | jumlah langkah per hari | – |

Nilai hilang dalam file ditulis -99; hari di luar rentang file ditandai -98 oleh pemanggil. Ambang "hilang" yang dipakai di seluruh modul adalah `<= -97`.

Enum `Source` menyatakan sumber satu variabel di sebuah stasiun: `Sim` (nama `sim` di `weather-sta.cli`, dibangkitkan) atau `Gage(n)` (indeks 1-basis ke daftar gage variabel tersebut; 0 bila nama tidak ditemukan, yaitu gage kosong yang selalu di luar batas).

#### 6.1.1  `Gage::place`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `pub fn place(series: &StationSeries, yrc: i32, day_start: i32, step: usize, ndays: Option<&mut Clock>) -> Option<Gage>`
- **Padanan SWAT+:** bagian penempatan data di `cli_pmeas.f90`, `cli_tmeas.f90`, `cli_smeas.f90`, `cli_hmeas.f90`, `cli_wmeas.f90`.
- **Tujuan:** menempatkan rekaman sebuah deret stasiun (yang sudah dibaca `swat-io`) ke larik `ts`/`ts2`/`tss` dengan aturan yang sama persis dengan pembaca Fortran.
- **Masukan dan keluaran:** `series` (rekaman tahun, hari Julian, nilai; `tstep`; `elev`), `yrc` tahun pertama simulasi, `day_start` hari pertama simulasi, `step` langkah per hari, `ndays` tabel bulan global (hanya `Some` untuk suhu). Keluaran `Some(Gage)`; `None` bila file tidak berisi rekaman data (pembaca berhenti di variabel itu).
- **Cara kerja:**
  1. Ambil rekaman pertama dan terakhir untuk `start_*`/`end_*`; `yrs_start = first.year - yrc` bila positif.
  2. Jika `tstep > 0` (sub-harian): cari rekaman pertama dengan `year >= yrc` dan `jday >= day_start`. Mulai dari situ, setiap rekaman dengan hari 1–366 mengisi langkah berikutnya `iss` dari harinya di kolom tahun `iyrs`; `iss` berputar 1..`step` tanpa melihat kolom jam di file. Bila hari 365 atau 366 diikuti rekaman bertahun lain, kolom tahun maju.
  3. Jika harian: cari rekaman awal yang sama. Bila ini file suhu, tabel bulan global di-set dengan aturan kabisat "habis dibagi 4" dari tahun rekaman tempat pencarian berhenti (rekaman terakhir bila tidak ada yang memenuhi).
  4. Isi `ts[iyrs-1][jday-1]` dengan nilai pertama dan `ts2` dengan nilai kedua (suhu minimum). Pada hari 365/366 yang diikuti tahun berbeda, kolom maju dan (untuk suhu) `ndays` di-set ulang dari tahun berikutnya.
- **Kebiasaan SWAT+ / catatan port:** (a) Kondisi awal `year >= yrc && jday >= day_start` diterapkan bersama, sehingga pada tahun-tahun setelah `yrc` rekaman dengan hari < `day_start` tetap dilompati sampai ditemukan hari yang memenuhi, persis seperti loop lompat Fortran. (b) Pembaca suhu mengubah tabel `ndays` global; tabel itulah yang kemudian dipakai `cli_initwgn` untuk panjang bulan (lihat 6.3.8). (c) Header file bisa salah (contoh Osu\_1hru: header 42 tahun, isi 43); port mengikuti rekaman, bukan header. (d) Hujan sub-harian mengisi langkah menurut urutan baris, bukan kolom jam.

#### 6.1.2  `Gage::value`, `Gage::value2`, `Gage::steps`, `Gage::at`

- **Lokasi:** `crates/swat-engine/src/climate.rs`; `value(&self, day, year) -> f32`, `value2(&self, day, year) -> f32`, `steps(&self, day, year) -> Vec<f32>`, privat `at(ts: &[[f32;366]], day, year) -> f32`.
- **Padanan SWAT+:** akses langsung `pcp(ig)%ts(day, yrs)`, `tmp(ig)%ts2(...)`, `pcp(ig)%tss(:, day, yrs)`.
- **Tujuan:** membaca nilai harian (`ts`), suhu minimum (`ts2`), atau vektor langkah sub-harian (`tss`) untuk hari dan tahun-ke-n tertentu.
- **Masukan dan keluaran:** `day` 1–366, `year` tahun ke-n (1-basis) terhitung dari kolom pertama yang ditempatkan. Keluaran nilai f32 atau vektor `step` nilai.
- **Cara kerja:** `value`/`value2` mendelegasikan ke `at`, yang mengembalikan 0 bila `day < 1`, `year < 1`, atau indeks di luar yang sudah ditempatkan. `steps` menghitung indeks rata `((year-1)·366 + day-1)·step` dan mengembalikan irisan `step` nilai, atau vektor nol.
- **Kebiasaan SWAT+ / catatan port:** nilai 0 di luar jangkauan meniru larik Fortran yang dialokasikan dan diisi nol. Indeks di luar batas larik Fortran (perilaku tak terdefinisi) diganti 0 secara aman.

#### 6.1.3  `Gage::out_of_bounds`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `pub fn out_of_bounds(&self, clock: &Clock) -> bool`
- **Padanan SWAT+:** `cli_bounds_check.f90`.
- **Tujuan:** menentukan apakah hari simulasi berjalan berada di luar rentang rekaman file.
- **Masukan dan keluaran:** `clock.yrc` (tahun kalender), `clock.day` (hari Julian); keluaran `true` bila di luar.
- **Cara kerja:**
  1. Di luar bila file mulai setelah hari ini: `start_yr > yrc`, atau tahunnya sama dan `start_day > day`.
  2. Di luar bila file berakhir sebelum hari ini: `end_yr < yrc`, atau tahunnya sama dan `end_day < day`.
- **Kebiasaan SWAT+ / catatan port:** gage kosong (`Gage::default`, semua nol) selalu di luar batas, sehingga stasiun yang nama gagenya tidak ditemukan jatuh ke generator. Contoh nyata: seri radiasi dan angin Osu\_1hru berakhir hari 304 tahun 2022; hari sesudahnya ditandai -98 dan dibangkitkan.

#### 6.1.4  `build_gages`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn build_gages(project, var: ClimateVar, yrc, day_start, step, clock: Option<&mut Clock>) -> Vec<Gage>`
- **Padanan SWAT+:** loop stasiun di setiap `cli_*meas.f90` (membaca file-file yang terdaftar di `pcp.cli`, `tmp.cli`, dst.).
- **Tujuan:** membangun daftar gage satu variabel dalam urutan daftar stasiun, dengan indeks 0 sebagai gage kosong.
- **Masukan dan keluaran:** daftar file dari `project.station_lists` untuk `var`, deret dari `project.series`; keluaran `Vec<Gage>` sepanjang jumlah file + 1.
- **Cara kerja:**
  1. Alokasikan `files.len() + 1` gage kosong.
  2. Untuk setiap file dalam urutan daftar, ambil deret berikutnya dari variabel yang sama; bila nama file tidak cocok atau deret habis, berhenti.
  3. Tempatkan dengan `Gage::place`; bila `None` (file tanpa data), berhenti.
- **Kebiasaan SWAT+ / catatan port:** file kosong atau hilang menghentikan seluruh variabel: pembaca Fortran keluar dari loop stasiun pada file pertama yang berakhir di dalam header, sehingga file sesudahnya juga tidak dibaca (gfortran bahkan membuat file yang hilang sebagai file kosong). Contoh: `tmp.cli` Ames\_sub1 mendaftar `ames.tmp` yang kosong, sehingga tidak ada suhu terukur.

#### 6.1.5  Fungsi bantu tabel bulan

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `Clock::set_ndays_mod4(&mut self, year)` | `crates/swat-engine/src/clock.rs` | Men-set tabel kumulatif hari per bulan `ndays` ke tabel kabisat bila `year % 4 == 0`, selain itu non-kabisat; aturan yang dipakai pembaca file (aturan abad hanya di `time_control`). |
| `Clock::month_index(&self) -> usize` | `crates/swat-engine/src/clock.rs` | Indeks bulan 0–11 dari `clock.mo` (di-clamp 1–12) untuk tabel bulanan generator. |

### 6.2  Bilangan acak dan distribusinya

Semua keacakan generator cuaca berasal dari satu generator kongruensial linear per benih (*seed*). Setiap stasiun generator (`WgnState`) memiliki sepuluh benih `seed[0..10]` (`rndseed(1:10, iwgn)`), dan larik global `idg[0..9]` memetakan "tarikan ke-k" ke slot benih mana yang dipakai. Fungsi-fungsi berikut berada di `crates/swat-engine/src/fortran.rs` dan sengaja ditulis dalam presisi tunggal dengan urutan operasi Fortran agar hasilnya identik bit demi bit.

Pemetaan pemakaian benih (indeks `idg` 1-basis, sebelum pengacakan oleh `igen`):

| `idg(k)` | Dipakai oleh | Kegunaan |
| --- | --- | --- |
| 1 | `pgen` | uji basah/kering rantai Markov |
| 2 | `weatgn` | deviat normal ke-3 (radiasi) |
| 3 | `pgen`, salinan di `init_wgn` | jumlah hujan (skew-normal), faktor koreksi `pcf` |
| 4 | – | tidak dipakai di port (arah angin di Fortran dimatikan) |
| 5 | `wndgen` | kecepatan angin |
| 6 | `gcycl` | salinan awal untuk benih ke-10 |
| 7 | `rhgen` | kelembapan relatif (distribusi segitiga) |
| 8 | `weatgn` | deviat normal ke-1 (suhu maksimum) |
| 9 | `weatgn` | deviat normal ke-2 (suhu minimum) |
| benih ke-10 | `climate_control` | fraksi hujan setengah jam |

#### 6.2.1  `aunif`

- **Lokasi:** `crates/swat-engine/src/fortran.rs`, `pub fn aunif(seed: &mut i32) -> f32`
- **Padanan SWAT+:** fungsi `Aunif` di `aunif.f90`.
- **Tujuan:** menghasilkan bilangan seragam di (0, 1) dan memajukan benih; generator "minimal standard" Park–Miller dengan metode Schrage untuk menghindari *overflow* bilangan bulat 32-bit.
- **Masukan dan keluaran:** `seed` (i32, diubah di tempat); keluaran f32.
- **Cara kerja:**
  1. Hitung `x2 = seed / 127773` (pembagian bulat).
  2. Perbarui benih dengan dekomposisi Schrage dari perkalian modular:

```latex
s_{n+1} = 16807\,(s_n - x_2\cdot 127773) - 2836\,x_2,\qquad s_{n+1} \leftarrow s_{n+1} + 2147483647 \text{ bila } s_{n+1}<0
```

3. Ini setara dengan

```latex
s_{n+1} = 16807\,s_n \bmod (2^{31}-1),\qquad u = s_{n+1}\cdot 4.656612875\times10^{-10}
```

4. Perkalian dilakukan dalam presisi ganda lalu dibulatkan ke tunggal.

- **Kebiasaan SWAT+ / catatan port:** konstanta `4.656612875d-10` di Fortran adalah literal presisi ganda yang hasilnya ditugaskan ke `real`; port meniru dengan `(f64::from(seed) * 4.656612875e-10) as f32`. Pembulatan langsung di f32 dapat berbeda pada bit terakhir.

#### 6.2.2  `dstn1`

- **Lokasi:** `crates/swat-engine/src/fortran.rs`, `pub fn dstn1(rn1: f32, rn2: f32) -> f32`
- **Padanan SWAT+:** `cli_dstn1.f90`.
- **Tujuan:** deviat normal baku dari dua bilangan seragam (transformasi Box–Muller, cabang kosinus).
- **Masukan dan keluaran:** `rn1`, `rn2` di (0,1); keluaran deviat N(0,1).
- **Cara kerja:**

```latex
z = \sqrt{-2\ln r_1}\,\cos(6.283185\,r_2)
```

- **Kebiasaan SWAT+ / catatan port:** pemanggil selalu memakai `r_1` = bilangan seragam yang disimpan dari hari sebelumnya (`rnd2`, `rnd3`, `rnd8`, `rnd9`) dan `r_2` = tarikan baru, lalu menyimpan `r_2` untuk hari berikutnya. Dengan begitu setiap hari hanya satu tarikan baru per deviat. Konstanta 2π ditulis 6.283185 (presisi tunggal), bukan `std::f32::consts::TAU`.

#### 6.2.3  `atri`

- **Lokasi:** `crates/swat-engine/src/fortran.rs`, `pub fn atri(at1: f32, at2: f32, at3: f32, seed: &mut i32) -> f32`
- **Padanan SWAT+:** fungsi `Atri` di `atri.f90`.
- **Tujuan:** menarik nilai dari distribusi segitiga dengan batas bawah `at1`, modus `at2`, batas atas `at3`, kemudian diskalakan agar rerata sama dengan `at2`, dan dijaga di (0, 1).
- **Masukan dan keluaran:** tiga parameter (tanpa satuan, fraksi); benih dimajukan satu kali; keluaran fraksi.
- **Cara kerja:**
  1. Tarik `rn = aunif(seed)`; hitung `u3 = at2 - at1`, `y = 2/(at3 - at1)`, `b2 = at3 - at2`, `b1 = rn / y`, `x1 = y·u3/2` (luas di kiri modus).
  2. Invers CDF segitiga:

```latex
x = \begin{cases} a_1 + \sqrt{2\,b_1\,u_3} & rn \le x_1 \\ a_3 - \sqrt{b_2^2 - 2\,b_2\,(b_1 - 0.5\,u_3)} & rn > x_1 \end{cases}
```

(Akar dari argumen <= 0 diganti 0.)

- Langkah 3. Skala ke modus, dengan rerata segitiga:

```latex
\bar{a} = \frac{a_1+a_2+a_3}{3},\qquad x \leftarrow x\,\frac{a_2}{\bar a}
```

4. Batasi: `x >= 1` menjadi 0.99; `x <= 0` menjadi 0.001.

- **Kebiasaan SWAT+ / catatan port:** penskalaan ke `at2` membuat rerata hasil sama dengan nilai "rerata" yang diminta pemanggil, bukan rerata segitiga asli. Batas (0.001, 0.99) berlaku untuk semua pemakaian (kelembapan relatif, fraksi hujan setengah jam, bentuk badai sub-harian).

#### 6.2.4  `rpow`

- **Lokasi:** `crates/swat-engine/src/fortran.rs`, `pub fn rpow(x: f32, y: f32) -> f32`
- **Padanan SWAT+:** operator Fortran `x ** y` dengan eksponen real.
- **Tujuan:** memaksa pemanggilan `powf` pustaka matematika seperti gfortran.
- **Masukan dan keluaran:** basis dan eksponen f32.
- **Cara kerja:** `x.powf(black_box(y))`: eksponen disembunyikan dari pengoptimal.
- **Kebiasaan SWAT+ / catatan port:** pengoptimal Rust mengubah `x.powf(0.5)` menjadi `sqrt`, yang dibulatkan benar, sedangkan `powf` tidak selalu (berbeda di bit terakhir untuk kira-kira 1 dari 1600 nilai). Satu bit beda pada PET Hargreaves pernah cukup untuk menggeser air satu lapisan tanah satu bit beberapa hari kemudian dan satu digit tercetak beberapa minggu sesudahnya (`docs/swat-quirks.md`, `docs/phase2.md`). Karena itu kedua rumus Hargreaves (stasiun dan HRU) dan Penman-Monteith memakai `rpow`.

#### 6.2.5  Pencarian nama: `search` dan `fortran_cmp`

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `fortran_cmp(a, b) -> Ordering` | `crates/swat-engine/src/fortran.rs` | Membandingkan dua nama seperti Fortran membandingkan `character(len=50)`: dipotong 50 karakter dan diisi spasi. |
| `search(list, find) -> Option<usize>` | `crates/swat-engine/src/fortran.rs` | Padanan `search` SWAT+: pencarian biner posisi 1-basis, menganggap daftar terurut dan menyerah setelah 25 kali pembelahan. Dipakai `read_stations` (nama generator dan gage), `run_with` (stasiun tiap HRU) dan deposisi atmosfer. |

Kebiasaan penting: karena biner, nama yang ada dalam daftar tidak terurut bisa tidak ditemukan. Bila tidak ditemukan, SWAT+ membiarkan nilai pemanggil: generator jatuh ke 1, stasiun objek jatuh ke 1, gage menjadi 0 (gage kosong). Contoh Ames\_sub1: stasiun menyebut gage suhu `ames.tem` yang tidak ada di daftar `["ames.tmp"]`, sehingga suhu selalu dibangkitkan; HRU-nya menyebut `wgn_01` sedangkan stasiunnya `wgn_sta1`, sehingga memakai stasiun 1.

### 6.3  Inisialisasi: struktur `Weather`, generator, dan stasiun

Struktur data inti:

- **`Weather`**: seluruh keadaan iklim. Field: `pcp`, `tmp`, `slr`, `hmd`, `wnd` (daftar `Gage`, indeks 0 kosong); `wgn: Vec<WgnState>`; `stations: Vec<Station>`; `idg: [usize; 9]` (pemetaan tarikan ke slot benih); `ppet_mce` (posisi jendela bergerak 30 hari, 1–30); `rndseed_cond` (benih untuk manajemen bersyarat `prob_unif`).
- **`WgnState`**: parameter generator setelah `cli_initwgn` dan keadaannya. Field: `lat` (°), `elev` (m), `m: [WgnMonth; 12]` (parameter bulanan: `tmp_max_ave`, `tmp_min_ave`, `tmp_max_sd`, `tmp_min_sd` °C; `pcp_ave` mm/bulan, `pcp_sd`, `pcp_skew`; `wet_dry`, `wet_wet` peluang; `pcp_days` hari; `pcp_hhr` hujan setengah jam maksimum mm; `slr_ave` MJ/m²/hari; `dew_ave` °C atau fraksi; `wnd_ave` m/s), `rain_yrs` (tahun data hujan maksimum setengah jam), `latsin`, `latcos`, `daylmn` (panjang hari minimum, jam), `daylth` (ambang dormansi, jam), `phutot` (satuan panas tahunan basis 0, °C·hari), `pr_wdays` (fraksi hari basah per bulan), `pcpmean` (hujan rerata per hari basah, mm), `pcf` (faktor koreksi hujan), `amp_r` (rerata fraksi hujan setengah jam), `pet` (PET bulanan, mm), `dew_is_rhum`, jendela 30 hari `precip_mce`/`pet_mce` dan jumlahnya `precip_sum`/`pet_sum`, `p_pet_rto` (rasio P/PET), `pcp_an` (mm/tahun), `tmp_an` (°C), `seed[10]`, bilangan seragam tersimpan `rnd2`, `rnd3`, `rnd8`, `rnd9`, dan deviat berkorelasi `wgncur`/`wgnold[3]`.
- **`Station`**: `name`, `wgn` (1-basis), sumber `pcp`/`tmp`/`slr`/`hmd`/`wnd` (`Source`), `petgage`, `weat: Weat`, `tlag[6]`, `tlag_mne`, `airlag_temp` (°C).
- **`Weat`** (padanan `weather_daily`): `precip`, `precip_next` (mm), `tmax`, `tmin`, `tave`, `dewpt` (°C), `solrad`, `solradmx` (MJ/m²/hari), `rhum` (fraksi), `windsp` (m/s), `pet` (mm), `phubase0` (fraksi satuan panas tahunan), `ppet` (akumulasi P/PET), `daylength` (jam), `precip_half_hr` (fraksi), `prior_wet`, `ts`/`ts_next` (hujan per langkah, mm).
- Konstanta **`SEEDS`**: sembilan benih bawaan `gcycl` (748932582, 1985072130, 1631331038, 67377721, 366304404, 1094585182, 1767585417, 1980520317, 392682216).

#### 6.3.1  `Weather::new`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `pub fn new(project: &Project, clock: &mut Clock) -> Self`
- **Padanan SWAT+:** urutan pembacaan di `proc_read`/`proc_db`: `cli_pmeas`, `cli_tmeas`, `cli_smeas`, `cli_hmeas`, `cli_wmeas`, lalu `cli_wgnread` (memanggil `gcycl` dan `cli_initwgn`), lalu `cli_staread`.
- **Tujuan:** menyiapkan seluruh keadaan cuaca sebelum simulasi.
- **Masukan dan keluaran:** `project` (data masukan), `clock` (diubah: tabel `ndays` oleh pembaca suhu). Keluaran `Weather`.
- **Cara kerja:**
  1. Bangun gage dalam urutan baca: hujan, suhu (satu-satunya yang diberi `clock` agar mengubah `ndays`), radiasi, kelembapan, angin.
  2. Inisialisasi `idg = [1..9]`, `ppet_mce = 0`, `rndseed_cond = 748932582`.
  3. `read_wgn`, lalu `read_stations`.
- **Kebiasaan SWAT+ / catatan port:** urutan baca penting karena tabel bulan global yang ditinggalkan pembaca suhu menentukan panjang bulan di `init_wgn`.

#### 6.3.2  `Weather::seed`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn seed(&mut self, iwgn: usize, which: usize) -> Option<&mut i32>`
- **Padanan SWAT+:** ekspresi `rndseed(idg(which), iwgn)`.
- **Tujuan:** mengembalikan referensi mutable ke benih yang dipetakan `idg(which)` pada generator `iwgn` (keduanya 1-basis).
- **Cara kerja:** `slot = idg[which-1]`, lalu `wgn[iwgn-1].seed[slot-1]`; `None` bila indeks di luar jangkauan (pemanggil lalu memakai 0.0 sebagai bilangan acak).

#### 6.3.3  `Weather::read_wgn`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn read_wgn(&mut self, project: &Project, clock: &Clock)`
- **Padanan SWAT+:** `cli_wgnread.f90`.
- **Tujuan:** membuat `WgnState` untuk setiap generator di `weather-wgn.cli`, menjalankan `gcycl`, lalu `cli_initwgn` untuk setiap generator.
- **Masukan dan keluaran:** `project.wgn`, `project.params.igen` (dari `parameters.bsn`), `adj_pkr`, `dorm_hr`; mengisi `self.wgn`.
- **Cara kerja:** (1) `new_wgn_state` untuk tiap baris; (2) `gcycl(igen)`; (3) `slot = idg[2]` (slot benih jumlah hujan sesudah pengacakan); (4) `init_wgn` untuk tiap generator dengan slot itu.
- **Kebiasaan SWAT+ / catatan port:** nilai `igen` bisa tidak terbaca: bila `parameters.bsn` menulis `0.00000`, gfortran tidak membacanya sebagai bilangan bulat dan `igen` tetap bawaan 5, yang mengubah benih (`docs/swat-quirks.md`).

#### 6.3.4  `Weather::gcycl`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn gcycl(&mut self, igen: i32)`
- **Padanan SWAT+:** `gcycl.f90`.
- **Tujuan:** menetapkan benih awal, menggesernya menurut kode `igen` bila tidak nol, mengacak pemetaan `idg`, dan melakukan tarikan pertama.
- **Masukan dan keluaran:** `igen`; mengubah `idg`, `seed`, `rnd2/3/8/9` setiap generator, dan `rndseed_cond`.
- **Cara kerja:**
  1. `idg = [1..9]`; setiap generator mendapat sembilan benih `SEEDS`.
  2. Bila `igen != 0`, dengan benih bantu `rndseed10 = 64298628`, untuk j = 1..9: tarik `rn`, hitung `ii = int(100·igen·rn)`, buang `ii` tarikan berikutnya, lalu simpan `rndseed10` sebagai benih j **hanya untuk generator pertama**.
  3. `rndseed_cond = rndseed10` (benih untuk `prob_unif` di tabel keputusan).
  4. Masih dengan `igen != 0`, acak `idg` (Fisher–Yates mundur): untuk j = 9..1, `k = int(j·rn + 1)` (di-clamp 1..9), tukar `idg(j)` dan `idg(k)`.
  5. Untuk setiap generator: `seed[9] = seed[idg(6)-1]` (benih ke-10 adalah salinan), lalu tarikan pertama `rnd2 = aunif(seed[idg(2)])`, `rnd3 = aunif(seed[idg(3)])`, `rnd8 = aunif(seed[idg(8)])`, `rnd9 = aunif(seed[idg(9)])`.
- **Kebiasaan SWAT+ / catatan port:** (a) penggeseran benih oleh `igen` hanya mengenai generator pertama; generator lain tetap memakai benih bawaan, sehingga pada `igen = 0` semua generator dengan parameter sama menghasilkan deret yang sama. (b) Pengacakan `idg` berlaku global. (c) `rndseed_cond` = 748932582 bila `igen = 0`.

#### 6.3.5  `Weather::read_stations`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn read_stations(&mut self, project: &Project)`
- **Padanan SWAT+:** `cli_staread.f90`.
- **Tujuan:** membuat setiap `Station` dari `weather-sta.cli` dan mencari generator, gage tiap variabel, dan gage PET menurut nama.
- **Cara kerja:**
  1. Generator dicari dengan `search` di daftar nama generator; tidak ditemukan berarti 1.
  2. Untuk setiap variabel: nama `sim` berarti `Source::Sim`; nama lain (termasuk `null` atau kosong) dicari di daftar file variabel (`pcp.cli`, dst.), tidak ditemukan berarti `Gage(0)`.
  3. `petgage` dari daftar `pet.cli` (0 bila tidak ada).
  4. Larik *lag* suhu `tlag[0..6]` diisi `(tmp_min_ave + tmp_max_sd)/2` dari bulan Januari generator; `tlag_mne = 1`.
- **Kebiasaan SWAT+ / catatan port:** (a) nama `null` tidak sama dengan `sim`: ia lewat jalur terukur dengan gage kosong, akhirnya juga dibangkitkan, tetapi menarik lebih banyak bilangan acak di jalan (suhu memanggil `cli_weatgn` dua kali). (b) Nilai awal `tlag` memakai simpangan baku suhu maksimum, bukan rerata suhu maksimum; ini ditiru apa adanya dari kode Fortran (kemungkinan salah ketik di SWAT+, tetapi dipertahankan).

#### 6.3.6  `new_wgn_state`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn new_wgn_state(w: &Wgn) -> WgnState`
- **Padanan SWAT+:** alokasi `wgn`/`wgn_pms` di `cli_wgnread.f90`.
- **Tujuan:** menyalin nama, lintang, elevasi, parameter bulanan, dan `rain_yrs` dari masukan; semua besaran turunan dan keadaan diisi nol.

#### 6.3.7  `Weather::gage`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn gage(list: &[Gage], i: usize) -> Option<&Gage>`
- **Tujuan:** akses aman ke gage ke-`i`; pemanggil memakai `Gage::default()` (selalu di luar batas) bila `None`.

#### 6.3.8  `init_wgn`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn init_wgn(w: &mut WgnState, clock: &Clock, adj_pkr: f32, dorm_hr: f32, seed3_slot: usize)`
- **Padanan SWAT+:** `cli_initwgn.f90`.
- **Tujuan:** menghitung besaran turunan generator: lintang, panjang hari minimum, ambang dormansi, peluang Markov, hujan rerata per hari basah, PET bulanan, faktor koreksi hujan, rerata fraksi hujan setengah jam, isi awal jendela P/PET 30 hari.
- **Masukan dan keluaran:** `clock.ndays` (tabel bulan global saat itu), `clock.mo` (bulan hari pertama), `adj_pkr` (faktor penyesuaian puncak hujan, `parameters.bsn`), `dorm_hr` (jam ambang dormansi), `seed3_slot`.
- **Cara kerja:**
  1. Lintang ke radian `x = lat/57.296`; simpan `sin`, `cos`. Panjang hari minimum (jam):

```latex
x_1 = 0.4348\,|\tan\phi|,\qquad \text{daylmn} = 7.6394\,\arccos(x_1)\ (0 \text{ bila } x_1\ge1)
```

(0.4348 ≈ tan 23.5°; 7.6394 = 24/π.)

- Langkah 2. Ambang dormansi `daylth`: bila `dorm_hr` ≈ 0, 1 jam untuk |lat| > 40°, -1 jam untuk |lat| < 20°, dan `(|lat| - 20)/20` di antaranya; selain itu `dorm_hr`.
- Langkah 3. Hujan setengah jam maksimum dihaluskan tiga bulan (siklik): `rain_hhsm(m) = (pcp_hhr(m-1) + pcp_hhr(m) + pcp_hhr(m+1))/3`.
- Langkah 4. Per bulan (dengan `md` = jumlah hari bulan dari `ndays`): akumulasi `phutot += tav·md` untuk `tav > 0`. Bila `wet_wet <= wet_dry` atau `wet_dry <= 0`, peluang Markov diturunkan dari jumlah hari basah:

```latex
P(W|D) = 0.75\,\frac{\text{pcp\_days}}{md},\qquad P(W|W) = 0.25 + P(W|D)
```

(dengan `pcp_days >= 0.1`); sebaliknya jumlah hari basah dihitung dari peluang stasioner rantai Markov:

```latex
\text{pcp\_days} = md\,\frac{P(W|D)}{1 - P(W|W) + P(W|D)}
```

Lalu `pcp_days >= 0.001`, `pr_wdays = pcp_days/md`, `pcpmean = pcp_ave/pcp_days` (mm per hari basah), `pcp_skew >= 0.2`.

- Langkah 5. PET bulanan ala Priestley-Taylor dengan albedo 0.15 dan α = 1.28 (suhu Kelvin `tk = tav + 273`):

```latex
\Delta = \frac{5304}{tk^2}\,e^{21.255 - 5304/tk},\quad \gamma' = \frac{\Delta}{\Delta + 0.68},\quad H_o = \frac{23.9\,R_{s}\,(1-0.15)}{58.3},\quad PET_m = 1.28\,H_o\,\gamma'\,md
```

6. `pcp_an = Σ pcp_ave`; `tmp_an = (Σ tmp_max_ave + Σ tmp_min_ave)/24`.
7. `dew_is_rhum` benar bila semua `dew_ave` bulanan di \[0, 1\] (artinya kolom titik embun berisi kelembapan relatif).
8. Jendela 30 hari diisi rerata harian bulan **sebelum** bulan awal simulasi: `precip_mce[k] = pcp_ave/md`, `pet_mce[k] = PET_m/md`, dan jumlahnya.
9. Faktor koreksi hujan: dengan salinan benih `seed[seed3_slot-1]`, tarik `rndm1`, lalu per bulan bangkitkan 1000 nilai skew-normal (rumus sama dengan `pgen`, batas bawah 0.01 mm) dan

```latex
pcf_m = \frac{1000\,\overline{p}_m}{\sum_{i=1}^{1000} p_{gen,i}}
```

10. Rerata fraksi hujan setengah jam: `rain_yrs < 1` menjadi 10;

```latex
x_2 = \frac{0.5/\text{rain\_yrs}}{\text{pcp\_days}},\quad x_3 = \frac{\text{rain\_hhsm}}{\ln x_2},\quad amp\_r = adj\_pkr\,\bigl(1 - e^{x_3/\overline{p}}\bigr)
```

(0.95 bila `pcpmean` <= 1e-4), di-clamp ke \[0.1, 0.95\].

- **Kebiasaan SWAT+ / catatan port:** (a) panjang bulan diambil dari tabel `ndays` global sebagaimana ditinggalkan pembaca suhu (kabisat bila tahun rekaman terakhirnya habis dibagi 4), atau tabel kabisat awal bila tidak ada file suhu terbaca; bukan dari tahun simulasi. (b) Faktor koreksi memakai salinan benih, sehingga benih generator sendiri tidak bergerak; tetapi `rndm1` berlanjut antar bulan. (c) `phutot` diakumulasikan ke nilai yang sudah ada (nol dari `new_wgn_state`). (d) `time%mo` adalah bulan hari pertama sebagaimana ditemukan `time_read`. (e) `daylth = -1` untuk |lat| < 20° berarti tanaman tahunan (perennial) tidak pernah dorman di daerah tropis (lihat `docs/swat-quirks.md`).

### 6.4  Hujan: generator harian, badai sub-harian, dan kontrol hujan

Hujan diputuskan satu hari di muka: pada hari t, `precip_control` memindahkan `precip_next` menjadi hujan hari ini lalu membaca atau membangkitkan hujan hari t+1. Generator hujan adalah rantai Markov orde satu (basah/kering) dengan jumlah hujan dari distribusi skew-normal (Nicks 1974; Neitsch et al. 2009).

#### 6.4.1  `Weather::pgen`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn pgen(&mut self, ist: usize, mo: usize)`
- **Padanan SWAT+:** `cli_pgen.f90`.
- **Tujuan:** membangkitkan hujan hari berikutnya (`precip_next`, mm) untuk stasiun `ist` pada bulan `mo` (0–11).
- **Masukan dan keluaran:** `prior_wet` stasiun, parameter bulanan `wet_wet`, `wet_dry`, `pcp_skew`, `pcp_sd`, `pcpmean`, `pcf`; benih `idg(1)` dan `idg(3)`; keluaran `weat.precip_next` (mm).
- **Cara kerja:**
  1. Tarik `vv` dari benih `idg(1)`.
  2. Ambang `xx = P(W|W)` bila hari sebelumnya basah, `P(W|D)` bila kering.
  3. Bila `vv > xx`, hari kering: hujan 0 (tidak ada tarikan lain).
  4. Bila basah: tarik `v8` dari benih `idg(3)`, hitung deviat normal `z = dstn1(rnd3, v8)`, simpan `rnd3 = v8`, lalu transformasi skew-normal (persamaan di bawah), kalikan `pcf`, dan batasi minimal 0.1 mm.
- **Persamaan:**

```latex
r_6 = \frac{g}{6},\qquad x = \frac{2}{g}\Bigl[\bigl((z - r_6)\,r_6 + 1\bigr)^3 - 1\Bigr],\qquad P = \max\bigl(0.1,\ (x\,\sigma_m + \mu_m)\,pcf_m\bigr)
```

dengan g = `pcp_skew`, σ = `pcp_sd`, μ = `pcpmean` (mm per hari basah).

- **Kebiasaan SWAT+ / catatan port:** karena `prior_wet` diperbarui `clgen` dari hujan hari ini sesudah `precip_control` menghitung hari esok, peluang hujan hari yang dibangkitkan mengikuti status basah/kering **dua hari sebelumnya** (`docs/swat-quirks.md`). Pangkat tiga ditulis `xlv*xlv*xlv`, bukan `powi`, agar sama dengan Fortran.

#### 6.4.2  `Weather::pgenhr`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn pgenhr(&mut self, ist: usize, step: usize)`
- **Padanan SWAT+:** `cli_pgenhr.f90`.
- **Tujuan:** menyebar hujan **hari ini** ke langkah-langkah **besok** (`ts_next`) sebagai badai berbentuk eksponensial ganda (naik lalu turun di sekitar puncak) untuk run sub-harian.
- **Masukan dan keluaran:** `weat.precip` (mm), `weat.precip_half_hr` (fraksi hujan pada setengah jam terderas, dari hari sebelumnya), `step` (langkah per hari); keluaran `weat.ts_next[0..step]` (mm per langkah); `weat.ts` dinolkan.
- **Cara kerja:**
  1. Nolkan `ts` hari ini; perbesar `ts_next` ke `step` bila perlu. Bila hujan < 1e-6, selesai.
  2. Laju puncak `pkrr = 2·P·f_{0.5h}` (mm/jam).
  3. Posisi relatif puncak `vv = atri(0.05, 0.25, 0.95, k)` dengan benih lokal `k = 8` yang dibuat ulang setiap panggilan.
  4. Konstanta peluruhan `xk1 = vv/4.605`, `xk2 = (1-vv)/4.605` (4.605 = ln 100).
  5. Durasi `dur = P / (pkrr·(xk1 + xk2))` jam; bila > 24, `dur = 24` dan `pkrr` dihitung ulang.
  6. Hujan kumulatif sampai puncak `pkrain = vv·P`; waktu puncak `rtp = vv·dur·60` menit; `xkp1 = dur·xk1`, `xkp2 = dur·xk2`; `dtm = 1440/step` menit.
  7. Fase naik: untuk `pt` (menit, bilangan bulat) mulai `dtm` selama `pt < rtp`, hujan kumulatif `rx` dari persamaan pertama; `ts_next[itime] = rx - sumrain`.
  8. Fase turun: selama `pt < dur·60`, `rx` dari persamaan kedua.
  9. Sisa `P - sumrain` (bila positif) ditambahkan ke langkah terakhir yang diisi.
- **Persamaan:**

```latex
R(t) = \begin{cases} R_p - i_p\,k'_1\Bigl(1 - \exp\bigl(\frac{t - t_p}{60\,k'_1}\bigr)\Bigr) & t < t_p \\ R_p + i_p\,k'_2\Bigl(1 - \exp\bigl(\frac{t_p - t}{60\,k'_2}\bigr)\Bigr) & t_p \le t < 60\,dur \end{cases}
```

dengan R\_p = `pkrain`, i\_p = `pkrr`, k'\_1 = `xkp1`, k'\_2 = `xkp2`, t\_p = `rtp` (menit).

- **Kebiasaan SWAT+ / catatan port:** (a) `pt` disimpan sebagai bilangan bulat dan dimajukan dengan `int(pt + dtm)`, meniru variabel integer Fortran. (b) Karena benih di-set 8 setiap kali, setiap badai berpuncak di posisi relatif yang sama. (c) Badai mulai tengah malam dan memakai fraksi setengah jam kemarin (yang hari ini baru dibangkitkan sesudahnya). (d) Untuk stasiun `sim`, hari ini dinolkan sebelum hujan disebar, dan hujan besok diambil dari jumlah langkah hari ini: akibatnya stasiun yang dibangkitkan **tidak mendapat hujan sama sekali** dalam run sub-harian (`docs/subdaily.md`). (e) Komentar dokumentasi di atas fungsi ini dalam kode sumber juga memuat kalimat untuk `cli_pgen` (salah tempat, tidak berpengaruh).

#### 6.4.3  `Weather::precip_control`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `pub fn precip_control(&mut self, istart: i32, clock: &Clock) -> Result<(), String>`
- **Padanan SWAT+:** `cli_precip_control.f90`.
- **Tujuan:** menggeser hujan esok menjadi hujan hari ini, lalu membaca atau membangkitkan hujan esok untuk setiap stasiun.
- **Masukan dan keluaran:** `istart` (0 sebelum hari pertama, dipanggil dari `run_with`; 1 sesudahnya, dari `climate_control`), `clock` (hari, tahun ke-n `yrs`, `day_end_yr`, `step`). Mengubah `precip`, `precip_next`, `ts`, `ts_next`.
- **Cara kerja:**
  1. Untuk setiap stasiun: `precip = precip_next`, `precip_next = 0`, `ts = ts_next`. Pada run sub-harian dengan `ts_next` kosong (file harian), alokasikan `step` nol.
  2. **Sumber `sim`:** `pgen`; bila `step > 1`, `pgenhr` lalu `precip_next = Σ ts`.
  3. **Sumber gage:** `cur_day = day + istart`; `yrs_to_start = yrs - yrs_start`; bila `cur_day` melewati akhir tahun, `cur_day = 1` dan tahun +1.
  4. Gage sub-harian (`tstep > 0`): ambil langkah `tss(:, cur_day, yrs)` dari **kolom tahun simulasi berjalan**; salin ke `ts_next` sampai dan termasuk celah pertama (-99) bila ada; `precip_next` = jumlah semua langkah. Bila ada celah: `pgen`, `pgenhr`, lalu `precip_next` dikembalikan ke jumlah file.
  5. Gage harian: di luar batas bila `out_of_bounds` atau `yrs_to_start` melebihi rentang tahun file; nilai -98 bila di luar, selain itu `ts(cur_day, yrs_to_start)`. Bila <= -97, `pgen`.
- **Kebiasaan SWAT+ / catatan port:** (a) Pada gage sub-harian, langkah diambil dari kolom tahun `time%yrs` apa pun harinya, sehingga pada hari terakhir tahun, 1 Januari tahun berikutnya mendapat jam-jam 1 Januari tahun ini. (b) Pemeriksaan batas tidak dipakai di jalur sub-harian (Fortran mengisi -98 lalu langsung menimpanya). (c) Pada celah, total harian esok tetap jumlah file termasuk -99: negatif, sehingga hari itu tanpa hujan setelah penyesuaian (lantai 0). (d) Akumulator diagnostik `precip_aa`/`pet_aa` di Fortran tidak dipindahkan ke port karena tidak memengaruhi keluaran yang divalidasi.

### 6.5  Suhu, radiasi, kelembapan, dan angin

#### 6.5.1  `Weather::weatgn`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn weatgn(&mut self, iwgn: usize)`
- **Padanan SWAT+:** `cli_weatgn.f90`.
- **Tujuan:** membangkitkan tiga deviat normal harian berkorelasi silang dan berkorelasi seri (suhu maksimum, suhu minimum, radiasi) dengan model autoregresif multivariat orde satu (Matalas 1967; Richardson 1981).
- **Masukan dan keluaran:** benih `idg(8)`, `idg(9)`, `idg(2)`; `rnd8`, `rnd9`, `rnd2`; `wgnold[3]`. Keluaran `wgncur[3]` (tanpa satuan, dibatasi \[-1, 1\]).
- **Cara kerja:**
  1. Tiga deviat N(0,1) independen: `e1 = dstn1(rnd8, u)` dengan `u` dari `idg(8)`; `e2` dari `rnd9` dan `idg(9)`; `e3` dari `rnd2` dan `idg(2)`; tiap `rnd*` diganti tarikan baru.
  2. Gabungkan dengan matriks A (korelasi seri) dan B (korelasi silang), lalu clamp setiap komponen ke \[-1, 1\] dan simpan sebagai `wgnold`.
- **Persamaan:**

```latex
\chi_t = A\,\chi_{t-1} + B\,\varepsilon_t,\quad A=\begin{pmatrix}0.567&0.086&-0.002\\0.253&0.504&-0.050\\-0.006&-0.039&0.244\end{pmatrix},\quad B=\begin{pmatrix}0.781&0&0\\0.328&0.637&0\\0.238&-0.341&0.873\end{pmatrix}
```

- **Kebiasaan SWAT+ / catatan port:** penjumlahan dilakukan per baris: suku B·ε dijumlahkan ke `wgncur` dan suku A·χ ke `xx` dalam loop yang sama, kemudian `wgncur += xx`; urutan ini dipertahankan untuk pembulatan f32 yang sama. Pemotongan ke \[-1, 1\] adalah perilaku SWAT+. `weatgn` dipanggil sekali per stasiun per hari di loop suhu (bahkan bila suhu terukur), dan sekali lagi bila suhu terukur hilang.

#### 6.5.2  `Weather::tgen`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn tgen(&mut self, ist: usize, mo: usize)`
- **Padanan SWAT+:** `cli_tgen.f90`.
- **Tujuan:** suhu maksimum dan minimum harian (°C) dari deviat `wgncur`.
- **Cara kerja:**
  1. Amplitudo `tamp = 0.5·(μ_mx - μ_mn)`.
  2. Rerata suhu maksimum disesuaikan agar hari kering lebih panas dan hari basah lebih dingin: `txxm = μ_mx + tamp·pr_wdays`, dikurangi `tamp` bila hari ini hujan.
  3. `tmax = txxm + σ_mx·χ_1`, `tmin = μ_mn + σ_mn·χ_2`.
  4. Bila `tmin > tmax`: `tmin = tmax - 0.2·|tmax|`.
- **Persamaan:**

```latex
T_{mx} = \mu_{mx} + \tfrac12(\mu_{mx}-\mu_{mn})\,(p_w - \delta_{wet}) + \sigma_{mx}\,\chi_1,\qquad T_{mn} = \mu_{mn} + \sigma_{mn}\,\chi_2
```

dengan p\_w = `pr_wdays` dan δ\_wet = 1 bila hujan hari ini > 0.

- **Kebiasaan SWAT+ / catatan port:** suhu minimum tidak disesuaikan menurut basah/kering.

#### 6.5.3  `Weather::clgen`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn clgen(&mut self, ist: usize, day: i32)`
- **Padanan SWAT+:** `cli_clgen.f90`.
- **Tujuan:** memperbarui status basah hari ini (`prior_wet`), menghitung panjang hari (jam) dan radiasi matahari maksimum (MJ/m²/hari).
- **Cara kerja:**
  1. `prior_wet = precip >= 0.1` mm.
  2. Deklinasi matahari `sd`, koreksi jarak bumi–matahari `dd`, sudut jam terbit `h` (0 bila matahari tidak terbit, 3.1416 bila tidak terbenam).
  3. Panjang hari `7.6394·h`; radiasi maksimum dari radiasi ekstraterestrial dengan konstanta 30 MJ/m².
- **Persamaan:**

```latex
\delta = \arcsin\Bigl(0.4\,\sin\frac{J-82}{58.09}\Bigr),\quad d_r = 1 + 0.033\cos\frac{J}{58.09},\quad h = \arccos(-\tan\phi\tan\delta)
```

```latex
T_L = 7.6394\,h,\qquad H_{mx} = 30\,d_r\,\bigl(h\,\sin\phi\sin\delta + \cos\phi\cos\delta\,\sin h\bigr)
```

- **Kebiasaan SWAT+ / catatan port:** π ditulis 3.1416; argumen `-sinφ·tanδ/cosφ` dihitung dalam urutan Fortran. `clgen` dipanggil di loop radiasi untuk setiap stasiun, sebelum sumber radiasi diperiksa.

#### 6.5.4  `Weather::slrgen`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn slrgen(&mut self, ist: usize, mo: usize)`
- **Padanan SWAT+:** `cli_slrgen.f90`.
- **Tujuan:** radiasi matahari harian yang dibangkitkan (MJ/m²/hari).
- **Cara kerja:** `solrad = slr_ave(mo)`.
- **Kebiasaan SWAT+ / catatan port:** Fortran menghitung nilai stokastik (rerata disesuaikan hari basah, `rav = slr_ave/(1 - 0.5·p_w)`, separuh bila hujan, ditambah `χ_3·(H_mx - rav)/4`, minimal 0.05·H\_mx), lalu baris terakhirnya **menimpanya dengan rerata bulanan**. Port hanya menulis hasil akhirnya; tidak ada tarikan acak yang hilang karena χ\_3 sudah ditarik di `weatgn`. Akibatnya radiasi yang dibangkitkan konstan sepanjang bulan.

#### 6.5.5  `Weather::rhgen` dan `ee`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn rhgen(&mut self, ist: usize, mo: usize)`; `pub fn ee(tk: f32) -> f32`
- **Padanan SWAT+:** `cli_rhgen.f90`; fungsi `Ee` di `ee.f90`.
- **Tujuan:** `ee` memberi tekanan uap jenuh (kPa) pada suhu °C; `rhgen` membangkitkan kelembapan relatif harian (fraksi).
- **Cara kerja:**
  1. Kelembapan rerata bulanan `rhmo`: bila kolom titik embun berisi suhu, `ee(T_dew)/ee(T_mean)`; bila berisi kelembapan relatif (`dew_is_rhum`), nilai itu sendiri.
  2. Sesuaikan untuk hari kering: `yy = 0.9·p_w`, `rhm = (rhmo - yy)/(1 - yy)`; bila < 0.05, `rhm = 0.5·rhmo`.
  3. Hari hujan: `rhm = 0.1·rhm + 0.9`.
  4. Batas segitiga `uplm = rhm - (rhm-1)·e^{rhm-1}`, `blm = rhm·(1 - e^{-rhm})`; tarik `atri(blm, rhm, uplm)` dengan benih `idg(7)`.
- **Persamaan:**

```latex
e^{o}(T) = \exp\Bigl(\frac{16.78\,T - 116.9}{T + 237.3}\Bigr)\ \text{kPa}
```

- **Kebiasaan SWAT+ / catatan port:** `ee` mengembalikan 0 bila `T + 237.3 = 0`. Fungsi `ee` juga dipakai HRU untuk PET (6.7).

#### 6.5.6  `Weather::wndgen`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn wndgen(&mut self, ist: usize, mo: usize)`
- **Padanan SWAT+:** `cli_wndgen.f90`.
- **Tujuan:** kecepatan angin harian (m/s) dari distribusi eksponensial termodifikasi.
- **Cara kerja:** tarik `v6` dari benih `idg(5)`; lalu persamaan di bawah.
- **Persamaan:**

```latex
u = \mu_{wnd}\,\bigl(-\ln v_6\bigr)^{0.3}
```

- **Kebiasaan SWAT+ / catatan port:** cabang arah angin Fortran (`iwndir`) selalu 0, sehingga tidak ada tarikan kedua; port menghilangkannya.

#### 6.5.7  `Weather::measured`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `fn measured(&self, list: &[Gage], ig: usize, clock: &Clock) -> f32`
- **Padanan SWAT+:** pola `cli_bounds_check` + `ts(time%day, yrs_to_start)` di `climate_control.f90` untuk radiasi, kelembapan, dan angin.
- **Tujuan:** nilai terukur hari ini, atau -98 bila di luar rentang file.
- **Cara kerja:** ambil gage (default kosong); bila `out_of_bounds`, -98; selain itu `value(day, yrs - yrs_start)`.

### 6.6  Urutan harian: `climate_control`

#### 6.6.1  `Weather::climate_control`

- **Lokasi:** `crates/swat-engine/src/climate.rs`, `pub fn climate_control(&mut self, clock: &Clock) -> Result<(), String>`
- **Padanan SWAT+:** `climate_control.f90`.
- **Tujuan:** menghasilkan satu hari cuaca lengkap di setiap stasiun, dalam urutan loop dan urutan tarikan acak yang sama dengan SWAT+.
- **Masukan dan keluaran:** `clock` (hari, bulan, tahun ke-n, `end_yr`); mengisi seluruh field `Weat` setiap stasiun dan memperbarui jendela P/PET generator.
- **Cara kerja (setiap langkah adalah loop terpisah atas semua stasiun):**
  1. **Hujan:** `precip_control(1)`.
  2. **Suhu:** `weatgn(iwgn)` selalu dipanggil dulu. Sumber `sim`: `tgen`. Sumber gage: bila di luar batas, `tmax = tmin = -98`; selain itu `ts` dan `ts2`. Bila salah satunya <= -97, `weatgn` dipanggil **lagi** lalu `tgen`. Kemudian `tave = (tmax + tmin)/2`.
  3. **Radiasi:** `clgen` (status basah, panjang hari, radiasi maksimum), lalu `slrgen` atau nilai terukur (`measured`), dengan `slrgen` bila <= -97.
  4. **Kelembapan:** `rhgen` atau terukur (dengan `rhgen` bila hilang); titik embun `dewpt = tave - (1 - rhum)/5`.
  5. **Angin:** `wndgen` atau terukur (dengan `wndgen` bila hilang).
  6. **PET stasiun dan indeks kelembapan:** `ppet_mce` maju melingkar 1..30. Untuk setiap stasiun dihitung PET Hargreaves (persamaan di bawah), minimal 0.01 mm (0.01 bila `tmax <= tmin`). Bila PET > 0.1, `ppet += P/PET`. Jendela 30 hari generator diperbarui: jumlah bergerak `precip_sum += P - precip_mce[k]`, `pet_sum += PET - pet_mce[k]`, `p_pet_rto = precip_sum / max(pet_sum, 1e-6)` (pembagi 1e-6 bila `pet_sum <= 1e-6`), lalu slot k diganti nilai hari ini.
  7. **Lag suhu udara:** `tlag[tlag_mne] = tave`, `tlag_mne` maju melingkar 1..6, `airlag_temp = tlag[tlag_mne]` (suhu rerata enam hari sebelumnya; dipakai suhu air sungai).
  8. **Fraksi hujan setengah jam:** batas bawah 0.02083 (= 1/48, hujan merata), batas atas `1 - exp(-125/(P + 5))`, modus `amp_r(mo)`; `precip_half_hr = atri(...)` dengan benih ke-10, **setiap hari** termasuk hari kering.
  9. **Satuan panas basis nol:** bila `phutot > 0` dan `tave > 0`, `phubase0 += tave/phutot`; bila `phutot <= 0`, `phubase0 = 0`; direset 0 pada hari terakhir tahun.
  10. **Penyesuaian bulanan dan batas:** hujan dikali `(1 + rfinc/100)` dan dilantai 0 (juga setiap langkah `ts`, sehingga celah -99 menjadi 0); suhu ditambah `tmpinc`; radiasi ditambah `radinc` dan dilantai 0; kelembapan ditambah `huminc` dan dibatasi \[0.01, 0.99\].
- **Persamaan PET Hargreaves stasiun:**

```latex
R_a = H_{mx}\,\frac{37.59}{30},\quad \lambda = 2.501 - 2.361\times10^{-3}\,\bar T,\quad PET = \max\Bigl(0.01,\ 0.0023\,\frac{R_a}{\lambda}\,(\bar T + 17.8)\,(T_{mx}-T_{mn})^{0.5}\Bigr)
```

- **Persamaan jendela kelembapan:**

```latex
\frac{P}{PET}\Big|_{30} = \frac{\sum_{i=t-29}^{t} P_i}{\sum_{i=t-29}^{t} PET_i}
```

- **Kebiasaan SWAT+ / catatan port:** (a) Penyesuaian `rfinc`, `tmpinc`, `radinc`, `huminc` ditulis sebagai nol tetap di port (`* (1 + 0/100)`, `+= 0`): di SWAT+ nilainya hanya diisi kalibrasi, yang belum didukung jalur ini; operasi lantai/batas tetap dijalankan. (b) PET terukur (`pet.cli`, `cli_petmeas`) tidak diporting di sini: Fortran membacanya ke `weat%pet` lalu langkah 6 menimpanya dengan Hargreaves untuk setiap stasiun, sehingga nilai terukur tidak pernah dipakai, bahkan oleh HRU dengan `pet` = 3 (`docs/engine1.md`, skenario Ps\_pet). (c) PET stasiun dihitung dari suhu stasiun yang belum dikoreksi *lapse rate*, dan dipakai juga oleh *revap* akuifer apa pun metode PET HRU (`docs/swat-quirks.md`). (d) `(T_mx - T_mn)^{0.5}` memakai `rpow`. (e) Jendela `precip_mce`/`pet_mce` adalah milik generator, bukan stasiun: bila beberapa stasiun berbagi satu generator, masing-masing memperbarui slot yang sama secara berurutan. (f) Karena setiap variabel diproses dalam loop terpisah, urutan tarikan adalah: semua suhu, semua radiasi, semua kelembapan, semua angin, lalu semua fraksi setengah jam.

### 6.7  Evapotranspirasi potensial HRU dan koreksi elevasi

Setelah `climate_control`, `run_with` menyusun `HruWeather` setiap HRU dari stasiunnya (`precip`, `ts`, `tmax`, `tmin`, `tave`, `solrad`, `solradmx`, `windsp`, `rhum`, `phubase0`, `station_pet`, `atmo`). Bila `lapse` = 1, hujan ditambah `plaps` (dilantai 0) dan ketiga suhu ditambah `tlaps`. Di fase lahan, HRU menghitung `ea = ee(tave)`, `ed = ea·rhum`, `vpd = ea - ed` (kPa), memilih metode menurut `codes.bsn` `pet` (0 Priestley-Taylor, 1 Penman-Monteith, 2 Hargreaves, lainnya PET stasiun), lalu mengalikan dengan `pet_co` HRU. Fungsi-fungsi ini berada di `crates/swat-engine/src/hru/mod.rs` dan merupakan padanan `et_pot.f90`.

#### 6.7.1  `lapse_rates`

- **Lokasi:** `crates/swat-engine/src/run.rs`, `fn lapse_rates(project, weather: &Weather, ist: usize, elev: f32) -> (f32, f32)`
- **Padanan SWAT+:** `cli_lapse.f90`.
- **Tujuan:** koreksi hujan (mm/hari) dan suhu (°C) untuk objek pada elevasi `elev` relatif terhadap elevasi sumber datanya.
- **Masukan dan keluaran:** `plaps` (mm/km) dan `tlaps` (°C/km) dari `parameters.bsn`; elevasi generator (`wgn.elev`) untuk sumber `sim` atau elevasi gage (`pcp`/`tmp` `elev`) untuk sumber terukur. Keluaran `(plaps_obj, tlaps_obj)`.
- **Persamaan:**

```latex
\Delta P = p_{laps}\,\frac{z_{obj} - z_{src}}{1000},\qquad \Delta T = t_{laps}\,\frac{z_{src} - z_{obj}}{1000}
```

- **Kebiasaan SWAT+ / catatan port:** (a) koreksi hujan ditambahkan setiap hari, termasuk hari kering, lalu dilantai 0 (jadi `plaps` positif membuat hujan setiap hari di HRU yang lebih tinggi). (b) Sumber yang tidak ditemukan (`Gage(0)`) memakai elevasi gage kosong, 0 m. (c) Port menghitungnya sekali saat menyusun daftar HRU; SWAT+ menyimpannya di `ob%plaps`/`ob%tlaps`. (d) Radiasi maksimum, kelembapan, dan PET stasiun tidak dikoreksi.

#### 6.7.2  `Hru::pet_constants`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn pet_constants(&self, w: &HruWeather, ea: f32) -> (f32, f32, f32)`
- **Padanan SWAT+:** bagian awal `et_pot.f90`.
- **Tujuan:** tekanan barometrik, panas laten penguapan, konstanta psikrometrik, dan kemiringan kurva tekanan uap jenuh.
- **Persamaan:**

```latex
P_b = 101.3 - z\,(0.01152 - 0.544\times10^{-6}\,z),\quad \lambda = 2.501 - 2.361\times10^{-3}\,\bar T,\quad \gamma = \frac{1.013\times10^{-3}\,P_b}{0.622\,\lambda},\quad \Delta = \frac{4098\,e^o}{(\bar T + 237.3)^2}
```

dengan z = elevasi HRU (m), P\_b kPa, λ MJ/kg, γ dan Δ kPa/°C.

#### 6.7.3  `Hru::pet_radiation`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn pet_radiation(&self, w: &HruWeather, ed: f32) -> (f32, f32)`
- **Padanan SWAT+:** perhitungan radiasi netto di `et_pot.f90`.
- **Tujuan:** radiasi gelombang pendek netto tanaman acuan (`ralb`) dan radiasi gelombang panjang netto (`rout`), MJ/m²/hari.
- **Cara kerja:** albedo 0.23, atau 0.8 bila salju > 0.5 mm; emisivitas netto dari `ed`; faktor awan dari rasio radiasi aktual/maksimum (0 bila `solradmx < 1e-4`).
- **Persamaan:**

```latex
R_{ns} = H_{day}(1-\alpha),\quad R_{nl} = -\bigl(0.34 - 0.139\sqrt{e_d}\bigr)\Bigl(0.9\frac{H_{day}}{H_{mx}} + 0.1\Bigr)\,4.9\times10^{-9}\,T_K^4
```

- **Kebiasaan SWAT+ / catatan port:** `T_K^4` dihitung sebagai `(tk·tk)·(tk·tk)`, sebagaimana gfortran mengalikan untuk pangkat bulat 4, dengan `tk = tave + 273.15`.

#### 6.7.4  `Hru::pet_priestley_taylor`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn pet_priestley_taylor(&self, w, ea, ed) -> f32`
- **Padanan SWAT+:** `et_pot.f90`, kasus `pet` = 0.
- **Tujuan:** PET Priestley-Taylor (mm/hari).
- **Persamaan:**

```latex
PET = \max\Bigl(0,\ 1.28\,\frac{\Delta}{\Delta+\gamma}\,\frac{R_{ns}+R_{nl}}{\lambda}\Bigr)
```

- **Kebiasaan SWAT+ / catatan port:** sudah diporting tetapi belum divalidasi terhadap skenario uji (`docs/phase3.md`). Fluks panas tanah diabaikan.

#### 6.7.5  `Hru::pet_penman_monteith`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn pet_penman_monteith(&self, w, albday: f32, ea, ed, vpd, b: &HruBasin) -> (f32, f32)`
- **Padanan SWAT+:** `et_pot.f90`, kasus `pet` = 1.
- **Tujuan:** PET Penman-Monteith untuk tanaman acuan alfalfa (tinggi 40 cm), dan transpirasi tanaman maksimum `ep_max` untuk kanopi HRU (mm/hari).
- **Masukan dan keluaran:** `windsp` (m/s, sudah dinaikkan minimal 0.01 oleh `run_with`), `albday` (albedo hari ini), CO₂ cekungan (ppmv), tinggi kanopi `cht_mx`, LAI tanaman, konduktansi stomata maksimum `gsi` dan parameter VPD `vpd2` tanaman. Keluaran `(pet_day, ep_max)`.
- **Cara kerja:**
  1. `pet_constants` dan `pet_radiation`; radiasi netto acuan `rn_pet = ralb + rout`, radiasi netto kanopi `rn = H(1 - albday) + rout`; `ρ = 1710 - 6.85·T`.
  2. Tahanan aerodinamik acuan `rv = 114/(u·(170/1000)^{0.2})` (angin dikoreksi ke 1.7 m) dan tahanan kanopi acuan `rc = 49/(1.4 - 0.4·CO2/330)`; PET acuan dari persamaan Penman-Monteith.
  3. Bila tidak ada tanaman tumbuh, `ep_max = 0`.
  4. Bila ada: tinggi ukur `zz` (170 cm, atau `cht_mx·100 + 100` cm bila kanopi > 1 m), angin pada `zz`, tinggi kanopi `chz` (cm), kekasaran `zom = 0.123·chz` (atau `0.058·chz^{1.19}` bila > 200 cm), `zov = 0.1·zom`, perpindahan nol `d = 0.667·chz`; tahanan aerodinamik log-profil.
  5. Konduktansi stomata disesuaikan VPD: `fvpd = max(0.1, 1 - vpd2·(vpd - 1))` bila vpd > 1 kPa; tahanan kanopi dari konduktansi, LAI total, dan CO₂; `ep_max` dari Penman-Monteith dibatasi \[0, PET\].
- **Persamaan:**

```latex
\lambda E = \frac{\Delta\,R_n + \gamma\,\rho\,D/r_a}{\Delta + \gamma\,(1 + r_c/r_a)},\qquad r_a = \frac{\ln\frac{z-d}{z_{om}}\,\ln\frac{z-d}{z_{ov}}}{0.41^2\,u_z},\qquad r_c = \frac{1}{g_l\;0.5\,LAI\,(1.4 - 0.4\,CO_2/330)}
```

dengan D = `vpd` (kPa) dan g\_l = konduktansi daun (+0.01).

- **Kebiasaan SWAT+ / catatan port:** (a) SWAT+ memakai konduktansi tanaman **terakhir** dalam loop (`gsi_adj`), bukan rerata tertimbang LAI (`gsi_wav`) yang baru saja dijumlahkan; dengan `--fix-swat` (`b.fixes`) dipakai rerata tertimbang. (b) Kenaikan angin ke 0.01 m/s dilakukan pada cuaca HRU dan ikut tercetak di `hru_pw_day` (`wndspd`). (c) `ep_max` yang dihitung di sini ditimpa `et_act` dengan bagian LAI dari PET, kecuali pada hari tanpa PET tersisa setelah kanopi (`docs/swat-quirks.md`). (d) Pangkat 0.2 dan 1.19 memakai `rpow`. (e) CO₂ adalah `co2` cekungan, tetap tiap tahun tanpa `co2_yr.dat`.

#### 6.7.6  `pet_hargreaves`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn pet_hargreaves(w: &HruWeather) -> f32`
- **Padanan SWAT+:** `et_pot.f90`, kasus `pet` = 2.
- **Tujuan:** PET Hargreaves untuk HRU (mm/hari), sebelum dikalikan `pet_co`.
- **Persamaan:** sama dengan PET stasiun (6.6.1), tetapi dengan batas bawah 0 (dan 0 bila `tmax <= tmin`), serta memakai suhu HRU yang sudah dikoreksi *lapse rate*.
- **Kebiasaan SWAT+ / catatan port:** perbedaan batas bawah (0 di HRU, 0.01 di stasiun) adalah perbedaan nyata di SWAT+ dan dipertahankan. Hargreaves digunakan untuk Ames\_sub1 dan Bh\_land; satu bit perbedaan dari `x ** 0.5` adalah alasan dibuatnya `rpow`.

### 6.8  Deposisi nitrogen atmosfer (`atmo.cli`)

Deposisi atmosfer tidak berada di `climate.rs`, tetapi melekat pada stasiun cuaca: setiap stasiun `weather-sta.cli` menunjuk satu stasiun deposisi melalui kolom `atmo_dep`. Logikanya tersebar di `crates/swat-engine/src/run.rs` (blok di `run_with`, bukan fungsi bernama) dan `crates/swat-engine/src/hru/mod.rs` (blok `nut_nrain` di fase nutrien HRU). Rinciannya juga ada di `docs/atmo.md`.

#### 6.8.1  Inisialisasi deposisi (blok di `run_with`)

- **Lokasi:** `crates/swat-engine/src/run.rs`, blok sesudah pembuatan `Saved` di `run_with`.
- **Padanan SWAT+:** `cli_read_atmodep.f90`.
- **Tujuan:** mencari stasiun deposisi setiap stasiun cuaca dan menentukan langkah waktu data untuk hari pertama.
- **Cara kerja:**
  1. `atmo_of[ist]` = posisi nama `atmo_dep` di daftar stasiun deposisi, via `search` (0 bila tidak ditemukan).
  2. Langkah waktu file: `aa` (satu nilai rerata tahunan), `yr` (satu nilai per tahun), `mo` (satu nilai per bulan).
  3. Untuk `yr`: `atmo_ts` = indeks tahun data yang sama dengan tahun awal simulasi. Untuk `mo`: telusuri bulan-bulan sejak `yr_init`/`mo_init` sampai cocok dengan tahun dan bulan awal; `atmo_ts = 12·(yrc0 - yr_init) + imo`. Bila ditemukan, `atmo_first = false`.

#### 6.8.2  Kontrol waktu deposisi dan nilai harian (blok `atmo_day` di `run_with`)

- **Lokasi:** `crates/swat-engine/src/run.rs`, di dalam loop hari; closure `atmo_day(ist) -> Option<[f32; 4]>`.
- **Padanan SWAT+:** `cli_atmodep_time_control.f90`.
- **Tujuan:** memajukan indeks waktu data deposisi dan memberi setiap HRU nilai harian `[no3_rf, no3_dry, nh4_rf, nh4_dry]`.
- **Cara kerja:**
  1. Bila data belum mulai (`atmo_first`), `atmo_ts = 1` pada tahun (dan bulan) pertama data.
  2. Selain itu, `atmo_ts += 1` pada hari terakhir tahun (`yr`) atau hari terakhir bulan (`mo`).
  3. Nilai harian: konsentrasi dalam hujan (mg/L) apa adanya; deposisi kering (kg/ha per periode) dibagi 365 untuk `aa`/`yr`, atau jumlah hari bulan untuk `mo`.
  4. Di luar rentang data (`atmo_ts` <= 0 atau > `num`), `None`.
- **Kebiasaan SWAT+ / catatan port:** penanda akhir bulan/tahun sudah di-set sebelum kontrol waktu dijalankan, sehingga **hari terakhir setiap bulan (atau tahun) sudah memakai deposisi periode berikutnya**. Kode `atmo_dep` di `codes.bsn` tidak dipakai; keberadaan file yang menentukan.

#### 6.8.3  Penambahan ke tanah (blok `nut_nrain` di HRU)

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, blok bertanda `nut_nrain` di fase nutrien harian.
- **Padanan SWAT+:** `nut_nrain.f90`.
- **Tujuan:** menambahkan nitrat dan amonium dari hujan dan deposisi kering ke lapisan tanah teratas (kg N/ha).
- **Persamaan:**

```latex
NO_3^{atmo} = 0.01\,C_{NO_3,rf}\,P + D_{NO_3,dry},\qquad NH_4^{atmo} = 0.01\,C_{NH_4,rf}\,P + D_{NH_4,dry}
```

dengan C dalam mg N/L, P hujan HRU (mm, sesudah *lapse rate*), D dalam kg/ha/hari; faktor 0.01 mengubah mg/L·mm menjadi kg/ha.

- **Kebiasaan SWAT+ / catatan port:** di luar data deposisi tidak ada yang ditambahkan, tetapi kolom `no3atmo`/`nh4atmo` di `hru_nb` mencetak ulang nilai hari terakhir (`hnb_d` tidak dinolkan); port meniru dengan `atmo_prev`. Skenario Ps\_atmo memvalidasi ketiga bentuk langkah waktu.
