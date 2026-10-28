## Bab 16  Modul WebAssembly dan program baris perintah

Bab ini membahas lapisan terluar engine: dua "pintu" yang dipakai program lain untuk menjalankan simulasi. Pintu pertama adalah modul WebAssembly `swat_wasm.wasm` (crate `swat-wasm`, file `crates/swat-wasm/src/lib.rs`) yang dijalankan RustGIS di dalam *Web Worker* peramban. Pintu kedua adalah program baris perintah `swatplus-rust` (crate `swatplus-rust`, file `crates/swatplus-rust/src/main.rs` dan `crates/swatplus-rust/src/compare.rs`) dengan tiga perintah: `run`, `inspect`, dan `compare`. Bab ini juga meringkas skrip `scripts/wasm-run.mjs`, kontrak keluaran yang dibakukan di `docs/runner.md`, dan alur kerja GitHub Actions (`.github/workflows/ci.yml`, `validate.yml`, `fortran.yml`, `release.yml`) yang memvalidasi dan merilis kedua pintu tersebut.

Dalam alur simulasi, kedua pintu ini membungkus urutan yang sama: (1) membaca skenario TxtInOut melalui `swat_io::load_project` (dari folder lewat `DirSource`, atau dari memori lewat `MemSource`), (2) menyiapkan penulis tabel `swat_output::TextTables` dengan judul baris pertama dan pengaturan `print.prt`, (3) memanggil `swat_engine::run::run_with` dengan `RunOptions { fixes }`, (4) menuntaskan tabel (`finish`), lalu (5) bila diminta mengonversi setiap tabel teks ke Parquet. Perbedaannya hanya pada tempat masukan dan keluaran (folder berkas vs arsip tar di memori) serta cara melaporkan progres (baris JSON di *stdout* vs fungsi impor `env.swat_progress`).

Alasan desain mengapa engine tidak pernah ditautkan sebagai pustaka ke program pemanggil dicatat di `docs/runner.md`: engine berlisensi LGPL-2.1 (karena turunan SWAT+), sedangkan pemanggilnya (RustGIS) tidak harus demikian; maka engine selalu berupa program terpisah atau modul WebAssembly (keputusan desain K11 RustGIS). SWAT+ Fortran sendiri tidak punya padanan langsung untuk sebagian besar fungsi di bab ini: program Fortran `main` (`main.f90.in`) dijalankan tanpa argumen di dalam folder TxtInOut, menulis tabel ke folder yang sama, dan menampilkan progres harian ke layar dari `time_control.f90`.

### 16.1  Modul WebAssembly (`crates/swat-wasm/src/lib.rs`)

Crate `swat-wasm` dibangun sebagai `cdylib` (untuk `.wasm`) sekaligus `rlib` (agar fungsinya dapat diuji dari Rust). Modul ini sengaja tidak bergantung pada pustaka apa pun di luar crate workspace (`swat-model`, `swat-io`, `swat-engine`, `swat-output`) dan tidak memerlukan *glue* JavaScript seperti `wasm-bindgen`: antarmukanya adalah ABI C mentah berupa pointer dan panjang di memori linear modul. Lint crate ini menetapkan `unsafe_code = "deny"`; satu-satunya pengecualian adalah modul `ffi` yang diberi `#[allow(unsafe_code)]` dan hanya dikompilasi untuk `target_arch = "wasm32"`.

Modul dibangun dengan `cargo build --release --target wasm32-unknown-unknown -p swat-wasm` dan menghasilkan `target/wasm32-unknown-unknown/release/swat_wasm.wasm`.

**Ringkasan ABI.** Semua bilangan adalah `i32` WebAssembly (pada wasm32, `usize` dan pointer berukuran 32 bit).

| Arah | Nama | Tanda tangan | Kegunaan |
| --- | --- | --- | --- |
| ekspor | `memory` | memori linear | memori modul; pemanggil menyalin masukan ke dalamnya dan membaca keluaran darinya |
| ekspor | `swat_alloc` | `(len) -> ptr` | memesan penyangga `len` bita untuk diisi pemanggil |
| ekspor | `swat_free` | `(ptr, len)` | membebaskan penyangga dari `swat_alloc` atau `swat_run` |
| ekspor | `swat_run` | `(ptr, len, flags) -> ptr` | menjalankan skenario dalam arsip tar di `ptr` |
| impor | `env.swat_progress` | `(done, days)` | dipanggil modul pada setiap persen bulat hari simulasi |

**Bit `flags`** (diterjemahkan oleh `Options::from_flags`):

| Bit | Nilai | Arti |
| --- | --- | --- |
| 0 | 1 | tulis tabel Parquet |
| 1 | 2 | tulis tabel teks SWAT+ |
| 2 | 4 | koreksi bug SWAT+ (setara `--fix-swat`) |

Bila bit 1 dan bit 2 sama-sama nol, tabel teks tetap ditulis (jadi `flags = 0` dan `flags = 2` setara; `flags = 3` menulis keduanya; `flags = 5` menulis Parquet saja dengan koreksi).

**Format masukan.** Arsip tar (ustar) berisi berkas-berkas TxtInOut, boleh di akar arsip atau di dalam satu folder (mis. `TxtInOut/file.cio`); bila semua entri berada di bawah satu folder yang sama, `swat_output::tar::read` membuang awalan folder itu. Entri GNU *long name* (`L`) didukung; jenis entri selain berkas biasa diabaikan.

**Format keluaran.** Penyangga yang dikembalikan `swat_run` berisi 4 bita panjang (`u32` *little-endian*) diikuti arsip tar sepanjang itu. Arsip memuat tabel keluaran (`*.txt`, `*.parquet`, atau keduanya) dan berkas terakhir `run.json`:

```text
{"version":"1.0.1","ok":true,"error":null,"notes":[...],"tables":["hru_wb_day.txt", ...]}
```

Pemanggil wajib membebaskan penyangga itu dengan `swat_free(ptr, 4 + panjang)`.

**Struct `Options`.** Tiga field `bool` dengan turunan `Clone, Copy, Debug, PartialEq, Eq`:

| Field | Arti |
| --- | --- |
| `parquet` | tulis `<tabel>.parquet` |
| `text` | tulis `<tabel>.txt` |
| `fixes` | jalankan mode terkoreksi (`docs/swat-fixes.md`) |

#### 16.1.1  `Options::from_flags`

- **Lokasi:** `crates/swat-wasm/src/lib.rs`, `pub fn from_flags(flags: u32) -> Options`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** menerjemahkan bilangan `flags` dari ABI menjadi `Options`.
- **Masukan dan keluaran:** `flags` (bit mask, tanpa satuan) → `Options`.
- **Cara kerja:**
  1. `p = flags & 1 != 0`, `t = flags & 2 != 0`.
  2. `parquet = p`; `text = t || !p` (teks menjadi bawaan bila Parquet tidak diminta); `fixes = flags & 4 != 0`.
  3. Bit lain diabaikan tanpa galat.
- **Catatan port:** aturan "keduanya bila tidak ada yang dipilih" di komentar modul berarti dalam praktik: bila bit Parquet nol, teks selalu ditulis. Tidak ada kombinasi bit yang menghasilkan keluaran kosong.

#### 16.1.2  `run_tar`

