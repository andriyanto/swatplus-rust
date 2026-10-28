## Bab 15  swat-output: tabel teks, Parquet, arsip tar, dan progres

Bab ini membahas *crate* `swat-output`, lapisan paling akhir dari engine: bagian yang mengubah angka harian, bulanan, tahunan, dan rata-rata tahunan yang dihitung `swat-engine` menjadi berkas yang dapat dibaca orang dan program lain. Crate ini kecil (sekitar 1.300 baris dalam enam berkas: `lib.rs`, `store.rs`, `text.rs`, `parquet.rs`, `tar.rs`, `progress.rs`) tetapi memuat empat format berkas yang berbeda: tabel teks bergaya SWAT+ (`<tabel>_day.txt`, `_mon`, `_yr`, `_aa`), berkas Apache Parquet yang ditulis tangan tanpa pustaka luar, arsip tar berformat *ustar* untuk WebAssembly, dan laporan progres berupa baris-baris JSON (*JSON lines*). Dependensinya hanya `swat-model` (untuk `PrintSettings` dari `print.prt`) dan `swat-engine` (untuk trait `OutputSink` dan tipe-tipe baris keluaran). Tidak ada dependensi pihak ketiga: komentar di `parquet.rs` menyatakan bahwa engine sengaja tidak mengambil dependensi di luar apa yang diizinkan lisensi LGPL-nya, sehingga protokol Thrift untuk metadata Parquet pun ditulis tangan.

Posisinya dalam alur simulasi: `swat_engine::run::run_with(project, &mut sink, opts)` (Bab tentang runner) memanggil method-method trait `OutputSink` pada setiap hari yang dicetak dan pada setiap akhir periode. Implementasi `OutputSink` yang dipakai program `swatplus-rust` dan modul WebAssembly `swat-wasm` adalah `TextTables<S: Store>` dari crate ini. `TextTables` menyusun baris-baris teks dalam penyangga per berkas, lalu mengalirkannya ke sebuah `Store` (folder di disk, `DirStore`, atau memori, `MemStore`) beberapa megabita sekali tulis. Setelah simulasi selesai, pemanggil (`crates/swatplus-rust/src/main.rs` atau `crates/swat-wasm/src/lib.rs`) membaca kembali setiap tabel teks dan, bila diminta `--format parquet|both` atau bendera WebAssembly 1, mengubahnya menjadi Parquet dengan `parquet::text_to_parquet`. Jadi Parquet di port ini adalah **turunan** tabel teks: nilainya persis angka yang tercetak di teks (dengan presisi cetak SWAT+), bukan nilai f32 internal. Arsip tar dipakai di peramban, tempat tidak ada sistem berkas: skenario masuk sebagai tar berisi berkas TxtInOut, tabel keluar sebagai tar. Progres dilaporkan oleh `progress::Reporter` sebagai objek JSON per baris di keluaran standar (opsi `--progress json`).

Padanan Fortran-nya tersebar: setiap tabel teks SWAT+ ditulis oleh subrutin `*_output.f90` sendiri (`hru_output.f90`, `basin_output.f90`, `lsu_output.f90`, `ru_output.f90`, `aquifer_output.f90`, `basin_aquifer_output.f90`, `sd_channel_output.f90`, `basin_sdchannel_output.f90`, `reservoir_output.f90`, `basin_reservoir_output.f90`, `recall_output.f90`, `basin_recall_output.f90`), dengan judul dan header kolom ditulis saat inisialisasi oleh `header_write.f90`, `header_sd_channel.f90`, `header_aquifer.f90`, `header_yield.f90`, `output_landscape_init.f90`, dan sejenisnya. Parquet, tar, dan JSON-line tidak punya padanan di SWAT+ Fortran (SWAT+ hanya menulis teks dan, opsional, CSV melalui `pco%csvout`, yang tidak ditulis port). Validasi "bit-for-bit" yang disebut `docs/outputs.md` dilakukan per nilai, kolom demi kolom, oleh perintah `swatplus-rust compare` (`crates/swatplus-rust/src/compare.rs`), bukan per karakter: lebar kolom port dan Fortran tidak selalu identik, dan bab ini mencatat perbedaannya di tempat yang relevan.

### 15.1  `lib.rs`: peta modul dan ringkasan dokumen keluaran

`lib.rs` hanya mendeklarasikan lima modul publik (`parquet`, `progress`, `store`, `tar`, `text`) dan mengekspor ulang `DirStore`, `MemStore`, `Store`, dan `TextTables` di akar crate. Atribut `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]` berlaku untuk seluruh crate: kode keluaran tidak boleh panik, setiap kegagalan harus dikembalikan sebagai `Result<_, String>` atau `std::io::Result`. Konsekuensinya terlihat di seluruh bab: `writeln!` ke `String` hasilnya dibuang dengan `let _ =`, parsing angka memakai `unwrap_or`, dan galat penulisan berkas disimpan dalam field `error` lalu dilaporkan di `finish`. `lib.rs` tidak berisi fungsi.

**Tabel yang ditulis (dari `docs/outputs.md`).** Port menulis tabel untuk objek dan periode yang diminta `print.prt` (nilai `y` di kolom *daily*, *monthly*, *yearly*, atau *avann*):

| Baris `print.prt` | Berkas | Penulis Fortran |
| --- | --- | --- |
| `hru_wb`, `hru_nb`, `hru_ls`, `hru_pw` | `hru_*_day`, `_mon`, `_yr`, `_aa` | `hru_output.f90` |
| `ru` | `ru_*` | `ru_output.f90` |
| `aquifer` | `aquifer_*` | `aquifer_output.f90` |
| `channel_sd` | `channel_sd_*`, `channel_sd_subday.txt` | `sd_channel_output.f90` |
| `basin_wb`, `_nb`, `_ls`, `_pw` | `basin_*` (perlu `ls_unit.ele`) | `basin_output.f90` |
| `lsunit_wb`, `_nb`, `_ls`, `_pw` | `lsunit_*` (perlu `ls_unit.def`) | `lsu_output.f90` |
| `basin_aqu` | `basin_aqu_*` (perlu `aqu_catunit.ele`) | `basin_aquifer_output.f90` |
| `basin_sd_cha` | `basin_sd_cha_*` | `basin_sdchannel_output.f90` |
| `reservoir`, `basin_res` | `reservoir_*`, `basin_res_*` | `reservoir_output.f90`, `basin_reservoir_output.f90` |
| `recall`, `basin_psc` | `recall_*`, `basin_psc_*` | `recall_output.f90`, `basin_recall_output.f90` |
| bendera `crop_yld` (`y`, `a`, `b`) | `basin_crop_yld_yr`, `_aa` (`y`, `b`); `crop_yld_aa` (`a`, `b`) | `time_control.f90`, `hru_output.f90` |

Baris harian mengikuti jendela cetak dan interval `print.prt`; periode dicetak setelah tahun pemanasan (`nyskip`). Aturan penjumlahan periode (fluks dijumlahkan, keadaan dirata-ratakan, `yrs_prt` = hari dibagi 365, dan seterusnya) adalah urusan `swat-engine/src/periods.rs` dan dibahas di bab periode; `swat-output` hanya mencetak nilai yang sudah jadi.

**Antarmuka runner (dari `docs/runner.md`).** Engine dijalankan sebagai program terpisah atau modul WebAssembly, tidak pernah ditautkan sebagai pustaka ke pemanggil, karena engine berlisensi LGPL-2.1 sedangkan pemanggil (runner RustGIS) tidak harus (keputusan desain K11 RustGIS). Perintahnya:

- `swatplus-rust run <TxtInOut> [--out <folder>] [--fix-swat] [--format text|parquet|both] [--progress json] [--strict]`. `--out` bawaannya folder TxtInOut itu sendiri (seperti SWAT+). `--format` memilih teks (bawaan), Parquet, atau keduanya; dengan `parquet` saja, berkas `.txt` dihapus setelah dikonversi. `--progress json` mengirim progres dan kejadian sebagai JSON-line ke keluaran standar; tanpa itu pesan pergi ke *standard error*. `--strict` keluar dengan kode 3 bila sebagian proyek tidak disimulasikan, setelah semua tabel ditulis lengkap (juga Parquet).
- Kode keluar: 0 selesai, 1 galat (proyek tidak terbaca/terjalankan atau tabel tidak dapat ditulis), 2 argumen salah, 3 sebagian proyek tidak disimulasikan (`--strict`).
- `swatplus-rust inspect <TxtInOut>`, `swatplus-rust compare <tabel SWAT+> <tabel kita> [--abs 0.001] [--rel 1e-4]`, `swatplus-rust --version`.
- Modul WebAssembly `swat_wasm.wasm` mengekspor `memory`, `swat_alloc(len) -> ptr`, `swat_free(ptr, len)`, dan `swat_run(ptr, len, flags) -> ptr`. Masukan adalah arsip tar berkas TxtInOut (dalam satu folder atau tanpa folder); `flags` 1 = Parquet, 2 = teks (keduanya bila tidak satu pun), 4 = `--fix-swat`. Keluaran: 4 bita panjang *little-endian* lalu arsip tar berisi tabel dan `run.json` (`{"version":…,"ok":…,"error":…,"notes":[…],"tables":[…]}`). Modul mengimpor `env.swat_progress(done, days)` yang dipanggil tiap persen bulat.
- Catatan presisi WebAssembly: di wasm, fungsi matematika (`exp`, `ln`, `powf`, trigonometri) adalah port musl milik Rust, bukan glibc, sehingga beberapa hasil f32 dapat berbeda di bit terakhir. Pada Osu\_1hru selama 14 tahun, 17 dari 18 tabel identik; tabel rata-rata tahunan unit routing (dicetak dengan 9 digit bermakna) berbeda 1 dan 2 ULP f32 pada dua nilainya, masih dalam toleransi (keputusan K12).
- Kecepatan: engine satu *thread*; seluruh Pessel (3.966 HRU, 1.454 kanal) 6,4 detik per tahun simulasi di satu inti, terhadap 16,7 detik untuk SWAT+ gfortran Release. Paralelisasi HRU tidak dapat memberi hasil SWAT+ karena beberapa variabel global membawa nilai satu HRU ke HRU berikutnya (bilangan acak tabel keputusan, permintaan hara tanaman, *enrichment ratio*, fraksi P sedimen, keadaan septik); itu menjadi wilayah mode `--fix-swat` kelak.

**Alur pemanggil (`main.rs`, fungsi `run`).** (1) Muat proyek; (2) susun judul tabel `format!(" {:<24} swatplus-rust {versi}", judul_file.cio)`; (3) bila JSON, pancarkan kejadian `start` dengan tanggal pertama dan terakhir; (4) buat `TextTables::new(&judul, DirStore::new(out), &project.print)`, set `fixes` dan `progress`; (5) `run_with`; (6) pancarkan `note` untuk bagian yang dilewati; (7) `tables.finish()` mengembalikan `DirStore`, yang `names()`-nya memberi daftar berkas; (8) untuk setiap berkas: bila Parquet diminta, buka `.txt`, panggil `text_to_parquet(BufReader, BufWriter, 1 << 20)`, pancarkan `table` dengan jumlah baris, hapus `.txt` bila teks tidak diminta; (9) pancarkan `done` dengan detik berlalu. Modul wasm (`run_inner`) melakukan hal yang sama dengan `MemStore` dan `tar`.

### 15.2  `store.rs`: tempat berkas keluaran ditulis

`store.rs` memisahkan *apa* yang ditulis dari *ke mana* ditulis. `TextTables` hanya mengenal trait `Store` dengan satu operasi, "tambahkan bita ke berkas bernama sekian". Dua implementasinya: `DirStore` (folder di disk, untuk program asli) dan `MemStore` (peta nama ke `Vec<u8>` di memori, untuk WebAssembly di peramban). Tidak ada padanan SWAT+: Fortran membuka unit berkas dengan `open_output_file(unit, nama, recl)` di subrutin header dan menulis langsung ke unit itu dengan `write (unit, fmt)`; penyangga ditangani runtime gfortran.

