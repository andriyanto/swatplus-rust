## Bab 10  Manajemen lahan dan tabel keputusan

Bab ini menjelaskan bagaimana engine menjalankan pengelolaan lahan pada setiap HRU: jadwal operasi bertanggal dan berbasis *heat unit* dari `management.sch` (tanam, panen, panen-dan-mati, mati, pengolahan tanah, irigasi, pemupukan, pupuk kandang, dan `skip`), tabel keputusan `lum.dtl` (kondisi dan aksi yang dievaluasi setiap hari), seluruh jenis panen dari `harv.ops`, operasi mati (*kill*), serta pemupukan ke tanah dan ke air sawah. Seluruh kode berada di satu file, `crates/swat-engine/src/hru/mgt.rs` (20 fungsi), dengan pencampuran tanah oleh pengolahan (`tillmix_cswat0`, `tillmix_cswat1`, `tillmix_wet`) berada di `hru/carbon.rs` dan dibahas di bab lain. Padanan Fortran utamanya adalah `read_mgtops.f90`/`mgt_read_mgtops.f90`, `plant_init.f90`, `dtbl_lum_read.f90`, `proc_cond.f90`, `conditions.f90`, `cond_real.f90`, `cond_integer.f90`, `actions.f90`, `mgt_operatn.f90`, `mgt_sched.f90`, `mgt_plantop.f90`, `mgt_harvbiomass.f90`, `mgt_harvgrain.f90`, `mgt_harvtuber.f90`, `mgt_harvresidue.f90`, `mgt_killop.f90`, `pl_fert.f90`, `pl_fert_wet.f90`, `pl_manure.f90`, `jdt.f90` dan `time_control.f90`.

Posisi dalam alur harian `Hru::day` (`hru/mod.rs`, padanan `hru_control.f90`): (1) pada awal hari irigasi dan pupuk harian dinolkan; (2) tabel keputusan dievaluasi lebih dulu (`dtbl_day`), memakai cekaman, LAI dan heat unit kemarin karena cekaman belum di-reset; (3) penghitung hari sejak tanam/panen/mati/irigasi bertambah satu; (4) setelah evapotranspirasi (`et_act`) dan sebelum limpasan permukaan, operasi terjadwal dijalankan (`mgt_operatn`) selama tahun belum di-*skip*; (5) di akhir tahun (belahan utara) atau hari ke-181 (belahan selatan) `mgt_new_year` melompati `skip` dan jadwal berlanjut. Irigasi yang dihasilkan (`irr_applied`, `irr_runoff`) dibaca oleh neraca air tanah atau neraca air sawah (`hru/wetland.rs`) pada hari yang sama. Validasi bit-for-bit dilakukan pada Ames\_sub1 (irigasi tabel keputusan, pupuk), Batanghari (tabel generik `pl_hv_*`), Osu\_1hru dan Ps\_paddy (sawah), serta varian Pessel Ps\_harv dan Ps\_hu (`docs/engine1.md`, `docs/paddy.md`).

### 10.1  Struktur data dan pembacaan basis data operasi

Semua struktur di bawah ini dibangun sekali pada inisialisasi dari `Project` (berkas input yang sudah diurai) dan disimpan per HRU di `Hru::mgt: Option<Mgt>`.

| Tipe | Field utama | Keterangan |
| --- | --- | --- |
| `OpKind` (enum) | `Plant`; `HarvKill {op: HarvOp, eff}`; `Harv {op, eff}`; `Kill`; `Till {effmix, deptil}`; `Irr {amt_mm, eff, surq}`; `Fert(Fert)`; `Manure(Manure)`; `Skip` | Jenis operasi jadwal (`plnt`, `hvkl`, `harv`, `kill`, `till`, `irrm`, `fert`, `manu`, `skip`). `eff` = `op3` (efisiensi panen residu pengganti); `effmix` fraksi pencampuran 0–1; `deptil` kedalaman olah (mm); `amt_mm` mm; `eff` irigasi 0–1; `surq` fraksi irigasi yang menjadi limpasan. |
| `Fert` | `fminn`, `fminp`, `forgn`, `forgp`, `fnh3n`, `surf_frac`, `kg` | Fraksi N/P mineral dan organik pupuk (`fertilizer.frt`), fraksi amonium dari N mineral, fraksi yang tinggal di permukaan (`chem_app.ops`), dosis (kg/ha). |
| `Manure` | seperti `Fert` + `frac_water` | Baris `manure_om.frt`; `kg` dosis kering (kg/ha); `frac_water` fraksi air pupuk kandang. |
| `Op` | `kind`, `mon`, `day`, `husc`, `op_char` | Satu operasi: bulan dan tanggal (0 bila berbasis heat unit), ambang heat unit `husc` (fraksi PHU tanaman atau heat unit dasar-0 tahunan; 0 = berdasarkan tanggal), nama tanaman sasaran. |
| `Mgt` | `ops`, `cur_op` (0-based), `yr_skip`, `dtbls`, `auto_crops`, `rot_yr` | Jadwal HRU dan posisinya (`hru%cur_op`), bendera tahun dilompati, tabel keputusan jadwal (`sched%num_autos`), tanaman tabel generik (`sched%auto_crop`), tahun rotasi (`pcom%rot_yr`). |
| `CondVar` (enum) | `WStress`, `IrrYear`, `PhuPlant`, `LeafArea`, `PhuBase0`, `SoilWater {targ, op}`, `Jday`, `YearRot`, `Month`, `DaysPlant/Harv/Kill/Irr`, `PrecipCur`, `WeirH`, `WetDepth`, `ProbUnif {first}` | Variabel kondisi tabel keputusan yang didukung. |
| `SwTarget` | `Wp`, `Fc`, `Ul` | Acuan kondisi `soil_water`: titik layu (dipakai sebagai 0), kapasitas lapang, porositas (batas atas). |
| `HarvTyp` | `Biomass`, `Grain`, `Residue`, `Tree`, `Tuber`, `Peanuts`, `Stripper`, `Picker`, `Other` | Jenis panen `harv.ops`; `Other` = jenis tak dikenal SWAT+ (tidak ada panen). |
| `HarvOp` | `typ`, `hi_ovr`, `eff`, `bm_min` | Indeks panen pengganti (kg/kg), efisiensi panen (0–1), biomassa minimum (kg/ha). |
| `Action` (enum) | `Irrigate`, `Till`, `Fertilize`, `WeirHeight`, `Puddle`, `Plant`, `HarvKill`, `Harvest`, `Kill`, `RotReset` | Aksi tabel keputusan beserta parameter dan batas per tahun `max_yr` (= `Int(const2)`). |
| `Dtbl` | `generic`, `alts`, `conds: Vec<(CondVar, lim_const, alternatif)>`, `acts: Vec<(Action, hasil per alternatif)>`, `num_actions`, `frac_app`, `apply_day` | Tabel keputusan siap pakai; `num_actions` (mulai 1) menghitung pelaksanaan tiap aksi tahun ini; `frac_app` dan `apply_day` untuk `prob_unif`. |

#### 10.1.1  `HarvOp::new`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn new(project: &Project, name: &str) -> HarvOp`
- **Padanan SWAT+:** pencarian `harvop_db` di `mgt_read_mgtops.f90`/`dtbl_lum_read.f90` atas data `mgt_read_harvops.f90`
- **Tujuan:** Membentuk deskripsi operasi panen dari baris `harv.ops` bernama `name`.
- **Masukan dan keluaran:** nama baris; keluaran `HarvOp` dengan `typ`, `hi_ovr`, `eff`, `bm_min` (kg/ha).
- **Cara kerja:**
  1. Cari baris bernama `name` di tabel `harv_ops`; ambil kolom teks `typ`.
  2. Petakan teks ke `HarvTyp` (`biomass`, `grain`, `residue`, `tree`, `tuber`, `peanuts`, `stripper`, `picker`); teks lain atau baris tak ada menjadi `Other`.
  3. Ambil `hi_ovr`, `eff`, `bm_min` lewat `row_real` dengan bawaan 0.