- **Lokasi:** `crates/swat-wasm/src/lib.rs`, `pub fn run_tar(input: &[u8], opts: Options, progress: Box<dyn FnMut(usize, usize)>) -> Vec<u8>`
- **Padanan SWAT+:** secara fungsional program `main` (`main.f90.in`): baca masukan, simulasi, tulis keluaran. Tidak ada padanan untuk pengemasan tar dan `run.json`.
- **Tujuan:** menjalankan satu skenario sepenuhnya di memori dan mengembalikan arsip tar berisi tabel dan laporan `run.json`. Fungsi ini adalah inti modul yang juga dapat dipanggil langsung dari Rust (dipakai uji unit).
- **Masukan dan keluaran:** `input` = bita arsip tar skenario; `opts` = format dan mode; `progress(done, days)` = *callback* progres (hari selesai, total hari). Keluaran: bita arsip tar. Fungsi ini **tidak pernah gagal**: setiap galat dimasukkan ke `run.json`.
- **Cara kerja:**
  1. Siapkan `files` (daftar pasangan nama dan bita) dan `notes` (catatan) kosong.
  2. Panggil `run_inner`; bila mengembalikan `Err(teks)`, simpan teks itu sebagai `error`.
  3. Susun JSON secara manual (tanpa *crate* serde) dengan `swat_output::progress::quote` untuk melolos-kan string: `version` (versi crate, `CARGO_PKG_VERSION`), `ok` (`true` bila tidak ada galat), `error` (string atau `null`), `notes` (larik string), `tables` (nama berkas yang sudah terkumpul, sesuai urutan).
  4. Tambahkan `run.json` (diakhiri baris baru) sebagai berkas terakhir, lalu bungkus semuanya dengan `swat_output::tar::write`.
- **Catatan port:** bila galat terjadi di tengah pengonversian Parquet, tabel yang sudah masuk `files` sebelum galat tetap ada di arsip dan tetap tercantum di `tables`, meskipun `ok` bernilai `false`. Pemanggil sebaiknya selalu memeriksa `ok` lebih dulu. Nama dalam tar dibatasi 100 bita oleh `tar::write` (tanpa entri *long name* saat menulis); nama tabel SWAT+ jauh lebih pendek dari batas ini.

#### 16.1.3  `run_inner`

- **Lokasi:** `crates/swat-wasm/src/lib.rs`, `fn run_inner(input: &[u8], opts: Options, progress: Box<dyn FnMut(usize, usize)>, files: &mut Vec<(String, Vec<u8>)>, notes: &mut Vec<String>) -> Result<(), String>` (privat)
- **Padanan SWAT+:** urutan `main.f90.in` (pembacaan `proc_*`, lalu `time_control`), tanpa padanan langsung untuk lapisan memori.
- **Tujuan:** pekerjaan sebenarnya dari `run_tar`, dengan operator `?` untuk meneruskan galat.
- **Masukan dan keluaran:** seperti `run_tar`; hasil ditambahkan ke `files` dan `notes`.
- **Cara kerja:**
  1. Buat `MemSource` kosong; untuk setiap entri `swat_output::tar::read(input)?`, `src.insert(nama, bita)`. `MemSource::insert` mendekode bita menjadi teks saat disisipkan.
  2. `load_project(&src)` membaca seluruh proyek; laporan pemuatan (`LoadReport`: berkas hilang dan tak terbaca) diabaikan di modul ini.
  3. Susun judul tabel: spasi, judul `file.cio` tanpa titik dua di ujung, rata kiri 24 karakter, lalu `swatplus-rust <versi>`. Judul ini identik dengan yang dipakai program baris perintah, sehingga tabel WASM dan tabel *native* dapat dibandingkan bita demi bita.
  4. Buat `TextTables::new(judul, MemStore::new(), &project.print)`, set `tables.fixes = opts.fixes`.
  5. Pasang `progress::Reporter` dengan fungsi *emit* yang menerima baris JSON `{"event":"progress","done":…,"days":…,"date":…}`. Karena `Reporter` menghasilkan teks, *closure* `num(key)` membaca kembali dua angka dari teks itu: memotong string sesudah `"done":` atau `"days":`, mengambil awalan berdigit, mem-*parse* sebagai `usize` (0 bila gagal), lalu memanggil `progress(done, days)`.
  6. `swat_engine::run::run_with(&project, &mut tables, RunOptions { fixes })?`.
  7. Dari `RunReport`, ubah setiap alasan yang tidak `None` menjadi catatan: `HRU water balance not simulated (…)`, `channels not routed (…)`, `routing units and aquifers not simulated (…)`.
  8. `tables.finish()?` mengembalikan `MemStore`; untuk setiap pasangan (nama, teks) di `store.files`:
     - bila `parquet`: nama dasar = nama tanpa `.txt`; `text_to_parquet(teks, Vec::new(), 1 << 20)` (satu *row group* per 2^20 = 1.048.576 baris), simpan `<dasar>.parquet`; galat diberi awalan nama berkas;
     - bila `text`: simpan teks apa adanya.
- **Catatan port:** tidak ada opsi `--strict` di WASM; bagian proyek yang tidak disimulasikan hanya muncul sebagai catatan di `run.json`. Urutan tabel dalam arsip mengikuti urutan iterasi `MemStore::files` (koleksi berkunci nama). `Reporter` memanggil *emit* setiap kali persentase bulat `floor(100·done/days)` berubah dan sekali lagi pada hari terakhir, sehingga skenario panjang menghasilkan sekitar 101 panggilan.

```latex
\mathrm{pct} = \left\lfloor \frac{100 \cdot \mathrm{done}}{\mathrm{days}} \right\rfloor
```

#### 16.1.4  `ffi::swat_alloc`

- **Lokasi:** `crates/swat-wasm/src/lib.rs`, modul `ffi` (hanya wasm32), `#[unsafe(no_mangle)] pub extern "C" fn swat_alloc(len: usize) -> *mut u8`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** memberi pemanggil JavaScript sebuah penyangga di memori modul untuk diisi arsip tar masukan.
- **Masukan dan keluaran:** `len` (bita) → pointer ke `len` bita bernilai nol.
- **Cara kerja:**
  1. `vec![0u8; len]` dialokasikan dengan kapasitas sama dengan panjang.
  2. Pointer diambil, lalu `std::mem::forget` mencegah Rust membebaskannya; kepemilikan berpindah ke pemanggil.
- **Catatan port:** karena kapasitas sama dengan `len`, pasangan `swat_free(ptr, len)` dapat merekonstruksi `Vec` dengan tepat. Alokasi baru dapat memperbesar memori linear (`memory.grow`), sehingga pemanggil JavaScript harus membuat ulang *view* (`Uint8Array`, `DataView`) atas `memory.buffer` setelah setiap panggilan yang mungkin mengalokasi; `scripts/wasm-run.mjs` melakukannya.

#### 16.1.5  `ffi::swat_free`