| Tipe | Field | Keterangan |
| --- | --- | --- |
| `trait Store` | (tidak ada) | Satu method `append`. |
| `struct DirStore` | `dir: PathBuf`, `files: BTreeMap<String, File>` | Folder tujuan dan berkas yang sudah dibuka, terurut menurut nama. |
| `struct MemStore` | `pub files: BTreeMap<String, Vec<u8>>` | Isi berkas di memori, terurut menurut nama; publik agar `swat-wasm` dapat mengiterasinya. |

#### 15.2.1  `Store::append`

- **Lokasi:** `crates/swat-output/src/store.rs`, `fn append(&mut self, name: &str, bytes: &[u8]) -> Result<(), String>` (method trait).
- **Padanan SWAT+:** tidak ada; khusus port (setara konseptual dengan `write (unit, …)` ke unit yang sudah dibuka).
- **Tujuan:** menambahkan `bytes` ke akhir berkas `name`, membuat berkas itu pada panggilan pertama.
- **Masukan dan keluaran:** nama berkas relatif (misalnya `hru_wb_day.txt`), potongan bita; galat sebagai teks yang menyebut jalur berkas.
- **Cara kerja:** kontrak saja; perilaku konkret di dua implementasi di bawah.
- **Catatan port:** kontrak "buat pada panggilan pertama, lalu tambahkan" membuat `TextTables` dapat menulis tabel besar sepotong demi sepotong (penyangga 4 MiB, lihat 15.3) tanpa pernah menyimpan seluruh tabel di memori pada program asli.

#### 15.2.2  `DirStore::append` (impl `Store for DirStore`)

- **Lokasi:** `store.rs`, `fn append(&mut self, name: &str, bytes: &[u8]) -> Result<(), String>`.
- **Padanan SWAT+:** `open_output_file` + `write`; tidak ada padanan langsung.
- **Tujuan:** menulis ke berkas `dir/name` di disk.
- **Masukan dan keluaran:** seperti 15.2.1; berkas di disk sebagai efek samping.
- **Cara kerja:**
  1. Bentuk jalur `path = dir.join(name)`.
  2. Bila `name` belum ada di peta `files`, buat berkas dengan `File::create(&path)` (memotong berkas lama bernama sama) dan simpan pegangan berkasnya di peta. Galat diberi awalan jalur: `"<jalur>: <galat>"`.
  3. Ambil pegangan dari peta dan `write_all(bytes)`. Bila (secara teoretis) pegangan tidak ditemukan, kembalikan `"<jalur>: not open"` (cabang ini ada karena crate melarang `unwrap`).
- **Catatan port:** pegangan berkas tetap terbuka sampai `DirStore` di-*drop*; `main.rs` memanggil `drop(store)` setelah mengambil `names()` dan sebelum membuka kembali berkas teks untuk dikonversi ke Parquet, sehingga semua data sudah ter-*flush* oleh sistem operasi. `File` tidak disangga, tetapi `TextTables` selalu menulis dalam potongan besar sehingga jumlah panggilan sistem kecil.

#### 15.2.3  `MemStore::append` (impl `Store for MemStore`)

- **Lokasi:** `store.rs`, `fn append(&mut self, name: &str, bytes: &[u8]) -> Result<(), String>`.
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** menyimpan berkas di memori, untuk lingkungan tanpa sistem berkas (WebAssembly).
- **Cara kerja:** `files.entry(name).or_default().extend_from_slice(bytes)`; selalu `Ok(())`.
- **Catatan port:** `swat-wasm::run_inner` membaca `store.files` setelah `finish`, mengonversi tiap entri ke Parquet bila diminta, dan memasukkannya ke arsip tar keluaran. Karena peta terurut (`BTreeMap`), urutan berkas di tar deterministik (alfabetis).

#### 15.2.4  Fungsi trivial `store.rs`

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `DirStore::new(dir: impl Into<PathBuf>) -> Self` | `store.rs` | Membuat store untuk folder `dir` dengan peta berkas kosong; folder tidak dibuat di sini (pemanggil memakai `create_dir_all`). |
| `DirStore::dir(&self) -> &Path` | `store.rs` | Mengembalikan folder tujuan. |
| `DirStore::names(&self) -> Vec<String>` | `store.rs` | Nama berkas yang sudah dibuat, terurut alfabetis (urutan kunci `BTreeMap`, meskipun komentar kode menyebut "in order"); dipakai `main.rs` untuk daftar tabel yang dikonversi dan dilaporkan. |
| `MemStore::new() -> Self` | `store.rs` | Sama dengan `MemStore::default()` (peta kosong). |

### 15.3  `text.rs` (1): struktur `TextTables`, konstanta, dan fungsi bantu format

`text.rs` adalah implementasi `OutputSink` yang menulis tabel teks bergaya SWAT+. Setiap "lembar" (*sheet*) adalah satu berkas `<stem>_<periode>.txt` dengan penyangga teksnya sendiri. Penyangga diisi oleh method baris (15.4) yang dipanggil oleh method trait (15.5), lalu dialirkan ke `Store` setiap kali melewati 4 MiB.

**Konstanta modul.**

| Konstanta | Nilai | Kegunaan |
| --- | --- | --- |
| `CHUNK` | `4 << 20` (4 MiB) | Ambang penyangga per berkas sebelum ditulis ke store. Komentar kode: tabel harian proyek besar mencapai gigabita. |
| `SUBDAY_UNITS` | satu baris panjang berisi satuan (`m^3/s`, `tons`, `kg_N`, `kg_P`, `m`, `m/m`, `mm`, `m/s`) | Baris satuan yang ditulis SWAT+ di bawah header `channel_sd_subday.txt`. Menurut komentar kode, itu adalah baris satuan tabel kanal harian, bukan yang semestinya untuk tabel sub-harian; port menyalinnya apa adanya. |
| `KEYS` | `"  jday   mon   day    yr    unit  gis_id  name            "` | Awal header untuk tabel berkunci objek (HRU, LSU, basin, akuifer, kanal, waduk). |
| `WEATHER` | `["tmx", "tmn", "tmpav", "solarad", "wndspd", "rhum"]` | Nama kolom `hru_pw_day` saat HRU tidak disimulasikan (hanya cuaca). |
| `TABLES` | 21 nama: `reservoir`, `basin_res`, `recall`, `basin_psc`, `hru_wb`, `hru_nb`, `hru_ls`, `hru_pw`, `ru`, `aquifer`, `channel_sd`, `basin_wb`, `basin_nb`, `basin_ls`, `basin_pw`, `lsunit_wb`, `lsunit_nb`, `lsunit_ls`, `lsunit_pw`, `basin_aqu`, `basin_sd_cha` | Baris `print.prt` yang dikenali port; bendera cetaknya dibaca di `new`. |

**Struktur data.**

| Tipe / field | Isi |
| --- | --- |
| `struct Sheet` (privat, `Default`) | `buf: String` penyangga teks yang belum ditulis; `written: bool` sudah pernah ditulis ke store; `started: bool` header sudah ditulis ke penyangga. |
| `TextTables<S: Store>.title` | Baris judul tabel (dari `main.rs`/`swat-wasm`: judul `file.cio` rata kiri 24 karakter + `swatplus-rust <versi>`). Padanan Fortran: `write (unit,*) bsn%name, prog`. |
| `.store: S` | Tujuan berkas. |
| `.sheets: BTreeMap<String, Sheet>` | Lembar per nama berkas (misalnya `hru_wb_day.txt`). |
| `.flags: BTreeMap<&'static str, [bool; 4]>` | Per tabel: apakah `print.prt` meminta harian, bulanan, tahunan, rata-rata tahunan. Padanan Fortran: `pco%wb_hru%d`, `%m`, `%y`, `%a`, dst. |
| `.error: Option<String>` | Galat tulis pertama; setelah ada galat, penulisan berikutnya dilewati. |
| `pub fixes: bool` | Mode `--fix-swat`: memperbaiki label kolom akuifer yang salah. |
| `.crop_yld: String` | Bendera `crop_yld` dari `print.prt`: `y` hasil panen basin, `a` per HRU, `b` keduanya. |
| `pub progress: Option<progress::Reporter>` | Pelapor progres JSON (15.9), bila diminta. |

Komentar struct mencatat perilaku saat HRU tidak disimulasikan (fitur di luar cakupan port): tabel neraca air harian hanya berisi presipitasi, tabel tanaman hanya cuaca, dan tabel HRU lain tidak ditulis.

#### 15.3.1  `e15_4`

- **Lokasi:** `text.rs`, `fn e15_4(v: f32) -> String` (privat).
- **Padanan SWAT+:** deskriptor format Fortran `e15.4` pada `sd_channel_output.f90` (format 100: `4i6,2i8,2x,a,73e15.4`; format 101 sub-harian: `4i6,3i8,2x,a,e15.4`) dan `reservoir_output.f90` (format 100: `…,63e15.4`); juga `e14.4` didahului `1x` di `basin_sdchannel_output.f90` dan `basin_reservoir_output.f90`, yang lebarnya sama (15 karakter).
- **Tujuan:** meniru keluaran Fortran `Ew.d` dengan `w = 15`, `d = 4`: mantisa berbentuk `0.dddd`, eksponen dua digit bertanda, rata kanan selebar 15.
- **Masukan dan keluaran:** satu nilai f32; string 15 karakter, misalnya `      0.1234E-01 `.
- **Cara kerja:**
  1. Bila `v` NaN, kembalikan `NaN`; bila tak hingga, `Infinity` atau `-Infinity`; ketiganya rata kanan 15 seperti gfortran (sejak 1.0.1). Bila `v == 0.0` (termasuk `-0.0`), kembalikan `0.0000E+00` rata kanan 15.
  2. Format `|v|` dalam notasi ilmiah Rust dengan 3 desimal: `d.ddde±x` (mantisa ternormalisasi 1 ≤ m < 10, empat digit bermakna).
  3. Pisahkan mantisa dan eksponen di huruf `e`; naikkan eksponen 1, karena Fortran menormalkan ke 0.1 ≤ m < 1.
  4. Ambil keempat digit mantisa (tanpa titik) sebagai `dddd`.
  5. Susun `[-]0.ddddE±ee` dengan eksponen dua digit dan rata kanan 15.

```latex
v = \pm\, d_1.d_2d_3d_4 \times 10^{x} = \pm\, 0.d_1d_2d_3d_4 \times 10^{x+1}
```

- **Catatan port:** pembulatan ke empat digit bermakna dilakukan sekali oleh formatter Rust pada nilai f32 eksak (bukan pada mantisa Fortran-style), jadi digit yang dihasilkan sama dengan pembulatan Fortran untuk semua nilai yang tidak tepat di titik tengah; untuk nilai yang tepat di titik tengah, aturan *tie-breaking* Rust dan gfortran belum diverifikasi di repo. Rentang f32 (|eksponen| ≤ 39) menjamin eksponen selalu dua digit, sehingga kasus Fortran tiga digit (huruf `E` dihilangkan) tidak pernah muncul. Sejak 1.0.1, NaN dan tak hingga dicetak seperti gfortran, rata kanan lebar 15: `NaN`, `Infinity`, `-Infinity`; sebelumnya fungsi ini mencetak `0.E+01`, karena `format!("{:.3e}")` menghasilkan `NaN`/`inf` tanpa huruf `e`. Uji `e15_4_as_gfortran` memeriksa kasus ini. Untuk `-0.0`, port mencetak tanpa tanda.

#### 15.3.2  `TextTables::new`

- **Lokasi:** `text.rs`, `pub fn new(title: &str, store: S, print: &PrintSettings) -> Self`.
- **Padanan SWAT+:** pembacaan bendera di `basin_print_codes_read.f90` (yang mengisi `pco`) digabung dengan pembukaan berkas di `header_write.f90` dan rekan-rekannya.
- **Tujuan:** membuat penulis tabel dengan bendera cetak dari `print.prt`.
- **Masukan dan keluaran:** judul, store, `PrintSettings` (berisi objek per baris `print.prt` dengan field `daily`, `monthly`, `yearly`, `avann` bernilai `"y"`/`"n"`, dan `crop_yld`).
- **Cara kerja:**
  1. Untuk setiap nama di `TABLES`, cari `print.object(name)`. Bila ada, bendera = `[daily == "y", monthly == "y", yearly == "y", avann == "y"]`; bila tidak ada, `[false; 4]`.
  2. Simpan judul, store, peta lembar kosong, bendera, `error = None`, `fixes = false`, salinan `crop_yld`, `progress = None`.
