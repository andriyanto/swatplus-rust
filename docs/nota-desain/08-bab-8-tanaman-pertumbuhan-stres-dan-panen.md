## Bab 8  Tanaman: pertumbuhan, stres, dan panen

Bab ini membahas model tanaman SWAT+ sebagaimana diporting ke Rust: pembacaan dan pelengkapan basis data tanaman (`plants.plt`, padanan `plant_parm_read` dan `plantparm_init`), inisialisasi tanaman pada awal simulasi (`plant_init`), lalu siklus harian komunitas tanaman (`pl_community`, `pl_waterup`, `pl_grow` dan semua rutin yang dipanggilnya): akumulasi *heat unit*, dormansi, biomassa dari radiasi yang diintersepsi, stres suhu/air/aerasi/N/P, penyerapan air dan nutrien menurut kedalaman akar, pertumbuhan akar, daun dan biji, partisi biomassa ke organ, senesens daun, dan kematian biomassa berlebih di akhir tahun. Persamaannya berakar pada model pertumbuhan tanaman EPIC (Williams dkk.) yang diadopsi SWAT (Neitsch dkk. 2009, bab 5:1–5:3), dengan sejumlah perubahan khas SWAT+.

File Rust utama adalah `crates/swat-engine/src/hru/plant.rs` (31 fungsi, seluruhnya didokumentasikan di sini). Karena tugas bab ini juga mencakup padanan `mgt_harv*`, subbab 8.7 mendokumentasikan fungsi panen, pematian dan penanaman yang tinggal di `crates/swat-engine/src/hru/mgt.rs` (`HarvOp::new`, `harvest`, `harvbiomass`, `harvtuber`, `harvresidue`, `harvgrain`, `killop`, `plantop`), dan fungsi bantu `ascrv` dari `crates/swat-engine/src/hru/mod.rs`. Penjadwalan operasi (`mgt_sched`, tabel keputusan) dibahas di bab manajemen; di sini hanya efeknya pada tanaman. Perilaku yang dicatat di `docs/engine1.md` (bagian *Harvests and kills* dan *Plants: days to maturity and population*), `docs/swat-quirks.md` (bagian *Plants*, *Plant communities*, *Management*) dan `docs/swat-fixes.md` dirujuk pada entri yang relevan.

Posisi dalam alur harian HRU (`Hru::day` di `hru/mod.rs`): pada awal hari semua stres tanaman di-*reset* ke 1 (padanan baris di `hru_control.f90`), lalu tabel keputusan dan operasi terjadwal berjalan, evapotranspirasi menghitung `ep_max` (transpirasi maksimum), kemudian setelah siklus nutrien tanah (mineralisasi, nitrifikasi, mineralisasi P, *biozone* septik) dipanggil `pl_community` lalu `pl_grow`. Hasilnya (LAI, biomassa, residu, stres rata-rata) dipakai keesokan harinya oleh albedo, intersepsi kanopi, ET, faktor C USLE dan keluaran harian. Dengan demikian, tanaman selalu tumbuh dengan air dan nutrien tanah yang sudah dikurangi oleh proses lain pada hari yang sama.

### 8.1  Struktur data, aritmetika massa organik, dan fungsi bantu

Model tanaman SWAT+ menyimpan keadaan dalam beberapa array modul Fortran (`pcom(j)%plcur`, `plg`, `plm`, `plstr`, `pl_mass(j)`). Port Rust menyatukannya per tanaman dalam struct `Plant` dan memisahkan konstanta basin, variabel harian bersama, dan variabel `SAVE` implisit Fortran.

**`OrgMass`** — massa organik (padanan tipe `organic_mass` di `organic_mineral_mass_module`), semuanya kg/ha:

| Field | Arti | Satuan |
| --- | --- | --- |
| `m` | massa kering | kg/ha |
| `c` | karbon | kg C/ha |
| `n` | nitrogen | kg N/ha |
| `p` | fosfor | kg P/ha |

**`PlantDb`** — satu baris `plants.plt` (padanan `pldb`/`plant_db`) setelah `plant_parm_read` dan `plantparm_init`, ditambah parameter bentuk kurva yang diturunkan (`plant_cp`). Field penting:

| Field | Arti (satuan) |
| --- | --- |
| `name`, `typ`, `trig` | nama, tipe (`perennial`, `warm_annual`, `cold_annual`, `warm_annual_tuber`, `cold_annual_tuber`, `null`…), pemicu tumbuh (`temp_gro` atau berbasis kelembapan) |
| `days_mat` | hari menuju kematangan (0 = heat unit sepanjang tahun) |
| `bio_e` | efisiensi penggunaan radiasi RUE ((kg/ha)/(MJ/m²)) |
| `hvsti` | indeks panen optimal (–) |
| `blai`, `dlai`, `dlai_rate` | LAI maksimum potensial, fraksi heat unit saat LAI mulai turun, laju penurunan |
| `chtmx`, `rdmx` | tinggi kanopi maksimum (m), kedalaman akar maksimum (m) |
| `t_opt`, `t_base` | suhu optimum dan dasar pertumbuhan (°C) |
| `cnyld`, `cpyld` | fraksi N dan P dalam hasil/biji (kg/kg) |
| `wsyf` | batas bawah indeks panen pada stres air (–) |
| `pltnfr1/3`, `pltpfr1/3` | fraksi N dan P biomassa pada awal dan saat matang (kg/kg) |
| `usle_c` | faktor C minimum USLE (–) |
| `wavp` | penurunan RUE per kPa defisit tekanan uap |
| `rsdco_pl` | koefisien dekomposisi residu tanaman |
| `alai_min`, `laixco_tree`, `mat_yrs`, `bmx_peren` | LAI minimum dormansi/senesens, koefisien LAI pohon muda, tahun menuju matang, biomassa maksimum perennial (t/ha) |
| `ext_coef` | koefisien pemadaman cahaya (–) |
| `leaf_tov_min/max`, `bm_dieoff` | *turnover* daun, fraksi kematian biomassa |
| `rsr1`, `rsr2` | rasio akar/biomassa awal dan akhir musim |
| `aeration` | ambang stres aerasi (fraksi kejenuhan) |
| `gsi`, `vpd2` | konduktansi stomata maksimum (m/s) dan kemiringan penurunannya per VPD (Penman-Monteith) |
| `leaf1`, `leaf2`, `ruc1`, `ruc2`, `nup1`, `nup2`, `pup1`, `pup2` | koefisien kurva-S (hasil `ascrv`) untuk LAI, RUE vs CO₂, serapan N dan P |
| `cvm` | ln(`usle_c`) |
| `fracs` | fraksi metabolik/struktural/lignin residu di atas dan di bawah tanah (`PartFracs`, model karbon) |

**`Plant`** — tanaman yang sedang tumbuh di HRU, gabungan `pcom(j)%plcur(ipl)`, `plg`, `plm`, `plstr` dan `pl_mass(j)` untuk tanaman itu: `db` (salinan `PlantDb`), `cpnm` (nama di `plant.ini`, dirujuk operasi), `yield_tot` dan `harv_num` (hasil kumulatif dan jumlah panen), `gro` dan `idorm` (sedang tumbuh, dorman), `phuacc`/`phuacc_p` (fraksi heat unit terkumpul musim ini dan seumur perennial), `phumat`/`phumat_p` (heat unit menuju matang, °C·hari), `curyr_mat`, `lai_pot`, `harv_idx`, `epco`, `leaf_tov`; dari `plg`: `lai`, `olai` (LAI saat penurunan dimulai), `dphu`, `d_senes`, `cht` (m), `laimxfr`, `root_dep` (mm), `root_frac`, `rtfr` (fraksi akar per lapisan), `hi_adj`, `hi_prev`, `plet`, `plpet` (mm); dari `plm`: `n_fr`, `p_fr`; dari `plstr`: `strsw`, `strst`, `strsn`, `strsp`, `strsa`, `strss` (faktor stres 0–1, 1 = tanpa stres), `reg`, dan akumulator musiman `sum_w`, `sum_tmp`, `sum_n`, `sum_p`, `sum_a` untuk `mgt_out.txt`; dari `pl_mass`: `tot`, `ab_gr`, `leaf`, `stem`, `seed`, `root` (semuanya `OrgMass`), `rsd` (residu permukaan milik tanaman) dan `soil_rsd` (residu per lapisan tanah); serta `par` (radiasi yang diintersepsi hari ini, MJ/m²).

**`PlantBasin`** — nilai basin yang dibaca rutin tanaman: `co2` (ppm), `n_updis`, `p_updis` (parameter distribusi serapan N dan P), `nfixmx` (fiksasi N maksimum, kg/ha/hari), `nostress` (kode `codes.bsn`), `water_norm`, `n_norm`, `p_norm` (penormal distribusi serapan), dan `fixes` (mode `--fix-swat`).

**`PlantDay`** — variabel global `hru_module` untuk satu HRU satu hari: `ep_max`, `ep_day`, `pet_day` (mm), `vpd` (kPa), `nplnt`, `pplnt`, `fixn` (kg/ha, total berjalan HRU), `uno3d`, `uapd`, `un2`, `up2` (permintaan dan kandungan optimal N, P), `uno3d_tot`, `uapd_tot`, `sum_no3`, `sum_solp`, `rto_no3`, `rto_solp`, `bm_grow`, `c_gro`, `nuptake`, `puptake`.

**`PlantWeather`** — cuaca dan kalender: `tave`, `tmin` (°C), `solrad` (MJ/m²), `station_tave` (suhu rata-rata stasiun tanpa penyesuaian), `daylength` dan `daylmn` (jam; panjang hari dan minimum tahunan generator), `dormhr` (ambang dormansi, jam), `tmp_an`, `end_yr`.

**`Saved`** — variabel yang di Fortran bertahan antar-panggilan dan dibagi semua HRU (`SAVE` implisit atau array modul): `rndseed_cond`, `delg_p`, array permintaan per nomor tanaman `uno3d`, `uapd`, `un2`, `up2` (21 elemen, indeks 1-basis; SWAT+ `mpc` = 20), `fr_orgp`, `fr_actmin`, `fr_stamin`, `c_microb_sed`, `enratio`, `septic_opt`. Hanya `delg_p` dan empat array permintaan yang dipakai di bab ini; sisanya milik bab nutrien/sedimen/septik.

#### 8.1.1  `exp_w`