- **Kebiasaan SWAT+ / catatan port:** Nama yang tidak ditemukan memberi elemen 0 basis data SWAT+, yaitu semua nol; tipe kosong berarti tidak ada panen sama sekali, tetapi operasi tetap dihitung (lihat 10.4).

#### 10.1.2  `row_real`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn row_real(t: Option<&Table>, name: &str, field: &str, default: f32) -> f32`
- **Padanan SWAT+:** tidak ada; khusus port (meniru pencarian nama dalam loop `do ... if (name == db(i)%name)` yang meninggalkan indeks 0 bila gagal).
- **Tujuan:** Membaca satu nilai real dari baris bernama di sebuah tabel basis data, dengan nilai bawaan.
- **Masukan dan keluaran:** tabel opsional, nama baris, nama kolom, nilai bawaan; keluaran `f32`.
- **Cara kerja:**
  1. Bila tabel ada dan baris `name` ditemukan (`Table::find`), kembalikan `real(r, field)` atau bawaan bila kolom tidak terurai.
  2. Selain itu kembalikan bawaan.
- **Kebiasaan SWAT+ / catatan port:** Bawaan dipilih sama dengan nilai inisialisasi elemen 0 tipe Fortran, misalnya `amt_mm` irigasi 25,4 mm dan semua fraksi pupuk 0.

#### 10.1.3  `jdt`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn jdt(ndays: &[i32; 13], day: i32, mon: i32) -> i32`
- **Padanan SWAT+:** fungsi `jdt` di `jdt.f90`
- **Tujuan:** Mengubah bulan dan tanggal operasi menjadi hari Julian, untuk menentukan operasi awal jadwal.
- **Masukan dan keluaran:** tabel hari kumulatif awal bulan `ndays` (13 elemen), tanggal, bulan; keluaran hari ke-n dalam tahun (0 bila bulan 0).
- **Cara kerja:**
  1. Bila bulan 0 (operasi heat unit atau `skip`), hasil 0.
  2. Bila bulan 1 atau 2: hari = `ndays[mon-1] + day`.
  3. Bila bulan 3 atau lebih: hari = `ndays[mon-1] - 1 + day`.

```latex
jdt = \begin{cases} 0 & m = 0 \\ N_{m} + d & m \le 2 \\ N_{m} - 1 + d & m \ge 3 \end{cases}
```

- **Kebiasaan SWAT+ / catatan port:** Pengurangan 1 untuk bulan ≥ 3 mengasumsikan tabel tahun kabisat (`ndays_leap`, Maret = 60). Bila yang diberikan adalah tabel non-kabisat, hasilnya satu hari lebih awal; port menyalin rumus apa adanya dan menerima tabel `ndays` yang sama dengan yang dipakai inisialisasi tanaman.

### 10.2  Membangun tabel keputusan dan jadwal

Dua konstruktor ini menerjemahkan teks `lum.dtl` dan `management.sch` menjadi struktur bertipe. Semua yang belum di-*port* ditolak dengan `Err(String)` yang menyebut apa yang tidak didukung, sehingga simulasi tidak berjalan diam-diam dengan perilaku berbeda dari SWAT+.

#### 10.2.1  `Dtbl::new`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn new(project: &Project, t: &swat_model::DecisionTable, auto_name: &str) -> Result<Dtbl, String>`
- **Padanan SWAT+:** `dtbl_lum_read.f90` (pembacaan dan pencarian pointer basis data), `hru_dtbl_actions_init.f90` (penghitung aksi), bagian pemilihan variabel di `conditions.f90` dan `actions.f90`
- **Tujuan:** Mengubah satu tabel keputusan `lum.dtl` menjadi `Dtbl`: kondisi dengan operator per alternatif, aksi dengan hasil (`y`/`n`) per alternatif, dan parameter aksi yang sudah dicari di basis data operasi.
- **Masukan dan keluaran:** tabel keputusan terurai (`conds`, `alt`, `acts`, `outcomes`, `alts`), nama otomatis dari jadwal (`auto_name`); keluaran `Dtbl` atau pesan galat.
- **Cara kerja:**
  1. Tentukan `generic`: 1 untuk `pl_hv_summer1` dan `pl_hv_winter1`, 2 untuk `pl_hv_summer2`, selain itu 0. Tabel generik mengambil nama tanaman dari jadwal, bukan dari kolom `option` aksi.
  2. Untuk setiap kondisi, petakan `var` ke `CondVar`: `month`, `days_plant`, `days_harv`, `days_kill`, `days_irr`, `precip_cur` (harus `ob = hru`), `weirh`, `wet_depth`, `prob_unif`, `w_stress`, `irr_year`, `phu_plant`, `leaf_area`, `phu_base0` (harus `ob = hru`), `soil_water`, `jday`, `year_rot`. Variabel lain ditolak.
  3. Untuk `prob_unif`, kolom objek (`ob`) dibaca ulang sebagai bilangan real dan menjadi probabilitas tabel `frac_app`; nomor objek (`ob_num`) menjadi hari pertama jendela.
  4. Untuk `soil_water`, `lim_var` harus `wp`, `fc` atau `ul`, dan `lim_op` harus salah satu dari `*`, `+`, `-`, `/` (disimpan sebagai byte).
  5. Kondisi dengan `ob_num` ≠ 0 (kondisi pada objek lain) ditolak, kecuali `prob_unif`. Setiap operator alternatif harus salah satu dari `-`, `<`, `>`, `<=`, `>=`, `=`, `/=`.
  6. Untuk setiap aksi, petakan `typ`:
     - `irrigate`: sumber `aqu`, `cha`, `sdc`, `res` ditolak (hanya sumber tak terbatas); `amt` = `const` (mm), `eff` dan `surq` dari baris `irr.ops` bernama `option`, `max_yr` = `Int(const2)`, `ponding` benar bila nama aksi `ponding`, dan `const2` disimpan apa adanya (ambang kedalaman sawah).
     - `plant`, `harvest_kill`, `rot_reset`, `till`, `fertilize`, `weir_height`, `puddle` dengan `ob_num` ≠ 0 ditolak.
     - `plant`: bila `file_pointer` ada di `transplant.ops`, ditolak (transplantasi belum di-*port*); selain itu `Plant { crop: option }`.
     - `till`: `effmix`, `deptil` dari `tillage.til` bernama `option`.
     - `fertilize`: fraksi dari `fertilizer.frt` bernama `option`, `surf_frac` dari `chem_app.ops` bernama `file_pointer`, dosis `kg` = `const`.
     - `weir_height`: harus `option = wet` (tinggi pelimpah reservoir ditolak); `h_mm` = `const`.
     - `puddle`: baris `puddle.ops` bernama `option` dicari dari belakang (yang terakhir dengan nama itu), dan tillage bernama sama dengan *nama aksi* harus ada; diambil `wet_hc`, `sed` (ppm), `effmix`, `deptil`.
     - `harvest_kill` dan `harvest`: `crop` = `option`, `op` = `HarvOp::new(file_pointer)`, `eff` = `const`.
     - `kill`: `crop` = `option`. `rot_reset`: `yr` = `max(const, 1)` dibulatkan ke bawah.
  7. Hasil per alternatif disimpan sebagai `bool` (`y` = benar); `num_actions` diisi 1 untuk setiap aksi; `apply_day` = 0.
- **Kebiasaan SWAT+ / catatan port:** (a) Probabilitas `prob_unif` berasal dari kolom *objek*, karena `dtbl_lum_read` membaca kolom itu lagi sebagai real (contoh Osu\_1hru: `prob_unif 1.0 120 null - 130`). (b) `puddle` mengambil baris terakhir bernama sama karena loop pencarian Fortran tidak keluar saat ketemu; baris pertama berkas dianggap judul sehingga baris pertama data adalah header bagi SWAT+. (c) Tillage untuk `puddle` dicari dengan nama aksi, bukan `option`. (d) Penghitung aksi mulai dari 1, sehingga `const2 = 20` mengizinkan 20 kali setahun. (e) `puddle.ops`, `manure_db.frt`, `manure_om.frt`, `transplant.ops` dibuka dengan nama tetap apa pun isi `file.cio` (`docs/swat-quirks.md`).

