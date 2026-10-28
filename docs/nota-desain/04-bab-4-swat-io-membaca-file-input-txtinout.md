## Bab 4  swat-io: membaca file input TxtInOut

Bab ini membahas crate `swat-io`, lapisan paling bawah engine: kode yang mengubah teks file masukan SWAT+ di folder *TxtInOut* (`file.cio`, `time.sim`, `print.prt`, `codes.bsn`, `parameters.bsn`, file `*.con`, file iklim, tabel basis data, `soils.sol`, `management.sch`, tabel keputusan, `septic.str`, `recall.rec`, file kalibrasi) menjadi struktur data crate `swat-model` (`Project`, `TimeSim`, `PrintSettings`, `SpatialObject`, `Table`, dan seterusnya). Crate ini tidak menghitung proses hidrologi apa pun; tugasnya hanya memastikan bahwa sebuah file *berarti hal yang sama* bagi port Rust seperti bagi SWAT+ Fortran: nilai yang sama, nilai bawaan (default) yang sama, dan kelonggaran yang sama terhadap file yang cacat atau tidak lengkap.

File Rust yang dicakup: `crates/swat-io/src/lib.rs`, `source.rs`, `list.rs`, `table.rs`, `cio.rs`, `time.rs`, `print.rs`, `basin.rs`, `connect.rs`, `routing.rs`, `climate.rs`, `hru.rs`, `septic.rs`, `recall.rs`, `cal.rs`, `project.rs`, ditambah uji integrasi `crates/swat-io/tests/refdata.rs`. Hampir setiap pembaca bergantung pada `list.rs`, emulasi masukan *list-directed* Fortran (`read (unit, *) a, b, c`) persis seperti gfortran memprosesnya; karena itu `list.rs` dibahas lebih dulu dan paling rinci, sebab aturan pemotongan token, nilai *null*, garis miring, dan perilaku akhir file di sana menentukan arti setiap file lainnya.

Dalam alur simulasi, `swat-io` dipanggil sekali di awal: `load_project` (subbab 4.9) membaca `file.cio`, lalu mengikuti urutan pembacaan SWAT+ (`readcio_read` → `basin_read_cc`/`basin_read_prm` → `time_read` → `basin_print_codes_read` → `basin_read_objs` → `hyd_read_connect` → `cli_staread`/`cli_wgnread` → basis data → `hru_read` → data terukur iklim). Hasilnya, sebuah `Project` plus `LoadReport` (daftar file yang hilang atau tidak terbaca), diteruskan ke `swat-engine` (bab-bab berikutnya) yang melakukan inisialisasi dan simulasi harian. Crate ini tidak pernah menyentuh disk secara langsung: semua file datang melalui trait `FileSource`, sehingga engine yang sama dapat dijalankan dari folder di disk maupun dari berkas yang dipegang di memori (misalnya di peramban). Seluruh crate dikompilasi dengan larangan `unwrap`, `expect`, `panic`, `unimplemented`, dan `unreachable` di luar uji (atribut `deny(clippy::…)` di `lib.rs`): file yang rusak menghasilkan `Error`, bukan kepanikan program.

### 4.1  Fondasi: galat, dekode teks, dan sumber file (`lib.rs`, `source.rs`)

Subbab ini mencakup tipe galat crate, fungsi dekode byte menjadi teks, dan abstraksi sumber file. Tidak ada padanan langsung di SWAT+: Fortran membuka file dengan `open` dan memeriksa keberadaannya dengan `inquire (file=…, exist=…)`; `FileSource::read` yang mengembalikan `None` adalah padanan `exist = .false.`.

**Tipe penting.**

| Tipe | Lokasi | Isi |
| --- | --- | --- |
| `Error` (enum) | `lib.rs` | `Parse { file, line, msg }` nilai tidak dapat dikonversi (nomor baris 1-based dari awal *read*); `Eof { file }` *read* mencapai akhir file sebelum selesai (padanan `iostat < 0`); `Io { file, source }` galat sistem operasi selain *not found*; `Missing(file)` file wajib (`file.cio`) tidak ada. |
| `Result<T>` | `lib.rs` | alias `std::result::Result<T, Error>`. |
| `FileSource` (trait) | `source.rs` | satu metode `read(&self, name) -> Result<Option<String>>`; nama adalah nama file atau jalur relatif seperti yang ditulis di file SWAT+. |
| `DirSource` | `source.rs` | folder skenario di disk (field privat `root: PathBuf`). |
| `MemSource` | `source.rs` | `HashMap<String, String>` nama → teks; untuk skenario yang dibongkar di peramban atau dibangun dalam uji. |

Modul publik yang diekspor: `basin, cal, cio, climate, con, hru, list, print, project, recall, routing, septic, source, table, time`, dan ekspor ulang `load_project`, `LoadReport`, `DirSource`, `FileSource`, `MemSource`.

#### 4.1.1  `decode`

- **Lokasi:** `crates/swat-io/src/lib.rs`, `pub fn decode(bytes: &[u8]) -> String`
- **Padanan SWAT+:** tidak ada; khusus port (Fortran membaca byte apa adanya).
- **Tujuan:** mengubah isi mentah file menjadi `String` tanpa pernah gagal.
- **Masukan dan keluaran:** byte file → teks.
- **Cara kerja:**
  1. Coba tafsirkan byte sebagai UTF-8 (`std::str::from_utf8`). Bila sah, kembalikan salinannya.
  2. Bila tidak sah, petakan setiap byte `b` ke karakter Unicode dengan kode `b` (tafsiran Latin-1). Setiap urutan byte dengan demikian selalu menjadi teks.
- **Kebiasaan SWAT+ / catatan port:** file SWAT+ pada dasarnya ASCII, tetapi sebagian membawa byte Latin-1 (nama yang ditulis alat lama). Fortran tidak peduli pengodean; port meniru ketidakpedulian itu dengan pemetaan byte-ke-karakter sehingga tidak ada file yang ditolak karena pengodean.

#### 4.1.2  `Error::fmt` (implementasi `Display`)

- **Lokasi:** `lib.rs`, `impl std::fmt::Display for Error`, `fn fmt(&self, f) -> std::fmt::Result`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** pesan galat yang dapat dibaca manusia.
- **Cara kerja:** `Parse` → `"<file>: line <n>: <msg>"`; `Eof` → `"<file>: unexpected end of file"`; `Io` → `"<file>: <galat OS>"`; `Missing` → `"<file>: not found"`.

#### 4.1.3  `Error::source` (implementasi `std::error::Error`)

- **Lokasi:** `lib.rs`, `fn source(&self) -> Option<&(dyn Error + 'static)>`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** merangkai galat: hanya varian `Io` yang memiliki penyebab di bawahnya (`std::io::Error`); varian lain mengembalikan `None`.

#### 4.1.4  `FileSource::read` (metode trait)

- **Lokasi:** `source.rs`, `fn read(&self, name: &str) -> Result<Option<String>>`
- **Padanan SWAT+:** pasangan `inquire (file=…, exist=i_exist)` + `open` + pembacaan isi.
- **Tujuan:** kontrak tunggal untuk memperoleh teks sebuah file berdasarkan nama.
- **Masukan dan keluaran:** nama file (dapat memuat jalur relatif, misalnya awalan folder data iklim dari `file.cio`) → `Ok(Some(teks))` bila ada, `Ok(None)` bila tidak ada, `Err` untuk galat lain.
- **Kebiasaan SWAT+ / catatan port:** membedakan "tidak ada" (`None`) dari "galat" penting karena SWAT+ melewati file yang tidak ada secara diam-diam pada kebanyakan pembaca (cabang `.not. i_exist`), sedangkan `load_project` mencatatnya di `LoadReport::missing`.

#### 4.1.5  `DirSource::read`

- **Lokasi:** `source.rs`, `impl FileSource for DirSource`
- **Padanan SWAT+:** `inquire` + `open` relatif terhadap direktori kerja.
- **Tujuan:** membaca file dari folder skenario.
- **Cara kerja:**
  1. Gabungkan `root` dengan `name` (`PathBuf::join`).
  2. `std::fs::read`; bila berhasil, `decode` byte-nya dan kembalikan `Some`.
  3. Bila galat berjenis `NotFound`, kembalikan `Ok(None)`.
  4. Galat lain (izin, perangkat) menjadi `Error::Io { file: name, source }`.

#### 4.1.6  `MemSource::insert`

- **Lokasi:** `source.rs`, `pub fn insert(&mut self, name: impl Into<String>, bytes: &[u8])`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** menyimpan satu file di memori; byte langsung didekode dengan `decode`, sehingga semua `read` berikutnya murah dan tidak dapat gagal karena pengodean. Nama yang sama menimpa isi lama.

#### 4.1.7  `MemSource::read`

- **Lokasi:** `source.rs`, `impl FileSource for MemSource`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** mengembalikan salinan teks (`cloned()`) untuk nama yang persis sama, atau `None`. Tidak pernah mengembalikan `Err`. Pencocokan nama peka huruf besar-kecil dan tidak menormalkan jalur.

**Fungsi trivial subbab 4.1**

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `DirSource::new(root: impl Into<PathBuf>) -> Self` | `source.rs` | membuat sumber dari folder TxtInOut. |
| `MemSource::new() -> Self` | `source.rs` | sumber memori kosong (`Default`). |

### 4.2  Masukan *list-directed* Fortran (`list.rs`)

Hampir semua subrutin baca SWAT+ memakai pernyataan `read (unit, *) v1, v2, …` (masukan *list-directed*). Arti sebuah file SWAT+ ditentukan bukan oleh nama kolom di baris judulnya, melainkan oleh aturan gfortran untuk pernyataan itu. `list.rs` menirukan aturan tersebut sebagai mesin pembaca *record* (baris) yang dipakai semua modul lain. Aturan yang ditiru (dari komentar modul dan `docs/swat-quirks.md`):

1. Setiap `read` mulai pada *record* (baris) baru. Bila baris memuat lebih sedikit nilai daripada yang diminta, pembacaan berlanjut ke baris berikutnya; nilai sisa di baris terakhir yang dipakai dibuang.
2. Nilai dipisahkan oleh spasi, tab, atau satu koma. Dua koma berturutan (`,,`) atau koma di awal menghasilkan nilai *null*: variabel Fortran tidak diubah (tetap bernilai default).
3. Garis miring `/` mengakhiri *read*: variabel yang tersisa tidak diubah.
4. `r*c` berarti nilai `c` diulang `r` kali; `r*` berarti `r` nilai *null*.
5. String bertanda kutip (`'…'` atau `"…"`) boleh memuat spasi, koma, dan garis miring; string tanpa kutip berakhir pada spasi, koma, garis miring, atau akhir baris.
6. Baris kosong dilewati ketika sebuah *read* membutuhkan nilai.
7. *Read* yang mencapai akhir file sebelum lengkap adalah `iostat < 0` (di port: `Error::Eof`).
8. Nilai yang tidak dapat dikonversi ke tipe variabel menghentikan *read* dengan `iostat > 0`; SWAT+ umumnya hanya memeriksa `eof < 0`, sehingga galat ini diabaikan diam-diam dan variabel sesudahnya tetap default.

**Tipe penting.**

| Tipe | Isi |
| --- | --- |
| `Item = Option<String>` | satu nilai hasil *read*; `None` = *null* (variabel mempertahankan default). |
| `Records<'a>` | `file: &str` (nama untuk pesan galat), `lines: Vec<&str>` (baris file, akhiran `\r` dibuang), `pos: usize` (indeks baris berikutnya). |
| `Mark(usize)` | posisi baris yang dapat dikembalikan, padanan `backspace`/`rewind` Fortran (boleh mundur beberapa baris sekaligus). |
| `Stop` (enum privat) | `Slash` (garis miring menghentikan *read*) atau `Full` (jumlah nilai sudah terpenuhi). |
| `Fields` | hasil satu *read*: `file`, `line` (nomor baris awal, 1-based), `items: Vec<Item>`; menyediakan akses bertipe. |

#### 4.2.1  `Records::new`

- **Lokasi:** `crates/swat-io/src/list.rs`, `pub fn new(file: &'a str, text: &'a str) -> Self`
- **Padanan SWAT+:** `open (unit, file=…)` (posisi di awal file).
- **Tujuan:** memecah teks menjadi *record* dan menaruh kursor di baris pertama.
- **Cara kerja:**
  1. `text.lines()` memecah pada `\n`; setiap baris yang berakhir `\r` dipotong akhiran itu (file Windows CRLF dibaca sama dengan file Unix).
  2. `pos = 0`.
- **Kebiasaan SWAT+ / catatan port:** `lines()` tidak menghasilkan baris kosong ekstra setelah `\n` terakhir, sama dengan gfortran yang tidak melihat *record* setelah akhir baris terakhir.

#### 4.2.2  `Records::raw`

- **Lokasi:** `list.rs`, `pub fn raw(&mut self) -> Option<&'a str>`
- **Padanan SWAT+:** `read (unit, '(A)') line` (pembacaan berformat seluruh baris).
- **Tujuan:** mengambil satu baris utuh, kosong atau tidak, dan maju satu baris; `None` di akhir file.
- **Kebiasaan SWAT+ / catatan port:** dipakai `cio::read` untuk baris folder keluaran, yang di `readcio_read` sengaja dibaca berformat `'(A)'` supaya spasi dan `/` dalam jalur tidak memotong nilai. Juga dipakai di dalam `read` untuk mengambil baris demi baris.

#### 4.2.3  `Records::skip`