- **Lokasi:** `crates/swat-engine/src/hru/plant.rs`, `pub fn exp_w(y: f32) -> f32`
- **Padanan SWAT+:** fungsi `exp_w` (`exp_w.f90`)
- **Tujuan:** eksponensial yang aman dari *overflow/underflow*.
- **Masukan dan keluaran:** argumen tak berdimensi; hasil tak berdimensi.
- **Cara kerja:**
  1. Bila y < −80, kembalikan 0.
  2. Bila y > 80, kembalikan e⁸⁰.
  3. Selain itu e^y (presisi f32).

```latex
\mathrm{exp\_w}(y)=\begin{cases}0 & y<-80\\ e^{80} & y>80\\ e^{y} & \text{lainnya}\end{cases}
```

- **Kebiasaan SWAT+ / catatan port:** dipakai di penormal dan distribusi serapan P (`p_norm`, `pl_pup`), tetapi tidak untuk N, yang memakai `exp` biasa; port mengikuti pilihan per lokasi agar bit-identik.

#### 8.1.2  `nuts`

- **Lokasi:** `plant.rs`, `fn nuts(u1: f32, u2: f32) -> f32`
- **Padanan SWAT+:** fungsi `nuts` (`nuts.f90`)
- **Tujuan:** faktor pertumbuhan relatif (0–1) dari kandungan unsur aktual `u1` terhadap optimal `u2`; dasar stres N dan P.
- **Masukan dan keluaran:** `u1`, `u2` dalam kg/ha; hasil 0–1.
- **Cara kerja:**
  1. Hitung φ = 200·(u1/(u2+0,0001) − 0,5).
  2. φ ≤ 0 → 0; 0 < φ < 99 → kurva logistik di bawah; φ ≥ 99 → 1.
  3. Bila u2 ≤ 10⁻⁶ (tak ada permintaan), hasil dipaksa 1.

```latex
\varphi = 200\left(\frac{u_1}{u_2+0.0001}-0.5\right),\qquad
\gamma = \frac{\varphi}{\varphi+\exp(3.535-0.02597\,\varphi)}
```

- **Kebiasaan SWAT+ / catatan port:** ini persamaan stres nutrien SWAT 2009 (Neitsch dkk. 2009, bagian 5, pertumbuhan aktual): stres mulai saat kandungan di bawah 100 % optimal dan menjadi penuh pada 50 %.

#### 8.1.3  `ascrv` (di `hru/mod.rs`)

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `pub fn ascrv(x1, x2, x3, x4: f32) -> (f32, f32)`
- **Padanan SWAT+:** `ascrv` (`ascrv.f90`)
- **Tujuan:** menghitung dua koefisien kurva-S f(x) = x / (x + exp(a − b·x)) yang melewati titik (x3, x1) dan (x4, x2). Dipakai untuk kurva LAI, RUE–CO₂, serta fraksi N dan P.
- **Masukan dan keluaran:** x1, x2 nilai kurva; x3, x4 absis; keluaran (a, b) = (`x5`, `x6`).
- **Cara kerja:**

```latex
\xi = \ln\!\left(\frac{x_3}{x_1}-x_3\right),\quad
b = \frac{\xi-\ln\!\left(\frac{x_4}{x_2}-x_4\right)}{x_4-x_3},\quad
a = \xi + x_3\,b
```

- **Kebiasaan SWAT+ / catatan port:** tidak ada pengaman terhadap argumen logaritma ≤ 0; nilai `plants.plt` yang tidak wajar memberi NaN, sama seperti Fortran. Ada uji unit `ascrv_passes_through_its_points` di `mod.rs`.

#### 8.1.4  `PlantBasin::new`

- **Lokasi:** `plant.rs`, `pub fn new(project: &swat_model::Project) -> PlantBasin`
- **Padanan SWAT+:** pembacaan `parameters.bsn`/`codes.bsn` dan `basin_prm_default` (bagian `uptake%water_norm`, `n_norm`, `p_norm`)
- **Tujuan:** mengumpulkan konstanta basin untuk rutin tanaman.
- **Masukan dan keluaran:** `co2` (ppm), `n_updis`, `p_updis`, `nfixmx`, kode `nostress`; keluaran struct dengan penormal.
- **Cara kerja:**
  1. Baca parameter; parameter yang tidak ada bernilai 0.
  2. Hitung penormal distribusi serapan (βw = 10 untuk air, tetap di kode):

```latex
W_{norm}=1-e^{-10},\qquad N_{norm}=1-e^{-\beta_n},\qquad P_{norm}=1-\mathrm{exp\_w}(-\beta_p)
```

3. `fixes` diawali `false`; pemanggil menyetelnya dari `RunOptions::fixes`.

- **Kebiasaan SWAT+ / catatan port:** `water_dis` SWAT+ (10) tidak dibaca dari file; port menuliskan 10 langsung di `pl_waterup` dan di sini.

#### 8.1.5  Fungsi trivial subbab 8.1

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `<OrgMass as Add>::add` | `plant.rs` | penjumlahan komponen demi komponen (m, c, n, p), padanan operator `+` `organic_mineral_mass_module` |
| `<OrgMass as Sub>::sub` | `plant.rs` | pengurangan komponen demi komponen, padanan operator `-` |
| `OrgMass::scale` | `plant.rs` | perkalian konstanta × `OrgMass` (operator `*` Fortran untuk `real * organic_mass`) |
| `<PartFracs as Default>::default` | `plant.rs` | fraksi bawaan `lignin_derived_partition_fracs`: metabolik 0,85, struktural 0,15, lignin 0,12 di atas dan bawah tanah |
| `PlantDb::perennial` | `plant.rs` | `typ == "perennial"` |
| `PlantDb::annual` | `plant.rs` | `typ` salah satu dari `warm_annual`, `cold_annual`, `warm_annual_tuber`, `cold_annual_tuber` |
| `<Saved as Default>::default` | `plant.rs` | nilai awal variabel tersimpan: `rndseed_cond` = 748932582, selebihnya nol/kosong |

### 8.2  Basis data tanaman: `PlantDb::new`

Satu-satunya fungsi non-trivial pada `PlantDb` adalah konstruktornya, yang menggabungkan dua tahap Fortran: pembacaan baris (`plant_parm_read.f90`) dan pelengkapan nilai bawaan serta penurunan koefisien kurva (`plantparm_init.f90`). Pemanggilnya adalah pembangun HRU di `hru/mod.rs`, satu kali untuk setiap tanaman di komunitas `plant.ini` yang dipakai HRU.

#### 8.2.1  `PlantDb::new`

- **Lokasi:** `crates/swat-engine/src/hru/plant.rs`, `pub fn new(t: &Table, r: usize, rsdco: f32) -> PlantDb`
- **Padanan SWAT+:** `plant_parm_read` (`plant_parm_read.f90`) dan `plantparm_init` (`plantparm_init.f90`), memanggil `ascrv` (`ascrv.f90`)
- **Tujuan:** membentuk parameter tanaman final dari baris `r` tabel `plants.plt`, termasuk koefisien kurva-S untuk LAI, RUE terhadap CO₂, dan fraksi N/P biomassa.
- **Masukan dan keluaran:** tabel `plants.plt` (kolom yang hilang bernilai 0 atau string kosong), indeks baris, `rsdco` basin (koefisien dekomposisi residu, bawaan 0,05 bila `parameters.bsn` tidak memuatnya); keluaran `PlantDb`.
- **Cara kerja:**
  1. Salin kolom langsung (nama, tipe, pemicu, `days_mat` sebagai bilangan bulat, `bio_e`, `hvsti`, `blai`, …, `gsi`). `mat_yrs` dipaksa minimal 1.
  2. Simpan sementara nilai yang hanya dipakai untuk menurunkan koefisien: `pltnfr2`, `pltpfr2`, `co2hi`, `bioehi`, `laimx1`, `laimx2`, `frgrw1`, `frgrw2`, `gmaxfr`, `vpdfr`.
  3. Fraksi partisi residu (model karbon): kolom `lig_frac` menjadi lignin di bawah tanah dan kolom `str_frac` menjadi lignin di atas tanah (sesuai cara `plant_parm_read` membacanya ke variabel yang namanya tertukar). Fraksi struktural = lignin / 0,80; metabolik = 1 − struktural.
  4. Nilai bawaan `plantparm_init`: `bm_dieoff` ≤ 10⁻⁶ → 1,00; `ext_coef` < 10⁻⁶ → 0,65; `rsdco_pl` < 10⁻⁶ → `rsdco` basin; `usle_c` dibatasi ke \[0,001; 1\]; `blai` dibatasi ke \[0; 10\]; `rsr1` ≤ 0 → 0,4; `rsr2` ≤ 0 → 0,2; `aeration` ≤ 0 → 0,2; tipe umbi (`warm_annual_tuber`, `cold_annual_tuber`) → `rsr2` = 0,7 tanpa syarat; `hvsti` > 0,7 → 0,02.
  5. Bila `bio_e` > 0 dan nama bukan `WATR`, turunkan koefisien kurva:
     - kurva LAI: (`leaf1`, `leaf2`) = `ascrv(laimx1, laimx2, frgrw1, frgrw2)`, sehingga fraksi LAI maksimum pada fraksi heat unit f adalah

```latex
f_{LAI}(f) = \frac{f}{f+\exp(\ell_1-\ell_2 f)}
```

- kurva RUE–CO₂: bila `co2hi` tepat 330 diganti 660; (`ruc1`, `ruc2`) = `ascrv(bio_e/100, bioehi/100, 330, co2hi)`.
- `cvm` = ln(`usle_c`).
- N: bila n1 − n2 < 0,0001 maka n2 = n1 − 0,0001; bila n2 − n3 < 0,0001 maka n3 dikali 0,75. Lalu:

```latex
b_1=n_1-n_3,\quad b_2 = 1-\frac{n_2-n_3}{b_1},\quad b_3 = 1-\frac{0.00001}{b_1},\quad (\nu_1,\nu_2)=\mathrm{ascrv}(b_2,b_3,0.5,1.0)
```

- P: idem dengan `pltpfr1..3` menghasilkan (`pup1`, `pup2`).
- kemiringan konduktansi stomata terhadap VPD untuk Penman-Monteith:

```latex
\mathrm{vpd2}=\frac{1-\mathrm{gmaxfr}}{\mathrm{vpdfr}-1}
```

- **Kebiasaan SWAT+ / catatan port:**
  - Indeks panen di atas 0,7 menjadi 0,02, bukan dipangkas ke 0,7 (`docs/swat-quirks.md`: padang rumput Ames memiliki 0,9 dan karenanya menjadi 0,02).
  - Untuk `WATR` atau `bio_e` = 0 semua koefisien kurva tetap 0.
  - Nama yang tidak ada di `plants.plt` memakai baris pertama (pemanggil memberi `r` = 0, meniru penunjuk bawaan 1 di `readpcom`).
  - `pltnfr2`/`pltpfr2` yang dikoreksi tidak disimpan; hanya koefisien kurva yang tersisa.