#### 10.2.2  `Mgt::new`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `pub fn new(project: &Project, s: &MgtSchedule, ndays: &[i32; 13], rot_yr_ini: i32, day_start: i32) -> Result<Mgt, String>`
- **Padanan SWAT+:** `read_mgtops.f90`/`mgt_read_mgtops.f90` (operasi), `proc_cond.f90` (tabel keputusan jadwal), bagian operasi awal di `plant_init.f90`
- **Tujuan:** Membangun jadwal operasi satu HRU beserta tabel keputusannya, dan menentukan operasi pertama yang akan dijalankan.
- **Masukan dan keluaran:** jadwal `s` (`ops`, `autos`, `auto_crops`), tabel hari kumulatif bulan, tahun rotasi awal komunitas tanaman `rot_yr_ini`, hari mulai simulasi `day_start`; keluaran `Mgt` dengan `cur_op` 0-based.
- **Cara kerja:**
  1. Untuk setiap nama tabel otomatis di `s.autos`, cari tabel `lum.dtl` bernama sama dari belakang (yang terakhir menang), lalu `Dtbl::new`. Nama yang tidak ada adalah galat.
  2. Untuk setiap operasi, bentuk `OpKind`:
     - `plnt` → `Plant`; `hvkl`/`harv` → `HarvKill`/`Harv` dengan `HarvOp` dari `op_plant` (kolom kedua) dan `eff` = `op3`; `kill` → `Kill`.
     - `till` → `effmix`, `deptil` dari `tillage.til` bernama `op_char`.
     - `irrm` → `amt_mm` (bawaan 25,4 mm), `eff`, `surq` dari `irr.ops` bernama `op_char`.
     - `fert` → fraksi dari `fertilizer.frt` bernama `op_char`, `surf_frac` dari `chem_app.ops` bernama `op_plant`, `kg` = `op3`.
     - `manu` → ditolak bila `cswat = 2`; selain itu nomor baris `manure_db.frt` bernama `op_char` dipakai sebagai nomor baris `manure_om.frt` (0 = semua nol), `surf_frac` dari `chem_app.ops`, `kg` = `op3`.
     - `skip` → `Skip`; operasi lain ditolak.
  3. Hitung hari Julian tiap operasi dengan `jdt`.
  4. Penentuan operasi awal (`plant_init`), hanya bila operasi pertama bertanggal (`jday > 0`): telusuri operasi sambil menghitung tahun rotasi `irot` (mulai 1, bertambah setiap `skip` atau setiap hari Julian lebih kecil dari sebelumnya); operasi pertama `k` dengan `irot = rot_yr_ini` dan `day_start ≤ jday_k` memberi `iop = k + 1` (1-based); bila tidak ada, `iop = n + 1`. Operasi awal adalah `iop − 1`, dibatasi ke `[1, n]`.
  5. Tabel generik tanpa cukup tanaman di `auto_crops` (kurang dari `generic`) ditolak.
  6. Kembalikan `Mgt` dengan `cur_op` dikonversi ke 0-based, `yr_skip = false`, `rot_yr = rot_yr_ini`.
- **Kebiasaan SWAT+ / catatan port:** (a) *Operasi pertama adalah satu sebelum yang dimaksud*: loop Fortran meninggalkan indeks satu lewat dari operasi yang dicari, lalu dikurangi satu; untuk simulasi yang mulai hari 1 hasilnya tetap operasi 1. (b) Pemanggil (`hru/mod.rs`) mengembalikan `cur_op` ke 0 bila HRU tidak punya tanaman (default Fortran 1), dan memakai `rot_yr_ini = 1` bila komunitas kosong. (c) Operasi `fert` menyebut pupuk dulu lalu metode aplikasi; Ames\_sub1 menuliskannya terbalik sehingga pupuknya elemen 0 (nol) dan tidak ada yang diberikan. Metode yang tidak ditemukan memberi `surf_frac = 0`, sehingga semua pupuk masuk lapisan kedua. (d) `pl_manure` membaca `manure_om.frt` pada nomor baris `manure_db.frt`, bukan pada kolom `org_min`-nya.

### 10.3  Evaluasi harian tabel keputusan

Sebuah tabel keputusan memiliki *A* alternatif (kolom). Setiap alternatif mula-mula "kena" (*hit*); setiap kondisi dengan operator pada alternatif itu dapat menggugurkannya; operator `-` berarti kondisi itu tidak berlaku bagi alternatif tersebut. Sebuah aksi dijalankan bila ada sedikitnya satu alternatif yang masih kena dan bernilai `y` untuk aksi itu. Jadi kondisi dalam satu alternatif digabung dengan DAN, dan alternatif-alternatif digabung dengan ATAU.

#### 10.3.1  `cond_test`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `pub(crate) fn cond_test<T: PartialOrd>(hit: &mut [bool], alt: &[String], cur: T, tbl: T)`
- **Padanan SWAT+:** `cond_real.f90` dan `cond_integer.f90`
- **Tujuan:** Menggugurkan alternatif yang operatornya tidak dipenuhi oleh nilai saat ini terhadap nilai batas tabel.
- **Masukan dan keluaran:** larik `hit` (diubah di tempat), operator per alternatif, nilai sekarang `cur`, nilai batas `tbl`; generik atas tipe yang dapat dibandingkan (`f32` untuk `cond_real`, `i32` untuk `cond_integer`).
- **Cara kerja:**
  1. Lewati alternatif dengan operator `-` atau yang sudah gugur.
  2. Alternatif gugur bila: `<` dan `cur ≥ tbl`; `>` dan `cur ≤ tbl`; `<=` dan `cur > tbl`; `>=` dan `cur < tbl`; `=` dan `cur ≠ tbl`; `/=` dan `cur = tbl`.
- **Kebiasaan SWAT+ / catatan port:** Perbandingan `=` pada bilangan real adalah kesamaan persis, seperti Fortran. Fungsi ini juga dipakai modul lain (visibilitas `pub(crate)`), misalnya tabel keputusan reservoir.

