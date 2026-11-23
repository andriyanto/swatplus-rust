## Lampiran B  Jalur M: kalibrasi, mode paralel, dan studi spin-up

Lampiran ini mencatat pekerjaan sesudah engine 1.0, yaitu jalur M di `docs/roadmap.md` untuk riset MPhil (kalibrasi berbasis tinggi muka air, TMA), fungsi demi fungsi seperti bab-bab sebelumnya. Isinya masuk ke versi 1.1.0 (Zenodo, DOI 10.5281/zenodo.24448341), kecuali B.7 (studi spin-up), yang masuk ke versi 1.2.0 (DOI 10.5281/zenodo.24480818). Sejak 24 November 2026 Bab 1 sampai 17 menjelaskan engine 1.2.0: fungsi baru dan yang berubah sejak 1.0.1 (juga perubahan di luar jalur M, misalnya format tabel recall 1.0.3) masuk ke bab fungsinya dengan keterangan versi, dan lampiran ini mencatat jalannya pekerjaan serta hasilnya. Lampiran ini diperbarui setiap kali satu langkah selesai; B.8 mencatat urutan langkahnya.

### B.1  Ringkasan

| Langkah | Isi | Tempat di kode | Hasil |
| --- | --- | --- | --- |
| M1 | Kalibrasi di memori | `swat_model::cal`, `Simulation::with_calibration` | Identik dengan run salinan proyek yang `calibration.cal`-nya diubah |
| M2 | Banyak run sekaligus | `swat_engine::batch`, perintah `batch` | Satu run per inti; hanya deret yang dibutuhkan disimpan |
| M3 | Uji sintetis kalibrasi berbasis TMA | `calibration/swatcal` (Python) | Lengkung debit dua arah, TMA buatan, fungsi tujuan |
| M4 | Perbandingan pengoptimasi | `swatcal.optimize`, `swatcal.compare` | 3 *seed* × 80 run: tidak ada metode yang unggul konsisten |
| M5 | Routing paralel per sub-DAS | `route::make_plan`, `Network::day`, kumpulan thread rayon | Tabel identik untuk 1–12 thread; 1,67× pada 12 inti |
| Spin-up | Panjang pemanasan dan spin-up bersama | `Simulation::calibrate`, `batch --shared-spinup`, `swatcal.spinup` | B.7 |

### B.2  M1: kalibrasi di memori

SWAT+ membaca perubahan parameter dari `calibration.cal` dan menerapkannya sekali, sesudah semua objek disiapkan (`proc_cal` → `cal_conditions`). Program kalibrasi yang memakai SWAT+ harus menulis berkas itu sebelum setiap run. Engine menerima daftar perubahan yang sama langsung di memori.

- **`Calibration`** (`swat-model/src/cal.rs`): pasangan `parms` (isi `cal_parms.cal`: jenis objek dan batas tiap parameter) dan `updates` (isi `calibration.cal`). `Calibration::of(project)` adalah kalibrasi proyek sendiri; `Calibration::with_updates(project, updates)` memakai `parms` proyek dengan perubahan lain.
- **`CalUpdate::all(name, chg_typ, val)`**: satu perubahan untuk semua objek jenisnya. `absval` mengganti nilai, `abschg` menambah, `pctchg` mengubah sekian persen, `relchg` mengalikan dengan 1 + `val`. Hasilnya dibatasi ke rentang di `cal_parms.cal` (`chg_par`).
- **`Simulation::with_calibration(project, opts, cal)`**: menyiapkan simulasi dengan `cal` sebagai ganti kalibrasi proyek. Perubahan HRU dan tanah diterapkan oleh `cal_conditions` (`hru/mod.rs`), perubahan akuifer oleh `calibrate_aquifers` (`route.rs`), di tempat yang sama dengan SWAT+. Parameter yang tidak ada di `parms` ditolak, begitu juga kalibrasi yang membuat HRU tidak bisa disimulasikan.
- **Baris perintah**: `run --set <nama> <jenis> <nilai>` (setelah perubahan proyek), `--calibration <berkas>`, `--cal-parms <berkas>`.
- **Uji**: `tests/calibration.rs`. Run dengan kalibrasi di memori identik byte demi byte dengan run salinan Ames_sub1 yang `calibration.cal`-nya ditulis; 37 skenario validasi tidak berubah.