### 8.3  Inisialisasi tanaman: `plant_init`

`plant_init` dipanggil oleh pembangun HRU (`hru/mod.rs`) untuk setiap tanaman komunitas, setelah `PlantDb::new`. Pembangun menolak komunitas lebih dari 20 tanaman, tanaman dengan model karbon `cswat` = 1, dan `co2_yr.dat`; ia juga menjumlahkan residu awal semua tanaman ke `rsd_tot` dan `blai` ke `laimx_sum`.

#### 8.3.1  `plant_init`

- **Lokasi:** `plant.rs`, `pub fn plant_init(db: PlantDb, init: &PlantInit, wgn_months: &[(f32,f32);12], ndays: &[i32;13], lat: f32, zmx: f32, layers: &[(f32,f32)], epco: f32, igrow_saved: &mut i32) -> Result<Plant, String>`
- **Padanan SWAT+:** bagian tanaman dari `plant_init` (`plant_init.f90`)
- **Tujuan:** menghitung heat unit menuju kematangan (`phumat`) dan menyiapkan keadaan awal tanaman (tinggi, LAI, biomassa, N, P, akar, partisi organ) dari `plant.ini`.
- **Masukan dan keluaran:** basis data tanaman; baris `plant.ini` (`cpnm`, `igro`, `lai`, `bioms` kg/ha, `phuacc`, `fr_yrmat`, `rsdin` kg/ha); suhu maksimum dan minimum rata-rata bulanan generator cuaca (°C); tabel hari kumulatif bulan; lintang generator (°); `zmx` profil tanah dari `soils.sol` (mm); kedalaman dan ketebalan lapisan (mm); `epco` hidrologi HRU; hari tanam tersimpan. Keluaran `Plant` atau galat.
- **Cara kerja:**
  1. Buat `Plant` dengan `cpnm`, `gro` = (`igro` == "y"), `idorm` = benar. Residu permukaan awal dari `rsdin` = r:

```latex
m = r,\quad c = 0.42\,r,\quad n = \frac{0.42\,r}{10},\quad p = \frac{0.42\,r}{100}
```

2. Suhu harian klimatologis hari ke-i adalah (Tmax\_bulan + Tmin\_bulan)/2 untuk bulan yang diberi `xmon(ndays, i)`; bila `xmon` tidak menemukan bulan (hari > 365), bulan sebelumnya dipertahankan, sehingga hari setelah 31 Desember memakai suhu Desember.
3. Heat unit tahunan berbasis 0 °C: HU₀ = Σᵢ max(T̄ᵢ, 0) untuk i = 1..365.
4. Heat unit menuju matang `phumat`:
   - `days_mat` = 0 (tumbuh sepanjang tahun):

```latex
PHU = 0.95\sum_{i=1}^{365}\max(\bar T_i - T_{base},\,0)
```

- `days_mat` < 0: `phumat` = −`phumat` (masih 0, jadi 0).
- `days_mat` > 0: tipe `null`/`null1` (hari tanam dari panjang hari) ditolak dengan galat. Untuk tanaman semusim, hari tanam adalah hari pertama ketika Σ max(T̄,0) melampaui 0,15·HU₀; di belahan bumi selatan (`lat` ≤ 0) penelusuran dimulai dari hari 182 dan berputar melewati akhir tahun. Hari ini disimpan ke `igrow_saved`. Untuk tanaman lain (misalnya perennial dengan `days_mat`) hari tanam tidak dihitung: dipakai `igrow_saved` apa adanya. Lalu:

```latex
PHU = \sum_{i=i_{grow}}^{i_{grow}+days\_mat}\max(\bar T_i - T_{base},\,0)
```

dengan hari > 365 dikurangi 365 hanya di belahan selatan; di utara hari itu memakai suhu Desember (langkah 2).

5. `phuacc` = `init.phuacc`. Fraksi kematangan: perennial memakai `fr_yrmat` dan tinggi `cht` = `fr_yrmat`·`chtmx`; lainnya memakai `phuacc` dan `cht` = `chtmx`·√f\_LAI(`phuacc`). `laimxfr` = f\_LAI(fraksi kematangan).
6. `lai` = `init.lai` bila tumbuh, selain itu 0; `tot.m` = `bioms`; `curyr_mat` = max(1, int(`fr_yrmat`·`mat_yrs`)); `phumat_p` = `phumat`·`mat_yrs`.
7. Fraksi N dan P awal dengan fraksi kematangan φ:

```latex
f_N = (n_1-n_3)\left[1-\frac{\phi}{\phi+\exp(\nu_1-\nu_2\phi)}\right]+n_3,\qquad N_{tot}=f_N\,B
```

(P analog dengan `pup1`, `pup2` dan `pltpfr1`, `pltpfr3`.)

8. `harv_idx` = `hvsti`, `lai_pot` = `blai`; vektor `rtfr` dan `soil_rsd` sepanjang jumlah lapisan.
9. Bila tanaman tumbuh: untuk perennial `phuacc_p` = `fr_yrmat` + `phumat`·`phuacc`/`phumat_p` (atau `fr_yrmat` bila `phumat_p` ≤ 0); lalu `root_gro(zmx, layers)`, `seed_gro(0,0,0)`, `partition(0, init = true)` dan `rootfr(layers)`.
10. `epco` dari hidrologi; semua faktor stres = 1.

- **Kebiasaan SWAT+ / catatan port:**
  - Hari tanam `igrow` adalah variabel `SAVE` di Fortran; port meneruskannya lewat `igrow_saved` yang dibagi semua HRU dan tanaman, sehingga perennial dengan `days_mat` > 0 memakai hari tanam terakhir yang dihitung untuk tanaman semusim mana pun sebelumnya (atau hari 0) — `docs/engine1.md`, *Plants: days to maturity*.
  - Penjumlahan PHU dari `igrow` sampai `igrow + days_mat` inklusif (days\_mat + 1 hari), sesuai batas loop Fortran.
  - Populasi tanaman `plnt_pop` di `plant.ini` dibaca tetapi tidak pernah sampai ke tanaman (`pop_com` tetap 0), sehingga `lai_pot` selalu `blai`.
  - Akar awal dibatasi `zmx` dari `soils.sol`, bukan kedalaman profil HRU yang baru ditetapkan kemudian oleh `hydro_init` (`swat-quirks.md`).
  - Penulisan `0.0 + r` untuk residu awal meniru penjumlahan Fortran terhadap nilai nol (tanpa efek numerik selain tanda nol).

### 8.4  Komunitas tanaman dan penyerapan air

Dua fungsi `impl Hru` ini berjalan pertama setiap hari, sebelum pertumbuhan. `pl_community` membagi transpirasi maksimum di antara tanaman menurut LAI, memanggil `pl_waterup` untuk tiap tanaman yang tumbuh, lalu menghitung radiasi yang diintersepsi. Karena penyerapan air terjadi sebelum `pl_grow`, stres air hari ini sudah tersedia saat biomassa dihitung.

#### 8.4.1  `pl_community`

- **Lokasi:** `crates/swat-engine/src/hru/plant.rs`, `pub(super) fn pl_community(&mut self, d: &mut PlantDay, solrad: f32, b: &PlantBasin)`
- **Padanan SWAT+:** `pl_community` (`pl_community.f90`) beserta pemanggilan `pl_waterup`
- **Tujuan:** menghitung total komunitas (LAI, biomassa, tinggi kanopi maksimum), membagi `ep_max`, menjalankan penyerapan air, dan menghitung PAR yang diintersepsi.
- **Masukan dan keluaran:** `d.ep_max` (mm), `solrad` (MJ/m²/hari); keluaran `self.lai_sum`, `self.tot_com`, `self.ab_gr_com` (kg/ha), `self.cht_mx` (m), `pl.par` (MJ/m²), `d.ep_day` (mm), kadar air lapisan.
- **Cara kerja:**
  1. LAI komunitas ΣLAI.
  2. Transpirasi maksimum per tanaman (0 bila ΣLAI ≤ 10⁻⁶):

```latex
E_{t,max,i} = E_{t,max}\,\frac{LAI_i}{\sum_j LAI_j}
```

3. `tot_com` dan `ab_gr_com` = jumlah `tot` dan `ab_gr` semua tanaman; `cht_mx` = tinggi maksimum.
4. Untuk setiap tanaman dengan `gro` (dorman atau tidak), urut komunitas, panggil `pl_waterup(i, E_t,max,i)`.
5. Radiasi terintersepsi (hanya tanaman tumbuh dan tidak dorman; selain itu 0), dengan koefisien pemadaman kₗ = `ext_coef`:

```latex
H_{phosyn,i} = 0.5\,H_{day}\left[1-\exp\big(-k_\ell\,(LAI_i+0.05)\big)\right]
```

- **Kebiasaan SWAT+ / catatan port:**
  - Air dibagikan berurutan: setiap tanaman mengambil bagiannya dari air yang ditinggalkan tanaman sebelumnya (`swat-quirks.md`, *Plant communities*).
  - Intersepsi cahaya tiap tanaman memakai LAI tanaman itu sendiri, tanpa naungan oleh tanaman lain; faktor 0,5 mengubah radiasi total menjadi PAR (hukum Beer seperti di Neitsch dkk. 2009, bagian 5).
  - Pembagian `ep_max` dihitung sekali sebelum penyerapan mana pun.

#### 8.4.2  `pl_waterup`

- **Lokasi:** `plant.rs`, `fn pl_waterup(&mut self, i: usize, epmax: f32, d: &mut PlantDay, b: &PlantBasin)`
- **Padanan SWAT+:** `pl_waterup` (`pl_waterup.f90`)
- **Tujuan:** menghitung stres aerasi dan menyerap air tanah menurut kedalaman akar; hasilnya transpirasi aktual tanaman dan stres air.
- **Masukan dan keluaran:** `self.sw`, `self.sumfc`, `self.sumul` (mm; air tanah, kapasitas lapang, porositas profil), `root_dep` (mm), `epco`, `epmax` (mm); keluaran `pl.strsa`, `pl.strsw`, `pl.strss`, `layers[k].st` (mm), `self.sw`, `d.ep_day` (mm).
- **Cara kerja:**
  1. Stres aerasi, hanya bila SW > FC (selain itu `strsa` tetap dari *reset* pagi, 1):

```latex
s = \frac{SW-FC}{UL-FC},\qquad \psi = 100\,\frac{s-a}{1.0001-a},\qquad
\gamma_a = \begin{cases}1-\dfrac{\psi}{\psi+\exp(2.9014-0.03867\,\psi)} & \psi>0\\ 1 & \psi\le 0\end{cases}
```