#### 10.3.2  `Hru::dtbl_day`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `pub(super) fn dtbl_day(&mut self, c: &DayCtx, phubase0: f32, b: &HruBasin, saved: &mut plant::Saved)`
- **Padanan SWAT+:** `conditions.f90` lalu `actions.f90` sebagaimana dipanggil `hru_control.f90` untuk `sched%num_autos` tabel; reset tahunan dari `time_control.f90`
- **Tujuan:** Mengevaluasi semua tabel keputusan jadwal HRU pada awal hari dan menjalankan aksi yang terpicu.
- **Masukan dan keluaran:** konteks hari `c` (hari Julian `day`, bulan `mo`, `day_end_yr`, `end_yr`, presipitasi stasiun `station_precip`), heat unit dasar-0 stasiun `phubase0` (°C·hari), pengaturan cekungan `b` (`cswat`, `carbon`, `fixes`), status Fortran yang disimpan `saved` (benih acak `rndseed_cond`). Keluaran berupa perubahan status HRU: `irr_applied`, `irr_runoff` (mm), `irr_yr`, `irr_hmax`/`irr_hmin` (sawah), lapisan tanah, tanaman, `harvests`, penghitung hari, `rot_yr`.
- **Cara kerja:**
  1. Ambil `Mgt` keluar dari HRU (`take`) agar HRU dan jadwal dapat diubah bersamaan; kembalikan di akhir.
  2. Untuk setiap tabel, `hit = [true; alts]`, lalu untuk setiap kondisi `(var, lim, alt)`:
     - `PhuPlant` tanpa tanaman: semua alternatif gugur.
     - `WStress`: rata-rata `strsw` tanaman yang tumbuh (`gro`); 1 bila tidak ada.
     - `IrrYear`: `irr_yr` (mm). `PhuPlant`: `phuacc` tanaman pertama dengan `phuacc > 10⁻⁶`, selain itu tanaman 1. `LeafArea`: `lai_sum` komunitas. `PhuBase0`: `phubase0`. `PrecipCur`: presipitasi stasiun hari ini (mm).
     - `SoilWater`: nilai saat ini `sw` (mm, seluruh profil); batas dihitung dari acuan `v` (0 untuk `wp`, `sumfc`, `sumul`) dan `lim` sesuai `lim_op`.
     - `WeirH`/`WetDepth`: tinggi pelimpah atau kedalaman genangan sawah (m), dengan batas `lim/1000` (tabel dalam mm); 0 bila bukan sawah.
     - Variabel bilangan bulat (`Jday`, `YearRot` = `rot_yr`, `Month`, `DaysPlant`, `DaysHarv`, `DaysKill`, `DaysIrr`): `cond_test` dengan `lim` dibulatkan ke bawah ke `i32`.
     - `ProbUnif`: lihat langkah 3.
     - Lainnya: `cond_test(hit, alt, cur, targ)` dalam `f32`.
  3. `ProbUnif { first }` (hari acak dalam jendela): pada hari `day = first` saja, undi hari aplikasi dengan `aunif(rndseed_cond)`:
     - Jendela di dalam satu tahun (`lim > first`): bila `r₁ < frac_app`, `days_tot = lim − first + 1`, `apply_day = int(r₂ · days_tot) + first + 1`; selain itu `apply_day = 0`.
     - Jendela melewati akhir tahun: bila `r₁ < frac_app`, maka bila `day ≥ lim`, `days_tot = day_end_yr − first + 1` dan `apply_day = int(r₂ · days_tot) + first + 1`; bila tidak, `days_tot = lim` dan `apply_day = int(r₂ · days_tot) + 1`. Selain itu `apply_day = 0`.
     - Pada hari apa pun yang bukan `apply_day`, semua alternatif berop `=` gugur (operator lain tidak diuji).
  4. Untuk setiap aksi yang terpicu, tentukan tanaman: `generic = 1` → `auto_crops[0]`; `generic = 2` → `auto_crops[clamp(rot_yr,1,2) − 1]`; selain itu `option` aksi. Lalu jalankan:
     - `Irrigate` (bila `num_actions ≤ max_yr`): biasa, `irr_applied = amt·eff·(1−surq)`, `irr_runoff = amt·surq`; *ponding*, `irr_hmax = amt`, `irr_hmin = const2`, dan jumlah yang diberikan hanya selisih ke `amt` (persamaan di bawah). `irr_yr += irr_applied`, `days_irr = 1`, penghitung +1.
     - `Fertilize` (bila ≤ `max_yr`): bila sawah berisi air (`water.flo > 0`) dan `surf_frac = 1`, `pl_fert_wet`; selain itu `pl_fert` dengan `rtof` model karbon (bawaan 0,5).
     - `Till` (bila ≤ `max_yr`): `tillmix_cswat1` bila `cswat = 2` dan parameter karbon ada, selain itu `tillmix_cswat0(0, effmix, deptil)`.
     - `WeirHeight`: `set_weir(h_mm, area_ha)` pada sawah (mengatur volume utama `area·h·10⁴` m³ dan menaikkan volume darurat menjadi sekurang-kurangnya 1,2 kali volume utama); tanpa batas tahunan dan tanpa penghitung.
     - `Puddle` (bila ≤ `max_yr`): bila `wet_hc > 0` ganti konduktivitas lapisan atas sawah; sedimen air `sed·flo/10⁶`; bila genangan > 0,001 m, `tillmix_wet`, selain itu pengolahan biasa seperti `Till`.
     - `Plant`: `days_plant = 1`; setiap tanaman bernama sama yang tidak tumbuh mulai tumbuh (`gro = true`, `idorm = false`); penghitung +1 tanpa batas.
     - `HarvKill` (bila ≤ `max_yr`): `days_harv = 1`; untuk setiap tanaman bernama sama (atau `all`), tumbuh atau tidak: bila `tot.m > bm_min` panen (`harvest`, `kill = true`), lalu `killop`, `yield_tot += pl_yield`, `harv_num += 1`, catat `(i, pl_yield.m)` di `harvests`.
     - `Harvest` (bila ≤ `max_yr`): seperti di atas tanpa `killop` dan dengan `kill = false`; `yield_tot += pl_yield` tetapi `harv_num` tidak bertambah, dan dicatat `(i, 0)`.
     - `Kill` (bila ≤ `max_yr`): `days_kill = 1`; `killop` untuk tanaman bernama sama; untuk *setiap* tanaman `phuacc = 0` dan `phubase` HRU = 0.
     - `RotReset`: `rot_yr = yr`.
  5. Pada hari terakhir tahun (`end_yr`), semua `num_actions` tabel kembali ke 1.
  6. Setelah semua tabel, bila akhir tahun dan HRU punya tabel keputusan, `rot_yr += 1`.

Batas `soil_water` dan irigasi genangan:

```latex
S_{targ} = \begin{cases} v \cdot L & \text{op} = * \\ v + L & \text{op} = + \\ v - L & \text{op} = - \\ v / L & \text{op} = / \end{cases}, \quad v \in \{0,\ \Sigma FC,\ \Sigma UL\}
```

```latex
I_{app} = \max(A - 1000\,h_{wet},\ 0)\cdot e\,(1 - f_{surq}), \qquad I_{roff} = \max(A - 1000\,h_{wet},\ 0)\cdot f_{surq}
```

dengan *A* jumlah aksi (mm), *h\_wet* kedalaman genangan (m), *e* efisiensi dan *f\_surq* fraksi limpasan dari `irr.ops`.

- **Kebiasaan SWAT+ / catatan port:**
  - Tabel dijalankan sebelum cekaman harian di-reset: cekaman air, LAI dan heat unit yang diuji adalah milik kemarin.
  - `irr_yr` tidak pernah di-reset di SWAT+ (reset-nya dikomentari), sehingga “irigasi tahun ini < 500 mm” berlaku sampai 500 mm tercapai selama seluruh simulasi (Ames\_sub1 mencapainya pada tahun kedua). Dengan `--fix-swat`, `end_year` di `hru/mod.rs` mengembalikannya ke 0 setiap tahun.
  - Kondisi `phu_plant` tidak memakai variabel batas `phu_mat`; heat unit adalah milik tanaman pertama yang memilikinya.
  - `prob_unif` memakai `rndseed_cond` (disetel `gcycl` ke benih teracak bila `igen ≠ 0`, selain itu 748932582); hari yang diundi adalah satu lewat dari `int(r·days)` dihitung dari hari pertama. Kode menyimpan `days_tot` sebagai bilangan bulat lalu mengalikan dalam `f32`, sama dengan Fortran.
  - Tanam oleh tabel tidak me-reset tanaman (tidak ada `mgt_plantop`) dan tidak membatasi kedalaman akar tanah; `plant` tidak punya batas tahunan.
  - `harvest_kill` tabel memanen dan mematikan tanaman bernama sama, tumbuh ataupun tidak, dan berbeda dengan `hvkl` terjadwal tidak me-reset heat unit dasar-0 HRU. Hasil (*yield*) dijumlahkan meskipun panen tidak terjadi (di bawah `bm_min`), yaitu `pl_yield` terakhir HRU.
  - `harvest` tabel tidak menambah jumlah panen tanaman (di SWAT+ jumlah tahunannya bertambah dua), sehingga rata-rata tahunan `crop_yld_aa` menjadi total; hasil cekungan menerima luasnya dengan hasil 0 (variabel `yield` tidak pernah diisi).
  - `kill` tabel me-reset `phuacc` setiap tanaman dan `phubase` HRU, bukan hanya tanaman yang dimatikan.
  - Untuk *ponding*, `const2` berperan ganda: batas jumlah aksi per tahun (`Int(const2)`) dan kedalaman ambang sawah (`irr_hmin`, mm).
  - Penghitung hari sejak tanam/panen/mati/irigasi dimulai −366 dan bertambah satu *setelah* tabel dievaluasi setiap hari (di `Hru::day`).
  - Tabel generik (`pl_hv_*`, ditulis SWAT+ Editor untuk tanaman Batanghari) memakai tanaman dari `management.sch` setelah nama tabel; `pl_hv_summer2` memilih tanaman sesuai tahun rotasi.

