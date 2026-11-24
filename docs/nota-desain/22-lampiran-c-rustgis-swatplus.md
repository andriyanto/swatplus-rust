## Lampiran C  RustGIS-SwatPlus: model SWAT+ di GIS dan asisten AI

Lampiran ini mencatat pekerjaan di repositori `andriyanto/rustgis-swatplus` (RustGIS-SwatPlus), turunan RustGIS (`opengeos/RustGIS`) berlisensi Apache-2.0, fungsi demi fungsi seperti bab-bab sebelumnya. Isinya butir A1 dan F1(e) di `docs/roadmap.md`: model SWAT+ dibuka, disunting, dijalankan, dan hasilnya dibaca lewat perintah RustGIS dan asisten AI-nya. Lampiran ini diperbarui setiap kali satu langkah selesai; C.6 mencatat urutan langkahnya.

### C.1  Kedudukan terhadap engine

RustGIS bekerja lewat **perintah**: setiap aksi adalah `CommandSpec` di `crates/engine/src/cmd/*.rs`, dipanggil oleh pita menu, pencarian perintah, baris perintah, skrip, dan asisten AI. Asisten menjalankan perintah yang sama dengan pengguna, dan perintah yang menyentuh berkas, jaringan, atau plugin menunggu persetujuan pengguna. Menambah kemampuan SWAT+ berarti menambah perintah `swat.*` dan skill untuk asisten.

**Keputusan K11 tetap berlaku.** Engine berlisensi LGPL-2.1 (turunan SWAT+), RustGIS-SwatPlus Apache-2.0. Kode engine tidak digabung ke dalam program RustGIS. Untuk menjalankan model (`swat.run`, A1.1), RustGIS-SwatPlus memanggil program `swatplus-rust run --progress json` di desktop dan memuat modul `swat_wasm.wasm` di browser, lewat antarmuka yang dijelaskan di Bab 15 dan 16 (`docs/runner.md`). Butir A1.1 roadmap, yang semula menulis "engine sebagai pustaka", dikoreksi pada 24 November 2026.

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

**CI di GitHub** menemukan tiga hal lagi, yang sudah diperbaiki: satu akses langsung ke baris lapisan melebihi batas yang hanya boleh turun (`xtask rows`; `rain.zonal` kini memakai `Features::rows`), nama berkas `time.sim` di `swat.rs` terbaca sebagai perintah oleh pemindai ID perintah karena RustGIS kini punya `time.*` (dikecualikan dengan alasannya), dan uji Python ekspor GeoJSON yang mengharapkan tinggi dibuang (kini memakai ukuran M). Sesudahnya semua pemeriksaan CI lulus.

### C.5  A1.1: menjalankan model (`swat.run`, tahap pertama)

Tahap pertama untuk desktop dan baris perintah: model dijalankan dari RustGIS-SwatPlus dengan program `swatplus-rust`, sesuai K11. Polanya sama dengan cara RustGIS menjalankan perangkat GeoLibre, yang juga program terpisah yang bisa diunduh (`crates/geolibre`).

**Crate `rustgis-swat-run`** (`crates/swat-run`, lapisan L3; bagian program hanya di desktop):