### B.3  M2: banyak run sekaligus

Run kalibrasi tidak saling bergantung, jadi cara tercepat adalah satu run per inti.

- **`Series`** (`batch.rs`): yang disimpan satu run untuk setiap hari yang dicetak: debit keluar saluran pilihan (`flo_out`, m³/s), kedalaman bila diminta (kurva rating SWAT+ dan rumus Manning), serta hujan dan ET rata-rata DAS (mm, dibobot luas HRU).
- **`SeriesSink`**: `OutputSink` yang mengisi `Series` dan membuang tabel lain, sehingga satu run hanya memakai memori sebesar deretnya.
- **`run_batch(project, opts, cals, channels, depth, threads, progress)`**: menjalankan proyek sekali per kalibrasi, `threads` run sekaligus. Setiap run berjalan dengan satu thread di dalamnya. Hasilnya tidak bergantung pada urutan atau jumlah run yang berjalan bersamaan.
- **Perintah `batch`**: `swatplus-rust batch <proyek> --sets sets.csv --channels 1,5 --out <folder>`. `sets.csv` berisi kolom `parameter:jenis` (misalnya `cn2:pctchg`) dan satu baris per run; hasilnya `run_<nama>.csv` dan ringkasan `runs.csv`.

### B.4  M3: uji sintetis kalibrasi berbasis TMA

Paket Python `calibration/swatcal` (hanya numpy) menguji metode ketika jawabannya diketahui.

- **`stage.Rating`**: lengkung debit Q = a(H − H₀)ᵇ dua arah (`stage`, `flow`).
- **`stage.synthetic_stage`**: TMA buatan dari debit model: tahap lengkung, galat bacaan normal, pembulatan ke resolusi papan duga (misalnya 1 cm), dan hari kosong.
- **`stage.fit_rating`**: lengkung yang paling cocok mengubah debit satu run menjadi TMA terukur, dengan kuadrat terkecil pada TMA. Untuk satu H₀, a dan b didapat dari regresi log lalu langkah Gauss-Newton; H₀ dicari dengan pemindaian dan pencarian *golden-section*. Hasilnya berubah mulus terhadap debit, tanpa loncatan grid, sehingga run dengan parameter berdekatan mendapat skor berdekatan.
- **`objective.evaluate`**: jarak dari titik ideal untuk empat bagian: KGE pada TMA, korelasi peringkat (Spearman) debit model dengan TMA, bias ET tahunan terhadap ET satelit, dan galat pengukuran debit (RMS log rasio debit lengkung terhadap debit terukur).
- **`metrics`**: NSE, KGE (beserta r, α, β), PBIAS, RSR, Spearman.
- **`runner.run_sets`**: menjalankan set parameter lewat `swatplus-rust batch` dan membaca deretnya (`RunSeries`).

### B.5  M4: perbandingan pengoptimasi