- **Catatan port:** berkas tidak dibuat di sini, melainkan saat baris pertama tabel itu diterima (`sheet`/`file`). SWAT+ membuka semua berkas yang diminta di awal; karena itu `finish` menulis satu baris kosong untuk lembar yang sudah dimulai tetapi tak berisi baris, agar berkasnya tetap ada. Lembar yang sama sekali tidak pernah dimulai (misalnya tabel bulanan sebuah run yang lebih pendek dari sebulan setelah pemanasan) tidak dibuat.

#### 15.3.3  `TextTables::wants`

- **Lokasi:** `text.rs`, `fn wants(&self, table: &str, p: Option<Period>) -> bool`.
- **Padanan SWAT+:** uji seperti `if (pco%wb_hru%d == "y")` di `hru_output.f90`.
- **Tujuan:** apakah `print.prt` meminta tabel `table` untuk periode `p` (`None` = harian).
- **Cara kerja:** `flags.get(table)` lalu indeks `period_index(p)`; tabel yang tidak dikenal memberi `false`.
- **Catatan port:** uji jendela dan interval cetak harian (`pco%day_print`, `pco%int_day_cur == pco%int_day`) dilakukan di engine sebelum memanggil sink, bukan di sini.

#### 15.3.4  `TextTables::file`

- **Lokasi:** `text.rs`, `fn file(&mut self, name: &str, header: impl FnOnce(&mut String)) -> &mut String`.
- **Padanan SWAT+:** `open_output_file` + penulisan judul dan header di subrutin `header_*`.
- **Tujuan:** mengembalikan penyangga berkas `name` (nama lengkap, misalnya `crop_yld_aa.txt`), menulis header lewat closure `header` pada pemakaian pertama.
- **Cara kerja:** `sheets.entry(name).or_default()`; bila `started` masih `false`, set `true` dan panggil `header(&mut buf)`; kembalikan `&mut buf`.
- **Catatan port:** dipakai untuk berkas yang namanya tidak mengikuti pola `<stem>_<periode>.txt` (`channel_sd_subday.txt`, `crop_yld_aa.txt`, `basin_crop_yld_<periode>.txt`).

#### 15.3.5  `TextTables::sheet`

- **Lokasi:** `text.rs`, `fn sheet(&mut self, stem: &str, p: Option<Period>, header: impl FnOnce(&mut String)) -> &mut String`.
- **Padanan SWAT+:** sama dengan 15.3.4.
- **Tujuan:** seperti `file`, dengan nama berkas dibentuk dari `stem` dan periode: `format!("{stem}_{}.txt", suffix(p))`, jadi `hru_wb_day.txt`, `hru_wb_mon.txt`, `hru_wb_yr.txt`, `hru_wb_aa.txt`.
- **Cara kerja:** identik dengan `file` setelah membentuk kunci.

#### 15.3.6  `TextTables::flush`

- **Lokasi:** `text.rs`, `fn flush(&mut self, limit: usize)`.
- **Padanan SWAT+:** tidak ada (penyanggaan runtime gfortran).
- **Tujuan:** menulis ke store setiap penyangga yang panjangnya melebihi `limit` bita.
- **Cara kerja:**
  1. Untuk setiap lembar (urut nama): lewati bila `buf.len() <= limit` atau sudah ada galat.
  2. `store.append(name, buf)`; bila gagal, simpan galat di `self.error` (galat pertama saja yang tersimpan karena lembar berikutnya dilewati).
  3. Set `written = true` dan kosongkan `buf`.
- **Catatan port:** dipanggil dengan `CHUNK` setelah setiap baris harian (HRU, RU, kanal, akuifer, region, waduk harian) dan dengan `0` di `finish`. Baris periode bulanan/tahunan untuk RU, kanal, akuifer, basin\_aqu, basin\_sd\_cha, basin\_res, recall, dan basin\_psc tidak memicu `flush` (volumenya kecil); penyangganya ikut tertulis pada `flush` berikutnya atau di `finish`. Catatan teknis: pada `limit = 0`, penyangga kosong yang sudah `written` dilewati karena `0 <= 0`.

#### 15.3.7  `TextTables::finish`

- **Lokasi:** `text.rs`, `pub fn finish(mut self) -> Result<S, String>`.
- **Padanan SWAT+:** penutupan berkas di akhir run (`close` unit-unit keluaran).
- **Tujuan:** menulis sisa penyangga dan mengembalikan store.
- **Cara kerja:**
  1. Untuk setiap lembar yang penyangganya kosong dan belum pernah ditulis, tambahkan satu `\n` (komentar kode: tabel dengan header tetapi tanpa baris tetap ditulis). Karena header ditulis ke penyangga saat lembar dimulai, kasus ini praktis hanya untuk lembar yang closure header-nya tidak menulis apa pun.
  2. `flush(0)` menulis semua penyangga yang tidak kosong.
  3. Kembalikan `Err(galat)` bila ada galat tulis (pesannya menyebut berkas), selain itu `Ok(store)`.
- **Catatan port:** `main.rs` memakai store yang dikembalikan untuk mendapatkan daftar berkas (`DirStore::names`); `swat-wasm` memakai `MemStore::files` langsung.

#### 15.3.8  `header`

- **Lokasi:** `text.rs`, `fn header(out: &mut String, title: &str, names: &[&str])` (fungsi bebas, privat).
- **Padanan SWAT+:** header HRU/LSU/basin di `header_write.f90` dan `output_landscape_init.f90` (`write (unit,*) bsn%name, prog`, lalu `write (unit,*) hdr`, lalu baris satuan).
- **Tujuan:** menulis tiga baris pembuka tabel HRU, LSU, dan basin.
- **Cara kerja:**
  1. Baris 1: judul.
  2. Baris 2: `KEYS`, lalu setiap nama kolom diberi satu spasi dan rata kanan 11 karakter (total 12, selaras dengan `f12.3`).
  3. Baris 3: 58 spasi (pengganti baris satuan SWAT+).
- **Catatan port:** port tidak mencetak satuan kolom HRU; baris ketiga hanya menjaga agar struktur "judul, header, satuan, data" yang diharapkan pembaca SWAT+ (termasuk `compare` dan `text_to_parquet`) tetap sama.

#### 15.3.9  `keys`

- **Lokasi:** `text.rs`, `fn keys(out: &mut String, d: Day, hru: usize, gis_id: i32, name: &str)`.
- **Padanan SWAT+:** bagian `4i6,2i8,2x,a` dari format 100/101/104/108 `hru_output.f90`, format 100 `aquifer_output.f90`, `sd_channel_output.f90`.
- **Tujuan:** menulis kolom kunci baris: `jday`, `mon`, `day`, `yr` (masing-masing lebar 6), nomor unit dan `gis_id` (lebar 8), dua spasi, nama rata kiri 16.
- **Masukan:** `Day { jday, mon, day, yr }` (hari Julian, bulan, hari dalam bulan, tahun kalender, padanan `time%day`, `time%mo`, `time%day_mo`, `time%yrc`).
- **Catatan port:** beberapa tabel Fortran memakai lebar kunci lain (misalnya `reservoir_output.f90` format 100 memakai `2i10`; `lsu_output.f90` memakai `1x,4i6,i7,i8`; `basin_output.f90` mencetak unit dan gis\_id sebagai teks `2a`); port memakai `{:6}×4{:8}×2` untuk semuanya. Karena pembanding dan konverter Parquet memisahkan kolom berdasarkan spasi, perbedaan lebar ini tidak memengaruhi nilai.

#### 15.3.10  `row`

- **Lokasi:** `text.rs`, `fn row(out: &mut String, values: impl IntoIterator<Item = f32>)`.
- **Padanan SWAT+:** bagian `42f12.3` / `25f12.3` dari format `hru_output.f90`, `basin_output.f90`, `lsu_output.f90`.
- **Tujuan:** menulis nilai-nilai f32 dengan format Fortran `f12.3` (lebar 12, tiga desimal) lalu baris baru.
- **Catatan port:** `{v:12.3}` Rust identik dengan `f12.3` untuk nilai yang muat; bila tidak muat, Fortran mencetak 12 tanda bintang sedangkan Rust melebarkan kolom. Format Fortran untuk tabel hara (`hru_output.f90` format 104: `4f12.3,15f17.3`) dan untuk `hru_ls` rata-rata tahunan (format 108: `12(1x,f16.3)`) memakai lebar lain; port memakai `f12.3` untuk semua kolom HRU/LSU/basin. Nilai tiga desimalnya sama, hanya spasi pemisah yang berbeda. Kolom teks penutup baris HRU di SWAT+ (`lum(ilu)%plant_cov`, `lum(ilu)%mgt_ops`, format `3x,a16,a30`) tidak ditulis port.

#### 15.3.11  Fungsi trivial bagian ini

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `period_index(p: Option<Period>) -> usize` | `text.rs` | `None` → 0 (harian), `Mon` → 1, `Yr` → 2, `Aa` → 3; indeks ke `[bool; 4]` bendera. |
| `suffix(p: Option<Period>) -> &'static str` | `text.rs` | `None` → `"day"`, selain itu `Period::suffix()` (`"mon"`, `"yr"`, `"aa"`, didefinisikan di `swat-engine/src/periods.rs`). |
| `TextTables::names(&self) -> Vec<String>` | `text.rs` | Nama berkas yang sudah ditulis atau masih di penyangga, terurut alfabetis. |

### 15.4  `text.rs` (2): penulis baris per jenis tabel

Lima method privat `TextTables` menulis baris untuk lima keluarga tabel. Semuanya mengikuti pola yang sama: (1) uji `wants(stem, p)`; (2) salin judul (karena closure header meminjam `self` secara terpisah); (3) ambil penyangga lewat `sheet(stem, p, header)`, yang menulis header pada baris pertama; (4) tulis kunci dan nilai. Tabel di bawah meringkas format setiap keluarga dan padanan format Fortran-nya.

| Keluarga | Method port | Format nilai port | Format Fortran (berkas: label) |
| --- | --- | --- | --- |
| HRU, LSU, basin (`_wb`, `_nb`, `_ls`, `_pw`) | `hru_rows` | `f12.3` | `hru_output.f90`: 100 `42f12.3`, 101 `25f12.3`, 104 `4f12.3,15f17.3`, 108 `12(1x,f16.3)`; `basin_output.f90`: 100/104; `lsu_output.f90`: 100/102/103 |
| Unit routing `ru_*` | `ru_row` | ilmiah 9 digit bermakna, lebar 15 | `ru_output.f90`: *list-directed* `write (2600,*)` |
| Kanal, waduk (`channel_sd`, `basin_sd_cha`, `reservoir`, `basin_res`) | `cha_row` | `e15.4` (`e15_4`), luas `f16.4` untuk basin | `sd_channel_output.f90` 100 `73e15.4`; `basin_sdchannel_output.f90` 100 `f16.4,59(1x,e14.4)`; `reservoir_output.f90` 100 `63e15.4`; `basin_reservoir_output.f90` 100 `f14.4,59(1x,e14.4)` |
| Recall dan titik sumber basin (`recall`, `basin_psc`) | `rec_row` | eksponensial terpendek Rust | `recall_output.f90`, `basin_recall_output.f90`: *list-directed* |
| Akuifer (`aquifer`, `basin_aqu`) | `aqu_row` | `f15.3` | `aquifer_output.f90` 100 `20f15.3`; `basin_aquifer_output.f90` 100 `17f15.3` |

#### 15.4.1  `TextTables::hru_rows`