| Modul dan fungsi | Isi |
| --- | --- |
| `event::Event::parse` | Membaca satu baris kejadian JSON engine (`--progress json`, Bab 15–16): `start` (versi, jumlah hari, tanggal pertama dan terakhir), `progress`, `note`, `table` (berkas dan barisnya untuk Parquet), `done` (hari dan detik), `error`. Baris yang bukan kejadian dilewati, tidak ditolak |
| `runner::find`, `choose`, `version_of`, `check_supported` | Mencari program berurutan: pilihan pengguna, variabel lingkungan `SWATPLUS_RUST`, folder pemasangan, samping aplikasi, `PATH`. Program dipakai bila menjawab `--version` sebagai `swatplus-rust` versi 1.1.0 sampai sebelum 2.0.0 (1.1.0 punya `--threads`). Jawaban `--version` ditunggu paling lama 10 detik; program lain yang kebetulan bernama sama dihentikan |
| `runner::RELEASE` | Versi yang dipasang: 1.2.0, versi yang sudah divalidasi. Dimajukan lewat pull request |
| `install::install`, `find_asset`, `asset_name`, `uninstall` | Mengunduh arsip platform ini dari rilis GitHub swatplus-rust yang dipatok, memeriksa SHA-256 yang diterbitkan GitHub untuk arsip itu (`digest`), membongkarnya, mencobanya, lalu baru memasangnya di folder data aplikasi. Unduhan yang gagal di salah satu langkah membiarkan program yang terpasang apa adanya. Enam platform: Linux dan Windows x86-64 dan ARM64, macOS Apple silicon dan Intel |
| `run::Spec::args` | Argumen program: `run <input> --out <output> --progress json --format <parquet\|text\|both>`, ditambah `--fix-swat --threads N` untuk mode koreksi |
| `run::run` | Menjalankan program, membaca kejadiannya dari keluaran standar di thread tersendiri (sehingga permintaan berhenti tetap diperiksa setiap 50 ms walau engine diam), menguras keluaran galat di thread lain, dan menghentikan proses bila diminta. Hasilnya `Outcome`: versi, hari, tanggal, tabel, catatan, detik, galat (kejadian `error`, atau 4 KB terakhir keluaran galat bila engine diam), berhenti atau tidak, dan kode keluar |
| `run::digest` | Sidik masukan: SHA-256 atas nama, panjang, dan isi setiap berkas, terurut menurut nama. Dua run dengan sidik dan versi engine yang sama membaca masukan yang sama |

**Perintah baru** (`crates/engine/src/cmd/swat.rs`):

| Perintah | Efek (izin) | Isi |
| --- | --- | --- |
| `swat.engine` | — | Program yang dipakai (jalur, versi, terpasang lewat aplikasi atau tidak), atau cara mendapatkannya |
| `swat.setEngine` | khusus pengguna, berkas | Menunjuk program; kosong berarti mencari lagi di tempat biasa |
| `swat.installEngine` | khusus pengguna | Memasang rilis yang dipatok (`install::install`) |
| `swat.run` | berkas | Menjalankan proyek seperti disunting. Run diberi folder `run-NNNN` sendiri di `swat-runs/<nama folder proyek>` dalam folder data aplikasi, atau di folder yang diberikan (`folder`). Bila tidak ada yang berubah, engine membaca folder proyek sendiri (engine hanya membaca); bila ada, proyek seperti disunting ditulis ke `input` di folder run, dan berkas proyek asli tidak tersentuh. Tabel ditulis ke `output`. Catatan run (`run.json` di folder run) memuat nomor, folder, jalur dan versi engine, mode (asli atau `fix-swat`) dan jumlah thread, format, berkas dan kolom yang berubah (dalam mode rahasia hanya jumlahnya), sidik masukan, berhasil atau tidak, hari dan tanggal, detik, tabel, catatan, dan galat |
| `swat.runs` | — | Semua run sejak proyek dibuka |

Asisten AI boleh menjalankan model, tetapi setiap `swat.run` menunggu persetujuan pengguna karena menulis berkas dan menjalankan program. Memasang atau menunjuk engine hanya boleh dilakukan pengguna.

**Uji.** `rustgis-swat-run`: pembacaan kejadian sesuai `docs/runner.md`, versi dan rentangnya, aset dan SHA-256 dari informasi rilis, argumen program, galat dari kejadian atau dari keluaran galat, penghentian run, dan sidik masukan. `swat.rs`: dengan program tiruan yang menjawab seperti `swatplus-rust`, run tanpa perubahan membaca folder proyek, run dengan perubahan membaca salinan yang disunting sementara berkas asli tetap, mode `--fix-swat --threads 2` sampai ke argumen, catatan `run.json` sama dengan jawaban perintah, dan masukan yang salah ditolak.