- **`params.Space`, `params.lhs`**: ruang parameter dan *Latin hypercube*.
- **`optimize.random_search`**: satu *Latin hypercube* seukuran anggaran run.
- **`optimize.sufi2`**: SUFI-2 (Abbaspour dkk., 2007): beberapa putaran *Latin hypercube*; sesudah tiap putaran, rentang dipersempit di sekitar run terbaik (persentil 2,5 dan 97,5 run *behavioural*).
- **`optimize.bayes_opt`**: optimasi Bayesian dengan proses Gaussian (Matérn 5/2, satu panjang skala per parameter), *expected improvement*, dan *batch* dengan *constant liar*. Bawaannya memakai *trust region* (TuRBO, Eriksson dkk., 2019): kandidat diambil di kotak sekitar titik terbaik; kotak membesar sesudah 3 *batch* berhasil, mengecil sesudah gagal, dan dimulai ulang bila terlalu kecil. `trust_region=False` adalah pencarian global.
- **`compare`**: dunia sintetis Batanghari 2015–2019 (dua tahun pemanasan). Kebenarannya CN2 +8 %, alpha aliran dasar −20 %, EPCO 0,6, *deep seepage* 0,15. Setiap metode mendapat 80 run, pada tiga keadaan pengetahuan (TMA saja; TMA + ET; TMA + ET + enam pengukuran debit), dengan beberapa *seed* (`--seeds 1,2,3`).
- **Hasil** (`calibration/README.md`): tidak ada metode yang unggul konsisten; selisih antar-*seed* selebar selisih antar-metode. Skor terbaik dengan pengukuran debit (0,068) lebih kecil dari skor parameter sebenarnya (0,070), padahal parameternya meleset 0,27 pada skala 0–1: pencarian ikut mencocokkan galat bacaan papan duga. Debitnya tetap baik (KGE ≥ 0,95).

### B.6  M5: routing paralel per sub-DAS

Di mode `--fix-swat`, HRU sudah berjalan paralel sejak sebelum M5 (`docs/parallel.md`). M5 menambahkan routing paralel dan mempercepat bagian yang masih berurutan.

- **Dua variabel `save` SWAT+ dijadikan milik tiap saluran** (mode `--fix-swat` saja): `inflo` di `ch_rtmusk` (nol bila tidak ada aliran masuk) dan `gra` di `ch_watqual4`. Di mode asli keduanya tetap dibagi antarsaluran seperti SWAT+ (`docs/swat-fixes.md`). Untuk itu `Channels` dipecah menjadi `ChanCfg` (pengaturan bersama), `Carry` (nilai bawaan mode asli) dan `AirLags`.
- **`make_plan(order, ins, threads) -> Plan`** (`route.rs`): memotong jaringan sekali per run. Setiap objek harus berjalan sesudah objek yang air hari ini-nya ia terima, dan sebelum objek yang air kemarin-nya ia terima. Dari outlet ke hulu, objek pertama yang bersama semua objek yang ia butuhkan berukuran paling banyak 1/(2 × thread) dari jaringan menjadi satu kelompok; kelompok yang berbagi objek (misalnya satu akuifer) digabung. Kelompok dibagi ke thread, yang terbesar dulu ke thread paling ringan. Sisanya (sungai utama dekat outlet) berjalan sesudahnya, berurutan.
- **`Network::day(nd, threads, pool)`**: menjalankan bagian-bagian `Plan` bersamaan (`run_part`), lalu sisanya. Hasilnya sama untuk berapa pun jumlah thread karena aliran masuk dijumlahkan dalam urutan tetap.
- **Kumpulan thread (rayon) dibuat sekali per run** dan dipakai untuk gelombang HRU (`hru_wave`) dan bagian routing. Sebelumnya thread dibuat dua kali sehari, yang di Windows lebih mahal daripada pekerjaan HRU proyek kecil.
- **`WaveAcc`**: jumlahan bulanan, tahunan dan rata-rata setiap HRU dibuat di thread HRU-nya, bersama hari HRU itu; barisnya dicetak sesudahnya dalam urutan HRU.
- **`TextTables::hru_day`** langsung kembali bila tidak ada tabel HRU harian yang dicetak. Sebelumnya fungsi ini menyusun baris setiap HRU setiap hari tanpa dipakai (Batanghari 1 thread: 16,8 s menjadi 13,7 s, semua mode).
- **Pengukuran** (`docs/parallel.md`, `scripts/bench-threads.ps1`): laptop Snapdragon X Elite 12 inti, Windows ARM, proyek seukuran Batanghari (2.360 HRU, 11 tahun): 15,4 s pada 1 thread, 9,2 s pada 12 thread (1,67×). Ke-24 run memberi tabel yang sama (satu sidik jari).

