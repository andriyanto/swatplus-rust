## Bab 17  Validasi bit-for-bit dan katalog kebiasaan SWAT+

Bab ini menjelaskan bagaimana engine Rust dibuktikan sama dengan SWAT+ Fortran commit `6f51536`, dan mengumpulkan di satu tempat seluruh kebiasaan (*quirk*) SWAT+ yang sengaja dipertahankan oleh port, koreksi yang tersedia lewat `--fix-swat`, fitur yang ditolak oleh engine 1.0, serta kinerja dan batas paralelisme. Berbeda dari bab lain, bab ini tidak terutama disusun per fungsi engine, melainkan per aspek validasi; meski demikian, fungsi-fungsi alat pembanding (`crates/swatplus-rust/src/compare.rs`, `compare_cmd` di `main.rs`) dan skrip validasi (`scripts/*.py`, `scripts/*.sh`) tetap dijelaskan satu per satu.

File yang dicakup: `crates/swatplus-rust/src/compare.rs`, `crates/swatplus-rust/src/main.rs` (bagian `compare_cmd` dan pelaporan `run`), `scripts/make-variants.sh`, `scripts/make-phase3-variants.py`, `scripts/make-pessel-variants.py`, `scripts/make-reservoir-variants.py`, `scripts/print-daily.sh`, `scripts/compare-yields.py`, `scripts/check-parquet.py`, `scripts/wasm-run.mjs`, alur kerja `.github/workflows/validate.yml`, `ci.yml`, `fortran.yml`, `release.yml`, serta dokumen `docs/swat-quirks.md`, `docs/swat-fixes.md`, `docs/engine1.md`, `docs/routing.md`, `docs/subdaily.md`, `docs/reservoirs.md`, `docs/nutrients.md`, `docs/structures.md`, `docs/paddy.md`, `docs/atmo.md`, `docs/outputs.md`, `docs/phase2.md`, `docs/phase3.md` dan `docs/runner.md`. Pengecekan penolakan fitur diambil langsung dari kode (`swat-engine/src/hru/mod.rs`, `hru/mgt.rs`, `hru/wetland.rs`, `hru/septic.rs`, `hru/plant.rs`, `route.rs`, `channel.rs`, `res.rs`).

Posisi dalam alur: validasi berada di luar simulasi. Engine dijalankan dua kali pada folder TxtInOut yang sama (sekali oleh SWAT+ Fortran, sekali oleh `swatplus-rust run`), lalu setiap tabel keluaran dibandingkan kolom demi kolom. Katalog kebiasaan menjadi acuan silang bagi semua bab sebelumnya: setiap butir menunjukkan di mana port sengaja meniru perilaku SWAT+ yang mengejutkan, karena tanpa peniruan itu hasil tidak akan identik.

### 17.1  Prinsip validasi dan acuan Fortran

#### 17.1.1  Apa arti "bit-for-bit"

- **Tujuan proyek** (README, keputusan K12 desain RustGIS): setiap keluaran harian sama dengan SWAT+ Fortran commit `6f51536` pada skenario acuan, dalam toleransi selisih relatif 1e-4 *atau* satu satuan pada desimal terakhir yang dicetak.
- **Dalam praktik** hampir semua tabel identik tanpa memakai toleransi sama sekali. `docs/phase2.md` mencatat untuk tahap 2a: "Every column the port writes is identical to SWAT+'s on every day, with no tolerance used." Toleransi K12 berfungsi sebagai jaring pengaman untuk dua keadaan: (a) nilai yang dicetak dengan banyak digit (mis. tabel rata-rata tahunan unit routing dengan 9 digit bermakna) dan (b) perbedaan pustaka matematika antara platform (WebAssembly, macOS, Windows), lihat 17.4.3.
- **Presisi.** SWAT+ menghitung hampir semua besaran dalam `real` 32-bit; port memakai `f32` di tempat yang sama dan menjaga urutan operasi, karena penjumlahan floating point tidak asosiatif. Dua contoh di mana satu bit pun berarti (dari `docs/swat-quirks.md`, bagian *Arithmetic*): gfortran melipat `Sin(Atan(x))` menjadi `x / Sqrt(x*x + 1)` sehingga port harus melakukan hal yang sama (tanpa itu dua dari 1.858 HRU Batanghari menyimpang sampai 2 % biomassa), dan `x ** 0.5` di gfortran adalah `powf`, bukan `sqrt`, sehingga port memanggil `fortran::rpow` yang menyembunyikan eksponen dari optimizer.
- **Yang dibandingkan.** Semua tabel teks yang ditulis port: tabel HRU (`hru_wb`, `hru_pw`, `hru_nb`, `hru_ls`), unit routing (`ru`), akuifer (`aquifer`), kanal SWAT-DEG (`channel_sd`, termasuk `channel_sd_subday`), DAS (`basin_wb`, `basin_nb`, `basin_ls`, `basin_pw`, `basin_sd_cha`), unit lanskap (`lsunit_*`), waduk dan sumber titik (`reservoir`, `basin_res`, `recall`, `basin_psc`), untuk periode harian, bulanan, tahunan dan rata-rata tahunan, serta tabel hasil panen (`crop_yld_aa`, `basin_crop_yld_yr`, `basin_crop_yld_aa`).

#### 17.1.2  Program acuan SWAT+ Fortran (`fortran.yml`)

- **Lokasi:** `.github/workflows/fortran.yml` (alur kerja "SWAT+ Fortran build").
- **Tujuan:** membangun SWAT+ Fortran pada commit yang diikuti port (`SWATPLUS_REF: 6f515368525e7d84089ae0f9d33f4c9115de5679`) sebagai biner Linux statis dan menyimpannya di cabang `fortran-bin`, supaya alur kerja validasi dapat mengambilnya dengan `git` (artefak Actions tidak selalu bisa diunduh).
- **Cara kerja:**
  1. *Checkout* `swat-model/swatplus` pada ref tersebut.
  2. Pasang `gfortran` dan `ninja-build`; konfigurasi CMake `-DCMAKE_BUILD_TYPE=Release -DCMAKE_Fortran_COMPILER=gfortran -DTAG=6f51536`; bangun.
  3. Uji asap: salin `refdata/Osu_1hru` milik SWAT+, jalankan, pastikan `success.fin` ada.
  4. Terbitkan ke cabang `fortran-bin` (dipaksa, satu commit) berkas `swatplus-fortran` dan `README.txt` yang mencatat commit, versi kompiler, tanggal, sha256 biner, dan tautan run.
- **Pemicu:** manual (`workflow_dispatch`) atau saat berkas alur kerja itu sendiri berubah di `main`.
- **Catatan:** acuan dan port sama-sama dijalankan di Linux dengan glibc, sehingga fungsi matematika (`expf`, `logf`, `powf`, trigonometri) membulatkan sama. `docs/phase3.md` mencatat bahwa build ini mereproduksi persis run pemilik proyek dengan SWAT+ rev. 62.0.1 untuk Batanghari dan Pessel, sehingga keluaran keduanya sah sebagai acuan apa adanya.

#### 17.1.3  Toleransi K12

| Alat | Kriteria cocok | Catatan |
| --- | --- | --- |
| `swatplus-rust compare` | selisih mutlak d <= `abs` x (1 + 1e-6) **atau** selisih relatif d / max(abs(x), abs(y)) <= `rel`; bawaan `abs` = 0,001, `rel` = 1e-4 | `abs` 0,001 = satu satuan desimal ketiga, presisi cetak SWAT+. Kelonggaran 1e-6 karena 0,144 - 0,143 dalam biner sedikit di atas 0,001. |
| `scripts/compare-yields.py` | d <= unit x (1 + 1e-6) atau d <= 1e-4 x max(abs) | unit 0,001 untuk `crop_yld_aa`, 1e-6 untuk `basin_crop_yld_yr` dan `_aa`. |
| Uji WebAssembly (`ci.yml`) | byte demi byte; bila berbeda, harus lolos `compare` dengan toleransi bawaan | lihat 17.4.3. |
| `scripts/check-parquet.py` | Parquet = float 32-bit terdekat dari teks, persis; NaN = NaN | bukan toleransi numerik, melainkan uji konversi. |

Ketat: sebuah kolom dianggap gagal bila *satu* nilai pun melampaui kedua batas, dan sebuah tabel gagal bila ada baris SWAT+ yang hilang, baris ekstra milik port, atau tidak ada kolom yang dapat dibandingkan.

#### 17.1.4  Strategi bertahap: varian yang mengisolasi satu ketergantungan

Tidak ada skenario acuan yang mengisolasi neraca air (`docs/phase2.md`): Osu\_1hru adalah sawah (penyimpanan permukaan, tabel keputusan, Penman-Monteith), Ames\_sub1 menanam jagung-kedelai dengan irigasi, pupuk, olah tanah, model karbon `cswat` 2 dan PET Hargreaves. Karena itu validasi berjalan bertahap: dari proyek nyata dibuat varian yang menghilangkan satu ketergantungan pada satu waktu, SWAT+ Fortran menjalankannya seperti proyek lain, dan setiap tahap harus tepat sebelum tahap berikutnya dimulai. Setelah engine 1.0, pola yang sama dipakai terbalik: varian *menambahkan* satu fitur (sawah, waduk, langkah jam-jaman, kalibrasi bersyarat, dsb.) ke proyek yang sudah tepat (`Ps_coast`, `Bh_res`), sehingga setiap fitur baru diuji terhadap Fortran dalam konteks yang sudah terbukti. Semua angka ringkasan yang dikutip di bab ini berasal dari dokumen repo, bukan dari data uji.

### 17.2  Skenario uji

CI menjalankan 37 skenario: 4 proyek acuan (`testdata/`) dan 33 varian yang dibangun skrip, termasuk Bh\_nosed dan Ps\_edge yang ditambahkan di 1.0.1. Satu varian lagi (`Bh_urb`) dibuat oleh `make-phase3-variants.py` tetapi memang tidak lagi dijalankan di `validate.yml`, karena Batanghari sendiri memuat HRU urban dan PET yang sama; `docs/phase3.md` sudah dikoreksi di 1.0.1.

#### 17.2.1  Proyek acuan

| Skenario | Asal | Isi (dari docs repo) | Fitur yang diuji |
| --- | --- | --- | --- |
| `Osu_1hru` | contoh sawah milik SWAT+ sendiri (`refdata`) | satu HRU 10 ha di Korea, 2010 sampai 2023, 3 tahun pemanasan | sawah: bendung yang tingginya diundi (`prob_unif`), irigasi genangan, pelumpuran, tanam pindah dan panen padi lewat tabel keputusan, pupuk kandang (`manu`), kalibrasi keras 13 parameter, Penman-Monteith; juga kebiasaan pembacaan (`day_lag_mx` = `0.00000` menghentikan pembacaan `parameters.bsn` sehingga `igen` tetap 5; header curah hujan 42 tahun padahal berisi 43; seri radiasi dan angin yang berakhir lebih awal; baris ekstra `hru_cb` di `print.prt`; `recall.con` dengan hitungan 0) |
| `Ames_sub1` | contoh SWAT+ | 12 HRU, 1975 sampai 2020, jagung dan kedelai (`cosy_comm`), `mgt_01` dengan 305 operasi, tabel keputusan irigasi `irr_year_irr`, `soil_lyr_depths.sol`, `cswat` 2, PET Hargreaves | seluruh fase HRU dengan model karbon Century; kebiasaan pembacaan (baris `init` di `file.cio` berisi 12 nama untuk 11 slot, `ames.tmp` kosong, gage `ames.tem` tidak ditemukan oleh pencarian biner, `cn_froz` 0 menjadi 0,000862, `nutrients.sol` dan `chem_app.ops` tidak ada, nama pupuk di kolom yang salah) |
| `Batanghari` | proyek nyata Sumatra Barat (QSWAT+ dan SWAT+ Editor 4.1.0) | 850.722 ha, 42 subDAS, 284 unit routing dan kanal SWAT-DEG, 44 akuifer, 1.858 HRU (79 HRU lahan basah, 167 HRU perkotaan) | Penman-Monteith, HRU perkotaan, lahan basah dengan pelepasan `res_rel.dtl`, tabel keputusan tanam-panen generik, QUAL2E di kanal; di CI dipotong ke 2015 sampai 2019 |
| `Pessel` | proyek nyata Pesisir Selatan | 272.817 ha, 230 subDAS, 1.454 unit routing dan kanal, 254 akuifer, 3.966 HRU (414 lahan basah), 26 outlet | skala, gambut, padi, banyak lahan basah, outlet di pantai; di CI hanya 2015, dicetak sejak hari pertama |

Data masukan Batanghari dan Pessel bersifat privat; bab ini tidak memuat nilai masukan atau keluarannya. Kedua `README.md` proyek mencatat penyuntingan setelah Editor (mis. `hu_lai_decl` 20 untuk tanaman tahunan hijau abadi), yang dibahas dalam katalog kebiasaan (17.5, kelompok "Proyek nyata").

#### 17.2.2  Varian Ames (`scripts/make-variants.sh`, fase 2 dan 4)

Skrip Bash ini menyalin Ames\_sub1 dan hanya mengganti tata guna lahan HRU (fungsi `variant` menulis ulang kolom ke-6 `hru-data.hru` dengan lebar kolom tetap dan menghapus `soil_lyr_depths.sol`).

| Varian | Perubahan | Fitur yang diuji | Hasil (docs) |
| --- | --- | --- | --- |
| `Ames_bare` | tata guna `bare_lum` baru: tanpa komunitas tanaman, tanpa pengelolaan, CN tanah kosong bera | neraca air tanah murni: salju, evaporasi tanah, limpasan CN, perkolasi, aliran lateral, suhu tanah, pencampuran biologis (tahap 2a) | tepat: 28 kolom x 201.624 HRU-hari |
| `Ames_past0` | padang rumput `past_lum`, `codes.bsn` karbon 0 | pertumbuhan tanaman, kanopi, transpirasi, N dan P tanah, residu, erosi dengan model karbon asli (2b-1) | tepat: 77 kolom x 201.624 HRU-hari pada 4 tabel harian |
| `Ames_past` | padang rumput dengan `cswat` 2 | `cbn_zhang2` dan kolam Century, pencampuran biologis harian (2b-2) | tepat; karbon per lapisan (`hru_cbn_lyr`) dalam presisi cetak |
| `Ames_corn` | `corn_lum`: jagung saja, jadwal `mgt_corn` = operasi `mgt_01` tanpa tabel keputusan | tanam, panen-mati, olah tanah dengan model karbon, irigasi terjadwal, lewati tahun (M1); pupuk tidak teraplikasi karena kebiasaan kolom | tepat |
| `Ames_cornf` | `Ames_corn` dengan nama pupuk dipindah ke kolom pertama, `elem_n` disuntik (`inject`), sisanya ditabur, `05_10_15` diganti pupuk kandang sapi perah, `chem_app.ops` diambil dari Osu\_1hru | pupuk mineral dan organik (M1); menemukan kebiasaan `x ** 0.5` | tepat, 80 kolom |
| `Ames_cosy` | tata guna asli (`cosy_comm` di 11 HRU dengan `mgt_corn`; `corn_comm` di HRU ke-12) tanpa tabel keputusan | komunitas dua tanaman (M2) | tepat |
| `Ames_cosy2` | setiap tanam dan panen jagung diikuti kedelai pada hari yang sama | persaingan cahaya, air, hara dua tanaman yang tumbuh bersama | tepat |
| `Ames_dtbl` | Ames\_sub1 apa adanya dengan tabel keputusan, tanpa `soil_lyr_depths.sol` | `lum.dtl`, `conditions`, `cond_real`, aksi `irrigate` | tepat |
| `Ames_tile` | `cosy_lum` dengan drainase `mw24_1000` sedalam 1 m | drainase bawah permukaan (fase 4) | cocok, tetapi air tanah tetap di bawah kapasitas lapang di atas drainase sehingga tidak ada aliran drainase |

Ames\_sub1 sendiri (dengan lapisan khusus, 16 lapisan alih-alih 5) tepat pada keempat tabel HRU harian, 12 HRU, 1975 sampai 2020.

#### 17.2.3  Varian Batanghari (`make-phase3-variants.py`, `make-reservoir-variants.py`)