- **Lokasi:** `crates/swat-wasm/src/lib.rs`, `pub unsafe extern "C" fn swat_free(ptr: *mut u8, len: usize)`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** mengembalikan penyangga yang sebelumnya diberikan modul.
- **Masukan dan keluaran:** `ptr` dan `len` dari `swat_alloc` (panjang yang sama) atau dari `swat_run` (`4 + panjang`).
- **Cara kerja:** bila `ptr` bukan null, `Vec::from_raw_parts(ptr, len, len)` lalu `drop`.
- **Catatan port:** kontrak keselamatan (`# Safety`): `ptr` dan `len` harus penyangga yang diberikan modul ini dan belum pernah dibebaskan. Memberi panjang yang salah adalah perilaku tak terdefinisi; modul tidak dapat memeriksanya.

#### 16.1.6  `ffi::swat_run`

- **Lokasi:** `crates/swat-wasm/src/lib.rs`, `pub unsafe extern "C" fn swat_run(ptr: *const u8, len: usize, flags: u32) -> *mut u8`
- **Padanan SWAT+:** program `main` secara keseluruhan.
- **Tujuan:** titik masuk ABI: menjalankan skenario dan mengembalikan hasil dalam satu penyangga.
- **Masukan dan keluaran:** `ptr`/`len` = arsip tar skenario; `flags` = lihat tabel bit di atas. Keluaran: pointer ke `[u32 LE panjang][tar]`.
- **Cara kerja:**
  1. Bentuk *slice* `from_raw_parts(ptr, len)` atas masukan (tanpa menyalin).
  2. Panggil `run_tar(input, Options::from_flags(flags), progress)` dengan *callback* yang memanggil impor `swat_progress(d as u32, n as u32)`.
  3. Buat `Vec` berkapasitas `4 + out.len()`, isi 4 bita panjang (`(out.len() as u32).to_le_bytes()`) lalu isi tar.
  4. Ubah ke *boxed slice* lalu kembali ke `Vec` (`into_boxed_slice().into_vec()`) dan `shrink_to_fit`, sehingga kapasitas dijamin sama dengan panjang; ini prasyarat `swat_free(ptr, 4 + panjang)`.
  5. `forget` dan kembalikan pointer.
- **Catatan port:** masukan tidak dibebaskan oleh `swat_run`; pemanggil membebaskannya sendiri (boleh segera setelah `swat_run` kembali). Panggilan bersifat sinkron dan memblokir *thread* pemanggil selama simulasi, karena itu dokumentasi menganjurkan menjalankannya dalam *Web Worker*. Galat Rust berupa `panic` tidak ditangkap: pada `wasm32-unknown-unknown` panic menjadi *trap* WebAssembly (`unreachable`) yang muncul sebagai pengecualian `RuntimeError` di JavaScript. Galat biasa (berkas tidak terbaca, dsb.) tidak menyebabkan *trap*, tetapi dilaporkan di `run.json`.

#### 16.1.7  Impor `env.swat_progress`

- **Lokasi:** `crates/swat-wasm/src/lib.rs`, blok `#[link(wasm_import_module = "env")] unsafe extern "C" { fn swat_progress(done: u32, days: u32); }`
- **Padanan SWAT+:** keluaran layar harian di `time_control.f90` (baris "Executing year/month/day" yang ditulis ke `*`).
- **Tujuan:** memberi tahu pemanggil kemajuan simulasi tanpa *glue* JavaScript.
- **Masukan dan keluaran:** `done` = hari yang sudah selesai, `days` = total hari periode `time.sim`. Tidak ada nilai balik.
- **Cara kerja:** pemanggil wajib menyediakan objek impor `{ env: { swat_progress: (done, days) => … } }` saat `WebAssembly.instantiate`; modul memanggilnya pada setiap persen bulat (lihat 16.1.3). Bila impor tidak disediakan, instansiasi modul gagal.
- **Catatan port:** tanggal (`date`) yang ada pada event JSON CLI tidak diteruskan ke impor ini; hanya dua bilangan bulat.

#### 16.1.8  `tests::osu_through_tar`

- **Lokasi:** `crates/swat-wasm/src/lib.rs`, `#[test] fn osu_through_tar()`
- **Padanan SWAT+:** tidak ada; uji port.
- **Tujuan:** memastikan jalur tar penuh bekerja pada skenario `Osu_1hru` dari Rust (*native*), tanpa memerlukan runtime WebAssembly.
- **Cara kerja:**
  1. Baca semua berkas di `testdata/Osu_1hru`, beri nama `TxtInOut/<berkas>` (menguji pembuangan folder tunggal), kemas dengan `tar::write`.
  2. Jalankan `run_tar` dengan `Options::from_flags(3)` (teks dan Parquet) dan penghitung panggilan progres berbasis `Rc<Cell<usize>>`.
  3. Periksa: `run.json` mengandung `"ok":true`; arsip memuat `hru_wb_aa.parquet` dan `hru_wb_aa.txt`; jumlah panggilan progres lebih dari 50.

### 16.2  Program baris perintah (`crates/swatplus-rust/src/main.rs`)

Biner `swatplus-rust` adalah program yang dijalankan *runner* RustGIS dan pengguna lain. File `main.rs` menolak `unwrap`, `expect`, `panic!`, `unimplemented!`, dan `unreachable!` lewat `#![deny(clippy::…)]`, sehingga setiap galat harus menjadi kode keluar, bukan *panic*. Teks bantuan disimpan di konstanta `USAGE`:

```text
swatplus-rust inspect <TxtInOut folder>
swatplus-rust run <TxtInOut folder> [--out <folder>] [--fix-swat] [--format text|parquet|both] [--progress json] [--strict]
swatplus-rust compare <SWAT+ table> <our table> [--abs 0.001] [--rel 1e-4]
```

Selain itu `--version` atau `-V` mencetak `swatplus-rust <versi>` (pada rilis ini `swatplus-rust 1.0.1`).

**Opsi `run`.**

| Opsi | Bawaan | Arti |
| --- | --- | --- |
| `--out <folder>` | folder TxtInOut itu sendiri (seperti SWAT+) | tempat tabel ditulis; dibuat bila belum ada |
| `--fix-swat` | mati | mode terkoreksi (`docs/swat-fixes.md`); hasil berbeda dari SWAT+ |
| `--format text\|parquet\|both` | `text` | tabel teks SWAT+, Parquet, atau keduanya |
| `--progress json` | mati | progres dan event sebagai baris JSON di *stdout*; tanpa ini pesan ke *stderr* |
| `--strict` | mati | tulis tabel lengkap (juga Parquet), lalu keluar dengan kode 3 bila sebagian proyek tidak disimulasikan |

**Kode keluar** (sesuai `docs/runner.md` dan kode):

| Kode | Arti |
| --- | --- |
| 0 | selesai (`run`, `inspect`, `--version`); pada `compare`: semua nilai cocok |
| 1 | galat: proyek tidak terbaca atau gagal dijalankan, folder atau tabel tidak dapat ditulis, berkas `compare` tidak terbuka; pada `compare` juga: ada ketidakcocokan, baris hilang/berlebih, urutan baris salah, atau tidak ada kolom yang dibandingkan |
| 2 | argumen salah (perintah tak dikenal, opsi tak dikenal, nilai opsi hilang atau tidak valid) |
| 3 | `run --strict`: sebagian proyek tidak disimulasikan |

**Event JSON** (`--progress json`), satu objek per baris di *stdout*:

| Event | Field | Kapan |
| --- | --- | --- |
| `start` | `version`, `days`, `first`, `last` (tanggal `yyyy-mm-dd`) | setelah proyek terbaca dan folder keluaran dibuat |
| `progress` | `done`, `days`, `date` (hari yang sedang dimulai) | setiap persen bulat dan pada hari terakhir |
| `note` | `text` | catatan (mode `--fix-swat`, bagian yang tidak disimulasikan) |
| `table` | `file`, dan `rows` untuk Parquet | setiap berkas yang ditulis |
| `done` | `days`, `seconds` | terakhir, bila berhasil |
| `error` | `text` | terakhir, bila gagal |

**Struct `RunArgs<'a>`** (privat): `out: &str` (folder keluaran), `fixes: bool`, `text: bool` (pertahankan tabel teks), `parquet: bool` (tulis Parquet), `json: bool` (event JSON), `strict: bool`.

#### 16.2.1  `main`

- **Lokasi:** `crates/swatplus-rust/src/main.rs`, `fn main() -> ExitCode`
- **Padanan SWAT+:** program `main` (`main.f90.in`), yang tidak membaca argumen apa pun.
- **Tujuan:** mengurai argumen dan meneruskan ke `inspect`, `run`, atau `compare_cmd`.
- **Masukan dan keluaran:** `std::env::args()` tanpa nama program → `ExitCode`.
- **Cara kerja:**
  1. Argumen dikumpulkan sebagai `Vec<&str>` lalu dicocokkan dengan pola *slice*: `["inspect", dir]`, `["run", dir, opts @ ..]`, `["compare", expected, actual, opts @ ..]`, `["--version"] | ["-V"]`; selain itu cetak `USAGE` ke *stderr* dan keluar 2. Catatan: `inspect` dengan argumen tambahan juga dianggap salah.
  2. Untuk `run`, `RunArgs` diawali dengan `out = dir`, `text = true`, lainnya `false`. Iterator opsi diperiksa berpasangan (opsi saat ini, intipan nilai berikutnya lewat `it.clone().next()`):
     - `--out v`: `out = v`, lompati `v`;
     - `--fix-swat`: `fixes = true`;
     - `--format v` dengan `v` ∈ {text, parquet, both}: `text = (v != "parquet")`, `parquet = (v != "text")`, lompati `v`;
     - `--progress json`: `json = true`, lompati nilai;
     - `--strict`: `strict = true`;
     - selainnya (termasuk `--out` tanpa nilai, format tak dikenal, `--progress` selain `json`): cetak `USAGE`, keluar 2.
  3. Opsi boleh diulang; nilai terakhir yang menang (mis. dua `--format`).
- **Catatan port:** sejak 1.0.1 `--out` hanya menerima nilai yang tidak diawali `--`, sehingga `--out --strict` tidak lagi memperlakukan `--strict` sebagai nama folder, melainkan ditolak sebagai argumen salah (kode 2).

#### 16.2.2  `inspect`

- **Lokasi:** `crates/swatplus-rust/src/main.rs`, `fn inspect(dir: &str) -> ExitCode`
- **Padanan SWAT+:** tidak ada; khusus port (SWAT+ hanya menulis ringkasan pembacaan ke berkas log dan layar selama berjalan).
- **Tujuan:** membaca skenario tanpa menjalankannya dan mencetak ringkasan untuk pemeriksaan cepat; CI menjalankannya pada `Osu_1hru` dan `Ames_sub1` sebagai uji asap pembaca.
- **Masukan dan keluaran:** folder TxtInOut → teks di *stdout*; kode 0, atau 1 bila `load_project` gagal (pesan `error: …` di *stderr*).
- **Cara kerja:** mencetak berurutan:
  1. `Scenario` (folder).
  2. `Period`: tahun awal dan hari Julian awal (`t.normalized().day_start`, tiga digit), tahun akhir dan hari Julian hari terakhir dari `t.days()`, jumlah tahun (`t.years()`), jumlah hari, dan langkah waktu (`t.normalized().step`).
  3. `Warm-up`: `project.print.nyskip` tahun yang tidak dicetak.
  4. `Objects`: untuk setiap `ObjectKind::ALL` yang berjumlah > 0, nama jenis, jumlah objek, dan jumlah luas `area_ha` (ha, dijumlahkan dalam `f32`, dua desimal).
  5. `Weather`: jumlah stasiun dan stasiun generator cuaca (`wgn`); lalu untuk setiap `ClimateVar::ALL` dan setiap seri berkas variabel itu: nama berkas, jumlah rekaman, rekaman pertama dan terakhir (`tahun-hari`).
  6. `Missing` untuk setiap berkas di `LoadReport::missing` (berkas yang disebut `file.cio` atau daftar stasiun tetapi tidak ada; SWAT+ melewatinya lewat `inquire (exist=…)`).
  7. `Not read` untuk setiap berkas di `LoadReport::unread` (berkas stasiun yang berakhir sebelum *header*-nya lengkap; SWAT+ berhenti membaca stasiun variabel itu di sana, sehingga berkas sesudahnya juga tidak terbaca).

#### 16.2.3  `emit`

- **Lokasi:** `crates/swatplus-rust/src/main.rs`, `fn emit(line: &str)`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** menulis satu baris ke *stdout* dan langsung mem-*flush*, dengan mengabaikan pipa yang tertutup.
- **Cara kerja:** kunci `stdout`, `writeln!`, `flush`; hasil keduanya dibuang (`let _ =`).
- **Catatan port:** `println!` akan *panic* bila pembaca pipa sudah menutupnya (mis. `| head`); `emit` menghindari itu. *Flush* setiap baris penting agar *runner* menerima event progres seketika, bukan tertahan di *buffer*.

#### 16.2.4  `run`

