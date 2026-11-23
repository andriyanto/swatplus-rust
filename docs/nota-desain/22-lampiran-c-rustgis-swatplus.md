## Lampiran C  RustGIS-SwatPlus: model SWAT+ di GIS dan asisten AI

Lampiran ini mencatat pekerjaan di repositori `andriyanto/rustgis-swatplus` (RustGIS-SwatPlus), turunan RustGIS (`opengeos/RustGIS`) berlisensi Apache-2.0, fungsi demi fungsi seperti bab-bab sebelumnya. Isinya butir A1 dan F1(e) di `docs/roadmap.md`: model SWAT+ dibuka, disunting, dijalankan, dan hasilnya dibaca lewat perintah RustGIS dan asisten AI-nya. Lampiran ini diperbarui setiap kali satu langkah selesai; C.6 mencatat urutan langkahnya.

### C.1  Kedudukan terhadap engine

RustGIS bekerja lewat **perintah**: setiap aksi adalah `CommandSpec` di `crates/engine/src/cmd/*.rs`, dipanggil oleh pita menu, pencarian perintah, baris perintah, skrip, dan asisten AI. Asisten menjalankan perintah yang sama dengan pengguna, dan perintah yang menyentuh berkas, jaringan, atau plugin menunggu persetujuan pengguna. Menambah kemampuan SWAT+ berarti menambah perintah `swat.*` dan skill untuk asisten.

**Keputusan K11 tetap berlaku.** Engine berlisensi LGPL-2.1 (turunan SWAT+), RustGIS-SwatPlus Apache-2.0. Kode engine tidak digabung ke dalam program RustGIS. Untuk menjalankan model (`swat.run`, A1.1), RustGIS-SwatPlus memanggil program `swatplus-rust run --progress json --format parquet` di desktop dan memuat modul `swat_wasm.wasm` di browser, lewat antarmuka yang dijelaskan di Bab 15 dan 16 (`docs/runner.md`). Butir A1.1 roadmap, yang semula menulis "engine sebagai pustaka", dikoreksi pada 24 November 2026.

Untuk membaca dan menyunting berkas TxtInOut, RustGIS-SwatPlus memakai dua crate miliknya sendiri, ditulis dari dokumentasi format, bukan dari kode SWAT+:

| Crate | Lapisan | Tugas |
| --- | --- | --- |
| `rustgis-swat-model` | L1 | Proyek SWAT+ sebagai teks berkasnya (`Project`, `TextFile`), tanpa I/O. Berkas yang dibaca lalu ditulis lagi sama byte demi byte; `TextFile::set_token` mengganti satu nilai dan menjaga kolom tetap sejajar |
| `rustgis-swat-txtinout` | L2 | Berkas dari dan ke byte (`decode`, `encode`, `read`, `write`), sehingga juga berjalan di browser; masukan `file.cio` (`cio::inputs`, `cio::missing`); tata letak setiap berkas (`view`: tabel, rekaman yang memiliki beberapa baris seperti lapisan tanah, jadwal pengelolaan, tabel keputusan) |

### C.2  Perintah `swat.*` (A1.2, sudah ada)

Berkas `crates/engine/src/cmd/swat.rs`. Proyek SWAT+ dibuka di samping peta (`Session::swat`, struct `SwatState`: `dir` folder asal, `original` proyek seperti dibaca, `project` proyek seperti disunting, `confidential`), bukan bagian proyek peta dan tidak ikut tersimpan bersamanya.