| Varian | Pembuat | Perubahan | Fitur yang diuji |
| --- | --- | --- | --- |
| `Bh_land` | `make-phase3-variants.py`, fungsi `variant(src, out, "Bh_land", "2", ("urbn_lum", "wetw_lum"))` | `codes.bsn` `pet` 2 (Hargreaves); HRU perkotaan dan lahan basah diberi `frse_lum` dan `surf_stor` `null` | fase lahan di data tropis (3a-0: 80 kolom, 6.107.246 HRU-hari, tepat), loop perintah, 284 unit routing, 44 akuifer (3a), seluruh jaringan 284 kanal SWAT-DEG dengan 59 kolom `channel_sd_day` (3b, 3c); di CI dipotong ke 2019 dengan `sed '3s/2025/2019/'` |
| `Bh_urb` | `variant(..., "Bh_urb", None, ("wetw_lum",))` | hanya lahan basah diganti | Penman-Monteith dan 167 HRU perkotaan (3d); tidak lagi dijalankan CI, karena Batanghari sendiri memuat HRU urban dan PET yang sama |
| `Bh_res`, Bh\_nosed | `make-reservoir-variants.py`, `bh_res` | Batanghari 2015 sampai 2017 dengan waduk hipotetis `res001` di hulu kanal cha037 (sekitar 47.000 ha hulu): kanal dan unit routing yang dulu mengalir ke cha037 dialihkan ke waduk, yang melepas ke cha037 lewat tabel `flood_season` milik Batanghari; sumber titik `mill` (limbah pabrik kelapa sawit, harian, ke cha060) dan `town` (air limbah kota, tahunan, ke cha100); Bh\_nosed (1.0.1, fungsi bh\_nosed) sama dengan Bh\_res tetapi waduknya menunjuk baris sediment.res yang tidak ada (nosed01) | `reservoir.con`, `reservoir.res`, `hydrology.res`, `res_rel.dtl`, `recall.con`, `recall.rec`; tabel `reservoir_*`, `basin_res_*`, `recall_*`, `basin_psc_*`; Bh\_nosed: faktor pengendapan 0 tanpa baris sedimen, semua tabel cocok |
| `Bh_rel` | `bh_rel` | `Bh_res` dengan waduk dilepas lewat `lake_multi`: di bawah pelimpah utama sebagai danau alami (`natlake`) ditambah skema Hanasaki (`nonirr-h06`) dan PLTA (`hydrop`) dan pelepasan minimum 2 m3/s; di antara pelimpah lewat `dyrt1` 30 hari ke volume banjir kondisi ketiga ditambah 1 % volume utama per hari; di atas pelimpah darurat lewat `inflo_targ` | opsi pelepasan baru `res_hydro` (engine 1.0); semua tabel, termasuk tabel harian waduk, cocok |

Ukuran waduk dan beban sumber titik `Bh_res` dicantumkan di `docs/reservoirs.md` sebagai nilai hipotetis yang tipikal (volume utama 30 juta m3 di 300 ha, darurat 50 juta m3 di 400 ha, mulai 80 % penuh; pabrik 540 m3 limbah per hari; kota 20.000 jiwa). Menurut dokumen itu waduk bertahan di 47 sampai 53 juta m3 dan menahan sekitar 99 % sedimen yang masuk.

#### 17.2.4  Varian Pessel (`scripts/make-pessel-variants.py`)

Semua varian Pessel diturunkan dari `Ps_coast`, kecuali `Ps_urbhr` dan `Ps_gap` yang diturunkan dari `Ps_hourly`. Fungsi pembangunnya bernama sama dengan variannya (`ps_paddy`, `ps_musk`, dst.); setiap fungsi menyalin folder dasar lalu menyunting beberapa berkas dengan `set_field` (mengganti medan ke-k sambil menjaga lebar kolom).

| Varian | Perubahan | Fitur yang diuji | Ringkasan hasil (docs) |
| --- | --- | --- | --- |
| `Ps_coast` | potongan dataran rendah pantai: 20 tangkapan kecil yang seluruh HRU-nya di bawah 20 m dan bermuara langsung ke laut; 74 kanal, 74 unit routing, 115 HRU (sawit, hutan rawa gambut, mangrove, lahan basah lain, padi, 6 HRU tanpa tata guna), 23 akuifer dangkal dan 20 dalam, semua dinomori ulang; 2015 sampai 2017 dicetak sejak hari pertama | skala kecil tetapi lengkap: gambut, lahan basah, padi, HRU `null` (mengambil baris pertama tabel), `varinit` yang mengenolkan laju puncak | tepat pada tujuh tabel harian |
| `Ps_paddy` | HRU padi menjadi sawah `paddy_lum`/`paddy_ps`: dua musim per tahun, bendung, irigasi genangan, pelumpuran `med_eff`, padi 120 hari, urea terbagi tiga, SP-36 | modul sawah SWAT+ pada data tropis | semua tabel dan hasil panen identik |
| `Ps_musk` | `codes.bsn` `rte_cha` 1 | routing Muskingum kanal SWAT-DEG | cocok |
| `Ps_atmo` | `atmo.cli` bulanan 2015 sampai 2017, dua stasiun (pedalaman, pantai) bergantian per stasiun cuaca; deposisi kering tiga kali lipat saat kabut asap September sampai Oktober 2015 | deposisi N atmosfer (`nut_nrain`) | cocok; bentuk tahunan dan rata-rata tahunan juga diperiksa |
| `Ps_tile` | sawit di atas drainase `peat_canal` 600 mm | drainase bawah permukaan di gambut | cocok |
| `Ps_bmp` | jalur penyaring riparian pada sawit, saluran berumput `grwway_low` pada tanaman baris, `bmpusr1` pada lahan pertanian generik | `smp_filter`, `smp_grass_wway`, `smp_bmpfixed` | cocok; sedimen DAS 0,52 menjadi 0,18 t/ha/tahun |
| `Ps_septic` | sistem `standard` beroperasi untuk 4 penghuni di bawah 12 HRU ladang kering | septik dan biozona | cocok; perkolasi HRU tersebut turun ke hampir nol |
| `Ps_solp` | `codes.bsn` `soil_p` 1 | model P tanah dinamis (Vadas dan White) | cocok |
| `Ps_urban` | lahan pertanian generik menjadi kota kepadatan sedang `urmd` dengan `urb_ro` `usgs_reg` | regresi beban USGS | cocok |
| `Ps_hourly` | `gampt` 1, `time.sim` 24 langkah; 37 berkas CHIRPS dipecah per jam sebagai badai Mononobe 4 jam mulai 15.00 | langkah sub-harian, Green-Ampt, erosi per langkah, `channel_sd_subday` | cocok, termasuk 1,9 juta debit kanal per jam |
| `Ps_land` | tiga HRU sawit hulu dialirkan antar-HRU (ke hutan rawa gambut `tot`, ke ladang kering `tot`, ke ladang kering `sur` dan `lat` terpisah) | routing lanskap HRU ke HRU | cocok |
| `Ps_urbhr` | `Ps_hourly` dengan kota `urmd` `buildup_washoff` | `hru_urbanhr` per jam | cocok |
| `Ps_pet` | `pet` 3 dan `pet.cli` konstan 4 mm/hari untuk semua stasiun | PET "read in" (yang ternyata tetap Hargreaves) | cocok |
| `Ps_cal`, Ps\_edge | 24 perubahan tambahan `PS_CAL` di `calibration.cal`, sebagian bersyarat (`hsg`, `landuse`, `plant`, `texture`) dan per lapisan; Ps\_edge (1.0.1): Ps\_coast dengan setiap lahan basah bernomor ganjil menunjuk baris sediment.res yang tidak ada dan surlag dikalibrasi dua kali (pctchg 50 lalu −20) | kalibrasi keras bersyarat untuk HRU, tanah, DAS, akuifer; Ps\_edge: lahan basah tanpa baris sedimen dan surlag bertumpuk | cocok |
| `Ps_harv` | panen tandan buah segar sawit tiap tanggal 15 (`ffb`, biomassa), panen biji, jerami, mati terpisah; umbi dan mati lewat tabel keputusan | semua jenis panen `harv.ops` | cocok, termasuk hasil panen |
| `Ps_peren` | `days_mat` 365 untuk sawit, populasi 136 pohon/ha dan 50.000 tanaman baris | hari ke kematangan tanaman tahunan dan populasi | cocok |
| `Ps_hu` | lahan generik dikelola dengan satuan panas (bajak 0,10, tanam 0,15, urea 0,30, panen-mati 1,20, lalu `skip`) | jadwal `hu_sch` dan olah tanah tanpa model karbon | cocok |
| `Ps_uh` | per jam dengan CN (`gampt` 0) dan hidrograf satuan gamma | `unit_hyd`, `flow_hyd_ru_hru` | cocok, termasuk 1,9 juta debit per jam |
| `Ps_gap` | `Ps_hourly` dengan jam-jam sejak 17.00 pada hari 40, 41, 300 tahun 2016 hilang (-99) di satu stasiun, dan satu stasiun lain dibangkitkan (`sim`) | `cli_pgenhr` | cocok |

Di luar itu `Pessel` utuh juga dijalankan (2015 saja; `print.prt` diubah ke `nyskip` 0).

### 17.3  Cara membandingkan dengan SWAT+ Fortran: alat dan fungsinya

Alur satu skenario: (1) `print-daily.sh` menyalakan cetak harian, bulanan, tahunan dan rata-rata tahunan untuk objek yang dibandingkan; (2) SWAT+ Fortran dijalankan di folder itu dan harus menghasilkan `success.fin`; (3) `swatplus-rust run <folder> --out <rust>` membaca folder yang sama (keluaran SWAT+ di dalamnya bukan masukan); (4) `swatplus-rust compare` dijalankan per tabel yang ditulis port; (5) `compare-yields.py` memeriksa tabel hasil panen.

Struktur data utama `compare.rs`:

| Nama | Jenis | Isi |
| --- | --- | --- |
| `Key` | alias `(i64, i64, i64, i64)` | tahun, hari, unit dan langkah waktu (0 di tabel non-subharian) |
| `Rows<R>` | struct | `names` (nama kolom, nama berulang diberi akhiran `#2`, `#3`, ...), `lines` (iterator baris), `cols` (indeks `yr`, `jday`, dan `unit` opsional), `tstep` (indeks kolom langkah), `last` (kunci terakhir untuk cek urutan) |
| `ColumnStats` | struct, `Default` | `values`, `mismatches`, `max_abs`, `max_rel`, `first` (uraian ketidakcocokan pertama) |

#### 17.3.1  `Rows::new`

- **Lokasi:** `crates/swatplus-rust/src/compare.rs`, `fn new(r: R) -> Result<Self, String>` (untuk `R: BufRead`).
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** membuka tabel keluaran SWAT+ (baris judul, baris nama kolom, baris satuan, lalu data) sebagai aliran.
- **Masukan dan keluaran:** pembaca berkas; `Rows` atau pesan galat bila tidak ada header atau tidak ada kolom `yr` dan `jday`.
- **Cara kerja:**
  1. Lewati baris judul; ambil baris kedua sebagai header.
  2. Untuk setiap nama, hitung berapa kali nama itu sudah muncul; kemunculan kedua dan seterusnya diberi akhiran `#k` (contoh: banyak kolom `null` untuk suhu tiap hidrograf di tabel kanal), sehingga kolom dicocokkan menurut urutan kemunculan.
  3. Cari indeks `yr`, `jday`, `unit` (opsional) dan `tstep` (opsional).
- **Catatan port:** kolom dicocokkan menurut nama, bukan posisi, sehingga port boleh menulis subset kolom selama tahap pengembangan.

#### 17.3.2  `Rows::next_row`

- **Lokasi:** `compare.rs`, `fn next_row(&mut self) -> Result<Option<(Key, Vec<String>)>, String>`.
- **Padanan SWAT+:** tidak ada.
- **Tujuan:** mengambil baris data berikutnya beserta kuncinya.
- **Cara kerja:**
  1. Pecah baris menurut spasi; baris yang `yr` atau `jday`-nya bukan bilangan bulat dilewati (baris satuan, baris kosong).
  2. Unit: dari kolom `unit` bila ada; bila tabel tidak punya kolom unit (`ru_day.txt`), baris dinomori berurutan dalam hari yang sama.
  3. Langkah: dari `tstep` bila ada, selain itu 0.
  4. Kunci harus naik ketat; bila tidak, galat "rows out of order". SWAT+ menulis hari demi hari dan unit demi unit, sehingga kedua berkas dapat dibaca serentak tanpa memuat seluruh tabel ke memori (tabel harian Batanghari mencapai gigabita).

#### 17.3.3  `compare`

- **Lokasi:** `compare.rs`, `pub fn compare(expected: impl BufRead, actual: impl BufRead, abs: f64, rel: f64, label: &str) -> Result<bool, String>`.
- **Padanan SWAT+:** tidak ada.
- **Tujuan:** membandingkan tabel SWAT+ (`expected`) dengan tabel port (`actual`) kolom demi kolom dan mencetak laporan.
- **Masukan dan keluaran:** dua aliran, toleransi `abs` dan `rel`, label; `true` bila semuanya cocok.
- **Cara kerja:**
  1. Kolom yang dibandingkan: semua kolom port kecuali kunci dan teks (`jday`, `mon`, `day`, `yr`, `unit`, `gis_id`, `name`, `type`, `tstep`) yang juga ada di tabel SWAT+.
  2. Gabung-urut (*merge*) kedua aliran menurut `Key`: kunci SWAT+ yang lebih kecil dihitung sebagai baris hilang, kunci port yang lebih kecil sebagai baris ekstra.
  3. Untuk baris dengan kunci sama, setiap nilai yang dapat diurai sebagai `f64` di kedua sisi dibandingkan:

```latex
d = |x - y|, \qquad r = \frac{d}{\max(|x|, |y|)} \ (r = 0 \text{ bila } d = 0)
```

```latex
\text{tidak cocok} \iff d > abs\,(1 + 10^{-6}) \ \wedge \ r > rel
```

4. Catat jumlah nilai, ketidakcocokan, `max_abs`, `max_rel`, dan uraian ketidakcocokan pertama (tahun, hari, unit, langkah, nilai SWAT+ dan port, nama kolom).
5. Cetak `label: N rows; columns compared: M`, lalu satu baris per kolom berstatus `ok` atau `MISMATCH`, dan jumlah baris hilang atau ekstra.
6. Hasil `true` hanya bila tidak ada baris hilang, tidak ada baris ekstra, ada minimal satu kolom, dan nol ketidakcocokan.

- **Catatan port:** nilai yang tidak dapat diurai (misalnya tanda bintang Fortran) dilewati, tidak dihitung. Uji unit `matches_and_counts_missing_rows` memeriksa bahwa 2,000 lawan 2,001 cocok, 2,000 lawan 2,003 tidak, dan baris yang hilang menggagalkan perbandingan.

#### 17.3.4  `compare_cmd`

- **Lokasi:** `crates/swatplus-rust/src/main.rs`, `fn compare_cmd(expected: &str, actual: &str, opts: &[&str]) -> ExitCode`.
- **Padanan SWAT+:** tidak ada.
- **Tujuan:** subperintah `swatplus-rust compare <tabel SWAT+> <tabel kita> [--abs 0.001] [--rel 1e-4]`.
- **Cara kerja:** urai opsi (nilai bawaan `abs` 0,001 dan `rel` 1e-4; opsi tak dikenal mencetak penggunaan dan keluar dengan kode 2); buka kedua berkas dengan `BufReader`; panggil `compare` dengan label nama berkas port; kode keluar 0 bila cocok, 1 bila tidak cocok atau galat.

#### 17.3.5  `scripts/compare-yields.py`: `rows`, `close`, `main`

- **Tujuan:** membandingkan tabel hasil panen yang tidak memiliki kunci `yr`/`jday` sehingga tidak dapat ditangani `compare`.
- **`rows(path, skip)`:** baris berisi setelah `skip` baris header, dipecah per spasi.
- **`close(a, b, unit)`:** benar bila d <= unit x (1 + 1e-6) atau d <= 1e-4 x max(abs(x), abs(y)).
- **`main()`:** untuk `crop_yld_aa.txt` (lewati 3 baris; kunci kolom 3 sampai 5; nilai kolom 6 sampai 9; unit 0,001), `basin_crop_yld_yr.txt` dan `basin_crop_yld_aa.txt` (lewati 2; kunci 0 sampai 2; nilai 3 sampai 5; unit 1e-6): bila berkas SWAT+ tidak ada, lewati; bila berkas port tidak ada tetapi SWAT+ punya baris, gagal (port menulis tabel saat baris pertama; tanpa tanaman tabel SWAT+ hanya berisi header). Jumlah baris harus sama; kunci harus sama persis; nilai harus `close`. Tiga ketidakcocokan pertama dicetak; kode keluar 1 bila ada yang gagal.

#### 17.3.6  `scripts/print-daily.sh`

- **Tujuan:** menyunting `print.prt` skenario untuk run validasi.
- **Masukan:** folder, daftar `ON` (dipisah koma), daftar `OFF` opsional.
- **Cara kerja:** dengan `awk`, setelah baris `objects`, setiap baris berisi 5 medan yang namanya ada di `ON` ditulis ulang menjadi `y y y y` (harian, bulanan, tahunan, rata-rata tahunan); yang ada di `OFF` mendapat `n` untuk harian dan mempertahankan tiga kolom lain. `\r` di akhir baris dibuang. Validasi mematikan `hru_cb` dan `hru_cb_vars` harian karena diagnostik karbon harian Ames\_sub1 sendiri mencapai 9 GB.

#### 17.3.7  `scripts/check-parquet.py`: `f32`, `text_rows`, `main`