- **Lokasi:** `crates/swatplus-rust/src/main.rs`, `fn run(dir: &str, o: &RunArgs) -> ExitCode`
- **Padanan SWAT+:** program `main` secara keseluruhan (pembacaan, `time_control`, penulisan keluaran).
- **Tujuan:** menjalankan simulasi penuh dari folder dan menulis tabel ke folder keluaran.
- **Masukan dan keluaran:** folder TxtInOut dan `RunArgs` → berkas tabel di `out`, event atau pesan, kode keluar 0/1/3.
- **Cara kerja:**
  1. Catat waktu mulai (`Instant::now`). Siapkan *closure* `note(text)` (event `note` atau `note: …` di *stderr*) dan `fail(text)` (event `error` atau `error: …` di *stderr*, mengembalikan kode 1).
  2. `load_project(&DirSource::new(dir))`; gagal → `fail`.
  3. Susun judul tabel (sama persis dengan 16.1.3 langkah 3), buat folder `out` dengan `create_dir_all`; gagal → `fail`.
  4. Hitung `days = project.time.days()`. Dalam mode JSON, kirim event `start` dengan tanggal hari pertama dan terakhir; tanggal dibentuk dari tahun dan hari Julian dengan `swat_engine::clock::xmon` atas tabel panjang bulan kabisat (`NDAYS_LEAP`) atau bukan (`NDAYS_NOLEAP`) sesuai `is_leap(tahun)`.
  5. Buat `TextTables::new(judul, DirStore::new(out), &project.print)` (tabel ditulis langsung ke berkas sambil berjalan), set `fixes`, dan dalam mode JSON pasang `progress::Reporter` dengan `emit` sebagai keluaran.
  6. Bila `--fix-swat`, kirim catatan bahwa bug SWAT+ dikoreksi dan hasil akan berbeda.
  7. `run_with(&project, &mut tables, RunOptions { fixes })`:
     - galat → `fail`;
     - berhasil: kumpulkan alasan yang tidak `None` dari `hru_skipped`, `channel_skipped`, `routing_skipped`. Bila `--strict` dan ada alasan, simpan pesan "not simulated: …" (alasan digabung dengan ` ;  `); sejak 1.0.1 semua tabel tetap ditulis lengkap (juga Parquet), baru kemudian program mengirim `error` itu dan keluar **3**. Setiap alasan juga menjadi catatan: neraca air HRU tidak disimulasikan (`hru_wb_day.txt` hanya berisi presipitasi), saluran tidak dirutekan (meneruskan aliran masuknya), atau unit perutean dan akuifer tidak disimulasikan.
  8. `tables.finish()` menuntaskan tabel (mis. tabel rata-rata tahunan); gagal → `fail`. Ambil `store.names()`, lalu lepaskan `store` agar berkas tertutup sebelum dibaca ulang.
  9. Untuk setiap nama tabel:
     - bila `parquet`: buka `<out>/<nama>.txt`, tulis `<dasar>.parquet` dengan `text_to_parquet(BufReader, BufWriter, 1 << 20)`; berhasil → event `table` dengan `rows`; gagal → `fail`. Bila `text` tidak diminta, hapus berkas `.txt` (kegagalan menghapus diabaikan);
     - bila `text` dan mode JSON: event `table` untuk berkas `.txt`.
  10. Akhiri dengan event `done` (jumlah hari dan detik, tiga desimal) atau pesan `wrote the output tables to <out>`; kode 0.
- **Kebiasaan SWAT+ / catatan port:**
  - Dengan `--format parquet`, tabel teks tetap ditulis lebih dulu (pengonversi membaca teks), lalu dihapus. Akibatnya nilai Parquet adalah angka yang tercetak di tabel teks (presisi SWAT+), bukan nilai internal engine (lihat `docs/runner.md`).
  - Pada jalur `--strict` (kode 3), sejak 1.0.1 `tables.finish()` tetap dipanggil, sehingga tabel akhir periode dituntaskan dan Parquet dibuat sebelum event `error` dikirim; sebelumnya program keluar sebelum tabel dituntaskan.
  - Folder bawaan `--out` sama dengan folder masukan, seperti SWAT+; alur kerja validasi memakai `--out` terpisah agar tabel Fortran dan Rust tidak saling menimpa.
  - Engine berjalan pada satu *thread*. `docs/runner.md` menjelaskan bahwa HRU tidak dapat dijalankan paralel tanpa mengubah hasil, karena beberapa variabel SWAT+ membawa nilai satu HRU ke HRU berikutnya pada hari yang sama (bilangan acak tabel keputusan, permintaan hara tanaman, *enrichment ratio*, fraksi fosfor sedimen, status sistem septik). Paralelisme hanya cocok untuk mode `--fix-swat` kelak.

#### 16.2.5  `compare_cmd`

- **Lokasi:** `crates/swatplus-rust/src/main.rs`, `fn compare_cmd(expected: &str, actual: &str, opts: &[&str]) -> ExitCode`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** mengurai opsi toleransi dan memanggil `compare::compare` atas dua berkas tabel.
- **Masukan dan keluaran:** `expected` = tabel SWAT+ Fortran, `actual` = tabel port; opsi `--abs` (bawaan 0,001) dan `--rel` (bawaan 1e-4) → laporan di *stdout*, kode 0 (cocok), 1 (tidak cocok atau galat), 2 (opsi salah).
- **Cara kerja:**
  1. Setiap opsi mengonsumsi argumen berikutnya sebagai `f64`; opsi tak dikenal atau nilai tak terurai → `USAGE`, kode 2.
  2. Buka kedua berkas sebagai `BufReader`; gagal → `error: <path>: …`, kode 1.
  3. Label laporan = nama berkas `actual` tanpa folder.
  4. `compare(e, a, abs, rel, label)`: `Ok(true)` → 0; `Ok(false)` → 1; `Err` → pesan dan 1.

### 16.3  Pembanding tabel dan toleransi K12 (`crates/swatplus-rust/src/compare.rs`)

Modul `compare` adalah alat ukur proyek: ia memutuskan apakah port "sama dengan SWAT+". Tujuan proyek (README) adalah setiap keluaran harian cocok dengan SWAT+ Fortran commit 6f51536 pada skenario acuan dalam **toleransi K12** (keputusan desain RustGIS): selisih relatif 1e-4 **atau** satu satuan pada desimal terakhir yang dicetak. SWAT+ mencetak sebagian besar nilai dengan tiga desimal, maka `--abs` bawaan adalah 0,001. Alasannya: dua nilai yang berbeda sangat tipis di dalam presisi `f32` bisa dibulatkan ke digit terakhir yang bertetangga saat dicetak, sehingga perbedaan satu digit terakhir bukan bukti ketidaksamaan perhitungan.

Kedua berkas dibaca sebagai aliran (*stream*) secara serempak, bukan dimuat utuh ke memori: SWAT+ menulis baris hari demi hari dan unit demi unit, sehingga kunci (tahun, hari, unit) naik di kedua berkas. Dengan cara ini tabel harian proyek besar (Batanghari, yang dapat mencapai gigabita) tetap dapat dibandingkan.

**Tipe dan struct.**

| Nama | Isi |
| --- | --- |
| `type Key = (i64, i64, i64, i64)` | (tahun, hari Julian, unit, langkah waktu); langkah = 0 untuk tabel non-subharian |
| `struct Rows<R>` | tabel yang sedang dibaca: `names` (nama kolom, duplikat diberi akhiran `#k`), `lines` (iterator baris), `cols` (indeks kolom `yr`, `jday`, dan opsional `unit`), `tstep` (indeks kolom `tstep` bila ada), `last` (kunci baris terakhir) |
| `struct ColumnStats` | per kolom: `values` (jumlah pasangan nilai yang dibandingkan), `mismatches`, `max_abs`, `max_rel`, `first` (deskripsi ketidakcocokan pertama) |

#### 16.3.1  `Rows::new`

- **Lokasi:** `crates/swatplus-rust/src/compare.rs`, `fn new(r: R) -> Result<Rows<R>, String>` dengan `R: BufRead`
- **Padanan SWAT+:** tidak ada; membaca format tabel yang ditulis subrutin keluaran SWAT+ (baris judul, baris nama kolom, baris satuan, lalu data).
- **Tujuan:** membaca kepala tabel dan menemukan kolom kunci.
- **Cara kerja:**
  1. Lewati baris pertama (judul).
  2. Baris kedua adalah nama kolom; tidak ada → galat `no header line`.
  3. Pisahkan nama dengan spasi. Nama yang berulang (mis. `null` untuk suhu tiap hidrograf) dibedakan menurut urutan: kemunculan ke-k (k ≥ 2) menjadi `nama#k`. Karena kedua berkas memakai aturan yang sama, kolom duplikat tetap berpasangan sesuai urutan.
  4. Cari `yr` dan `jday`; tidak ada → galat. `unit` dan `tstep` opsional.
