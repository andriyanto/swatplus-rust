## Lampiran B  Jalur M: kalibrasi, mode paralel, dan studi spin-up

Lampiran ini mencatat pekerjaan sesudah engine 1.0, yaitu jalur M di `docs/roadmap.md` untuk riset MPhil (kalibrasi berbasis tinggi muka air, TMA), fungsi demi fungsi seperti bab-bab sebelumnya. Isinya masuk ke versi 1.1.0 (Zenodo, DOI 10.5281/zenodo.24448341), kecuali B.7 (studi spin-up), yang menyusul sesudahnya. Bab 1 sampai 17 tetap menjelaskan engine 1.0.1; perubahan kecil di luar jalur M ada di `CHANGELOG.md`. Lampiran ini diperbarui setiap kali satu langkah selesai; B.8 mencatat urutan langkahnya.

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

_Diisi sesudah percobaan selesai._

### B.8  Catatan langkah kerja

| Tanggal | Langkah | Hasil dan tempatnya |
| --- | --- | --- |
| 10 November 2026 | M1 kalibrasi di memori, M2 run *batch* | PR #13, #14; `docs/calibration.md` |
| 13 November 2026 | M3 uji sintetis, M4 perbandingan pengoptimasi (1 *seed*), M5 routing paralel per sub-DAS | PR #15, #16, #18; `calibration/README.md`, `docs/parallel.md` |
| 19 November 2026 | M4: *rating fit* mulus, *trust region*, perbandingan 3 *seed* | PR #19 |
| 23 November 2026 | Pengukuran 1–12 inti di laptop; thread sekali per run; rilis 1.1.0 | PR #20–#23; Zenodo 10.5281/zenodo.24448341 |
| 24 November 2026 | Studi spin-up: `Simulation::calibrate`, `batch --shared-spinup`, `swatcal.spinup` | B.7, `docs/spinup.md` |