- **Lokasi:** `text.rs`, `fn hru_rows(&mut self, prefix: &str, p: Option<Period>, d: Day, hru: usize, gis_id: i32, name: &str, r: &HruRows, nutrients: bool)`.
- **Padanan SWAT+:** `hru_output.f90` (prefix `hru`), `lsu_output.f90` (`lsunit`), `basin_output.f90` (`basin`), masing-masing untuk empat tabel: neraca air (`hwb_d/m/y/a`), neraca hara (`hnb_*`), kehilangan dari lanskap (`hls_*`), tanaman dan cuaca (`hpw_*`).
- **Tujuan:** menulis satu baris ke setiap tabel `<prefix>_wb`, `_pw`, `_nb`, `_ls` yang diminta untuk periode `p`.
- **Masukan dan keluaran:** `HruRows { wb, nb, ls, pw }`, larik nilai f32 yang sudah dihitung oleh engine (fluks dalam mm, kg/ha, t/ha; keadaan dalam mm, °C, MJ/m², dst., sesuai satuan tabel SWAT+); `nutrients` menandai apakah siklus hara disimulasikan.
- **Cara kerja:**
  1. Nama kolom `wb`, `nb`, `ls` diambil dari `HruWb::default().columns()`, `HruNb`, `HruLs` (32, 15, dan 13 kolom; definisi di `swat-engine/src/hru/mod.rs`).
  2. `_wb`: bila diminta, header dengan nama kolom wb, lalu `keys` dan `row(r.wb)`.
  3. `_pw`: pasangan `(PW_NAMES[k], r.pw[k])` (25 kolom: `lai`, `bioms`, `yield`, `residue`, `sol_tmp`, tekanan `strsw`…`strss`, `nplt`, `percn`, `pplnt`, cuaca `tmx`…`rhum`, `phubas0`, `lai_max`, `bm_max`, `bm_grow`, `c_gro`); bila `nutrients` palsu, kolom `percn` dibuang dari header maupun nilai.
  4. `_nb` dan `_ls`: hanya bila `nutrients` benar dan diminta.
- **Catatan port:** header ditulis sekali per berkas, jadi nama kolom `_pw` ditentukan oleh nilai `nutrients` pada baris pertama. Dibuangnya `percn` saat hara tidak disimulasikan dan tidak ditulisnya `_nb`/`_ls` adalah keputusan port: kolom yang tidak dihitung port tidak dicetak (komentar `docs/outputs.md`: "dengan kolom yang dihitung port"), sehingga `compare` hanya membandingkan kolom yang ada di berkas port.

#### 15.4.2  `TextTables::ru_row`

- **Lokasi:** `text.rs`, `fn ru_row(&mut self, p: Option<Period>, d: Day, name: &str, hd: &Hyd)`.
- **Padanan SWAT+:** `ru_output.f90` (`write (2600,*) time%day, time%mo, time%day_mo, time%yrc, ob(iob)%name, ob(iob)%typ, ru_d(iru)` dan unit 2601–2603 untuk periode), header dari `header_write.f90` (`hyd_hdr`).
- **Tujuan:** menulis hidrograf yang diteruskan unit routing (`ru_d`, jumlah kelima hidrograf menurut `docs/swat-quirks.md`).
- **Masukan dan keluaran:** `Hyd` dengan 18 kolom: `flo` (m³), `sed` (t), `orgn`, `sedp`, `no3`, `solp`, `chla`, `nh3`, `no2`, `cbod`, `dox` (kg), `san`, `sil`, `cla`, `sag`, `lag`, `grv` (t), dan suhu berlabel `null`.
- **Cara kerja:**
  1. Header: judul; lalu `jday`, `mon`, `day`, `yr` (lebar 6), dua spasi, `name` rata kiri 16, `type` lebar 6, dan setiap nama kolom `Hyd::columns()` lebar 15 didahului spasi; lalu baris kosong (pengganti baris satuan).
  2. Baris: tanggal (lebar 6), nama, `ru`, lalu setiap nilai sebagai `{v:15.8e}` didahului spasi.
- **Catatan port:** SWAT+ menulis tabel ini *list-directed*, sehingga gfortran memilih sendiri bentuk angkanya; port memakai notasi ilmiah Rust dengan 8 desimal (9 digit bermakna), misalnya `1.23456789e-3`. Bentuk tulisan berbeda dari gfortran, tetapi presisinya setara, dan nilainya dibaca ulang secara numerik oleh `compare` dan `text_to_parquet`. `docs/runner.md` menyebut tabel ini ("dicetak dengan 9 digit bermakna") sebagai satu-satunya tabel Osu\_1hru yang berbeda di bit terakhir antara wasm dan program asli. Kolom `unit`/`gis_id` tidak ada; `compare.rs` menomori baris dalam satu hari sebagai gantinya. Baris periode tidak memicu `flush`.

#### 15.4.3  `TextTables::cha_row`

- **Lokasi:** `text.rs`, `fn cha_row(&mut self, stem: &str, p: Option<Period>, d: Day, ich: usize, gis_id: i32, name: &str, w: &ChWat, stor: &Hyd, hin: &Hyd, hout: &Hyd, wtemp: Option<f32>, area_fixed: bool)`.
- **Padanan SWAT+:** `sd_channel_output.f90` (`channel_sd_*`), `basin_sdchannel_output.f90` (`basin_sd_cha_*`), `reservoir_output.f90` (`reservoir_*`), `basin_reservoir_output.f90` (`basin_res_*`); header dari `header_sd_channel.f90` dan `header_reservoir.f90`.
- **Tujuan:** menulis satu baris tabel badan air: neraca air permukaan, lalu tiga hidrograf (simpanan, masuk, keluar), lalu suhu air.
- **Masukan dan keluaran:** `ChWat { area_ha (ha), precip, evap, seep (m³) }`; `stor`, `hin`, `hout` masing-masing `Hyd` 18 kolom; `wtemp` suhu air (°C) hanya untuk `channel_sd`; `area_fixed` untuk tabel basin.
- **Cara kerja:**
  1. Header: judul; `KEYS`; `area`, `precip`, `evap`, `seep` (lebar 14 + spasi); untuk akhiran `_stor`, `_in`, `_out` berturut-turut, setiap nama kolom `Hyd` diberi akhiran itu kecuali `null` yang tetap `null`; `water_temp` bila `wtemp` ada; baris kosong.
  2. Baris: `keys`; luas sebagai `{:16.4}` (fixed) bila `area_fixed`, selain itu `e15_4`; `precip`, `evap`, `seep` dengan `e15_4`; 3 × 18 nilai hidrograf dengan `e15_4`; `wtemp` bila ada; baris baru.
- **Catatan port:** komentar kode: `basin_sdchannel_output` dan `basin_reservoir_output` mencetak luas dalam titik tetap (`f16.4` dan `f14.4` di Fortran; port memakai `f16.4` untuk keduanya). Karena kolom `null` muncul tiga kali, header berisi nama ganda; `compare.rs` membedakannya dengan urutan (`null#2`, `null#3`) dan `ParquetWriter::new` dengan akhiran (`null_2`, `null_3`). Repetisi `73e15.4`/`63e15.4` di Fortran hanya batas atas; jumlah nilai sebenarnya ditentukan daftar `write`.

#### 15.4.4  `TextTables::rec_row`

- **Lokasi:** `text.rs`, `fn rec_row(&mut self, stem: &str, p: Option<Period>, d: Day, name: &str, typ: Option<&str>, h: &Hyd)`.
- **Padanan SWAT+:** `recall_output.f90` (`write (4600,*) …, ob(iob)%name, ob(iob)%typ, rec_d(irec)`) dan `basin_recall_output.f90` (`write (4500,*) …, bsn%name, brec_d`), header `header_write.f90` (`rec_hdr_time`, `hyd_hdr`).
- **Tujuan:** menulis beban *recall* (titik sumber yang dibaca dari berkas) per objek atau dijumlahkan untuk basin (`basin_psc`).
- **Cara kerja:**
  1. Header baris 1: judul. Baris 2: `jday`, `mon`, `day`, `yr` lebar 12, spasi, `name` rata kiri 16, `type` rata kiri 8 bila ada, nama kolom `Hyd` lebar 14 + spasi.
  2. Baris 3 (satuan): 65 spasi lalu 18 satuan lebar 14: `m^3/s`, `tons`, `kgN`, `kgP`, `kgN`, `kgP`, `kg`, `kgN`, `kgN`, `kg`, `kg`, enam `tons`, `degc`.
  3. Baris data: tanggal lebar 12, nama, tipe (`recall` untuk tabel per objek), lalu setiap nilai sebagai `{v:>14e}` didahului spasi.
- **Catatan port:** komentar kode menandai tabel ini *list-directed*: lebar 12 untuk bilangan bulat meniru keluaran list-directed gfortran untuk `integer(4)`. Nilai ditulis dengan format eksponensial **terpendek yang dapat dibaca ulang tepat** (`{:e}` Rust, misalnya `1.5e0`, `0e0`), sehingga nilai f32 internal tersimpan utuh. Berbeda dari tabel lain, baris satuan berisi satuan sungguhan (meniru header SWAT+), dan baris satuan itu dimulai dengan spasi lalu teks, sehingga dilewati oleh `text_to_parquet`.

#### 15.4.5  `TextTables::aqu_row`

- **Lokasi:** `text.rs`, `fn aqu_row(&mut self, stem: &str, p: Option<Period>, d: Day, iaq: usize, gis_id: i32, name: &str, a: &AquDay)`.
- **Padanan SWAT+:** `aquifer_output.f90` (format 100 `4i6,2i8,2x,a,20f15.3`) dan `basin_aquifer_output.f90` (format 100 `4i6,a,2x,a,2x,a16,17f15.3`); header dari `header_aquifer.f90` (`aqu_hdr`).
- **Tujuan:** menulis keadaan dan fluks harian/periode satu akuifer atau gabungan basin.
- **Masukan dan keluaran:** `AquDay` dengan 17 kolom: `flo`, `dep_wt` (m), `stor`, `rchrg`, `seep`, `revap` (mm), `no3_st`, `minp`, karbon (`cbn`), `orgn`, `no3_rchg`, `no3_loss`, `no3_lat`, `no3_seep` (kg/ha), `flo_cha`, `flo_res`, `flo_ls` (mm).
- **Cara kerja:**
  1. Header: judul, `KEYS`, nama kolom dari `a.columns_as(self.fixes)` lebar 14 + spasi, baris kosong.
  2. Baris: `keys`, lalu nilai dari `a.columns()` dengan `{v:15.3}`.
- **Kebiasaan SWAT+ / catatan port:** header SWAT+ (`aqu_hdr`) memberi label `orgn` dan `orgp` pada dua kolom yang sebenarnya berisi karbon dan N organik (`aquifer_dynamic`). Port mempertahankan label yang salah ini secara bawaan; dengan `--fix-swat` (`fixes = true`) labelnya menjadi `cbn` dan `orgn`. Hanya label yang berubah: urutan dan nilai kolom sama di kedua mode.

### 15.5  `text.rs` (3): implementasi `OutputSink for TextTables<S>`

Trait `OutputSink` didefinisikan di `swat-engine/src/run.rs`; semua method-nya kecuali `hru_day` punya implementasi bawaan kosong, sehingga sink lain (misalnya sink uji) hanya perlu mengisi yang diperlukan. `TextTables` mengisi kesembilan belas method. Engine memanggilnya hanya untuk hari yang dicetak (jendela dan interval `print.prt`) dan untuk periode setelah tahun pemanasan; `TextTables` tinggal memeriksa bendera tabel. Tipe `Day { jday, mon, day, yr }` membawa tanggal; `Period` adalah `Mon`, `Yr`, atau `Aa`; `Region` adalah `Basin` atau `Lsu`.

#### 15.5.1  `OutputSink::progress`

- **Lokasi:** `text.rs`, `fn progress(&mut self, done: usize, total: usize, day: Day)`.
- **Padanan SWAT+:** tidak ada di berkas keluaran; SWAT+ mencetak tanggal yang sedang disimulasikan ke layar (bukan bagian bab ini).
- **Tujuan:** meneruskan progres simulasi ke `Reporter` bila terpasang.
- **Masukan:** `done` hari yang sudah selesai (sama dengan `total` setelah hari terakhir), `total` hari, `day` hari yang mulai disimulasikan.
- **Cara kerja:** `if let Some(p) = self.progress.as_mut() { p.day(done, total, day) }`.