- **Lokasi:** `list.rs`, `pub fn skip(&mut self) -> Result<Item>`
- **Padanan SWAT+:** `read (unit, *) titldum` / `read (unit, *) header`.
- **Tujuan:** membaca satu nilai (biasanya judul atau kepala kolom) lalu pindah ke baris berikutnya.
- **Cara kerja:** memanggil `read(1)` dan mengembalikan nilai tunggalnya (`None` bila *null*).
- **Kebiasaan SWAT+ / catatan port:** karena *read* list-directed melewati baris kosong, `skip` juga melewati baris kosong sebelum judul. Nilai yang dikembalikan hanya kata pertama (mis. judul `"file.cio: test"` menjadi `"file.cio:"`), sama dengan `titldum` yang dibaca Fortran. Pada file kosong, `skip` menghasilkan `Error::Eof`. Pola "hitung baris dengan `skip` sampai `Eof`" dipakai untuk meniru penghitungan `imax` SWAT+ (`do; read (107,*,iostat=eof) titldum; if (eof<0) exit; imax=imax+1`), sehingga baris kosong tidak dihitung.

#### 4.2.4  `Records::read`

- **Lokasi:** `list.rs`, `pub fn read(&mut self, n: usize) -> Result<Fields>`
- **Padanan SWAT+:** `read (unit, *) v1, …, vn` dengan pemeriksaan `iostat < 0`.
- **Tujuan:** pembacaan list-directed `n` nilai.
- **Masukan dan keluaran:** jumlah nilai `n` → `Fields` berisi tepat `n` item, atau kurang bila garis miring mengakhiri *read*; `Error::Eof` bila akhir file tercapai sebelum `n` nilai terkumpul.
- **Cara kerja:**
  1. Catat nomor baris awal (`line()`).
  2. Kasus `n == 0`: *read* tanpa variabel tetap mengonsumsi satu *record* (`read (107,*)` tanpa daftar, dipakai di `dtbl_lum_read`). Bila tidak ada baris, `Eof`.
  3. Ulangi: ambil baris berikutnya dengan `raw`; bila tidak ada, `Eof`. Panggil `tokenize(baris, n - terkumpul, &mut items)`.
  4. Bila `tokenize` melaporkan `Slash` atau `Full`, berhenti; bila item sudah ≥ `n`, berhenti; selain itu lanjut ke baris berikutnya (baris kosong menghasilkan nol item dan dengan sendirinya terlewati).
  5. Potong `items` menjadi panjang `n` (pengulangan `r*c` dapat melampaui) dan kembalikan `Fields`.
- **Kebiasaan SWAT+ / catatan port:** nilai sisa pada baris terakhir dibuang karena *read* berikutnya selalu mulai di baris baru (`pos` sudah maju). `read` tidak memeriksa tipe; konversi dilakukan kemudian oleh `Fields::int`/`real`. Bila tipe juga harus menghentikan *read* (perilaku gfortran untuk nilai salah tipe), dipakai `read_checked`.

#### 4.2.5  `Records::read_partial`

- **Lokasi:** `list.rs`, `pub fn read_partial(&mut self, n: usize) -> Option<(Fields, bool)>`
- **Padanan SWAT+:** `read (unit, *, iostat=eof) …` di mana nilai yang sudah terbaca sebelum akhir file tetap tersimpan di variabel.
- **Tujuan:** *read* `n` nilai yang tidak gagal di akhir file, tetapi mengembalikan apa yang sempat terbaca beserta penanda `eof`.
- **Cara kerja:** `read_checked(n, |_, _| true)` (semua nilai dianggap dapat dikonversi).
- **Masukan dan keluaran:** `None` bila akhir file tercapai tanpa satu nilai pun; selain itu `(Fields, eof)`.
- **Kebiasaan SWAT+ / catatan port:** Fortran mengisi variabel satu per satu; ketika `eof` terjadi di tengah, variabel awal sudah terisi. Pembaca seperti `read_hru_data`, `read_aquifers`, `read_cal_parms`, `read_soils` memakai ini lalu menghentikan loop bila `eof` benar (meniru `if (eof < 0) exit` yang datang *setelah* nilai disimpan).

#### 4.2.6  `Records::read_checked`

- **Lokasi:** `list.rs`, `pub fn read_checked(&mut self, n: usize, ok: impl Fn(usize, &str) -> bool) -> Option<(Fields, bool)>`
- **Padanan SWAT+:** `read (unit, *, iostat=eof) tipe_turunan` ketika sebuah nilai gagal dikonversi (gfortran: *read* berhenti dengan galat, *read* berikutnya mulai pada *record* berikutnya).
- **Tujuan:** pembacaan yang berhenti pada baris tempat nilai pertama yang tidak lolos `ok(i, teks)` berada.
- **Cara kerja:**
  1. Ulangi per baris: ambil `raw`; bila habis, tandai `eof` dan berhenti.
  2. `tokenize` baris itu; periksa setiap item baru (indeks ≥ jumlah sebelum baris ini): bila ada yang bukan *null* dan `ok(i, teks)` salah, tandai `bad`.
  3. Berhenti bila `bad`, `Slash`, `Full`, atau item ≥ `n`.
  4. Bila tidak ada item dan `eof`, kembalikan `None`; selain itu potong ke `n` dan kembalikan `(Fields, eof)`.
- **Kebiasaan SWAT+ / catatan port:** item yang salah tipe tetap ada di `Fields`; yang menghentikan pengisian adalah `table::fill`, yang berhenti di nilai pertama yang tidak terkonversi. Akibat penting (dicatat di `docs/swat-quirks.md`): `plants.plt` Batanghari (SWAT+ Editor 4.1.0) memiliki 53 nilai plus deskripsi per baris sedangkan `plant_db` mempunyai 56 variabel; deskripsi gagal sebagai nilai ke-54, sehingga tiga fraksi lignin tetap default dan setiap baris tetap satu baris. Membaca terus ke baris berikutnya akan menggeser semua tanaman setelah yang pertama.

#### 4.2.7  `tokenize` (privat)

- **Lokasi:** `list.rs`, `fn tokenize(line: &str, want: usize, out: &mut Vec<Item>) -> Option<Stop>`
- **Padanan SWAT+:** pemindai leksikal masukan list-directed gfortran (pustaka *runtime*, bukan kode SWAT+).
- **Tujuan:** menambahkan paling banyak `want` nilai dari satu baris ke `out` dan melaporkan mengapa berhenti.
- **Cara kerja:**
  1. `target = out.len() + want`; baris diubah menjadi larik karakter; `after_value = false` mencatat apakah hal terakhir yang dilihat adalah sebuah nilai (menentukan apakah koma berarti *null*).
  2. Untuk setiap posisi: bila `out` sudah mencapai `target`, kembalikan `Full`.
  3. Spasi atau tab: lewati.
  4. Koma: bila `after_value` salah (koma di awal atau koma kedua berturutan), dorong `None`; set `after_value = false`.
  5. `/`: kembalikan `Slash` (sisa baris dan sisa variabel diabaikan).
  6. Kutip `'` atau `"`: panggil `quoted`, dorong teksnya, `after_value = true`.
  7. Selain itu: kumpulkan karakter sampai spasi, tab, koma, atau `/`. Bila token berbentuk pengulangan (`repeat`), dorong nilai ulang sebanyak hitungan (dibatasi `target`); bila tidak, dorong token apa adanya. `after_value = true`.
  8. Di akhir baris: `Full` bila target tercapai, `None` bila belum (pemanggil lanjut ke baris berikutnya).
- **Kebiasaan SWAT+ / catatan port:** satu koma setelah nilai hanyalah pemisah (`a, b ,c` → tiga nilai); `,2` → *null* lalu `2`. Garis miring di tengah teks tanpa kutip memotong token: `data/pcp.pcp` dibaca sebagai `data` lalu *read* berakhir; inilah alasan `readcio_read` membaca baris folder keluaran dengan format `'(A)'`. Koma di akhir baris tidak menghasilkan *null* tambahan (yang dihitung hanya koma kedua). Uji `blanks_tabs_and_commas`, `nulls_slash_and_repeats` memverifikasi aturan ini.

#### 4.2.8  `quoted` (privat)

- **Lokasi:** `list.rs`, `fn quoted(chars: &[char], start: usize, quote: char) -> (String, usize)`
- **Padanan SWAT+:** konstanta karakter bertanda kutip dalam masukan list-directed.
- **Tujuan:** membaca string bertanda kutip mulai dari `start`.
- **Cara kerja:** mulai setelah kutip pembuka; kutip yang sama dua kali berturut (`''`) menghasilkan satu kutip literal; kutip tunggal menutup string dan fungsi mengembalikan indeks setelahnya. Bila baris habis tanpa kutip penutup, seluruh sisa baris menjadi teks.
- **Kebiasaan SWAT+ / catatan port:** kutip jenis lain di dalam string tidak istimewa (`"it''s"` tetap `it''s`). String kutip yang tidak ditutup tidak berlanjut ke baris berikutnya.

#### 4.2.9  `repeat` (privat)

- **Lokasi:** `list.rs`, `fn repeat(token: &str) -> Option<(usize, Item)>`
- **Padanan SWAT+:** faktor pengulangan `r*c` list-directed.
- **Tujuan:** mengenali `r*c` dan `r*`.
- **Cara kerja:** pisahkan pada `*` pertama; bagian kiri harus angka desimal tak kosong dan bukan 0 (selain itu token biasa). Bagian kanan kosong → nilai `None` (*null* berulang); selain itu `Some(c)`. Hitungan dibatasi 100 000.
- **Kebiasaan SWAT+ / catatan port:** batas 100 000 adalah pelindung khusus port agar file jahat tidak meminta miliaran nilai; `tokenize` juga hanya mendorong sampai `target`. Contoh: `3*0.5 2*` → `0.5, 0.5, 0.5, null, null`.

#### 4.2.10  `Records::eof` (privat)

- **Lokasi:** `list.rs`, `fn eof(&self) -> Error`
- **Padanan SWAT+:** kondisi `iostat < 0`.
- **Tujuan:** membangun `Error::Eof { file }` dengan nama file pembaca.

#### 4.2.11  `Fields::get` dan `Fields::bad` (privat)

- **Lokasi:** `list.rs`, `fn get(&self, i) -> Option<&str>`, `fn bad(&self, i, what: &str) -> Error`
- **Padanan SWAT+:** tidak ada; khusus port.
- **Tujuan:** `get` mengembalikan teks nilai ke-`i` atau `None` bila *null* atau tidak ada (garis miring/di luar panjang); `bad` membangun `Error::Parse` dengan pesan `value <i+1> ("<teks>") is not <what>` dan nomor baris awal *read*.

#### 4.2.12  `Fields::int`

- **Lokasi:** `list.rs`, `pub fn int(&self, i: usize) -> Result<Option<i32>>`
- **Padanan SWAT+:** konversi ke variabel `integer` dalam *read* list-directed.
- **Tujuan:** nilai ke-`i` sebagai bilangan bulat.
- **Cara kerja:** `None` bila *null*/hilang; selain itu `parse_int`; gagal → `Error::Parse` ("an integer").
- **Kebiasaan SWAT+ / catatan port:** angka dengan titik desimal (`1.0`, `0.00000`) ditolak, sama dengan gfortran. Pemanggil memutuskan apakah galat ini fatal (kebanyakan pembaca: `?`) atau diabaikan seperti SWAT+ (`read_params`, `read_hru_data`).

#### 4.2.13  `Fields::real`

- **Lokasi:** `list.rs`, `pub fn real(&self, i: usize) -> Result<Option<f32>>`
- **Padanan SWAT+:** konversi ke variabel `real` (presisi tunggal) dalam *read* list-directed.
- **Tujuan:** nilai ke-`i` sebagai `f32`.
- **Cara kerja:** `None` bila *null*/hilang; selain itu `parse_real`; gagal → `Error::Parse` ("a number").
- **Kebiasaan SWAT+ / catatan port:** teks desimal dibulatkan **sekali** langsung ke `f32` (bukan ke `f64` lalu ke `f32`), sama seperti gfortran membaca `real` default. Pembulatan ganda dapat menghasilkan bit berbeda pada kasus batas, sehingga ini penting untuk kecocokan *bit-for-bit*.

#### 4.2.14  `parse_int`

- **Lokasi:** `list.rs`, `pub fn parse_int(s: &str) -> Option<i32>`
- **Padanan SWAT+:** konversi integer gfortran.
- **Tujuan:** menafsirkan tanda opsional (`+`/`-`) diikuti digit.
- **Cara kerja:** lepaskan tanda; sisa harus tak kosong dan hanya digit ASCII; parse sebagai `i64` lalu `i32::try_from` (nilai di luar rentang 32-bit → `None`).
- **Kebiasaan SWAT+ / catatan port:** `99999999999` ditolak (melampaui `integer` 4-byte), `1.0` ditolak.

#### 4.2.15  `parse_real`

- **Lokasi:** `list.rs`, `pub fn parse_real(s: &str) -> Option<f32>`
- **Padanan SWAT+:** konversi `real` gfortran.
- **Tujuan:** menerima semua bentuk bilangan real Fortran: `1`, `1.`, `.5`, `1.5e3`, `1.5d3`, `1.5q3`, eksponen tanpa huruf (`1.5+3` = 1500), serta `inf` dan `nan`.
- **Cara kerja:**
  1. Ubah ke huruf kecil; bila parser `f32` Rust langsung menerimanya, selesai.
  2. Ganti `d` dan `q` dengan `e`.
  3. Cari tanda `+`/`-` yang bukan karakter pertama dan tidak didahului `e`; sisipkan `e` sebelumnya (eksponen tanpa huruf).
  4. Parse sebagai `f32`; tolak bila teks memuat huruf selain `e` dan hasilnya hingga (mencegah kata seperti `infinity` diterima sebagai angka hingga yang aneh).
- **Kebiasaan SWAT+ / catatan port:** `0.1` menghasilkan tepat `0.1_f32`. Pembacaan `-99` (nilai hilang data iklim) menghasilkan `-99.0`.