### B.7  Studi spin-up

Model hidrologi dimulai dari keadaan awal yang ditebak: air tanah, muka air tanah, simpanan lain. Tahun-tahun awal (pemanasan, *spin-up*; di SWAT+ `nyskip` di `print.prt`) dijalankan agar keadaan itu menyesuaikan diri dengan iklim, lalu tidak dicetak. Kalibrasi menjalankan model ratusan kali, sehingga dua pertanyaan penting: berapa tahun pemanasan yang cukup, dan apakah satu pemanasan bersama aman dipakai untuk semua set parameter. Roadmap mencatat yang kedua sebagai "diuji, tidak diasumsikan".

#### B.7.1  `Simulation::calibrate`

`Simulation::calibrate(&mut self, cal)` (`run.rs`) menerapkan perubahan `cal` pada simulasi yang sedang berjalan, di antara dua hari: parameter HRU dan akuifer berubah, sedangkan air, tanaman dan nutriennya tetap. Fungsi ini memanggil `cal_conditions` untuk HRU dan `Network::calibrate` (`calibrate_aquifers`) untuk akuifer, fungsi yang sama dengan saat penyiapan.

- Ini **bukan langkah SWAT+**: SWAT+ hanya menerapkan `calibration.cal` sekali, sebelum hari pertama. Karena itu fungsi ini hanya untuk kalibrasi dan studi, bukan untuk run yang dibandingkan dengan SWAT+.
- **Ditolak**: parameter yang tidak ada di `cal_parms.cal`, dan perubahan yang menyiapkan ulang keadaan, bukan mengubah parameter: kedalaman, bulk density dan air tersedia lapisan tanah (`soil_awc_init` menghitung ulang air tanah), *specific yield* dan nilai awal akuifer, serta `surlag` DAS (dihitung dari nilai proyek).
- **Uji** (`tests/calibration.rs`): perubahan yang diterapkan sebelum hari pertama memberi tabel yang sama persis dengan run yang disiapkan dengan perubahan itu; perubahan di tengah run membiarkan hari-hari sebelumnya sama dan mengubah hari sesudahnya; perubahan `awc` di tengah run ditolak.

#### B.7.2  `run_batch_shared` dan `batch --shared-spinup`

`run_batch_shared(project, opts, base, sets, year, …)` (`batch.rs`) menjalankan proyek dengan kalibrasinya sendiri (`base`) sekali sampai akhir tahun `year`, lalu setiap set mulai dari salinan keadaan itu (`checkpoint`) dengan perubahannya diterapkan lewat `calibrate`. Di baris perintah: `swatplus-rust batch … --shared-spinup`, dengan tahun pemanasan dari `nyskip`. Dengan demikian pemanasan suatu set memakai parameter proyek, bukan parameternya sendiri; seberapa besar pengaruhnya diukur di B.7.4.

Uji (`tests/batch.rs`): set tanpa perubahan dari pemanasan bersama sama persis, hari demi hari, dengan run proyek sendiri; set dengan perubahan berbeda dari run proyek dan dekat (tidak sama) dengan run set itu dengan pemanasannya sendiri.

#### B.7.3  Rancangan percobaan (`swatcal.spinup`)

`python -m swatcal.spinup --exe <program> --project <TxtInOut> --out <folder>` menjalankan dua bagian.

- **Bagian A, keadaan awal.** Tiga salinan proyek dimulai dari keadaan kering, biasa dan basah, dan mencetak setiap hari sejak hari pertama (`nyskip` 0):

  | Keadaan | Air tanah awal (`sw_init` di `parameters.bsn`, bagian dari kapasitas lapang) | Muka air tanah akuifer (`dep_wt` di `aquifer.aqu`) |
  | --- | --- | --- |
  | Kering | 0,05 | 95 % kedalaman akuifer (hampir kosong) |
  | Biasa | nilai proyek (0: dari hujan tahunan) | nilai proyek |
  | Basah | 1,0 | 5 % kedalaman akuifer (hampir penuh) |

  Untuk setiap tahun dihitung: selisih debit rata-rata outlet antara basah dan kering (persen dari debit biasa), NSE debit harian kering terhadap basah, selisih debit rendah (persentil 10, yang dilampaui 90 % hari), serta air tanah dan simpanan akuifer DAS akhir tahun. Pemanasan cukup bila ketiganya sudah tidak berbeda.
