## Bab 2  Arsitektur dan prinsip desain

Engine terdiri dari enam crate Rust tanpa dependensi eksternal. Data mengalir satu arah: file TxtInOut dibaca menjadi model proyek, model disimulasikan hari demi hari, dan hasilnya ditulis sebagai tabel. RustGIS menjalankan engine sebagai program terpisah atau modul WebAssembly, tidak pernah sebagai pustaka yang ditautkan.

### 2.1  Crate dan tugasnya

| Crate | Jenis | Tugas | Bergantung pada |
| --- | --- | --- | --- |
| `swat-model` | pustaka | Data proyek: `file.cio`, kode dan parameter DAS, periode simulasi, pengaturan cetak, objek spasial dan alirannya, cuaca, HRU, tanah, tabel database. Tanpa I/O | — |
| `swat-io` | pustaka | Pembaca file TxtInOut, mengikuti aturan input list-directed Fortran seperti SWAT+ membacanya; dari folder atau dari file di memori | `swat-model` |
| `swat-engine` | pustaka | Simulasi: jam, cuaca, jadwal cetak, HRU, routing unit, akuifer, saluran, reservoir, recall, dalam urutan perintah SWAT+ | `swat-model` |
| `swat-output` | pustaka | Keluaran: tabel teks SWAT+, Parquet, arsip tar, progres JSON-line; penyimpanan di folder atau memori | `swat-model`, `swat-engine` |
| `swat-wasm` | cdylib | Engine sebagai modul WebAssembly: arsip tar skenario masuk, arsip tar tabel keluar | semua di atas |
| `swatplus-rust` | program | Program baris perintah: `inspect`, `run`, `compare` | semua kecuali `swat-wasm` |

`swat-engine` tidak bergantung pada `swat-io`: engine menerima `Project` yang sudah jadi, dari mana pun asalnya. Ini memungkinkan RustGIS kelak membangun proyek tanpa file TxtInOut.

![Arsitektur engine: enam crate](img/arsitektur.png)

Empat pustaka membentuk satu jalur dari file ke tabel; program baris perintah dan modul WebAssembly hanyalah dua cara membungkus jalur yang sama.

### 2.2  Alur data satu kali jalan

1. **Baca.** `swat-io` membaca `file.cio`, lalu setiap file yang dirujuknya, menjadi `swat_model::Project`. Sumber file bisa folder (`DirSource`) atau memori (arsip tar di browser).
2. **Siapkan.** `swat-engine::run_with` membangun objek simulasi: stasiun cuaca, HRU dengan tanah dan tanamannya, unit routing, akuifer, saluran, reservoir. Kalibrasi diterapkan setelah inisialisasi, seperti di SWAT+.
3. **Simulasikan.** Untuk setiap hari: perbarui jam, hitung cuaca, jalankan perintah objek dalam urutan `object.cnt`/`*.con` (HRU, unit routing, akuifer, saluran, reservoir, recall), akumulasikan periode.
4. **Tulis.** Setiap hasil harian dikirim ke `OutputSink`. `swat-output` mengubahnya menjadi tabel teks persis seperti format `write` Fortran, atau Parquet, lalu menyimpannya lewat `Store` (folder atau memori).
5. **Laporkan.** Progres dan peristiwa (`start`, `progress`, `note`, `table`, `done`, `error`) keluar sebagai baris JSON.

### 2.3  Prinsip desain

**Bit demi bit.** Tujuannya bukan model yang “mirip SWAT+”, melainkan tabel yang sama persis. Akibatnya:

- Presisi mengikuti SWAT+: variabel `real` tetap f32, `real*8` tetap f64, dan konversi antar presisi terjadi di tempat yang sama.
- Urutan operasi aritmetika dipertahankan, karena penjumlahan floating point tidak asosiatif.
- Fungsi matematika dan format angka meniru gfortran dan glibc (`fortran.rs`).
- Urutan pemrosesan objek mengikuti urutan perintah SWAT+, termasuk variabel yang membawa nilai dari satu HRU ke HRU berikutnya.

**Kebiasaan SWAT+ dipertahankan.** Perilaku yang tampak salah tetap ditiru, dan dicatat di `docs/swat-quirks.md`. Yang jelas bug bisa dikoreksi dengan `--fix-swat`, dan daftarnya ada di `docs/swat-fixes.md`.

**Tolak, jangan tebak.** Fitur yang belum di-port menghasilkan catatan bernama, bukan perkiraan. Bagian yang ditolak meneruskan airnya ke hilir; `--strict` mengubahnya menjadi kode keluar 3.

**Tanpa dependensi, tanpa `unsafe`.** Semua crate hanya memakai pustaka standar Rust (`unsafe_code = "forbid"`). Penulis Parquet, tar, dan JSON ditulis sendiri. Karena itu engine bisa dibangun untuk `wasm32-unknown-unknown` tanpa glue JavaScript.

**Pemisahan lisensi (keputusan K11).** Engine berlisensi LGPL-2.1, sama dengan SWAT+. RustGIS berlisensi Apache-2.0 dan hanya menjalankan engine sebagai proses terpisah atau modul WASM; keduanya bertemu lewat format data: TxtInOut masuk, tabel dan JSON keluar.

### 2.4  Mode koreksi `--fix-swat`

Tanpa flag ini, hasil sama dengan SWAT+. Dengan flag ini, bug yang terdokumentasi diperbaiki: misalnya variabel yang tak sengaja dibawa dari HRU sebelumnya, atau pembagian luas yang dilakukan dua kali. Hasilnya tidak bisa dibandingkan lagi dengan SWAT+, sehingga validasi bit demi bit hanya berlaku untuk mode bawaan. Paralelisme HRU direncanakan hanya untuk mode koreksi, karena di mode bawaan urutan HRU memengaruhi hasil (Bab 17).

### 2.5  Hubungan dengan RustGIS

RustGIS (editor SWAT+) membaca dan mengedit TxtInOut dengan crate-nya sendiri (`rustgis-swat-model`, `rustgis-swat-txtinout`), lalu menjalankan engine ini lewat runner. Di desktop runner memanggil program `swatplus-rust run --progress json --format parquet`; di browser runner memuat `swat_wasm.wasm` di Web Worker. Kontrak antarmukanya dijelaskan di Bab 15 dan 16.