- **Lokasi:** `scripts/check-parquet.py <folder keluaran>`; membutuhkan `duckdb` dan `pyarrow`.
- **Tujuan:** memeriksa bahwa setiap tabel Parquet dari `run --format both` dapat dibaca pyarrow dan DuckDB dan memuat isi tabel teks di sebelahnya, kolom demi kolom.
- **`f32(x)`:** membulatkan `float` Python ke float 32-bit terdekat lewat `struct.pack('<f')`.
- **`text_rows(path)`:** lewati baris judul; baris pertama yang diawali `jday` atau `year` menjadi nama kolom; baris berikutnya yang medan pertamanya bilangan bulat menjadi data (baris satuan terlewati).
- **`main(folder)`:**
  1. Untuk setiap `*.parquet`, baca dengan `pyarrow.parquet.read_table` dan hitung baris dengan DuckDB `select count(*) from read_parquet(...)`.
  2. Jumlah baris pyarrow, jumlah baris DuckDB dan jumlah kolom harus sama dengan tabel teks.
  3. Untuk setiap kolom dan baris: string harus sama persis; integer sama dengan `int(token)`; float harus sama persis dengan `f32(float(token))` (token `.E` diubah menjadi `.0E`), atau keduanya NaN (token yang tidak dapat diurai, mis. bintang Fortran, dianggap NaN).
  4. Cetak `N parquet tables, B bad`; kode keluar 1 bila ada yang buruk.
- **Kaitan dengan format:** sesuai `docs/runner.md`, kunci (`jday`, `mon`, `day`, `yr`, `unit`, `gis_id`, `tstep`, `year`, `plant_no`) disimpan sebagai integer 32-bit, nama (`name`, `type`, `plantnm`, `plant_name`) sebagai UTF-8, dan semua nilai sebagai float 32-bit yang sama dengan angka di tabel teks; NaN di tempat SWAT+ mencetak NaN atau bintang. Berkas tanpa kompresi, *plain encoding*, satu *row group* per sejuta baris.

#### 17.3.8  Fungsi pembangun varian

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `variant` | `make-variants.sh` | salin Ames\_sub1, ganti kolom tata guna di `hru-data.hru` dengan lebar kolom tetap, hapus `soil_lyr_depths.sol` |
| `rewrite`, `set_field`, `variant` | `make-phase3-variants.py` | tulis ulang berkas lewat fungsi; ganti medan ke-k dengan menjaga lebar; buat `Bh_land`/`Bh_urb` (PET di `codes.bsn`, tata guna dan `surf_stor` HRU tertentu), hapus `README.md` |
| `read`, `write` | `make-pessel-variants.py`, `make-reservoir-variants.py` | baca/tulis berkas sebagai daftar baris dengan akhir baris asli dipertahankan (`newline=""`) |
| `set_field` | idem | ganti medan ke-k dengan rata kanan pada lebar asli |
| `expand`, `compress` | `make-pessel-variants.py` | daftar elemen SWAT+ (angka diikuti negatif akhir rentang, mis. `1 -111`) ke daftar penuh dan sebaliknya |
| `Con.__init__`, `Con.write` | idem | baca berkas koneksi (`*.con`) beserta aliran keluar (tipe, id, hidrograf, fraksi); tulis baris yang dipertahankan dengan nomor baru, penunjuk data (medan 7) dan id tujuan yang dipetakan ulang |
| `ps_coast` | idem | potong Pessel: cari outlet setiap kanal, pilih kelompok kanal yang semua HRU-nya di bawah `MAX_ELEV` 20 m, ambil unit routing, elemen, HRU, akuifer dangkal dan dalamnya; pastikan potongan tertutup (asersi); potong dan nomori ulang `hru-data.hru`, `aquifer.aqu`, `channel-lte.cha`, `rout_unit.rtu`, berkas `*.con`, `wetland.wet` dan `hydrology.wet` (dipotong ke lahan basah yang dipertahankan dalam urutan HRU, karena kebiasaan `wet_hyd(ihyd)`), `rout_unit.ele/.def`, `ls_unit.ele/.def` (fraksi DAS dihitung ulang atas luas varian), `calibration.cal` (objek dipetakan ulang; asersi bahwa tidak ada kondisi), `object.cnt`, `file.cio` (`aqu_catunit.ele` menjadi `null`), `time.sim` (berakhir 2017), `print.prt` (`nyskip` 0) |
| `ps_paddy` dan `append` (lokal) | idem | lihat 17.2.4; juga menambah `weir.res`, `puddle.ops`, baris `irr.ops` `ponding_ps`, `tillage.til` `puddle`, tabel keputusan `PADDY_DTL`, dan mengaktifkan `weir.res` di `file.cio` |
| `ps_musk`, `ps_atmo`, `ps_tile`, `ps_bmp`, `ps_septic`, `ps_solp`, `ps_urban`, `ps_hourly`, `mononobe`, `ps_uh`, `ps_gap`, `ps_pet`, `ps_cal`, ps\_edge, `ps_harv`, `ps_peren`, `ps_hu`, `ps_urbhr`, `ps_land` | idem | lihat 17.2.4. `mononobe(depth, hours=4)` membagi hujan harian R ke jam ke-t menurut kedalaman kumulatif R (t/T)^(1/3), yaitu sekitar 63 %, 16 %, 12 % dan 9 % untuk T = 4 |
| `hyd`, `append`, `bh_res`, `bh_rel`, bh\_nosed | `make-reservoir-variants.py` | baris hidrograf untuk berkas recall; tambah baris ke berkas; bangun `Bh_res`, `Bh_rel` dan Bh\_nosed |

```latex
P_t = R\left[\left(\tfrac{t+1}{T}\right)^{1/3} - \left(\tfrac{t}{T}\right)^{1/3}\right], \quad t = 0,\dots,T-1
```

### 17.4  Alur kerja CI

#### 17.4.1  `validate.yml` ("Validate against SWAT+")

- **Pemicu:** setiap *push* ke `main`, setiap *pull request*, dan manual. `concurrency` membatalkan run lama pada ref yang sama. Satu job `validate` di `ubuntu-24.04`, batas waktu 60 menit.
- **Langkah:**
  1. *Checkout*; ambil biner acuan dengan `git fetch --depth 1 origin fortran-bin` lalu `git show origin/fortran-bin:swatplus-fortran`, cetak `README.txt`-nya (commit, kompiler, sha256), simpan path di `FORTRAN`.
  2. Pasang Rust stabil; `cargo build --release -p swatplus-rust`; simpan path di `RUST`.
  3. Bangun varian: `make-variants.sh testdata/Ames_sub1 variants`; `make-phase3-variants.py testdata/Batanghari variants`, lalu potong `time.sim` `Bh_land` dan Batanghari ke 2019 (tiga tahun tercetak setelah dua tahun pemanasan); `make-pessel-variants.py testdata/Pessel variants` (Ps\_coast sudah 2015 sampai 2017); `make-reservoir-variants.py testdata/Batanghari variants`; potong Pessel utuh ke 2015 dan `nyskip` 0.
  4. Untuk setiap skenario dalam daftar tetap (Osu\_1hru, Ames\_sub1, Ames\_bare, Ames\_past0, Ames\_past, Ames\_corn, Ames\_cornf, Ames\_cosy, Ames\_cosy2, Ames\_dtbl, Ames\_tile, Bh\_land, Batanghari, Bh\_res, Bh\_rel, Ps\_coast, Ps\_paddy, Ps\_musk, Ps\_atmo, Ps\_tile, Ps\_bmp, Ps\_septic, Ps\_solp, Ps\_urban, Ps\_hourly, Ps\_land, Ps\_urbhr, Ps\_pet, Ps\_cal, Ps\_harv, Ps\_peren, Ps\_hu, Ps\_uh, Ps\_gap, Ps\_edge, Bh\_nosed, Pessel): salin folder sumber ke `runs/<s>/fortran`; jalankan `print-daily.sh` dengan 20 objek ON (`hru_wb`, `hru_pw`, `hru_nb`, `hru_ls`, `ru`, `aquifer`, `channel_sd`, empat `basin_*`, empat `lsunit_*`, `basin_sd_cha`, `reservoir`, `basin_res`, `recall`, `basin_psc`) dan `hru_cb`, `hru_cb_vars` OFF.
  5. Jalankan SWAT+ Fortran di folder itu; wajib ada `success.fin`.
  6. Jalankan port pada folder yang sama ke `runs/<s>/rust`; bila gagal, anotasi `::error` berisi tiga baris terakhir log dan job berhenti.
  7. Untuk setiap kombinasi 20 *stem* x 5 periode (`day`, `mon`, `yr`, `aa`, `subday`) yang ditulis port: `compare`; hasil cocok menjadi anotasi `::notice` (jumlah kolom `ok` dan jumlah baris), tidak cocok menjadi `::error` dengan tiga baris `MISMATCH`/`missing`/`not in` pertama, dan status job menjadi 1. Tabel yang tidak ditulis port dilewati (port menulis tabel nutrien dan periode hanya untuk HRU yang disimulasikan penuh).
  8. `compare-yields.py`; kegagalan juga menyetel status 1.
  9. Hapus `*_day.txt` dan `*_subday.txt` kedua sisi untuk menghemat ruang disk.
  10. Keluar dengan status gabungan; `trap ERR` menandai skenario tempat skrip berhenti.
- **Catatan:** varian dibangun ulang pada setiap run dari `testdata/`, sehingga berkas varian tidak perlu disimpan di repo. Karena semua tabel selain harian tetap dibandingkan, agregasi bulanan, tahunan dan rata-rata tahunan (`docs/outputs.md`) ikut tervalidasi.

#### 17.4.2  `ci.yml`: format, lint, uji, inspeksi

| Job | Isi |
| --- | --- |
| `test` ("Format, lint, test") | `RUSTFLAGS=-D warnings`; `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings` (setiap diagnostik juga dijadikan anotasi); `cargo test --workspace` (56 fungsi `#[test]` di repo, termasuk `swat-io/tests/refdata.rs` yang memeriksa isi Osu\_1hru dan Ames\_sub1 seperti `igen` 5, header 42/43 tahun, baris `init` 12 nama, `ames.tmp` kosong, tekstur `L- L- L- L`, `cn_froz`); `inspect` pada Osu\_1hru dan Ames\_sub1 |
| `wasm` ("WebAssembly build") | bangun `swat-wasm` untuk `wasm32-unknown-unknown`; jalankan Osu\_1hru lewat `scripts/wasm-run.mjs` di Node 22 dengan flag 3 (teks dan Parquet); `run.json` harus `"ok":true`; bandingkan setiap tabel teks dengan run native (17.4.3) |
| `parquet` | Python 3.12 dengan `duckdb` dan `pyarrow`; Osu\_1hru dengan 12 objek dicetak harian, `run --format both --progress json`; `check-parquet.py` |
| `msrv` | `cargo build --workspace` dengan Rust 1.90 (versi minimum) |

#### 17.4.3  WebAssembly lawan native

Job `wasm` membandingkan berkas byte demi byte (`cmp -s`). Tabel yang berbeda diberi peringatan (`::warning`) dan harus lolos `compare` dengan toleransi bawaan; job gagal hanya bila ada tabel di luar toleransi. Penyebab perbedaan yang dicatat `docs/runner.md`: di WebAssembly fungsi matematika (`exp`, `ln`, `powf`, trigonometri) adalah milik Rust sendiri (port dari musl), bukan glibc, dan beberapa membulatkan bit terakhir hasil presisi tunggal secara berbeda. Pada Osu\_1hru selama 14 tahun, 17 dari 18 tabel identik; tabel rata-rata tahunan unit routing (dicetak 9 digit) berbeda pada dua nilai sebesar 1 dan 2 ULP float 32-bit. Program native di macOS dan Windows dapat berbeda dengan cara yang sama karena pustaka matematikanya bukan glibc.

`scripts/wasm-run.mjs` menunjukkan antarmuka modul: `swat_alloc(len)` untuk penyangga masukan, salin arsip tar skenario, `swat_run(ptr, len, flags)` (1 Parquet, 2 teks, 4 `--fix-swat`), baca 4 byte panjang *little-endian* lalu arsip tar keluaran (tabel dan `run.json`), `swat_free` kedua penyangga. Modul mengimpor `env.swat_progress(done, days)`.

#### 17.4.4  `release.yml`

Saat tag `v*` didorong: bangun `swatplus-rust` (`--locked`) untuk `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`, kemas bersama `LICENSE`, `NOTICE`, `README.md` dan `docs/runner.md`, serta modul WebAssembly, lalu terbitkan sebagai rilis GitHub. Rilis tidak menjalankan validasi ulang; validasi bergantung pada `validate.yml` di `main`.

#### 17.4.5  Cara mengulang secara lokal

Menurut `docs/phase2.md`: bangun varian dengan `make-variants.sh`, jalankan `print-daily.sh` pada folder varian, jalankan `swatplus-fortran` di folder itu, lalu `cargo run -p swatplus-rust -- run <folder> --out <rust>` dan `cargo run -p swatplus-rust -- compare <folder>/hru_wb_day.txt <rust>/hru_wb_day.txt`. Untuk melacak penyimpangan halus, dokumen yang sama menyebut pembandingan air tanah tiap lapisan dari keluaran `object.prt` SWAT+ (`sol_water`) dan keadaan yang ditulis SWAT+ pada setiap operasi (`mgt_out.txt`); cara inilah yang menemukan kebiasaan `x ** 0.5`.

### 17.5  Katalog kebiasaan SWAT+ yang dipertahankan

Katalog ini menggabungkan semua butir di `docs/swat-quirks.md` serta bagian *Quirks* di `docs/engine1.md`, `routing.md`, `subdaily.md`, `reservoirs.md`, `nutrients.md`, `structures.md`, `paddy.md`, `atmo.md` dan `outputs.md`. Nomor berlanjut di seluruh kelompok. Kolom "Lokasi Fortran" menyebut subrutin SWAT+ (berkas `.f90` bernama sama di `/root/swatplus/src`); kolom terakhir menyatakan apakah `--fix-swat` mengoreksinya (rincian di 17.6). Semua butir lain direproduksi juga dalam mode `--fix-swat`, karena koreksi hanya menyentuh butir yang tercantum di `docs/swat-fixes.md`. Contoh angka di kolom "Dampak" diambil dari dokumen repo.

#### 17.5.1  Pembacaan berkas masukan

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 1 | Masukan *list-directed* `read (unit, *)`: pembacaan berlanjut ke baris berikut bila nilai kurang; sisa nilai di baris terakhir diabaikan; baris kosong dilewati; `/` mengakhiri; `,,` = nilai null yang tidak mengubah variabel; teks tanpa kutip berakhir di spasi, koma atau garis miring | semua pembaca (`read (unit,*)`); port `swat-io/src/list.rs` | tata letak berkas bebas; berkas yang "rusak" tetap terbaca dengan cara yang sama seperti SWAT+ | Tidak |
| 2 | Integer menolak desimal: gfortran menolak `1.0` untuk variabel integer | `list::parse_int` | lihat butir 10 | Tidak |
| 3 | `file.cio` dibaca menurut posisi: nama di awal baris diabaikan; nama ekstra dilewati (baris `init` Ames\_sub1 punya 12 nama untuk 11 slot); baris setelah akhir berkas memakai nama bawaan; `null` mematikan slot | `readcio_read`; `swat-io/src/cio.rs` | berkas yang salah nama baris tetap terbaca | Tidak |
| 4 | `time.sim`: `step` <= 0 berarti harian; `day_start` <= 0 berarti 1 Januari; `day_end` 0 berarti 31 Desember tahun terakhir; `day_end` melebihi panjang tahun dipangkas | `time_read`, `time_control`; `swat-model/src/time.rs` | periode simulasi | Tidak |
| 5 | `print.prt`: jumlah interval rata-rata tahunan dibaca, lalu rekamannya dibaca ulang beserta tahun-tahunnya (`backspace`); jendela cetak dilengkapi (hari pertama 0 menjadi 1, hari terakhir 0 menjadi 366, tahun pertama 0 menjadi tahun simulasi pertama, tahun terakhir 0 menjadi satu setelah tahun terakhir); penghitung interval dimulai pada nilai interval | `basin_print_codes_read`; `swat-io/src/print.rs`, `swat-engine/src/run.rs` | baris mana yang dicetak | Tidak |
| 6 | Nilai yang gagal dikonversi mengakhiri pembacaan diam-diam (SWAT+ hanya memeriksa `iostat < 0`). `day_lag_mx` = `0.00000` di `parameters.bsn` Osu\_1hru menghentikan pembacaan sehingga `igen` tetap 5 (Ames\_sub1 menulis `0 0`, jadi `igen` 0) | `basin_read_prm`; `swat-io/src/basin.rs` | mengubah benih bilangan acak generator cuaca | Tidak |
| 7 | Tabel dihitung dulu (satu nilai per rekaman, baris kosong tidak dihitung) lalu dibaca sebanyak itu sebagai *derived type* utuh; baris pendek mengambil nilai dari baris berikut. `hru-data.hru` diukur dari nomor HRU terbesar dan setiap baris ditaruh di nomornya | pembaca database; `swat-io/src/table.rs`, `hru.rs` | penempatan baris | Tidak |
| 8 | Baris berhenti pada nilai yang tidak dapat dikonversi: variabel sesudahnya tetap bawaan dan pembacaan berikutnya mulai di baris baru. `plants.plt` Batanghari (Editor 4.1.0) berisi 53 nilai dan deskripsi per baris untuk 56 variabel `plant_db`: deskripsi gagal sebagai variabel ke-54, tiga fraksi lignin tetap bawaan | `plant_parm_read`; `list::Records::read_checked` | tanpa ini setiap tanaman setelah yang pertama bergeser | Tidak |
| 9 | Pencarian menurut nama linear, mengambil kecocokan pertama, dibandingkan sebagai string Fortran (spasi di belakang diabaikan); nama tak ditemukan memberi penunjuk 0 | semua pencarian nama; `swat-model/src/table.rs` | lihat butir 13, 26, 27 | Tidak |
| 10 | Tekstur tanah satu kata (`L- L- L- L` dibaca sampai spasi pertama). Lapisan pertama lebih tipis dari 20 mm menjadi 20 mm bila tanah satu lapis atau lapis kedua lebih dalam dari 20 mm | `soil_db_read`; `swat-io/src/hru.rs` | kedalaman lapisan | Tidak |
| 11 | Parameter DAS di bawah 1e-6 diganti bawaan oleh `basin_prm_default`, yang untuk beberapa variabel bukan nilai awalnya (`prf` 1 bukan 484, `cmn` 0,0003 bukan 0,003). `cn_froz` 0 di Ames\_sub1 menjadi 0,000862 | `basin_prm_default`; `BasinParams::complete` | limpasan tanah beku, dsb. | Tidak |
| 12 | Berkas koneksi dibaca dua kali per baris, kedua kalinya dengan jumlah aliran keluar sebanyak `out_tot` | `hyd_read_connect`; `swat-io/src/con.rs` |  | Tidak |
| 13 | Nama data di `hru-data.hru` yang tidak ditemukan meninggalkan penunjuk 1 (bawaan `hru_databases`): HRU dengan tata guna `null` dari Editor mendapat tata guna pertama `landuse.lum` beserta tanaman, pengelolaan dan CN-nya; begitu pula topografi, hidrologi, salju, data tanah-tanaman | `hru_read`; `Row::find_or_first` | HRU tanpa tata guna tetap bertanaman | Tidak |
| 14 | `object.cnt` menentukan berapa objek tiap jenis dibaca: berkas koneksi dengan hitungan 0 (`recall.con` Osu) tidak dibaca, dan hanya n baris pertama yang dibaca | `basin_read_objs` |  | Tidak |
| 15 | `print.prt` tanpa label objek (`dbout` `n`) dibaca baris demi baris dalam urutan tetap SWAT+ apa pun nama barisnya: baris ekstra (`hru_cb` di Osu) menggeser bendera semua baris sesudahnya; Osu mencetak `channel_sd` dengan bendera baris `channel` | `basin_print_codes_read` | tabel mana yang dicetak | Tidak |