**Fungsi trivial subbab 4.2**

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `Records::file(&self) -> &str` | `list.rs` | nama file seperti diberikan. |
| `Records::line(&self) -> usize` | `list.rs` | nomor baris (1-based) tempat *read* berikutnya mulai. |
| `Records::at_end(&self) -> bool` | `list.rs` | tidak ada baris tersisa. |
| `Records::mark(&self) -> Mark` | `list.rs` | simpan posisi (padanan sebelum `backspace`). |
| `Records::reset(&mut self, Mark)` | `list.rs` | kembali ke posisi tersimpan (dibatasi panjang file); padanan `backspace`/`rewind`. |
| `Fields::len`, `Fields::is_empty` | `list.rs` | jumlah item / apakah kosong. |
| `Fields::text(i) -> Option<String>` | `list.rs` | nilai sebagai string, `None` bila *null* atau hilang. |
| `Fields::text_or(i, default) -> String` | `list.rs` | nilai sebagai string, `default` bila *null* atau hilang. |
| `Fields::int_or(i, default) -> Result<i32>` | `list.rs` | `int` dengan default untuk *null*/hilang; galat konversi tetap diteruskan. |
| `Fields::real_or(i, default) -> Result<f32>` | `list.rs` | `real` dengan default untuk *null*/hilang. |

**Uji unit `list.rs`.** Fungsi bantu uji `read(text, n)` dan `s(v)`, serta uji `blanks_tabs_and_commas`, `extra_values_are_skipped_and_short_records_continue` (sisa nilai dibuang; baris pendek berlanjut), `blank_records_are_skipped`, `nulls_slash_and_repeats`, `quoted_strings`, `end_of_file`, `crlf_and_mark`, `numbers`, dan `typed_access` (tiga nilai tanpa baris berikut → `Eof`; dengan `/` → tiga item dan default untuk yang keempat).

### 4.3  Tabel basis data generik (`table.rs`)

Banyak file basis data SWAT+ dibaca dengan pola yang sama: satu tipe turunan Fortran per baris, `read (unit, *) db(i)`. Contoh: `topography.hyd` (`topo_read`), `hydrology.hyd`, `field.fld`, `landuse.lum`, `cntable.lum`, `ovn_table.lum`, `cons_practice.lum`, `snow.sno`, `tillage.til`, `plants.plt` (`plant_parm_read`), `nutrients.sol`, `urban.urb`, `fertilizer.frt`, `irr.ops`, `chem_app.ops`, `harv.ops`, file reservoir/lahan basah, `tiledrain.str`, `septic.sep`, `filterstrip.str`, `grassedww.str`, `bmpuser.str`, `weir.res`, file kanal `*.cha`/`*.lte`, `om_water.ini`, `puddle.ops`, `manure_om.frt`, `manure_db.frt`, `transplant.ops`, `carbon.bsn`, dan `carbon_lyr.bsn`. Port tidak menulis pembaca terpisah untuk masing-masing; sebagai gantinya setiap tipe Fortran dideskripsikan sebagai skema di `swat_model::schema` dan dibaca oleh tiga fungsi di `table.rs`.

**Tipe penting (dari `swat_model::table`).**

| Tipe | Isi |
| --- | --- |
| `Field { name, kind, default, count }` | satu komponen tipe turunan: nama, jenis (`Kind::Int`, `Real`, `Text`), nilai default sebagaimana tertulis di definisi Fortran (string, mis. `".02"`), dan panjang larik (1 untuk skalar). |
| `Value` | `Int(i32)`, `Real(f32)`, atau `Text(String)`. |
| `Table { file, fields, rows }` | skema dan baris; setiap baris adalah nilai-nilai komponen dalam urutan tipe, larik dibentangkan. Pencarian `find(name)` mengembalikan baris pertama yang nama (komponen pertama)-nya sama menurut perbandingan string Fortran (spasi di akhir diabaikan). |

#### 4.3.1  `read_table`

- **Lokasi:** `crates/swat-io/src/table.rs`, `pub fn read_table(file: &str, text: &str, fields: &'static [Field]) -> Result<Table>`
- **Padanan SWAT+:** pola umum `*_read` basis data, misalnya `topo_read`, `hydrol_read`, `plant_parm_read`, `cntbl_read`, dan seterusnya (dua tahap: hitung `imax`, `rewind`, lalu `read (107,*,iostat=eof) db(i)` untuk `i = 1..imax`).
- **Tujuan:** membaca seluruh tabel menjadi `Table` dengan nilai default untuk komponen yang tidak terbaca.
- **Masukan dan keluaran:** nama file, teks, skema → `Table` (baris dalam urutan file, indeks 0-based; padanan `db(1)` Fortran adalah `rows[0]`).
- **Cara kerja:**
  1. Lewati judul dan kepala kolom (`skip` dua kali).
  2. Tandai posisi badan, lalu hitung baris: panggil `skip` sampai `Eof`, tambah `imax` setiap kali (baris kosong tidak dihitung). Kembali ke tanda (`rewind` + dua `read` judul di Fortran).
  3. Hitung lebar baris `width = Σ count` dan daftar jenis per posisi (larik dibentangkan).
  4. Ulangi `imax` kali: `read_checked(width, converts)`, dengan `converts` menerima teks bila jenis posisi itu `Int` dan `parse_int` berhasil, `Real` dan `parse_real` berhasil, atau `Text`. Hasilnya diisi ke baris lewat `fill`. Bila `read_checked` mengembalikan `None`, atau menandai `eof`, loop berhenti (setelah baris terakhir disimpan).
- **Kebiasaan SWAT+ / catatan port:** baris yang pendek mengambil nilai dari baris berikutnya (aturan list-directed), sehingga sebuah baris kekurangan kolom dapat "memakan" baris setelahnya; uji `rows_defaults_and_lookup` menunjukkan baris pendek terakhir yang bertemu akhir file tetap disimpan dengan default untuk sisanya. Nilai yang tidak terkonversi menghentikan baris itu (komponen berikutnya default) dan baris berikutnya dibaca dari *record* setelahnya (uji `arrays_and_bad_values`, juga kasus `plants.plt` Batanghari di 4.2.6). Akhir file mengakhiri tabel. Nilai *null* (`,,`) membiarkan default.

#### 4.3.2  `read_record`

- **Lokasi:** `table.rs`, `pub fn read_record(file: &str, text: &str, fields: &'static [Field]) -> Result<Table>`
- **Padanan SWAT+:** `carbon_bsn_read` (satu *record* setelah dua baris kepala, dibaca tanpa toleransi).
- **Tujuan:** membaca file satu-baris menjadi tabel satu baris, dengan galat bila nilai hilang atau tidak terkonversi.
- **Cara kerja:**
  1. Lewati dua baris kepala; `read(width)` (galat `Eof` diteruskan).
  2. Bangun baris dengan `fill`.
  3. Periksa setiap posisi: *null* sah; teks sah; `Int` harus lolos `parse_int`; `Real` harus lolos `parse_real`; nilai yang hilang (garis miring memotong *read*) tidak sah.
  4. Bila ada yang tidak sah, `Error::Parse` di baris 3 dengan nama komponen; selain itu, kembalikan tabel berisi satu baris.
- **Kebiasaan SWAT+ / catatan port:** komentar kode menyatakan SWAT+ berhenti dengan galat bila *record* hilang atau nilai tidak terkonversi (pembacaan tanpa `iostat`); port meniru dengan mengembalikan `Err`. Hanya dipanggil bila `codes.bsn` `cswat == 2` (lihat 4.9).

#### 4.3.3  `fill`

- **Lokasi:** `table.rs`, `pub fn fill(fields: &[Field], items: &Fields) -> Vec<Value>`
- **Padanan SWAT+:** pengisian komponen tipe turunan satu per satu oleh *read* list-directed.
- **Tujuan:** membangun satu baris: default, lalu nilai terbaca ditimpakan sampai nilai pertama yang gagal.
- **Cara kerja:**
  1. `default_row(fields)`: tiap komponen diisi dari string default skema (bilangan diparse; gagal → 0).
  2. Untuk setiap posisi `i` (jenis dari skema): bila item ke-`i` tidak ada (garis miring atau *read* pendek), berhenti; bila *null*, lewati (default tetap); bila ada, konversi menurut jenis. Konversi berhasil → timpa; gagal → **berhenti** (semua komponen sesudahnya tetap default).
- **Kebiasaan SWAT+ / catatan port:** perilaku "berhenti di nilai buruk" adalah perilaku gfortran untuk galat konversi; karena SWAT+ hanya memeriksa `eof < 0`, galat ini tidak terlihat dan komponen setelahnya tetap bernilai inisialisasi tipenya.

**Uji unit `table.rs`:** `rows_defaults_and_lookup` (baris kosong tidak dihitung, baris pendek mendapat default, `find`), `arrays_and_bad_values` (larik `cn` dan penghentian pada `x`).

### 4.4  Pengaturan simulasi dan cekungan (`cio.rs`, `time.rs`, `print.rs`, `basin.rs`)

Empat file ini dibaca pertama kali oleh SWAT+ dan menentukan apa yang dibaca dan disimulasikan berikutnya. `file.cio` adalah daftar isi skenario; `time.sim` menetapkan periode dan langkah waktu; `print.prt` menetapkan keluaran; `codes.bsn` dan `parameters.bsn` menetapkan pilihan metode dan parameter tingkat cekungan.

**Tipe penting.**

| Tipe (crate `swat-model`) | Field utama |
| --- | --- |
| `FileCio` | `title`, `sections: Vec<CioSection>`, `out_path: Option<String>`; `CioSection { key, name_in_file, files: Vec<(slot, Option<nama file>)> }`. Skema `cio::SCHEMA` memuat urutan baris dan nama file default tiap slot. |
| `TimeSim` | `day_start`, `yrc_start`, `day_end`, `yrc_end` (hari Julian dan tahun kalender), `step` (langkah sub-harian per hari; ≤ 0 = harian). |
| `PrintSettings` | `nyskip` (tahun pemanasan yang tidak dicetak), jendela cetak (`day_start`, `yrc_start`, `day_end`, `yrc_end`), `interval`, `aa_years`, bendera `csvout`, `use_obj_labels` (kolom `dbout`), `cdfout`, `crop_yld`, `mgtout`, `hydcon`, `fdcout`, dan `objects: Vec<PrintObject { name, daily, monthly, yearly, avann }>`. |
| `BasinCodes` | kode metode `codes.bsn` (`pet`, `nam1`, `crk`, `swift_out`, `sed_det`, `rte`, `deg`, `wq`, `nostress`, `cn`, `cfac`, `cswat`, `lapse`, `uhyd`, `sed_ch`, `tdrn`, `wtdn`, `sol_p_model`, `gampt`, `atmo`, `smax`, `qual2e`, `gwflow`, `idc_till`) plus `petfile`, `wwqfile`. |
| `BasinParams` | `reals` (42 parameter real `parameters.bsn` dalam urutan tipe `basin_parms`), `day_lag_mx`, `igen` (default 5); metode `complete()` meniru `basin_prm_default`. |

#### 4.4.1  `cio::read`

- **Lokasi:** `crates/swat-io/src/cio.rs`, `pub fn read(text: &str) -> Result<FileCio>`
- **Padanan SWAT+:** `readcio_read` (`readcio_read.f90`).
- **Tujuan:** membaca `file.cio`, peta "slot → nama file" untuk seluruh skenario.
- **File dan kolom:** baris 1 judul; lalu satu baris per kelompok dalam urutan tetap (`simulation`, `basin`, `climate`, `connect`, `channel`, `reservoir`, `routing_unit`, `hru`, `exco`, `recall`, `dr`, `aquifer`, `herd`, `water_rights`, `link`, `hydrology`, `structural`, `hru_parm_db`, `ops`, `lum`, `chg`, `init`, `soils`, `decision_table`, `regions`, lalu lima baris jalur data iklim `pcp_path` … `wnd_path`); setiap baris berisi nama kelompok diikuti nama file per slot. Baris setelah jalur iklim berisi label dan folder keluaran.
- **Cara kerja:**
  1. Mulai dari `FileCio::defaults()` (semua slot bernama file default SWAT+).
  2. Judul = nilai pertama baris 1 (`skip`).
  3. Untuk setiap kelompok dalam `SCHEMA`: `read(1 + jumlah_slot)`. Bila `Eof`, kembalikan hasil sejauh ini (kelompok sisanya tetap default). Nilai 0 disimpan sebagai `name_in_file` (hanya informasi). Untuk slot ke-`i`: bila nilai `i+1` ada (bukan *null*/terpotong), `"null"` → `None` (slot mati), selain itu `Some(nama)`.
  4. Folder keluaran: ambil baris berikutnya utuh dengan `raw`; buang spasi awal; teks setelah spasi pertama (dipangkas) menjadi `out_path`, atau `None` bila kosong.
- **Kebiasaan SWAT+ / catatan port:** baris dibaca **berdasarkan posisi**, bukan nama di awal baris (uji `rows_by_position_and_defaults`: nama `anything` diabaikan). Baris dengan nama file lebih banyak dari slot membuang kelebihannya (aturan list-directed; Ames\_sub1 baris `init` memuat 12 nama untuk 11 slot). Baris yang kurang nama berlanjut ke baris berikutnya; bila bertemu akhir file, `Eof` dan default dipertahankan. Folder keluaran dibaca seperti `readcio_read` membaca `line_buffer` dengan format `'(A)'` lalu `adjustl` dan `index(line, ' ')`, sehingga jalur dengan `/` atau spasi tidak terpotong.

#### 4.4.2  `time::read`

