## Bab 14  Akuifer dan kalibrasi keras

Bab ini membahas dua bagian engine yang sama-sama bekerja di luar HRU tetapi berdekatan dalam urutan inisialisasi SWAT+: (1) akuifer dangkal satu dimensi (*1-D aquifer*) yang menerima resapan (*recharge*) dari HRU dan unit routing, lalu melepaskan aliran balik (*return flow*, baseflow), rembesan dalam (*deep seepage*), *revap*, serta nitrat dan P mineral; dan (2) kalibrasi keras (*hard calibration*) melalui berkas `cal_parms.cal` dan `calibration.cal`, yaitu perubahan parameter sekali jalan yang diterapkan setelah objek disiapkan (`cal_conditions` / `cal_parm_select` di SWAT+).

File Rust yang dicakup: bagian akuifer dari `crates/swat-engine/src/route.rs` (`AquDay`, `Aqu`, blok `aqu_initial` dan kalibrasi akuifer di dalam `Network::new`, `aqu_1d_control`, `aquifers`), akumulator periode akuifer di `crates/swat-engine/src/periods.rs` (`AquAcc`, `aqu_add`, `aqu_div`, `aqu_scaled`, `aqu_sum`), tipe `AquiferDb` di `crates/swat-model/src/routing.rs` dan pembacanya `read_aquifers` di `crates/swat-io/src/routing.rs`; untuk kalibrasi: `crates/swat-model/src/cal.rs`, `crates/swat-io/src/cal.rs`, dan fungsi `chg_par`, `cal_parm`, `cal_cond_met`, `cal_elems`, `cal_conditions`, `cal_hru`, `cal_soil`, `soil_awc_init` di `crates/swat-engine/src/hru/mod.rs`. Fungsi `route.rs` lain (perintah, unit routing, hidrograf sub-harian, kanal) dibahas di Bab 12; di sini hanya dirujuk bila perlu.

Posisi dalam alur simulasi: di SWAT+ `main` memanggil `hyd_connect`, `proc_hru`, `proc_cha`, `proc_aqu` (berisi `aqu_read`, `aqu_initial`, `aqu_read_init`, `aqu_read_init_cs`), `proc_cond`, `proc_res`, lalu `proc_cal` (berisi `cal_parm_read`, `cal_parmchg_read`, …, `cal_conditions`). Port Rust memecah `cal_conditions` menjadi dua lintasan: perubahan HRU/tanah/basin dijalankan di akhir `init_hrus` (sebelum `nutcarb_stage`, sejalan dengan `soil_nutcarb_init` yang datang setelah `proc_cal`), sedangkan perubahan akuifer dijalankan di `Network::new` tepat setelah blok `aqu_initial`. Karena kedua lintasan menyentuh status yang terpisah, pemisahan ini tidak mengubah hasil. Selama simulasi harian, setiap akuifer dijalankan oleh loop perintah `Network::day` saat gilirannya tiba (setelah semua objek yang mengirim air kepadanya) melalui `aqu_1d_control`.

### 14.1  Tipe data akuifer

SWAT+ menyimpan akuifer dalam tiga struktur: `aquifer_database` (`aqudb`/`aqu_dat`, satu baris `aquifer.aqu`), `aquifer_data_parameters` (`aqu_prm`: luas, konstanta turunan) dan `aquifer_dynamic` (`aqu_d`, status harian yang juga menjadi baris `aquifer_day.txt`). Port memetakan ketiganya ke `AquiferDb`, `Aqu` dan `AquDay`.

**`AquiferDb`** (`crates/swat-model/src/routing.rs`, padanan `aquifer_database`):

| Field | Satuan | Arti | Bawaan |
| --- | --- | --- | --- |
| `name`, `init` | teks | nama akuifer; nama data awal (`aqu_ini`, menunjuk `initial.aqu`) | kosong |
| `flo` | mm | aliran balik awal | 0,05 |
| `dep_bot` | m | kedalaman dari permukaan (tengah lereng) ke dasar akuifer | 0 |
| `dep_wt` | m | kedalaman muka air tanah awal | 0 |
| `no3`, `minp` | ppm (mg/L) | konsentrasi awal NO3-N dan P mineral | 0 |
| `cbn` | % | karbon organik (statis) | 0,5 |
| `flo_dist` | m | jarak alir rata-rata ke sungai (hanya akuifer 2-D) | 50 |
| `bf_max` | mm | baseflow harian maksimum saat semua kanal berkontribusi (hanya akuifer 2-D) | 0 |
| `alpha` | 1/hari | konstanta resesi aliran balik | 0 |
| `revap_co` | – | koefisien revap: revap = PET · `revap_co` | 0 |
| `seep` | fraksi | bagian resapan yang merembes ke akuifer dalam | 0 |
| `spyld` | m3/m3 | *specific yield* | 0 |
| `hlife_n` | hari | waktu paruh nitrat di air tanah | 30 |
| `flo_min` | m | kedalaman muka air tanah maksimum agar aliran balik terjadi | 0 |
| `revap_min` | m | kedalaman muka air tanah maksimum agar revap terjadi | 0 |

**`Aqu`** (privat, `route.rs`): satu akuifer dalam jaringan: `ob` (indeks objek di larik objek), `db` (salinan `AquiferDb` miliknya, padanan `aqu_dat(iaq)`, dapat diubah kalibrasi), `alpha_e` = exp(−`alpha`) dan `nloss` = exp(−0,693/(`hlife_n`+0,1)) (padanan `aqu_prm%alpha_e`, `aqu_prm%nloss`), serta `d: AquDay`. Luas akuifer tidak disalin; selalu dibaca dari objek (`obs[ob].area_ha`, padanan `aqu_prm%area_ha` = `ob%area_ha`).

**`AquDay`** (publik, `route.rs`, padanan `aquifer_dynamic`), 17 nilai `f32`:

| Field | Satuan | Arti |
| --- | --- | --- |
| `flo` | mm | aliran balik (baseflow) hari ini |
| `dep_wt` | m | kedalaman muka air tanah |
| `stor` | mm | simpanan air akuifer |
| `rchrg` | mm | resapan masuk |
| `seep` | mm | rembesan ke akuifer dalam |
| `revap` | mm | revap (naik ke zona akar/atmosfer) |
| `no3_st` | kg/ha N | nitrat tersimpan |
| `minp` | lihat catatan | P mineral; setelah hari pertama berisi beban P resapan (kg), bukan simpanan |
| `cbn` | % | karbon organik, konstan dari basis data |
| `orgn` | kg/ha | N organik; tidak pernah diisi (selalu 0) |
| `no3_rchg` | kg/ha N | nitrat yang masuk bersama resapan |
| `no3_loss` | kg/ha N | nitrat hilang karena peluruhan (waktu paruh) |
| `no3_lat` | kg/ha N | nitrat yang keluar bersama aliran balik |
| `no3_seep` | kg/ha N | nitrat yang keluar bersama rembesan |
| `flo_cha`, `flo_res`, `flo_ls` | mm | bagian aliran balik yang menuju kanal, waduk, atau objek lanskap/akuifer lain |

Akuifer memiliki dua hidrograf (`hd_tot` = 2 di tabel `KINDS`): `hd[0]` (jenis `tot`, aliran balik beserta nitrat dan P terlarut) dan `hd[1]` (jenis `rhg`, rembesan dalam beserta nitratnya). Hidrograf memakai satuan `Hyd`: air dalam m3, zat dalam kg.

#### 14.1.1  `AquDay::columns`

- **Lokasi:** `crates/swat-engine/src/route.rs`, `pub fn columns(&self) -> [(&'static str, f32); 17]`
- **Padanan SWAT+:** urutan kolom `aqu_hdr` dan pernyataan `write` di `aquifer_output.f90`.
- **Tujuan:** Memberi 17 pasangan (label kolom, nilai) untuk `aquifer_day.txt` dan berkas periodenya, persis seperti header SWAT+.
- **Masukan dan keluaran:** `self`; larik pasangan label/nilai dalam urutan berkas.
- **Cara kerja:**
  1. Memanggil `columns_as(false)`, yaitu label asli SWAT+.