### 10.4  Jadwal operasi bertanggal dan berbasis heat unit

Jadwal adalah daftar melingkar: penunjuk `cur_op` maju satu setiap kali operasi dijalankan dan kembali ke awal setelah operasi terakhir. Operasi `skip` menandai akhir tahun rotasi: begitu dicapai, sisa tahun dilewati (`yr_skip`) dan penunjuk tetap pada `skip` sampai pergantian tahun.

#### 10.4.1  `Hru::mgt_operatn`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `pub(super) fn mgt_operatn(&mut self, mo: i32, day_mo: i32, b: &HruBasin)`
- **Padanan SWAT+:** `mgt_operatn.f90`
- **Tujuan:** Menjalankan semua operasi yang jatuh pada tanggal hari ini secara berurutan, lalu operasi berbasis heat unit yang ambangnya sudah dilewati.
- **Masukan dan keluaran:** bulan `mo`, tanggal dalam bulan `day_mo`, pengaturan cekungan `b`; mengubah status HRU melalui `mgt_sched`.
- **Cara kerja:**
  1. Bila tidak ada jadwal atau jadwal kosong, selesai.
  2. Loop tanggal: selama operasi saat ini memiliki `mon = mo` dan `day = day_mo`, panggil `mgt_sched`. Berhenti bila jadwal hanya berisi satu operasi, bila `yr_skip` menjadi benar, atau setelah `n` iterasi (pengaman agar jadwal yang semua operasinya bertanggal sama tidak berputar tanpa akhir).
  3. Hitung heat unit acuan `a`: `phuacc` tanaman pertama komunitas bila sedang tumbuh, selain itu `phubase` HRU (fraksi heat unit dasar-0 tahunan).
  4. Loop heat unit: selama operasi saat ini punya `husc > 0` dan `a > husc`, panggil `mgt_sched`; berhenti bila jadwal satu operasi; hitung ulang `a`; bila operasi berikutnya adalah `skip`, jalankan segera (`mgt_sched` lagi); berhenti bila `yr_skip`.

```latex
a = \begin{cases} \mathrm{phuacc}_1 & \text{tanaman 1 tumbuh} \\ \mathrm{phubase} & \text{lainnya} \end{cases}, \qquad \text{jalankan bila } h_{sc} > 0 \wedge a > h_{sc}
```

- **Kebiasaan SWAT+ / catatan port:**
  - Dipanggil dari `Hru::day` setelah `et_act` dan sebelum limpasan, hanya bila `yr_skip` salah.
  - Operasi `skip` (bulan 0) tidak pernah cocok dengan tanggal: jadwal menunggu di sana sampai `time_control` melangkahinya pada akhir tahun.
  - *Heat unit selalu milik tanaman pertama komunitas*: `mgt_sched` Fortran mencari tanaman yang disebut operasi (`op2`) hanya pada salinan operasinya, yang diganti oleh operasi berikutnya, sehingga uji ini selalu melihat `op2 = 0` (`docs/engine1.md`).
  - *`skip` setelah operasi heat unit dijalankan seketika*, melompati sisa tahun.
  - Varian uji Ps\_hu (bajak `fallplow` pada 0,10 heat unit dasar-0, tanam 0,15, urea 100 kg/ha pada 0,30 PHU tanaman, panen-mati 1,20, lalu `skip`) cocok bit-for-bit dengan SWAT+.

#### 10.4.2  `Hru::mgt_sched`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn mgt_sched(&mut self, b: &HruBasin)`
- **Padanan SWAT+:** `mgt_sched.f90`
- **Tujuan:** Menjalankan operasi saat ini lalu memajukan penunjuk jadwal.
- **Masukan dan keluaran:** operasi `m.ops[cur_op]` (disalin agar HRU dapat diubah), pengaturan cekungan; mengubah tanaman, lapisan tanah, irigasi, pupuk, `harvests`, penghitung hari dan `cur_op`.
- **Cara kerja:** menurut jenis operasi:
  1. `Plant`: `days_plant = 1`; setiap tanaman bernama `op_char` yang tidak tumbuh mulai tumbuh (`gro = true`, `idorm = false`) dan di-*reset* oleh `plantop`, yang sekaligus membatasi kedalaman akar tanah `zmx`.
  2. `HarvKill`: untuk setiap tanaman yang tumbuh: bila namanya cocok (atau `all`), `days_harv = days_kill = 1`, panen bila biomassa total > `bm_min` (`harvest`, `kill = true`), `killop`, `yield_tot += pl_yield`, `harv_num += 1`, catat `(i, pl_yield.m)`; lalu, *untuk setiap tanaman tumbuh* (cocok atau tidak), cekaman di-reset, `phuacc = 0` dan `phubase` HRU = 0. Tanaman yang tidak tumbuh hanya menjalani panen residu bila tipe panennya `residue`, apa pun namanya.
  3. `Harv`: tanaman tumbuh yang cocok: `days_harv = 1`, panen bila biomassa > `bm_min` (`kill = false`), `yield_tot += pl_yield`, `harv_num += 1`, catat hasil. Tanaman tidak tumbuh yang cocok: panen residu bila tipenya `residue`.
  4. `Kill`: tanaman tumbuh yang cocok: `days_kill = 1`, `killop`; untuk setiap tanaman `phuacc = 0`.
  5. `Till`: `tillmix_cswat1` bila `cswat = 2` dengan parameter karbon, selain itu `tillmix_cswat0(0, effmix, deptil)`.
  6. `Irr`: `days_irr = 1`; `irr_applied = amt·eff·(1−surq)`, `irr_runoff = amt·surq` (mm); `irr_yr += irr_applied`.
  7. `Fert`: `pl_fert` dengan `rtof` (bawaan 0,5). `Manure`: `pl_manure`.
  8. `Skip`: `yr_skip = true`.
  9. Kecuali untuk `Skip`, `cur_op += 1`; bila melewati akhir daftar, kembali ke 0.
- **Kebiasaan SWAT+ / catatan port:**
  - Panen-mati terjadwal me-reset cekaman, heat unit, dan heat unit dasar HRU untuk *setiap tanaman yang tumbuh*, mana pun yang disebut operasi.
  - Setelah `killop`, total komunitas (`tot_com`, `ab_gr_com`) kehilangan tanaman itu, tetapi untuk sisa hari (limpasan dan erosi) masih menghitung biji yang dipanen, karena biji sudah dikurangkan dari tanaman sebelum `killop` mengurangkan sisanya.
  - Hasil dihitung walaupun tidak ada panen: di bawah `bm_min`, atau setelah panen residu yang tidak mengisi `pl_yield`, tanaman menambahkan `pl_yield` panen terakhir HRU.
  - `harv` terjadwal hanya mencapai tanaman yang tumbuh; aksi `harvest` tabel keputusan mencapai tanaman bernama sama apa pun statusnya.
  - Tanaman diambil keluar dari vektor komunitas (`std::mem::take`) selama diproses, agar fungsi panen dapat mengubah tanaman dan HRU sekaligus; indeks `i` dibawa sebagai `me`.
  - Irigasi terjadwal tidak memiliki batas tahunan.

#### 10.4.3  `Hru::mgt_new_year`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `pub(super) fn mgt_new_year(&mut self)`
- **Padanan SWAT+:** bagian reset jadwal di `time_control.f90`
- **Tujuan:** Melanjutkan jadwal pada tahun baru: membatalkan `yr_skip` dan melangkahi `skip` tempat jadwal berhenti.
- **Masukan dan keluaran:** `Mgt` HRU; mengubah `yr_skip` dan `cur_op`.
- **Cara kerja:**
  1. `yr_skip = false`; bila jadwal kosong, selesai.
  2. Bila operasi saat ini `skip`, `cur_op += 1`; bila melewati akhir, kembali ke 0.
- **Kebiasaan SWAT+ / catatan port:** Dipanggil dari `Hru::end_year` untuk HRU belahan utara (lintang ≥ 0) dan dari `Hru::midyear` (hari ke-181) untuk belahan selatan, bersamaan dengan reset `phubase = 0`.