Selain uji otomatis, `swat.run` dicoba sekali dengan program sungguhan `swatplus-rust 1.2.0` pada proyek Osu_1hru, tanpa perubahan dan sesudah `swat.set` `esco` 0,5 dalam mode `--fix-swat --threads 2`. Keduanya berhasil: 5.113 hari, 1 Januari 2010 sampai 31 Desember 2023. Uji ini tidak disimpan di repositori karena butuh program engine. Panduan pengguna RustGIS (`docs/user-guide/analysis.md`, bagian SWAT+ models) serta tabel crate di `README.md` dan `docs/architecture.md` ikut diperbarui.

#### Tahap kedua: run di latar, dengan kemajuan dan tombol Batal

Pada tahap pertama, `swat.run` menahan jendela aplikasi selama model berjalan. Untuk Batanghari, itu bisa berarti beberapa menit tanpa respons. RustGIS sudah punya jalur untuk pekerjaan panjang seperti itu: perintah yang terdaftar di `reads::IN_BACKGROUND` (Add Data, Run Model, dan lainnya) dikerjakan di thread tersendiri, dan baris status menampilkan kemajuan serta tombol Batal. `swat.run` kini masuk daftar itu dengan nama "Run SWAT+ Project", jadi tidak perlu mekanisme baru.

| Fungsi | Isi |
| --- | --- |
| `swat::prepare_run` | Bekerja di thread jendela. Memeriksa parameter, mengambil daftar berkas proyek, dan hanya meng-encode berkas yang diubah, sehingga ringan walau berkas cuaca proyek besar. Proyek tanpa folder meng-encode semua berkasnya |
| `Prepared::go` | Bekerja di thread sendiri. Mencari engine, membuat folder `run-NNNN`, menyusun masukan (berkas yang tidak berubah disalin dari folder proyek karena isinya sama dengan yang dibaca, berkas yang diubah ditulis dari suntingan), menghitung sidik masukan, menjalankan engine, lalu menulis `run.json` bagaimanapun run berakhir (berhasil, gagal, atau dihentikan) |
| kemajuan | Diambil dari kejadian engine: `start` memberi jumlah hari, dan setiap `progress` menambah hari yang sudah disimulasikan, dicatat pada kontrol thread (`rustgis_analysis::control`). Baris status menampilkannya sebagai persentase |
| Batal | Ditanyakan di antara berkas dan setiap 50 ms selama engine berjalan. Bila diminta, proses engine dihentikan, `run.json` mencatat `"stopped": true` dan sidik masukan kosong, dan run tidak masuk `swat.runs` karena perintahnya tidak mendarat. Run yang gagal tetap dicatat bersama galatnya |
| `swat::run_ahead`, `swat::run` | `run_ahead` menyerahkan pekerjaan ke antrean baca (`reads::prepare`). Saat mendarat, `run` mengambil hasilnya dan mencatatnya pada proyek yang dijalankan (`run.json` kini memuat `project`), bukan proyek lain yang dibuka sementara run berjalan. Tanpa antrean (baris perintah, skrip, asisten di aplikasi), `run` mengerjakan semuanya langsung seperti tahap pertama |
| `run::Fingerprint` | Sidik masukan dihitung berkas demi berkas, sehingga berkas proyek besar tidak perlu dimuat ke memori sekaligus. Hasilnya sama dengan `digest` |

**Jangkauan.** Di desktop (pita, pencarian perintah, Record Task), `swat.run` berjalan di latar. Agen yang tersambung lewat MCP juga mendapat jalur latar karena `swat.run` ditambahkan ke daftar baca bercakupan di `mcp.rs` dan `dispatch.rs`: jawabannya datang saat run mendarat, dan aplikasi tetap bisa dipakai sementara itu. Baris perintah, skrip, dan asisten di dalam aplikasi menunggu run selesai, sama seperti untuk semua perintah baca di RustGIS. Bila thread pekerja gagal dibuat, aplikasi menolak run dan meminta pengguna mencoba lagi, bukan menjalankannya langsung sehingga jendela tertahan. Browser belum menjalankan model; itu tahap berikutnya.