- **Kebiasaan SWAT+ / catatan port:** Header SWAT+ menulis `orgn orgp` di kolom 9 dan 10, padahal nilainya adalah `cbn` dan `orgn` (header bergeser satu kolom). Mode bawaan mempertahankan label yang salah ini demi kesamaan berkas (lihat `docs/swat-quirks.md`, bagian *Routing units and aquifers*).

#### 14.1.2  `AquDay::columns_as`

- **Lokasi:** `crates/swat-engine/src/route.rs`, `pub fn columns_as(&self, fixes: bool) -> [(&'static str, f32); 17]`
- **Padanan SWAT+:** `aqu_hdr` (`aquifer_module.f90`) terhadap `aquifer_dynamic`.
- **Tujuan:** Sama dengan `columns`, tetapi dapat memakai label yang diperbaiki.
- **Masukan dan keluaran:** `fixes` (true bila `--fix-swat`); 17 pasangan berurutan `flo, dep_wt, stor, rchrg, seep, revap, no3_st, minp, <l1>, <l2>, no3_rchg, no3_loss, no3_lat, no3_seep, flo_cha, flo_res, flo_ls`.
- **Cara kerja:**
  1. Bila `fixes`, `(l1, l2) = ("cbn", "orgn")`; selain itu `("orgn", "orgp")`.
  2. Menyusun larik dengan nilai dari field yang bersesuaian; nilai tidak berubah, hanya labelnya.
- **Kebiasaan SWAT+ / catatan port:** Koreksi label tercatat di `docs/swat-fixes.md` (baris `aqu_hdr`). Perhatikan bahwa urutan nilai di berkas (`rchrg, seep, revap` sesudah `stor`) berbeda dari urutan field di `aquifer_dynamic` Fortran; port mengikuti urutan penulisan.

#### 14.1.3  `AquiferDb::default`

- **Lokasi:** `crates/swat-model/src/routing.rs`, `impl Default for AquiferDb`
- **Padanan SWAT+:** nilai inisialisasi field `aquifer_database` di `aquifer_module.f90`.
- **Tujuan:** Memberi nilai bawaan yang sama dengan Fortran untuk kolom yang kosong atau baris yang tidak ada.
- **Masukan dan keluaran:** tidak ada; `AquiferDb` dengan `flo` 0,05 mm, `cbn` 0,5 %, `flo_dist` 50 m, `hlife_n` 30 hari, lainnya 0.
- **Cara kerja:** konstruktor literal (lihat tabel di atas).
- **Kebiasaan SWAT+ / catatan port:** `alpha` dan `spyld` bawaan 0: akuifer tanpa baris basis data memiliki `alpha_e` = 1 (aliran balik tidak pernah berubah dari nilai awal) dan `spyld` = 0, sehingga pembagian `stor/(1000·spyld)` menghasilkan tak hingga dan `dep_wt` dipotong ke 0. Port mereproduksi aritmetika ini tanpa pengaman tambahan.

#### 14.1.4  `read_aquifers`

- **Lokasi:** `crates/swat-io/src/routing.rs`, `pub fn read_aquifers(file: &str, text: &str) -> Result<Vec<AquiferDb>>`
- **Padanan SWAT+:** `aqu_read.f90` (dipanggil dari `proc_aqu`).
- **Tujuan:** Membaca `aquifer.aqu` menjadi tabel basis data akuifer berindeks nomor baris.
- **Masukan dan keluaran:** nama dan isi berkas; `Vec<AquiferDb>` dengan elemen 0 berisi bawaan dan elemen *i* berisi baris bernomor *i*.
- **Cara kerja:**
  1. Menghitung jumlah baris data (`count_rows`, melewati judul dan header).
  2. Untuk tiap baris, membaca sampai 18 nilai secara *list-directed*: nomor, nama, `init`, lalu 15 bilangan dalam urutan `flo, dep_bot, dep_wt, no3, minp, cbn, flo_dist, bf_max, alpha, revap_co, seep, spyld, hlife_n, flo_min, revap_min`.
  3. Menaruh baris di indeks sesuai nomornya, memperbesar vektor bila perlu; nilai yang hilang di akhir baris memakai bawaan.
- **Kebiasaan SWAT+ / catatan port:** Penempatan menurut nomor (bukan urutan baca) mengikuti `aqudb(i)` di Fortran. Fungsi pembacaan secara umum dibahas di bab masukan; di sini dicantumkan karena khusus akuifer.

### 14.2  Inisialisasi akuifer di `Network::new`

Konstruktor `Network::new` (dibahas utuh di Bab 12) memuat dua blok khusus akuifer yang dijelaskan di sini sebagai entri tersendiri karena masing-masing berpadanan dengan subrutin Fortran yang utuh. Sebelum blok tersebut, `Network::new` juga menolak proyek yang memiliki akuifer 2-D (`ObjectKind::Aquifer2d`, pesan `"2-D aquifers"`), sehingga semua kode `aqu2d` di `aqu_1d_control` Fortran (fraksi kontribusi kanal `aq_ch`, `bf_max`) tidak pernah aktif dalam port.

#### 14.2.1  Blok `aqu_initial` (bagian dari `Network::new`)

- **Lokasi:** `crates/swat-engine/src/route.rs`, di dalam `pub fn new(project, stations, tconc, ovn_km, fixes) -> Result<Option<Network>, String>`, ditandai komentar `// aqu_initial`.
- **Padanan SWAT+:** `aqu_initial.f90` (dipanggil dari `proc_aqu.f90` setelah `aqu_read`). `aqu_read_init` dan `aqu_read_init_cs` tidak diport: keduanya hanya membaca `initial.aqu` untuk konstituen (pestisida, garam, cs), dan loop yang seharusnya memakai datanya di `aqu_read_init` kosong.
- **Tujuan:** Membuat satu `Aqu` per objek akuifer, menyalin baris basis datanya, menghitung konstanta resesi dan peluruhan nitrat, dan mengisi status awal (`aqu_d`).
- **Masukan dan keluaran:** objek akuifer dari `project.objects_of(ObjectKind::Aquifer)` (urutan larik objek, dimulai di `first[&Aquifer]`), `props` tiap objek (nomor baris `aquifer.aqu`), `project.aquifer_db`, luas objek (ha); keluaran vektor `aqus` dan peta `aqu_of` (indeks objek ke nomor akuifer).
- **Cara kerja:** untuk setiap akuifer `iaq` (0-based) dengan objek `ob = aq0 + iaq`:
  1. `iaqdb = max(props, 0)`; `db` = `aquifer_db[iaqdb]` (bawaan bila tidak ada).
  2. Status awal: `flo = db.flo`, `dep_wt = db.dep_wt`, `cbn = db.cbn`, field lain 0 (padanan pengenolan eksplisit `rchrg`, `seep`, `revap`, `no3_*`, `flo_*` di Fortran).
  3. Simpanan awal (mm) dari tebal zona jenuh dan *specific yield*:

```latex
S_{0} = 1000\,(z_{bot} - z_{wt}^{*})\,S_{y}
```

dengan z\_bot = `dep_bot` (m), S\_y = `spyld`, dan z\_wt\* adalah kedalaman muka air yang dipakai (lihat catatan).

4. Nitrat dan P mineral awal (kg/ha menurut komentar Fortran) dari konsentrasi (ppm) dikali volume aliran awal:

```latex
NO3_{st} = \frac{(10\,q_{0}\,A)\,c_{NO3}}{1000},\qquad P_{min} = \frac{(10\,q_{0}\,A)\,c_{P}}{1000}
```

dengan q0 = `flo` (mm), A = luas (ha); 10·mm·ha = m3, dan m3·ppm/1000 = kg.

5. `alpha_e = exp(-alpha)`, `nloss = exp(-0.693 / (hlife_n + 0.1))`.
6. `aqu_of[ob] = Some(iaq)` dan `Aqu` didorong ke `aqus`.