- **Bagian B, pemanasan bersama.** Dua puluh set parameter (*Latin hypercube* di ruang M4: CN2, alpha, EPCO, *deep seepage*) dan set kebenaran M4 dijalankan dua kali pada proyek penuh (2015–2025, dua tahun pemanasan): dengan pemanasan masing-masing, dan dari pemanasan bersama. Dihitung KGE dan PBIAS run bersama terhadap run sendiri, KGE masing-masing terhadap debit kebenaran, korelasi peringkat skor-skor itu, apakah set terbaiknya sama, dan waktunya.

#### B.7.4  Hasil

Percobaan dijalankan 24 November 2026 pada Batanghari dan Pessel (2015–2025), di outlet berupa saluran dengan debit rata-rata terbesar (saluran 1 pada keduanya). Tabel lengkap ada di `docs/spinup.md`; berikut ringkasannya.

**Bagian A.** Selisih basah dikurangi kering, dibagi run biasa:

| Tahun | Batanghari: debit tahunan | debit rendah | Pessel: debit tahunan | debit rendah |
| --- | ---: | ---: | ---: | ---: |
| 2015 | 40,5 % | 99,8 % | 55,5 % | 96,7 % |
| 2016 | 3,9 % | 19,4 % | 3,9 % | 2,1 % |
| 2017 | 1,3 % | −6,3 % | 0,6 % | 1,5 % |
| 2018–2025 | antara −1,1 dan 0,4 % | antara −8,7 dan 7,7 % | antara −0,2 dan 0,5 % | antara −9,0 dan 8,9 % |

- **Air tanah sudah lupa keadaan awalnya dalam tahun pertama**: akhir 2015, air tanah DAS ketiga run berselisih paling banyak 1 mm.
- **Volume tahunan perlu dua tahun**: selisihnya turun dari 40–56 % di tahun pertama menjadi 4 % di tahun kedua, dan sesudahnya dalam ±1 %. Dua tahun pemanasan proyek sudah cukup untuk volume.
- **Akuifer dangkal perlu dua sampai tiga tahun; akuifer dalam tidak pernah lupa.** Simpanan akuifer DAS (kolom `stor` di `basin_aqu_yr`) mempertahankan selisih awalnya sampai 2025 (Batanghari: kering 1.062, biasa 3.298, basah sekitar 3.390; Pessel: 1.105; 3.352; 3.395), tetapi jumlah itu didominasi akuifer dalam (B.7.5). Per akuifer di Batanghari, simpanan akuifer dangkal run basah dan kering berselisih 226 mm (median) akhir 2015, 55 mm pada 2016, 8 mm pada 2017, dan paling banyak 10 mm sejak 2020; dua akuifer dalam mempertahankan selisihnya.
- **Sisa keadaan awal masih terlihat di debit rendah dan harian**: debit rendah masih berbeda sampai 9 % antar-start pada tahun-tahun tertentu sesudah tahun ketiga, tanpa tren, dan di Pessel debit harian run kering dan basah hanya cocok dengan NSE 0,48–0,99. Akuifer dalam tidak mengalirkan air ke sungai, jadi sisa ini berasal dari akuifer dangkal; dugaan penyebabnya adalah aliran balik yang hidup-mati di ambang muka air tanah (`flo_min`), tempat selisih beberapa milimeter berpengaruh. Belum dibuktikan.

**Bagian B.** Dua puluh set parameter dan set kebenaran, pemanasan sendiri dibandingkan pemanasan bersama:

| | Batanghari | Pessel |
| --- | ---: | ---: |
| KGE harian, bersama terhadap sendiri (terendah; median) | 0,970; 0,980 | 0,866; 0,921 |
| Bias selama 9 tahun tercetak | −0,2 sampai +0,2 % | −0,3 sampai +0,4 % |
| Perubahan KGE tiap set terhadap kebenaran (rentang; median besarnya) | −0,011 sampai +0,007; 0,002 | −0,030 sampai +0,020; 0,010 |
| Korelasi peringkat KGE, bersama terhadap sendiri | 0,99 | 0,77 |
| Set terbaik, sendiri / bersama | s004 / s004 | s006 / s016 |
| Waktu 21 run, sendiri / bersama | 163 s / 139 s (−14 %) | 432 s / 364 s (−16 %) |

- **Batanghari masih menoleransi pemanasan bersama**: set terbaik sama, peringkat hampir tidak berubah.
- **Pessel tidak**: peringkat berubah (0,77) dan set terbaiknya lain. Di bagian A pun debit harian Pessel lebih lama membawa keadaan awalnya.
- **Penghematannya kecil**: dua dari sebelas tahun, 14–16 % waktu.

#### B.7.5  Neraca air akuifer (`swatcal.aquifers`)

`python -m swatcal.aquifers --exe <program> --project <TxtInOut> --out <folder>` menjalankan salinan proyek yang mencetak setiap akuifer per tahun sejak tahun pertama, lalu menjumlahkan neraca air akuifer dangkal dan akuifer dalam (nama `aqu_deep…` di `aquifer.con`, dibuat editor SWAT+ untuk menampung *deep seepage* akuifer dangkal), serta memeriksa apakah aliran balik akuifer dalam dikirim ke mana pun (`out_tot`).

Kedua proyek memakai akuifer bawaan editor. Akuifer dangkal (Batanghari 42, Pessel 230): alpha aliran dasar 0,05, *revap* 0,02, *deep seepage* 0,05 dari imbuhan, *specific yield* 0,05, aliran balik bila muka air tanah dalam 3 m, `dep_bot` 10 m. Akuifer dalam (Batanghari 2, Pessel 24): `dep_bot` 100 m, aliran balik hanya bila penuh. Rata-rata per akuifer, mm per tahun, 2015–2025:

| | Batanghari dangkal | Batanghari dalam | Pessel dangkal | Pessel dalam |
| --- | ---: | ---: | ---: | ---: |
| Imbuhan | 1.023 | 59 | 1.102 | 63 |
| Aliran balik | 943 | 7 | 1.020 | 10 |
| *Deep seepage* | 51 | 0 | 55 | 0 |
| *Revap* | 26 | 0 | 25 | 0 |
| Perubahan simpanan 2015–2025, mm (min, median, maks) | −12; 3; 17 | 490; 546; 601 | −88; 13; 25 | 0; 602; 607 |

- **Akuifer dangkal seimbang**: imbuhan sama dengan aliran balik, *seepage* dan *revap* dalam beberapa milimeter per tahun.
- **Akuifer dalam hanya terisi.** Mereka menerima *deep seepage* (5 % imbuhan dangkal), tidak mengembalikan apa pun sampai penuh, dan `aquifer.con` tidak mengirim alirannya ke mana pun (`out_tot` 0). Di Batanghari, satu dari keduanya penuh pada 2025 dan aliran baliknya (77 mm tahun itu) keluar dari model. Sekitar 5 % imbuhan hilang dari sungai karena rancangan ini; kalibrasi volume harus memperhitungkannya.
- **Aliran balik SWAT+ di atas ambang tidak bergantung pada simpanan** (`aqu_1d_control`: aliran balik hari itu adalah imbuhan yang dihaluskan dengan `alpha_bf`). Tidak ada yang menarik simpanan akuifer dangkal kembali ke suatu tingkat; selisih simpanan hanya meluruh lewat ambang. Itulah sebabnya keadaan awal bertahan lama, dan kalibrasi akuifer dangkal (alpha, `flo_min`, *deep seepage*) memerlukan debit rendah terukur, bukan hanya volume.