#### 17.5.2  Iklim

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 16 | Berkas stasiun kosong atau tidak ada menghentikan variabelnya: pembaca terukur keluar dari loop stasiun pada berkas pertama yang berakhir di header, jadi berkas sesudahnya tidak dibaca; berkas yang tidak ada dibuat kosong oleh gfortran. `ames.tmp` kosong: tidak ada suhu terukur | `cli_pmeas`, `cli_tmeas`, dst.; `swat-io/src/project.rs`, `LoadReport::unread` | variabel itu dibangkitkan | Tidak |
| 17 | Berkas stasiun dicari dengan pencarian biner (`search`), andal hanya pada daftar terurut. Gage `ames.tem` tidak ada dalam `["ames.tmp"]` sehingga gage 0: selalu di luar batas, suhu dibangkitkan setiap hari | `cli_staread`, `search`; `swat-engine/src/fortran.rs::search` |  | Tidak |
| 18 | Nama tak ditemukan jatuh ke yang pertama: generator tak ditemukan memakai generator 1; stasiun tak ditemukan memakai stasiun 1 (`wgn_01` lawan `wgn_sta1` di Ames) | `cli_staread`, `hru_read` |  | Tidak |
| 19 | Gage bernama `null` tidak sama dengan `sim`: hanya `sim` mengambil jalur generator; nama lain yang tak ditemukan melewati jalur terukur dengan gage kosong, berakhir di generator tetapi menarik lebih banyak bilangan acak (suhu memanggil `cli_weatgn` dua kali) | `cli_precip_control`, `cli_tmeas` | urutan bilangan acak | Tidak |
| 20 | Header bisa salah (curah hujan Osu: 42 tahun tertulis, 43 isi); SWAT+ mengukur larik dari header tetapi mengisinya mulai tahun simulasi pertama | `cli_pmeas` | tidak berbahaya di Osu | Tidak |
| 21 | Seri pendek: hari di luar batas berkas ditandai -98 untuk generator (`cli_bounds_check`); radiasi dan angin Osu berakhir 2022-304 | `cli_bounds_check`; `swat-engine/src/climate.rs` | sisa periode dibangkitkan | Tidak |
| 22 | Radiasi bangkitan diganti rata-rata bulanan oleh baris terakhir `cli_slrgen` | `cli_slrgen` | radiasi bangkitan tanpa variasi harian | Tidak |
| 23 | Hujan diputuskan sehari lebih awal: hujan hari berikut dibaca/dibangkitkan sebelum `cli_clgen` memperbarui status basah/kering, sehingga peluang hujan hari bangkitan mengikuti status dua hari sebelumnya | `cli_precip_control`, `cli_clgen` | rantai Markov bergeser satu hari | Tidak |
| 24 | Tabel bulan global: `cli_initwgn` mengambil panjang bulan dari `ndays` global yang terakhir disetel pembaca suhu (tahun kabisat bila habis dibagi 4) atau tabel kabisat awal; loop simulasi lalu memakai aturan Gregorian penuh per tahun | `cli_initwgn`, `time_control` | faktor koreksi generator | Tidak |
| 25 | Benih bergantung `igen`: bila bukan 0, `gcycl` menggeser benih (generator pertama saja) dan mengacak benih tiap tarikan (`idg`); `cli_initwgn` memakai salinan benih hujan untuk 1000 faktor koreksinya | `gcycl`, `cli_initwgn` | urutan acak | Tidak |
| 26 | Nilai hilang ditulis -99 di berkas | pembaca iklim |  | Tidak |

#### 17.5.3  Neraca air HRU

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 27 | Lapisan atas 10 mm ditambahkan ke setiap tanah yang lapis pertamanya lebih dalam dari 19,5 mm (tanpa `soil_lyr_depths.sol`); lapis j mengambil lapis j - 1 berkas | `soils_init`; `hru.rs::soil_layers` | jumlah lapisan | Tidak |
| 28 | Suhu tanah awal = maksimum Desember dirata-ratakan dengan dirinya `(tmpmx(12) + tmpmx(12)) / 2`, semua lapisan | `hydro_init` |  | Tidak |
| 29 | Air tanah awal dari `sw_init` (`ffcb`) `parameters.bsn`, atau dari hujan tahunan bila 0; `sw_frac` di `soil_plant.ini` dibaca tetapi tidak dipakai | `soils_init`, `hydro_init` |  | Tidak |
| 30 | Nama tabel CN tak ditemukan menunjuk elemen 0 (bawaan 30, 55, 70, 77); kelompok hidrologi selain A sampai D memberi CN 0 | `cn2_init` |  | Tidak |
| 31 | n aliran permukaan bawaan 0,05 bila nama `ovn` tidak ada di `ovn_table.lum` (bawaan tabel sendiri 0,5) | `plant_init` | waktu konsentrasi | Tidak |
| 32 | Suhu salju tanpa memori: `sq_snom` menyetel suhu salju 0 setiap panggilan sebelum lag, sehingga selalu `tave * timp` | `sq_snom` | pencairan salju | Tidak |
| 33 | Perkolasi berjalan setiap hari walau tanpa hujan; lapisan beku (suhu <= 0) menahan air; air di atas jenuh naik lapis demi lapis, kelebihan lapis atas menjadi limpasan | `swr_percmain`, `swr_satexcess` |  | Tidak |
| 34 | Batas evaporasi tanah: tiap lapisan memberi paling banyak 80 % airnya per hari (`etco`); lapisan tersisa < 1e-6 mm dibiarkan 1e-6 | `et_act` |  | Tidak |
| 35 | Pencampuran biologis memindahkan air (dengan `cswat` 2, `mgt_biomix` harian): massa tiap lapisan memakai berat isi lapis pertama; lapisan yang sebagian di atas kedalaman campur memakai suhu lapisan di atasnya; total air tanah tidak dijumlah ulang | `mgt_biomix` | tanpa ini evaporasi menyimpang sampai 0,003 mm/hari | Tidak |
| 36 | `sw_init` hanya bergerak pada hari yang dicetak: dengan interval cetak > 1, `sw_init` dan `sw_ave` mencakup seluruh interval; tahun yang dilewati (`nyskip`) tidak mencetak | `hru_output`, `basin_sw_init` | kolom neraca | Tidak |
| 37 | Dua nilai untuk satu nama: `es_max` diubah sebelum `ep_max` dihitung darinya bila PET membatasi keduanya | `et_act` | transpirasi | Tidak |

#### 17.5.4  Tanaman

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 38 | Nama tanaman tak ada di `plants.plt` mengambil tanaman pertama (penunjuk bawaan 1); komunitas yang tidak ada di `plant.ini` berarti tanpa tanaman | `readpcom` |  | Tidak |
| 39 | Indeks panen > 0,7 menjadi 0,02 (padang rumput Ames 0,9) | `plantparm_init` | hasil | Tidak |
| 40 | Akar mulai dari `zmx` `soils.sol`, bukan dasar lapisan terakhir yang disetel kemudian oleh `hydro_init` | `plant_init` |  | Tidak |
| 41 | Dormansi pertama kali diputuskan untuk hari 0 (`hydro_init` sebelum jam disetel) | `hydro_init` |  | Tidak |
| 42 | Permintaan hara usang: `pl_nut_demand` mengenolkan elemen setelah tanaman terakhir (sisa penghitung loop), bukan tiap tanaman; larik milik modul, sehingga tanaman yang bangun di hari itu tumbuh dengan permintaan HRU lain dan stresnya diukur terhadap optimum HRU itu | `pl_nut_demand`, `pl_dormant`; `hru/plant.rs::Saved` | membawa keadaan antar-HRU (lihat 17.8) | Ya (`pl_nupd`, `pl_pupd`) |
| 43 | Stres suhu selalu sinus: `if (rto > 0 .or. rto < 2)` selalu benar, jadi strst = sin(1,5707 (tave - tbase)/(topt - tbase)), dibatasi 0 sampai 1 | `pl_tstr` | pertumbuhan | Tidak |
| 44 | Serapan air melewati `epco` di lapis kedua (`k == 2`, bukan 1) | `pl_waterup` |  | Tidak |
| 45 | Tanaman tahunan yang tumbuh menurut suhu menurunkan LAI tanpa menjatuhkan daun (`leaf_tov` hanya untuk tanaman menurut kelembapan); daun yang jatuh saat dormansi tidak membawa karbon | `pl_leaf_senes`, `pl_dormant` | residu | Tidak |
| 46 | Indeks panas memakai cuaca objek j (`ob(j)%wst`, j nomor HRU); karena HRU pertama di daftar objek, itu stasiun HRU itu sendiri | `pl_grow` |  | Tidak |
| 47 | Satuan panas dasar direset akhir tahun di belahan utara dan pada hari 181 di belahan selatan; proyek Indonesia mereset di Juli | `time_control` | jadwal satuan panas | Tidak |

#### 17.5.5  Nutrien dan erosi (model karbon asli, `cswat` 0)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 48 | Lapis atas mineralisasi dengan suhu dan air lapis kedua (`kk = 2` untuk `k = 1`) | `nut_nminrl` | N mineral | Tidak |
| 49 | `rsd_nitorg_n` dan `rsd_laborg_p` dicetak 0: dijumlah oleh `rsd_decomp`, lalu dinolkan `nut_nminrl` | `rsd_decomp`, `nut_nminrl` | kolom keluaran | Tidak |
| 50 | Pencampuran tanah tiap akhir tahun: `mgt_newtillmix_cswat0` dengan `biomix` hidrologi pada hari terakhir; lapisan di atas 50 mm bertukar air, N, P, humus; sebagian residu permukaan masuk | `time_control`; `hru/nutrient.rs::year_end_mixing` |  | Tidak |
| 51 | Metode faktor C selalu 1: `bsn_cc%cfac = 1` disetel sebelum diuji | `ero_cfactor` | C dari residu dan biomassa | Tidak |
| 52 | Uji salju pada sedimen terbalik: dengan salju hanya hasil < 1e-6 yang dinolkan; tanpa salju dibagi `exp(0)` | `ero_ysed` | erosi di bawah salju | Ya |
| 53 | Simpanan sedimen dikosongkan dua kali: sedimen yang dilepas diambil dari lag store tepat setelah dihitung dan lagi bersama konstituen lain; simpanan menjadi negatif | `stor_surfstor` | pelepasan hari berikut lebih kecil | Ya (`sq_sed`; per langkah tetap sekali) |
| 54 | P mineral pada sedimen tertukar: bagian aktif dihitung dari ukuran kolam stabil dan sebaliknya | `nut_psed` | P sedimen | Ya (`psed`) |
| 55 | `nutrients.sol` tidak ada (Ames menyebutnya) berarti bawaan SWAT+: nitrat 7 ppm dan P labil 5 ppm di permukaan, menurun dengan kedalaman | `soil_nutcarb_init` |  | Tidak |

#### 17.5.6  Model karbon (`cswat` 2)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 56 | N termineralisasi tidak pernah masuk kolam mineral: `cbn_zhang2` memindahkan N antar litter, mikroba, humus dan mengurangkan bagian termineralisasi dari kolam sumber, tetapi tidak menambahkannya ke nitrat atau amonium; N mineral hanya berubah lewat denitrifikasi dan transfer nitrat ke amonium saat surplus | `cbn_zhang2`, `nut_np_flow` | ketersediaan N | Tidak |
| 57 | `nut_np_flow` membuang N yang dilepas bersama CO2: menghitung `EfromCO2` tetapi menambahkan variabel lain yang selalu 0 (`efco2`) | `nut_np_flow` |  | Tidak |
| 58 | Rasio N:C mikroba lapis atas mengabaikan N residu > 2 %: `else` sesudahnya menimpanya dengan 0,05 | `cbn_zhang2` |  | Tidak |
| 59 | Dekomposisi lapis atas memakai kedalaman lapis kedua untuk faktor oksigen; lapisan di bawah yang pertama menyetel alokasi bersama `org_allo(2)%abco2` dari pasirnya sendiri | `cbn_zhang2` |  | Tidak |
| 60 | N organik dan karbon hanya keluar pada hari bersedimen: `nut_orgnc2` (juga pemindah karbon mikroba ke bawah) berjalan hanya bila ada limpasan, puncak aliran dan hujan; `c_microb_sed` disimpan dari perhitungan terakhir (HRU mana pun) dan diambil lagi; N residu yang dibuang milik tanaman 1; total residu DAS tidak diperbarui | `nut_orgnc2` | membawa keadaan antar-HRU | Tidak |
| 61 | Pencampuran biologis mencampur tekstur dan kolam setiap hari, tetapi tidak `nonlig` litter struktural; residu permukaan dihitung sekali per lapisan tercampur; efisiensi `mgt_tillfactor` (`idc_till` 3) setelah dikurangi di lapisan tempat kedalaman campur berakhir tetap 0 untuk lapisan di bawahnya | `mgt_biomix`, `mgt_tillfactor` |  | Tidak |

#### 17.5.7  Komunitas tanaman

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 62 | Air dibagi menurut urutan komunitas: tiap tanaman mengambil bagiannya dari transpirasi maksimum (menurut LAI) dari sisa tanaman sebelumnya | `pl_community`, `pl_waterup` |  | Tidak |
| 63 | N dan P terserap adalah total berjalan HRU (`nplnt`, `pplnt`, `fixn`): serapan tanaman kedua dimulai dari total tanaman pertama dan menambahkannya lagi; begitu pula N fiksasi untuk non-legum setelah legum | `pl_nup`, `pl_pup`, `pl_nfix` |  | Tidak |
| 64 | Pertumbuhan daun dibagi menurut tinggi kanopi, tetapi jumlahnya memakai LAI tanaman yang tumbuh dengan tinggi semua tanaman: bagian = tinggi sendiri / jumlah tinggi | `pl_leaf_gro` |  | Tidak |
| 65 | Stres garam yang dicetak dijumlahkan atas tanaman tumbuh; stres lain dirata-ratakan | `pl_grow` | kolom keluaran | Tidak |
| 66 | Sedimen mengambil N residu permukaan tanaman 1 sekali per tanaman, bukan tanaman lain | `nut_orgnc2` |  | Tidak |