| Perintah | Fungsi | Efek (izin) | Isi |
| --- | --- | --- | --- |
| `swat.open` | `open`, `read_folder` | berkas | Membaca `file.cio`, lalu setiap berkas yang disebutnya, lalu yang disebut berkas-berkas itu (berkas cuaca, recall), sampai tidak ada yang baru. Nama dengan `..` atau jalur mutlak dilewati; paling banyak 20.000 berkas dan 4 GB. `confidential` membuka dalam mode rahasia. Di browser ditolak (proyek dibuka dari folder, di desktop) |
| `swat.close` | `close` | — | Menutup proyek; menyebut jumlah berkas yang belum disimpan |
| `swat.summary` | `summary` | — | Periode simulasi (`time.sim`), jumlah setiap jenis objek (`object.cnt`), berkas, dan berkas yang hilang |
| `swat.check` | `check` | — | Berkas yang disebut `file.cio` tetapi tidak ada (`cio::missing`), dan nilai parameter di luar rentang `cal_parms.cal`. Rentang itu batas kalibrasi, sehingga nilai di luarnya didaftar, tidak dianggap salah (nilai bawaan SWAT+ Editor bisa berada di luarnya) |
| `swat.find` | `find` | — | Berkas dan kolom yang namanya memuat teks yang dicari (`cn`, `esco`, `awc`), dengan jumlah rekaman, rentang nilainya, dan rentang `cal_parms.cal` |
| `swat.get` | `get` | — | Nilai satu kolom (paling banyak 500 rekaman didaftar) dan statistiknya (`stats`: jumlah, rentang, rata-rata, banyaknya nilai berbeda). Rekaman dipilih dengan `match` atas namanya dan `where` atas kolom lain, keduanya dengan `*` dan `?` (`glob`, `picked`) |
| `swat.set` | `set`, `format_like` | — | Mengubah nilai rekaman terpilih: `replace`, `add`, atau `multiply` (dengan 1 + nilai, seperti `r__` SWAT-CUP). Semua perubahan dihitung dulu; bila satu saja keluar dari rentang `cal_parms.cal`, tidak ada yang diubah, kecuali dengan `force`. Nilai baru ditulis dengan desimal sebanyak nilai lamanya, atau dalam bentuk eksponen bila nilai lama begitu. Hanya di memori sampai `swat.save` |
| `swat.diff` | `diff`, `changed_files` | — | Setiap nilai yang berubah sejak dibuka, per berkas dan kolom (dalam mode rahasia hanya jumlahnya) |
| `swat.revert` | `revert` | — | Membatalkan perubahan: semua, satu berkas, atau satu kolom satu berkas |
| `swat.save` | `save` | berkas | Menulis berkas yang berubah ke folder asalnya, atau seluruh proyek ke folder lain (`path`). Byte lain tetap |
| `swat.confidential` | `confidential` | khusus pengguna | Menyalakan atau mematikan mode rahasia. Hanya pengguna yang dapat mematikannya; asisten dan plugin tidak |

**Mode rahasia.** Proyek yang ditandai rahasia (seperti Batanghari dan Pessel) hanya menjawab dengan jumlah dan statistik, tidak pernah nama atau nilai rekaman, sehingga data itu tidak terkirim ke penyedia model AI. Pilihan lain adalah model lokal lewat Ollama.

**Skill asisten.** `swat-model` (berkas, objek, dan arti parameter) dan `swat-edit` (menyunting dengan aman: rentang, konfirmasi, selisih), di `crates/assistant/skills`.

### C.3  Hujan GSMaP dan geometri ber-Z/M (F1(e), T1.0, sudah ada)