dengan a = `aeration`.

2. Bila `epmax` ≤ 10⁻⁶: `strsw` = 1 dan selesai.
3. Untuk lapisan k = 1, 2, … sampai lapisan yang memuat ujung akar (`ir`): kedalaman efektif g = min(z\_root, d\_k). Permintaan kumulatif hingga g (βw = 10):

```latex
w_{up,g} = \frac{E_{t,max}}{1-e^{-\beta_w}}\left[1-\exp\!\left(-\beta_w\frac{g}{z_{root}}\right)\right]
```

(bila z\_root ≤ 0,01 mm, w = E\_t,max / W\_norm).

4. Serapan lapisan: pada lapisan kedua w\_use = w\_up,g; pada lapisan lain w\_use = w\_up,g − w\_up,prev·(1 − `epco`). Batasi w\_use ≤ st\_k.
5. Jumlahkan; bila jumlah melampaui E\_t,max, w\_use = E\_t,max − jumlah sebelumnya dan jumlah = E\_t,max.
6. Batasi lagi w\_use ≤ st\_k, lalu st\_k = max(10⁻⁶, st\_k − w\_use).
7. SW profil dijumlah ulang dari lapisan; `strss` = 1; stres air dan transpirasi:

```latex
\gamma_w = \frac{\sum_k w_{use,k}}{E_{t,max,i}},\qquad E_{t,day} \mathrel{+}= \sum_k w_{use,k}
```

- **Kebiasaan SWAT+ / catatan port:**
  - Lapisan kedua mengabaikan `epco` (Fortran menguji `k == 2`; port dengan indeks 0 menguji `k == 1`). Komentar Fortran: lapisan 10 mm di atas bermasalah bila akar dangkal (`swat-quirks.md`).
  - Karena w\_up,g kumulatif, dengan `epco` = 1 setiap lapisan boleh mengambil seluruh permintaan kumulatif hingga kedalamannya; pembatas hanyalah total E\_t,max dan air lapisan. Ini berbeda dari rumus kompensasi SWAT 2009 (Neitsch dkk. 2009, bagian 5, penyerapan air).
  - Lantai 10⁻⁶ mm per lapisan dapat "menciptakan" air sangat kecil; dipertahankan.
  - Bagian stres salinitas Fortran (`salt_tol_sim`, `salt_chem_soil_single`) tidak diporting; `strss` selalu 1. Array `uptake(k)` per lapisan juga tidak disimpan.
  - Stres aerasi tidak di-*reset* bila SW ≤ FC di dalam fungsi ini; nilai 1 berasal dari *reset* di awal hari (`Hru::day`).

### 8.5  Pertumbuhan harian: heat unit, dormansi, biomassa, dan nutrien

`pl_grow` adalah inti harian. Ia menggabungkan `pl_nut_demand` (akumulasi heat unit dan permintaan N/P), lalu untuk setiap tanaman: dormansi, biomassa (dengan stres suhu dan serapan N/P), akar, daun, senesens, biji dan partisi (subbab 8.6), serta kematian akhir tahun.

#### 8.5.1  `pl_grow`

- **Lokasi:** `plant.rs`, `pub(super) fn pl_grow(&mut self, d: &mut PlantDay, w: &PlantWeather, b: &PlantBasin, saved: &mut Saved)`
- **Padanan SWAT+:** `pl_grow` (`pl_grow.f90`) dengan `pl_nut_demand` (`pl_nut_demand.f90`) di awalnya
- **Tujuan:** memajukan fenologi dan menumbuhkan semua tanaman HRU satu hari.
- **Masukan dan keluaran:** cuaca (`tave` °C, dll.), `PlantDay` dari `pl_community`, array permintaan bersama di `Saved`; keluaran keadaan tanaman, `d.bm_grow`, `d.c_gro` (kg/ha), `self.rsd_tot`, total komunitas.
- **Cara kerja:**
  1. Pengosongan permintaan (`pl_nut_demand`): mode asli mengosongkan hanya elemen `uno3d` dan `uapd` bernomor npl + 1 (sisa penghitung loop); mode `--fix-swat` mengosongkan elemen 1..npl dari keempat array.
  2. Untuk setiap tanaman yang tumbuh dan tidak dorman, pertambahan heat unit (Δ ≥ 0):

```latex
\Delta PHU = \frac{\bar T - T_{base}}{PHU_{mat}},\qquad \Delta PHU_p = \frac{\bar T - T_{base}}{PHU_{mat}\cdot mat\_yrs}
```

dihitung hanya bila `phumat` > 0,1; `phuacc` += ΔPHU, `phuacc_p` += ΔPHU\_p. Lalu `nupd`, `pupd`; hasil disimpan di `saved` pada indeks tanaman, dan `uno3d_tot`, `uapd_tot` dijumlahkan.

3. ΣNO₃ dan ΣP labil profil (kg/ha) ke `sum_no3`, `sum_solp`.
4. Untuk setiap tanaman i: muat permintaan dari `saved[i]`; bila tumbuh dan pemicunya `temp_gro`, panggil `pl_dormant`; bila tumbuh dan tidak dorman: `pl_biomass_gro`, `root_gro`, `leaf_gro` (dengan tinggi kanopi tanaman lain), `leaf_senes` (daun gugur ditambahkan ke `rsd_tot` dan dikurangkan dari total komunitas), `seed_gro`, `partition(up)`.
5. Bila hari terakhir tahun, `pl_mortality` (untuk setiap tanaman, tumbuh atau tidak).
6. Tulis kembali permintaan ke `saved[i]`.

- **Kebiasaan SWAT+ / catatan port:**
  - Permintaan basi (`swat-quirks.md`, `swat-fixes.md`): array `uno3d`, `uapd`, `un2`, `up2` milik modul, bukan HRU. Tanaman yang masih dorman saat permintaan dihitung tetapi bangun hari itu juga oleh `pl_dormant` tumbuh dengan permintaan terakhir yang dihitung untuk HRU mana pun. Diperbaiki di `--fix-swat`.
  - `delg_p` adalah variabel `SAVE`: bila `phumat` ≤ 0,1, `phuacc_p` bertambah dengan nilai terakhir dari tanaman/HRU lain.
  - Dormansi diputuskan setelah permintaan dihitung; urutan ini penting untuk bit-for-bit.
  - `std::mem::take` mengeluarkan tanaman dari vektor selama diproses agar `&mut self` (lapisan tanah) dan `&mut Plant` dapat dipakai bersamaan; tinggi tanaman sendiri diisi `NaN` sebagai penanda dan diganti di `leaf_gro`.

#### 8.5.2  `nupd` (method `Plant`)

- **Lokasi:** `plant.rs`, `fn nupd(&mut self, d: &mut PlantDay)`
- **Padanan SWAT+:** `pl_nupd` (`pl_nupd.f90`)
- **Tujuan:** permintaan N hari ini.
- **Masukan dan keluaran:** biomassa total B, N tanaman (kg/ha); keluaran `n_fr`, `d.un2` (N optimal), `d.uno3d` (permintaan), kg/ha.
- **Cara kerja:**
  1. Fraksi kematangan φ: `curyr_mat`/`mat_yrs` untuk perennial, `phuacc` untuk lainnya.
  2. f\_N dengan kurva (`nup1`, `nup2`) seperti di 8.3.1.
  3. N optimal dan permintaan:

```latex
N_{opt} = \max(f_N B,\ N_{tot}),\qquad N_{dem} = N_{opt}-N_{tot}
```

- **Kebiasaan SWAT+ / catatan port:** perennial memakai umur tahun, bukan heat unit musim, sehingga fraksi N-nya tetap sepanjang tahun.

#### 8.5.3  `pupd` (method `Plant`)

- **Lokasi:** `plant.rs`, `fn pupd(&mut self, d: &mut PlantDay)`
- **Padanan SWAT+:** `pl_pupd` (`pl_pupd.f90`)
- **Tujuan:** permintaan P hari ini.
- **Masukan dan keluaran:** biomassa di atas tanah dan P-nya (kg/ha); keluaran `p_fr`, `d.up2`, `d.uapd`.
- **Cara kerja:**
  1. φ seperti `nupd`; f\_P dengan (`pup1`, `pup2`).
  2. Berbeda dengan N, basisnya biomassa di atas tanah B\_ag dan P di atas tanah, dan permintaan dikali 1,5:

```latex
P_{opt} = \max(f_P B_{ag},\ P_{ag}),\qquad P_{dem} = 1.5\,(P_{opt}-P_{ag})
```

- **Kebiasaan SWAT+ / catatan port:** faktor 1,5 (kemewahan serapan P) ada di SWAT 2009 (Neitsch dkk. 2009, bagian 5, penyerapan P); pemakaian biomassa di atas tanah khas SWAT+.

#### 8.5.4  `pl_dormant`

- **Lokasi:** `plant.rs`, `fn pl_dormant(&mut self, pl: &mut Plant, w: &PlantWeather)`
- **Padanan SWAT+:** `pl_dormant` (`pl_dormant.f90`)
- **Tujuan:** memasukkan dan mengeluarkan perennial dan tanaman semusim musim dingin dari dormansi menurut panjang hari.
- **Masukan dan keluaran:** panjang hari (jam), `dormhr` (jam), `daylmn` (jam); keluaran `idorm`, `phuacc`, residu.
- **Cara kerja:**
  1. Masuk dormansi bila tidak dorman dan `daylength` − `dormhr` < `daylmn`:
     - perennial: `idorm` = benar; rasio gugur rto = 0, sehingga batang dan daun gugur 0 dan LAI menjadi 0; seluruh biji gugur ke residu (`tot`, `ab_gr` dikurangi, `rsd` dan `rsd_tot` ditambah); `seed` = 0; `phuacc` = 0; `strsw` = 1.
     - `cold_annual` dengan `phuacc` < 0,75: `idorm` = benar, `strsw` = 1.
  2. Keluar dormansi bila dorman dan `daylength` − `dormhr` ≥ `daylmn`: perennial `idorm` = salah, `phuacc` = 0, `d_senes` = 0; `cold_annual` `idorm` = salah, `phuacc` = 0.