- **Catatan port:** baris ketiga (satuan) tidak dilewati secara khusus; ia tersaring di `next_row` karena tidak memiliki angka bulat di kolom `yr` dan `jday`.

#### 16.3.2  `Rows::next_row`

- **Lokasi:** `crates/swatplus-rust/src/compare.rs`, `fn next_row(&mut self) -> Result<Option<(Key, Vec<String>)>, String>`
- **Padanan SWAT+:** tidak ada.
- **Tujuan:** mengembalikan baris data berikutnya beserta kuncinya, atau `None` di akhir berkas.
- **Cara kerja:**
  1. Untuk setiap baris, pisahkan dengan spasi.
  2. Baris yang kolom `yr` atau `jday`-nya bukan bilangan bulat dilewati (baris satuan, baris kosong).
  3. Unit: dari kolom `unit` bila ada (bukan bilangan bulat → baris dilewati). Tanpa kolom `unit` (mis. `ru_day.txt`), baris diberi nomor urut dalam harinya: `lu + 1` bila (tahun, hari) sama dengan baris sebelumnya, selain itu 1.
  4. Langkah: dari kolom `tstep` bila ada (bukan bilangan bulat → baris dilewati), selain itu 0.
  5. Kunci harus **naik tegas** dibanding baris sebelumnya (urutan leksikografis tuple); bila tidak, galat `rows out of order at year … day … unit … step …`.
- **Catatan port:** syarat kenaikan tegas berarti tabel yang tidak urut menurut (tahun, hari, unit) tidak dapat dibandingkan. Ini sesuai untuk tabel harian dan subharian; tabel bulanan dan tahunan juga memuat `yr` dan `jday` sehingga dapat dibandingkan sepanjang urutannya naik.

#### 16.3.3  `compare`

- **Lokasi:** `crates/swatplus-rust/src/compare.rs`, `pub fn compare(expected: impl BufRead, actual: impl BufRead, abs: f64, rel: f64, label: &str) -> Result<bool, String>`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** membandingkan tabel SWAT+ (`expected`) dengan tabel port (`actual`) kolom demi kolom, mencetak laporan, dan mengembalikan apakah semuanya cocok.
- **Masukan dan keluaran:** dua aliran tabel, toleransi `abs` (dalam satuan kolom, mis. mm atau kg/ha) dan `rel` (tanpa satuan), label laporan → `Ok(true/false)` atau `Err` (kepala tidak valid, urutan salah, galat baca).
- **Cara kerja:**
  1. Baca kepala kedua tabel dengan `Rows::new`.
  2. Tentukan kolom yang dibandingkan: setiap kolom di tabel **port** yang bukan kolom kunci/nama (`jday`, `mon`, `day`, `yr`, `unit`, `gis_id`, `name`, `type`, `tstep`) dan yang namanya juga ada di tabel SWAT+. Kolom yang tidak ditulis port tidak dibandingkan (port menulis kolom sesuai cakupannya); kolom port yang tidak ada di SWAT+ diabaikan.
  3. Gabung-urut (*merge*) kedua aliran menurut kunci:
     - hanya SWAT+ tersisa, atau kunci SWAT+ < kunci port: baris SWAT+ hilang dari port (`missing_rows`), dihitung dalam `rows`;
     - hanya port tersisa, atau kunci port < kunci SWAT+: baris port berlebih (`extra_rows`);
     - kunci sama: bandingkan setiap kolom terpilih.
  4. Untuk setiap pasangan nilai yang keduanya dapat diurai sebagai angka `f64` berhingga, hitung selisih mutlak `d` dan relatif `r`, perbarui `max_abs` dan `max_rel`. Sejak 1.0.1, angka melawan NaN, tak hingga, atau tanda bintang `****` (format Fortran yang meluap) dihitung sebagai beda; NaN sama dengan NaN, tak hingga sama dengan tak hingga bertanda sama, dan tanda bintang sama dengan teks yang sama. Teks nama tetap tidak dibandingkan. Pasangan angka dinyatakan **tidak cocok** hanya bila kedua syarat berikut berlaku:

```latex
d = |x - y|, \qquad r = \begin{cases} \dfrac{d}{\max(|x|, |y|)} & d > 0 \\ 0 & d = 0 \end{cases}
```

```latex
\text{tidak cocok} \iff d > \mathrm{abs}\,(1 + 10^{-6}) \;\wedge\; r > \mathrm{rel}
```

5. Ketidakcocokan pertama per kolom dicatat sebagai `<tahun> day <hari> unit <unit> [step <k>]: SWAT+ <x> vs <y> (<kolom>)`.
6. Cetak laporan: baris ringkas `<label>: <rows> rows; columns compared: <n>`, lalu per kolom status `ok` atau `MISMATCH`, jumlah nilai, jumlah ketidakcocokan, `max abs` (4 desimal), `max rel` (notasi ilmiah), dan baris `first:` bila ada; lalu jumlah baris SWAT+ yang hilang dan baris port yang berlebih.
7. Hasil `true` hanya bila tidak ada baris hilang, tidak ada baris berlebih, minimal satu kolom dibandingkan, dan semua kolom tanpa ketidakcocokan.

- **Kebiasaan SWAT+ / catatan port:**
  - Faktor `(1 + 10^{-6})` adalah "sedikit kelonggaran": dalam biner, 0,144 − 0,143 sedikit lebih besar dari 0,001, sehingga tanpa faktor ini selisih tepat satu digit terakhir akan dianggap tidak cocok.
  - Selisih relatif dihitung terhadap nilai **yang lebih besar** dari keduanya, bukan terhadap nilai SWAT+, sehingga simetris.
  - Karena syaratnya "dan", nilai besar (mis. volume aliran m3) dinilai dengan `rel`, sedangkan nilai kecil di dekat nol dinilai dengan `abs`.
  - Penguraian `f64` di Rust menerima `NaN` dan `inf`. Sebelum 1.0.1, selisih yang melibatkan NaN menghasilkan NaN dan perbandingan `>` dengan NaN bernilai salah, sehingga pasangan itu tidak pernah dihitung sebagai ketidakcocokan; kini nilai yang bukan angka berhingga dibandingkan menurut aturan di atas dan tidak memengaruhi `max_abs`/`max_rel`.
  - Jumlah `rows` menghitung baris SWAT+ (cocok atau hilang), bukan baris port berlebih.

#### 16.3.4  `tests::matches_and_counts_missing_rows`

- **Lokasi:** `crates/swatplus-rust/src/compare.rs`, `#[test]`
- **Padanan SWAT+:** tidak ada; uji port.
- **Tujuan:** memastikan aturan toleransi dan penghitungan baris hilang. Uji kedua, `nan_and_overflow_differ_from_numbers` (1.0.1), memastikan NaN dan tanda bintang melawan angka dihitung beda, sedangkan NaN dengan NaN, `Infinity` dengan `inf`, dan bintang dengan bintang dianggap sama.
- **Cara kerja:** dengan kepala sintetis (kolom kunci, `name`, dan dua kolom nilai `a`, `b`) dan `abs = 0,001`, `rel = 1e-4`:
  1. Port kehilangan satu baris (unit 2 hari pertama) → `Ok(false)`.
  2. Semua baris ada, satu nilai berbeda 0,001 (2,000 vs 2,001) → `Ok(true)` (tepat satu digit terakhir, berkat kelonggaran 1e-6).
  3. Satu nilai berbeda 0,003 (relatif sekitar 1,5e-3) → `Ok(false)`.