- **Kebiasaan SWAT+ / catatan port:**
  - Fortran menghitung simpanan dengan `aqu_d(iaqdb)%dep_wt`, yaitu muka air dari akuifer yang **nomornya sama dengan nomor baris basis data**, bukan milik akuifer ini. Port mereproduksinya dalam mode bawaan: bila `iaqdb-1 < iaq` dipakai `aqus[iaqdb-1].d.dep_wt` (akuifer itu sudah diinisialisasi), bila `iaqdb-1 == iaq` dipakai `dep_wt` miliknya sendiri, selain itu 0 (akuifer itu belum diinisialisasi, nilai bawaan larik Fortran 0). Dengan satu baris per akuifer seperti ditulis SWAT+ Editor, hasilnya sama dengan muka air sendiri. Dengan `--fix-swat`, selalu dipakai `d.dep_wt` sendiri (`docs/swat-fixes.md`, baris `aqu_initial`).
  - Nitrat dan P awal dihitung dari *aliran* awal (`flo`, mm), bukan dari simpanan; ini perilaku SWAT+ apa adanya.
  - Perhitungan memakai `f32` dengan urutan operasi Fortran: `(10*flo*area) * no3 / 1000`.

#### 14.2.2  Blok kalibrasi akuifer (bagian dari `Network::new`)

- **Lokasi:** `crates/swat-engine/src/route.rs`, di dalam `Network::new`, tepat setelah blok `aqu_initial` (komentar `// cal_conditions: the aquifers' calibration`).
- **Padanan SWAT+:** cabang `!!AQU` di `cal_parm_select.f90`, dipanggil dari `cal_conditions.f90` untuk parameter yang `ob_typ`-nya `aqu`.
- **Tujuan:** Menerapkan baris `calibration.cal` yang parameternya milik akuifer, setelah akuifer diinisialisasi, dan menghitung ulang besaran turunan yang ikut dihitung ulang oleh SWAT+.
- **Masukan dan keluaran:** `project.cal_upd` (perubahan), `project.cal_parms` (jenis objek dan batas), `aqus`; keluaran `aqus[*].db`, `alpha_e`, `nloss`, `d.stor`, `d.no3_st` yang diubah; `Err` untuk hal yang tidak didukung.
- **Cara kerja:** untuk setiap perubahan `u` berurutan:
  1. `p = cal_parm(project, u)`; lewati bila `p.ob_typ != "aqu"`.
  2. Bila `u` memiliki kondisi, kembalikan `Err("calibration of … with conditions (aquifer)")`. (Di SWAT+ kondisi akan diuji pada HRU yang bernomor sama dengan akuifer; port memilih menolak.)
  3. Untuk setiap elemen `e` dari `cal_elems(u, aqus.len())` (daftar objek, atau semua akuifer): ambil `aqus[e-1]` (galat bila di luar rentang), definisikan `chg(v) = chg_par(u, &p, v)`.
  4. Terapkan menurut nama parameter:

| Nama di `calibration.cal` | Field yang diubah | Dihitung ulang |
| --- | --- | --- |
| `alpha` | `db.alpha` | `alpha_e = exp(-alpha)` |
| `revap_co` | `db.revap_co` | – |
| `deep_seep` | `db.seep` | – |
| `flo_min` | `db.flo_min` | – |
| `revap_min` | `db.revap_min` | – |
| `dep_bot` | `db.dep_bot` | – (simpanan tidak dihitung ulang) |
| `minp_init` | `db.minp` | – |
| `sp_yld` | `db.spyld` | `d.stor = 1000·(dep_bot − d.dep_wt)·spyld` |
| `hlife_n` | `db.hlife_n` | `nloss = exp(−0,693/(hlife_n+0,1))` |
| `no3_init` | `db.no3` | `d.no3_st = (10·d.flo·A)·no3/1000` |
| `flo_init_mm`, `dep_wt_init`, `cbn_init`, `flo_dist`, `bf_max` | field basis data terkait | – (tidak berefek setelah inisialisasi atau tanpa akuifer 2-D) |
| lainnya | – | `Err("calibration of <nama>")` |

- **Kebiasaan SWAT+ / catatan port:**
  - Seperti di Fortran, perubahan `dep_bot` tidak menghitung ulang simpanan; karena kedalaman muka air harian dihitung ulang dari `dep_bot` dan simpanan, efeknya adalah menggeser muka air tanpa menambah air.
  - `sp_yld` menghitung ulang simpanan dengan `dep_wt` akuifer **itu sendiri** (`aqu_d(ielem)`), bukan dengan aturan `aqu_d(iaqdb)` dari `aqu_initial`; port mengikuti persis.
  - `minp_init` hanya mengubah konsentrasi basis data; karena `aqu_1d_control` menghitung P dari konsentrasi itu setiap hari, perubahan tetap berpengaruh pada beban P harian, tetapi P awal `d.minp` tidak dihitung ulang.
  - Urutan: SWAT+ menjalankan perubahan HRU dan akuifer bergantian sesuai urutan baris `calibration.cal`; port menjalankan akuifer di lintasan terpisah. Hasilnya identik karena tidak ada status yang dipakai bersama; urutan di antara perubahan akuifer sendiri dipertahankan (penting bila dua baris mengubah parameter yang sama, misalnya `pctchg` berturut-turut).

### 14.3  Simulasi harian akuifer

#### 14.3.1  `Network::aqu_1d_control`

- **Lokasi:** `crates/swat-engine/src/route.rs`, `fn aqu_1d_control(&mut self, iaq: usize, pet: f32)`
- **Padanan SWAT+:** `aqu_1d_control.f90`.
- **Tujuan:** Satu hari neraca air dan nitrat sebuah akuifer dangkal: resapan masuk, aliran balik dengan resesi eksponensial, rembesan dalam, revap, transport nitrat, P mineral, dan pembagian aliran balik menurut tujuan.
- **Masukan dan keluaran:** `iaq` (nomor akuifer 0-based); `pet` = PET harian stasiun cuaca objek (mm), diambil `Network::day` dari `nd.weat[wst].pet`; masukan air dari `obs[ob].hin` (jumlah hidrograf yang masuk hari ini: `flo` m3, `no3` kg); luas A (ha). Keluaran: `aqus[iaq].d` (status `AquDay`), `obs[ob].hd[0]` (aliran balik: `flo` m3, `no3` kg, `solp` kg) dan `obs[ob].hd[1]` (rembesan: `flo` m3, `no3` kg). Semua field hidrograf lain bernilai 0 (hidrograf dibuat dari `Hyd::default()`, padanan `ob%hd = hz`).
- **Cara kerja:**
  1. **Resapan** (mm) dari volume masuk:

```latex
R = \frac{V_{in}}{10\,A},\qquad S \leftarrow S + R
```

2. **Kedalaman muka air tanah** (m), dibatasi tidak negatif:

```latex
z_{wt} = \max\left(0,\; z_{bot} - \frac{S}{1000\,S_{y}}\right)
```

3. **Aliran balik.** Bila `dep_wt <= flo_min` (muka air cukup dangkal), aliran mengikuti persamaan resesi dengan resapan sebagai masukan, lalu dibatasi 0 ≤ q ≤ S dan dikurangkan dari simpanan:

```latex
q_{t} = q_{t-1}\,e^{-\alpha} + R\,(1 - e^{-\alpha}),\qquad q_{t} = \min(S, \max(0, q_{t})),\qquad S \leftarrow S - q_{t}
```

Selain itu `flo = 0`. Volume hidrograf aliran balik: `hd1.flo = 10·q·A` (m3).

4. **Rembesan dalam** sebagai fraksi resapan, dibatasi simpanan:

```latex
W_{seep} = \min(R\,\beta_{deep},\; S),\qquad S \leftarrow S - W_{seep}
```

dengan β\_deep = `seep`; `hd2.flo = 10·W_seep·A`.

5. **Revap.** Bila `dep_wt < revap_min`: `revap = min(PET·revap_co, S)` dan `S -= revap`; selain itu 0. Perhatikan bahwa uji ini memakai `dep_wt` dari langkah 2 (sebelum aliran balik dan rembesan dikurangkan).
6. **Nitrat masuk:** `no3_rchg = hin.no3 / A` (kg/ha), ditambahkan ke `no3_st`.
7. **Konsentrasi nitrat** (kg/ha per mm) dihitung dari simpanan *setelah* aliran balik, rembesan dan revap: `conc = no3_st / S` bila `S > 1e-6`, selain itu 0.
8. **Nitrat aliran balik:** `hd1.no3 = min((conc·q)·A, no3_st·A)` (kg); `no3_lat = hd1.no3 / A`; `no3_st -= no3_lat`.
9. **Peluruhan nitrat** orde satu dengan waktu paruh:

```latex
N_{st} \leftarrow N_{st}\,\exp\!\left(\frac{-0.693}{t_{1/2} + 0.1}\right),\qquad N_{loss} = N_{st}^{sebelum} - N_{st}^{sesudah}
```

10. **Nitrat rembesan:** `no3_seep = min(conc·seep, no3_st)`; `no3_st -= no3_seep`; `hd2.no3 = no3_seep·A`. Konsentrasi yang dipakai tetap `conc` dari langkah 7 (sebelum peluruhan).
11. **P mineral:** `minp = hin.flo · db.minp / 1000` dan `hd1.solp = minp`.
12. **Pembagian aliran balik.** `flo_cha`, `flo_res`, `flo_ls` dinolkan, lalu untuk setiap keluaran objek dengan jenis hidrograf `tot` (`ihtyp == 1`): kode `cha` atau `sdc` menambah `flo_cha += q·frac`, `res` menambah `flo_res`, `aqu` menambah `flo_ls`; kode lain diabaikan.
13. Menyimpan `hd1`, `hd2` ke `obs[ob].hd[0..2]`.

- **Kebiasaan SWAT+ / catatan port:**
  - **Satuan P.** `minp` dihitung dari *volume resapan* (m3) kali konsentrasi basis data (ppm) / 1000, sehingga satuannya kg, dan nilainya menimpa P awal dari `aqu_initial`; P mineral akuifer tidak pernah disimpan atau dikurangi. Beban P terlarut aliran balik sama dengan P yang dibawa resapan pada konsentrasi basis data, bukan fungsi aliran balik. Fortran menghitung baris ini dua kali (sebelum dan sesudah blok garam/konstituen) dengan hasil sama; port sekali.
  - **Aliran balik ke akuifer lain dihitung sebagai `flo_ls`** (kode `aqu` di Fortran berada di cabang `flo_ls`).
  - **PET untuk revap** adalah `wst(iwst)%weat%pet`, PET Hargreaves stasiun cuaca, apa pun metode PET HRU (catatan di `docs/swat-quirks.md`).
  - **Urutan operasi** sama persis dengan Fortran (resapan → muka air → aliran balik → rembesan → revap → nitrat → peluruhan → rembesan nitrat → P), dan semuanya `f32`; misalnya `(conc_no3 * d.flo) * area` dipertahankan berkurung seperti Fortran agar hasil bulat-identik.
  - **Tidak diport** (port menolak proyek yang membutuhkannya di tempat lain, atau fiturnya tidak ada): alokasi air dari akuifer (`wallo_control` bila `wallo_pod > 0`), garam (`salt_chem_aqu`), konstituen (`cs_rctn_aqu`, `cs_sorb_aqu`), pestisida akuifer, akuifer 2-D (`aq_ch`). Variabel `rchrg_prev`/`rchrgn_prev` (hanya dipakai gwflow/alokasi) dan total `hin_tot`/`hout_tot` juga tidak disimpan.
  - **Sub-harian.** Fortran membagi `hd(1)` rata ke setiap langkah (`ob%ts(1,ii) = hd(1)/step`). Port tidak menyimpan `ts` untuk akuifer; sebagai gantinya `sum_tsin` (Bab 12) membagi rata hidrograf akuifer yang diterima objek hilir (`even(src.hd[ihtyp-1].flo)`), yang menghasilkan masukan per langkah yang sama.

#### 14.3.2  `Network::aquifers`

- **Lokasi:** `crates/swat-engine/src/route.rs`, `pub fn aquifers(&self) -> impl Iterator<Item = (usize, i32, &str, &AquDay)>`
- **Padanan SWAT+:** tidak ada sebagai fungsi; padanan akses `aqu_d(iaq)`, `ob(iob)%gis_id`, `ob(iob)%name` di `aquifer_output.f90`.
- **Tujuan:** Memberi status harian semua akuifer untuk keluaran.
- **Masukan dan keluaran:** tidak ada; iterator berisi (nomor akuifer 1-based, id GIS, nama objek, `&AquDay`) dalam urutan akuifer.
- **Cara kerja:** memetakan `aqus` dengan `enumerate`, mengambil id GIS dan nama dari `obs[a.ob]`.

### 14.4  Keluaran akuifer: akumulasi periode dan basin

Status harian `AquDay` dicetak ke `aquifer_day.txt` dan dijumlahkan menjadi bulanan, tahunan dan rata-rata tahunan (`aquifer_mon/yr/aa`), serta ditimbang luas menjadi akuifer basin (`basin_aqu_*`). Fungsi-fungsi ini ada di `crates/swat-engine/src/periods.rs` dan berpadanan dengan operator `+`, `/`, `*` yang didefinisikan untuk `aquifer_dynamic` di `aquifer_module.f90`, serta subrutin `aquifer_output.f90` dan `basin_aquifer_output.f90`. Penulisan berkas dan logika umum periode (`PeriodDay`, `Period`) dibahas di bab keluaran; di sini hanya bagian khusus akuifer.

**`AquAcc`** (publik): tiga `AquDay` sebagai penampung jumlah bulan (`m`, padanan `aqu_m`), tahun (`y`, `aqu_y`) dan seluruh periode cetak (`a`, `aqu_a`). Satu `AquAcc` per akuifer disimpan di `run.rs` (`aqu_acc`), ditambah satu untuk basin (`basin_aqu_acc`).

#### 14.4.1  `aqu_add`

- **Lokasi:** `crates/swat-engine/src/periods.rs`, `fn aqu_add(a: &mut AquDay, b: &AquDay)`
- **Padanan SWAT+:** fungsi `aqu_add` (operator `+`) di `aquifer_module.f90`.
- **Tujuan:** Menjumlahkan dua status akuifer field demi field.
- **Masukan dan keluaran:** `a` (akumulator, diubah), `b` (yang ditambahkan).
- **Cara kerja:**
  1. Untuk 16 field (`flo, dep_wt, stor, no3_st, minp, orgn, rchrg, no3_rchg, no3_loss, seep, revap, no3_lat, no3_seep, flo_cha, flo_res, flo_ls`): `a.x += b.x`.
  2. `a.cbn = b.cbn` (karbon tidak dijumlahkan, diambil dari operand kedua).
- **Kebiasaan SWAT+ / catatan port:** Urutan penjumlahan field mengikuti Fortran (tidak berpengaruh pada nilai karena tiap field independen). Karena `cbn` diambil dari operand kanan, karbon periode adalah nilai hari terakhir; di basin, karbon adalah milik akuifer terakhir dalam loop.

#### 14.4.2  `aqu_div`

- **Lokasi:** `crates/swat-engine/src/periods.rs`, `fn aqu_div(a: &AquDay, c: f32) -> AquDay`
- **Padanan SWAT+:** fungsi `aqu_div` (operator `/`) di `aquifer_module.f90`.
- **Tujuan:** Membagi semua field kecuali karbon dengan konstanta.
- **Masukan dan keluaran:** status `a`, pembagi `c` (jumlah tahun cetak); status baru.
- **Cara kerja:** setiap field `x / c`, `cbn` disalin.

#### 14.4.3  `AquAcc::day`

- **Lokasi:** `crates/swat-engine/src/periods.rs`, `pub fn day(&mut self, d: &AquDay, c: &PeriodDay, emit: impl FnMut(Period, &AquDay))`
- **Padanan SWAT+:** bagian akumulasi `aquifer_output.f90` (dan bentuk yang identik di `basin_aquifer_output.f90`).
- **Tujuan:** Menambahkan hari ini ke jumlah bulanan dan, di akhir bulan/tahun/simulasi, mengubah jumlah menjadi nilai periode dan memanggil `emit`.
- **Masukan dan keluaran:** `d` (status hari ini atau jumlah basin hari ini), `c` (penanda akhir bulan/tahun/simulasi, jumlah hari bulan `mon_days`, tahun cetak `yrs_prt`), `emit` (penulis baris).
- **Cara kerja:**
  1. `m += d` (`aqu_add`).
  2. Bila akhir bulan: `m.stor`, `m.dep_wt`, `m.no3_st` dibagi `mon_days` (besaran keadaan dirata-rata, besaran fluks tetap jumlah); `y += m`; `emit(Mon, m)`; `m` dinolkan.
  3. Bila akhir tahun: `y.stor`, `y.dep_wt`, `y.no3_st` dibagi 12 (rata-rata dari rata-rata bulanan); `a += y`; `emit(Yr, y)`; `y` dinolkan.
  4. Bila akhir simulasi: `emit(Aa, aqu_div(a, yrs_prt))`.