- **Kebiasaan SWAT+ / catatan port:**
  - Rasio gugur ditulis 0 di Fortran (nilai `bm_dieoff` dikomentari), jadi dormansi perennial hanya membuang biji dan menolkan LAI.
  - Karbon daun yang gugur tidak pernah diisi (`leaf_drop%c` = 0); tak berpengaruh karena massanya 0.
  - Pembatasan `tot_com.m` ≥ 0 diterapkan pada total komunitas yang tidak diubah di sini (tanpa efek), sesuai Fortran.
  - Hanya dipanggil untuk tanaman `temp_gro`. Di bawah 20° lintang, `cli_initwgn` memberi ambang −1 jam bila `dorm_hr` = 0, sehingga perennial tropis tidak pernah dorman dan `phuacc`-nya tidak pernah kembali ke 0 (`swat-quirks.md`, kasus Batanghari dan Pessel).

#### 8.5.5  `tstr` (method `Plant`)

- **Lokasi:** `plant.rs`, `fn tstr(&mut self, w: &PlantWeather)`
- **Padanan SWAT+:** `pl_tstr` (`pl_tstr.f90`)
- **Tujuan:** faktor stres suhu.
- **Masukan dan keluaran:** `tave`, `t_base`, `t_opt` (°C); keluaran `strst` (0–1).
- **Cara kerja:**

```latex
r = \frac{\bar T - T_{base}}{T_{opt}-T_{base}},\qquad \gamma_T = \min\!\big(1,\ \max(0,\ \sin(1.5707\,r))\big)
```

- **Kebiasaan SWAT+ / catatan port:** Fortran menghitung dulu stres klasik SWAT 2009 (γ = exp(−0,1054 (T\_opt − T)²/(T − T\_base)²), dengan cerminan di atas T\_opt dan pemutusan saat Tmin ≤ T\_an − 15), lalu selalu menimpanya karena syarat `rto > 0 .or. rto < 2` selalu benar. Port tidak menghitung versi klasik (hasil identik) dan hanya mempertahankan penugasan 0 untuk T ≤ T\_base yang juga tertimpa. Secara matematis sinus kembali positif untuk r > 4, suhu yang tidak realistis.

#### 8.5.6  `pl_biomass_gro`

- **Lokasi:** `plant.rs`, `fn pl_biomass_gro(&mut self, pl: &mut Plant, d: &mut PlantDay, w: &PlantWeather, b: &PlantBasin, up: &mut OrgMass)`
- **Padanan SWAT+:** `pl_biomass_gro` (`pl_biomass_gro.f90`)
- **Tujuan:** pertambahan biomassa potensial dari radiasi, dikurangi faktor stres terkecil; memicu serapan N dan P.
- **Masukan dan keluaran:** `par` (MJ/m²), CO₂ (ppm), VPD (kPa); keluaran `up.m`, `up.c` (kg/ha), stres, akumulator musiman.
- **Cara kerja:**
  1. Bila `phuacc` > 1 (matang), tidak ada pertumbuhan.
  2. RUE disesuaikan CO₂ (bila CO₂ > 350 ppm), lalu VPD (bila VPD > 1 kPa):

```latex
RUE = \frac{100\,CO_2}{CO_2+\exp(r_1-r_2\,CO_2)},\qquad RUE \leftarrow \max\big(RUE-\Delta_{ruedcl}(vpd-1),\ 0.27\,RUE_{bio\_e}\big)
```

3. Biomassa potensial ΔB\_act = RUE·H\_phosyn (≥ 0).
4. `tstr`. Rasio pembagian bila permintaan total melampaui persediaan: `rto_no3` = `uno3d`/`uno3d_tot` bila `uno3d_tot` > ΣNO₃, selain itu 1 (sama untuk P).
5. Batas permintaan harian: `uno3d` = min(4·`pltnfr3`·ΔB, `uno3d`); bila `uapd` > 10, `uapd` = min(4·`pltpfr3`·ΔB, `uapd`).
6. `pl_nup`, `pl_pup`.
7. `nostress` = 1: semua stres 1; = 2: stres N dan P 1.
8. Faktor pengatur dan pertumbuhan aktual:

```latex
reg = \min(\gamma_w,\gamma_T,\gamma_N,\gamma_P,\gamma_a,\gamma_s)\in[0,1],\qquad \Delta B = \Delta B_{pot}\,reg,\qquad \Delta C = 0.42\,\Delta B
```

9. `bm_grow`, `c_gro` dijumlahkan; `sum_w`, `sum_tmp`, `sum_n`, `sum_p`, `sum_a` bertambah (1 − γ).

- **Kebiasaan SWAT+ / catatan port:**
  - Pembatas 4·`pltpfr3`·ΔB untuk P hanya berlaku bila permintaan P > 10 kg/ha.
  - Stres diambil minimum (Liebig), bukan perkalian.

#### 8.5.7  `pl_nup`

- **Lokasi:** `plant.rs`, `fn pl_nup(&mut self, pl: &mut Plant, d: &mut PlantDay, b: &PlantBasin, up: &mut OrgMass)`
- **Padanan SWAT+:** `pl_nup` (`pl_nup.f90`), memanggil `pl_nfix`
- **Tujuan:** serapan nitrat per lapisan menurut kedalaman akar, fiksasi N legum, dan stres N.
- **Masukan dan keluaran:** `uno3d`, `rto_no3`, `un2`, NO₃ lapisan (kg/ha), `n_updis`; keluaran `d.nplnt`, `d.fixn`, N tanaman, `up.n`, `strsn`.
- **Cara kerja:**
  1. `strsn` = 1, `nuptake` = 0; selesai bila permintaan < 10⁻⁶.
  2. Kedalaman akar efektif z\_r = max(10,1; `root_dep`), lalu diperpanjang ke dasar lapisan yang memuatnya.
  3. Untuk setiap lapisan dengan dasar d\_k ≤ z\_r:

```latex
U_{max,k} = \frac{N_{dem}\,r_{NO_3}\left[1-\exp(-\beta_n d_k/z_r)\right]}{1-e^{-\beta_n}},\qquad u_k=\min(U_{max,k}-N_{plnt},\ NO_{3,k})
```

dengan N\_plnt bertambah u\_k dan NO₃,k berkurang u\_k.

4. N\_plnt ≥ 0. Bila legum (`nfix_co` > 10⁻⁶), `fixn` = `pl_nfix`. N\_plnt += `fixn`.
5. N tanaman: total += N\_plnt; di atas tanah += N\_plnt(1 − `root_frac`); akar += N\_plnt·`root_frac`; `up.n` = N\_plnt.
6. Stres: legum → 1; lainnya γ\_N = min(1, max(`nuts`(N\_tot, N\_opt), N\_plnt/N\_dem)).

- **Kebiasaan SWAT+ / catatan port:**
  - `nplnt` dan `fixn` adalah total berjalan HRU (direset harian, bukan per tanaman): tanaman kedua memulai dari total tanaman pertama dan menambahkannya lagi ke N-nya sendiri; tanaman non-legum setelah legum ikut menerima N fiksasi (`swat-quirks.md`, *Plant communities*). Secara aritmetika, bila U\_max,k < N\_plnt, u\_k negatif dan nitrat dikembalikan ke lapisan.
  - Pembagian N organ di sini akan ditimpa oleh `partition` pada hari yang sama.

#### 8.5.8  `pl_nfix`

- **Lokasi:** `plant.rs`, `fn pl_nfix(&self, pl: &Plant, d: &PlantDay, b: &PlantBasin) -> f32`
- **Padanan SWAT+:** `pl_nfix` (`pl_nfix.f90`)
- **Tujuan:** fiksasi N biologis legum.
- **Masukan dan keluaran:** sisa permintaan, SW dan FC profil (mm), ΣNO₃ (kg/ha), `phuacc`, `nfix_co`, `nfixmx`; keluaran fiksasi (kg N/ha).
- **Cara kerja:**
  1. Sisa permintaan u = N\_dem − N\_plnt; bila ≤ 0, 0.
  2. Faktor air f\_w = SW/(0,85·FC); faktor nitrat f\_n = 1 (ΣNO₃ ≤ 100), 1,5 − 0,005·ΣNO₃ (100–300), 0 (> 300); faktor fase f\_g = 6,67φ − 1 (0,15 < φ ≤ 0,30), 1 (0,30–0,55), 3,75 − 5φ (0,55–0,75), selain itu 0.
  3. Fiksasi:

```latex
f_r = \max\big(0,\ \min(1,f_w,f_n)\,f_g\big),\quad F=\min(6,\ f_r u),\quad F \leftarrow c_{fix}F+(1-c_{fix})u,\quad F=\min(F,u,F_{max})
```

- **Kebiasaan SWAT+ / catatan port:** dengan `nfix_co` = 1 berlaku model fiksasi SWAT 2009 (Neitsch dkk. 2009, bagian 5; batas 6 kg/ha/hari); nilai lebih kecil mencampurnya dengan pemenuhan penuh permintaan.

#### 8.5.9  `pl_pup`

- **Lokasi:** `plant.rs`, `fn pl_pup(&mut self, pl: &mut Plant, d: &mut PlantDay, b: &PlantBasin, up: &mut OrgMass)`
- **Padanan SWAT+:** `pl_pup` (`pl_pup.f90`)
- **Tujuan:** serapan P labil per lapisan dan stres P.
- **Masukan dan keluaran:** `uapd`, `rto_solp`, `up2`, P labil lapisan (kg/ha), `p_updis`; keluaran `d.pplnt`, P tanaman, `up.p`, `strsp`.
- **Cara kerja:** sama dengan langkah 1–3 `pl_nup` dengan β\_p, P labil, dan `exp_w`:

```latex
U_{max,k} = \frac{P_{dem}\,r_{P}\left[1-\mathrm{exp\_w}(-\beta_p d_k/z_r)\right]}{1-\mathrm{exp\_w}(-\beta_p)},\qquad u_k=\min(U_{max,k}-P_{plnt},\ P_{lab,k})
```

lalu P\_plnt ≥ 0; P total/atas tanah/akar ditambah seperti N; stres γ\_P = `nuts`(P\_ag, P\_opt).

- **Kebiasaan SWAT+ / catatan port:** stres P memakai P di atas tanah dan tidak diambil maksimum dengan rasio serapan seperti N. `pplnt` juga total berjalan HRU.

### 8.6  Organ tanaman: akar, daun, biji, partisi, dan kematian

Method `impl Plant` berikut (kecuali `pl_mortality`, yang ada di `impl Hru` karena menyentuh `rsd_tot` dan lapisan) mengubah keadaan satu tanaman. Urutan panggil dalam `pl_grow` adalah `root_gro` → `leaf_gro` → `leaf_senes` → `seed_gro` → `partition`, dan `pl_mortality` pada akhir tahun.

#### 8.6.1  `reset_stress`