#### 15.5.2  `OutputSink::hru_day`

- **Lokasi:** `text.rs`, `fn hru_day(&mut self, d: Day, hru: usize, gis_id: i32, name: &str, w: &HruWeather, h: Option<&HruDay>)`.
- **Padanan SWAT+:** cabang harian (`pco%day_print == "y"`) dari `hru_output.f90`, unit 2000 (wb), 2020 (nb), 2030 (ls), 2040 (pw).
- **Tujuan:** menulis baris harian keempat tabel HRU.
- **Masukan dan keluaran:** cuaca HRU (`tmax`, `tmin`, `tave` °C; `solrad` MJ/m²; `windsp` m/s; `rhum` fraksi; `precip` mm); hasil hari HRU (`HruDay`) bila HRU disimulasikan.
- **Cara kerja:**
  1. Susun larik cuaca `[tmax, tmin, tave, solrad, windsp, rhum]`.
  2. Bila `h` ada: `hru_values(h, cuaca)` (di `swat-engine/src/periods.rs`) menghasilkan `(wb, nb, ls, pw)`; tulis lewat `hru_rows("hru", None, …, h.nutrients)`.
  3. Bila `h` kosong (HRU di luar cakupan port): bila `hru_wb` harian diminta, tulis header dengan satu kolom `precip` dan nilai presipitasi; bila `hru_pw` harian diminta, tulis header `WEATHER` dan enam nilai cuaca. Tabel `nb` dan `ls` tidak ditulis.
  4. `flush(CHUNK)`.
- **Catatan port:** mode degradasi (langkah 3) membuat run tetap menghasilkan tabel yang berguna saat neraca air HRU tidak disimulasikan; `main.rs` memancarkan `note` "hru\_wb\_day.txt has precipitation only".

#### 15.5.3  `OutputSink::hru_period`

- **Lokasi:** `text.rs`, `fn hru_period(&mut self, p: Period, d: Day, hru: usize, gis_id: i32, name: &str, rows: &HruRows, nutrients: bool)`.
- **Padanan SWAT+:** cabang bulanan (unit 2001/2021/2031/2041), tahunan (2002/…), dan rata-rata tahunan (2003/…) di `hru_output.f90`, dengan `hwb_m`, `hwb_y`, `hwb_a`, dst.
- **Tujuan:** menulis baris periode HRU (dipanggil pada hari terakhir periode, dengan nilai yang sudah dijumlahkan/dirata-ratakan engine).
- **Cara kerja:** `hru_rows("hru", Some(p), …)` lalu `flush(CHUNK)`.

#### 15.5.4  `OutputSink::ru_day` dan `ru_period`

- **Lokasi:** `text.rs`, `fn ru_day(&mut self, d: Day, name: &str, hd: &Hyd)`; `fn ru_period(&mut self, p: Period, d: Day, name: &str, hd: &Hyd)`.
- **Padanan SWAT+:** `ru_output.f90` (unit 2600 harian; 2601–2603 periode).
- **Tujuan:** baris harian/periode unit routing.
- **Cara kerja:** `ru_row(None|Some(p), …)`; versi harian lalu `flush(CHUNK)`, versi periode tidak.

#### 15.5.5  `OutputSink::cha_day` dan `cha_period`

- **Lokasi:** `text.rs`, `fn cha_day(&mut self, d, ich, gis_id, name, w: &ChWat, stor: &Hyd, hin: &Hyd, hout: &Hyd, wtemp: f32)`; `cha_period` sama dengan tambahan `p: Period`.
- **Padanan SWAT+:** `sd_channel_output.f90` (format 100).
- **Tujuan:** baris kanal SWAT-DEG harian/periode.
- **Cara kerja:** `cha_row("channel_sd", None|Some(p), …, Some(wtemp), false)`; versi harian lalu `flush(CHUNK)`.
- **Catatan port:** menurut dokumentasi trait, pada periode simpanan dan suhu air adalah nilai hari terakhir, aliran dirata-ratakan, beban dijumlahkan (aturan di `periods.rs`, lihat `docs/outputs.md`).

#### 15.5.6  `OutputSink::cha_subday`

- **Lokasi:** `text.rs`, `fn cha_subday(&mut self, d: Day, ich: usize, gis_id: i32, name: &str, flo: &[f32])`.
- **Padanan SWAT+:** `sd_channel_output.f90`, `write (2508,101)` dengan format 101 `4i6,3i8,2x,a,e15.4`, satu baris per langkah waktu `ii`; header dari `header_sd_channel.f90` (`sdch_hdr_subday`, baris satuan).
- **Tujuan:** menulis aliran keluar kanal per langkah waktu sub-harian ke `channel_sd_subday.txt`.
- **Masukan dan keluaran:** `flo[ii]` aliran keluar langkah `ii` (komentar trait: dalam m³, meskipun header SWAT+ menyebut m^3/s).
- **Cara kerja:**
  1. Hanya bila `channel_sd` harian diminta.
  2. Penyangga `file("channel_sd_subday.txt")`; header: judul, baris header tetap `jday mon day yr unit gis_id tstep name flo_out`, baris `SUBDAY_UNITS`.
  3. Untuk setiap `ii`: `jday`, `mon`, `day`, `yr` (lebar 6), `ich`, `gis_id`, `ii + 1` (lebar 8), nama rata kiri 16, `e15_4(flo[ii])`.
- **Kebiasaan SWAT+ / catatan port:** baris satuan dan label `m^3/s` disalin dari SWAT+ walaupun tidak sesuai (lihat konstanta `SUBDAY_UNITS`). Method ini tidak memanggil `flush`; penyangga ikut tertulis pada `flush` yang dipicu baris harian berikutnya.

#### 15.5.7  `OutputSink::aqu_day` dan `aqu_period`

- **Lokasi:** `text.rs`, `fn aqu_day(&mut self, d, iaq, gis_id, name, a: &AquDay)`; `aqu_period` dengan tambahan `p`.
- **Padanan SWAT+:** `aquifer_output.f90`.
- **Cara kerja:** `aqu_row("aquifer", None|Some(p), …)`; versi harian lalu `flush(CHUNK)`.

#### 15.5.8  `OutputSink::region`

- **Lokasi:** `text.rs`, `fn region(&mut self, r: Region, p: Option<Period>, d: Day, unit: usize, gis_id: i32, name: &str, rows: &HruRows)`.
- **Padanan SWAT+:** `basin_output.f90` (`Region::Basin`, tabel `basin_wb/nb/ls/pw`) dan `lsu_output.f90` (`Region::Lsu`, tabel `lsunit_*`).
- **Tujuan:** baris harian atau periode untuk basin atau unit lanskap.
- **Cara kerja:** prefix `basin` atau `lsunit`; `hru_rows(prefix, p, …, rows, true)`; `flush(CHUNK)`.
- **Catatan port:** `nutrients` selalu `true` untuk region, jadi keempat tabel ditulis bila diminta dan `percn` selalu ada di `_pw`. Pembobotan HRU (`bsn_frac`, `sub_frac` dari `ls_unit.ele`) dilakukan engine.

#### 15.5.9  `OutputSink::crop_yld_aa`

- **Lokasi:** `text.rs`, `fn crop_yld_aa(&mut self, d: Day, hru: usize, plant: &str, y: [f32; 4])`.
- **Padanan SWAT+:** `hru_output.f90`, `write (4008,103)` dengan format 103 `4i6,i8,4x,a,5x,4f12.3`; header oleh `output_landscape_init.f90` (judul list-directed lalu format 1000 yang memuat `--YIELD (kg/ha)--`).
- **Tujuan:** hasil panen rata-rata per panen untuk setiap tanaman setiap HRU, ditulis pada hari terakhir.
- **Masukan dan keluaran:** `y = [massa, C, N, P]` dalam kg/ha; nama tanaman (`plantnm`, hingga 40 karakter).
- **Cara kerja:**
  1. Hanya bila `crop_yld` bernilai `a` atau `b`.
  2. Header: judul; `--YIELD (kg/ha)--` rata kanan 93; baris `jday mon day yr unit PLANTNM MASS C N P`.
  3. Baris: tanggal (lebar 6), HRU (lebar 8), 4 spasi, nama rata kiri 40, 5 spasi, empat nilai `f12.3`.
- **Catatan port:** `docs/outputs.md`: hasil rata-rata tahunan tanaman HRU adalah jumlah hasil sepanjang run, *termasuk tahun pemanasan*, dibagi jumlah panen (kebiasaan SWAT+ yang dipertahankan engine).

#### 15.5.10  `OutputSink::basin_crop_yld`

- **Lokasi:** `text.rs`, `fn basin_crop_yld(&mut self, p: Period, yrc: i32, iplt: usize, plant: &str, area: f32, yld: f32, t_ha: f32)`.
- **Padanan SWAT+:** `time_control.f90`, `write (5100,*) time%yrc, iplt, plts_bsn(iplt), bsn_crop_yld(iplt)%area_ha, …` (tahunan) dan `write (5101,*)` (rata-rata tahunan); header `header_yield.f90` (`bsn_yld_hdr`).
- **Tujuan:** hasil panen basin per tanaman: luas panen (ha), hasil (t), dan t/ha.
- **Cara kerja:**
  1. Hanya bila `crop_yld` bernilai `y` atau `b`.
  2. Berkas `basin_crop_yld_<periode>.txt`; header: judul, lalu baris `year plant_no plant_name harv_area(ha) yld(t) yld(t/ha)`.
  3. Baris: `{yrc:12}{iplt:12} {plant:<40}{area:17.9}{yld:17.9}{t_ha:17.9}`.
- **Catatan port:** SWAT+ menulis *list-directed*; port meniru tata letaknya dengan lebar tetap (bilangan bulat lebar 12) dan menulis nilai dengan 9 desimal titik tetap. Header berawalan `year`, yang dikenali `text_to_parquet` sebagai baris nama kolom.

#### 15.5.11  `OutputSink::basin_aqu`, `basin_cha`, `res`, `basin_res`

- **Lokasi:** `text.rs`. `fn basin_aqu(&mut self, p: Option<Period>, d, name, a: &AquDay)`; `fn basin_cha(&mut self, p: Option<Period>, d, name, w, stor, hin, hout)`; `fn res(&mut self, p: Option<Period>, d, j: usize, gis_id, name, w, stor, hin, hout)`; `fn basin_res(&mut self, p: Option<Period>, d, unit: usize, name, w, stor, hin, hout)`.
- **Padanan SWAT+:** `basin_aquifer_output.f90`, `basin_sdchannel_output.f90`, `reservoir_output.f90`, `basin_reservoir_output.f90`.
- **Tujuan:** baris tabel akuifer basin, kanal basin, waduk, dan waduk basin (harian bila `p` kosong, periode bila tidak).
- **Cara kerja:**
  - `basin_aqu` → `aqu_row("basin_aqu", p, d, 1, 1, name, a)` (unit dan gis\_id tetap 1).
  - `basin_cha` → `cha_row("basin_sd_cha", p, d, 1, 1, …, None, true)` (tanpa suhu air, luas titik tetap).
  - `res` → `cha_row("reservoir", p, d, j, gis_id, …, None, false)`; `flush(CHUNK)` hanya untuk baris harian.
  - `basin_res` → `cha_row("basin_res", p, d, unit, 1, …, None, true)`.
- **Catatan port:** keempatnya tidak memanggil `flush` kecuali `res` harian; volume tabel basin kecil (satu baris per hari).

#### 15.5.12  `OutputSink::recall` dan `basin_psc`