- **Kebiasaan SWAT+ / catatan port:**
  - Pembagian tahunan dengan 12 berlaku walau tahun pertama cetak tidak dimulai Januari (Fortran sama).
  - Akumulasi dilakukan setiap hari tanpa bergantung pada bendera cetak, seperti Fortran yang menjumlah dan membagi tanpa melihat `pco%aqu`; hanya penulisan yang bergantung pada bendera (diurus pemanggil). Di Fortran pembagian `aqu_a / yrs_prt` hanya terjadi bila rata-rata tahunan dicetak; karena nilainya hanya dipakai untuk dicetak, hasil cetak sama.
  - `docs/outputs.md`: "Aquifers average storage, water table and nitrate stored over a month's days, then over 12 months; the carbon is the last day's."

#### 14.4.4  `aqu_scaled`

- **Lokasi:** `crates/swat-engine/src/periods.rs`, `pub fn aqu_scaled(a: &AquDay, c: f32) -> AquDay`
- **Padanan SWAT+:** fungsi `aqu_mult` (operator `*`) di `aquifer_module.f90`.
- **Tujuan:** Mengalikan semua field kecuali karbon dengan konstanta (bobot luas).
- **Masukan dan keluaran:** status `a`, faktor `c`; status baru.
- **Cara kerja:** salin `a`, lalu 16 field dikali `c`; `cbn` tetap.

#### 14.4.5  `aqu_sum`

- **Lokasi:** `crates/swat-engine/src/periods.rs`, `pub fn aqu_sum(acc: &mut AquDay, a: &AquDay, c: f32)`
- **Padanan SWAT+:** baris `baqu_d = baqu_d + aqu_d(iaq) * const` di `basin_aquifer_output.f90`.
- **Tujuan:** Menambahkan satu akuifer, ditimbang bagiannya dari luas basin, ke jumlah basin harian.
- **Masukan dan keluaran:** `acc` (jumlah basin hari ini), `a` (akuifer), `c` = `area_ha` akuifer / `area_tot_ha` basin.
- **Cara kerja:** `aqu_add(acc, &aqu_scaled(a, c))`.
- **Kebiasaan SWAT+ / catatan port:** Pemanggil di `run.rs` hanya menghitung akuifer basin bila proyek memiliki `aqu_catunit.ele` (`project.aqu_elems`, dibaca dari kolom `ele_aqu` bagian `regions` di `file.cio`), seperti komentar port: "basin\_aquifer\_output needs aqu\_catunit.ele". Luas yang dipakai adalah luas objek akuifer (`aqu_prm%area_ha`), dan karbon basin adalah karbon akuifer terakhir.

#### 14.4.6  Kait keluaran akuifer pada trait `OutputSink`

Trait keluaran di `crates/swat-engine/src/run.rs` (dibahas di bab runner) memiliki tiga metode khusus akuifer, semuanya berimplementasi bawaan kosong sehingga penulis keluaran hanya mengisi yang diperlukan:

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `aqu_day` | `run.rs`, `fn aqu_day(&mut self, day, iaq, gis_id, name, d: &AquDay)` | satu baris `aquifer_day` (nomor 1-based, id GIS, nama, status) |
| `aqu_period` | `run.rs`, `fn aqu_period(&mut self, p: Period, day, iaq, gis_id, name, d)` | satu baris `aquifer_mon/yr/aa` |
| `basin_aqu` | `run.rs`, `fn basin_aqu(&mut self, p: Option<Period>, day, name, d)` | baris `basin_aqu_*` (`None` = harian) |

Di loop harian `run.rs`, setelah `Network::day`, urutannya: (1) jumlah basin akuifer dibentuk dengan `aqu_sum` atas `n.aquifers()` bila `project.aqu_elems`; bila ada akuifer, ditulis harian (bila hari dicetak) dan diakumulasi `basin_aqu_acc.day`; (2) kanal ditulis; (3) setiap akuifer ditulis harian (bila hari dicetak) dan diakumulasi pada `aqu_acc[k]`.

### 14.5  Kalibrasi keras: model data dan pembacaan berkas

Kalibrasi keras SWAT+ terdiri dari dua berkas di bagian `chg` `file.cio`: `cal_parms.cal` (daftar parameter yang boleh diubah, jenis objeknya, dan batas mutlaknya) dan `calibration.cal` (daftar perubahan: parameter, jenis perubahan, nilai, kondisi, lapisan tanah, tahun, hari, dan daftar objek). Port membaca keduanya di `crates/swat-io/src/project.rs` (komentar `// proc_cal`) ke `project.cal_parms` dan `project.cal_upd`. Pembacaan berkas secara umum dibahas di bab masukan; subbab ini mencantumkan setiap fungsi dan tipe agar bab ini lengkap, dengan penekanan pada makna field bagi penerapan. Kalibrasi lunak (`calsoft_*`, `caltsoft_*`, `calhard_control`, `pl_read_parms_cal`, `ls_read_lsparms_cal`, `ch_read_*_cal`, `aqu_read_elements` untuk wilayah) tidak diport.

**Tipe data** (`crates/swat-model/src/cal.rs`):

| Tipe | Padanan Fortran | Field utama |
| --- | --- | --- |
| `CalParm` | `calibration_parameters` (`cal_parms`) | `name` (nama parameter, mis. `cn2`), `ob_typ` (`hru`, `sol`, `bsn`, `aqu`, `plt`, `cli`, `rdt`, …), `absmin`, `absmax` (batas nilai hasil perubahan, satuan parameter). Kolom `units` Fortran tidak disimpan. |
| `CalUpdate` | `update_parameters` (`cal_upd`) | `name`, `chg_typ` (`absval`, `abschg`, `pctchg`, `relchg`), `val`, `conds: Vec<CalCond>`, `val1`, `val2` (batas kondisi `range` terakhir), `lyr1`, `lyr2` (lapisan tanah), `year1`, `year2`, `day1`, `day2` (dipakai SWAT+ untuk iklim dan tabel keputusan waduk), `elems` (nomor objek 1-based; kosong = semua objek jenisnya). |
| `CalCond` | `calibration_conditions` | `var` (variabel yang diuji: `hsg`, `texture`, `landuse`, `cal_group`, `plant`, …), `alt` (operator), `targ` (angka; tidak dipakai kondisi yang diterapkan), `targc` (teks yang harus sama). |

#### 14.5.1  `CalParm::default`

- **Lokasi:** `crates/swat-model/src/cal.rs`, `impl Default for CalParm`
- **Padanan SWAT+:** nilai awal field `calibration_parameters` di `calibration_data_module.f90`.
- **Tujuan:** Baris parameter pengganti bila nama perubahan tidak ditemukan di `cal_parms.cal`.
- **Masukan dan keluaran:** `CalParm { name: "default", ob_typ: "plt", absmin: 0.0, absmax: 1.0e6 }`.
- **Kebiasaan SWAT+ / catatan port:** Di Fortran, `cal_upd%num_db` tetap 0 bila nama tidak ditemukan dan `cal_parms(0)` berisi nilai bawaan ini (`ob_typ` "plt"). Akibatnya di port, nama yang tidak dikenal berakhir sebagai parameter jenis `plt`, yang ditolak `cal_conditions` dengan pesan "(plt parameter)".

#### 14.5.2  `define_unit_elements`