#### 17.5.8  Tabel keputusan

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 67 | Dijalankan paling awal di hari itu, sebelum stres direset: stres air, LAI dan satuan panas yang diuji adalah milik kemarin | `hru_control`, `conditions` |  | Tidak |
| 68 | Irigasi tahunan tidak pernah direset (`hru%irr_yr`; reset dikomentari): "irigasi tahun ini < 500 mm" berlaku sampai 500 mm sepanjang run (Ames mencapainya tahun 1976) | `time_control` | irigasi berhenti | Ya |
| 69 | Satuan panas yang diuji milik tanaman pertama yang punya satuan panas (atau tanaman 1); `phu_mat` kondisi tidak dipakai | `cond_real` |  | Tidak |
| 70 | Hitungan aksi mulai 1 dan diulang 1 tiap tahun, sehingga `const2` 20 mengizinkan 20 aplikasi per tahun (`const2` minimal 1); dari dua tabel bernama sama dipakai yang terakhir | `actions`, `proc_cond` |  | Tidak |
| 71 | Tabel tanam-panen generik (`pl_hv_summer1`, `pl_hv_winter1`, `pl_hv_summer2`): `management.sch` menyebut tanaman setelah nama tabel, dan `plant`/`harvest_kill` bertindak atas tanaman itu (tahun rotasi untuk `pl_hv_summer2`) | `actions` | tanaman Batanghari | Tidak |
| 72 | Tanam lewat tabel tidak mereset tanaman (tanpa `mgt_plantop`); `plant` tanpa batas tahunan; `harvest_kill` memanen dan mematikan tanaman bernama walau tidak tumbuh dan tidak menyentuh satuan panas dasar nol | `actions` |  | Tidak |
| 73 | `phu_base0` adalah satuan panas dasar nol stasiun cuaca, direset tiap tahun; `soil_water` membandingkan air profil dengan FC, porositas atau 0 dikombinasikan dengan batas (`*`, `+`, `-`, `/`); `jday` dan `year_rot` integer; tahun rotasi mulai `rot_yr_ini` dan maju tiap akhir tahun di setiap HRU bertabel | `cond_real`, `time_control` |  | Tidak |

#### 17.5.9  Lapisan tanah khusus (`soil_lyr_depths.sol`)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 74 | Profil dipotong menjadi irisan 1 mm menurut milimeter bulat (kedalaman dipotong ke integer), masing-masing dengan sifat lapisan `soils.sol`-nya; lapisan baru merata-ratakan irisannya satu per satu dalam presisi tunggal (berat isi seragam 1,26 keluar sedikit berbeda) | `soils_init`; `custom_layers` di `hru/mod.rs` | sifat lapisan | Tidak |
| 75 | Kedalaman <= 10 mm dilewati; lapisan atas 10 mm selalu pertama; kedalaman pertama di atau melewati dasar profil menjadi dasar; bila daftar berakhir sebelum dasar, profil berakhir di situ. Berkas dibaca apa pun isi `file.cio`, dengan nama itu | `soils_init` | jumlah lapisan (Ames: 16 bukan 5) | Tidak |

#### 17.5.10  Pengelolaan terjadwal

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 76 | Operasi berjalan pada bulan dan tanggalnya, dalam urutan jadwal, setelah ET dan sebelum limpasan, hanya sampai tahun dilewati. `skip` (bulan 0) tidak pernah cocok tanggal: jadwal menunggu sampai akhir tahun ketika `time_control` melangkahinya; di akhir daftar mulai lagi | `mgt_operatn`, `mgt_sched`, `time_control` |  | Tidak |
| 77 | Operasi pertama satu sebelum yang dimaksud: `plant_init` mencari operasi pertama tahun rotasi awal pada atau setelah hari mulai dan menyimpan indeks dikurangi satu (minimal 1); HRU berjadwal tanpa tanaman tetap 1 | `plant_init` |  | Tidak |
| 78 | Operasi pupuk menyebut pupuk dulu lalu metode aplikasi (`chem_app.ops`); `mgt_01` Ames terbalik (`null elem_n`) sehingga pupuknya elemen 0 `fertilizer.frt` (semua nol): tidak ada yang diaplikasikan. Metode tak ditemukan (Ames tanpa `chem_app.ops`) tidak punya bagian permukaan: semua ke lapis kedua. Dengan model karbon asli, P organik menyetel P humus aktif dua lapis teratas = P humus stabil + P pupuk | `mgt_sched`, `pl_fert` | Ames tanpa pupuk | Tidak |
| 79 | Panen-mati (`hvkl`, biji): biji = hasil x efisiensi panen; yang tertinggal masuk humus aktif lapis atas (`hact`) yang tidak dibaca model karbon; kematian memasukkan sisa di atas tanah ke residu permukaan dan akar ke residu lapisan; tanaman dikeluarkan dari total komunitas yang sepanjang sisa hari (limpasan, erosi) masih menghitung biji yang dipanen; stres, satuan panas dan satuan panas dasar HRU direset untuk setiap tanaman tumbuh, apa pun yang disebut operasi | `mgt_harvgrain`, `mgt_killop` |  | Tidak |
| 80 | Tanam membatasi kedalaman akar tanah untuk selamanya: `zmx` = min(`zmx`, akar maksimum tanaman) | `mgt_plantop` |  | Tidak |
| 81 | Satuan panas tanaman semusim (`days_mat` > 0): tanam dianggap hari satuan panas dasar 0 mencapai 15 % tahunan; kematangan menjumlah `days_mat` hari dari situ, hari setelah 31 Desember memakai suhu Desember (`xmon` membiarkan bulan) | `plant_init` | PHU | Tidak |
| 82 | Olah tanah dengan model karbon: lapisan di atas kedalaman olah memberi bagian tanah (liat, debu, pasir, batu), air, N dan P mineral, kolam ke campuran yang dibagikan kembali menurut ketebalan; massa memakai berat isi lapis pertama; residu permukaan masuk sekali per lapisan tetapi keluar lapis demi lapis sebagai bagian dari sisa: lebih sedikit keluar daripada masuk | `mgt_newtillmix_cswat1` | neraca residu | Tidak |
| 83 | Efek olah pada dekomposisi (`idc_till` 3) di lapisan tempat kedalaman berakhir dibobot dengan bagian lapisan *di bawah* kedalaman; selama `till_eff_days` faktor meluruh setiap kali `mgt_tillfactor` berjalan (setiap hari pencampuran biologis); pencampuran berhenti pada hari olah dan pulih `bio_consf` per hari | `mgt_tillfactor` |  | Tidak |

#### 17.5.11  Proyek nyata: tropis dan SWAT+ Editor

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 84 | Tanaman tahunan tidak pernah dorman di bawah 20 derajat lintang: `daylth` -1 jam bila abs(lat) < 20 dan `dorm_hr` 0, sehingga `phuacc` tanaman `temp_gro` tidak pernah kembali 0; setelah melewati `dlai` (`hu_lai_decl` 0,99) LAI turun ke `lai_min` dalam 15 hari dan tetap; ET jatuh ke sekitar separuh PET. Editor membatasi `hu_lai_decl` <= 1; kasus uji menyetelnya 20 | `cli_initwgn`, `pl_dormant`, `pl_leaf_senes` | kanopi hijau abadi hilang tanpa penyuntingan | Tidak |
| 85 | `perco` fungsi curam: perco\_lim = exp(-(1,0052 ln(-ln(perco - 1e-6)) + 5,6862)), maks 1 (0,05 memberi sekitar 0,001; 0,5 sekitar 0,005; 0,8 sekitar 0,015; 0,95 sekitar 0,067). HRU berdrainase dipaksa `perco` 0,1 dan `cn3_swf` 0,95 | `topohyd_init` | drainase dasar profil | Tidak |
| 86 | Editor menulis `perco` menurut kelompok hidrologi (0,5 untuk C, 0,05 untuk D) dan `cn3_swf` 0,95: perkolasi hampir tertahan, tanah mendekati jenuh, CN tinggi | Editor (data) |  | Tidak |
| 87 | `dp_tot` `soils.sol` bukan kedalaman akar: `soil%zmx` = dasar lapis terbawah | `hydro_init` |  | Tidak |
| 88 | Histosol HWSD dengan subsoil kosong: lapis kedua Od19 `awc` 0 dan `soil_k` 0 (lapisan kedap 30 cm); Pessel menggantinya | data Editor |  | Tidak |
| 89 | Daftar objek `calibration.cal`: angka sebelum daftar adalah jumlah *entri*, bukan objek; entri negatif menutup rentang (`1 -111 113 115 -1123`); Editor hanya mendaftar HRU bertata guna | `cal_parmchg_read`, `define_unit_elements` |  | Tidak |
| 90 | QSWAT+ mengubah `WATR` menjadi `WETW` (HRU lahan basah); sel tanpa tata guna yang dikenal mendapat `lu_mgt` `null` | QSWAT+ (data) | lihat butir 13 | Tidak |
| 91 | HRU lahan basah menguap sedikit: di Pessel sekitar 375 mm/tahun lawan PET sekitar 1.300 mm; hampir semua hujan keluar sebagai `wet_oflo` | `wetland_control` | direproduksi apa adanya | Tidak |

#### 17.5.12  Unit routing dan akuifer

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 92 | Objek dijalankan dalam lintasan atas larik objek (HRU, unit routing, akuifer, kanal); objek berjalan setelah semua pengirimnya (dan semua elemennya); aliran masuk dijumlah menurut urutan pengirim | `command`, `hyd_connect`; `route.rs` | urutan penjumlahan f32 | Tidak |
| 93 | Jumlah hidrograf mencampur suhu menurut debit (`hydout_add`); skala dan `**` mempertahankan suhu hidrograf pertama; air HRU bersuhu 5 + 0,75 tave, recharge dan aliran lateral 0 | `hydout_add`, `hru_hyds` | suhu air | Tidak |
| 94 | Rasio penghantaran `0` (tulisan Editor) dihitung: akar (tc HRU / tc unit routing), maks 1, untuk semua kecuali debit; hidrograf recharge tanpa rasio; nama tak ada di `delratio.del` memberi rasio 0 | `dr_ru` | beban sedimen dan hara | Tidak |
| 95 | `ru_day` menjumlah kelima hidrograf (total, recharge, permukaan, lateral, drainase), sehingga total terhitung dua kali | `ru_control` | debit unit routing berlipat | Ya |
| 96 | Kelas sedimen tidak pernah nol: lag store dimulai 1e-6 setiap hari, sehingga HRU melepas 1e-6 x faktor lag pasir, debu, liat dan dua agregat setiap hari | `stor_surfstor` |  | Tidak |
| 97 | Rasio pengayaan dibagi semua HRU: `varinit` mengenolkan variabel lokal bernama sama, sehingga `swr_subwq` memakai rasio terakhir untuk HRU mana pun (CBOD) | `varinit`, `swr_subwq` | membawa keadaan antar-HRU | Ya (`hru_hyds`) |
| 98 | Simpanan awal akuifer memakai muka air akuifer yang bernomor sama dengan baris databasenya (`aqu_d(iaqdb)`), 0 bila akuifer itu datang kemudian | `aqu_initial` | sama bila satu baris per akuifer | Ya |
| 99 | Label `aquifer_day` bergeser satu kolom: header `orgn`, `orgp` padahal nilai karbon (konstan) dan N organik; aliran balik ke akuifer lain tercatat `flo_ls` | `aqu_hdr` | label kolom | Ya (label `cbn orgn`) |
| 100 | Revap memakai PET Hargreaves stasiun cuaca, apa pun metode PET HRU | `aqu_1d_control` |  | Tidak |

#### 17.5.13  Penman-Monteith dan HRU perkotaan

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 101 | Penman-Monteith menaikkan angin HRU ke 0,01 m/s, juga di cuaca yang dicetak (`wndspd` `hru_pw_day`) | `et_pot` |  | Tidak |
| 102 | Transpirasi maksimum yang dihitungnya ditimpa `et_act` dengan bagian LAI dari PET, kecuali bila PET habis setelah kanopi | `et_act` |  | Tidak |
| 103 | Resistansi kanopi memakai konduktansi stomata tanaman terakhir (`gsi_adj` setelah loop), bukan rata-rata tertimbang LAI `gsi_wav` | `et_pot` |  | Ya |
| 104 | CO2 tanaman acuan = `co2` DAS (sama tiap tahun tanpa `co2_yr.dat`) | `et_pot` |  | Tidak |
| 105 | Limpasan kedap memakai CN 98, bukan `urb_cn`, dan hanya pada hari dengan hujan efektif > 0,1 mm | `sq_daycn` |  | Tidak |
| 106 | Debu perkotaan dalam ton, bukan t/ha: `hru_urban` mengalikan padatan dengan luas HRU untuk kelas debu, bukan untuk sedimen | `hru_urban` |  | Tidak |
| 107 | Selain `usgs_reg` dan `buildup_washoff`, nilai `urb_ro` lain tidak menambah beban (catatan lama di `swat-quirks.md` menyebut `usgs_reg` belum diport; kini sudah, lihat butir 190) | `hru_urban` |  | Tidak |

#### 17.5.14  HRU lahan basah

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 108 | Koefisien evaporasi air terbuka diambil dari `wet_hyd(ihyd)`: data lahan basah HRU yang bernomor sama dengan baris `hydrology.wet`-nya; HRU tanpa lahan basah memakai bawaan 0,7 | `et_act` | karena itu `Ps_coast` memotong `wetland.wet` dan `hydrology.wet` | Ya |
| 109 | CN HRU lahan basah tidak pernah dihitung (`surface` tidak dipanggil): `hru_wb_day` mencetak 0 | `hru_control` |  | Tidak |
| 110 | N organik dan P sedimen adalah yang dilepas: `nut_orgn`/`nut_psed` butuh laju puncak yang dinolkan `varinit` dan hanya disetel `surface`; `nut_nlch` dan `nut_solp` berjalan dan mengganti nitrat dan P larut dengan milik tanah | `varinit`, `nut_orgn`, `nut_psed` |  | Tidak |
| 111 | Lahan basah kosong yang melimpas menghitung sedimennya dua kali | `wetland_control` |  | Tidak |
| 112 | Cek rembesan pelepasan bekerja pada salinan air tanah: membatasi rembesan pada daya tampung profil, tetapi tanah baru terisi saat perkolasi berikutnya | `wetland_control`, `swr_percmain` |  | Tidak |

#### 17.5.15  Kanal SWAT-DEG

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 113 | Luas kanal kurva debit = b d + d/z, bukan b d + z d^2; dataran banjir memakai d^2/fps | `sd_rating_curve` | tampungan dan rembesan | Ya (trapesium dan keliling basah) |
| 114 | Setiap hari debit penuh tebing disetel 1,5 kali kurva, genangan dataran banjir 1 hari, `d50` dan eksponen dasar 1; tanpa `sed_nut.cha` bawaan tipe berlaku | `sd_channel_sediment3` |  | Tidak |
| 115 | Kualitas air mengabaikan routing: `ch_watqual4` memakai waktu tempuh titik terdangkal kurva (`ht3%flo` tak pernah disetel) dan massa air *masuk*; `qual2e` 1 mengambil kembali N dan P organik dari aliran masuk | `ch_watqual4` | hara kanal | Ya (waktu tempuh pada debit masuk) |
| 116 | Suhu sungai `ch_temp` hanya mencapai suhu tampungan: hidrograf keluar disetel sebelum dihitung; "hari sebelumnya" = suhu aliran keluar; rata-rata suhu udara 2 sampai 250 hari | `sd_channel_control3`, `ch_temp` | suhu keluar = suhu masuk | Ya |
| 117 | Kolom terakhir `channel_sd_day` sama untuk semua kanal: `wtemp` stasiun kanal terakhir yang dirouting hari itu | `channel_sd_day` |  | Ya |
| 118 | Laju QUAL2E dan lebar/kedalaman untuk `ch_watqual4` dan `ch_temp` diambil dari baris data bernomor sama dengan kanal (`sd_dat(ich)`, `sd_chd(ich)`), bukan penunjuk `channel-lte.cha` | `ch_watqual4`, `ch_temp` | sama bila satu baris per kanal | Tidak |
| 119 | Sebelum ada aliran, kurva keluar terakhir mempertahankan waktu tempuh titik terdangkal (`rcurv_interp_dep` di kedalaman 0), yang menentukan koefisien tampungan hari pertama | `sd_hydsed_init` |  | Tidak |

#### 17.5.16  Kalibrasi keras (`calibration.cal`)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 120 | Diterapkan sekali setelah HRU disiapkan (`proc_cal` ke `cal_conditions`); `perco` baru menghitung ulang `perco_lim` dengan ambang 1e-6 (`hydro_init` 1e-9) | `cal_conditions` |  | Tidak |
| 121 | Daftar objek: jumlah entri lalu entri; negatif menutup rentang; 0 berarti semua objek jenisnya; perubahan dibatasi rentang `cal_parms.cal` | `define_unit_elements` |  | Tidak |
| 122 | Kalibrasi juga mencapai CN (`curno` lagi), koefisien PET dan aliran lateral, `epco` (juga tiap tanaman), `esco`, `canmx`, `lat_len`, AWC (`soil_awc_init` lagi, mengembalikan air tanah ke bagian awal FC), `surlag` (faktor lag HRU), `alpha` dan `revap_co` akuifer | `cal_conditions` |  | Tidak |
| 123 | Kondisi diuji pada HRU bernomor sama dengan objek, apa pun jenis parameternya | `cal_conditions` |  | Tidak |
| 124 | `plant` hanya melihat tanaman pertama komunitas; HRU tanpa tanaman memenuhinya | `cal_conditions` |  | Tidak |
| 125 | `landuse` berhenti pada kondisi pertama yang gagal; kondisi lain semuanya diuji | `cal_conditions` |  | Tidak |
| 126 | Parameter tanah berubah sekali per lapisan, tetapi `anion_excl` milik tanah: `pctchg`/`relchg` dimajemukkan sebanyak lapisan yang diubah | `cal_conditions` |  | Tidak |
| 127 | `lat_ttime` diubah setelah menjadi 1 - exp(-1/t), lalu diubah lagi ke bentuk itu | `cal_conditions` |  | Tidak |
| 128 | Perubahan datang setelah penyiapan: waktu konsentrasi dan LS USLE mempertahankan lereng lama; nutrien tanah disiapkan setelah kalibrasi (`soil_nutcarb_init` setelah `proc_cal`) dari tanah terkalibrasi | `proc_cal`, `soil_nutcarb_init` |  | Tidak |