| Bagian | Lokasi | Isi |
| --- | --- | --- |
| Pembaca GSMaP | `crates/data/src/raster/gsmap.rs`: `is_gsmap_name`, `read`, `label`, `gunzip`, `crc32` | Grid biner JAXA (GSMaP_NOW, Gauge_NOW, NRT, MVK, Gauge; per jam 0,1° dan harian 0,25°), dengan atau tanpa gzip, dibaca dalam Rust murni. Float 4 byte *little-endian* dari 60° LU sampai 60° LS, diputar agar mulai di 180° BB; nilai negatif (tak ada pengamatan) menjadi kosong. Satu pita, laju hujan mm/jam; nama lapisan dari produk dan waktu mulai |
| `rain.style` | `crates/engine/src/cmd/rain.rs`: `style`, `classes` | Menggambar grid hujan dalam kelas intensitas BMKG (per jam: ringan 1–5, sedang 5–10, lebat 10–20, sangat lebat > 20 mm/jam; harian: 0,5–20, 20–50, 50–100, 100–150, > 150 mm) |
| `rain.accumulate` | `accumulate`, `start_minutes`, `hours_in` | Menjumlah grid laju hujan per jam menjadi hujan selama periodenya (mm) |
| `rain.zonal` | `zonal`, `zone_mean` | Rata-rata hujan setiap zona (misalnya sub-DAS) dari setiap lapisan, dibobot bagian setiap sel yang tercakup zona, sebagai tabel |
| Z dan M | `crates/data`: `Dataset::zm`, `VertexZm`, `push_next_zm`, `zm_of`, `wkb::decode_zm`/`encode_zm` | Tinggi dan ukuran setiap verteks disimpan di samping geometri 2D, per id fitur; dibaca dan ditulis untuk Shapefile (PointZ, PolylineZ, …), GeoJSON (koordinat ketiga), dan WKB. Entri yang jumlah verteksnya tidak lagi cocok (diubah alat atau suntingan) diabaikan, sehingga tinggi tidak pernah jatuh ke verteks yang salah |

Data GSMaP tidak disimpan di repositori (ketentuan JAXA).

### C.4  A1.0: sinkronisasi dengan RustGIS asli

Pada 24 November 2026 repositori tertinggal 1.145 commit dari `opengeos/RustGIS`; sinkronisasi sebelumnya awal November. Penggabungan menyentuh 28 berkas yang juga diubah di sisi kita, 13 di antaranya berkonflik. Yang perlu dipikirkan, bukan sekadar disalin:

1. **Izin perintah.** RustGIS kini menyatakan efek setiap perintah (`CommandSpec::effects`: berkas, jaringan, plugin, khusus pengguna, dan lain-lain) dan perannya (`roles`); perintah tanpa pernyataan ditolak untuk asisten, dan uji `permission_policy_metadata_presence_and_unreviewed_default` mewajibkannya. Daftar lama di `cmd/plugins.rs` (`NEVER`, `WRITES_FILES`) dihapus di sana. Semua perintah `swat.*` dan `rain.*` kini menyatakannya: `swat.open` dan `swat.save` berkas, `swat.confidential` khusus pengguna (sebelumnya di `NEVER`), selebihnya tanpa efek di luar proyek.
2. **Z dan M.** RustGIS menambahkan `Dataset::zm_dropped`: tanda bahwa Z atau M dibuang saat dibaca, dengan catatan untuk pengguna. Repositori kita menyimpannya (`Dataset::zm`). Keduanya dipertahankan:
   - Shapefile dan GeoJSON yang dibaca utuh menyimpan Z/M dan tidak ditandai dibuang. Untuk GeoJSON, ukuran (M, nilai keempat; `geojson_has_m`) dan tinggi yang tidak cocok dengan geometrinya tetap dihitung dibuang dan diberitahukan.
   - TopoJSON, yang dibaca lewat GeoJSON, hanya menyimpan x dan y seperti di RustGIS (`read_geojson_flat`), karena tinggi di busur bersama tidak kita simpan.
   - Pembaca "di tempat" (berkas besar yang dibaca sebagian) dan format lain (FlatGeobuf, KML, GML, CityJSON, FileGDB) tetap seperti RustGIS: dibuang dan ditandai. Penulis Shapefile memakai penulis Z/M kita (`write_shp_feature`) dengan langkah kemajuan RustGIS. Penulis GeoJSON bertahap RustGIS (`GeoJsonWriter`), yang kini dipakai ekspor, diberi tinggi lewat `GeoJsonWriter::heights`, sehingga ekspor GeoJSON tetap menulis koordinat ketiga.