- **Lokasi:** `crates/swat-io/src/time.rs`, `pub fn read(text: &str) -> Result<TimeSim>`
- **Padanan SWAT+:** `time_read` (`time_read.f90`).
- **Tujuan:** membaca periode simulasi dari `time.sim`.
- **File dan kolom:** judul, kepala, lalu `day_start yrc_start day_end yrc_end step`.
- **Cara kerja:** lewati dua baris; `read(5)`; setiap kolom dengan `int_or(…, 0)` (nilai *null* → 0). Nilai tidak bulat → `Error::Parse`.
- **Kebiasaan SWAT+ / catatan port:** nilai disimpan **sebagaimana tertulis**; penyesuaian SWAT+ dilakukan di `TimeSim::normalized` (swat-model): `step ≤ 0` → 1 (harian) dan `day_start ≤ 0` → 1. `day_end = 0` berarti 31 Desember tahun terakhir dan `day_end` di atas panjang tahun dibatasi (`time_control`), ditangani di `TimeSim::days`. Port menolak file yang nilainya bukan bilangan bulat, sedangkan Fortran hanya memeriksa `eof`; ini perbedaan ketegasan, bukan nilai.

#### 4.4.3  `print::read`

- **Lokasi:** `crates/swat-io/src/print.rs`, `pub fn read(text: &str) -> Result<PrintSettings>`
- **Padanan SWAT+:** `basin_print_codes_read` (`basin_print_codes_read.f90`).
- **Tujuan:** membaca pengaturan keluaran `print.prt`.
- **File dan kolom:**
  1. judul; kepala; `nyskip day_start yrc_start day_end yrc_end interval`;
  2. kepala; `aa_int_cnt` diikuti tahun-tahun rata-rata tahunan;
  3. kepala; `csvout dbout cdfout`;
  4. kepala; `crop_yld mgtout hydcon fdcout` (kepala di file contoh menamai kolom pertama `soilout`);
  5. kepala; lalu satu baris per objek: `nama daily monthly yearly avann`.
- **Cara kerja:**
  1. Lewati dua baris; `read(6)` untuk enam bilangan bulat (default 0).
  2. Lewati kepala; tandai posisi; `read(1)` untuk hitungan. Bila > 0: kembali ke tanda (`backspace`), `read(1 + n)` dan simpan `n` tahun (`n` dibatasi 10 000).
  3. Lewati kepala; `read(3)` untuk `csvout`, `use_obj_labels`, `cdfout`.
  4. Lewati kepala; `read(4)` untuk `crop_yld`, `mgtout`, `hydcon`, `fdcout`.
  5. Lewati kepala; baca baris objek `read(5)` sampai `Eof`, disimpan dengan nama dalam urutan file.
  6. Bila `use_obj_labels == "n"`: potong daftar menjadi 52 baris dan **ganti nama** setiap baris dengan nama pada posisi yang sama di `SEQUENTIAL` (`basin_wb`, `basin_nb`, …, `wetland_cs`).
- **Kebiasaan SWAT+ / catatan port:** tanpa label objek (`dbout` = `n`, kasus umum), `basin_print_codes_read` membaca baris dalam urutan tetap tanpa melihat namanya. Akibatnya file dengan baris tambahan menggeser bendera semua baris setelahnya: Osu\_1hru memuat baris `hru_cb` ekstra sehingga `channel_sd` dicetak dengan bendera baris `channel` (`docs/swat-quirks.md`). Port mempertahankan pergeseran itu dengan sengaja. Penyempurnaan jendela cetak (hari pertama 0 → 1, hari terakhir 0 → 366, tahun pertama 0 → tahun simulasi pertama, tahun terakhir 0 → satu setelah yang terakhir) dilakukan di `swat-engine`, bukan di sini.

#### 4.4.4  `basin::read_record` (privat)

- **Lokasi:** `crates/swat-io/src/basin.rs`, `fn read_record(file: &str, text: &str, n: usize) -> Result<Option<Fields>>`
- **Padanan SWAT+:** pola `read (107,*,iostat=eof) bsn_cc` / `bsn_prm` (satu *read* tipe turunan setelah judul dan kepala).
- **Tujuan:** satu *read* `n` nilai setelah dua baris kepala; `Ok(None)` bila file terlalu pendek (`Eof`).
- **Kebiasaan SWAT+ / catatan port:** komentar menyatakan bahwa pada akhir file Fortran mempertahankan nilai yang sudah terbaca; implementasi ini (memakai `read`, bukan `read_partial`) mengembalikan `None` dan pemanggil mempertahankan **semua** default. Uji `short_file_keeps_defaults` mengonfirmasi perilaku port: berkas pendek → seluruh default. Untuk file yang ditulis lengkap oleh SWAT+ Editor, kedua perilaku sama.

#### 4.4.5  `read_codes`

- **Lokasi:** `basin.rs`, `pub fn read_codes(text: &str) -> Result<BasinCodes>`
- **Padanan SWAT+:** `basin_read_cc` (`basin_read_cc.f90`), tipe `basin_control_codes`.
- **Tujuan:** membaca 26 nilai `codes.bsn`.
- **File dan kolom (posisi 0-based):** 0 `pet_file`, 1 `wq_file` (teks); 2–20 bilangan bulat `pet`, `nam1`, `crk`, `swift_out`, `sed_det`, `rte`, `deg`, `wq`, `nostress`, `cn`, `cfac`, `cswat`, `lapse`, `uhyd`, `sed_ch`, `tdrn`, `wtdn`, `sol_p_model`, `gampt`; 21 `atmo` (teks); 22 `smax`; 23 `qual2e`; 24 `gwflow`; 25 `idc_till`.
- **Cara kerja:**
  1. Mulai dari `BasinCodes::default()`; `read_record(…, 26)`; `None` → kembalikan default.
  2. Teks 0 dan 1 menimpa bila bukan *null*.
  3. Larik 20 referensi ke field integer dibagi dua di indeks 19: 19 pertama diisi dari posisi 2–20 (`int_or` dengan nilai lama sebagai default), lalu `atmo` dari posisi 21, lalu `smax` dari posisi 22; `qual2e`, `gwflow`, `idc_till` dari 23–25.
- **Kebiasaan SWAT+ / catatan port:** urutan mengikuti deklarasi tipe Fortran, dengan satu komponen karakter (`atmo`) di tengah deretan bilangan bulat. Nilai `null` di kolom teks disimpan sebagai string `"null"` (diperiksa pemakai). Nilai salah tipe di sini menjadi galat port (bukan diabaikan).

#### 4.4.6  `read_params`

- **Lokasi:** `basin.rs`, `pub fn read_params(text: &str) -> Result<BasinParams>`
- **Padanan SWAT+:** `basin_read_prm` (`basin_read_prm.f90`), tipe `basin_parms`.
- **Tujuan:** membaca 42 parameter real dan dua bilangan bulat (`day_lag_mx`, `igen`) dari `parameters.bsn`, berdasarkan posisi.
- **Cara kerja:**
  1. Mulai dari `BasinParams::default()`; `read_record(…, 44)`; `None` → default.
  2. Untuk `i = 0..41`: `real(i)`; *null* mempertahankan default; galat konversi → **kembalikan segera** (sisanya default).
  3. `int(42)` → `day_lag_mx`; galat → kembalikan segera.
  4. `int(43)` → `igen` bila berhasil.
- **Kebiasaan SWAT+ / catatan port:** meniru gfortran: nilai yang tidak terkonversi mengakhiri *read* dengan `iostat > 0`, yang diabaikan SWAT+. SWAT+ Editor menulis `day_lag_mx` sebagai `0.00000` yang bukan bilangan bulat, sehingga *read* berhenti di sana dan `igen` tetap 5; ini mengubah benih bilangan acak generator cuaca (`docs/swat-quirks.md`; uji `a_bad_integer_ends_the_read`). Setelah membaca, `load_project` memanggil `BasinParams::complete` (padanan `basin_prm_default`) yang mengganti nilai < 1e-6 dengan default tertentu.

**Uji unit subbab 4.4:** `cio::tests::rows_by_position_and_defaults`, `extra_names_are_ignored`; `time::tests::reads`; `print::tests::reads` (urutan tetap tanpa label; dengan `dbout` `y` nama asli dipakai), `no_intervals`; `basin::tests::codes`, `params_by_position`, `a_bad_integer_ends_the_read`, `short_file_keeps_defaults`.

### 4.5  Objek spasial dan unit (`connect.rs`, `routing.rs`)