**Uji.**
- `cmd::swat`: run yang disiapkan di depan menghitung hari engine sebagai kemajuan dan mendarat sama seperti run langsung. Batal ketika engine berjalan (`Control::cancelled_at_look`, tanpa menunggu waktu) menghentikan engine, `run.json` mencatat `stopped`, dan tidak ada yang mendarat. Uji-uji yang menunjuk engine bergiliran karena pilihan engine berlaku untuk seluruh aplikasi.
- `long_commands` mengecualikan `swat.run`, dengan alasan bahwa perintah ini menjalankan program yang tidak ada di fikstur; Batal-nya diuji di `cmd::swat`.
- Di aplikasi, `readjob::a_swat_run_is_made_off_the_window_and_lands`.
- Dengan engine sungguhan pada Osu_1hru lewat `read_ahead`, `run`, dan `read_land`: kemajuan 5.113 dari 5.113 hari, berhasil.

Dua koreksi kecil ikut masuk: panduan pengguna sempat menulis `both` sebagai format bawaan, padahal bawaannya `parquet`; dan crate `rustgis-swat-run` kembali memuat blok `#![deny(clippy::unwrap_used, …)]` yang diwajibkan AGENTS.md.

**Tahap berikutnya A1.1:** modul WASM engine untuk browser.

#### Langkah sesudah A1.1

| Langkah | Isi |
| --- | --- |
| A1.2 keluaran | `swat.output.list`, `swat.output.read`, `swat.balance` (neraca air DAS), `swat.compare` (dua run) |
| A1.2 pengamatan | `swat.observed`, `swat.metrics` (NSE, KGE, PBIAS), juga untuk TMA lewat lengkung debit `swatcal` |
| A1.2 peta dan grafik | `swat.map`, `swat.chart` |
| A1.3 skill | `swat-run-results`, `swat-diagnose`; dokumentasi swatplus-rust untuk pencarian asisten |

### C.6  Catatan langkah kerja

| Tanggal | Langkah | Hasil dan tempatnya |
| --- | --- | --- |
| awal November 2026 | A1.0 sinkronisasi pertama; T1.0 Z/M; A1.2 perintah `swat.*` dan skill; F1(e) GSMaP dan `rain.*` | rustgis-swatplus, cabang `feat/swat-txtinout`, `sync-upstream`, `t1-zm`, `a1-swat-commands`, `f1e-gsmap`, `f1e-rain`, `ci-free-disk` |
| 24 November 2026 | Keputusan: K11 berlaku untuk RustGIS-SwatPlus (engine dijalankan sebagai program atau modul WASM, tidak digabung); pekerjaan dicatat di Lampiran C | `docs/roadmap.md` A1.1 dikoreksi; `CLAUDE.md` |
| 24 November 2026 | A1.0 sinkronisasi kedua: 1.145 commit RustGIS, 13 berkas berkonflik diselesaikan, izin perintah `swat.*`/`rain.*`, Z/M GeoJSON dan Shapefile tetap disimpan, job penandatanganan `mobile.yml` hanya di repo asli; 5.290 uji lokal, 4 gagal di luar penggabungan; CI lulus setelah tiga perbaikan (C.4) | rustgis-swatplus PR #10 |
| 24 November 2026 | A1.1 tahap pertama: crate `rustgis-swat-run` (mencari, memasang, dan menjalankan `swatplus-rust`), perintah `swat.engine`, `swat.setEngine`, `swat.installEngine`, `swat.run`, `swat.runs` (C.5) | rustgis-swatplus PR #11 |
| 24 November 2026 | A1.1 tahap kedua: `swat.run` di thread sendiri lewat `reads::IN_BACKGROUND`, dengan hari engine sebagai kemajuan dan tombol Batal yang menghentikan engine; `run.json` juga ditulis untuk run yang dihentikan; agen MCP ikut lewat jalur latar (C.5) | rustgis-swatplus PR #12 |