#### 10.4.4  `plantop`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn plantop(pl: &mut Plant, zmx: f32) -> f32`
- **Padanan SWAT+:** `mgt_plantop.f90`
- **Tujuan:** Memulai tanaman dari nol pada operasi tanam terjadwal dan membatasi kedalaman perakaran tanah.
- **Masukan dan keluaran:** tanaman `pl`, kedalaman perakaran maksimum tanah `zmx` (mm); keluaran `zmx` baru (mm).
- **Cara kerja:**
  1. Nolkan `phuacc`; massa `tot`, `ab_gr`, `leaf`, `stem`, `seed`, `root`; `plet`, `plpet`, `laimxfr`, `hi_adj`, `olai`, `dphu`; reset cekaman.
  2. Kembalikan `min(zmx, 1000·rdmx)`, dengan `rdmx` kedalaman akar maksimum tanaman (m) dari `plants.plt`.

```latex
z_{mx}^{baru} = \min(z_{mx},\ 1000\, r_{dmx})
```

- **Kebiasaan SWAT+ / catatan port:** Batas ini permanen: `zmx` tanah tidak pernah kembali naik, sehingga tanaman berakar dangkal yang ditanam sekali membatasi semua tanaman berikutnya. Tanam lewat tabel keputusan tidak memanggil fungsi ini.

### 10.5  Panen dan mati

Massa tanaman dicatat sebagai `OrgMass` (massa `m`, karbon `c`, nitrogen `n`, fosfor `p`, semuanya kg/ha) per organ: biji (`seed`), daun (`leaf`), batang (`stem`), akar (`root`), di atas tanah (`ab_gr`) dan total (`tot`). Residu permukaan per tanaman `rsd`, residu permukaan total HRU `rsd_tot`, dan residu per lapisan tanah `soil_rsd`. Hasil panen hari ini disimpan di `Hru::pl_yield` (kg/ha). `OrgMass::scale(f, x)` mengalikan keempat komponen dengan `f`.

Pemilihan rutin panen menurut tipe `harv.ops` dan konteks:

| Tipe | Panen saja (`harv`, `harvest`) | Panen dan mati (`hvkl`, `harvest_kill`) |
| --- | --- | --- |
| `biomass` | `harvbiomass` | `harvbiomass` |
| `grain`, `picker` | `harvgrain` | `harvgrain` |
| `residue` | `harvresidue` | `harvresidue` |
| `tree` | `harvbiomass` | tidak ada |
| `tuber`, `peanuts` | `harvtuber` | `harvtuber` |
| `stripper` | `harvbiomass` | `harvgrain` |
| lain/tidak dikenal | tidak ada | tidak ada |

#### 10.5.1  `Hru::harvest`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `pub(super) fn harvest(&mut self, pl: &mut Plant, me: usize, op: &HarvOp, eff: f32, kill: bool)`
- **Padanan SWAT+:** blok `select case (harvop_db(iharvop)%typ)` di `mgt_sched.f90` dan `actions.f90`
- **Tujuan:** Memilih rutin panen sesuai tipe dan apakah operasi juga mematikan tanaman (tabel di atas).
- **Masukan dan keluaran:** tanaman `pl` (dikeluarkan dari komunitas) dengan indeks `me`, baris panen `op`, efisiensi pengganti `eff` (hanya untuk residu, dipakai bila > 0), bendera `kill`.
- **Cara kerja:** satu `match` atas `op.typ` dan `kill`; `harvgrain` dan `harvtuber` menerima `op.eff`, `harvresidue` menerima `eff` operasi.
- **Kebiasaan SWAT+ / catatan port:** Perbedaan `tree` dan `stripper` antara panen saja dan panen-mati mengikuti dua blok `select case` yang berbeda di SWAT+ (`docs/engine1.md`).

#### 10.5.2  `Hru::harvbiomass`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn harvbiomass(&mut self, pl: &mut Plant, me: usize, op: &HarvOp)`
- **Padanan SWAT+:** `mgt_harvbiomass.f90`
- **Tujuan:** Memanen fraksi `hi_ovr·eff` dari biji, daun dan batang (misalnya hijauan, tandan buah segar sawit di Ps\_harv).
- **Masukan dan keluaran:** massa organ tanaman, `hi_ovr`, `eff`, `bm_min` (kg/ha); keluaran `pl_yield`, organ berkurang, residu bertambah, LAI dan heat unit berkurang.
- **Cara kerja:**
  1. `hi_tot = hi_ovr · eff`; panen per organ `hi_tot·seed`, `hi_tot·leaf`, `hi_tot·stem`; `pl_yield` = jumlah ketiganya.
  2. Hanya bila `ab_gr.m − pl_yield.m > bm_min`: kurangi organ, `tot` dan `ab_gr` dengan panen; tambahkan `(1 − eff)·pl_yield` ke residu tanaman; hitung ulang `rsd_tot` sebagai jumlah residu semua tanaman.
  3. Bila `tot.m > 0,001`: `lai` dan `phuacc` dikali `(1 − 0,5·hi_tot)`, `root_frac = root.m / tot.m`; selain itu `tot.m`, `lai`, `phuacc` = 0.

```latex
Y = h_{ovr}\,e\,(S + L + T), \quad R \leftarrow R + (1-e)\,Y, \quad LAI \leftarrow LAI\,(1 - 0.5\,h_{ovr} e)
```

- **Kebiasaan SWAT+ / catatan port:** (a) Hasil dihitung sebelum uji `bm_min`; bila uji gagal, tanaman tidak berubah tetapi `pl_yield` tetap terisi dan ikut dijumlahkan ke hasil tanaman. (b) Tanaman kehilangan seluruh `Y`, padahal bagian `(1 − e)Y` dikembalikan ke residu: `pl_yield` yang dilaporkan mencakup bagian yang tidak terangkut. (c) Faktor `(1 − pest_stress)` SWAT+ menjadi `scale(1.0, …)` karena cekaman pestisida tidak di-*port* (selalu 0); pengalian tetap ditulis agar urutan pembulatan sama. (d) Bagian pestisida dan keluaran karbon (`hrc_d`, `hpc_d`) tidak ada di port.

#### 10.5.3  `Hru::harvtuber`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn harvtuber(&mut self, pl: &mut Plant, eff: f32)`
- **Padanan SWAT+:** `mgt_harvtuber.f90`
- **Tujuan:** Memanen umbi (atau kacang tanah) sebagai fraksi `eff` dari akar.
- **Masukan dan keluaran:** massa akar dan bagian atas tanaman, efisiensi `eff`; keluaran `pl_yield` (kg/ha), residu lapisan dan permukaan.
- **Cara kerja:**
  1. `pl_yield = eff·root`; `root -= pl_yield`.
  2. Hitung distribusi akar per lapisan `rtfr` (`Plant::rootfr` dengan kedalaman dan ketebalan lapisan).
  3. Sisa akar ke residu lapisan: `soil_rsd[ly] += rtfr[ly]·root`.
  4. Bagian atas tanah ke residu permukaan: `rsd_tot += ab_gr`, `rsd += ab_gr`.
  5. Nolkan `tot`, `ab_gr`, `leaf`, `stem`, `seed`, `root`.
- **Kebiasaan SWAT+ / catatan port:** Tanaman tidak dimatikan (`gro` tidak diubah): ia terus tumbuh dari nol. `rsd_tot` ditambah langsung, tidak dihitung ulang dari semua tanaman.

#### 10.5.4  `Hru::harvresidue`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn harvresidue(&mut self, pl: &mut Plant, me: usize, op: &HarvOp, harveff: f32)`
- **Padanan SWAT+:** `mgt_harvresidue.f90`
- **Tujuan:** Mengangkut sebagian residu permukaan (misalnya membal jerami `stover_med` di Ps\_harv).
- **Masukan dan keluaran:** residu permukaan setiap tanaman komunitas, `hi_ovr`, `eff`, `bm_min`; mengubah `rsd` semua tanaman dan `rsd_tot`. Tidak mengisi `pl_yield`.
- **Cara kerja:**
  1. `eff` = `harveff` bila ≥ 0,00001, selain itu `eff` baris `harv.ops`; `net_eff = eff·hi_ovr`.
  2. Untuk setiap tanaman `j` (memakai `pl` bila `j = me`): `removed = net_eff·rsd`; bila `rsd.m − removed.m < bm_min`, `removed = ((rsd.m − bm_min)/rsd.m)·rsd`.
  3. `rsd -= removed`; `rsd_tot -= removed`; bila `rsd_tot.m < 10⁻⁶`, `rsd_tot` dinolkan.