- **Lokasi:** `crates/swat-engine/src/hru/plant.rs`, `pub(super) fn reset_stress(&mut self)`
- **Padanan SWAT+:** `plstrz` (bagian `plant_module`/`mgt_*` yang menolkan `plstr`)
- **Tujuan:** menghapus stres dan akumulator musiman tanaman.
- **Masukan dan keluaran:** keluaran `strsw`, `strst`, `strsn`, `strsp`, `strsa`, `strss` = 1, `reg` = 0, `sum_w`, `sum_tmp`, `sum_n`, `sum_p`, `sum_a` = 0.
- **Cara kerja:** penugasan langsung.
- **Kebiasaan SWAT+ / catatan port:** dipanggil dari `hru/mgt.rs` saat penanaman (`plantop`), pematian (`killop`) dan panen-dan-matikan; panen-dan-matikan terjadwal me-*reset* stres semua tanaman yang tumbuh, apa pun nama tanaman pada operasi (`swat-quirks.md`, *Management*). *Reset* harian stres (tanpa akumulator) dilakukan terpisah di `Hru::day`.

#### 8.6.2  `root_gro`

- **Lokasi:** `plant.rs`, `fn root_gro(&mut self, zmx: f32, layers: &[(f32, f32)])`
- **Padanan SWAT+:** `pl_root_gro` (`pl_root_gro.f90`)
- **Tujuan:** kedalaman akar dan fraksi biomassa akar, lalu distribusi akar per lapisan.
- **Masukan dan keluaran:** `phuacc` atau `phuacc_p`, `rdmx` (m), `zmx` (mm), lapisan (kedalaman dasar, ketebalan; mm); keluaran `root_dep` (mm), `root_frac`, `rtfr`.
- **Cara kerja:**
  1. Kedalaman akar, dengan φ = `phuacc` untuk tanaman semusim dan `phuacc_p` untuk lainnya:

```latex
z_{root} = \min\big(2.5\,\phi\cdot 1000\,z_{root,mx},\ z_{mx}\big),\quad z_{root}\ge 25.4\ \mathrm{mm}
```

2. Fraksi akar. Perennial (dengan `mat_yrs` > 0, selalu benar karena minimal 1): r = `curyr_mat`/`mat_yrs`; bila r < 0,2 maka rsr1 − (rsr1 − rsr2)·r/0,2, selain itu rsr2. Lainnya: rsr1 − (rsr1 − rsr2)·min(`phuacc`, 1).
3. `rootfr(layers)`.

- **Kebiasaan SWAT+ / catatan port:**
  - Batas `zmx` diterapkan sebelum batas bawah 25,4 mm, jadi 25,4 mm menang bila `zmx` lebih kecil.
  - Akar mencapai maksimum pada φ = 0,4 (faktor 2,5), seperti SWAT 2009.
  - `zmx` HRU dapat dipangkas permanen oleh `plantop` saat penanaman (8.7.8).

#### 8.6.3  `rootfr`

- **Lokasi:** `plant.rs`, `pub(super) fn rootfr(&mut self, layers: &[(f32, f32)])`
- **Padanan SWAT+:** `pl_rootfr` (`pl_rootfr.f90`)
- **Tujuan:** fraksi akar di setiap lapisan tanah (dipakai untuk mengirim akar mati ke residu lapisan).
- **Masukan dan keluaran:** `root_dep` (mm), lapisan (d, tebal); keluaran `rtfr[k]` (jumlah 1).
- **Cara kerja:**
  1. Nolkan `rtfr`; bila `root_dep` < 10⁻⁶, seluruh akar di lapisan 1.
  2. Kerapatan akar ternormalisasi ρ(x) = a·e^(−bx) + c dengan a = 1,15, b = 11,7, c = 0,022, x kedalaman relatif; integralnya dari 0 sampai 1 adalah d = 0,12029. Untuk setiap lapisan sampai lapisan yang memuat ujung akar, dengan z = min(d\_k, z\_root), x₁ = (z − tebal\_k)/z\_root, x₂ = z/z\_root:

```latex
r_k = \frac{\dfrac{a}{b}\left(e^{-b x_1}-e^{-b x_2}\right)+c\,(x_2-x_1)}{d}
```

3. Eksponen −b·x dibatasi ≤ 20. Jumlah kumulatif dibatasi 1: bila terlampaui, r\_k = 1 − jumlah sebelumnya.
4. Normalisasi: setiap r\_k sampai lapisan terakhir yang diproses dibagi jumlah kumulatif.

- **Kebiasaan SWAT+ / catatan port:**
  - Untuk lapisan yang memuat ujung akar, x₁ dihitung dari z\_root dikurangi seluruh ketebalan lapisan, bukan dari dasar lapisan di atasnya. Bila akar berakhir di tengah lapisan, interval ini tumpang tindih dengan lapisan sebelumnya (atau x₁ negatif), sehingga fraksi lapisan terakhir terlalu besar sebelum dipangkas oleh batas jumlah 1 dan normalisasi. Port mempertahankannya.
  - Komentar Fortran menyebut fungsi kerapatan ini dari Dwyer dkk. (tahun tidak dicantumkan di sumber).

#### 8.6.4  `leaf_gro`

- **Lokasi:** `plant.rs`, `fn leaf_gro(&mut self, chts: &[f32], me: usize)`
- **Padanan SWAT+:** `pl_leaf_gro` (`pl_leaf_gro.f90`)
- **Tujuan:** pertambahan LAI dan tinggi kanopi selama fase sebelum penurunan daun.
- **Masukan dan keluaran:** `phuacc`, `phuacc_p`, `laimxfr`, `lai`, `lai_pot`, `strsw`, tinggi kanopi tanaman lain (m); keluaran `lai`, `cht` (m), `laimxfr`, `olai`, `dphu`.
- **Cara kerja:**
  1. f = f\_LAI(`phuacc`) (kurva `leaf1`, `leaf2`). Pertambahan fraksi: `laimxfr` = min(f, `laimxfr`), Δf = f − `laimxfr`, lalu `laimxfr` = f.
  2. f\_p = f\_LAI(`phuacc_p`) dengan eksponen dibatasi ≥ −16.
  3. Hanya bila `phuacc` < `dlai`: LAI maksimum tahun ini; perennial (dengan `curyr_mat` ≥ 1 dipaksa):

```latex
LAI_{mx} = \min\!\left(LAI_{pot}\cdot 10^{\,\log_{10}(yr/yr_{mat})\cdot c_{tree}},\ LAI_{pot}\right)
```

selain perennial LAI\_mx = `lai_pot`.

4. Tinggi kanopi: perennial `chtmx`·√f\_p, lainnya `chtmx`·√f. LAI dipangkas ke LAI\_mx.
5. Pertambahan LAI, diperkecil oleh stres air dan dibagi menurut tinggi kanopi:

```latex
\Delta LAI = \Delta f\cdot LAI_{mx}\left[1-\exp\big(5(LAI-LAI_{mx})\big)\right]\sqrt{\gamma_w}\cdot\frac{LAI_i\,h_i}{\sum_j LAI_i\,h_j}
```

6. LAI += ΔLAI, dipangkas ke LAI\_mx; `olai` = LAI; `dphu` = `phuacc`.

- **Kebiasaan SWAT+ / catatan port:** penyebut pembagian memakai LAI tanaman yang sedang tumbuh dikalikan tinggi setiap tanaman, sehingga bagiannya sama dengan h\_i/Σh\_j (`swat-quirks.md`, *Plant communities*). Tinggi tanaman sendiri diambil setelah diperbarui di langkah 4 (port memberi `NaN` sebagai penanda di `chts[me]` dan menggantinya). Untuk perennial, tinggi kanopi tumbuh menurut heat unit seumur hidup (`phuacc_p`), bukan musim.

#### 8.6.5  `leaf_senes`

- **Lokasi:** `plant.rs`, `fn leaf_senes(&mut self) -> OrgMass`
- **Padanan SWAT+:** `pl_leaf_senes` (`pl_leaf_senes.f90`)
- **Tujuan:** penurunan LAI setelah fraksi heat unit `dlai`, dan pengguguran daun perennial ke residu.
- **Masukan dan keluaran:** `phuacc`, `dphu`, `olai`, `dlai_rate`, `alai_min`, `leaf_tov`; keluaran `lai`, `d_senes`, organ, `rsd`; mengembalikan daun gugur (kg/ha).
- **Cara kerja:**
  1. Tanaman semusim dengan `dlai` < φ < 1:

```latex
LAI = LAI_{o}\left(\frac{1-\phi}{1-\phi_{d}}\right)^{r_{dlai}}
```

2. Perennial `temp_gro` dengan φ > `dlai` dan `d_senes` < 15: `d_senes` += 1; r = 1 − `d_senes`/15; LAI = (LAI\_o − LAI\_min)·r + LAI\_min (≥ LAI\_min). Daun gugur: m = `leaf_tov`·m\_daun; n = max(0; 0,68·m·`n_fr`); p = max(0; 0,68·m·`p_fr`).
3. Bila m > 0: kurangkan dari daun, total dan atas tanah; tambahkan ke `rsd`; kembalikan nilai itu (pemanggil menambahkannya ke `rsd_tot` dan mengurangkannya dari total komunitas).

- **Kebiasaan SWAT+ / catatan port:**
  - `leaf_tov` hanya diisi untuk tanaman yang tumbuh menurut kelembapan, sehingga perennial `temp_gro` kehilangan LAI tetapi tidak massa daun (`swat-quirks.md`). Karbon daun gugur juga tidak diisi.
  - Di tropis (tanpa dormansi), begitu φ melewati `dlai`, LAI turun ke `alai_min` dalam 15 hari dan tetap di sana sepanjang simulasi; kasus uji Batanghari dan Pessel menyetel `dlai` (`hu_lai_decl`) = 20 sebagai pengaman.

#### 8.6.6  `seed_gro`

- **Lokasi:** `plant.rs`, `fn seed_gro(&mut self, ep_day: f32, pet_day: f32, station_tave: f32)`
- **Padanan SWAT+:** `pl_seed_gro` (`pl_seed_gro.f90`)
- **Tujuan:** perkembangan indeks panen (fraksi biji) sepanjang musim, dikoreksi defisit air.
- **Masukan dan keluaran:** `harv_idx`, `hvsti`, `phuacc`, transpirasi dan PET hari ini (mm); keluaran `plet`, `plpet` (mm), `hi_adj`, `hi_prev`.
- **Cara kerja:**
  1. Indeks panen potensial pada φ:

```latex
HI = HI_{opt}\,\frac{100\,\phi}{100\,\phi+\exp(11.1-10\,\phi)}
```