### 16.4  Pemanggil, format tabel Parquet, dan perbedaan pustaka matematika

#### 16.4.1  Skrip `scripts/wasm-run.mjs`

- **Lokasi:** `scripts/wasm-run.mjs` (modul ES untuk Node; logika yang sama berlaku di peramban). Pemakaian: `node scripts/wasm-run.mjs <swat_wasm.wasm> <scenario.tar> <out.tar> [flags]`, `flags` bawaan 3.
- **Padanan SWAT+:** tidak ada.
- **Tujuan:** contoh acuan pemakaian ABI tanpa *glue*, dan alat CI untuk menjalankan `Osu_1hru` lewat WebAssembly.
- **Cara kerja:**
  1. `WebAssembly.instantiate(bita wasm, { env: { swat_progress } })`; fungsi progres menghitung panggilan dan mencetak `progress done/days` setiap panggilan ke-20.
  2. Ambil ekspor `memory`, `swat_alloc`, `swat_free`, `swat_run`.
  3. `ptr = swat_alloc(n)`; salin arsip tar masukan ke `new Uint8Array(memory.buffer, ptr, n)`.
  4. `res = swat_run(ptr, n, flags)`; lalu `swat_free(ptr, n)`.
  5. Baca panjang dengan `new DataView(memory.buffer).getUint32(res, true)` (*little-endian*), salin `len` bita mulai `res + 4` (`.slice()` agar lepas dari memori modul), `swat_free(res, len + 4)`.
  6. Tulis tar keluaran dan cetak jumlah panggilan progres serta ukuran keluaran.
- **Catatan:** *view* atas `memory.buffer` selalu dibuat baru setelah `swat_run`, karena simulasi dapat memperbesar memori dan membuat `ArrayBuffer` lama terlepas (*detached*).

#### 16.4.2  Kontrak tabel Parquet (`docs/runner.md`)

Pengonversian sendiri dibahas di bab keluaran (`swat_output::parquet::text_to_parquet`); di sini hanya kontrak yang dijanjikan kepada pemanggil. Setiap tabel teks SWAT+ `<tabel>_<day|mon|yr|aa>.txt` (sesuai `print.prt`) menjadi `<tabel>_<periode>.parquet` dengan kolom yang sama, urutan sama, dan nama sama (nama berulang diberi akhiran `_2`):

| Jenis kolom | Kolom | Tipe Parquet |
| --- | --- | --- |
| kunci | `jday`, `mon`, `day`, `yr`, `unit`, `gis_id`, `tstep`, `year`, `plant_no` | bilangan bulat 32 bit |
| nama | `name`, `type`, `plantnm`, `plant_name` | string UTF-8 |
| nilai | semua kolom lain | `float` 32 bit: angka yang tampil di tabel teks (presisi SWAT+); NaN bila SWAT+ mencetak NaN atau tanda bintang |

Berkas dikodekan *plain*, tanpa kompresi, satu *row group* per sejuta baris (2^20). DuckDB, Arrow, Polars, dan DuckDB-Wasm dapat membacanya. Perhatikan bahwa penamaan kolom berulang berbeda dari alat `compare` (`_2` di Parquet, `#2` di dalam `compare`).

#### 16.4.3  Perbedaan `libm` di WebAssembly

Hasil modul WebAssembly **dapat berbeda pada bit terakhir** dari program *native*. Menurut `docs/runner.md`, di target `wasm32-unknown-unknown` fungsi matematika (`exp`, `ln`, `powf`, dan fungsi trigonometri) adalah implementasi Rust sendiri (port dari musl), bukan glibc; beberapa di antaranya membulatkan bit terakhir sebagian hasil presisi tunggal (`f32`) secara berbeda. Operasi aritmetika dasar (+, −, ×, ÷, `sqrt`) sudah ditetapkan IEEE 754 dan identik di kedua target; perbedaan hanya datang dari fungsi transendental.

Angka yang dilaporkan dokumen: pada `Osu_1hru` selama 14 tahun, 17 dari 18 tabel identik bita demi bita; tabel rata-rata tahunan unit perutean, yang dicetak dengan 9 angka penting, berbeda pada dua nilai sebesar 1 dan 2 satuan pada tempat terakhir `f32` (ULP). Semua tabel tetap cocok dalam toleransi K12. Program *native* di Linux memakai glibc, sama dengan acuan Fortran (gfortran juga memanggil libm sistem), sehingga validasi bit-for-bit terhadap SWAT+ dilakukan di Linux. Di macOS dan Windows pustaka matematika sistem dapat berbeda dengan cara yang sama.

Konsekuensi praktis:

1. Kesamaan bit-for-bit dengan SWAT+ hanya dijanjikan untuk biner Linux/glibc; WASM dan biner macOS/Windows dijanjikan dalam toleransi K12.
2. Pada skenario yang lebih panjang atau lebih kompleks, selisih satu ULP dalam satu hari dapat merambat melalui variabel keadaan (air tanah, biomassa); CI hanya menguji `Osu_1hru`, sehingga besarnya perambatan pada skenario lain belum diukur di WASM (tidak dinyatakan di repo).
3. Mode `--fix-swat` tidak mengubah hal ini; sumbernya pustaka matematika, bukan logika model.

### 16.5  Validasi dan rilis di GitHub Actions (`.github/workflows/*.yml`)

Empat alur kerja menjaga kedua pintu di atas. Ringkasannya:

| Alur kerja | Pemicu | Yang diperiksa atau dihasilkan |
| --- | --- | --- |
| `ci.yml` (CI) | *push* ke `main`, *pull request*, manual | format, lint, uji, build WASM, kesetaraan WASM vs *native*, Parquet, MSRV |
| `validate.yml` (Validate against SWAT+) | *push* ke `main`, *pull request*, manual | menjalankan SWAT+ Fortran dan port pada puluhan skenario lalu `compare` setiap tabel |
| `fortran.yml` (SWAT+ Fortran build) | manual, atau perubahan berkas alur itu sendiri | membangun SWAT+ Fortran acuan dan menyimpannya di cabang `fortran-bin` |
| `release.yml` (Release) | *tag* `v*` | biner untuk empat target dan modul WASM, dipublikasikan sebagai *GitHub release* |

#### 16.5.1  `ci.yml`

Semua *job* berjalan di `ubuntu-latest` dengan `RUSTFLAGS=-D warnings` (peringatan kompilasi menjadi galat).