```latex
\Delta R_j = \begin{cases} e\,h_{ovr}\,R_j & R_j - e\,h_{ovr}R_j \ge B_{min} \\ \dfrac{R_j - B_{min}}{R_j}\,R_j & \text{lainnya} \end{cases}
```

- **Kebiasaan SWAT+ / catatan port:** (a) Bekerja atas residu *semua* tanaman, bukan hanya tanaman yang disebut. (b) Residu yang sudah di bawah `bm_min` membuat `removed` negatif: residu *bertambah* sampai `bm_min`; residu nol memberi NaN (0/0). Keduanya dipertahankan. (c) Karena `pl_yield` tidak diisi, panen residu mencatat hasil panen terakhir HRU sebelumnya ke tanaman yang dipanen. (d) Pada jadwal, tanaman yang tidak tumbuh tetap menjalani panen residu (10.4.2).

#### 10.5.5  `Hru::harvgrain`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn harvgrain(&mut self, pl: &mut Plant, eff: f32)`
- **Padanan SWAT+:** `mgt_harvgrain.f90`
- **Tujuan:** Memanen biji; bagian yang tidak terangkut masuk humus aktif lapisan atas.
- **Masukan dan keluaran:** `seed`, `eff`; keluaran `pl_yield`, `tot`/`ab_gr` berkurang, `hact` lapisan 1 bertambah, `harv_idx`.
- **Cara kerja:**
  1. `harv_idx = max(harv_idx, wsyf)` (indeks panen minimum di bawah cekaman dari `plants.plt`).
  2. `tot -= seed`, `ab_gr -= seed`.
  3. `pl_yield = eff·seed`; `hact[1] += (1 − eff)·seed`.
  4. `seed` dinolkan.
- **Kebiasaan SWAT+ / catatan port:** Biji yang tertinggal masuk ke humus aktif (`hact`) lapisan atas, pool yang tidak pernah dibaca model karbon. Urutan `harv_idx` dipindah ke awal di port tanpa memengaruhi hasil (tidak ada ketergantungan).

#### 10.5.6  `Hru::killop`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn killop(&mut self, pl: &mut Plant, me: usize)`
- **Padanan SWAT+:** `mgt_killop.f90` beserta `plg_zero` (di dalam modul tanaman)
- **Tujuan:** Mematikan tanaman: bagian atas tanah menjadi residu permukaan, akar menjadi residu lapisan, dan status pertumbuhan di-reset.
- **Masukan dan keluaran:** tanaman `pl` (indeks `me`); mengubah `rsd`, `rsd_tot`, `soil_rsd`, `tot_com`, `ab_gr_com` dan seluruh variabel pertumbuhan.
- **Cara kerja:**
  1. Hitung `rtfr` (`rootfr`).
  2. `rsd += ab_gr`; hitung ulang `rsd_tot` dari semua tanaman.
  3. `soil_rsd[ly] += rtfr[ly]·root`.
  4. `tot_com -= tot`, `ab_gr_com -= ab_gr`.
  5. Nolkan massa organ; `plg_zero`: `cht`, `lai`, `plet`, `plpet`, `laimxfr`, `hi_adj`, `hi_prev`, `olai`, `dphu`, `d_senes`, `root_dep`, `root_frac`, `rtfr`, `n_fr`, `p_fr`.
  6. Reset cekaman; `gro = false`, `idorm = false`, `phuacc = 0`, `curyr_mat = 1`.
- **Kebiasaan SWAT+ / catatan port:** Total komunitas dikurangi hanya untuk sisa hari; nilai yang dikurangkan adalah massa setelah panen, sehingga biji yang dipanen pada hari itu masih ada di `tot_com` untuk perhitungan limpasan dan erosi hari itu.

### 10.6  Pemupukan dan pupuk kandang

Ketiga fungsi ini menambahkan unsur hara ke pool tanah (atau air sawah) dan mencatat total pupuk harian HRU `fertn`, `fertp` (kg/ha) yang dinolkan setiap awal hari. Pembagian ke lapisan selalu ke dua lapisan teratas: lapisan 1 menerima fraksi permukaan `surf_frac`, lapisan 2 sisanya. Notasi: *F* dosis (kg/ha), *f\_mn*, *f\_mp*, *f\_on*, *f\_op* fraksi N mineral, P mineral, N organik, P organik; *f\_nh* fraksi amonium dari N mineral; *r* = `rtof` (fraksi organik yang menjadi residu segar, dari parameter model karbon, bawaan 0,5); *s\_l* fraksi lapisan.

#### 10.6.1  `Hru::pl_fert`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn pl_fert(&mut self, f: &Fert, cswat: i32, rtof: f32)`
- **Padanan SWAT+:** `pl_fert.f90`
- **Tujuan:** Menambahkan pupuk ke dua lapisan tanah teratas: N dan P mineral ke pool mineral, N dan P organik ke residu dan humus, dan pada model karbon `cswat = 2` sebagai bahan organik bergizi karbon.
- **Masukan dan keluaran:** `Fert` (fraksi dan `kg`), `cswat`, `rtof`; mengubah `no3`, `nh4`, `lab` (P labil), `cp` (pool karbon lapisan), `hact`, `soil_rsd` tanaman 1, `fertn`, `fertp`.
- **Cara kerja:**
  1. Pupuk dianggap organik bila `cswat = 2` dan `forgn > 0` atau `forgp > 0`. Maka `org = (m = F, c = 10·f_on·F, n = f_on·F, p = f_op·F)`, rasio C:N `0,175·org.c / (f_mn + f_on + 10⁻⁵)`, dan fraksi metabolik `meta_fr = 0,85 − 0,018·C:N`, dibatasi ke \[0,01; 0,7\].
  2. Untuk lapisan 1 dan 2 dengan `s_l`:
     - `no3 += s_l·F·(1 − f_nh)·f_mn`; `nh4 += s_l·F·f_nh·f_mn`; `lab += s_l·F·f_mp`.
     - `cp.tot.n += r·s_l·F·f_on`; `cp.tot.p += r·s_l·F·f_op`.
     - Bila `cswat = 0`: residu lapisan tanaman 1 `soil_rsd.n/p += r·s_l·F·f_on/f_op`; `hact.n += (1 − r)·s_l·F·f_on`; `hact.p = hsta.p + (1 − r)·s_l·F·f_op`.
     - Bila organik: dengan `q = (1 − r)·s_l`, `cp.tot += q·org`, `cp.hs += q·org`, `cp.meta += q·meta_fr·org`, `cp.str += q·(1 − meta_fr)·org`, `cp.lig += 0,175·q·(1 − meta_fr)·org`.
  3. `fertn += F·(f_mn + f_on)`, `fertp += F·(f_mp + f_op)`.

```latex
NO_3^{(l)} \mathrel{+}= s_l F (1-f_{nh}) f_{mn}, \quad NH_4^{(l)} \mathrel{+}= s_l F f_{nh} f_{mn}, \quad P_{lab}^{(l)} \mathrel{+}= s_l F f_{mp}
```

```latex
f_{meta} = \min\!\left(0.7,\ \max\!\left(0.01,\ 0.85 - 0.018\,\frac{0.175\, C_{org}}{f_{mn} + f_{on} + 10^{-5}}\right)\right)
```

- **Kebiasaan SWAT+ / catatan port:** (a) Dengan model karbon asli (`cswat = 0`), P organik *menetapkan* P humus aktif dua lapisan teratas menjadi P humus stabil ditambah P pupuk (bukan menambah), sesuai `docs/swat-quirks.md`. (b) Residu organik dimasukkan ke residu lapisan tanaman 1; SWAT+ akan gagal tanpa tanaman, port melewatinya. (c) `cp.tot.n/p` bertambah oleh bagian `rtof` apa pun model karbonnya. (d) Tanpa `chem_app.ops` yang cocok, `surf_frac = 0` dan seluruh pupuk masuk lapisan 2.