- **Lokasi:** `crates/swat-model/src/cal.rs`, `pub fn define_unit_elements(cnt: &[i32]) -> Vec<usize>`
- **Padanan SWAT+:** `define_unit_elements.f90`.
- **Tujuan:** Mengurai daftar nomor objek ringkas, di mana bilangan negatif menutup rentang yang dibuka bilangan sebelumnya (`1 -1858` berarti 1 sampai 1858).
- **Masukan dan keluaran:** `cnt` (bilangan bulat dari berkas); daftar nomor objek 1-based.
- **Cara kerja:**
  1. Bila hanya satu entri: hasilnya entri itu (negatif dipotong ke 0).
  2. Selain itu, dengan penunjuk `ii` dari 1: bila `ii` di entri terakhir, pasangan (sebelumnya, terakhir) diperiksa: terakhir positif → tambahkan terakhir; negatif → tambahkan rentang `ie1..=|ie2|`. Bila bukan terakhir, pasangan (`cnt[ii]`, `cnt[ii+1]`): yang kedua positif → tambahkan yang pertama, maju satu; negatif → tambahkan rentang, maju dua.
- **Kebiasaan SWAT+ / catatan port:** Uji unit: `[1,-4]` → 1..4; `[7]` → 7; `[2,5,-7,9]` → 2,5,6,7,9. Port menghasilkan daftarnya langsung (Fortran menghitung panjang dulu, lalu mengisi `defunit_num`).

#### 14.5.3  `read_cal_parms`

- **Lokasi:** `crates/swat-io/src/cal.rs`, `pub fn read_cal_parms(file: &str, text: &str) -> Result<Vec<CalParm>>`
- **Padanan SWAT+:** `cal_parm_read.f90`.
- **Tujuan:** Membaca `cal_parms.cal`.
- **Masukan dan keluaran:** isi berkas; daftar `CalParm` dalam urutan berkas.
- **Cara kerja:**
  1. Lewati judul; baca jumlah parameter *n*; lewati header.
  2. Untuk *n* baris: baca sampai 5 nilai (`name`, `ob_typ`, `absmin`, `absmax`, `units`); nilai yang hilang memakai bawaan `CalParm`. Berhenti pada akhir berkas.

#### 14.5.4  `read_calibration`

- **Lokasi:** `crates/swat-io/src/cal.rs`, `pub fn read_calibration(file: &str, text: &str) -> Result<Vec<CalUpdate>>`
- **Padanan SWAT+:** `cal_parmchg_read.f90`.
- **Tujuan:** Membaca `calibration.cal` menjadi daftar perubahan beserta objek dan kondisinya.
- **Masukan dan keluaran:** isi berkas; daftar `CalUpdate`.
- **Cara kerja:**
  1. Lewati judul; baca jumlah perubahan; lewati header.
  2. Untuk tiap perubahan, catat posisi, baca 11 nilai: `name, chg_typ, val, conds, lyr1, lyr2, year1, year2, day1, day2, nspu`. Akhir berkas menghentikan loop.
  3. Bila `nspu > 0`, kembali ke posisi catatan (padanan `backspace`) dan baca ulang baris dengan `11 + nspu` nilai: nilai ke-11 kini dibaca sebagai `num_tot` dan sisanya sebagai daftar objek ringkas; bila `num_tot != 0`, `elems = define_unit_elements(daftar)`. `num_tot = 0` berarti semua objek.
  4. Untuk setiap kondisi (`conds` baris): intip kata pertama; bila `range`, baca `range var val1 val2` (hanya `var` disimpan sebagai kondisi; `val1`/`val2` disimpan di perubahan, menimpa yang sebelumnya); selain itu baca `var alt targ targc`.
- **Kebiasaan SWAT+ / catatan port:** Jumlah objek bawaan bila daftar kosong (Fortran mengisi `num_elem` dari jumlah objek jenisnya: `sp_ob%hru` untuk `hru`/`sol`, `sp_ob%aqu` untuk `aqu`, 1 untuk `bsn`, dst.) ditentukan saat penerapan oleh `cal_elems`, bukan saat membaca. Pencarian `num_db` (indeks parameter) juga ditunda ke `cal_parm`.

### 14.6  Penerapan kalibrasi keras pada HRU, tanah dan basin

Semua fungsi subbab ini ada di `crates/swat-engine/src/hru/mod.rs`. `cal_conditions` dipanggil sekali di akhir `init_hrus`, setelah semua HRU (termasuk lahan basah dan nilai `evrsv_et`) disiapkan dan sebelum `nutcarb_stage` (inisialisasi nutrien dan karbon tanah), sehingga nutrien tanah dihitung dari tanah yang sudah dikalibrasi, sama seperti `soil_nutcarb_init` yang datang setelah `proc_cal` di SWAT+. Perubahan akuifer memakai `chg_par`, `cal_parm` dan `cal_elems` yang sama dari `Network::new` (14.2.2). Rangkuman kebijakan port tercatat di `docs/engine1.md`, bagian *Hard calibration with conditions*.

#### 14.6.1  `chg_par`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `pub(crate) fn chg_par(u: &CalUpdate, p: &CalParm, cur: f32) -> f32`
- **Padanan SWAT+:** fungsi `chg_par` di `chg_par.f90`.
- **Tujuan:** Menghitung nilai baru parameter dari nilai sekarang, jenis perubahan dan besarnya, lalu membatasinya ke rentang mutlak `cal_parms.cal`.
- **Masukan dan keluaran:** `u.chg_typ`, `u.val` (v), `p.absmin`, `p.absmax`, nilai sekarang `cur` (x); nilai baru.
- **Cara kerja:**
  1. Menurut jenis perubahan:

```latex
x' = \begin{cases} v & \text{absval} \\ x + v & \text{abschg} \\ (1 + v/100)\,x & \text{pctchg} \\ (1 + v)\,x & \text{relchg} \end{cases}
```

2. Batasi: `x' = min(max(x', absmin), absmax)` (Fortran: `Max` lalu `amin1`).

- **Kebiasaan SWAT+ / catatan port:** Cabang `_` di Rust memperlakukan semua jenis lain sebagai `relchg`; jenis yang tidak dikenal sudah ditolak lebih dulu oleh `cal_conditions` (di Fortran hasilnya tak terdefinisi). Perhitungan `(1.0 + u.val / 100.0) * cur` dalam `f32` dengan urutan yang sama dengan Fortran.

#### 14.6.2  `cal_parm`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `pub(crate) fn cal_parm(project: &Project, u: &CalUpdate) -> CalParm`
- **Padanan SWAT+:** pencarian `num_db` di `cal_parmchg_read.f90` (loop `ical` yang mencari `cal_upd(i)%name == cal_parms(ical)%name`).
- **Tujuan:** Mencari baris `cal_parms.cal` milik sebuah perubahan.
- **Masukan dan keluaran:** proyek, perubahan; salinan `CalParm` yang cocok, atau `CalParm::default()` bila tidak ada.
- **Cara kerja:** mencari baris pertama yang namanya sama menurut `fortran_eq` (perbandingan string gaya Fortran, mengabaikan spasi di akhir).

#### 14.6.3  `cal_elems`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `pub(crate) fn cal_elems(u: &CalUpdate, n: usize) -> Vec<usize>`
- **Padanan SWAT+:** pengisian `cal_upd%num` di `cal_parmchg_read.f90` (`num_tot == 0` → semua objek jenisnya).
- **Tujuan:** Menentukan objek (1-based) yang diubah oleh perubahan.
- **Masukan dan keluaran:** `u.elems`, `n` (jumlah objek jenis parameter); daftar nomor.
- **Cara kerja:** bila `u.elems` kosong, `1..=n`; selain itu salinan `u.elems`.

#### 14.6.4  `cal_cond_met`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `pub(crate) fn cal_cond_met(u: &CalUpdate, hrus: &[Hru], e: usize) -> Result<bool, String>`
- **Padanan SWAT+:** loop `ic` (kondisi) di `cal_conditions.f90`.
- **Tujuan:** Menentukan apakah objek bernomor `e` memenuhi semua kondisi perubahan.
- **Masukan dan keluaran:** perubahan `u`, larik HRU, nomor `e` (1-based); `Ok(true/false)`, atau `Err` untuk kondisi yang tidak didukung.
- **Cara kerja:**
  1. Tanpa kondisi: `true`.
  2. Ambil HRU `e` (galat bila tidak ada). Pembanding teks `eq(a, targc)`: `targc` dipotong ke 25 karakter (panjang `character(len=25)` Fortran), lalu keduanya dibandingkan tanpa spasi di akhir.
  3. Untuk setiap kondisi menurut `var`: `hsg` → kelompok hidrologi tanah HRU (`hydgrp`); `texture` → tekstur tanah; `plant` → nama tanaman **pertama** komunitas (bila komunitas kosong, kondisi dianggap terpenuhi); `landuse` → nama `land_use_mgt` HRU, dan bila gagal loop kondisi **berhenti** (`break`); `cal_group` → grup kalibrasi HRU. Kegagalan menjadikan hasil `false` (kecuali `landuse`, kondisi lain tetap diperiksa).
  4. `res_pvol`, `res_typ`, `pl_class` → `Err` (tidak didukung). Variabel lain (termasuk `range` yang `var`-nya dipakai sebagai nama) tidak dianggap kondisi.