2. Bila φ > 0,5: akumulasikan `plet` += E\_t,day dan `plpet` += PET; bila `plpet` > 10⁻⁶, rasio e = 100·plet/plpet dan

```latex
HI \leftarrow \left(HI-\tfrac{HI}{2}\right)\frac{e}{e+\exp(6.13-0.0883\,e)}+\tfrac{HI}{2},\qquad HI\le HI_{opt,db}
```

3. ΔHI = HI − `hi_prev`; `hi_adj` += ΔHI, dibatasi \[0, `hvsti`\]; `hi_prev` = HI.
4. Bila φ ≤ 0,5: `hi_prev` = 0.

- **Kebiasaan SWAT+ / catatan port:**
  - Fortran juga menghitung faktor e/(e + exp(6,13 − 0,0883e)) kedua dan penyesuaian suhu dari suhu stasiun (`wst(ob(j)%wst)`), tetapi perkalian dengan keduanya dikomentari; port meniru perhitungannya tanpa efek (`let _ = …`).
  - E\_t,day dan PET adalah total HRU hari itu (semua tanaman), bukan milik tanaman.
  - `HI_opt` di langkah 1 adalah `harv_idx` tanaman (dapat dinaikkan ke `wsyf` oleh panen biji), sedangkan batas atas memakai `hvsti` basis data.
  - Stasiun cuaca diambil dari objek bernomor j (nomor HRU); karena HRU berada di awal daftar objek, ini stasiun HRU itu sendiri (`swat-quirks.md`).

#### 8.6.7  `partition`

- **Lokasi:** `plant.rs`, `fn partition(&mut self, up: OrgMass, init: bool)`
- **Padanan SWAT+:** `pl_partition` (`pl_partition.f90`)
- **Tujuan:** membagi pertumbuhan hari ini (atau seluruh biomassa saat inisialisasi) ke akar, biji, daun dan batang, lalu menetapkan C, N dan P organ.
- **Masukan dan keluaran:** `up` (kg/ha), `root_frac`, `hi_adj`, `lai`, `lai_pot`, `cnyld`, `cpyld`; keluaran organ dan total.
- **Cara kerja:**
  1. `tot.m` += `up.m`, `tot.c` += `up.c`.
  2. Fraksi daun vegetatif: λ = 0,02 (perennial) atau 0,30, dikali `lai`/`lai_pot`.
  3. Fraksi target. Umbi: akar r = `root_frac`; atas tanah a = (1 − r)/(1 + `hi_adj`); biji s = 1 − r − a; daun λ·a; batang (1 − λ)·a. Lainnya: a = 1 − r; s = `hi_adj`; daun λ(1 − s); batang 1 − (λ + s).
  4. Harian (`init` = salah): untuk akar, biji, daun berurutan, bila massa < fraksi·`tot.m`, tambahkan selisihnya dari sisa `up.m` (tak melebihi sisa); seluruh sisa ke batang; `ab_gr.m` = batang + daun + biji.
  5. Inisialisasi (`init` = benar): `ab_gr.m` = a·B, akar r·B, daun/biji/batang = fraksinya × `ab_gr.m`.
  6. Karbon: daun 0,41, batang 0,46, biji 0,45, akar 0,46 dari massa; `ab_gr.c` dan `tot.c` dijumlah ulang.
  7. N dan P. Perennial (bila daun + batang + akar > 10⁻⁹): biji n = `cnyld`·m\_biji; sisa = N\_tot − n\_biji dibagi ke daun, batang, akar sebanding massa. Lainnya: n\_biji = `cnyld`·m\_biji; bila sisa < 0, n\_biji = 0,9·N\_tot dan sisa = 0,1·N\_tot; konsentrasi sisa = sisa/(B − m\_biji) diterapkan ke daun, batang, akar. P analog dengan `cpyld`. `ab_gr.n`/`ab_gr.p` = biji + daun + batang.
- **Kebiasaan SWAT+ / catatan port:**
  - Untuk perennial, bila N biji melebihi N total, Fortran menolkan N biji lalu menambahkan N biji (yang sudah 0) ke sisa, sehingga sisa tetap negatif dan N daun, batang, akar menjadi negatif. Dipertahankan.
  - Fraksi target di jalur harian dikalikan `tot.m`, sedangkan di jalur inisialisasi fraksi daun/biji/batang dikalikan `ab_gr.m`.
  - Penambahan `tot.c` di langkah 1 tertimpa di langkah 6.
  - N dan P total tidak berubah di sini; pembagian organ di `pl_nup`/`pl_pup` tertimpa.
  - Bila `lai_pot` = 0 (`blai` 0), λ menjadi NaN, seperti Fortran.

#### 8.6.8  `pl_mortality`

- **Lokasi:** `plant.rs`, `fn pl_mortality(&mut self, pl: &mut Plant)` (`impl Hru`)
- **Padanan SWAT+:** `pl_mortality` (`pl_mortality.f90`)
- **Tujuan:** pada akhir tahun, mematikan biomassa di atas maksimum perennial dan memindahkannya ke residu.
- **Masukan dan keluaran:** `ab_gr.m` (kg/ha), `bmx_peren` (t/ha), `bm_dieoff`; keluaran organ, `rsd`, `rsd_tot`, `soil_rsd`.
- **Cara kerja:**
  1. Kelebihan dikalikan (1 + D), D = `bm_dieoff`:

```latex
X = (1+D)\,\big(B_{ag}-1000\,B_{mx}\big)
```

2. Hanya bila X > 10⁻⁶ dan B\_ag > X: r = min(1, X/B\_ag), r₁ = max(0, 1 − r).
3. r·`ab_gr` ke residu permukaan tanaman dan `rsd_tot`; r·`rtfr`\[k\]·`root` ke residu lapisan k.
4. Total, atas tanah, daun, batang, biji, akar dikalikan r₁.

- **Kebiasaan SWAT+ / catatan port:**
  - Dipanggil untuk setiap tanaman pada `end_yr`, tumbuh atau tidak, semusim atau perennial.
  - Konsekuensi aritmetika syarat B\_ag > X: kematian hanya terjadi bila 0 < B\_ag − 1000B\_mx < B\_ag/(1 + D). Dengan D = 1, biomassa di atas dua kali maksimum tidak mati sama sekali, dan tanaman dengan `bmx_peren` = 0 tidak pernah terkena.
  - Akar yang mati masuk residu lapisan, tetapi `rsd_tot` hanya bertambah bagian atas tanah.

### 8.7  Panen, pematian, dan penanaman (`hru/mgt.rs`)

Fungsi berikut tinggal di `crates/swat-engine/src/hru/mgt.rs` dan dipanggil oleh operasi terjadwal (`mgt_sched`) dan tabel keputusan (`dtbl_day`). Semua kecuali `HarvOp::new` dan `plantop` adalah method `impl Hru` (blok `impl super::Hru` kedua di file itu). Tanaman yang diproses dikeluarkan sementara dari vektor komunitas (`std::mem::take`), sehingga fungsi yang menjumlah ulang residu komunitas memakai parameter `pl` untuk tanaman bernomor `me` dan vektor untuk yang lain.

**`HarvTyp`** — tipe panen `harv.ops`: `Biomass`, `Grain`, `Residue`, `Tree`, `Tuber`, `Peanuts`, `Stripper`, `Picker`, dan `Other` (tipe yang tidak dikenal SWAT+: tidak ada panen). **`HarvOp`** — satu baris `harv.ops`: `typ`, `hi_ovr` (indeks panen pengganti), `eff` (efisiensi panen), `bm_min` (biomassa minimum, kg/ha).

Ringkasan perilaku pemanggil (operasi terjadwal), dari kode `mgt_sched` dan `docs/engine1.md`:

- **`plant`**: tanaman bernama sama yang belum tumbuh menjadi tumbuh (`gro`), tidak dorman, dan di-*reset* oleh `plantop`.
- **`hvkl`** (panen dan matikan): untuk setiap tanaman tumbuh yang namanya cocok (atau `all`): panen bila `tot.m` > `bm_min`, lalu `killop`; hasil (`pl_yield` HRU) ditambahkan ke `yield_tot` dan `harv_num` + 1. Untuk setiap tanaman tumbuh (cocok atau tidak): `reset_stress`, `phuacc` = 0, dan heat unit dasar HRU `phubase` = 0. Tanaman tidak tumbuh hanya menerima panen residu (apa pun namanya).
- **`harv`** (panen saja): hanya tanaman tumbuh yang cocok; hasil dihitung walau biomassa di bawah minimum. Tanaman tidak tumbuh yang cocok hanya menerima panen residu.
- **`kill`**: `killop` untuk tanaman tumbuh yang cocok; `phuacc` = 0 untuk setiap tanaman.
- Tabel keputusan: `harvest` tidak menambah `harv_num`; `harvest_kill` memanen dan mematikan tanaman bernama baik tumbuh maupun tidak dan tidak menyentuh heat unit dasar; `plant` dari tabel tidak memanggil `plantop` (rincian di bab manajemen dan `swat-quirks.md`, *Decision tables*).

#### 8.7.1  `HarvOp::new`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn new(project: &Project, name: &str) -> HarvOp`
- **Padanan SWAT+:** pencarian baris `harv.ops` saat membaca jadwal/tabel keputusan (penunjuk tetap di elemen 0 bila tidak ditemukan)
- **Tujuan:** membentuk operasi panen dari nama baris `harv.ops`.
- **Masukan dan keluaran:** tabel `harv.ops` proyek, nama; keluaran `HarvOp`.
- **Cara kerja:** cari baris; petakan teks `typ` ke `HarvTyp` (tak dikenal → `Other`); baca `hi_ovr`, `eff`, `bm_min` (0 bila tak ada).
- **Kebiasaan SWAT+ / catatan port:** nama yang tidak ada memberi elemen 0 (semua nol, tipe kosong) sehingga operasi itu tidak memanen apa pun.

#### 8.7.2  `harvest`

- **Lokasi:** `mgt.rs`, `pub(super) fn harvest(&mut self, pl: &mut Plant, me: usize, op: &HarvOp, eff: f32, kill: bool)`
- **Padanan SWAT+:** percabangan tipe panen di `mgt_sched` dan `actions` (`mgt_sched.f90`, `actions.f90`)
- **Tujuan:** memilih rutin panen menurut tipe dan apakah tanaman juga dimatikan.
- **Masukan dan keluaran:** tanaman, nomor, operasi, efisiensi aksi (untuk residu), `kill`.
- **Cara kerja:**