#### 10.6.2  `Hru::pl_fert_wet`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn pl_fert_wet(&mut self, f: &Fert, cswat: i32, fixes: bool)`
- **Padanan SWAT+:** `pl_fert_wet.f90`
- **Tujuan:** Menaburkan pupuk ke air genangan sawah (dipanggil oleh aksi `fertilize` bila sawah berisi air dan `surf_frac = 1`).
- **Masukan dan keluaran:** `Fert`, `cswat`, bendera `--fix-swat`; mengubah air sawah `w.water` (`no3`, `nh3`, `solp`, `orgn`, `sedp`, kg) dan `fertn`, `fertp`.
- **Cara kerja:**
  1. Bila `cswat = 2` atau `fixes`, dan HRU punya sawah: `no3 += F(1 − f_nh)f_mn`, `nh3 += F·f_nh·f_mn`, `solp += F·f_mp`, `orgn += F·f_on`, `sedp += F·f_op`.
  2. Dalam semua kasus `fertn`, `fertp` bertambah.
- **Kebiasaan SWAT+ / catatan port:** Dengan model karbon asli, SWAT+ hanya menambahkan pupuk ke air pada `cswat = 2`, sehingga pupuk yang ditabur pada sawah tergenang hilang (tercatat diberikan, tetapi tidak masuk pool mana pun). `--fix-swat` memasukkannya ke air pada model mana pun (`docs/swat-fixes.md`); pada Ps\_paddy ini menaikkan hasil gabah dari 4,11–4,60 menjadi 4,12–4,73 t/ha per musim (`docs/paddy.md`).

#### 10.6.3  `Hru::pl_manure`

- **Lokasi:** `crates/swat-engine/src/hru/mgt.rs`, `fn pl_manure(&mut self, mn: &Manure, rtof: f32)`
- **Padanan SWAT+:** `pl_manure.f90` (cabang model karbon asli)
- **Tujuan:** Memberikan pupuk kandang: airnya ke lengas tanah, N dan P mineral ke pool mineral, N dan P organik ke total dan humus.
- **Masukan dan keluaran:** `Manure` (`kg` kering, `frac_water`, fraksi hara, `surf_frac`), `rtof`; mengubah `st` (mm), `no3`, `nh4`, `lab`, `cp.tot`, `hact.n`, `hsta.p`, `fertn`, `fertp`.
- **Cara kerja:**
  1. Massa air pupuk: `liq_kg = F/(1 − frac_water) − F`; dalam mm `liq_mm = 0,0001·liq_kg`.
  2. Untuk lapisan 1 dan 2 dengan `s_l` dan `M = s_l·F`: `st += s_l·liq_mm`; `no3 += M(1 − f_nh)f_mn`; `cp.tot.n += r·M·f_on`; `hact.n += (1 − r)·M·f_on`; `cp.tot.p += r·M·f_op`; `hsta.p += (1 − r)·M·f_op`; `nh4 += M·f_nh·f_mn`; `lab += M·f_mp`.
  3. `fertn`, `fertp` bertambah.

```latex
W_{mm} = 10^{-4}\left(\frac{F}{1 - f_w} - F\right)
```

- **Kebiasaan SWAT+ / catatan port:** (a) Air pupuk kandang masuk ke lengas tanah. (b) Berbeda dengan `pl_fert`, P organik bagian humus masuk ke humus *stabil* (`hsta.p`) dan ditambahkan. (c) Pupuk kandang dengan `cswat = 2` ditolak saat membangun jadwal (belum di-*port*). (d) Baris `manure_om.frt` adalah nomor baris `manure_db.frt` yang disebut (10.2.2). Divalidasi pada Osu\_1hru.

### 10.7  Sawah: manajemen genangan

Sawah (*paddy*) adalah lahan basah pada HRU (`surf_stor`) yang hidrologinya (`hydrology.wet`) bernama `paddy`; hidrologinya dibahas di bab lahan basah. Dari sisi manajemen, sawah dikendalikan sepenuhnya oleh tabel keputusan di bab ini: aksi `weir_height` (`set_weir` di `hru/wetland.rs` menetapkan tinggi pelimpah `h/1000` m dan volume utama `area·h·10⁴` m³, lalu menaikkan volume darurat menjadi sekurang-kurangnya 1,2 kali volume utama), irigasi `ponding` (mengisi kembali sampai `const` mm, dengan `const2` sebagai kedalaman ambang dan batas jumlah per tahun), `puddle` (pelumpuran: konduktivitas lapisan atas dan sedimen air dari `puddle.ops` serta pengolahan di bawah air `tillmix_wet`), `fertilize` ke air (`pl_fert_wet`), dan kondisi `weirh`, `wet_depth`, `prob_unif` (hari acak dalam jendela tanam).

Skenario Osu\_1hru (contoh sawah SWAT+ sendiri, Korea 2010–2023: pelimpah dinaikkan pada hari acak `prob_unif`, irigasi genangan, pelumpuran, padi ditanam dan dipanen oleh tabel keputusan, pupuk kandang `manu`) dan Ps\_paddy (Ps\_coast dengan HRU padinya sebagai sawah, dua musim per tahun 2015–2017: pelimpah 50 mm, irigasi kembali ke 50 mm bila genangan di bawah 30 mm paling banyak 30 kali setahun, dikeringkan pada 85 % heat unit dan dipanen pada 100 %, urea dalam tiga kali pemberian dengan dua pemupukan susulan ke air) cocok bit-for-bit untuk semua tabel keluaran (`docs/paddy.md`). Dua koreksi `--fix-swat` yang relevan: `pl_fert_wet` (10.6.2) dan `mgt_newtillmix_wet` (nutrien genangan yang hilang saat pelumpuran; dibahas bersama fungsi pengolahan tanah).

### 10.8  Rekapitulasi fungsi

| No. | Fungsi | Padanan SWAT+ | Subbab |
| --- | --- | --- | --- |
| 1 | `HarvOp::new` | pencarian `harvop_db` | 10.1.1 |
| 2 | `row_real` | khusus port | 10.1.2 |
| 3 | `jdt` | `jdt.f90` | 10.1.3 |
| 4 | `Dtbl::new` | `dtbl_lum_read.f90` | 10.2.1 |
| 5 | `Mgt::new` | `read_mgtops.f90`, `proc_cond.f90`, `plant_init.f90` | 10.2.2 |
| 6 | `cond_test` | `cond_real.f90`, `cond_integer.f90` | 10.3.1 |
| 7 | `Hru::dtbl_day` | `conditions.f90`, `actions.f90` | 10.3.2 |
| 8 | `Hru::mgt_operatn` | `mgt_operatn.f90` | 10.4.1 |
| 9 | `Hru::mgt_sched` | `mgt_sched.f90` | 10.4.2 |
| 10 | `Hru::mgt_new_year` | `time_control.f90` | 10.4.3 |
| 11 | `plantop` | `mgt_plantop.f90` | 10.4.4 |
| 12 | `Hru::harvest` | `select case` di `mgt_sched.f90`/`actions.f90` | 10.5.1 |
| 13 | `Hru::harvbiomass` | `mgt_harvbiomass.f90` | 10.5.2 |
| 14 | `Hru::harvtuber` | `mgt_harvtuber.f90` | 10.5.3 |
| 15 | `Hru::harvresidue` | `mgt_harvresidue.f90` | 10.5.4 |
| 16 | `Hru::harvgrain` | `mgt_harvgrain.f90` | 10.5.5 |
| 17 | `Hru::killop` | `mgt_killop.f90` | 10.5.6 |
| 18 | `Hru::pl_fert` | `pl_fert.f90` | 10.6.1 |
| 19 | `Hru::pl_fert_wet` | `pl_fert_wet.f90` | 10.6.2 |
| 20 | `Hru::pl_manure` | `pl_manure.f90` | 10.6.3 |