- **Lokasi:** `text.rs`, `fn recall(&mut self, p: Option<Period>, d: Day, name: &str, h: &Hyd)`; `fn basin_psc(&mut self, p: Option<Period>, d: Day, name: &str, h: &Hyd)`.
- **Padanan SWAT+:** `recall_output.f90` (unit 4600–4603) dan `basin_recall_output.f90` (unit 4500–4503, berkas `basin_psc_*.txt` menurut `header_write.f90`).
- **Cara kerja:** `rec_row("recall", p, d, name, Some("recall"), h)` dan `rec_row("basin_psc", p, d, name, None, h)` (tabel basin tanpa kolom tipe).

### 15.6  `parquet.rs` (1): format Parquet yang ditulis tangan, kolom, dan protokol Thrift *compact*

`parquet.rs` adalah penulis Parquet minimal tanpa dependensi. Menurut komentar modul, ia hanya mendukung tabel datar berisi kolom wajib (*required*) bertipe bilangan bulat 32-bit, float 32-bit, dan string UTF-8, dengan *encoding* PLAIN, tanpa kompresi, satu *data page* per *column chunk*; cukup untuk tabel keluaran, dan dapat dibaca pembaca Parquet mana pun (DuckDB, Arrow, Polars, DuckDB-Wasm). CI memeriksa setiap tabel Osu\_1hru dengan DuckDB dan pyarrow terhadap teksnya (`scripts/check-parquet.py`, menurut `docs/runner.md`). Metadata berkas ditulis dengan protokol *compact* Apache Thrift yang juga ditulis tangan. Tidak ada padanan di SWAT+ Fortran.

**Tata letak berkas yang dihasilkan.**

```text
"PAR1"                                   4 bita magic
[row group 1]
  [column chunk kolom 1]  PageHeader (Thrift compact) + data PLAIN
  [column chunk kolom 2]  PageHeader + data PLAIN
  ...
[row group 2] ...
FileMetaData (Thrift compact)
panjang FileMetaData                     4 bita, u32 little-endian
"PAR1"                                   4 bita magic
```

Karena semua kolom `REQUIRED` dan datar (tidak bersarang), *definition level* dan *repetition level* maksimum adalah 0, sehingga halaman data tidak memuat level sama sekali: isinya langsung nilai PLAIN. Tiap *column chunk* hanya berisi satu halaman (tanpa *dictionary page*), sehingga `data_page_offset` = awal chunk.

**Struktur Thrift yang ditulis** (nomor field sesuai `parquet.thrift`; hanya field yang ditulis port):

| Struct | Field yang ditulis |
| --- | --- |
| `PageHeader` | 1 `type` = DATA\_PAGE (0); 2 `uncompressed_page_size`; 3 `compressed_page_size` (sama, tanpa kompresi); 5 `data_page_header` |
| `DataPageHeader` | 1 `num_values`; 2 `encoding` = PLAIN (0); 3 `definition_level_encoding` = RLE (3); 4 `repetition_level_encoding` = RLE (3) |
| `FileMetaData` | 1 `version` = 1; 2 `schema` (list `SchemaElement`); 3 `num_rows`; 4 `row_groups` (list `RowGroup`); 6 `created_by` = `"swatplus-rust <versi>"` |
| `SchemaElement` akar | 4 `name` = `"schema"`; 5 `num_children` = jumlah kolom |
| `SchemaElement` kolom | 1 `type` (INT32 = 1, FLOAT = 4, BYTE\_ARRAY = 6); 3 `repetition_type` = REQUIRED (0); 4 `name`; 6 `converted_type` = UTF8 (0) untuk string |
| `RowGroup` | 1 `columns` (list `ColumnChunk`); 2 `total_byte_size`; 3 `num_rows` |
| `ColumnChunk` | 2 `file_offset`; 3 `meta_data` |
| `ColumnMetaData` | 1 `type`; 2 `encodings` = \[PLAIN, RLE\]; 3 `path_in_schema` = \[nama kolom\]; 4 `codec` = UNCOMPRESSED (0); 5 `num_values`; 6 `total_uncompressed_size`; 7 `total_compressed_size`; 9 `data_page_offset` |

**Tipe data.**

| Tipe | Isi |
| --- | --- |
| `enum ColType { Int32, Float, Utf8 }` | Tipe kolom (publik, `Copy`, `Eq`). |
| `enum Column { Int32(Vec<i32>), Float(Vec<f32>), Utf8(Vec<String>) }` | Nilai satu kolom untuk satu row group. |
| `struct Compact { buf: Vec<u8>, last: Vec<i16> }` (privat) | Penulis Thrift compact: penyangga keluaran dan tumpukan nomor field terakhir per tingkat struct. |
| konstanta `CT_I32 = 5`, `CT_I64 = 6`, `CT_BINARY = 8`, `CT_LIST = 9`, `CT_STRUCT = 12` | Kode tipe Thrift compact. |
| konstanta Parquet: `TYPE_INT32 = 1`, `TYPE_FLOAT = 4`, `TYPE_BYTE_ARRAY = 6`, `REQUIRED = 0`, `CONVERTED_UTF8 = 0`, `ENC_PLAIN = 0`, `ENC_RLE = 3`, `CODEC_UNCOMPRESSED = 0`, `PAGE_DATA = 0` | Nilai enum Parquet. |

#### 15.6.1  `Column::plain`

- **Lokasi:** `parquet.rs`, `fn plain(&self) -> Vec<u8>` (privat).
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** menyandikan nilai kolom dengan *encoding* PLAIN Parquet.
- **Cara kerja:**
  1. `Int32`: setiap nilai 4 bita little-endian (`to_le_bytes`), dengan kapasitas dipesan `4n`.
  2. `Float`: setiap nilai 4 bita IEEE 754 little-endian.
  3. `Utf8`: setiap string sebagai panjang u32 little-endian diikuti bita UTF-8-nya (BYTE\_ARRAY PLAIN).
- **Catatan port:** float ditulis bit-demi-bit, jadi NaN tetap NaN (pola bit NaN kanonik Rust dari `f32::NAN` atau hasil parsing).

#### 15.6.2  `Compact::varint`

- **Lokasi:** `parquet.rs`, `fn varint(&mut self, v: u64)`.
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** menulis bilangan tak bertanda sebagai ULEB128 (varint Thrift/Protocol Buffers).
- **Cara kerja:** ulangi: ambil 7 bit terendah; geser `v` 7 bit ke kanan; bila `v` sudah 0, tulis bita itu dan berhenti; selain itu tulis bita dengan bit 7 menyala (`| 0x80`).

```latex
v = \sum_{k} b_k \cdot 2^{7k}, \quad b_k \in [0, 127]
```

#### 15.6.3  `Compact::zigzag`

- **Lokasi:** `parquet.rs`, `fn zigzag(v: i64) -> u64` (fungsi asosiasi).
- **Tujuan:** memetakan bilangan bertanda ke tak bertanda agar bilangan kecil (positif maupun negatif) menjadi varint pendek.
- **Cara kerja:** `((v << 1) ^ (v >> 63)) as u64`, dengan `>>` geser aritmetika.

```latex
z(v) = \begin{cases} 2v & v \ge 0 \\ -2v - 1 & v < 0 \end{cases}
```

#### 15.6.4  `Compact::field`

- **Lokasi:** `parquet.rs`, `fn field(&mut self, id: i16, t: u8)`.
- **Tujuan:** menulis header field Thrift compact.
- **Cara kerja:**
  1. `last` = nomor field terakhir di tingkat struct saat ini (puncak tumpukan `self.last`).
  2. `delta = id - last`. Bila `0 < delta ≤ 15`: bentuk pendek, satu bita `(delta << 4) | t`.
  3. Selain itu: bentuk panjang, bita `t` lalu `varint(zigzag(id))`.
  4. Simpan `id` sebagai nomor terakhir di tingkat ini.
- **Catatan port:** uji `compact_field_headers` memeriksa urutan `i32(1, 1)`, `i32(3, -1)`, `i32(20, 5)` menghasilkan `15 02 25 01 05 28 0a` (heksadesimal): delta 1 tipe 5, zigzag(1) = 2; delta 2, zigzag(−1) = 1; delta 17 > 15 sehingga bentuk panjang dengan zigzag(20) = 40 = `0x28`, lalu zigzag(5) = 10.

#### 15.6.5  `Compact::list_header`

- **Lokasi:** `parquet.rs`, `fn list_header(&mut self, elem: u8, n: usize)`.
- **Tujuan:** menulis header list Thrift compact: tipe elemen dan jumlah elemen.
- **Cara kerja:** bila `n < 15`, satu bita `(n << 4) | elem`; selain itu bita `0xF0 | elem` lalu `varint(n)`.

#### 15.6.6  `Compact::begin` dan `Compact::end`

- **Lokasi:** `parquet.rs`, `fn begin(&mut self)`, `fn end(&mut self)`.
- **Tujuan:** membuka dan menutup struct bersarang (elemen list bertipe struct, atau nilai field struct setelah `field(id, CT_STRUCT)`).
- **Cara kerja:** `begin` mendorong 0 ke tumpukan `last` (nomor field di dalam struct baru dimulai dari 0); `end` menulis bita *stop* `0x00` lalu membuang puncak tumpukan.
- **Catatan port:** struct tingkat atas (`PageHeader`, `FileMetaData`) tidak dibuka dengan `begin`; penutupnya ditulis langsung dengan `buf.push(0)` di `write_row_group` dan `finish`.

#### 15.6.7  Fungsi kecil `Column`, `Compact`, dan `physical`

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `Column::new(t: ColType) -> Column` | `parquet.rs` | Kolom kosong bertipe `t`. |
| `Column::len(&self) -> usize` | `parquet.rs` | Jumlah nilai. |
| `Column::is_empty(&self) -> bool` | `parquet.rs` | `len() == 0`. |
| `Column::clear(&mut self)` | `parquet.rs` | Mengosongkan nilai (kapasitas dipertahankan) setelah row group ditulis. |
| `Compact::new() -> Self` | `parquet.rs` | Penyangga kosong, tumpukan `last = [0]` untuk struct tingkat atas. |
| `Compact::i32(&mut self, id: i16, v: i32)` | `parquet.rs` | Field `CT_I32`: header lalu `varint(zigzag(v))`. |
| `Compact::i64(&mut self, id: i16, v: i64)` | `parquet.rs` | Field `CT_I64`: header lalu `varint(zigzag(v))`. |
| `Compact::string(&mut self, id: i16, s: &str)` | `parquet.rs` | Field `CT_BINARY`: header, `varint(len)`, bita string. |
| `Compact::list(&mut self, id: i16, elem: u8, n: usize)` | `parquet.rs` | Field `CT_LIST` lalu `list_header(elem, n)`. |
| `Compact::struct_field(&mut self, id: i16)` | `parquet.rs` | Field `CT_STRUCT` lalu `begin()`. |
| `Compact::list_i32(&mut self, v: i32)` | `parquet.rs` | Elemen list i32 (tanpa header field): `varint(zigzag(v))`. |
| `Compact::list_string(&mut self, s: &str)` | `parquet.rs` | Elemen list biner: `varint(len)` lalu bita. |
| `physical(t: ColType) -> i32` | `parquet.rs` | `Int32` → INT32 (1), `Float` → FLOAT (4), `Utf8` → BYTE\_ARRAY (6). |

### 15.7  `parquet.rs` (2): `ParquetWriter`, row group, dan konversi teks ke Parquet

`ParquetWriter<W: Write>` menulis berkas secara mengalir: magic di awal, satu row group per panggilan `write_row_group`, metadata di `finish`. Ia hanya mengingat metadata (offset dan ukuran) tiap chunk, bukan datanya, sehingga memori yang dipakai sebanding dengan satu row group.

| Struct (privat kecuali `ParquetWriter`) | Field |
| --- | --- |
| `ChunkMeta` | `t: ColType`, `name: String`, `num_values: i64`, `size: i64` (bita header halaman + data), `offset: i64` (posisi awal chunk dalam berkas) |
| `RowGroupMeta` | `chunks: Vec<ChunkMeta>`, `num_rows: i64`, `size: i64` (jumlah ukuran chunk) |
| `ParquetWriter<W>` | `w: W` tujuan; `pos: u64` posisi tulis saat ini; `schema: Vec<(String, ColType)>`; `groups: Vec<RowGroupMeta>` |