#### B.7.6  Kesimpulan untuk kalibrasi

1. Setiap run kalibrasi tetap memakai pemanasannya sendiri (bawaan `batch`). `--shared-spinup` hanya untuk eksplorasi cepat dan harus dicek terhadap pemanasan sendiri sebelum dipercaya.
2. Dua tahun pemanasan cukup untuk volume dan air tanah di kedua DAS.
3. Akuifer dangkal perlu dua sampai tiga tahun. Untuk kalibrasi debit rendah (Q80, Q90) atau bentuk debit harian, pakai tiga tahun pemanasan atau mulai akuifer dari keadaan akhir run panjang, dan perlakukan muka air tanah awal sebagai ketidakpastian.
4. Akuifer dalam terus terisi dan airnya tidak kembali ke sungai: kenaikan simpanan akuifer DAS bukan tanda model tidak stabil, tetapi sekitar 5 % imbuhan hilang lewat jalur ini. Parameter akuifer masih bawaan editor di semua akuifer; alpha, `flo_min` dan *deep seepage* adalah yang perlu dikalibrasi begitu debit rendah terukur (rekaman TMA) tersedia.

### B.8  Catatan langkah kerja

| Tanggal | Langkah | Hasil dan tempatnya |
| --- | --- | --- |
| 10 November 2026 | M1 kalibrasi di memori, M2 run *batch* | PR #13, #14; `docs/calibration.md` |
| 13 November 2026 | M3 uji sintetis, M4 perbandingan pengoptimasi (1 *seed*), M5 routing paralel per sub-DAS | PR #15, #16, #18; `calibration/README.md`, `docs/parallel.md` |
| 19 November 2026 | M4: *rating fit* mulus, *trust region*, perbandingan 3 *seed* | PR #19 |
| 23 November 2026 | Pengukuran 1–12 inti di laptop; thread sekali per run; rilis 1.1.0 | PR #20–#23; Zenodo 10.5281/zenodo.24448341 |
| 24 November 2026 | Studi spin-up: `Simulation::calibrate`, `batch --shared-spinup`, `swatcal.spinup`; pemanasan 2 tahun cukup untuk volume, akuifer dangkal menyatu dalam 2–3 tahun, akuifer dalam tidak pernah, pemanasan bersama tidak dipakai bawaan | B.7, `docs/spinup.md` |
| 24 November 2026 | Pemeriksaan neraca air akuifer (`swatcal.aquifers`): akuifer dangkal seimbang, akuifer dalam terus terisi dan airnya hilang dari sungai | B.7.5, `docs/spinup.md` |
| 24 November 2026 | Bab 1–17 diperbarui ke versi 1.1: `Simulation` dan method-nya, `batch.rs`, `aunif_skip`, `hru_wave` (Bab 5); `Calibration` (Bab 3); pembaca byte (Bab 4); `cal_conditions`/`calibrate_aquifers` dengan kalibrasi dari memori (Bab 7, 14); `Arc<PlantDb>` dan `Saved` per HRU (Bab 8); gelombang HRU, rencana routing per sub-DAS, `ChanCfg`/`Carry`/`AirLags`, `rating_curve`, `manning_depth`, `channel_hyd` (Bab 12); `list_real4` 1.0.3 dan `channel_hyd_day.txt` (Bab 15); opsi `run` baru dan perintah `batch` (Bab 16); koreksi F20–F22, validasi 1.0.3 dan 1.1, mode paralel (Bab 17) | PR #25; nota dibangun ulang (`scripts/build-nota-desain.sh`) |
| 24 November 2026 | Rilis 1.2.0: kalibrasi di tengah run, `batch --shared-spinup`, studi spin-up dan neraca akuifer, nota desain 1.2; run setia tidak berubah | PR #26, #27; judul nota menjadi 1.2; Zenodo 10.5281/zenodo.24480818 |