3. **Pembaca raster.** RustGIS menambahkan PDF dan beberapa format; GSMaP tetap dikenali di samping PDF (`is_raster_name` dan `read_path_placing` di `raster/mod.rs`).
4. **API yang berubah.** `imagery::name` kini mengembalikan `Result` (dua pemanggilan di `rain.rs`); pola destrukturisasi `Dataset` di `map` dan `engine` mendapat field `zm`.
5. **CI.** Langkah pengosongan disk dari repositori kita dipertahankan di `test-rust.yml`, dengan `actions/checkout` yang dipatok RustGIS; `ci-label.yml` tetap hanya berjalan di `opengeos/RustGIS`. Workflow baru `mobile.yml` menandatangani aplikasi Android dan iOS lalu mengunggahnya ke toko aplikasi dengan kunci dan akun RustGIS; job-job itu kini hanya berjalan di repositori asli.
6. **Lint dan uji.** RustGIS kini melarang `#[allow]` (`allow_attributes`): modul uji kita memakai cara RustGIS (`#[expect(…, reason = …)]`, atau tanpa atribut untuk `#[cfg(test)]`). Uji RustGIS yang mengharapkan tinggi GeoJSON dan Shapefile dibuang disesuaikan: kini memeriksa bahwa tinggi disimpan, dan uji pemberitahuan memakai ukuran (M) sebagai nilai yang dibuang. Snapshot daftar perintah (`command-surface.txt`) memuat `swat.*` dan `rain.*`.

**Hasil pemeriksaan lokal** (Linux, Rust 1.97, 2 inti): clippy `-D warnings` lulus pada crate yang tersentuh (data, engine, plugins, assistant, map, swat-model, swat-txtinout). Dua lint baru Rust 1.97 dikecualikan untuk pemeriksaan lokal ini saja, karena hanya muncul di dua baris kode RustGIS yang tidak kita ubah (`manual_option_zip`, `manual_range_contains`). Dari 5.290 uji, 4 gagal karena hal di luar penggabungan: uji izin folder (proses berjalan sebagai root), uji proyeksi yang lulus bila dijalankan sendiri, uji waktu WAL SQLite pada MBTiles (bagian yang tidak tersentuh), dan `never_silent_reading` yang kemudian disesuaikan dan lulus.

### C.5  Langkah berikutnya

| Langkah | Isi |
| --- | --- |
| A1.1 `swat.run` | Menjalankan model sebagai program terpisah (desktop) atau modul WASM (browser), mode asli atau `--fix-swat`, dengan kemajuan dan pembatalan; riwayat run mencatat versi engine, selisih parameter, dan sidik berkas |
| A1.2 keluaran | `swat.output.list`, `swat.output.read`, `swat.balance` (neraca air DAS), `swat.compare` (dua run) |
| A1.2 pengamatan | `swat.observed`, `swat.metrics` (NSE, KGE, PBIAS), juga untuk TMA lewat lengkung debit `swatcal` |
| A1.2 peta dan grafik | `swat.map`, `swat.chart` |
| A1.3 skill | `swat-run-results`, `swat-diagnose`; dokumentasi swatplus-rust untuk pencarian asisten |

### C.6  Catatan langkah kerja

| Tanggal | Langkah | Hasil dan tempatnya |
| --- | --- | --- |
| awal November 2026 | A1.0 sinkronisasi pertama; T1.0 Z/M; A1.2 perintah `swat.*` dan skill; F1(e) GSMaP dan `rain.*` | rustgis-swatplus, cabang `feat/swat-txtinout`, `sync-upstream`, `t1-zm`, `a1-swat-commands`, `f1e-gsmap`, `f1e-rain`, `ci-free-disk` |
| 24 November 2026 | Keputusan: K11 berlaku untuk RustGIS-SwatPlus (engine dijalankan sebagai program atau modul WASM, tidak digabung); pekerjaan dicatat di Lampiran C | `docs/roadmap.md` A1.1 dikoreksi; `CLAUDE.md` |
| 24 November 2026 | A1.0 sinkronisasi kedua: 1.145 commit RustGIS, 13 berkas berkonflik diselesaikan, izin perintah `swat.*`/`rain.*`, Z/M GeoJSON dan Shapefile tetap disimpan, job penandatanganan `mobile.yml` hanya di repo asli; 5.290 uji, 4 gagal di luar penggabungan (C.4) | rustgis-swatplus PR #10 |