- **Kebiasaan SWAT+ / catatan port:**
  - Kondisi selalu diuji pada **HRU yang bernomor sama dengan objek**, apa pun jenis parameternya (Fortran memakai `soil(ielem)`, `hru(ielem)`, `pcom(ielem)` langsung).
  - Kondisi `plant` di Fortran berada dalam loop tanaman yang keluar (`exit`) setelah tanaman pertama, sehingga hanya tanaman pertama yang diperiksa; komunitas tanpa tanaman tidak masuk loop sehingga kondisi terpenuhi.
  - `exit` pada `landuse` keluar dari loop kondisi, sedangkan kondisi lain tidak; port mereproduksi asimetri ini (bagi hasil akhir tidak berbeda karena hasil sudah `false`).

#### 14.6.5  `cal_conditions`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn cal_conditions(project: &Project, hrus: &mut [Hru]) -> Result<(), String>`
- **Padanan SWAT+:** `cal_conditions.f90` (dipanggil dari `proc_cal.f90`), cabang HRU, `sol` dan `bsn`.
- **Tujuan:** Menerapkan setiap baris `calibration.cal` pada HRU/tanah/basin yang memenuhi kondisinya.
- **Masukan dan keluaran:** proyek (`cal_upd`, `cal_parms`, parameter basin), HRU yang sudah disiapkan; HRU yang diubah, atau `Err` dengan pesan yang menyebut apa yang tidak diport.
- **Cara kerja:** untuk setiap perubahan `u` berurutan:
  1. `p = cal_parm(project, u)`. Bila `p.ob_typ` bukan `hru`, `sol`, `bsn` atau `aqu`: `Err("calibration of … (<ob_typ> parameter)")`. Bila `chg_typ` bukan salah satu dari empat jenis: `Err`.
  2. `aqu`: dilewati (ditangani di `Network::new`).
  3. `bsn`: bila `cal_cond_met(u, hrus, 1)` (kondisi diuji pada HRU 1), terapkan sekali. Satu-satunya parameter basin yang didukung adalah `surlag`: `surlag = chg(surlag)`, dengan `surlag` lokal yang diawali dari `params["surlag"]` (atau 4,0) dan memuat hasil perubahan sebelumnya, lalu untuk setiap HRU koefisien lag aliran permukaan dihitung ulang:

```latex
brt = 1 - \exp\!\left(\frac{-surlag}{t_{conc}}\right)
```

dengan t\_conc waktu konsentrasi HRU (jam). Parameter basin lain: `Err`.

4. `hru` dan `sol`: untuk setiap `e` dalam `cal_elems(u, jumlah HRU)` (galat bila `e` = 0 atau > jumlah), lewati bila kondisi tidak terpenuhi. Untuk `sol`: lapisan `lyr1 = max(u.lyr1, 1)` sampai `lyr2` (`u.lyr2 <= 0` → lapisan terakhir, selain itu `min(u.lyr2, nly)`), dan `cal_soil` dipanggil untuk tiap lapisan. Untuk `hru`: `cal_hru`.

- **Kebiasaan SWAT+ / catatan port:**
  - Perubahan datang **setelah** inisialisasi HRU: hanya besaran yang dihitung kemudian (atau yang dihitung ulang secara eksplisit di `cal_parm_select`) melihat nilai baru. Misalnya perubahan `slope` tidak mengubah waktu konsentrasi dan faktor LS USLE yang sudah dihitung.
  - Jenis parameter `plt` (`phu_mat`, `lai_pot`, `harv_idx`, …), `cli` (`precip`, `temp` per hari), `rdt` (tabel keputusan waduk), `hlt`, `cha`, `res`, `swq`, `gwf` dan lain-lain tidak diport dan ditolak.
  - Untuk `bsn`, port menerapkan perubahan tepat sekali dan mengabaikan daftar objek; di Fortran, `num_elem` bawaan untuk `bsn` adalah 1, sehingga sama untuk berkas biasa. Sejak 1.0.1 `cal_conditions` menyimpan `surlag` lokal yang diawali dari nilai basin, dan setiap perubahan dimulai dari hasil perubahan sebelumnya, seperti `bsn_prm%surlag` di Fortran: dua `pctchg` 50 lalu −20 membuat 4 hari menjadi 4,8. Divalidasi oleh **Ps\_edge** (Ps\_coast dengan `surlag` dikalibrasi dua kali dan setiap lahan basah bernomor ganjil tanpa baris `sediment.res`).
  - Pemeriksaan jenis objek dan jenis perubahan dilakukan untuk semua baris sebelum penerapan baris itu, sehingga proyek dengan kalibrasi yang tidak didukung gagal secara eksplisit, tidak diam-diam diabaikan.

#### 14.6.6  `cal_hru`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn cal_hru(h: &mut Hru, name: &str, chg: &dyn Fn(f32) -> f32) -> Result<(), String>`
- **Padanan SWAT+:** cabang HRU di `cal_parm_select.f90`.
- **Tujuan:** Mengubah satu parameter HRU dan menghitung ulang turunannya.
- **Masukan dan keluaran:** HRU, nama parameter, penutup `chg` (= `chg_par` dengan perubahan dan batasnya); HRU diubah, `Err` untuk nama yang tidak diport.
- **Cara kerja** (menurut nama):

| Nama | Field | Dihitung ulang / catatan |
| --- | --- | --- |
| `perco` | `perco` | `perco_lim` (lihat persamaan di bawah); dilewati pada HRU berdrainase pipa |
| `cn2` | `cn2` | `curno` (CN1, CN3, parameter retensi; Bab HRU) |
| `cn3_swf` | `cn3_swf` | `curno`; dilewati pada HRU berdrainase pipa |
| `petco` | `pet_co` | – |
| `latq_co`, `canmx`, `esco`, `lat_len`, `erorgn`, `erorgp`, `usle_p`, `elev`, `slope`, `field_wid` | field bernama sama | – |
| `biomix` | `hyd_biomix` | – |
| `lat_ttime` | `lat_ttime` | nilai tersimpan sudah berbentuk 1−exp(−1/t); perubahan diterapkan pada bentuk itu, lalu `lat_ttime = 1 − exp(−1/chg(lat_ttime))` |
| `epco` | `epco` | disalin ke `epco` setiap tanaman komunitas |
| `snofall_tmp`, `snomelt_tmp`, `snomelt_max`, `snomelt_min`, `snomelt_lag` | `sno.falltmp`, `sno.melttmp`, `sno.meltmx`, `sno.meltmn`, `sno.timp` | – |
| `ovn`, `slope_len`, `field_len`, `field_ang`, `dis_stream` | – | hanya dipakai saat inisialisasi (atau tidak sama sekali): diterima tanpa efek |
| lainnya | – | `Err("calibration of <nama>")` |

Persamaan batas perkolasi untuk `perco` (bila `perco > 1e-6`, selain itu `perco_lim = 0`):

```latex
f = 1.0052\,\ln\!\big(-\ln(perco - 10^{-6})\big) + 5.6862,\qquad perco_{lim} = \min\big(1,\; e^{-f}\big)
```

- **Kebiasaan SWAT+ / catatan port:**
  - `lat_ttime` diubah setelah nilainya menjadi 1−exp(−1/t), lalu dijadikan bentuk itu lagi (`docs/engine1.md`). Port menulisnya sebagai satu ekspresi; urutan operasinya sama dengan dua baris Fortran.
  - Syarat drainase: Fortran memeriksa `hru%tiledrain == 0`; port memeriksa `h.tile.is_some()`.
  - `usle_p` diubah tanpa menghitung ulang `usle_mult` (seperti Fortran); faktor USLE gabungan baru berubah bila `rock` lapisan 1 atau `usle_k` juga diubah sesudahnya.
  - Tidak diport (ditolak): `usle_c`, `lat_sed`, `lat_orgn`, `lat_orgp`, semua `tile_*`, `cmn_h`, `nperco_h`, `pperco_h`, `phoskd_h`, `psp_h`, `nperco_lchtile_h`.