1. **test** (Format, lint, test): `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings` (diagnostik juga dijadikan anotasi `::error::`); `cargo test --workspace` (termasuk `osu_through_tar` dan uji `compare`); lalu `swatplus-rust inspect` pada `testdata/Osu_1hru` dan `testdata/Ames_sub1` sebagai uji asap pembaca.
2. **wasm** (WebAssembly build): bangun `swat_wasm.wasm` untuk `wasm32-unknown-unknown`; bangun biner *native*; kemas `Osu_1hru` menjadi tar; jalankan `node scripts/wasm-run.mjs … 3`; pastikan `run.json` berisi `"ok":true`; jalankan `swatplus-rust run` *native* ke folder lain. Setiap tabel `.txt` dibandingkan bita demi bita dengan `cmp`; tabel yang berbeda menghasilkan peringatan (bukan galat) dan harus lolos `swatplus-rust compare` dengan toleransi bawaan. *Job* gagal hanya bila ada tabel di luar toleransi.
3. **parquet** (Parquet tables read by DuckDB and Arrow): salin `Osu_1hru`, ubah `print.prt` dengan `scripts/print-daily.sh` agar semua tabel utama dicetak harian, jalankan `run --format both --progress json`, lalu `scripts/check-parquet.py` membandingkan setiap tabel Parquet (dibaca DuckDB dan pyarrow) dengan tabel teksnya.
4. **msrv**: `cargo build --workspace` dengan Rust 1.90 (versi minimum yang didukung).

#### 16.5.2  `validate.yml`

Inilah pembuktian utama klaim bit-for-bit (dalam K12) terhadap SWAT+ Fortran.

1. Ambil biner `swatplus-fortran` dari cabang `fortran-bin` (lihat 16.5.3) dan bangun `swatplus-rust` mode *release*.
2. Buat varian skenario: `scripts/make-variants.sh` (varian fase 2 dari `Ames_sub1`), `scripts/make-phase3-variants.py` (Batanghari dan `Bh_land`, dipotong sampai 2019), `scripts/make-pessel-variants.py` (dataran pantai Pessel 2015–2017 dan varian `Ps_*`, termasuk padi sawah `Ps_paddy`), `scripts/make-reservoir-variants.py` (Batanghari dengan waduk dan sumber titik: `Bh_res`, `Bh_rel`, Bh\_nosed), dan Pessel utuh untuk 2015 saja dengan pencetakan sejak hari pertama.
3. Untuk setiap skenario dalam daftar (`Osu_1hru`, `Ames_sub1`, varian `Ames_*`, `Bh_land`, `Batanghari`, `Bh_res`, `Bh_rel`, `Bh_nosed`, varian `Ps_*`, dan `Pessel`): salin ke `runs/<skenario>/fortran`, ubah `print.prt` dengan `print-daily.sh` agar tabel yang dibandingkan dicetak harian, jalankan SWAT+ Fortran dan pastikan `success.fin` ada, lalu jalankan port pada folder yang sama dengan `--out runs/<skenario>/rust` (port tidak membaca keluaran SWAT+ sebagai masukan).
4. Untuk setiap kombinasi tabel (`hru_wb`, `hru_pw`, `hru_nb`, `hru_ls`, `ru`, `aquifer`, `channel_sd`, `basin_*`, `lsunit_*`, `basin_sd_cha`, `reservoir`, `basin_res`, `recall`, `basin_psc`) dan periode (`day`, `mon`, `yr`, `aa`, `subday`) yang ditulis port: `swatplus-rust compare` dengan toleransi bawaan; hasil menjadi anotasi `::notice` atau `::error`. Tabel yang tidak ditulis port (misalnya tabel hara untuk HRU yang tidak disimulasikan penuh) dilewati.
5. `scripts/compare-yields.py` membandingkan hasil panen tanaman.
6. Tabel harian dan subharian dihapus setelah dibandingkan (tabel harian Batanghari bisa bergigabita).
7. *Job* gagal bila ada satu saja tabel atau hasil panen yang tidak cocok; batas waktu 60 menit.

#### 16.5.3  `fortran.yml`

Membangun SWAT+ Fortran pada commit acuan `6f515368525e7d84089ae0f9d33f4c9115de5679` di `ubuntu-24.04` dengan gfortran, CMake + Ninja, mode Release, tertaut statis. Biner diuji asap pada `refdata/Osu_1hru` milik repositori SWAT+ (harus menghasilkan `success.fin`), lalu dipaksa-dorong (*force push*) bersama `README.txt` (commit, versi kompiler, tanggal, sha256, tautan run, lisensi LGPL-2.1) ke cabang `fortran-bin`. Cabang dipakai sebagai penyimpanan karena artefak *workflow* tidak selalu dapat diunduh; dengan begitu `validate.yml` cukup memakai `git fetch` dan `git show`.

#### 16.5.4  `release.yml`

Dipicu *tag* `v*`.

1. **build** (matriks): `x86_64-unknown-linux-gnu` (ubuntu-latest), `aarch64-apple-darwin` (macos-latest), `x86_64-apple-darwin` (macos-13), `x86_64-pc-windows-msvc` (windows-latest). Masing-masing `cargo build --release --locked -p swatplus-rust --target …`, dikemas bersama `LICENSE`, `NOTICE`, `README.md`, dan `docs/runner.md` sebagai `.tar.gz` (atau `.zip` untuk Windows).
2. **wasm**: `cargo build --release --locked -p swat-wasm --target wasm32-unknown-unknown`; dikemas bersama `scripts/wasm-run.mjs`, `LICENSE`, `NOTICE`, dan `docs/runner.md`.
3. **publish**: unduh semua artefak dan `gh release create <tag>` dengan catatan rilis dari `CHANGELOG.md`.

Catatan: rilis tidak menjalankan ulang validasi terhadap SWAT+; validasi bergantung pada `validate.yml` yang berjalan pada setiap *push* ke `main`. Biner macOS dan Windows tidak diuji bit-for-bit di CI; sesuai 16.4.3, kesetaraannya dengan SWAT+ hanya dapat diharapkan dalam toleransi K12.

### 16.6  Daftar periksa fungsi bab ini

| Fungsi | Lokasi | Subbab |
| --- | --- | --- |
| `Options::from_flags` | `swat-wasm/src/lib.rs` | 16.1.1 |
| `run_tar` | `swat-wasm/src/lib.rs` | 16.1.2 |
| `run_inner` | `swat-wasm/src/lib.rs` | 16.1.3 |
| `ffi::swat_alloc` | `swat-wasm/src/lib.rs` | 16.1.4 |
| `ffi::swat_free` | `swat-wasm/src/lib.rs` | 16.1.5 |
| `ffi::swat_run` | `swat-wasm/src/lib.rs` | 16.1.6 |
| impor `swat_progress` | `swat-wasm/src/lib.rs` | 16.1.7 |
| `tests::osu_through_tar` | `swat-wasm/src/lib.rs` | 16.1.8 |
| `main` | `swatplus-rust/src/main.rs` | 16.2.1 |
| `inspect` | `swatplus-rust/src/main.rs` | 16.2.2 |
| `emit` | `swatplus-rust/src/main.rs` | 16.2.3 |
| `run` | `swatplus-rust/src/main.rs` | 16.2.4 |
| `compare_cmd` | `swatplus-rust/src/main.rs` | 16.2.5 |
| `Rows::new` | `swatplus-rust/src/compare.rs` | 16.3.1 |
| `Rows::next_row` | `swatplus-rust/src/compare.rs` | 16.3.2 |
| `compare` | `swatplus-rust/src/compare.rs` | 16.3.3 |
| `tests::matches_and_counts_missing_rows` | `swatplus-rust/src/compare.rs` | 16.3.4 |