#### 17.5.17  Sawah (Osu\_1hru, Ps\_paddy)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 129 | Bendung dari baris 1 `weir.res` (`wet_ob%iweir` mulai 1; kolom `WEIR` `wetland.wet` tidak dibaca); tingginya menetapkan volume utama (luas x h x 10^4 m3) dan volume darurat 1,2 kali; aksi `weir_height` mengubah keduanya, penunjuk berkasnya diabaikan | `wet_initial`, `actions` | satu bendung untuk semua lahan basah | Tidak |
| 130 | Pelepasan di atas bendung: kelebihan di atas volume darurat melimpas, lalu 24 langkah jam (luas x 0,45) c w h^k x 3600 m3, luas dalam ha; tanpa bendung (atau tinggi 0 terendam) tidak ada pelepasan; tabel pelepasan sawah tidak dipakai | `res_weir_release` |  | Tidak |
| 131 | Sawah datar: luas rembesan seluruh HRU, bukan kurva volume-luas | `wetland_control` |  | Tidak |
| 132 | Bilangan acak `prob_unif` dari `rndseed_cond`, disetel `gcycl` ke benih teracak bila `igen` bukan 0 (748932582 bila 0); peluang = kolom *objek* kondisi; jendela = nomor objek dan batas; hari terundi satu setelah int(r x hari) dari hari pertama | `cond_real`, `gcycl` | membawa keadaan antar-HRU | Tidak |
| 133 | Irigasi genangan (`irrigate` bernama `ponding`): hingga `const` mm di atas genangan, paling banyak Int(`const2`) kali setahun, `const2` juga kedalaman ambang | `actions` |  | Tidak |
| 134 | Pelumpuran memakai baris `puddle.ops` bernama opsi (yang terakhir; baris pertama berkas dianggap judul) dan olah tanah bernama *aksi*. Di bawah air (`mgt_newtillmix_wet`) hara genangan dibuang (jumlah dinolkan setelah ditambah); genangan mendapat nitrat, amonium, P mineral aktif (sebagai P larut), N dan P organik total tanah sebanding kedalamannya; tekstur dicampur dengan bagian lapisan terakhir (sisa variabel loop) | `actions`, `mgt_newtillmix_wet` | hara genangan | Ya (hara genangan ikut tercampur) |
| 135 | Pupuk ke air (`pl_fert_wet`) hilang dengan model karbon asli: hanya total yang mencatatnya | `pl_fert_wet` | Ps\_paddy: hasil gabah 4,11 sampai 4,60 menjadi 4,12 sampai 4,73 t/ha dengan koreksi | Ya |
| 136 | Pupuk kandang (`manu`): `pl_manure` membaca `manure_om.frt` pada baris bernomor sama dengan baris `manure_db.frt`, bukan `org_min`; air pupuk masuk tanah | `pl_manure` |  | Tidak |
| 137 | `puddle.ops`, `manure_db.frt`, `manure_om.frt`, `transplant.ops` dibuka dengan nama itu apa pun `file.cio` (Osu memuat `transplant.plt`, jadi tanpa tanam pindah) | pembaca |  | Tidak |
| 138 | Laju pengendapan `nutrients.res` per tahun: `res_read_nut` membagi enam laju dengan 365 | `res_read_nut` |  | Tidak |
| 139 | Hari sejak tanam, panen, mati dan irigasi mulai -366 dan naik setelah tabel keputusan tiap hari | `hru_control` | kondisi `days_*` | Tidak |

#### 17.5.18  Aritmetika

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 140 | gfortran melipat `Sin(Atan(x))` menjadi x / Sqrt(x x + 1); `sin_sl` di `plant_init` dibulatkan berbeda dari `sinf(atanf(x))` untuk sekitar dua dari lima lereng (diperiksa pada 7.562 lereng Batanghari dan Pessel) | `plant_init` (kompiler) | satu bit faktor USLE menggeser sedimen, N organik, dan pertumbuhan; dua HRU Batanghari menyimpang sampai 2 % biomassa sebelum diperbaiki | Tidak (bukan bug) |
| 141 | `x ** 0.5` adalah `powf`, bukan `sqrt`; optimizer Rust mengubah `powf(0.5)` menjadi `sqrt` yang dibulatkan benar (beda bit terakhir sekitar 1 dari 1.600 nilai); port memanggil `fortran::rpow` dengan eksponen tersembunyi | PET Hargreaves (`et_pot`), kompiler | satu bit PET menggeser air tanah dan digit cetak beberapa minggu kemudian | Tidak (bukan bug) |

#### 17.5.19  Fitur engine 1.0 (`docs/engine1.md`)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 142 | PET "read in" (`pet` 3): PET terukur `pet.cli` tidak pernah dipakai, karena `climate_control` kemudian menghitung PET Hargreaves setiap stasiun untuk indeks kelembapan iklim ke variabel yang sama; jadi `pet` 3 = Hargreaves pada suhu stasiun (tanpa koreksi lapse), minimal 0,01 mm/hari | `climate_control`, `cli_petmeas`, `et_pot` | Ps\_pet: PET DAS 1.252 mm/tahun (Hargreaves), bukan 1.461 mm berkas | Tidak |
| 143 | Danau alami (`natlake`) di bawah volume inaktifnya melepas NaN (bilangan negatif dipangkatkan 1,5), dan begitu pula semua hilir sejak itu | `res_hydro` | NaN merambat | Tidak |
| 144 | `nonirr-h06` dan `hydrop` tidak melepas apa pun: rata-rata aliran masuk lima tahun per bulan di `res_control` tidak pernah terisi (hari bulan, bendera akhir bulan dan aliran masuk hari itu tak disetel), jadi 0; faktor musiman `hydrop` juga bukan sinus: 1 + a (2 pi day + b)/365 | `res_hydro`, `res_control` |  | Tidak |
| 145 | `irr-h06` membagi rata-rata kebutuhan (0) dengan dirinya: NaN; port menolaknya | `res_hydro` | ditolak (17.7) | Tidak |
| 146 | Setiap aksi pelepasan diambil sekali per langkah waktu (`inflo_targ` melepas aliran masuk harian `step` kali) | `res_hydro` | sub-harian | Tidak |
| 147 | Hasil dihitung walau tidak ada yang dipanen: di bawah biomassa minimum, dan setelah panen residu, hasil tanaman ditambah hasil panen terakhir HRU; panen biomassa yang akan menyisakan terlalu sedikit menghitung hasilnya tanpa mengambilnya | `mgt_harvbiomass`, `mgt_harvresidue` | tabel hasil | Tidak |
| 148 | `harvest` tabel keputusan tidak menghitung panen: menambah hasil tetapi tidak jumlah panen (menambah 2 ke jumlah tahunan), sehingga `crop_yld_aa` adalah total; hasil DAS mendapat luas dengan hasil 0 | `actions` | `crop_yld_aa` | Tidak |
| 149 | Panen saja: `harv` jadwal hanya mencapai tanaman tumbuh, `harvest` tabel keputusan semua tanaman bernama itu; `kill` memulai ulang satuan panas setiap tanaman (tabel keputusan juga satuan panas dasar 0 HRU) | `mgt_sched`, `actions` |  | Tidak |
| 150 | Panen residu yang menyisakan kurang dari biomassa minimum mengembalikan residu (sampai minimum), dan NaN bila residu nol | `mgt_harvresidue` |  | Tidak |
| 151 | Hari ke kematangan tanaman tahunan-panjang (`days_mat` > 0): hari tanamnya tidak dihitung; `plant_init` menyimpannya antarpanggilan, jadi milik tanaman semusim terakhir yang dihitung (HRU demi HRU, tanaman demi tanaman), atau hari 0 | `plant_init` | Ps\_peren: LAI sawit 4,9 menjadi 1,1, ET DAS 1.253 menjadi 657 mm | Tidak |
| 152 | Populasi `plnt_pop` di `plant.ini` dibaca tetapi tidak diberikan ke tanaman (`pop_com` tetap 0): LAI potensial selalu `blai` | `plant_init` | tidak berpengaruh | Tidak |
| 153 | Operasi jadwal menurut satuan panas selalu memakai satuan panas tanaman pertama komunitas: `mgt_sched` mencari tanaman `op2` hanya pada salinan operasinya yang diganti operasi berikut, jadi uji melihat `op2` 0 | `mgt_operatn`, `mgt_sched` |  | Tidak |
| 154 | `skip` setelah operasi menurut satuan panas langsung berjalan, melewati sisa tahun | `mgt_operatn` |  | Tidak |
| 219 | `surlag` dikalibrasi bertumpuk: setiap perubahan dimulai dari hasil perubahan sebelumnya (dua `pctchg` 50 lalu −20 membuat 4 hari menjadi 4,8), dan masing-masing menghitung ulang faktor lag semua HRU | `cal_conditions` (`bsn_prm%surlag`) | Ps\_edge (port mengikuti sejak 1.0.1) | Tidak |

#### 17.5.20  Routing lanskap HRU ke HRU (`docs/routing.md`)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 155 | Runon drainase dibagi luas dua kali: `rls_routesurf` menambahkan `hin_til%flo / (10 area)` padahal `command` sudah mengubahnya ke mm | `rls_routesurf`, `command` |  | Tidak |
| 156 | HRU lahan basah menerima runon ke genangannya per hektar: jumlah per ha (mm, kg/ha) ditambahkan ke genangan dalam m3 dan kg | `rls_routesurf`, `wetland_control` | Ps\_land: rawa menerima sekitar 1.600 mm/tahun dari sawit di atasnya tetapi hanya melepas 2 mm lebih | Tidak |
| 157 | Runon yang sama juga meresap ke tanah lahan basah: `ht1` ditambahkan ke infiltrasi `swr_percmain` (dibagi lagi 10 x luas), P larutnya ke P labil lapis atas (`nut_solp`, kg/ha) | `swr_percmain`, `nut_solp` |  | Tidak |
| 158 | Air dari objek lain ke HRU, atau ke unit routing, ditolak port (SWAT+ menambahkannya ke runon permukaan HRU, atau kehilangannya di unit routing) | `command`, `ru_control` | ditolak (17.7) | Tidak |
| 159 | HRU dihitung dalam urutan perintah (hulu sebelum hilir) tetapi tabelnya ditulis menurut nomor; aliran masuk diubah per hektar HRU penerima; `tot` dipecah ke bagian permukaan, lateral, drainase; `rhg` ke HRU diabaikan; runon permukaan ke HRU kering menjadi hujan efektif (sedimennya lolos sampai kapasitas angkut dep\_co x usle\_cfac x q x slope^1,4 x lebar lahan^1,4); runon lateral masuk tanah sebanding ketebalan lapisan | `command`, `hru_control`, `rls_routesurf`, `rls_routesoil` | Ps\_land: limpasan ladang 274 menjadi 339 atau 784 mm/tahun | Tidak |

#### 17.5.21  Langkah sub-harian (`docs/subdaily.md`)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 160 | Langkah suatu hari diambil dari kolom tahun yang sedang disimulasikan (`tss(:, 1, time%yrs)`): pada hari terakhir tahun, langkah hari berikut dibaca dari kolom tahun ini, jadi 1 Januari tahun kedua dan seterusnya mendapat jam 1 Januari tahun sebelumnya; langkah mengisi hari dalam urutan berkas apa pun kolom jamnya | `cli_pmeas`, `cli_precip_control` |  | Tidak |
| 161 | Kanopi mengambil air dari langkah dua kali dan tidak dari hujan harian: langkah dikurangi saat kanopi terisi, lalu lagi oleh seluruh yang diambilnya; infiltrasi `inflpcp = precip_eff - surfq` tetap menghitung hujan harian penuh | `sq_canopyint`, `sq_greenampt` |  | Tidak |
| 162 | Limpasan yang ditambahkan setelah langkah permukaan (kelebihan jenuh) tidak pernah masuk hidrograf: `qday` = limpasan langkah yang dilag, disetel sebelum perkolasi | `sq_surfst`, `swr_satexcess` |  | Tidak |
| 163 | Pelepasan lahan basah atau sawah disebar rata ke langkah, dan `res_hydro` mengambil setiap aksi sekali per langkah: laju tetap, bagian volume utama, aliran masuk atau volume di atas pelimpah darurat dilepas `step` kali sehari; penurunan muka n hari diambil dalam `step` bagian (dan suku pertama `dyrt` `step` kali); sama untuk waduk | `res_hydro`, `wetland_control` |  | Tidak |
| 164 | Sedimen harian MUSLE masih menentukan hara, tetapi yang keluar adalah erosi langkah; simpanan sedimen harian mengambil hasil MUSLE sekali per langkah dan mengembalikan total langkah sekali | `stor_surfstor`, `ero_ovrsed` |  | Tidak |
| 165 | Hidrograf unit routing = milik HRU terakhirnya ditambah limpasan per langkah HRU itu dalam m3/s (setiap elemen HRU menggantinya), digeser waktu konsentrasi tetap 2 jam; aliran lateral, drainase, akuifer, waduk dan outlet disebar rata | `ru_control` |  | Tidak |
| 166 | Laju aliran masuk kanal untuk kurva debit = volume langkah dibagi sehari (`/ 86400`), sedangkan laju keluar dibagi langkah; `inflo` dipertahankan dari kanal terakhir bila kanal tanpa aliran masuk | `sd_channel_control3`, `ch_rtmusk` | membawa keadaan antar-kanal | Tidak |
| 167 | Laju QUAL2E dibagi jumlah langkah per hari (`ch_read_nut`), tetapi kualitas air tetap dihitung sekali sehari: alga, N dan oksigen berubah `step` kali lebih lambat | `ch_read_nut`, `ch_watqual4` |  | Tidak |
| 168 | Per langkah, padatan tercuci apa pun `urb_ro` (juga `usgs_reg`); setiap pencucian mengganti lagi bagian kedap hara harian di atas langkah sebelumnya; sedimen tidak diganti; nitrat (`tno3`) integer, jadi hanya kg/ha bulat yang dihitung (praktis bagian kedap tanpa nitrat) | `hru_urbanhr` |  | Tidak |
| 169 | Stasiun bangkitan tidak mendapat hujan sama sekali: `cli_pgenhr` mengenolkan langkah hari ini sebelum menyebar hujan hari ini ke langkah besok, dan hujan besok diambil sebagai jumlah langkah hari ini | `cli_pgenhr` | Ps\_gap: HRU stasiun bangkitan tanpa hujan tiga tahun | Tidak |
| 170 | Celah (-99) membangkitkan hari sebelumnya: berkas dibaca sehari lebih awal; langkah hari ini dinolkan (total harian tetap), hujan hari ini mengisi langkah besok dari jam pertama; total harian besok tetap jumlah berkas termasuk celah (negatif, jadi tanpa hujan) | `cli_pgenhr`, `cli_precip_control` |  | Tidak |
| 171 | Setiap badai berbentuk sama: posisi puncak dari bilangan acak yang benihnya disetel 8 setiap kali | `cli_pgenhr` |  | Tidak |
| 172 | Badai mulai tengah malam dan memakai fraksi setengah jam kemarin (milik hari ini dibangkitkan sesudahnya) | `cli_pgenhr` |  | Tidak |
| 173 | Dengan CN sub-harian, hujan berkas harian tidak pernah mencapai langkah (`cli_pgenhr` hanya menyebar hujan bangkitan): kanopi tidak mencegat dan tidak menguapkan apa pun | `cli_pgenhr`, `sq_canopyint` | Ps\_uh: ET DAS 1.253 menjadi 1.170 mm | Tidak |
| 174 | Dengan CN sub-harian tidak ada sedimen keluar HRU: lag sedimen melepas erosi langkah yang hanya dihitung Green-Ampt; sedimen MUSLE masuk simpanan yang tidak pernah dilepas (haranya tetap keluar harian) | `stor_surfstor` |  | Tidak |
| 175 | Hidrograf satuan hanya membawa limpasan permukaan: lateral dan drainase diberikan ke `flow_hyd_ru_hru` tetapi tidak ditambahkan; keduanya mencapai kanal rata per langkah | `flow_hyd_ru_hru` |  | Tidak |
| 176 | Langkah hidrograf satuan suatu hari berhenti bila laju < 1e-4, tetapi langkah hari berikut tetap dihitung | `unit_hyd`, `flow_hyd_ru_hru` |  | Tidak |
| 177 | `channel_sd_subday.txt` berisi volume per langkah (m3) walau headernya `m^3/s`; bagi dengan dtm x 60 (3.600 untuk langkah jam) untuk laju | penulis tabel | interpretasi keluaran | Tidak |