#### 14.6.7  `cal_soil`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn cal_soil(h: &mut Hru, ly: usize, name: &str, chg: &dyn Fn(f32) -> f32) -> Result<(), String>`
- **Padanan SWAT+:** cabang tanah di `cal_parm_select.f90`.
- **Tujuan:** Mengubah satu parameter lapisan tanah `ly` (0-based) dan menghitung ulang turunannya.
- **Masukan dan keluaran:** HRU, lapisan, nama, penutup `chg`; HRU diubah atau `Err`.
- **Cara kerja:**
  1. `z` (kedalaman dasar lapisan, mm), `bd` (berat isi, g/cm3), `awc` (kapasitas air tersedia, mm/mm): ubah field lapisan, lalu `soil_awc_init(h)` dan `curno(h)`.
  2. `k` (konduktivitas jenuh, mm/jam): ubah, lalu waktu tempuh perkolasi lapisan `hk = (ul − fc)/k`, minimal 1.
  3. `rock` (% batuan): ubah; bila lapisan pertama, hitung ulang faktor USLE gabungan.
  4. `usle_k`: ubah dan selalu hitung ulang faktor USLE gabungan (memakai `usle_k` dan `rock` lapisan **pertama**):

```latex
usle_{mult} = e^{-0.053\,rock_{1}}\; K_{1}\; P\; LS\; 11.8
```

5. `alb`, `cal` (CaCO3): ubah field lapisan. `anion_excl`: ubah field tanah (satu nilai untuk seluruh profil).
6. `ec`, `ph`: diterima tanpa efek (hanya dipakai saat inisialisasi tanah). Lainnya (`cbn`, `clay`, `silt`, `sand`, `crk`, …): `Err`.

- **Kebiasaan SWAT+ / catatan port:**
  - `anion_excl` milik tanah tetapi dipanggil per lapisan; dengan `pctchg`/`relchg` perubahan **bertumpuk** sebanyak jumlah lapisan yang diubah (`docs/engine1.md`). Port mereproduksinya.
  - Kalibrasi `usle_k` pada lapisan selain pertama tetap menghitung ulang `usle_mult` dari lapisan pertama, sehingga tidak mengubah apa pun selain field lapisan itu.

#### 14.6.8  `soil_awc_init`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn soil_awc_init(h: &mut Hru)`
- **Padanan SWAT+:** `soil_awc_init.f90` (dipanggil dari `cal_parm_select` untuk `z`, `bd`, `awc`, `clay`).
- **Tujuan:** Menghitung ulang kandungan air karakteristik setiap lapisan dari `awc`, `bd` dan liat, lalu mengembalikan air tanah ke bagian awal kapasitas lapang.
- **Masukan dan keluaran:** HRU (lapisan: `awc`, `bd`, `clay`, `d`, `k`; `ffc`); keluaran per lapisan `wp`, `up`, `por`, `thick`, `ul`, `fc`, `st`, `hk`, `wpmm`; per HRU `sumfc`, `sumul`, `sw`, `avbd`.
- **Cara kerja:**
  1. Per lapisan: batasi `awc` ke \[0,005; 0,8\] (≤ 1e-6 → 0,005; ≥ 0,8 → 0,8). Titik layu `wp = 0,4·clay·bd/100` (≤ 0 → 0,005); kapasitas lapang volumetrik `up = wp + awc`; porositas `por = 1 − bd/2,65`. Bila `up ≥ por`: `up = por − 0,05`, `wp = up − awc`, dan bila `wp ≤ 0`: `up = 0,75·por`, `wp = 0,25·por`.
  2. Per lapisan, dari atas: `thick = d − d_sebelumnya`; `ul = (por − wp)·thick`; `fc = thick·(up − wp)`; `st = fc·ffc`; `hk = (ul − fc)/k` minimal 1; `wpmm = wp·thick`; jumlahkan `sumul`, `sumfc`, `sw` dan pori total.
  3. Porositas rata-rata = pori total / kedalaman profil; `avbd = 2,65·(1 − porositas rata-rata)`.
- **Kebiasaan SWAT+ / catatan port:**
  - Air tanah dikembalikan ke `fc·ffc`, yaitu bagian awal kapasitas lapang (`docs/swat-quirks.md`: "which puts the soil water back to its initial share of field capacity").
  - Port tidak menghitung besaran Fortran yang tidak dipakai engine: `vwt`, `sumwp`, `crdep`, `volcr`, `swpwt`, `wat_tbl` dan `avpor` sebagai field (pencarian di `crates/swat-engine/src` tidak menemukan pemakaiannya).

### 14.7  Ringkasan, cakupan dan verifikasi

**Daftar fungsi bab ini** (26 fungsi/metode, ditambah dua blok di dalam `Network::new`):

| Kelompok | Fungsi | Subbab |
| --- | --- | --- |
| Tipe akuifer | `AquDay::columns`, `AquDay::columns_as`, `AquiferDb::default`, `read_aquifers` | 14.1 |
| Inisialisasi | blok `aqu_initial`, blok kalibrasi akuifer (di `Network::new`) | 14.2 |
| Harian | `Network::aqu_1d_control`, `Network::aquifers` | 14.3 |
| Keluaran | `aqu_add`, `aqu_div`, `AquAcc::day`, `aqu_scaled`, `aqu_sum`, `OutputSink::aqu_day`, `OutputSink::aqu_period`, `OutputSink::basin_aqu` | 14.4 |
| Data/pembacaan kalibrasi | `CalParm::default`, `define_unit_elements`, `read_cal_parms`, `read_calibration` | 14.5 |
| Penerapan kalibrasi | `chg_par`, `cal_parm`, `cal_elems`, `cal_cond_met`, `cal_conditions`, `cal_hru`, `cal_soil`, `soil_awc_init` | 14.6 |

**Alur data akuifer.** HRU mengirim hidrograf resapan (`rhg`, jenis 2) atau total ke akuifer menurut berkas sambungan (`*.con`); unit routing meneruskan resapan elemennya tanpa rasio pengantaran (Bab 12). Loop perintah menjumlahkan masukan menjadi `hin`, lalu `aqu_1d_control` menghasilkan `hd[0]` (aliran balik, biasanya ke kanal SWAT-DEG `sdc` dengan jenis `tot`) dan `hd[1]` (rembesan dalam, jenis `rhg`, yang hanya berpindah bila sambungan menyebut objek tujuan). Air yang meninggalkan sistem lewat revap tidak dikembalikan ke HRU; revap hanya mengurangi simpanan.

**Batas port.** Proyek dengan akuifer 2-D, gwflow, alokasi air dari akuifer, garam/konstituen, atau kalibrasi jenis selain `hru`/`sol`/`bsn`/`aqu` ditolak dengan pesan eksplisit, bukan dijalankan sebagian.

**Verifikasi.** `docs/engine1.md` mencatat skenario **Ps\_cal** (dibuat oleh `scripts/make-pessel-variants.py`): Ps\_coast ditambah 24 perubahan kalibrasi, mencakup CN pada kelompok hidrologi C dan D, `esco`, `epco` dengan kondisi tanaman, `canmx`, `awc` pada dua lapisan atas tanah lempung berliat, `k`, `bd`, `anion_excl`, `usle_k`, `alb`, aliran lateral, kemiringan, `elev`, `erorgn`, `petco` dengan dua kondisi, `surlag`, dan enam parameter akuifer. Menurut dokumen itu semua tabel keluaran sama dengan SWAT+. Kebiasaan akuifer yang dipertahankan dirangkum di `docs/swat-quirks.md` (bagian *Routing units and aquifers (phase 3a)*) dan koreksinya di `docs/swat-fixes.md`; skenario uji yang memiliki objek akuifer dibandingkan pada tabel `aquifer_*` dan `basin_aqu_*` seperti tabel keluaran lainnya (daftar skenario per fitur ada di bab verifikasi).