SWAT+ memodelkan DAS sebagai graf objek spasial (HRU, HRU-lte, unit perutean/*routing unit*, akuifer, kanal, reservoir, *recall*, ekspor koefisien, *delivery ratio*, *outlet*, kanal SWAT-DEG, akuifer 2D). Setiap jenis punya file koneksi `*.con` yang menyatakan atribut objek dan aliran keluarnya; unit perutean dan unit lanskap juga punya file definisi dan elemen. Subbab ini membahas pembaca file tersebut serta `aquifer.aqu` dan `channel-lte.cha`, yang dibaca di modul yang sama.

**Tipe penting.**

| Tipe | Field utama (satuan) |
| --- | --- |
| `SpatialObject` | `kind: ObjectKind`, `id`, `name`, `gis_id`, `area_ha` (ha), `lat`, `lon` (derajat), `elev` (m), `props` (nomor basis data sifat), `weather_station` (nama stasiun cuaca), `constit`, `props2` (*overflow*), `ruleset`, `outflows: Vec<Outflow>`. |
| `Outflow` | `obj_type` (jenis objek tujuan, mis. `sdc`, `aqu`), `obj_id`, `hyd_type` (komponen hidrograf, mis. `tot`, `sur`, `lat`, `rhg`, `til`), `frac` (fraksi 0–1). |
| `RuElement` | `name`, `obtyp`, `obtypno`, `frac`, `dr_name` (nama *delivery ratio*). |
| `RuDef` | `name`, `elems: Vec<usize>` (nomor elemen hasil `define_unit_elements`). |
| `RuData` | `name`, `elem_def`, `elem_dr`, `topo`, `field` (nama rujukan basis data). |
| `LsuDef` | `name`, `area_ha`, `elems`. |
| `LsuElem` | `name`, `obtyp`, `obtypno`, `bsn_frac`, `ru_frac`. |
| `AquiferDb` | `name`, `init`, `flo` (mm), `dep_bot`, `dep_wt` (m), `no3`, `minp` (ppm), `cbn`, `flo_dist`, `bf_max` (mm), `alpha` (1/hari), `revap_co`, `seep`, `spyld`, `hlife_n` (hari), `flo_min`, `revap_min` (m). |
| `SdChannelData` | `name`, `init`, `hyd`, `sed`, `nut` (nama rujukan). |

Fungsi bantu `swat_model::cal::define_unit_elements` (padanan `define_unit_elements.f90`) mengubah daftar nomor dengan konvensi "angka negatif menutup rentang" menjadi daftar eksplisit, misalnya `[1, -4]` → `[1, 2, 3, 4]`; ia dipakai `read_ru_defs`, `read_lsu_defs`, dan `read_calibration`.

#### 4.5.1  `con::read`

- **Lokasi:** `crates/swat-io/src/connect.rs`, `pub fn read(kind: ObjectKind, file: &str, text: &str) -> Result<Vec<SpatialObject>>`
- **Padanan SWAT+:** `hyd_read_connect` (`hyd_read_connect.f90`).
- **Tujuan:** membaca satu file koneksi (`hru.con`, `rout_unit.con`, `aquifer.con`, `chandeg.con`, `reservoir.con`, `recall.con`, dan lain-lain) menjadi daftar objek.
- **File dan kolom:** judul, kepala, lalu per objek 13 nilai tetap (`FIXED`): `id name gis_id area lat lon elev props wst cst ovfl rule out_tot`, diikuti `out_tot` kuartet `obj_typ obj_id hyd_typ frac`.
- **Cara kerja:**
  1. Lewati dua baris. Ulangi sampai `Eof`:
  2. Tandai posisi; `read(13)`. `n_out = int_or(12, 0)` dibatasi ke `[0, 10000]`.
  3. Bila `n_out > 0`: kembali ke tanda dan `read(13 + 4·n_out)` (galat diteruskan, termasuk `Eof`). Bila tidak, pakai hasil pertama.
  4. Isi `SpatialObject` dari posisi 0–11 (bilangan bulat dan real default 0, teks default kosong), lalu `n_out` aliran keluar dari posisi `13 + 4k`.
- **Kebiasaan SWAT+ / catatan port:** setiap baris dibaca **dua kali**, seperti SWAT+ (sekali untuk nilai tetap, lalu `backspace` dan dibaca lagi dengan aliran keluar), sehingga aliran keluar boleh berlanjut ke baris berikutnya (uji `rows_without_outflows_and_continued_rows`). Port kembali ke awal *record* pertama baris objek, sedangkan `backspace` Fortran mundur tepat satu *record*; keduanya sama bila 13 nilai tetap berada di satu baris (selalu demikian di file tulisan SWAT+ Editor). Port tidak menghitung `area_ha_calc` atau mengalokasikan hidrograf; itu dilakukan engine. `load_project` memotong hasil ke jumlah objek menurut `object.cnt`.

#### 4.5.2  `count_rows` (privat)

- **Lokasi:** `crates/swat-io/src/routing.rs`, `fn count_rows(rec: &mut Records) -> Result<usize>`
- **Padanan SWAT+:** loop hitung `imax` standar (`read titldum` sampai `eof`).
- **Tujuan:** lewati dua baris kepala, hitung *record* tak-kosong sisanya dengan `skip`, lalu kembali ke awal badan; kembalikan jumlahnya.

#### 4.5.3  `read_ru_elements`

- **Lokasi:** `routing.rs`, `pub fn read_ru_elements(file: &str, text: &str) -> Result<Vec<RuElement>>`
- **Padanan SWAT+:** bagian pertama `ru_read_elements` (`ru_read_elements.f90`).
- **Tujuan:** membaca `rout_unit.ele`: elemen-elemen unit perutean, masing-masing ditempatkan pada nomornya sendiri.
- **File dan kolom:** judul, kepala, lalu `id name obj_typ obj_typ_no frac dlr` per baris.
- **Cara kerja:**
  1. Lewati dua baris; tandai badan.
  2. Lintasan hitung: `read(1)` berulang; nomor `i` (default 0). `i == 0` menghentikan hitungan; selain itu `imax = max(imax, i)` dan `n += 1`. `Eof` menghentikan.
  3. Kembali ke badan; alokasikan `imax + 1` elemen default (indeks 0 tak dipakai).
  4. Ulangi `n` kali: `read(6)`; tempatkan di `out[i]` bila `i` dalam rentang.
- **Kebiasaan SWAT+ / catatan port:** Fortran membaca nomor dulu, `backspace`, lalu baris lengkap ke `ru_elem(i)`, dalam loop `do isp = 1, imax`. Port mengulang sebanyak baris yang terhitung (`n`), yang sama dengan `imax` untuk penomoran berurutan 1..n. Penghentian pada nomor 0 adalah tambahan port (Fortran `ru_read_elements` hanya berhenti di `eof`; `ru_read` memang berhenti pada `i == 0`).

#### 4.5.4  `read_ru_defs`

- **Lokasi:** `routing.rs`, `pub fn read_ru_defs(file: &str, text: &str, n_ru: usize) -> Result<Vec<RuDef>>`
- **Padanan SWAT+:** bagian kedua `ru_read_elements` (pembacaan `rout_unit.def`).
- **Tujuan:** daftar elemen setiap unit perutean.
- **File dan kolom:** judul, kepala, lalu `id name elem_tot elem1 elem2 …`.
- **Cara kerja:**
  1. Lewati dua baris. Ulangi `n_ru` kali (jumlah objek `RoutingUnit` dari `rout_unit.con`, padanan `sp_ob%ru`):
  2. Tandai; `read(3)`; `nspu = int_or(2, 0)` (min 0). `Eof` menghentikan.
  3. Bila `nspu > 0`: kembali, `read(3 + nspu)`, ambil nama dan `nspu` nomor, lalu `define_unit_elements`.
- **Kebiasaan SWAT+ / catatan port:** sama dengan Fortran, nama hanya diisi bila `nspu > 0` (pada pembacaan kedua). Daftar boleh berlanjut ke baris berikutnya.

#### 4.5.5  `read_lsu_defs`

- **Lokasi:** `routing.rs`, `pub fn read_lsu_defs(file: &str, text: &str) -> Result<Vec<LsuDef>>`
- **Padanan SWAT+:** `lsu_read_elements` (`lsu_read_elements.f90`), bagian `ls_unit.def`.
- **Tujuan:** membaca definisi unit lanskap (*landscape unit*, LSU) untuk agregasi keluaran.
- **File dan kolom:** judul; jumlah unit `mlsu` (satu nilai); kepala; lalu per unit `id name area elem_tot elem…`.
- **Cara kerja:** lewati judul; `read(1)` → `mlsu`; lewati kepala; ulangi `mlsu` kali seperti `read_ru_defs`, dengan `area_ha` dari posisi 2 dan `nspu` dari posisi 3.

#### 4.5.6  `read_lsu_elements`

- **Lokasi:** `routing.rs`, `pub fn read_lsu_elements(file: &str, text: &str) -> Result<Vec<LsuElem>>`
- **Padanan SWAT+:** `lsu_read_elements`, bagian `ls_unit.ele`.
- **Tujuan:** elemen LSU, ditempatkan pada nomornya (indeks 0 tak dipakai).
- **File dan kolom:** judul, kepala, `id name obj_typ obj_typ_no bsn_frac ru_frac`.
- **Cara kerja:** sama dengan `read_ru_elements`, tanpa penghentian pada nomor 0 (hanya `Eof`).

#### 4.5.7  `read_ru_data`

- **Lokasi:** `routing.rs`, `pub fn read_ru_data(file: &str, text: &str) -> Result<Vec<RuData>>`
- **Padanan SWAT+:** `ru_read` (`ru_read.f90`), pembacaan `rout_unit.rtu` (`read k, ru(i)%name, ru(i)%dbsc`).
- **Tujuan:** nama definisi, *delivery ratio*, topografi, dan lahan (*field*) setiap unit perutean.
- **File dan kolom:** `id name define dlr topo field`.
- **Cara kerja:** `count_rows`; ulangi sebanyak itu: `read(6)`; nomor `i` (minimal 1); perbesar vektor bila perlu dan simpan di `out[i-1]` (indeks 0-based untuk unit 1).
- **Kebiasaan SWAT+ / catatan port:** vektor ini berbasis 0 (unit 1 di indeks 0), berbeda dari `read_aquifers`/`read_sd_channels` yang menyediakan elemen 0 kosong. Nomor 0 atau negatif diperlakukan sebagai 1.

#### 4.5.8  `read_aquifers`

- **Lokasi:** `routing.rs`, `pub fn read_aquifers(file: &str, text: &str) -> Result<Vec<AquiferDb>>`
- **Padanan SWAT+:** `aqu_read` (`aqu_read.f90`).
- **Tujuan:** parameter basis data akuifer dari `aquifer.aqu`.
- **File dan kolom (18):** `id name init flo dep_bot dep_wt no3 minp cbn flo_dist bf_max alpha revap_co seep spyld hlife_n flo_min revap_min`.
- **Cara kerja:**
  1. `count_rows`; vektor mulai dengan satu elemen default (indeks 0).
  2. Ulangi: `read_partial(18)`; nomor `i`; perbesar vektor sampai `i`; isi `out[i]` dengan nilai terbaca atau default `AquiferDb::default()` per kolom; bila `eof`, berhenti setelah menyimpan.
- **Kebiasaan SWAT+ / catatan port:** pemakaian `read_partial` meniru bahwa Fortran menyimpan nilai yang sudah terbaca walau *read* bertemu akhir file. Default kolom berasal dari tipe `aquifer_database` (swat-model).

#### 4.5.9  `read_sd_channels`

- **Lokasi:** `routing.rs`, `pub fn read_sd_channels(file: &str, text: &str) -> Result<Vec<SdChannelData>>`
- **Padanan SWAT+:** `sd_channel_read` (`sd_channel_read.f90`).
- **Tujuan:** baris `channel-lte.cha` (nama dan rujukan ke basis data inisialisasi, hidrologi-sedimen, dan nutrien kanal SWAT-DEG).
- **File dan kolom:** `id name init hyd sed nut`.
- **Cara kerja:** `count_rows`; elemen 0 default; setiap baris `read(6)` ditempatkan di `out[i]`.

**Uji unit subbab 4.5:** `con::tests::outflows`, `rows_without_outflows_and_continued_rows`; `routing::tests::routing_unit_files` (elemen, definisi dengan rentang negatif, akuifer).

### 4.6  Iklim (`climate.rs`)

File iklim SWAT+ tersusun berlapis: `weather-sta.cli` mendefinisikan stasiun cuaca dan, untuk tiap variabel, nama *gage* terukur atau `sim` (disimulasikan generator); `weather-wgn.cli` berisi statistik bulanan generator cuaca; daftar stasiun (`pcp.cli`, `tmp.cli`, `slr.cli`, `hmd.cli`, `wnd.cli`, dan `pet.cli` bila ada) menamai file deret terukur; file deret (`*.pcp`, `*.tmp`, `*.slr`, `*.hmd`, `*.wnd`) memuat nilai harian atau sub-harian; `atmo.cli` memuat deposisi atmosfer nitrogen. Pembaca di modul ini tidak memotong deret ke periode simulasi; penempatan per tanggal dilakukan engine.

**Tipe penting.**

| Tipe | Field utama (satuan) |
| --- | --- |
| `WeatherStation` | `name`, `wgn`, `pcp`, `tmp`, `slr`, `hmd`, `wnd`, `pet`, `atmo_dep`: semua `Option<String>`; `None` bila kolom berisi `null` atau *null*. |
| `Wgn` | `name`, `lat`, `lon` (derajat), `elev` (m), `rain_yrs` (tahun data untuk curah hujan maksimum setengah jam), `months: [WgnMonth; 12]`. |
| `WgnMonth` | `tmp_max_ave`, `tmp_min_ave`, `tmp_max_sd`, `tmp_min_sd` (°C); `pcp_ave` (mm/bulan), `pcp_sd`, `pcp_skew`; `wet_dry`, `wet_wet` (peluang hari basah setelah hari kering/basah); `pcp_days` (rata-rata hari hujan); `pcp_hhr` (hujan maksimum setengah jam, mm); `slr_ave` (MJ/m²/hari); `dew_ave` (°C); `wnd_ave` (m/s). |
| `StationSeries` | `var: ClimateVar`, `file`, `nbyr`, `tstep` (0 = harian; > 0 = langkah per hari untuk hujan sub-harian), `lat`, `lon`, `elev`, `records: Vec<ClimateRecord { year, jday, hour: Option<i32>, values: [f32; 2] }>`. Suhu memakai dua nilai (maks, min); variabel lain satu. Nilai `-99` = data hilang dibiarkan apa adanya. |
| `AtmoDep` | `timestep` (`aa`, `mo`, atau `yr`), `mo_init`, `yr_init`, `num` (jumlah bulan/tahun), `stations: Vec<AtmoStation { name, nh4_rf, no3_rf, nh4_dry, no3_dry }>` (konsentrasi dalam hujan, mg/L, dan deposisi kering, kg/ha; satuan menurut dokumentasi SWAT+, tidak dikonversi di sini). |

#### 4.6.1  `opt` (privat)

- **Lokasi:** `crates/swat-io/src/climate.rs`, `fn opt(value: Option<String>) -> Option<String>`
- **Padanan SWAT+:** pemeriksaan `/= "null"` di `cli_staread`.
- **Tujuan:** menjadikan string `"null"` sama dengan tidak ada (`None`).

#### 4.6.2  `read_stations`

- **Lokasi:** `climate.rs`, `pub fn read_stations(file: &str, text: &str) -> Result<Vec<WeatherStation>>`
- **Padanan SWAT+:** `cli_staread` (`cli_staread.f90`): `read (107,*) wst(i)%name, wst(i)%wco_c`.
- **Tujuan:** daftar stasiun cuaca dan nama *gage* setiap variabel.
- **File dan kolom:** `name wgn pcp tmp slr hmd wnd wnd_dir atmo_dep`.
- **Cara kerja:** lewati dua baris; `read(9)` sampai `Eof`; nama dengan `text_or(0, "")`, delapan kolom lain melalui `opt`.
- **Kebiasaan SWAT+ / catatan port:** kolom ke-8 berlabel `wnd_dir` di kepala file tetapi dipakai SWAT+ sebagai *gage* PET (`wco_c%petgage`); port menyimpannya sebagai `pet`. Nama `sim` disimpan apa adanya (bukan `None`): artinya variabel disimulasikan.

#### 4.6.3  `read_wgn`

- **Lokasi:** `climate.rs`, `pub fn read_wgn(file: &str, text: &str) -> Result<Vec<Wgn>>`
- **Padanan SWAT+:** `cli_wgnread` (`cli_wgnread.f90`).
- **Tujuan:** statistik generator cuaca per stasiun.
- **File dan kolom:** judul; lalu per stasiun: baris `name lat lon elev rain_yrs`, kepala, dan 12 baris bulanan berisi 14 nilai dalam urutan `WgnMonth` di atas.
- **Cara kerja:**
  1. Lewati judul (hanya satu baris; file wgn tidak memiliki kepala global).
  2. Ulangi: `read(5)` (berhenti pada `Eof`); lewati kepala bulanan; 12 kali `read(14)` (galat atau `Eof` di tengah stasiun diteruskan); simpan.
- **Kebiasaan SWAT+ / catatan port:** tidak ada perhitungan turunan di sini (mis. peluang hari basah, `amp_r`, atau `pcf`); itu dikerjakan engine pada inisialisasi generator cuaca.

#### 4.6.4  `read_station_list`

- **Lokasi:** `climate.rs`, `pub fn read_station_list(file: &str, text: &str) -> Result<Vec<String>>`
- **Padanan SWAT+:** bagian awal `cli_pmeas`, `cli_tmeas`, `cli_smeas`, `cli_hmeas`, `cli_wmeas`, `cli_petmeas`: `read (107,*) pcp(i)%filename`.
- **Tujuan:** daftar nama file deret terukur untuk satu variabel.
- **Cara kerja:** lewati dua baris; `skip` berulang sampai `Eof`; nilai tidak-*null* ditambahkan.
- **Kebiasaan SWAT+ / catatan port:** karena `skip` membaca satu token list-directed, spasi di akhir nama dan CRLF tidak masuk (uji `station_list`).

#### 4.6.5  `read_series`

- **Lokasi:** `climate.rs`, `pub fn read_series(var: ClimateVar, file: &str, text: &str) -> Result<StationSeries>`
- **Padanan SWAT+:** bagian pembacaan file data di `cli_pmeas`, `cli_tmeas`, `cli_smeas`, `cli_hmeas`, `cli_wmeas`, `cli_petmeas`.
- **Tujuan:** membaca satu file deret terukur seluruhnya.
- **File dan kolom:** judul; kepala; `nbyr tstep lat lon elev`; lalu per catatan `tahun hari nilai` (suhu: `tahun hari tmax tmin`). Hujan sub-harian (`tstep > 0` dan variabel `Pcp`): `tahun hari bulan hari_bulan jam nilai`.
- **Cara kerja:**
  1. Lewati dua baris; `read(5)` kepala stasiun (galat `Eof` diteruskan; `load_project` menafsirkan ini sebagai file kosong/tak lengkap).
  2. `subdaily = tstep > 0 && var == Pcp`; lebar *read* 6 untuk sub-harian, selain itu `2 + var.values()` (3 atau 4).
  3. Ulangi `read(width)` sampai `Eof`: sub-harian → `hour = Some(kolom 4)`, `values[0] = kolom 5`; harian → `values[0] = kolom 2`, `values[1] = kolom 3` bila variabel bernilai dua.
- **Kebiasaan SWAT+ / catatan port:** pemisah tab dan nilai `-99` dibaca seperti Fortran (uji `series_tabs_and_missing`). `nbyr` di kepala tidak membatasi pembacaan: port membaca semua catatan sampai akhir file (pada Osu\_1hru isi file melampaui jumlah tahun di kepala; uji `refdata`). SWAT+ sendiri mencari catatan awal simulasi lalu menyimpan per tanggal; padanan itu ada di engine. Untuk hujan sub-harian kolom bulan dan hari-bulan dibaca tetapi tidak disimpan.

#### 4.6.6  `read_atmodep`

- **Lokasi:** `climate.rs`, `pub fn read_atmodep(name: &str, text: &str) -> Result<AtmoDep>`
- **Padanan SWAT+:** `cli_read_atmodep` (`cli_read_atmodep.f90`).
- **Tujuan:** deposisi atmosfer NH4 dan NO3 (basah dan kering) per stasiun.
- **File dan kolom:** judul; kepala; `num_sta timestep mo_init yr_init num`; lalu per stasiun: nama, kemudian empat baris (`nh4_rf`, `no3_rf`, `nh4_dry`, `no3_dry`) masing-masing berisi `n` nilai.
- **Cara kerja:**
  1. Lewati dua baris; `read(5)` kepala; `n = 1` bila `timestep == "aa"` (rata-rata tahunan), selain itu `n = num`.
  2. Untuk setiap stasiun (`num_sta`): `read(1)` nama (gagal → berhenti); hingga empat kali `read(n)` (gagal → berhenti di stasiun itu); larik yang tidak terbaca diisi `n` nol; simpan stasiun.
- **Kebiasaan SWAT+ / catatan port:** meniru `if (eof < 0) exit`: pembacaan berhenti di akhir file, tetapi stasiun yang sedang dibaca tetap disimpan dengan nol untuk sisanya. Fortran menyimpan rata-rata tahunan, bulanan, dan tahunan di larik berbeda (`nh4_rf` vs `nh4_rfmo` vs `nh4_rfyr`); port memakai satu vektor per komponen panjang `n`, sedangkan makna (per bulan/tahun) ditentukan oleh `timestep`.

**Uji unit `climate.rs`:** `stations`, `series_tabs_and_missing`, `subdaily_precipitation`, `station_list`.

### 4.7  HRU, tanah, tanaman, pengelolaan, dan tabel keputusan (`hru.rs`)

`hru.rs` membaca file yang mendefinisikan unit respons hidrologi (HRU) dan basis data yang dirujuknya: `hru-data.hru`, `soils.sol`, `soil_plant.ini`, `plant.ini`, `management.sch`, `lum.dtl`, `res_rel.dtl`, dan `soil_lyr_depths.sol`. File-file ini punya struktur bertingkat (catatan kepala diikuti sejumlah catatan anak), sehingga tidak dapat dibaca dengan `table::read_table`. Pola SWAT+ yang ditiru hampir selalu dua lintasan: lintasan hitung (`imax`) yang melompati catatan anak, lalu `rewind` dan lintasan baca.

**Tipe penting.**

| Tipe | Field utama (satuan) |
| --- | --- |
| `HruData` | `id`, `name`, dan nama rujukan `topo`, `hyd`, `soil`, `land_use_mgt`, `soil_plant_init`, `surf_stor`, `snow`, `field`. |
| `Soil` | `name`, `nly` (jumlah lapisan), `hydgrp` (kelompok hidrologi A–D), `zmx` (kedalaman akar maksimum, mm), `anion_excl` (fraksi pori yang mengecualikan anion), `crk` (volume retak potensial), `texture`, `layers`. |
| `SoilLayer` | `z` (kedalaman dasar lapisan, mm), `bd` (Mg/m³), `awc` (mm/mm), `k` (konduktivitas jenuh, mm/jam), `cbn` (C organik, %), `clay`, `silt`, `sand`, `rock` (%), `alb` (albedo tanah lembap), `usle_k`, `ec` (dS/m), `cal` (CaCO₃, %), `ph`. |
| `SoilPlantInit` | `name`, `sw_frac` (fraksi air tanah awal), nama tabel awal `nutc`, `pestc`, `pathc`, `saltc`, `hmetc`, `csc`. |
| `PlantCommunity` | `name`, `rot_yr_ini` (tahun rotasi awal), `plants: Vec<PlantInit>`. |
| `PlantInit` | `cpnm` (nama tanaman di `plants.plt`), `igro` (`y`/`n`, tumbuh saat awal), `lai` (m²/m²), `bioms` (kg/ha), `phuacc` (fraksi akumulasi satuan panas), `pop` (tanaman/m²), `fr_yrmat` (fraksi tahun menuju matang), `rsdin` (residu awal, kg/ha). |
| `MgtSchedule` | `name`, `autos` (nama tabel keputusan otomatis), `auto_crops`, `ops: Vec<MgtOp>`. |
| `MgtOp` | `op` (kode operasi, mis. `till`, `fert`, `plnt`, `harv`, `skip`), `mon`, `day`, `husc` (fraksi satuan panas), `op_char` (nama basis data operasi), `op_plant` (tanaman atau nama tambahan), `op3` (nilai, mis. kg/ha pupuk). |
| `DecisionTable` | `name`, `alts`, `conds: Vec<DtblCond>`, `alt: Vec<Vec<String>>`, `acts: Vec<DtblAct>`, `outcomes: Vec<Vec<String>>`. |
| `DtblCond` | `var`, `ob`, `ob_num`, `lim_var`, `lim_op`, `lim_const`. |
| `DtblAct` | `typ`, `ob`, `ob_num`, `name`, `option`, `const_`, `const2`, `file_pointer`. |

#### 4.7.1  `read_hru_data`

- **Lokasi:** `crates/swat-io/src/hru.rs`, `pub fn read_hru_data(text: &str) -> Result<Vec<HruData>>`
- **Padanan SWAT+:** `hru_read` (`hru_read.f90`).
- **Tujuan:** membaca `hru-data.hru` ke larik berindeks nomor HRU.
- **File dan kolom:** judul, kepala, lalu `id name topo hydro soil lu_mgt soil_plant_init surf_stor snow field`.
- **Cara kerja:**
  1. Lewati dua baris; tandai badan.
  2. Lintasan hitung: `read(1)` berulang; nomor yang sah memperbarui `imax = max(imax, i)`; *null* dan galat konversi diabaikan; `Eof` menghentikan. `imax` dibatasi `[0, 1 000 000]`.
  3. Alokasikan `imax` `HruData` kosong; kembali ke badan.
  4. Ulangi `imax` kali: `read_partial(10)`; bila tidak ada atau nomor tidak sah, berhenti; tempatkan di indeks `i - 1` (bila dalam rentang) dengan semua kolom teks (`text_or(k, "")`); berhenti setelah menyimpan bila `eof`.
- **Kebiasaan SWAT+ / catatan port:** SWAT+ mengukur tabel dengan nomor HRU terbesar, membaca sebanyak itu, dan menaruh setiap baris pada nomornya; nomor yang tidak muncul tetap baris kosong (uji `hru_rows_by_number` dengan baris tidak berurutan). Galat konversi nomor diabaikan saat menghitung, seperti Fortran yang hanya memeriksa `eof < 0`.

#### 4.7.2  `read_soils`

- **Lokasi:** `hru.rs`, `pub fn read_soils(text: &str) -> Result<Vec<Soil>>`
- **Padanan SWAT+:** `soil_db_read` (`soil_db_read.f90`).
- **Tujuan:** basis data tanah `soils.sol` beserta lapisannya.
- **File dan kolom:** judul, kepala; per tanah: `name nly hyd_grp dp_tot anion_excl perc_crk texture`, lalu `nly` baris lapisan `dp bd awc soil_k carbon clay silt sand rock alb usle_k ec caco3 ph`.
- **Cara kerja:**
  1. Lintasan hitung: `read(2)` (nama, `nly`), lalu `skip` sebanyak `nly`; bila akhir file tercapai di tengah lapisan, berhenti tanpa menghitung tanah itu.
  2. Kembali ke badan. Untuk setiap tanah: tandai, `read(2)` untuk `nly` (dibatasi 0–100), kembali, lalu `read_partial(7)` untuk kepala tanah.
  3. `nly` kali `read_partial(14)` untuk lapisan (default 0 bila tidak ada).
  4. Koreksi lapisan tipis: bila `z` lapisan pertama < 20 mm dan (tanah hanya berlapis satu atau `z` lapisan kedua > 20 mm), `z` pertama menjadi 20 mm.
- **Kebiasaan SWAT+ / catatan port:** tekstur dibaca sebagai satu kata list-directed, sehingga `L- L- L- L` menjadi `L-` (`docs/swat-quirks.md`); sisa kata dibuang karena baris berikutnya dibaca dari *record* baru. Aturan 20 mm sama dengan baris 79–81 `soil_db_read`. Kolom kedalaman tanah total (`dp_tot`) disimpan sebagai `zmx`; catatan di `docs/swat-quirks.md` menjelaskan bahwa `hydro_init` kemudian menetapkan `soil%zmx` ke kedalaman lain, jadi nilai ini bukan kedalaman akar akhir.

#### 4.7.3  `read_soil_plant_init`

- **Lokasi:** `hru.rs`, `pub fn read_soil_plant_init(text: &str, nam1: i32) -> Result<Vec<SoilPlantInit>>`
- **Padanan SWAT+:** `soil_plant_init` (`soil_plant_init.f90`).
- **Tujuan:** tabel inisialisasi tanah-tanaman per HRU.
- **File dan kolom:** `name sw_frac nutc pestc pathc saltc hmetc` dan, bila `codes.bsn` `nam1 ≠ 0`, kolom ke-8 `csc` (konstituen).
- **Cara kerja:** lewati dua baris; hitung baris dengan `skip`; kembali; `n = 7` atau `8` menurut `nam1`; ulangi `read_partial(n)`; berhenti setelah menyimpan bila `eof`.
- **Kebiasaan SWAT+ / catatan port:** jumlah kolom tergantung `nam1` persis seperti dua cabang `read` di Fortran; dengan `nam1 = 0` kolom ke-8 (bila ada) diabaikan.

#### 4.7.4  `read_plant_communities`

- **Lokasi:** `hru.rs`, `pub fn read_plant_communities(text: &str) -> Result<Vec<PlantCommunity>>`
- **Padanan SWAT+:** `readpcom` (`readpcom.f90`).
- **Tujuan:** komunitas tanaman awal `plant.ini`.
- **File dan kolom:** judul, kepala; per komunitas `name plt_cnt rot_yr_ini`, lalu `plt_cnt` baris `plt_name lc_status lai_init bm_init phu_init plnt_pop yrs_init rsd_init`.
- **Cara kerja:**
  1. Lintasan hitung: `read(2)` lalu `skip` sebanyak jumlah tanaman; akhir file di tengah → berhenti tanpa menghitung.
  2. Lintasan baca: `read_partial(3)` untuk kepala (jumlah tanaman default 1, dibatasi 0–100; nama default `frsd_frsd`; tahun rotasi default 1).
  3. Untuk setiap tanaman: `read_partial(8)`; nilai yang tidak terbaca memakai `PlantInit::default()`; `igro` hanya karakter pertamanya.
- **Kebiasaan SWAT+ / catatan port:** `igro` dipotong ke satu karakter karena variabel Fortran-nya `character(len=1)`. Default `PlantInit` mengikuti tipe di `plant_data_module` (mis. `rsdin` 10000 kg/ha, `fr_yrmat` 1).

#### 4.7.5  `read_management`

- **Lokasi:** `hru.rs`, `pub fn read_management(text: &str) -> Result<Vec<MgtSchedule>>`
- **Padanan SWAT+:** `mgt_read_mgtops` (`mgt_read_mgtops.f90`).
- **Tujuan:** jadwal pengelolaan `management.sch`.
- **File dan kolom:** judul, kepala; per jadwal `name numb_ops numb_auto`; lalu `numb_auto` baris nama tabel keputusan; lalu `numb_ops` baris operasi `op_typ mon day hu_sch op_data1 op_data2 op_data3`.
- **Cara kerja:**
  1. Lintasan hitung: `read(3)`, `skip` sebanyak `nauto + nops`; akhir file → berhenti tanpa menghitung.
  2. Lintasan baca: `read_partial(3)` kepala.
  3. Untuk setiap tabel otomatis: tandai, `skip` untuk nama. Untuk tabel tanam-panen generik (`pl_hv_summer1`, `pl_hv_winter1` → 1 tanaman; `pl_hv_summer2` → 2), kembali ke tanda dan `read(1 + ncrop)` untuk nama tanaman (`auto_crops`).
  4. Untuk setiap operasi: `read(7)` dengan konversi bertipe; galat konversi dan `Eof` diteruskan sebagai galat.
- **Kebiasaan SWAT+ / catatan port:** pembacaan ulang nama tabel generik adalah padanan `backspace` Fortran di baris 72–81. Komentar kode menyatakan SWAT+ berhenti pada operasi yang buruk dan port pun demikian (mengembalikan `Err`). Tab di akhir baris operasi tidak berpengaruh (uji `management_schedules`).

#### 4.7.6  `read_tables` (privat)

- **Lokasi:** `hru.rs`, `fn read_tables(file: &str, text: &str, res: bool) -> Result<Vec<DecisionTable>>`
- **Padanan SWAT+:** `dtbl_lum_read` (`dtbl_lum_read.f90`) bila `res = false`; `dtbl_res_read` (`dtbl_res_read.f90`) bila `res = true`.
- **Tujuan:** pembaca bersama file tabel keputusan.
- **File dan kolom:** judul; jumlah tabel; satu *record* dilewati; lalu per tabel: kepala; `name conds alts acts`; kepala; `conds` baris `var obj obj_num lim_var lim_op lim_const alt1 … alt_n`; kepala; `acts` baris `act_typ obj obj_num name option const const2 fp outcome1 … outcome_n`; untuk `res_rel.dtl` satu *record* tambahan dilewati di akhir tabel.
- **Cara kerja:**
  1. `skip` judul; `read(1)` jumlah `n`; `read(0)` melewati satu *record* (padanan `read (107,*)` tanpa variabel).
  2. Untuk setiap tabel: `skip` kepala (gagal → berhenti); `read(4)` untuk nama dan ukuran; `skip` kepala.
  3. `conds` kali `read(6 + alts)`: kondisi dan alternatifnya (string, mis. `>`, `<`, `=`, `-`).
  4. `skip` kepala; `acts` kali `read(8 + alts)`: aksi dan hasilnya. `const2`: untuk `lum.dtl` `max(1.0, nilai)` (default 1); untuk `res_rel.dtl` apa adanya (default 0).
  5. Simpan; bila `res`, `read(0)` melewati *record* penutup (akhir file → berhenti).
- **Kebiasaan SWAT+ / catatan port:** `const2 = Max(1., const2)` adalah baris eksplisit di `dtbl_lum_read`; `dtbl_res_read` tidak memilikinya. `dtbl_lum_read` membaca ulang kondisi `prob_unif` untuk mengambil `frac_app` dari kolom objek; port tidak melakukannya di sini, melainkan engine (`swat-engine/src/hru/mgt.rs`) menafsirkan kolom `ob` kondisi `prob_unif` sebagai peluang. Pemetaan nama ke nomor basis data (*cross walk*) dan penghitungan HRU per `land_use` juga dilakukan engine, bukan pembaca.

#### 4.7.7  `read_decision_tables` dan `read_res_decision_tables`

- **Lokasi:** `hru.rs`, `pub fn read_decision_tables(file, text) -> Result<Vec<DecisionTable>>` dan `pub fn read_res_decision_tables(file, text) -> Result<Vec<DecisionTable>>`
- **Padanan SWAT+:** `dtbl_lum_read` dan `dtbl_res_read`.
- **Tujuan:** pembungkus `read_tables` dengan `res = false` (untuk `lum.dtl`) dan `res = true` (untuk `res_rel.dtl`).

#### 4.7.8  `read_soil_lyr_depths`

- **Lokasi:** `hru.rs`, `pub fn read_soil_lyr_depths(text: &str) -> Result<Vec<i32>>`
- **Padanan SWAT+:** bagian `soils_init` (`soils_init.f90`) yang membuka `soil_lyr_depths.sol`.
- **Tujuan:** daftar kedalaman lapisan baku (mm, bilangan bulat) untuk membagi ulang profil tanah.
- **Cara kerja:** lewati **tiga** *record* kepala; `read(1)` berulang sampai `Eof`; setiap nilai dikonversi ke integer (galat diteruskan; *null* → 0).
- **Kebiasaan SWAT+ / catatan port:** file ini dibuka dengan nama tetap, apa pun isi `file.cio` (lihat 4.9). Pemakaian kedalaman untuk membelah lapisan dijelaskan di bab tentang inisialisasi tanah.

**Uji unit `hru.rs`:** `management_schedules`, `hru_rows_by_number`, `soils_and_thin_first_layer` (lapisan pertama 10 mm menjadi 20 mm; tekstur satu kata).

### 4.8  Septik, *recall*, dan kalibrasi (`septic.rs`, `recall.rs`, `cal.rs`)

Tiga pembaca dalam subbab ini menangani file yang strukturnya istimewa. `septic.str` dibaca dengan tokenisasi manual karena perilaku *read* tipe turunan SWAT+ pada file tulisan SWAT+ Editor menghasilkan pergeseran baris yang harus ditiru persis. `recall.rec` menunjuk file-file lain (deret hidrograf terukur atau tercatat dari luar model). `cal_parms.cal` dan `calibration.cal` memuat batas parameter dan perubahan parameter kalibrasi keras (*hard calibration*).

**Tipe penting.**

| Tipe | Field utama |
| --- | --- |
| `SepticSys` | `name`, `typ`, `yr`, `opt` (operasi: 0 tidak aktif, 1 aktif, 2 gagal; `docs/structures.md`), `tfail` (hari), dan larik `r: [f32; 23]` berisi komponen real tipe `septic_system` (mis. indeks bernama `SepticSys::Z`, `SLG2`, `NITR`). |
| `RecallDb` | `id`, `name`, `typ` (0 sub-harian, 1 harian, 2 bulanan, 3 tahunan, 4 koefisien ekspor rata-rata tahunan), `file`, `rows: Vec<RecallRow>`. |
| `RecallRow` | `jday`, `mo`, `day_mo`, `iyr`, `vals: [f32; 18]` (18 komponen hidrograf dalam urutan tipe `hyd_output` SWAT+: volume aliran, sedimen, unsur hara, dan seterusnya). |
| `CalParm` | `name`, `ob_typ`, `absmin`, `absmax` (batas mutlak parameter). |
| `CalUpdate` | `name`, `chg_typ` (`absval`, `abschg`, `pctchg`), `val`, `lyr1`, `lyr2`, `year1`, `year2`, `day1`, `day2`, `elems` (nomor objek; kosong = semua), `conds: Vec<CalCond>`, `val1`, `val2` (rentang kondisi `range`). |
| `CalCond` | `var`, `alt`, `targ`, `targc`. |

#### 4.8.1  `int` dan `real` (privat, `septic.rs`)

- **Lokasi:** `crates/swat-io/src/septic.rs`, `fn int(tok: &str) -> Option<i32>`, `fn real(tok: &str) -> Option<f32>`
- **Padanan SWAT+:** konversi gfortran untuk komponen integer dan real tipe `septic_system`.
- **Tujuan:** memutuskan apakah sebuah token dapat dibaca sebagai bilangan jenis itu.
- **Cara kerja:** `int`: parser `i32` Rust (tanda opsional dan digit). `real`: ganti `d`/`D` dengan `e`, parse `f32`, dan tolak bila token memuat huruf selain `e`, `E`, `d`, `D`.
- **Kebiasaan SWAT+ / catatan port:** versi yang lebih sederhana dari `list::parse_int`/`parse_real` (tidak mengenali eksponen tanpa huruf atau `q`); cukup untuk file septik yang ditulis Editor, dan yang penting di sini adalah bahwa nama sistem berikutnya *gagal* sebagai bilangan.

#### 4.8.2  `read_septic`

- **Lokasi:** `septic.rs`, `pub fn read_septic(text: &str) -> Vec<SepticSys>`
- **Padanan SWAT+:** `sep_read` (`sep_read.f90`): hitung `imax` dengan `read titldum` sampai `eof`, `rewind`, lalu `read (172,*,iostat=eof) sep(isep)` untuk `isep = 1..imax`.
- **Tujuan:** sistem septik `septic.str`, elemen 0 kosong (sistem yang ditunjuk HRU tanpa septik).
- **File dan kolom:** judul, kepala, lalu per sistem 28 item: nama, tiga integer (`typ`, `yr`, `operation`), dua real, satu integer (`tfail`), dan 21 real. Kepala yang ditulis Editor hanya menamai 22 kolom, tetapi Editor menulis 28 nilai.
- **Cara kerja:**
  1. Pisahkan teks pada `\n` (buang `\r`); badan = baris setelah dua baris pertama. `nlines` = jumlah baris badan, dikurangi satu bila teks diakhiri `\n` (baris kosong terakhir tidak dihitung). Inilah `imax`: berbeda dari pola umum, *read* list-directed ke variabel karakter juga "membaca" baris kosong, sehingga baris kosong di tengah ikut dihitung, tetapi akhir file tidak.
  2. Mulai dengan satu sistem default (elemen 0); kursor baris `line = 0`.
  3. Ulangi `nlines` kali: sistem default `s`, `item = 0`. Ambil token dari baris di kursor (pemisah spasi, koma, tab), maju satu baris. Untuk setiap token sampai `item = 28`: item 0 → nama; item 1, 2, 3, 6 → integer (`typ`, `yr`, `opt`, `tfail`); item lain → real ke `r[k]` dengan `k = item − 4` untuk item 4–5 dan `k = item − 5` untuk item 7–27. Token gagal → hentikan *read* sistem ini. Bila baris habis sebelum 28 item dan tidak ada galat, lanjut ke baris berikutnya.
  4. Bila kursor melewati `nlines` sebelum 28 item (akhir file), simpan `s` (apa yang terbaca tetap) dan hentikan loop.
  5. Isi sisa hingga `nlines + 1` elemen dengan sistem default.
- **Kebiasaan SWAT+ / catatan port:** dengan baris Editor yang memuat 22 nilai, *read* 28 item melanjutkan ke baris sistem berikutnya; nama sistem berikutnya, dibaca sebagai bilangan, menghentikan *read* dengan galat (item yang belum terbaca tetap 0), dan *read* berikutnya mulai di baris sesudahnya. Akibatnya setiap sistem kedua "termakan" oleh sistem sebelumnya, dan *read* yang mencapai akhir file menghentikan pembacaan; uji `editor_rows_take_the_next_line` menunjukkan dua baris Editor menghasilkan satu sistem terbaca dan satu sistem default. Port menirukan perilaku ini secara sengaja demi kecocokan *bit-for-bit*. Mengapa sistem septik praktis tidak mengalirkan efluen sama sekali dijelaskan di `docs/structures.md` dan bab struktur.

#### 4.8.3  `read_recalls`

- **Lokasi:** `crates/swat-io/src/recall.rs`, `pub fn read_recalls(src: &dyn FileSource, name: &str, text: &str) -> Result<Vec<RecallDb>>`
- **Padanan SWAT+:** `recall_read` (`recall_read.f90`).
- **Tujuan:** membaca `recall.rec` dan setiap file data yang dirujuknya.
- **File dan kolom:** `recall.rec`: judul, kepala, lalu `id name rec_typ file`.
- **Cara kerja:**
  1. Lewati dua baris. Ulangi `read(4)` sampai `Eof`.
  2. Bangun `RecallDb` (nomor minimal 0).
  3. Bila `typ ≠ 4`: baca file data lewat `src` (file tidak ada → teks kosong) dan panggil `read_rows`. Bila `read_rows` mengembalikan `None` (file berakhir sebelum kepalanya lengkap), **hentikan** pembacaan `recall.rec` di sana (catatan ini tidak disimpan).
  4. Simpan catatan (tipe 4 tidak punya file sendiri).
- **Kebiasaan SWAT+ / catatan port:** di Fortran, `open` pada file yang tidak ada membuatnya kosong, lalu `read titldum` bertemu `eof < 0` dan `exit` keluar dari loop seluruh `recall.rec`; port meniru dengan memperlakukan file hilang sebagai teks kosong.

#### 4.8.4  `read_rows` (privat)

- **Lokasi:** `recall.rs`, `fn read_rows(file: &str, text: &str) -> Result<Option<Vec<RecallRow>>>`
- **Padanan SWAT+:** bagian dalam `recall_read` yang membaca file data.
- **File dan kolom:** judul; jumlah tahun (`nbyr`); kepala; lalu per langkah waktu `jday mo day_mo iyr ob_typ ob_name` diikuti 18 nilai hidrograf (24 item).
- **Cara kerja:** `skip` tiga kali (gagal → `Ok(None)`); lalu `read(24)` berulang sampai `Eof`; nilai 6–23 ke `vals` (default 0), empat bilangan bulat tanggal ke field masing-masing; `ob_typ` dan `ob_name` dibaca tetapi tidak disimpan.
- **Kebiasaan SWAT+ / catatan port:** Fortran mencari tahun awal simulasi, menyimpan per hari/bulan/tahun menurut tipe, dan berhenti setelah `yrc_end`; port menyimpan semua baris apa adanya, sehingga pemilihan periode dan penempatan menurut tipe terjadi di luar crate ini.

#### 4.8.5  `read_cal_parms`

- **Lokasi:** `crates/swat-io/src/cal.rs`, `pub fn read_cal_parms(file: &str, text: &str) -> Result<Vec<CalParm>>`
- **Padanan SWAT+:** `cal_parm_read` (`cal_parm_read.f90`).
- **Tujuan:** daftar parameter yang dapat dikalibrasi beserta jenis objek dan batas mutlaknya.
- **File dan kolom:** judul; jumlah parameter; kepala; lalu `name obj_typ absmin absmax units`.
- **Cara kerja:** `skip`; `read(1)` jumlah (min 0); `skip`; ulangi: `read_partial(5)` (tidak ada → berhenti), kolom yang tidak terbaca memakai `CalParm::default()`; berhenti setelah menyimpan bila `eof`. Kolom satuan dibaca tetapi tidak disimpan.

#### 4.8.6  `read_calibration`

- **Lokasi:** `cal.rs`, `pub fn read_calibration(file: &str, text: &str) -> Result<Vec<CalUpdate>>`
- **Padanan SWAT+:** `cal_parmchg_read` (`cal_parmchg_read.f90`).
- **Tujuan:** perubahan parameter kalibrasi dari `calibration.cal`.
- **File dan kolom:** judul; jumlah perubahan; kepala; per perubahan baris `name chg_typ val conds soil_lyr1 soil_lyr2 yr1 yr2 day1 day2 obj_tot obj1 …`, lalu `conds` baris kondisi.
- **Cara kerja:**
  1. `skip`; `read(1)` jumlah `n`; `skip`.
  2. Untuk setiap perubahan: tandai; `read(11)` (`Eof` → berhenti); `nspu = obj_tot` (posisi 10, min 0); `nconds` (posisi 3).
  3. Bila `nspu > 0`: kembali; `read(11 + nspu)`; daftar nomor → `define_unit_elements` → `elems` (bila `obj_tot ≠ 0`; 0 berarti semua objek).
  4. Untuk setiap kondisi: intip kata pertama (tandai, `read(1)`, kembali). Bila `range`: `read(4)` → `range var val1 val2`, kondisi dengan `var` dan rentang disimpan di `val1`/`val2` milik perubahan. Selain itu: `read(4)` → `var alt targ targc`.
- **Kebiasaan SWAT+ / catatan port:** "intip lalu baca ulang" meniru `read range` + `backspace`-lalu-`read` di Fortran (baris 94–100). Seperti di Fortran, rentang `range` disimpan per perubahan, sehingga beberapa kondisi `range` dalam satu perubahan saling menimpa `val1`/`val2`.

**Uji unit subbab 4.8:** `septic::tests::editor_rows_take_the_next_line`; `cal::tests::calibration_with_object_list` (rentang `1 -5` → lima objek; `cal_parms`). `recall.rs` tidak memiliki uji unit.

### 4.9  Memuat satu skenario (`project.rs`)

`project.rs` adalah pintu masuk crate: satu fungsi yang memuat semua file yang dibutuhkan engine dari sebuah `FileSource`, dalam urutan yang mengikuti urutan pembacaan SWAT+, dan melaporkan apa yang hilang tanpa gagal.

**Tipe penting.**

| Tipe | Isi |
|---|---|
| `LoadReport` | `missing: Vec<String>`: file yang dinamai `file.cio` atau daftar stasiun tetapi tidak ada di skenario (SWAT+ memeriksa dengan `inquire (exist=…)` dan melewatinya); `unread: Vec<String>`: file deret terukur yang berakhir sebelum kepalanya lengkap (mis. file kosong) beserta file sesudahnya dalam daftar yang sama, karena SWAT+ berhenti membaca variabel itu di sana. |
| `Project` (swat-model) | `cio`, `codes`, `params`, `time`, `print`, `basin_name`, `area_ls_ha`, `area_tot_ha`, `objects`, `stations`, `wgn`, tabel basis data (`topography`, `hydrology`, `field`, `landuse`, `cntable`, `ovn_table`, `cons_practice`, `snow`, `tillage`, `plants`, `soil_test`, `urban`, `fertilizer`, `irr_ops`, `chem_app`, `harv_ops`, reservoir, struktur, kanal, dsb.), `lum_dtl`, `res_dtl`, `septic`, `recalls`, `management`, `plant_communities`, unit LSU/RU, `aquifer_db`, `sd_channel_data`, `cal_parms`, `cal_upd`, `soils`, `soil_plant_init`, `hru_data`, `station_lists`, `series`, dan beberapa bendera (`aqu_elems`, `ch_temp_file`, `co2_yr_dat`). |

#### 4.9.1  `load_project`

- **Lokasi:** `crates/swat-io/src/project.rs`, `pub fn load_project(src: &dyn FileSource) -> Result<(Project, LoadReport)>`
- **Padanan SWAT+:** urutan pembacaan masukan di program utama dan `proc_*` SWAT+: `readcio_read`, `basin_read_cc`, `basin_read_prm` (+ `basin_prm_default`), `time_read`, `basin_print_codes_read`, `basin_read_objs`, `hyd_read_connect`, `cli_staread`, `cli_wgnread`, pembaca basis data (`proc_db`), `proc_res`, struktur, `cli_read_atmodep`, `recall_read`, `mgt_read_mgtops`, `readpcom`, `lsu_read_elements`, `ru_read`/`ru_read_elements`, `aqu_read`, `proc_cha`, `proc_cal`, `soils_init`, `carbon_bsn_read`, `soil_db_read`, `soil_plant_init`, `hru_read`, dan pembaca data iklim terukur (`cli_pmeas` dan kerabatnya).
- **Tujuan:** menghasilkan `Project` lengkap dan `LoadReport`.
- **Masukan dan keluaran:** sumber file → `(Project, LoadReport)`; `Err(Missing("file.cio"))` bila `file.cio` tidak ada; galat parse dari pembaca mana pun diteruskan.
- **Cara kerja:**
  1. Baca dan urai `file.cio` (`cio::read`). Siapkan *closure* `fetch(section, slot)`: ambil nama file slot itu dari `file.cio`; slot `null` → `None` tanpa catatan; file tidak ada → catat di `report.missing` dan `None`; ada → `(nama, teks)`.
  2. `codes.bsn` → `basin::read_codes`; `parameters.bsn` → `basin::read_params`; lalu **selalu** `project.params.complete()` (padanan `basin_prm_default`, juga bila file tidak ada).
  3. `time.sim` → `time::read`; `print.prt` → `print::read`.
  4. `object.cnt` (padanan `basin_read_objs`): lewati dua baris, `read(19)`: nama cekungan, luas daratan dan luas total (ha), lalu 15 hitungan objek di posisi 4–18 (urutan `sp_ob`: hru, hru_lte, ru, gwflow, aqu, chan, res, recall, exco, dr, canal, pump, outlet, chandeg, aqu2d). Kegagalan `read` diabaikan (hitungan tidak diketahui).
  5. Untuk setiap `ObjectKind`: tentukan indeks hitungannya; bila hitungan diketahui dan 0, **lewati** file koneksi; selain itu baca file koneksi dari slot `connect` jenis itu dengan `con::read` dan potong ke `n` objek bila hitungan diketahui.
  6. `weather-sta.cli` → `read_stations`; `weather-wgn.cli` → `read_wgn`.
  7. Dua belas tabel HRU dengan `read_table` (topografi, hidrologi, lahan, `landuse.lum`, `cntable.lum`, `ovn_table.lum`, `cons_practice.lum`, salju, olah tanah, tanaman, `nutrients.sol`, perkotaan); empat tabel operasi (pupuk, irigasi, aplikasi kimia, panen).
  8. Tabel keputusan `lum.dtl` dan `res_rel.dtl`; tabel reservoir/lahan basah (`init_res`, `sed_res`, `nut_res`, `wet`, `hyd_wet`, `res`, `hyd_res`); struktur (`tiledrain.str`, `septic.str` lewat `read_septic`, `septic.sep`, `filterstrip.str`, `grassedww.str`, `bmpuser.str`); `atmo.cli`; `recall.rec` (lewat `read_recalls`, yang sendiri membaca file data dari `src`); `weir.res`.
  9. `management.sch`, `plant.ini`; `ls_unit.def`, `ls_unit.ele`; bendera `aqu_elems` bila slot `ele_aqu` dinamai dan filenya ada; `rout_unit.ele`, `rout_unit.rtu`, dan `rout_unit.def` (dengan `n_ru` = jumlah objek `RoutingUnit` yang telah dimuat); `aquifer.aqu`.
  10. Kanal: `hyd-sed-lte.cha`, `nutrients.cha`, `initial.cha` (slot `init`), `om_water.ini`, `channel-lte.cha` (`read_sd_channels`); bendera `ch_temp_file` bila slot suhu kanal tidak `null`.
  11. Kalibrasi: `cal_parms.cal`, `calibration.cal`. Bendera `co2_yr_dat` bila file `co2_yr.dat` ada (nama tetap).
  12. Empat tabel bernama tetap, apa pun isi `file.cio`: `puddle.ops`, `manure_om.frt`, `manure_db.frt`, `transplant.ops`; juga `soil_lyr_depths.sol` (nama tetap, seperti `soils_init`). Ketidakhadiran file-file ini tidak dicatat.
  13. Bila `codes.cswat == 2`: `carbon.bsn` dengan `read_record` (ketat), lalu file per lapisan yang namanya diturunkan dari nama `carbon.bsn` dengan menyisipkan `_lyr` sebelum titik terakhir (`x.bsn` → `x_lyr.bsn`; tanpa titik → `<nama>_lyr.bsn`); bila tidak ada, dicatat di akhir `missing`.
  14. `soils.sol`, `soil_plant.ini` (dengan `codes.nam1`), `hru-data.hru`.
  15. Iklim terukur: untuk setiap `ClimateVar`, baca daftar stasiunnya (`read_station_list`). Lalu per variabel: awalan folder dari slot jalur `file.cio` (dipangkas; kosong bila tidak ada) digabung langsung dengan nama file (tanpa pemisah tambahan, seperti `TRIM(ADJUSTL(in_path_pcp%pcp))//filename`). Untuk setiap file secara berurutan: bila variabel sudah dihentikan, masukkan ke `unread`; file tidak ada → `missing` dan hentikan variabel; `read_series` menghasilkan `Eof` → `unread` dan hentikan; galat lain diteruskan; berhasil → `project.series`.
  16. Simpan `station_lists`; tambahkan `late_missing` (file lapisan karbon) ke `missing`; kembalikan.
- **Kebiasaan SWAT+ / catatan port:**
  - File koneksi dengan hitungan 0 di `object.cnt` tidak dibaca sama sekali, dan hanya `n` baris pertama yang dipakai (`docs/swat-quirks.md`, bagian *Inputs read by count or position*). Akibatnya slot `file.cio` yang berisi salah ketik (Ames_sub1: `nbull` untuk akuifer) tidak menimbulkan catatan hilang bila tidak ada akuifer.
  - Berkas deret iklim yang hilang atau kosong menghentikan **seluruh** stasiun berikutnya untuk variabel yang sama, meniru `exit` di `cli_pmeas` dan kerabatnya (file yang dibuka `open` tanpa ada menjadi kosong, lalu `read` kepala bertemu `eof`).
  - Nama file bertetap (`puddle.ops`, `manure_om.frt`, `manure_db.frt`, `transplant.ops`, `soil_lyr_depths.sol`, `co2_yr.dat`) meniru subrutin SWAT+ yang membuka nama literal; contoh dari `docs/swat-quirks.md`: Osu_1hru menyertakan `transplant.plt`, bukan `transplant.ops`, sehingga tidak memiliki data tanam pindah.
  - `params.complete()` dipanggil sebelum `time.sim` dibaca, dan tidak bergantung pada keberadaan `parameters.bsn`.
  - Urutan pembacaan di port tidak memengaruhi nilai (pembaca saling independen kecuali `n_ru`, `nam1`, `cswat`, dan hitungan `object.cnt`), tetapi menentukan urutan entri `LoadReport::missing`.

### 4.10  Pengujian (`tests/refdata.rs` dan uji unit)

Selain uji unit yang disebut di akhir setiap subbab (34 fungsi uji, termasuk dua fungsi bantu di `list.rs`), crate ini memiliki uji integrasi `crates/swat-io/tests/refdata.rs` yang memuat skenario rujukan SWAT+ dari folder `testdata/` repo dan memeriksa isinya. Nilai data skenario tidak dicantumkan di nota ini; yang diuji adalah *perilaku* pembaca berikut.

| Fungsi | Lokasi | Kegunaan |
|---|---|---|
| `scenario(name) -> DirSource` | `tests/refdata.rs` | membuat `DirSource` ke `testdata/<name>` relatif terhadap `CARGO_MANIFEST_DIR`. |
| `osu_1hru` | `tests/refdata.rs` | skenario Osu_1hru: tidak ada file hilang; periode `time.sim` dan jumlah hari simulasi; `nyskip`; `igen` tetap default karena `day_lag_mx` ditulis `0.00000` (4.4.6); kode PET; bendera cetak; jumlah objek per jenis sesuai `object.cnt`; dua aliran keluar unit perutean (ke kanal SWAT-DEG `tot` dan ke akuifer `rhg`); stasiun dengan kelembapan `sim`; generator cuaca; deret hujan dan suhu (catatan pertama; jumlah catatan sama dengan jumlah hari yang benar-benar ada di file walaupun `nbyr` di kepala lebih kecil). |
| `ames_sub1` | `tests/refdata.rs` | skenario Ames_sub1: daftar `missing` tepat (file yang dinamai `file.cio` tetapi tidak ada; slot akuifer salah ketik tidak dicatat karena tidak ada akuifer); jadwal pengelolaan, tabel otomatis dan operasinya; tabel panen dan pupuk; `soil_lyr_depths.sol`; tabel keputusan (kondisi, alternatif, aksi, `const2`, objek `unlim`); baris `init` `file.cio` dengan nama berlebih; `time.sim` dengan `day_start`/`day_end` 0; HRU tanpa aliran keluar; `tmp.cli` menunjuk file kosong sehingga masuk `unread` dan tidak ada deret suhu; deret hujan dengan CRLF dan `-99`. |
| `ames_hru_inputs` | `tests/refdata.rs` | skenario Ames_sub1: `hru-data.hru`; tanah, tekstur satu kata, lapisan; `cn_froz` 0 menjadi default oleh `complete()`; baris `biomix` `tillage.til`; `carbon.bsn` (dibaca karena `cswat = 2`); tanaman `plants.plt`; komunitas `plant.ini`; `nutrients.sol` tidak ada; `co2_yr.dat` tidak ada; topografi, hidrologi, `landuse.lum`, `cntable.lum` (larik `cn`), salju, `soil_plant.ini`. |
| `osu_hru_inputs` | `tests/refdata.rs` | skenario Osu_1hru: satu HRU dengan penyimpanan permukaan *paddy*; tanah yang dirujuknya ada dan berlapis. |

**Ringkasan cakupan bab.** Bab ini mendokumentasikan 78 fungsi non-uji crate `swat-io`: `lib.rs` 3, `source.rs` 6, `list.rs` 27, `table.rs` 3, `cio.rs` 1, `time.rs` 1, `print.rs` 1, `basin.rs` 3, `connect.rs` 1, `routing.rs` 8, `climate.rs` 6, `hru.rs` 9, `septic.rs` 3, `recall.rs` 2, `cal.rs` 2, `project.rs` 1; ditambah 34 fungsi uji unit dan 5 fungsi di `tests/refdata.rs`.