#### 17.5.22  Waduk dan sumber titik (`docs/reservoirs.md`)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 178 | `hydrology.res` serta baris sedimen dan hara diambil untuk nomor waduk itu sendiri, sedangkan tabel pelepasan dan air awal untuk baris yang ditunjuk `reservoir.con` (`res_read` mengulang waduk dengan indeks baris) | `res_read` | sama bila urutan Editor | Tidak |
| 179 | `lnvol` kurva volume-luas integer: log10(evol) - log10(pvol) dipotong; untuk volume darurat < 10 kali volume utama eksponen menjadi sangat besar dan dibatasi 0,9, sehingga luas = (psa/pvol)^0,9 x V^0,9 | `res_initial` | luas permukaan dan evaporasi | Tidak |
| 180 | `br1` dan `br2` `hydrology.res` adalah lag pelepasan (`lag_up`, `lag_down`, sebagai exp(-lag)), bukan koefisien kurva yang selalu dihitung | `res_read` |  | Tidak |
| 181 | Kondisi `vol` dengan `+` dan `-` dalam ha-m (x 10.000 m3); `*` dan `/` adalah bagian | `res_rel_conds` |  | Tidak |
| 182 | Sedimen hanya mengendap di atas konsentrasi normalnya; di bawahnya aliran keluar tanpa sedimen dan kelas (pasir, debu, liat) tetap di air | `res_sediment` |  | Tidak |
| 183 | Suhu aliran keluar 0 derajat C (hidrograf kosong tak pernah disetel), dicampur oleh kanal di hilir | `res_control` | suhu kanal hilir | Tidak |
| 184 | Waduk tanpa aliran masuk tidak dijalankan; sebelum bulan operasional pertamanya waduk meneruskan aliran masuk | `res_control` |  | Tidak |
| 185 | `basin_reservoir_output` mengenolkan aliran masuk dan keluar harian tiap waduk setelah menjumlahkannya, sehingga waduk yang tidak dijalankan mencetak nol | `basin_reservoir_output` |  | Tidak |
| 186 | Recall tahunan memasukkan beban setahun penuh setiap hari (`recall%hd(1, yrs)`; uji tahunnya `.or.`, selalu benar); recall harian atau bulanan tidak memberi apa pun di luar tahun berkasnya; debit berkas dalam m3 (header `m^3/s`) | `recall_read`, `command` | beban tahunan berlipat 365 | Tidak |
| 187 | `recall.rec` dibaca ada atau tidaknya objek recall, dan berhenti pada berkas pertama yang hilang atau berakhir sebelum headernya (`RECYR.` di Osu\_1hru) | `recall_read` |  | Tidak |
| 220 | Reservoir atau lahan basah tanpa baris `sediment.res` memakai faktor 0 (`sed_stlr_co`, `soln_stl_fr`, `solp_stl_fr`): semua sedimen di atas konsentrasi normal (0) langsung mengendap, dan N/P terlarut tidak mengendap | `res_read`, `wet_initial` | Bh\_nosed, Ps\_edge (port mengikuti sejak 1.0.1) | Tidak |

#### 17.5.23  P tanah dinamis dan regresi perkotaan (`docs/nutrients.md`)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 188 | Lapisan tanpa liat mempertahankan PSP terakhir yang dihitung saat awal (`psp` lokal tersimpan `soil_nutcarb_init`: milik lapisan sebelumnya atau HRU sebelumnya) dan memakai 0,4 setiap hari | `soil_nutcarb_init`, `nut_pminrl2` |  | Tidak |
| 189 | Lapisan tanpa CaCO3 mendapat 2,8 % (rata-rata 20.000 tanah SSURGO) dari `soil_phys_init`, jadi `soils.sol` Editor yang berisi nol mendapat nilai itu; laju aktif ke stabil = 0,0023 CaCO3 + 0,005, dibatasi 0,002 sampai 0,05 | `soil_phys_init`, `nut_pminrl2` |  | Tidak |
| 190 | `usgs_reg`: pada hari dengan hujan dan limpasan > 0,1 mm, beban padatan, N dan P regresi Driver dan Tasker (pita curah hujan tahunan generator: < 508, < 1.016, lebih) mengganti bagian kedap sedimen (0,001 x padatan kg), N organik dan nitrat (70 % dan 30 % N), P organik dan larut (75 % dan 25 % P) | `hru_urban`, `regres` | Ps\_urban: nitrat permukaan 0,16 menjadi 3,0 sampai 3,5 kg/ha/tahun | Tidak |

#### 17.5.24  Praktik struktural (`docs/structures.md`)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 191 | `topohyd_init` menyetel `cn3_swf` 0,95 dan `perco` 0,1 untuk HRU berdrainase; kalibrasi keduanya lalu dilewati untuknya | `topohyd_init`, `cal_conditions` |  | Tidak |
| 192 | Drainase terletak di lapisan pertama yang dasarnya lebih dalam dari drainase (`ldrain`) | `soils_init` |  | Tidak |
| 193 | Muka air dihitung di akhir `swr_percmain` dari air di atas FC; aliran drainase (wt\_shall - d)/wt\_shall x (sw - fc) x (1 - exp(-24/time)), dibatasi koefisien drainase, diambil dari lapisan di atas FC | `swr_percmain`, `swr_drains` |  | Tidak |
| 194 | Nitrat drainase tercuci dengan `nperco_lchtile`, dan bagian drainase menimpa bagian lateral dan perkolasi; P larut drainase (`tilelabp`) ditambahkan ke milik limpasan permukaan | `nut_nlch`, `nut_solp` |  | Tidak |
| 195 | Aliran drainase dan nitratnya dilag di `swr_substor` dengan 1 - exp(-24/lag) sebelum mencapai unit routing | `swr_substor` |  | Tidak |
| 196 | Jalur penyaring hanya bekerja bila `flag` 1, sedangkan saluran berumput dan praktik pengguna bekerja apa pun benderanya | `smp_filter`, `smp_grass_wway`, `smp_bmpfixed` |  | Tidak |
| 197 | Sedimen tertahan jalur penyaring diambil dari kelas setelah hasil dikurangi (sedtrap = sedyld x sed\_remove/100 dengan hasil baru): kelas kehilangan lebih sedikit dari total | `smp_filter` |  | Tidak |
| 198 | P larut jalur penyaring memakai bagian kedua P partikulat: baris yang dimaksud menulis `remove21`, sehingga `remove2` tetap yang partikulat | `smp_filter` |  | Tidak |
| 199 | Waktu konsentrasi saluran berumput tanpa waktu aliran permukaan: `structure_init` berjalan sebelum `time_conc_init` menghitung `t_ov` | `structure_init` |  | Tidak |
| 200 | Efisiensi `bmpuser.str` adalah bagian 0 sampai 1, bukan persen SWAT2012 (1 - `bmp_sed`); bagian N larut juga mengurangi nitrat lateral | `smp_bmpfixed` |  | Tidak |
| 201 | Efluen tidak pernah mencapai biozona: debit harian tangki (`qstemm`) hanya dinolkan, tidak pernah disetel dari penghuni, kepadatan sistem atau `septic.sep` | `sep_biozone` | septik hanya menyegel tanah | Tidak |
| 202 | Biozona gagal pada hari pertama lalu menyegel tanah: kejenuhan direset ke porositas x `thk_bioz` sementara FC lapisan asal penuh dipertahankan, sehingga langsung di bawah FC dan sistem menjadi gagal; waktu perkolasi biozona gagal 10^10 jam; tidak pernah diperbaiki (hari sejak gagal dihitung hanya bila > 0, dan tak ada yang memulainya) | `sep_biozone` | Ps\_septic: perkolasi sekitar 1.350 mm/tahun menjadi nol, limpasan 270 menjadi 1.590 mm | Tidak |
| 203 | Status operasi milik tipe septik, bukan HRU: bila biozona satu HRU gagal, setiap HRU dengan baris `septic.str` itu ikut gagal mulai HRU berikut yang dihitung hari itu | `sep_biozone` | membawa keadaan antar-HRU | Tidak |
| 204 | Lapisan yang dipecah mempertahankan air dan kapasitas mm lapisan asal (`layersplit` hanya mengubah kedalaman dan ketebalan): tanah terpecah menyimpan lebih banyak air | `layersplit` |  | Tidak |
| 205 | Bila biozona mulai dalam satu inci dari atas lapisan, `i_sep` adalah lapisan di bawahnya, dan lapisan itu yang diperlakukan sebagai biozona | `soils_init` |  | Tidak |
| 206 | Tanah dari biozona ke bawah dijaga lebih hangat (di bawah 10 derajat C, sembilan persepuluh selisih dihilangkan) sejak tahun pertama sistem; biozona aktif dikecualikan dari nitrifikasi `nut_nitvol` dan denitrifikasi `nut_nminrl`; semua lapisan biozona dikecualikan dari pencucian P larut | `stmp_solt`, `nut_nitvol`, `nut_nminrl`, `nut_solp` |  | Tidak |

#### 17.5.25  Deposisi atmosfer dan Muskingum (`docs/atmo.md`)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 207 | Langkah waktu deposisi maju pada awal hari terakhir bulan (atau tahun): bendera akhir bulan disetel sebelum `cli_atmodep_time_control`, sehingga hari terakhir setiap bulan mendapat deposisi bulan berikutnya | `cli_atmodep_time_control` |  | Tidak |
| 208 | Setelah data habis tidak ada yang ditambahkan, dan `hru_nb` mencetak ulang nilai hari terakhir (`hnb_d` tidak dinolkan); kode `atmo_dep` di `codes.bsn` tidak dipakai, berkas yang menentukan | `nut_nrain`, `cli_read_atmodep` | kolom keluaran | Tidak |
| 209 | Muskingum: hari dipotong menjadi Int(24/step) + 1 bagian untuk menghitung koefisien, tetapi run harian lalu merouting hari itu dalam satu langkah dengan koefisien tersebut (`ch_rtmusk` mereset langkah) | `ch_rtmusk` |  | Tidak |

#### 17.5.26  Penjumlahan periode keluaran (`docs/outputs.md`)

| No | Kebiasaan | Lokasi Fortran | Dampak | Koreksi --fix-swat? |
| --- | --- | --- | --- | --- |
| 210 | Fluks dijumlah, keadaan dirata-rata: keadaan bulanan dibagi jumlah hari bulan (semua hari, walau bulan hanya sebagian disimulasikan), tahunan dibagi hari terakhirnya (`day_end_yr`), rata-rata tahunan dibagi semua hari tercetak; fluks dibagi jumlah tahun hanya untuk rata-rata tahunan | `hru_output`, `basin_output`, dsb.; `swat-engine/src/periods.rs` |  | Tidak |
| 211 | Tahun tercetak (`yrs_prt`) = hari / 365 (ditambah tahun kabisat dalam integer, jadi 0 kecuali semua kabisat); selama tahun pemanasan `yrs_prt` disetel jumlah tahun simulasi, sehingga dengan `nyskip` > 0 nilainya jumlah itu ditambah hari tercetak | `time_control` | pembagi rata-rata tahunan | Tidak |
| 212 | Air tanah dan salju bulanan HRU = rata-rata akhir tiap hari, sedangkan tabel harian mencetak rata-rata awal dan akhir; DAS dan unit lanskap menjumlah rata-rata harian | `hru_output` |  | Tidak |
| 213 | N dan P tanaman (`nplt`, `pplnt`) pada periode HRU adalah milik hari terakhir; milik DAS dirata-rata, dan `pplnt` DAS selalu 0 (`hruout_plantweather_add` melewatkannya), begitu pula unit lanskap | `hruout_plantweather_add` |  | Tidak |
| 214 | Biomassa maksimum (`bm_max`) adalah terbesar periode; rata-rata tahunan melewatkan hari terakhir setiap tahun (dikembalikan ke nilai hari sebelumnya) | `hru_output` |  | Tidak |
| 215 | Akuifer merata-ratakan simpanan, muka air dan nitrat tersimpan atas hari-hari bulan, lalu atas 12 bulan; karbon milik hari terakhir | `aquifer_output` |  | Tidak |
| 216 | Kanal hanya merata-ratakan debit (bukan beban) atas hari periode; simpanan dan suhu air milik hari terakhir; luas air rata-rata tahunan dibagi jumlah tahun dua kali; kanal DAS menjumlah nilai harian tanpa merata-ratakan debit | `sd_channel_output`, `basin_ch_output` |  | Tidak |
| 217 | Unit lanskap dan DAS membobot HRU dengan `sub_frac`/`bsn_frac` `ls_unit.ele`; DAS membaca `obtypno` elemen i untuk HRU i, dan unit lanskap membaca nomor elemen sebagai nomor HRU | `lsu_output`, `basin_output` |  | Tidak |
| 218 | Hasil panen rata-rata tahunan tanaman HRU = jumlah hasil sepanjang run termasuk pemanasan dibagi jumlah panennya; hasil DAS per tahun menurut luas; aksi panen-saja tabel keputusan tidak menghitung panen (menambah `harv_num_yr` dua kali) | `hru_output`, `basin_output` | tabel hasil | Tidak |

### 17.6  Koreksi `--fix-swat`

Secara bawaan port mereproduksi SWAT+ termasuk bug-nya, karena itulah cara port divalidasi. Dengan `swatplus-rust run <dir> --fix-swat` (dalam kode `run_with(.., RunOptions { fixes: true })`, di WebAssembly flag 4) bug di bawah ini dikoreksi. Setiap koreksi dijaga bendera yang sama (`fixes`), sehingga run setia tidak berubah sedikit pun. Hasil terkoreksi berbeda dari SWAT+, jadi run terkoreksi harus dikalibrasi ulang, bukan dibandingkan dengan kalibrasi SWAT+. Program mencetak catatan "--fix-swat: SWAT+'s bugs corrected, results will differ from SWAT+". Kolom "Penjaga di kode" menyebut tempat bendera diuji (hasil pencarian `fixes` di `swat-engine`); kolom terakhir merujuk nomor butir katalog 17.5.

| No | Lokasi SWAT+ | Bug | Koreksi | Penjaga di kode | Butir katalog |
| --- | --- | --- | --- | --- | --- |
| F1 | `ero_ysed` (tutupan salju) | cabang terbalik: salju apa pun hanya mengenolkan hasil < 1e-6; batas 100 mm dan reduksi eksponensial tak pernah berlaku | tanpa erosi di atas 100 mm salju; di bawahnya dibagi exp(sno x 3/25,4) | `hru/nutrient.rs` | 52 |
| F2 | `psed` | bagian P mineral aktif dan stabil tertukar | aktif/terikat ke kolam aktif, stabil/terikat ke kolam stabil | `hru/nutrient.rs` | 54 |
| F3 | `sq_sed` / kelas sedimen | sedimen dari simpanan kelas dikurangkan dua kali | dikurangkan sekali (per langkah sub-harian memang sekali) | `hru/mod.rs` (`surf_bs_sed[0] -= lagged[0]` hanya bila tidak `fixes` atau sub-harian) | 53 |
| F4 | `pl_nupd`, `pl_pupd` | hanya permintaan N dan P tanaman terakhir dinolkan tiap hari; tanaman lain dalam komunitas memakai permintaan kemarin | permintaan setiap tanaman mulai hari dari nol | `hru/plant.rs` | 42 |
| F5 | `hru_hyds` (`subwq`) | rasio pengayaan dari HRU sebelumnya (variabel modul) | rasio HRU itu sendiri hari itu (`nd.enratio`) | `hru/mod.rs` | 97 |
| F6 | `et_act` (lahan basah) | koefisien evaporasi air terbuka dari HRU bernomor sama dengan baris hidrologi lahan basah | milik lahan basah itu sendiri (`evrsv`) | `hru/mod.rs` | 108 |
| F7 | `et_pot` (Penman-Monteith) | resistansi kanopi memakai konduktansi stomata tanaman terakhir | rata-rata tertimbang LAI (`gsi_wav`) | `hru/mod.rs` | 103 |
| F8 | akhir tahun | reset total irigasi tahunan dikomentari, batas tahunan tercapai sekali untuk selamanya | direset tiap tahun (`irr_yr = 0`) | `hru/mod.rs::end_year` | 68 |
| F9 | `pl_fert_wet` (sawah) | pupuk pada sawah tergenang hanya masuk air dengan model karbon 2; dengan model asli hilang (tercatat diaplikasikan, tidak di kolam mana pun) | masuk air dengan kedua model (kondisi `cswat == 2` atau `fixes`) | `hru/mgt.rs` | 135 |
| F10 | `mgt_newtillmix_wet` (pelumpuran) | hara genangan ditambahkan ke jumlah pencampuran yang lalu dinolkan: hilang, air hanya mendapat hara tanah | hara genangan bercampur dengan milik tanah | `hru/nutrient.rs` | 134 |
| F11 | `ru_control` | `ru_d` menjumlah lima hidrograf (total, recharge, permukaan, lateral, drainase); total sudah memuat tiga terakhir | hanya total dan recharge | `route.rs` | 95 |
| F12 | `aqu_initial` | simpanan awal memakai `aqu_d(iaqdb)%dep_wt`, keadaan akuifer yang nomornya penunjuk database akuifer ini, terisi hanya bila akuifer itu lebih dulu | `dep_wt` akuifer itu sendiri | `route.rs` | 98 |
| F13 | `aqu_hdr` | kolom 9 dan 10 berlabel `orgn orgp` padahal karbon dan N organik | berlabel `cbn orgn` | `route.rs` | 99 |
| F14 | `sd_rating_curve` (dan `stor_dis` Muskingum) | luas b d + d/z, keliling basah memperlakukan z sebagai vertikal per horizontal | trapesium: luas b d + z d^2, keliling b + 2 d sqrt(1 + z^2) | `channel.rs` | 113 |
| F15 | `sd_rating_curve` (dataran banjir) | di atas penuh tebing lebar aliran = lebar dasar kanal | lebar atas (`chw`) | `channel.rs` | 113 |
| F16 | `ch_watqual4` | waktu tempuh memakai `ht3%flo` yang tak pernah disetel (kurva pada debit nol) | kurva pada laju aliran masuk hari itu (`ht1.flo / 86400`) | `channel.rs` | 115 |
| F17 | `sd_channel_control3` / `ch_temp` | hidrograf keluar disalin sebelum suhu sungai dihitung, jadi keluar dengan suhu aliran masuk | aliran keluar membawa suhu terhitung | `channel.rs` | 116 |
| F18 | `ch_temp` | suhu air hulu diambil dari aliran masuk terakhir dan sebelumnya, apa pun itu | rata-rata tertimbang debit semua kanal hulu | `channel.rs` | 116 |
| F19 | `channel_sd_day` `water_temp` | satu variabel modul dicetak untuk semua kanal: perkiraan berbasis udara kanal terakhir | suhu sungai setiap kanal | `channel.rs` | 117 |