| Tipe | Panen saja | Panen dan matikan |
|---|---|---|
| `biomass` | `harvbiomass` | `harvbiomass` |
| `grain`, `picker` | `harvgrain` | `harvgrain` |
| `residue` | `harvresidue` | `harvresidue` |
| `tree` | `harvbiomass` | tidak ada |
| `tuber`, `peanuts` | `harvtuber` | `harvtuber` |
| `stripper` | `harvbiomass` | `harvgrain` |
| lainnya | tidak ada | tidak ada |

- **Kebiasaan SWAT+ / catatan port:** pembagian `tree`/`stripper` sesuai `docs/engine1.md` (*Harvests and kills*).

#### 8.7.3  `harvbiomass`

- **Lokasi:** `mgt.rs`, `fn harvbiomass(&mut self, pl: &mut Plant, me: usize, op: &HarvOp)`
- **Padanan SWAT+:** `mgt_harvbiomass` (`mgt_harvbiomass.f90`)
- **Tujuan:** panen sebagian biomassa di atas tanah (hijauan, tandan buah, dsb.).
- **Masukan dan keluaran:** organ (kg/ha), `hi_ovr`, `eff`, `bm_min`; keluaran `self.pl_yield`, organ, `rsd`, `rsd_tot`, `lai`, `phuacc`, `root_frac`.
- **Cara kerja:**
  1. h = `hi_ovr`·`eff`; hasil Y = h·(biji + daun + batang), dicatat ke `pl_yield` apa pun yang terjadi kemudian.
  2. Hanya bila B_ag − Y_m > `bm_min`: kurangkan h·organ dari biji, daun, batang; Y dari total dan atas tanah.
  3. Residu tanaman += (1 − `eff`)·Y; `rsd_tot` dijumlah ulang dari residu semua tanaman.
  4. Bila `tot.m` > 0,001: LAI dan `phuacc` dikalikan (1 − 0,5h); `root_frac` = m_akar/B. Selain itu `tot.m`, LAI, `phuacc` = 0.

```latex
Y = h\,(S + L + T),\qquad h = HI_{ovr}\,\eta,\qquad R \mathrel{+}= (1-\eta)\,Y,\qquad LAI \leftarrow LAI\,(1-0.5h)
```

- **Kebiasaan SWAT+ / catatan port:** hasil dihitung walau panen tidak diambil karena biomassa tersisa terlalu sedikit (`engine1.md`). Bagian (1 − `eff`)·Y yang masuk residu ditambahkan di samping Y yang sudah dikeluarkan seluruhnya dari tanaman; secara aritmetika massa yang keluar dari tanaman adalah Y, sedangkan hasil + residu baru = (2 − `eff`)·Y. Perkalian `scale(1.0, …)` meniru ekspresi Fortran tanpa efek.

#### 8.7.4  `harvtuber`

- **Lokasi:** `mgt.rs`, `fn harvtuber(&mut self, pl: &mut Plant, eff: f32)`
- **Padanan SWAT+:** `mgt_harvtuber` (`mgt_harvtuber.f90`)
- **Tujuan:** panen umbi/kacang tanah: hasil diambil dari akar.
- **Masukan dan keluaran:** organ, `eff`; keluaran `pl_yield`, `soil_rsd`, `rsd`, `rsd_tot`.
- **Cara kerja:**
  1. Y = `eff`·akar; akar −= Y.
  2. `rootfr`; akar sisa ke residu lapisan menurut `rtfr`.
  3. Seluruh bagian atas tanah ke residu permukaan tanaman dan `rsd_tot`.
  4. Semua organ dinolkan.
- **Kebiasaan SWAT+ / catatan port:** tanaman tetap berstatus tumbuh (`gro` tidak diubah) dan tumbuh lagi dari nol bila tidak dimatikan (`engine1.md`).

#### 8.7.5  `harvresidue`

- **Lokasi:** `mgt.rs`, `fn harvresidue(&mut self, pl: &mut Plant, me: usize, op: &HarvOp, harveff: f32)`
- **Padanan SWAT+:** `mgt_harvresidue` (`mgt_harvresidue.f90`)
- **Tujuan:** mengangkut residu permukaan (misalnya jerami dibal).
- **Masukan dan keluaran:** residu permukaan setiap tanaman (kg/ha), efisiensi; keluaran residu, `rsd_tot`.
- **Cara kerja:**
  1. e = `harveff` aksi, atau `eff` baris bila `harveff` < 10⁻⁵; e_net = e·`hi_ovr`.
  2. Untuk residu setiap tanaman R: diangkat A = e_net·R; bila R_m − A_m < `bm_min`, A = ((R_m − `bm_min`)/R_m)·R.
  3. R −= A; `rsd_tot` −= A (dinolkan bila < 10⁻⁶).
- **Kebiasaan SWAT+ / catatan port:** bila residu sudah di bawah `bm_min`, A negatif dan residu justru bertambah sampai `bm_min` (`engine1.md`). `pl_yield` tidak diisi, sehingga pemanggil mencatat hasil panen sebelumnya di HRU itu sebagai hasil panen residu.

#### 8.7.6  `harvgrain`

- **Lokasi:** `mgt.rs`, `fn harvgrain(&mut self, pl: &mut Plant, eff: f32)`
- **Padanan SWAT+:** `mgt_harvgrain` (`mgt_harvgrain.f90`)
- **Tujuan:** panen biji.
- **Masukan dan keluaran:** biji (kg/ha), `eff`, `wsyf`; keluaran `pl_yield`, `hact` lapisan 1, organ.
- **Cara kerja:**
  1. `harv_idx` = max(`harv_idx`, `wsyf`).
  2. Biji dikeluarkan dari total dan atas tanah; Y = `eff`·biji.
  3. (1 − `eff`)·biji ditambahkan ke humus aktif (`hact`) lapisan teratas; biji = 0.
- **Kebiasaan SWAT+ / catatan port:** bagian yang luput dari panen masuk `hact`, yang tidak dibaca model karbon (`swat-quirks.md`). Indeks panen yang dinaikkan hanya mempengaruhi `seed_gro` berikutnya.

#### 8.7.7  `killop`

- **Lokasi:** `mgt.rs`, `fn killop(&mut self, pl: &mut Plant, me: usize)`
- **Padanan SWAT+:** `mgt_killop` (`mgt_killop.f90`) dengan `plg_zero`
- **Tujuan:** mematikan tanaman: bagian atas tanah ke residu permukaan, akar ke residu lapisan.
- **Masukan dan keluaran:** organ, lapisan; keluaran residu, total komunitas, keadaan tanaman.
- **Cara kerja:**
  1. `rootfr`; `rsd` += `ab_gr`; `rsd_tot` dijumlah ulang.
  2. Akar ke residu lapisan menurut `rtfr`.
  3. `tot_com` dan `ab_gr_com` dikurangi milik tanaman.
  4. Organ dinolkan; `plg_zero` (tinggi, LAI, `plet`, `plpet`, `laimxfr`, `hi_adj`, `hi_prev`, `olai`, `dphu`, `d_senes`, `root_dep`, `root_frac`, `rtfr`, `n_fr`, `p_fr` = 0); `reset_stress`; `gro` = `idorm` = salah; `phuacc` = 0; `curyr_mat` = 1.
- **Kebiasaan SWAT+ / catatan port:** pada panen-dan-matikan, biji yang dipanen sudah keluar dari tanaman tetapi tidak dari total komunitas, sehingga untuk sisa hari (limpasan, erosi) total komunitas masih memuat biji itu (`swat-quirks.md`).

#### 8.7.8  `plantop`

- **Lokasi:** `mgt.rs`, fungsi bebas `fn plantop(pl: &mut Plant, zmx: f32) -> f32`
- **Padanan SWAT+:** `mgt_plantop` (`mgt_plantop.f90`)
- **Tujuan:** memulai tanaman dari nol saat penanaman.
- **Masukan dan keluaran:** tanaman, `zmx` HRU (mm); keluaran `zmx` baru (mm).
- **Cara kerja:** `phuacc` = 0; semua organ = 0; `plet`, `plpet`, `laimxfr`, `hi_adj`, `olai`, `dphu` = 0; `reset_stress`; kembalikan min(`zmx`, 1000·`rdmx`).
- **Kebiasaan SWAT+ / catatan port:** `zmx` HRU dipangkas untuk selamanya oleh tanaman yang ditanam (`swat-quirks.md`). `hi_prev` dan LAI tidak dinolkan di sini.

### 8.8  Ringkasan kebiasaan, koreksi, dan cakupan fungsi

**Koreksi `--fix-swat` di bab ini** (`docs/swat-fixes.md`):

| Lokasi SWAT+ | Perilaku asli (dipertahankan secara bawaan) | Koreksi |
|---|---|---|
| `pl_nut_demand` (`pl_nupd`, `pl_pupd`) | hanya elemen setelah tanaman terakhir yang dinolkan; tanaman lain memakai permintaan kemarin atau HRU lain | permintaan setiap tanaman dimulai dari nol |

**Kebiasaan SWAT+ yang dipertahankan** (ringkas): indeks panen > 0,7 menjadi 0,02; stres suhu selalu berbentuk sinus; lapisan kedua mengabaikan `epco`; hari tanam `igrow` dibagi antar-HRU; populasi `plant.ini` diabaikan; dormansi perennial hanya membuang biji dan menolkan LAI; perennial `temp_gro` tidak menggugurkan massa daun; total berjalan `nplnt`/`pplnt`/`fixn` dalam komunitas; pembagian LAI menurut tinggi; perennial tropis tanpa dormansi; N organ perennial negatif bila N biji berlebih; tumpang tindih lapisan terakhir di `rootfr`; syarat kematian akhir tahun yang hanya aktif untuk kelebihan kecil; hasil dihitung walau panen tidak terjadi; residu bertambah oleh panen residu di bawah minimum.

**Cakupan fungsi.** `plant.rs`: 31 fungsi (`add`, `sub`, `scale`, `PartFracs::default`, `PlantDb::new`, `perennial`, `annual`, `PlantBasin::new`, `exp_w`, `nuts`, `Saved::default`, `plant_init`, `reset_stress`, `root_gro`, `rootfr`, `seed_gro`, `partition`, `tstr`, `nupd`, `pupd`, `leaf_gro`, `leaf_senes`, `pl_community`, `pl_waterup`, `pl_grow`, `pl_dormant`, `pl_biomass_gro`, `pl_nup`, `pl_nfix`, `pl_pup`, `pl_mortality`). Tambahan: `ascrv` (`hru/mod.rs`) dan delapan fungsi `hru/mgt.rs` (`HarvOp::new`, `harvest`, `harvbiomass`, `harvtuber`, `harvresidue`, `harvgrain`, `killop`, `plantop`). Total 40 fungsi.