Konstanta untuk konversi: `INT_COLUMNS` = `jday`, `mon`, `day`, `yr`, `unit`, `gis_id`, `tstep`, `year`, `plant_no` (kunci yang menjadi INT32), dan `TEXT_COLUMNS` = `name`, `type`, `plantnm`, `plant_name`, `plnt_name` (kolom teks). Perbandingan nama tidak peka huruf besar-kecil (`PLANTNM` pada `crop_yld_aa.txt` cocok dengan `plantnm`).

#### 15.7.1  `ParquetWriter::new`

- **Lokasi:** `parquet.rs`, `pub fn new(mut w: W, schema: &[(String, ColType)]) -> std::io::Result<Self>`.
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** memulai berkas dan menetapkan skema dengan nama kolom yang unik.
- **Cara kerja:**
  1. Tulis magic `PAR1`.
  2. Untuk setiap kolom skema: bila nama sudah terpakai, coba `nama_2`, `nama_3`, … sampai unik.
  3. Kembalikan penulis dengan `pos = 4` dan daftar row group kosong.
- **Catatan port:** penamaan unik diperlukan karena header tabel kanal/waduk SWAT+ mengulang `null` (suhu tiap hidrograf); Parquet tidak melarang nama ganda secara formal, tetapi kebanyakan pembaca (Arrow, DuckDB) memetakan kolom menurut nama. `docs/runner.md`: "nama yang berulang mendapat `_2`".

#### 15.7.2  `ParquetWriter::write_row_group`

- **Lokasi:** `parquet.rs`, `pub fn write_row_group(&mut self, cols: &[Column]) -> std::io::Result<()>`.
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** menulis satu row group: satu column chunk (satu halaman data) per kolom skema.
- **Masukan dan keluaran:** satu `Column` per kolom skema, semuanya sepanjang `n`.
- **Cara kerja:**
  1. `n` = panjang kolom pertama. Bila jumlah kolom tidak sama dengan skema atau panjangnya tidak seragam, kembalikan `InvalidInput` ("columns don't match the schema").
  2. Bila `n = 0`, tidak menulis apa pun (row group kosong tidak dibuat).
  3. Untuk setiap kolom: `data = col.plain()`; susun `PageHeader` dengan `Compact`: field 1 = DATA\_PAGE, field 2 dan 3 = `data.len()` (tak terkompresi = terkompresi), field 5 = struct `DataPageHeader` {1 `num_values = n`, 2 PLAIN, 3 RLE, 4 RLE}, tutup struct dalam, lalu tulis bita stop `0` untuk menutup `PageHeader`.
  4. Catat `offset = pos`, tulis header lalu data, `size = |header| + |data|`, majukan `pos`.
  5. Simpan `ChunkMeta` dan, di akhir, `RowGroupMeta { chunks, num_rows: n, size: Σ size }`.
- **Catatan port:** ukuran halaman disimpan sebagai `i32` Thrift, jadi satu kolom satu row group dibatasi < 2 GiB; dengan 2^20 baris dan nilai 4 bita, kolom numerik berukuran 4 MiB. Kolom string juga tidak dikompresi atau dikodekan kamus, sehingga nama objek berulang di setiap baris; ini pilihan kesederhanaan, bukan keterbatasan pembaca.

#### 15.7.3  `ParquetWriter::finish`

- **Lokasi:** `parquet.rs`, `pub fn finish(mut self) -> std::io::Result<W>`.
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** menulis `FileMetaData`, panjangnya, dan magic penutup; mengembalikan penulis.
- **Cara kerja:**
  1. Field 1 `version = 1`.
  2. Field 2 list struct berisi `schema.len() + 1` elemen: akar (`name = "schema"`, `num_children`), lalu per kolom `type`, `repetition_type = REQUIRED`, `name`, dan `converted_type = UTF8` bila string.
  3. Field 3 `num_rows` = jumlah baris semua row group (i64).
  4. Field 4 list `RowGroup`: per group, field 1 list `ColumnChunk`; per chunk: field 2 `file_offset = offset`, field 3 struct `ColumnMetaData` {1 `type`, 2 list i32 \[PLAIN, RLE\], 3 list biner \[nama kolom\], 4 UNCOMPRESSED, 5 `num_values`, 6 dan 7 ukuran, 9 `data_page_offset = offset`}, tutup `ColumnMetaData` dan `ColumnChunk`; lalu field 2 `total_byte_size`, field 3 `num_rows`, tutup `RowGroup`.
  5. Field 6 `created_by = "swatplus-rust " + CARGO_PKG_VERSION`; bita stop penutup `FileMetaData`.
  6. Tulis metadata, panjangnya sebagai u32 little-endian, `PAR1`; `flush`; kembalikan `w`.
- **Catatan port:** `encodings` mencantumkan RLE walaupun tidak ada level yang ditulis, karena header halaman menyebut RLE sebagai *encoding* level (praktik umum penulis Parquet). Nomor field melompat (misalnya 2 → 9 di `ColumnMetaData`) ditangani `field` dengan bentuk pendek selama selisihnya ≤ 15. Pembaca mencari metadata dari belakang: 4 bita magic, 4 bita panjang, lalu metadata.

#### 15.7.4  `parse_f32`

- **Lokasi:** `parquet.rs`, `fn parse_f32(tok: &str) -> f32` (privat).
- **Padanan SWAT+:** kebalikan dari format keluaran Fortran (`f12.3`, `e15.4`, *list-directed*).
- **Tujuan:** membaca satu token angka dari tabel teks, termasuk gaya Fortran.
- **Cara kerja:**
  1. Coba `tok.parse::<f32>()` langsung.
  2. Bila gagal dan token mengandung `.E` atau `.e` (bentuk Fortran seperti `0.E+00` tanpa digit setelah titik), sisipkan `0` (`.E` → `.0E`) lalu coba lagi.
  3. Bila tetap gagal (misalnya `*****`, luapan lebar Fortran), kembalikan NaN.
- **Catatan port:** `docs/runner.md`: "NaN di tempat SWAT+ mencetak NaN atau tanda bintang". Nilai yang tersimpan adalah angka **yang tercetak** (presisi cetak SWAT+), bukan nilai internal; konversi desimal ke f32 memakai pembulatan ke terdekat pustaka standar Rust.

#### 15.7.5  `text_to_parquet`

- **Lokasi:** `parquet.rs`, `pub fn text_to_parquet<R: BufRead, W: Write>(reader: R, w: W, group: usize) -> Result<(usize, W), String>`.
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** mengubah satu tabel teks SWAT+ (buatan port atau SWAT+ sendiri) menjadi Parquet dengan kolom, urutan, dan nama yang sama.
- **Masukan dan keluaran:** pembaca baris, penulis tujuan, ukuran row group (`1 << 20` = 1.048.576 baris pada kedua pemanggil); kembalian jumlah baris dan penulis.
- **Cara kerja:**
  1. Lewati baris pertama (judul).
  2. Pecah setiap baris menurut spasi. Baris kosong dilewati. Sebelum header ditemukan, baris yang token pertamanya `jday` atau `year` menjadi daftar nama kolom; baris lain dilewati.
  3. Setelah header, hanya baris yang token pertamanya bilangan bulat (`parse::<i64>`) yang dianggap baris data; baris satuan dan baris kosong dilewati.
  4. Fase sampel: 1.000 baris data pertama ditampung di `pending`. Saat mencapai 1.000, skema disimpulkan dengan `infer`, `ParquetWriter::new` dibuat, dan baris sampel dipindahkan ke kolom.
  5. Fase aliran: setiap baris berikutnya didorong ke kolom (`push`); bila panjang kolom ≥ `group`, tulis row group dan kosongkan kolom.
  6. Di akhir: bila penulis belum dibuat (tabel < 1.000 baris), simpulkan skema dari semua baris yang ada; tulis sisa kolom sebagai row group terakhir; `finish`.
- **Catatan port:** uji `group` hanya berlaku pada fase aliran, sehingga bila `group` < 1.000 row group pertama tetap berisi 1.000 baris sampel ditambah baris berikutnya (uji unit memakai `group = 1`). Pemisahan menurut spasi mengandaikan nama objek tanpa spasi (benar untuk nama SWAT+, yang dibaca *list-directed*). Tabel tanpa baris data tetap menghasilkan berkas Parquet sah dengan skema dan nol row group (dengan sampel kosong, kolom teks tetap UTF-8, kolom kunci menjadi INT32 karena uji "semua sampel" atas himpunan kosong bernilai benar, dan sisanya FLOAT); tabel tanpa header menghasilkan skema kosong.

#### 15.7.6  `infer` (fungsi bersarang di `text_to_parquet`)

- **Lokasi:** `parquet.rs`, `fn infer(names: &[String], sample: &[Vec<String>]) -> Vec<(String, ColType)>`.
- **Tujuan:** menentukan tipe setiap kolom dari namanya dan sampel baris.
- **Cara kerja,** untuk kolom ke-`k` bernama `n` (dibandingkan dalam huruf kecil):
  1. `n` di `TEXT_COLUMNS` → `Utf8`.
  2. `n` di `INT_COLUMNS` dan semua sampel di posisi `k` dapat dibaca sebagai `i32` → `Int32`.
  3. Semua sampel di posisi `k` kosong, atau terbaca sebagai float (bukan NaN), atau secara harfiah memuat `nan` → `Float`.
  4. Selain itu → `Utf8`.
- **Catatan port:** aturan 3 berarti kolom berisi `*****` pada 1.000 baris pertama menjadi string, bukan float; di luar sampel, nilai bintang di kolom float menjadi NaN. Nama kolom yang tidak dikenali (misalnya `flo_out`) selalu mengikuti aturan 3/4.

#### 15.7.7  `push` (fungsi bersarang di `text_to_parquet`)

- **Lokasi:** `parquet.rs`, `fn push(cols: &mut [Column], row: &[String])`.
- **Tujuan:** menambahkan satu baris token ke kolom-kolom.
- **Cara kerja:** untuk setiap kolom ke-`k`, token = `row[k]` atau `""` bila baris lebih pendek. `Int32`: `parse::<i32>()` atau 0 bila gagal. `Float`: NaN bila token kosong, selain itu `parse_f32`. `Utf8`: token apa adanya.
- **Catatan port:** token di luar jumlah kolom header diabaikan; kolom yang tidak terisi menjadi NaN (float), 0 (int), atau string kosong.

#### 15.7.8  Fungsi kecil dan uji `parquet.rs`

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `ParquetWriter::schema(&self) -> &[(String, ColType)]` | `parquet.rs` | Skema dengan nama yang sudah dibuat unik; dipakai `text_to_parquet` untuk membuat kolom kosong. |
| `tests::compact_field_headers()` | `parquet.rs` (uji) | Memeriksa bentuk pendek/panjang header field dan zigzag (lihat 15.6.4). |
| `tests::table_to_parquet()` | `parquet.rs` (uji) | Mengonversi tabel dua baris yang memuat `0.2E+01` dan `*****`; memeriksa 2 baris dan magic `PAR1` di awal dan akhir. |

### 15.8  `tar.rs`: arsip tar format *ustar*

`tar.rs` membaca dan menulis arsip tar polos (POSIX *ustar*) agar skenario dapat masuk ke engine dan tabel keluar darinya sebagai satu gumpal bita di tempat tanpa sistem berkas (peramban). Pemakainya `swat-wasm`: `run_inner` membaca arsip masukan menjadi `MemSource` (berkas TxtInOut), dan `run_tar` membungkus tabel serta `run.json` menjadi arsip keluaran. Tidak ada padanan SWAT+.

**Format ustar.** Arsip adalah deretan blok 512 bita. Setiap berkas = satu blok header + data yang dibulatkan ke atas ke kelipatan 512 (sisa diisi nol). Arsip diakhiri dua blok nol. Field header yang dipakai port:

| Offset | Panjang | Field | Isi yang ditulis port | Dibaca port? |
|---|---|---|---|---|
| 0 | 100 | `name` | nama berkas (dipotong 100 bita) | ya |
| 100 | 8 | `mode` | `0000644` + NUL | tidak |
| 108 | 8 | `uid` | `0000000` + NUL | tidak |
| 116 | 8 | `gid` | `0000000` + NUL | tidak |
| 124 | 12 | `size` | ukuran data, 11 digit oktal + NUL | ya |
| 136 | 12 | `mtime` | 0 (11 digit oktal + NUL) | tidak |
| 148 | 8 | `chksum` | 6 digit oktal + NUL + spasi | tidak (tidak diverifikasi) |
| 156 | 1 | `typeflag` | `'0'` (berkas biasa) | ya |
| 257 | 6 | `magic` | `"ustar\0"` | ya (5 bita pertama) |
| 263 | 2 | `version` | `"00"` | tidak |
| 345 | 155 | `prefix` | (kosong) | ya, bila magic ustar |

#### 15.8.1  `read`

- **Lokasi:** `crates/swat-output/src/tar.rs`, `pub fn read(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String>`.
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** mengambil berkas-berkas biasa dari arsip tar, berurutan, dengan folder bersama dilepas dari nama.
- **Masukan dan keluaran:** bita arsip; daftar `(nama, isi)`.
- **Cara kerja:**
  1. Selama masih ada satu blok utuh di posisi `pos`: bila blok itu seluruhnya nol, berhenti (penanda akhir).
  2. Fungsi bantu lokal `field(a, n)` membaca teks dari offset `a` sepanjang `n` sampai NUL pertama (UTF-8 *lossy*).
  3. Ukuran = field 124/12 dibaca sebagai oktal setelah memangkas spasi dan NUL; bila gagal, galat `tar: bad size … at byte …`.
  4. Nama = field 0/100; bila `h[257..262] == "ustar"` dan field `prefix` tidak kosong, nama = `prefix/nama`.
  5. Bila data melewati akhir arsip, galat `tar: <nama> runs past the end`.
  6. Menurut `typeflag`: `'0'` atau NUL (berkas biasa, termasuk gaya tar lama) → simpan; bila sebelumnya ada nama panjang GNU, nama itu dipakai. `'L'` (GNU *long name*) → isi datanya sampai NUL menjadi nama entri berikutnya. Tipe lain (folder `'5'`, tautan, header pax `'x'`/`'g'`, dll.) dilewati.
  7. `pos = awal_data + ⌈size/512⌉ · 512`.
  8. Pelepasan folder: bila berkas pertama punya komponen folder pertama `d` dan **semua** berkas memiliki komponen pertama yang sama, `d/` dibuang dari setiap nama (jadi `TxtInOut/file.cio` → `file.cio`).
- **Catatan port:** `docs/runner.md`: skenario boleh berada dalam satu folder atau tanpa folder. Checksum header tidak diperiksa, ukuran dalam pengodean biner GNU (berkas > 8 GiB) tidak didukung, dan atribut pax (nama panjang gaya pax) diabaikan sehingga untuk arsip pax nama yang dipakai adalah nama di header ustar. Hanya satu tingkat folder yang dilepas.

#### 15.8.2  `write`

- **Lokasi:** `tar.rs`, `pub fn write<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Vec<u8>`.
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** membuat arsip ustar dari pasangan nama dan isi (nama hingga 100 bita).
- **Cara kerja,** untuk setiap berkas:
  1. Header 512 bita nol; salin nama (maksimum 100 bita).
  2. Fungsi bantu lokal `octal(h, at, width, v)` menulis `v` sebagai oktal berisi nol di depan selebar `width - 1` digit; bita terakhir tetap NUL. Isi: `mode` 0644, `uid` 0, `gid` 0, `size`, `mtime` 0.
  3. `typeflag = '0'`, `magic = "ustar\0"`, `version = "00"`.
  4. Checksum: isi field `chksum` dengan 8 spasi, jumlahkan semua 512 bita header sebagai bilangan tak bertanda, tulis sebagai 6 digit oktal + NUL + spasi.
  5. Tambahkan header, data, dan bantalan nol sampai kelipatan 512.
  6. Setelah semua berkas: 1.024 bita nol (dua blok akhir).

```latex
\text{chksum} = \sum_{i=0}^{511} h_i \quad (h_{148..155} = \text{8 spasi})
```

- **Catatan port:** `mtime = 0`, `uid = gid = 0`, dan tanpa nama pengguna membuat arsip **deterministik**: input yang sama selalu memberi bita yang sama. Nama lebih dari 100 bita dipotong diam-diam (tidak ada field `prefix` atau GNU long name yang ditulis); nama tabel SWAT+ jauh lebih pendek. Ukuran 11 digit oktal membatasi satu berkas < 8 GiB.

#### 15.8.3  Uji `tar.rs`

| Fungsi | Lokasi | Kegunaan |
|---|---|---|
| `tests::round_trip()` | `tar.rs` (uji) | Menulis `TxtInOut/file.cio` (5 bita) dan `TxtInOut/x.txt` (600 bita, melewati satu blok), membaca kembali, dan memeriksa bahwa folder `TxtInOut/` terlepas dan isi sama. |

### 15.9  `progress.rs`: progres dan kejadian sebagai JSON-line

`progress.rs` menyusun objek JSON satu per baris untuk program yang menjalankan engine (runner RustGIS). Setiap baris adalah satu objek JSON lengkap dengan field `event`; pembaca cukup membaca keluaran standar baris demi baris dan mem-*parse* tiap baris secara terpisah. Jenis kejadian (dari komentar modul dan `docs/runner.md`):

| `event` | Field lain | Dibuat oleh |
|---|---|---|
| `start` | `version`, `days`, `first`, `last` (tanggal `yyyy-mm-dd`) | `main.rs` (langsung dengan `format!`) |
| `progress` | `done` (hari selesai), `days` (total), `date` (hari yang dimulai) | `progress::progress` lewat `Reporter` |
| `note` | `text` | `progress::text("note", …)` |
| `table` | `file`, dan `rows` untuk Parquet | `main.rs` (memakai `quote`) |
| `done` | `days`, `seconds` (3 desimal) | `main.rs` |
| `error` | `text` | `progress::text("error", …)` |

`done` atau `error` selalu menjadi kejadian terakhir. Padanan SWAT+: Fortran hanya mencetak tanggal simulasi ke layar dengan `write (*,1234) …` di `time_control.f90` (teks bebas, bukan JSON); tidak ada padanan untuk kejadian lainnya.

**Struktur `Reporter`.** Field `last_pct: Option<usize>` (persen terakhir yang dilaporkan) dan `emit: Box<dyn FnMut(&str)>` (penerima baris: `main.rs` mencetak ke stdout; `swat-wasm` mem-*parse* kembali `done` dan `days` dari baris itu lalu memanggil impor `env.swat_progress(done, days)`).

#### 15.9.1  `quote`

- **Lokasi:** `crates/swat-output/src/progress.rs`, `pub fn quote(s: &str) -> String`.
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** menulis `s` sebagai literal string JSON, lengkap dengan tanda kutip.
- **Cara kerja:** tanda kutip pembuka; untuk setiap karakter: `"` → `\"`, `\` → `\\`, baris baru → `\n`, CR → `\r`, tab → `\t`, karakter kendali lain (< U+0020) → `\u00XX` (heksadesimal kecil 4 digit); karakter lain (termasuk non-ASCII) disalin apa adanya sebagai UTF-8; tanda kutip penutup.
- **Catatan port:** ini adalah himpunan *escape* minimum yang diwajibkan JSON (RFC 8259); dipakai juga oleh `swat-wasm` untuk `run.json` dan oleh `main.rs` untuk nama berkas di kejadian `table`.

#### 15.9.2  `Reporter::day`

- **Lokasi:** `progress.rs`, `pub fn day(&mut self, done: usize, days: usize, d: Day)`.
- **Padanan SWAT+:** tidak ada (bandingkan pencetakan tanggal harian `time_control.f90`).
- **Tujuan:** memancarkan kejadian `progress` hanya saat persentase bulat berubah, dan selalu di akhir.
- **Masukan:** dari `OutputSink::progress`: hari selesai, total hari, hari yang dimulai.
- **Cara kerja:**
  1. Persen bulat ke bawah; bila `days = 0`, `checked_div` gagal dan persen = 100:

```latex
p = \left\lfloor \frac{100 \cdot \text{done}}{\text{days}} \right\rfloor
```

  2. Bila `p` berbeda dari `last_pct` (termasuk panggilan pertama, saat `last_pct` kosong) atau `done == days`, simpan `p` dan panggil `emit(progress(done, days, d))`.
- **Catatan port:** untuk run yang lebih pendek dari 100 hari, hampir setiap hari mengubah persen sehingga dilaporkan; untuk run panjang paling banyak sekitar 101 baris progres (persen 0 sampai 100). Persentase dihitung dalam bilangan bulat, tanpa floating-point.

#### 15.9.3  Fungsi trivial dan uji `progress.rs`

| Fungsi | Lokasi | Kegunaan |
|---|---|---|
| `date(d: Day) -> String` | `progress.rs` | Tanggal sebagai `yyyy-mm-dd` dari `d.yr`, `d.mon`, `d.day` (nol di depan). |
| `progress(done: usize, days: usize, d: Day) -> String` | `progress.rs` | Baris `{"event":"progress","done":…,"days":…,"date":"yyyy-mm-dd"}`. |
| `text(event: &str, text: &str) -> String` | `progress.rs` | Baris `{"event":<event>,"text":<text>}` dengan keduanya di-`quote`; untuk `note` dan `error`. |
| `Reporter::new(emit: Box<dyn FnMut(&str)>) -> Self` | `progress.rs` | Pelapor dengan `last_pct = None`. |
| `tests::json()` | `progress.rs` (uji) | Memeriksa *escape* `quote("a\"b\\c\n")` dan bentuk persis baris `progress` untuk 1 Januari 2015. |

### 15.10  Rekapitulasi bab

Bab ini mencakup **84 fungsi** di enam berkas `crates/swat-output/src/`: `store.rs` 7 (trait method, 2 implementasi `append`, 4 fungsi kecil), `text.rs` 37 (3 fungsi format bebas `e15_4`/`period_index`/`suffix`, 3 bantu `header`/`keys`/`row`, 12 method `TextTables`, 19 method `OutputSink`), `parquet.rs` 29 (4 method `Column` + `plain`, 14 method `Compact`, `physical`, 4 method `ParquetWriter`, `parse_f32`, `text_to_parquet` beserta `infer` dan `push`, 2 uji), `tar.rs` 3 (`read`, `write`, 1 uji), `progress.rs` 7 (`quote`, `date`, `progress`, `text`, `Reporter::new`, `Reporter::day`, 1 uji), `lib.rs` 0. Closure lokal (`field` dan `octal` di `tar.rs`, `io` di `text_to_parquet`) dijelaskan di dalam entri induknya.

Hal yang perlu diingat saat membaca keluaran port:

1. **Nilai tabel teks = nilai SWAT+, tata letak kolom tidak selalu.** Lebar kunci dan beberapa lebar nilai (misalnya `f17.3` tabel hara, `2i10` waduk) disederhanakan; pembanding dan konverter memisahkan kolom berdasarkan spasi. Port tidak pernah mencetak tanda bintang luapan; kolom melebar.
2. **Kolom yang tidak dihitung port tidak dicetak** (tabel HRU tanpa hara, mode degradasi HRU, kolom teks `plant_cov`/`mgt_ops`).
3. **Kebiasaan SWAT+ yang dipertahankan di keluaran:** label `orgn`/`orgp` pada kolom akuifer yang berisi karbon dan N organik (diperbaiki dengan `--fix-swat`), baris satuan kanal harian di bawah header `channel_sd_subday.txt`, label `m^3/s` untuk aliran sub-harian yang sebenarnya m³.
4. **Parquet adalah turunan teks**: tipe kolom disimpulkan dari nama dan 1.000 baris pertama; nilai float adalah angka yang tercetak; NaN untuk NaN atau bintang.
5. **Semua format biner ditulis tangan** (Thrift compact, PLAIN, ustar) tanpa dependensi, demi lisensi LGPL.