Dampak pada `Bh_land` (2015 sampai 2016) menurut `docs/swat-fixes.md`: debit unit routing menjadi separuh (air permukaan, lateral dan drainase ganda hilang); tampungan dan rembesan kanal naik beberapa persen karena luas trapesium lebih besar, debit outlet berubah kurang dari 0,1 %; suhu air kanal kini berbeda per kanal (21 sampai 28 derajat C pada satu hari pertengahan tahun, padahal SWAT+ mencetak satu nilai). Pada `Ps_paddy` koreksi F9 menaikkan hasil gabah dari 4,11 sampai 4,60 menjadi 4,12 sampai 4,73 t/ha per musim (`docs/paddy.md`).

Kebiasaan lain di 17.5 (misalnya stres suhu sinus, N termineralisasi yang hilang di `cbn_zhang2`, efluen septik yang tak pernah ada, recall tahunan) tidak dikoreksi `--fix-swat` pada engine 1.0; mode setia maupun terkoreksi mereproduksinya.

### 17.7  Fitur yang ditolak atau belum didukung engine 1.0

Prinsip README: "What isn't ported is refused by name (`run` says why), never guessed." Penolakan muncul sebagai `Err(String)` dari tahap penyiapan. `run` (`main.rs`) mengumpulkannya di `RunReport`: `hru_skipped` (neraca air HRU tidak disimulasikan; `hru_wb_day.txt` hanya berisi presipitasi), `routing_skipped` (unit routing dan akuifer tidak disimulasikan) dan `channel_skipped` (kanal tidak dirouting dan hanya meneruskan aliran masuk). Masing-masing menjadi catatan (`note`, atau event JSON `note`); dengan `--strict` program tetap menulis semua tabel (juga Parquet), lalu gagal dengan kode keluar 3 dan pesan `not simulated: ...`. Di WebAssembly alasan yang sama masuk daftar `notes` di `run.json`. Daftar di bawah dikumpulkan dari semua `Err(...)` di `swat-engine/src` (tidak ada string "refused" atau "unsupported" di kode; istilah itu hanya ada di docs).

#### 17.7.1  Kode dan opsi tingkat DAS

| Fitur | Pesan / pengecekan | Lokasi Rust | Dampak |
| --- | --- | --- | --- |
| model karbon `cswat` selain 0 dan 2 (mis. 1) | `carbon model N` | `hru/mod.rs::init_hrus` | HRU tidak disimulasikan |
| Green-Ampt dengan langkah harian | `Green and Ampt infiltration with daily time steps` | `init_hrus` | idem |
| lebih dari 96 langkah per hari | `N time steps a day (the port keeps up to 96)` | `init_hrus` | idem |
| metode PET di luar 0 sampai 3 | `potential ET method N` | `init_hrus` | idem (Priestley-Taylor `pet` 0 diport tetapi menurut `phase3.md` belum divalidasi) |
| aliran retakan (`crk`) | `crack flow` | `init_hrus` | idem |
| gwflow | `gwflow` | `init_hrus` | idem |
| `co2_yr.dat` | `co2_yr.dat` | `hru/mod.rs` (inisialisasi tanaman) | idem |
| kanal dengan *lapse rate* (`lapse` 1) | `channels with lapse rates` | `channel.rs` | kanal tidak dirouting |
| `temperature.cha` | `temperature.cha` | `channel.rs` | idem |
| waduk dengan *lapse rate* | `reservoirs with lapse rates` | `route.rs` | routing tidak disimulasikan |

#### 17.7.2  HRU, tanah, tanaman

| Fitur | Pesan | Lokasi Rust |
| --- | --- | --- |
| lahan basah HRU dengan model karbon 2 | `wetland with the carbon model` | `hru/mod.rs` |
| hidrologi lahan basah tanpa psa, esa, evap atau k | `wetland hydrology ... without psa, esa, evap or k` | `hru/wetland.rs` |
| air awal lahan basah (`om_water`) | `wetland ...: initial water (...)` | `hru/wetland.rs` |
| tabel pelepasan lahan basah tidak ada di `res_rel.dtl` (kecuali sawah) | `release table ... not in res_rel.dtl` | `hru/wetland.rs` |
| kondisi pelepasan lahan basah selain `vol_wet` pada objek 0, terhadap selain `pvol`/`evol`, operator selain `* + - /` | `wetland release condition ...` | `hru/wetland.rs` |
| basis pelepasan selain `null`, `pvol`, `evol`; opsi selain `rate`, `rate_pct`, `ab_emer`, `days`, `dyrt` | `wetland release base/option ...` | `hru/wetland.rs` |
| sedimen dalam aliran lateral (`lat_sed` > 0) | `sediment in lateral flow (lat_sed)` | `hru/mod.rs` |
| drainase yang namanya tidak ada di `tiledrain.str` | `tile drain ... not in tiledrain.str` | `hru/mod.rs` |
| drainase DRAINMOD (`tdrn` 1) | `tile drains by the DRAINMOD equations` | `hru/mod.rs` |
| muka air dari kelembapan tanah (`wtdn` 1) | `tile drains with the water table of the soil moisture (wtable 1)` | `hru/mod.rs` |
| septik dengan model karbon (`cswat` 1 atau 2) | `septic system with carbon model N` | `hru/mod.rs` |
| biozona di bawah dasar tanah; biozona di lapisan terakhir atau dua lapisan pertama (SWAT+ akan membaca di luar tanah) | `septic biozone at ... below the soil`, `septic biozone layer ...` | `hru/septic.rs` |
| komunitas tanaman > 20 tanaman | `plant community ... (SWAT+ keeps 20)` | `hru/mod.rs` |
| tanaman dengan model karbon 1 | `plants with carbon model 1` | `hru/mod.rs` |
| `days_mat` untuk tanaman bertipe `null` (hari dari panjang hari) | `days to maturity for a plant of type ...` | `hru/plant.rs` |

#### 17.7.3  Pengelolaan dan tabel keputusan

| Fitur | Pesan | Lokasi Rust |
| --- | --- | --- |
| operasi jadwal selain `plnt`, `hvkl`, `harv`, `kill`, `till`, `irrm`, `fert`, `manu`, `skip` (mis. penggembalaan, pestisida, pembakaran) | `schedule ...: operation ...` | `hru/mgt.rs` |
| pupuk kandang (`manu`) dengan model karbon 2 | `schedule ...: manure with carbon model 2` | `hru/mgt.rs` |
| tabel keputusan generik tanpa tanamannya di `management.sch` | `generic decision table without its crops` | `hru/mgt.rs` |
| tabel keputusan yang tidak ada di `lum.dtl` | `decision table ... not in lum.dtl` | `hru/mgt.rs` |
| kondisi selain `month`, `days_plant`, `days_harv`, `days_kill`, `days_irr`, `precip_cur`, `weirh`, `wet_depth`, `prob_unif`, `w_stress`, `irr_year`, `phu_plant`, `leaf_area`, `phu_base0`, `soil_water`, `jday`, `year_rot` | `condition ...` | `hru/mgt.rs` |
| `precip_cur` atau `phu_base0` untuk objek selain `hru`; `soil_water` terhadap selain `wp`, `fc`, `ul` atau operator selain `* + - /`; kondisi pada nomor objek bukan 0 (kecuali `prob_unif`); alternatif selain `- < > <= >= = /=` | pesan masing-masing | `hru/mgt.rs` |
| aksi selain `irrigate`, `plant`, `till`, `fertilize`, `weir_height`, `puddle`, `harvest_kill`, `harvest`, `kill`, `rot_reset` | `action ...` | `hru/mgt.rs` |
| irigasi dari akuifer, kanal atau waduk | `irrigation from ...` | `hru/mgt.rs` |
| aksi pada nomor objek bukan 0 | `... on object N` | `hru/mgt.rs` |
| tanam pindah (`transplant.ops`) | `transplant ...` | `hru/mgt.rs` |
| tinggi bendung waduk (`weir_height` dengan opsi selain `wet`) | `weir height of a reservoir` | `hru/mgt.rs` |
| pelumpuran yang tidak ada di `puddle.ops` atau olah tanahnya tidak ada di `tillage.til` | `puddling ... not in ...` | `hru/mgt.rs` |

#### 17.7.4  Kalibrasi keras

| Fitur | Pesan | Lokasi Rust |
| --- | --- | --- |
| parameter jenis selain `hru`, `sol`, `bsn`, `aqu` | `calibration of ... (... parameter)` | `hru/mod.rs::cal_conditions` |
| jenis perubahan selain `absval`, `abschg`, `pctchg`, `relchg` | `calibration of ...: change ...` | idem |
| parameter yang tidak ada dalam daftar yang diport (`docs/engine1.md`) | `calibration of ...` | `hru/mod.rs`, `route.rs` |
| kondisi `res_pvol`, `res_typ`, `pl_class` | `calibration of ... with a condition on ...` | `hru/mod.rs` |
| kondisi pada perubahan parameter akuifer | `calibration of ... with conditions (aquifer)` | `route.rs` |
| nomor HRU di luar jangkauan | `calibration of ...: HRU N` | `hru/mod.rs` |

Parameter yang hanya dipakai SWAT+ saat penyiapan (`ovn`, `slope_len`, `field_len`, `ec`, `ph`, `flo_init_mm`, `dep_wt_init`, `cbn_init`, `flo_dist` akuifer, `bf_max` tanpa akuifer 2-D) diterima tetapi tidak mengubah apa pun.

#### 17.7.5  Objek, routing, waduk, recall

| Fitur | Pesan | Lokasi Rust |
| --- | --- | --- |
| objek jenis HRU-lte, gwflow, kanal lama (`channel.con`), koefisien ekspor, rasio penghantaran (`delratio`) | `<Jenis> objects` | `route.rs` |
| akuifer 2-D | `2-D aquifers` | `route.rs` |
| aliran keluar ke jenis tak dikenal atau nomor di luar jangkauan; hidrograf selain `tot`, `rhg`, `sur`, `lat`, `til`; hidrograf melebihi jumlah milik objek | pesan masing-masing | `route.rs` |
| air dikirim ke unit routing, atau dari objek bukan HRU ke HRU | `water routed into ...` | `route.rs` |
| elemen unit routing bukan HRU | `routing unit ...: element of type ...` | `route.rs` |
| rasio penghantaran selain `calc`/`0`, `full`/`1`, atau nama `delratio.del` | `routing unit ...: delivery ratio ...` | `route.rs` |
| loop tak hingga di berkas koneksi | `an infinite loop in the connection files` | `route.rs` |
| air awal kanal (bukan `no_init`) | `channel ...: initial water ...` | `channel.rs` |
| kanal sepanjang <= 0,001 km (kanal lintas) | `channel ...: length ... km (pass-through channels)` | `channel.rs` |
| pelepasan waduk dengan tabel kondisi `ctbl_` | `reservoir release by condition table ...` | `res.rs` |
| kondisi pelepasan waduk selain `vol` (terhadap `pvol`, `evol`, `e-pv`) dan `month`, pada objek bukan 0, operator selain `* + - /` | pesan masing-masing | `res.rs` |
| opsi pelepasan selain `rate`, `rate_pct`, `inflo_rate`, `inflo_frac`, `ab_emer`, `days`, `dyrt`, `dyrt1`, `inflo_targ`, `natlake`, `nonirr-h06`, `hydrop` (jadi `weir` dan `meas` ditolak) | `reservoir release option ...` | `res.rs` |
| `irr-h06` (SWAT+ melepas NaN) | `reservoir release irr-h06 (SWAT+ releases NaN)` | `res.rs` |
| `dyrt1` pada selain `con1`, `con2`, `con3` atau operator lain | `reservoir release dyrt1 ...` | `res.rs` |
| recall sub-harian (tipe 0) atau tipe selain 1 sampai 3 | `subdaily recall objects`, `recall objects of type N` | `res.rs` |
| diversi (debit recall negatif) | `recall ...: a diversion (negative flow)` | `res.rs` |
| rekaman recall di luar jangkauan | `recall ...: record out of range ...` | `res.rs` |

Ketidaksesuaian dokumen yang dulu dicatat di sini sudah dikoreksi di 1.0.1: `docs/reservoirs.md` kini menyebut bahwa opsi pelepasan danau, Hanasaki dan PLTA (`natlake`, `nonirr-h06`, `hydrop`) sudah diport dan merujuk ke `docs/engine1.md`, sesuai kode (`res.rs`). Demikian pula `swat-quirks.md` tidak lagi menulis bahwa `usgs_reg` belum diport (lihat `docs/nutrients.md`).

### 17.8  Kinerja dan batas paralelisme

- **Angka kinerja** (`docs/runner.md`): engine berjalan pada satu thread. Seluruh Pessel (3.966 HRU, 1.454 kanal) membutuhkan 6,4 detik per tahun simulasi pada satu inti, dibanding 16,7 detik untuk build Release SWAT+ dengan gfortran, yaitu 2,6 kali lebih cepat. `docs/phase3.md` (lebih awal) mencatat Pessel 2015 berjalan sekitar satu menit di port dan dua menit di SWAT+.
- **Faktor pendukung:** tanpa dependensi eksternal (README); tabel dibaca dan dibandingkan sebagai aliran; tabel Parquet ditulis tanpa kompresi dengan satu *row group* per sejuta baris (`docs/runner.md`).
- **Mengapa HRU tidak dijalankan paralel:** beberapa variabel SWAT+ membawa nilai satu HRU ke hari HRU berikutnya, sehingga urutan perhitungan menentukan hasil. `docs/runner.md` menyebut: bilangan acak tabel keputusan (butir 132), permintaan hara tanaman (butir 42), rasio pengayaan (butir 97), fraksi fosfor sedimen, dan status sistem septik (butir 203). Butir katalog lain dengan sifat serupa: `c_microb_sed` di `nut_orgnc2` (butir 60), PSP tersimpan `soil_nutcarb_init` dan hari tanam tersimpan `plant_init` saat inisialisasi (butir 188, 151), `inflo` dan `wtemp` kanal yang dipertahankan dari kanal terakhir (butir 166, 117), serta urutan penjumlahan aliran masuk dalam loop perintah (butir 92). Dalam port, keadaan semacam ini dipegang di struktur `Saved` dan dibaca serta ditulis dalam urutan yang sama dengan SWAT+ (`docs/phase2.md`, risiko *implicit SAVE*).
- **Arah ke depan:** menurut `docs/runner.md`, run paralel termasuk mode terkoreksi (`--fix-swat`), setelah variabel-variabel tersebut dibuat per HRU. Pada engine 1.0 baru sebagian yang dikoreksi (F4, F5); bilangan acak `prob_unif`, status septik dan lainnya masih bersama, sehingga mode terkoreksi pun masih sekuensial.
- **Batas determinisme lintas platform:** hasil bit-for-bit dijamin hanya untuk program native Linux dengan glibc (sama dengan acuan). WebAssembly dan macOS/Windows dapat berbeda di bit terakhir fungsi matematika presisi tunggal, tetapi tetap dalam toleransi K12 (17.4.3).
- **Batas sumber daya CI:** tabel harian Batanghari mencapai gigabita, sehingga `validate.yml` menghapus tabel harian setelah dibandingkan, memotong periode Batanghari dan `Bh_land` ke 2015 sampai 2019 dan Pessel ke 2015, dan mematikan diagnostik karbon harian (`hru_cb`); job dibatasi 60 menit.
